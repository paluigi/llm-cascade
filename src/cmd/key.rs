use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Args, Subcommand};

use llm_cascade::config::{self, ProviderConfig};
use llm_cascade::secrets;

#[derive(Args, Debug)]
pub struct KeyArgs {
    #[command(subcommand)]
    pub command: KeyCommand,
}

#[derive(Subcommand, Debug)]
pub enum KeyCommand {
    /// Store an API key in the OS keyring for a provider
    Set {
        /// Provider name (as defined in config)
        provider: String,
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Retrieve and display an API key for a provider
    Get {
        /// Provider name (as defined in config)
        provider: String,
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Show the full key instead of masked
        #[arg(long)]
        show_full: bool,
    },
    /// List all providers and their key status
    List {
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Delete an API key from the OS keyring
    Delete {
        /// Provider name (as defined in config)
        provider: String,
        /// Path to config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

pub fn execute(args: KeyArgs) {
    match args.command {
        KeyCommand::Set { provider, config } => cmd_set(provider, config),
        KeyCommand::Get {
            provider,
            config,
            show_full,
        } => cmd_get(provider, config, show_full),
        KeyCommand::List { config } => cmd_list(config),
        KeyCommand::Delete { provider, config } => cmd_delete(provider, config),
    }
}

fn load_provider_config(
    config_path: Option<PathBuf>,
    provider_name: &str,
) -> Result<ProviderConfig, String> {
    let path = config_path.unwrap_or_else(config::default_config_path);
    let app_config = config::load_config(&path)?;
    app_config
        .providers
        .get(provider_name)
        .cloned()
        .ok_or_else(|| format!("Provider '{}' not found in config", provider_name))
}

pub fn set_key_for_provider(
    config_path: Option<PathBuf>,
    provider_name: &str,
) -> Result<(), String> {
    let provider = load_provider_config(config_path, provider_name)?;
    let service_name = provider.api_key_service.as_deref().ok_or_else(|| {
        format!(
            "Provider '{}' does not use an API key (e.g. Ollama)",
            provider_name
        )
    })?;

    eprint!("Enter API key for {}: ", provider_name);
    io::stderr()
        .flush()
        .map_err(|e| format!("Failed to flush stderr: {}", e))?;
    let key = rpassword::read_password().map_err(|e| format!("Failed to read password: {}", e))?;
    if key.trim().is_empty() {
        return Err("API key cannot be empty".to_string());
    }

    secrets::set_key(service_name, &key)?;
    println!("API key for '{}' stored in keyring.", provider_name);
    Ok(())
}

fn cmd_set(provider: String, config: Option<PathBuf>) {
    if let Err(e) = set_key_for_provider(config, &provider) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_get(provider: String, config: Option<PathBuf>, show_full: bool) {
    let provider_config = match load_provider_config(config, &provider) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let service_name = match provider_config.api_key_service.as_deref() {
        Some(s) => s,
        None => {
            println!("Provider '{}' does not use an API key.", provider);
            return;
        }
    };

    match secrets::get_key(service_name) {
        Ok(key) => {
            if show_full {
                println!("{}", key);
            } else {
                println!("{}", secrets::mask_key(&key));
            }
        }
        Err(_) => {
            if let Some(env_var) = &provider_config.api_key_env {
                match std::env::var(env_var) {
                    Ok(key) => {
                        if show_full {
                            println!("(from env var {}) {}", env_var, key);
                        } else {
                            println!("(from env var {}) {}", env_var, secrets::mask_key(&key));
                        }
                    }
                    Err(_) => {
                        eprintln!(
                            "No API key found for '{}' in keyring or env var '{}'.",
                            provider, env_var
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("No API key found for '{}' in keyring.", provider);
                std::process::exit(1);
            }
        }
    }
}

fn cmd_list(config: Option<PathBuf>) {
    let path = config.unwrap_or_else(config::default_config_path);
    let app_config = match config::load_config(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut rows: Vec<(String, String, String)> = Vec::new();

    for (name, provider) in &app_config.providers {
        let (keyring_status, env_status) = match &provider.api_key_service {
            Some(service) => {
                let kr = if secrets::has_key(service) {
                    "set".to_string()
                } else {
                    "not set".to_string()
                };
                let env = match &provider.api_key_env {
                    Some(var) if std::env::var(var).is_ok() => "set".to_string(),
                    Some(_) => "not set".to_string(),
                    None => "n/a".to_string(),
                };
                (kr, env)
            }
            None => ("n/a".to_string(), "n/a".to_string()),
        };
        rows.push((name.clone(), keyring_status, env_status));
    }

    rows.sort_by(|a, b| a.0.cmp(&b.0));

    let provider_w = rows.iter().map(|r| r.0.len()).max().unwrap_or(8).max(8);
    let kr_w = rows.iter().map(|r| r.1.len()).max().unwrap_or(8).max(8);
    let env_w = rows.iter().map(|r| r.2.len()).max().unwrap_or(8).max(8);

    println!(
        "{:width$}  {:kr_w$}  {:env_w$}",
        "PROVIDER",
        "KEYRING",
        "ENV VAR",
        width = provider_w,
        kr_w = kr_w,
        env_w = env_w,
    );
    println!(
        "{:-<width$}  {:-<kr_w$}  {:-<env_w$}",
        "",
        "",
        "",
        width = provider_w,
        kr_w = kr_w,
        env_w = env_w,
    );
    for (provider, kr, env) in &rows {
        println!(
            "{:width$}  {:kr_w$}  {:env_w$}",
            provider,
            kr,
            env,
            width = provider_w,
            kr_w = kr_w,
            env_w = env_w,
        );
    }
}

fn cmd_delete(provider: String, config: Option<PathBuf>) {
    let provider_config = match load_provider_config(config, &provider) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let service_name = match provider_config.api_key_service.as_deref() {
        Some(s) => s,
        None => {
            println!("Provider '{}' does not use an API key.", provider);
            return;
        }
    };

    match secrets::delete_key(service_name) {
        Ok(()) => println!("API key for '{}' deleted from keyring.", provider),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
