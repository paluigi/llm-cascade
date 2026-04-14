use std::time::Instant;

use chrono::{Duration, Utc};
use rusqlite::Connection;

use crate::config::{AppConfig, ProviderConfig, ProviderType};
use crate::db;
use crate::error::{CascadeError, ProviderError};
use crate::models::{Conversation, LlmResponse};
use crate::persistence;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::gemini::GeminiProvider;
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai::OpenAiProvider;
use crate::providers::LlmProvider;
use crate::secrets;

const BASE_COOLDOWN_SECS: i64 = 30;
const MAX_COOLDOWN_SECS: i64 = 3600;

fn build_provider(
    provider_name: &str,
    provider_config: &ProviderConfig,
    model: &str,
) -> Result<Box<dyn LlmProvider>, ProviderError> {
    match provider_config.r#type {
        ProviderType::Openai => {
            let service = provider_config.api_key_service.as_deref().unwrap_or(provider_name);
            let env_var = provider_config.api_key_env.as_deref().unwrap_or("OPENAI_API_KEY");
            let api_key = secrets::resolve_api_key(service, env_var)
                .map_err(|_| ProviderError::MissingApiKey(provider_name.into()))?;
            Ok(Box::new(OpenAiProvider::new(api_key, model.into(), provider_config.base_url.clone())))
        }
        ProviderType::Anthropic => {
            let service = provider_config.api_key_service.as_deref().unwrap_or(provider_name);
            let env_var = provider_config.api_key_env.as_deref().unwrap_or("ANTHROPIC_API_KEY");
            let api_key = secrets::resolve_api_key(service, env_var)
                .map_err(|_| ProviderError::MissingApiKey(provider_name.into()))?;
            Ok(Box::new(AnthropicProvider::new(api_key, model.into(), provider_config.base_url.clone())))
        }
        ProviderType::Gemini => {
            let service = provider_config.api_key_service.as_deref().unwrap_or(provider_name);
            let env_var = provider_config.api_key_env.as_deref().unwrap_or("GOOGLE_API_KEY");
            let api_key = secrets::resolve_api_key(service, env_var)
                .map_err(|_| ProviderError::MissingApiKey(provider_name.into()))?;
            Ok(Box::new(GeminiProvider::new(api_key, model.into(), provider_config.base_url.clone())))
        }
        ProviderType::Ollama => {
            Ok(Box::new(OllamaProvider::new(model.into(), provider_config.base_url.clone())))
        }
    }
}

fn compute_cooldown(entry_key: &str, conn: &Connection) -> Duration {
    let current = query_cooldown_level(entry_key, conn);
    let secs = (BASE_COOLDOWN_SECS * 2_i64.pow(current)).min(MAX_COOLDOWN_SECS);
    Duration::seconds(secs)
}

fn query_cooldown_level(entry_key: &str, conn: &Connection) -> u32 {
    let now = Utc::now().to_rfc3339();
    let count = conn.query_row(
        "SELECT COUNT(*) FROM attempt_log
         WHERE provider_model = ?1 AND http_status >= 400 AND timestamp > ?2",
        rusqlite::params![entry_key, now],
        |row| row.get::<_, i64>(0),
    );

    match count {
        Ok(c) if c > 0 => (c as u32).saturating_sub(1),
        _ => 0,
    }
}

pub async fn run_cascade(
    cascade_name: &str,
    conversation: &Conversation,
    config: &AppConfig,
    conn: &Connection,
) -> Result<LlmResponse, CascadeError> {
    let cascade = config.cascades.get(cascade_name).ok_or_else(|| CascadeError {
        cascade_name: cascade_name.to_string(),
        message: format!("Cascade '{}' not found in configuration", cascade_name),
        failed_prompt_path: persistence::save_failed_conversation(
            conversation,
            &config.failure_persistence.dir,
            cascade_name,
        ),
    })?;

    if cascade.entries.is_empty() {
        let path = persistence::save_failed_conversation(
            conversation,
            &config.failure_persistence.dir,
            cascade_name,
        );
        return Err(CascadeError {
            cascade_name: cascade_name.to_string(),
            message: format!("Cascade '{}' has no entries", cascade_name),
            failed_prompt_path: path,
        });
    }

    let mut errors = Vec::new();
    let mut skipped = Vec::new();

    for entry in &cascade.entries {
        let provider_config = match config.providers.get(&entry.provider) {
            Some(c) => c,
            None => {
                tracing::warn!(
                    "Provider '{}' referenced in cascade '{}' not found in config",
                    entry.provider,
                    cascade_name,
                );
                errors.push(format!("{}/{}: provider not found", entry.provider, entry.model));
                continue;
            }
        };

        let entry_key = format!("{}/{}", entry.provider, entry.model);

        if db::is_on_cooldown(conn, &entry_key) {
            tracing::info!("Skipping '{}' — currently on cooldown", entry_key);
            skipped.push(entry_key);
            continue;
        }

        let provider = match build_provider(&entry.provider, provider_config, &entry.model) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to initialize provider '{}': {}", entry_key, e);
                errors.push(format!("{}: {}", entry_key, e));
                continue;
            }
        };

        tracing::info!("Attempting provider: {}", entry_key);
        let start = Instant::now();

        match provider.complete(conversation).await {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                db::log_attempt(
                    conn,
                    cascade_name,
                    &entry_key,
                    Some(200),
                    latency_ms,
                    response.input_tokens,
                    response.output_tokens,
                );
                tracing::info!(
                    "Success from {} ({}ms, in_tokens: {:?}, out_tokens: {:?})",
                    entry_key,
                    latency_ms,
                    response.input_tokens,
                    response.output_tokens,
                );
                return Ok(response);
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let http_status = e.http_status();
                db::log_attempt(
                    conn,
                    cascade_name,
                    &entry_key,
                    http_status,
                    latency_ms,
                    None,
                    None,
                );

                let cooldown = if e.is_rate_limited() {
                    if let Some(retry_secs) = e.retry_after_seconds() {
                        Duration::seconds(retry_secs as i64)
                    } else {
                        compute_cooldown(&entry_key, conn)
                    }
                } else {
                    compute_cooldown(&entry_key, conn)
                };

                let cooldown_until = (Utc::now() + cooldown).to_rfc3339();
                db::set_cooldown(conn, &entry_key, &cooldown_until);

                tracing::warn!(
                    "Provider '{}' failed (HTTP {:?}): {}. Cooldown until {}",
                    entry_key,
                    http_status,
                    e,
                    cooldown_until,
                );
                errors.push(format!("{}: {}", entry_key, e));
            }
        }
    }

    let mut message = String::new();
    if !skipped.is_empty() {
        message.push_str(&format!("Skipped (on cooldown): {}\n", skipped.join(", ")));
    }
    message.push_str(&format!("Failed entries: {}", errors.join("; ")));

    let failed_prompt_path = persistence::save_failed_conversation(
        conversation,
        &config.failure_persistence.dir,
        cascade_name,
    );

    Err(CascadeError {
        cascade_name: cascade_name.to_string(),
        message,
        failed_prompt_path,
    })
}
