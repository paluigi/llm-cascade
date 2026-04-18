use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use clap::Args;

use llm_cascade::config::{self, DatabaseConfig, FailureConfig, ProviderConfig, ProviderType};
use llm_cascade::db;

use super::key;

#[derive(Args, Debug)]
pub struct SetupArgs {
    /// Path to config file (default: ~/.config/llm-cascade/config.toml)
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Run interactive setup wizard
    #[arg(short, long)]
    pub interactive: bool,
}

pub fn execute(args: SetupArgs) {
    let config_path = args.config.unwrap_or_else(config::default_config_path);
    let config_dir = config_path
        .parent()
        .expect("Config path should have a parent directory");

    if config_path.exists() {
        eprintln!("Config file already exists at '{}'.", config_path.display());
        eprintln!("Remove or rename it before running setup.");
        std::process::exit(1);
    }

    std::fs::create_dir_all(config_dir).unwrap_or_else(|e| {
        eprintln!(
            "Failed to create config directory '{}': {}",
            config_dir.display(),
            e
        );
        std::process::exit(1);
    });

    if args.interactive {
        run_interactive(&config_path);
    } else {
        run_default(&config_path);
    }

    match config::load_config(&config_path) {
        Ok(app_config) => {
            if let Err(e) = db::init_db(&app_config.database.path) {
                eprintln!("Warning: Failed to initialize database: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to load config for DB init: {}", e);
        }
    }

    println!();
    println!(
        "Setup complete. Edit '{}' to customize providers and cascades.",
        config_path.display()
    );
    println!("Use 'llm-cascade key set <provider>' to store API keys.");
}

fn run_default(config_path: &PathBuf) {
    let example_content = include_str!("../../config.example.toml");
    std::fs::write(config_path, example_content).unwrap_or_else(|e| {
        eprintln!(
            "Failed to write config file '{}': {}",
            config_path.display(),
            e
        );
        std::process::exit(1);
    });

    let data_dir = config::expand_tilde("~/.local/share/llm-cascade");
    let failed_dir = data_dir.join("failed_prompts");
    std::fs::create_dir_all(&failed_dir).unwrap_or_else(|e| {
        eprintln!(
            "Failed to create data directory '{}': {}",
            failed_dir.display(),
            e
        );
        std::process::exit(1);
    });

    println!("Created config: {}", config_path.display());
    println!("Created data directory: {}", data_dir.display());

    offer_set_keys(config_path);
}

fn run_interactive(config_path: &PathBuf) {
    println!("=== llm-cascade Interactive Setup ===\n");

    let mut providers: HashMap<String, ProviderConfig> = HashMap::new();
    let mut cascades: HashMap<String, llm_cascade::config::CascadeConfig> = HashMap::new();

    println!("Select providers to configure (comma-separated numbers, or 'all'):");
    println!("  1) openai     - OpenAI (gpt-4o, gpt-4o-mini, ...)");
    println!("  2) anthropic  - Anthropic (claude-sonnet-4, ...)");
    println!("  3) gemini     - Google Gemini (gemini-2.0-flash, ...)");
    println!("  4) groq       - Groq (openai-compatible)");
    println!("  5) ollama     - Ollama (local, no API key)");

    loop {
        eprint!("Providers [1-5]: ");
        io::stderr().flush().ok();
        let input = read_line();
        let input = input.trim();

        if input == "all" || input == "a" {
            providers.insert(
                "openai".into(),
                ProviderConfig {
                    r#type: ProviderType::Openai,
                    api_key_service: Some("openai".into()),
                    api_key_env: Some("OPENAI_API_KEY".into()),
                    base_url: None,
                },
            );
            providers.insert(
                "anthropic".into(),
                ProviderConfig {
                    r#type: ProviderType::Anthropic,
                    api_key_service: Some("anthropic".into()),
                    api_key_env: Some("ANTHROPIC_API_KEY".into()),
                    base_url: None,
                },
            );
            providers.insert(
                "gemini".into(),
                ProviderConfig {
                    r#type: ProviderType::Gemini,
                    api_key_service: Some("gemini".into()),
                    api_key_env: Some("GOOGLE_API_KEY".into()),
                    base_url: None,
                },
            );
            providers.insert(
                "groq".into(),
                ProviderConfig {
                    r#type: ProviderType::Openai,
                    api_key_service: Some("groq".into()),
                    api_key_env: Some("GROQ_API_KEY".into()),
                    base_url: Some("https://api.groq.com/openai/v1".into()),
                },
            );
            providers.insert(
                "ollama".into(),
                ProviderConfig {
                    r#type: ProviderType::Ollama,
                    api_key_service: None,
                    api_key_env: None,
                    base_url: Some("http://localhost:11434".into()),
                },
            );
            break;
        }

        let mut selected = Vec::new();
        for part in input.split(',') {
            match part.trim() {
                "1" | "openai" => {
                    providers.insert(
                        "openai".into(),
                        ProviderConfig {
                            r#type: ProviderType::Openai,
                            api_key_service: Some("openai".into()),
                            api_key_env: Some("OPENAI_API_KEY".into()),
                            base_url: None,
                        },
                    );
                    selected.push("openai");
                }
                "2" | "anthropic" => {
                    providers.insert(
                        "anthropic".into(),
                        ProviderConfig {
                            r#type: ProviderType::Anthropic,
                            api_key_service: Some("anthropic".into()),
                            api_key_env: Some("ANTHROPIC_API_KEY".into()),
                            base_url: None,
                        },
                    );
                    selected.push("anthropic");
                }
                "3" | "gemini" => {
                    providers.insert(
                        "gemini".into(),
                        ProviderConfig {
                            r#type: ProviderType::Gemini,
                            api_key_service: Some("gemini".into()),
                            api_key_env: Some("GOOGLE_API_KEY".into()),
                            base_url: None,
                        },
                    );
                    selected.push("gemini");
                }
                "4" | "groq" => {
                    providers.insert(
                        "groq".into(),
                        ProviderConfig {
                            r#type: ProviderType::Openai,
                            api_key_service: Some("groq".into()),
                            api_key_env: Some("GROQ_API_KEY".into()),
                            base_url: Some("https://api.groq.com/openai/v1".into()),
                        },
                    );
                    selected.push("groq");
                }
                "5" | "ollama" => {
                    providers.insert(
                        "ollama".into(),
                        ProviderConfig {
                            r#type: ProviderType::Ollama,
                            api_key_service: None,
                            api_key_env: None,
                            base_url: Some("http://localhost:11434".into()),
                        },
                    );
                    selected.push("ollama");
                }
                "" => continue,
                other => {
                    eprintln!("Unknown option: '{}'", other);
                    continue;
                }
            }
        }

        if !selected.is_empty() {
            break;
        }
    }

    let mut add_custom = true;
    while add_custom {
        eprint!("Add a custom OpenAI-compatible provider? [y/N]: ");
        io::stderr().flush().ok();
        let input = read_line();
        if input.trim().to_lowercase() != "y" {
            add_custom = false;
            continue;
        }

        eprint!("  Provider name: ");
        io::stderr().flush().ok();
        let name = read_line().trim().to_string();
        if name.is_empty() || providers.contains_key(&name) {
            eprintln!("  Invalid or duplicate name.");
            continue;
        }

        eprint!("  Base URL (e.g. https://api.example.com/v1): ");
        io::stderr().flush().ok();
        let base_url = read_line().trim().to_string();
        if base_url.is_empty() {
            eprintln!("  Base URL is required.");
            continue;
        }

        let service_name = name.clone();
        let env_var = name.to_uppercase().replace('-', "_") + "_API_KEY";
        providers.insert(
            name.clone(),
            ProviderConfig {
                r#type: ProviderType::Openai,
                api_key_service: Some(service_name),
                api_key_env: Some(env_var),
                base_url: Some(base_url),
            },
        );
        println!("  Added provider '{}'.", name);
    }

    if providers.is_empty() {
        eprintln!("No providers selected. Aborting.");
        std::process::exit(1);
    }

    println!();
    println!("Define cascades (ordered provider/model lists).");
    println!("Leave cascade name empty to finish.");

    loop {
        eprint!("Cascade name (or Enter to finish): ");
        io::stderr().flush().ok();
        let cascade_name = read_line().trim().to_string();
        if cascade_name.is_empty() {
            break;
        }

        if cascades.contains_key(&cascade_name) {
            eprintln!("  Cascade '{}' already exists. Skipping.", cascade_name);
            continue;
        }

        let mut entries = Vec::new();
        println!(
            "  Add entries for cascade '{}' (provider/model pairs).",
            cascade_name
        );
        println!(
            "  Available providers: {}",
            providers.keys().cloned().collect::<Vec<_>>().join(", ")
        );

        loop {
            eprint!("    Entry (provider:model, or Enter to finish): ");
            io::stderr().flush().ok();
            let entry_input = read_line().trim().to_string();
            if entry_input.is_empty() {
                break;
            }

            let parts: Vec<&str> = entry_input.splitn(2, ':').collect();
            if parts.len() != 2 {
                eprintln!("    Format: provider:model (e.g. openai:gpt-4o)");
                continue;
            }

            let prov = parts[0].trim();
            let model = parts[1].trim();

            if !providers.contains_key(prov) {
                eprintln!("    Provider '{}' not configured.", prov);
                continue;
            }

            entries.push(llm_cascade::config::CascadeEntry {
                provider: prov.to_string(),
                model: model.to_string(),
            });
            println!("    Added: {} / {}", prov, model);
        }

        if entries.is_empty() {
            eprintln!("  Cascade '{}' has no entries. Skipping.", cascade_name);
            continue;
        }

        cascades.insert(cascade_name, llm_cascade::config::CascadeConfig { entries });
    }

    let has_keyring_providers = providers.values().any(|p| p.api_key_service.is_some());

    let app_config = llm_cascade::config::AppConfig {
        providers,
        cascades,
        database: DatabaseConfig::default(),
        failure_persistence: FailureConfig::default(),
    };

    let toml_str = toml::to_string_pretty(&app_config).expect("Failed to serialize config");
    std::fs::write(config_path, &toml_str).unwrap_or_else(|e| {
        eprintln!(
            "Failed to write config file '{}': {}",
            config_path.display(),
            e
        );
        std::process::exit(1);
    });

    let data_dir = config::expand_tilde("~/.local/share/llm-cascade");
    let failed_dir = data_dir.join("failed_prompts");
    std::fs::create_dir_all(&failed_dir).unwrap_or_else(|e| {
        eprintln!(
            "Failed to create data directory '{}': {}",
            failed_dir.display(),
            e
        );
        std::process::exit(1);
    });

    println!();
    println!("Created config: {}", config_path.display());
    println!("Created data directory: {}", data_dir.display());

    if has_keyring_providers {
        offer_set_keys(config_path);
    }
}

fn offer_set_keys(config_path: &std::path::Path) {
    println!();
    eprint!("Would you like to set API keys now? [y/N]: ");
    io::stderr().flush().ok();
    let input = read_line();
    if input.trim().to_lowercase() != "y" {
        return;
    }

    let app_config = match config::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            return;
        }
    };

    let mut key_providers: Vec<String> = app_config
        .providers
        .iter()
        .filter(|(_, p)| p.api_key_service.is_some())
        .map(|(name, _)| name.clone())
        .collect();
    key_providers.sort();

    if key_providers.is_empty() {
        println!("No providers require API keys.");
        return;
    }

    println!();
    for provider_name in &key_providers {
        println!();
        match key::set_key_for_provider(Some(config_path.to_path_buf()), provider_name) {
            Ok(()) => {}
            Err(e) => eprintln!("  Skipped '{}': {}", provider_name, e),
        }
    }
}

fn read_line() -> String {
    let stdin = io::stdin();
    stdin
        .lock()
        .lines()
        .next()
        .unwrap_or(Ok(String::new()))
        .unwrap_or_default()
}
