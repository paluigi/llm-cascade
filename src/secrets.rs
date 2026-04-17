//! API key resolution from the OS keyring or environment variables.

/// Resolves an API key by trying the OS keyring first, then falling back to an environment variable.
///
/// When the `keyring` feature is disabled, only the environment variable is checked.
pub fn resolve_api_key(service_name: &str, env_var: &str) -> Result<String, String> {
    #[cfg(feature = "keyring")]
    {
        match keyring::Entry::new("llm-cascade", service_name).and_then(|entry| entry.get_password()) {
            Ok(key) => {
                tracing::debug!("API key for '{}' resolved from keyring", service_name);
                return Ok(key);
            }
            Err(_) => {
                tracing::debug!(
                    "Keyring unavailable for '{}', trying env var '{}'",
                    service_name,
                    env_var
                );
            }
        }
    }

    #[cfg(not(feature = "keyring"))]
    let _ = service_name;

    std::env::var(env_var).map_err(|_| {
        #[cfg(feature = "keyring")]
        let keyring_info = format!("keyring service '{}'", service_name);
        #[cfg(not(feature = "keyring"))]
        let keyring_info = String::from("keyring support disabled at compile time");
        format!(
            "API key not found: {} and env var '{}' are both unavailable",
            keyring_info,
            env_var
        )
    })
}
