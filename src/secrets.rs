//! API key resolution and management via the OS keyring or environment variables.

const KEYRING_APP_NAME: &str = "llm-cascade";

/// Resolves an API key by trying the OS keyring first, then falling back to an environment variable.
///
/// When the `keyring` feature is disabled, only the environment variable is checked.
pub fn resolve_api_key(service_name: &str, env_var: &str) -> Result<String, String> {
    #[cfg(feature = "keyring")]
    {
        match keyring::Entry::new(KEYRING_APP_NAME, service_name)
            .and_then(|entry| entry.get_password())
        {
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
            keyring_info, env_var
        )
    })
}

pub fn set_key(service_name: &str, key: &str) -> Result<(), String> {
    #[cfg(feature = "keyring")]
    {
        let entry = keyring::Entry::new(KEYRING_APP_NAME, service_name).map_err(|e| {
            format!(
                "Failed to create keyring entry for '{}': {}",
                service_name, e
            )
        })?;
        entry
            .set_password(key)
            .map_err(|e| format!("Failed to store key for '{}': {}", service_name, e))?;
        tracing::debug!("API key for '{}' stored in keyring", service_name);
        Ok(())
    }

    #[cfg(not(feature = "keyring"))]
    {
        let _ = (service_name, key);
        Err(
            "Keyring support is disabled at compile time. Rebuild with --features keyring."
                .to_string(),
        )
    }
}

pub fn get_key(service_name: &str) -> Result<String, String> {
    #[cfg(feature = "keyring")]
    {
        let entry = keyring::Entry::new(KEYRING_APP_NAME, service_name).map_err(|e| {
            format!(
                "Failed to create keyring entry for '{}': {}",
                service_name, e
            )
        })?;
        entry
            .get_password()
            .map_err(|e| format!("No key found for '{}' in keyring: {}", service_name, e))
    }

    #[cfg(not(feature = "keyring"))]
    {
        let _ = service_name;
        Err(
            "Keyring support is disabled at compile time. Rebuild with --features keyring."
                .to_string(),
        )
    }
}

pub fn delete_key(service_name: &str) -> Result<(), String> {
    #[cfg(feature = "keyring")]
    {
        let entry = keyring::Entry::new(KEYRING_APP_NAME, service_name).map_err(|e| {
            format!(
                "Failed to create keyring entry for '{}': {}",
                service_name, e
            )
        })?;
        entry
            .delete_credential()
            .map_err(|e| format!("Failed to delete key for '{}': {}", service_name, e))?;
        tracing::debug!("API key for '{}' deleted from keyring", service_name);
        Ok(())
    }

    #[cfg(not(feature = "keyring"))]
    {
        let _ = service_name;
        Err(
            "Keyring support is disabled at compile time. Rebuild with --features keyring."
                .to_string(),
        )
    }
}

pub fn has_key(service_name: &str) -> bool {
    #[cfg(feature = "keyring")]
    {
        keyring::Entry::new(KEYRING_APP_NAME, service_name)
            .and_then(|entry| entry.get_password())
            .is_ok()
    }

    #[cfg(not(feature = "keyring"))]
    {
        let _ = service_name;
        false
    }
}

pub fn mask_key(key: &str) -> String {
    let len = key.len();
    if len <= 8 {
        return "*".repeat(len);
    }
    format!("{}...{}", &key[..4], &key[len - 4..])
}
