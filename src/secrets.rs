use tracing;

pub fn resolve_api_key(service_name: &str, env_var: &str) -> Result<String, String> {
    match keyring::Entry::new("llm-cascade", service_name).and_then(|entry| entry.get_password()) {
        Ok(key) => {
            tracing::debug!("API key for '{}' resolved from keyring", service_name);
            Ok(key)
        }
        Err(_) => {
            tracing::debug!(
                "Keyring unavailable for '{}', trying env var '{}'",
                service_name,
                env_var
            );
            std::env::var(env_var).map_err(|_| {
                format!(
                    "API key not found: keyring service '{}' and env var '{}' are both unavailable",
                    service_name, env_var
                )
            })
        }
    }
}
