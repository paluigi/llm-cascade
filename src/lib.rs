//! Resilient, cascading LLM inference across multiple providers.
//!
//! `llm-cascade` provides automatic failover, circuit breaking, and retry cooldowns
//! when calling LLM APIs. Define ordered provider/model lists in a TOML config;
//! the library tries each entry in sequence, skipping those on cooldown, and returns
//! the first successful response.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use llm_cascade::{run_cascade, load_config, db, Conversation, Message};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = load_config(&"config.toml".into()).expect("config");
//!     let conn = db::init_db(&config.database.path).expect("db");
//!
//!     let conversation = Conversation::single_user_prompt("What is 2 + 2?");
//!     match run_cascade("my_cascade", &conversation, &config, &conn).await {
//!         Ok(response) => println!("{}", response.text_only()),
//!         Err(e) => eprintln!("All providers failed: {}", e),
//!     }
//! }
//! ```

pub mod cascade;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod persistence;
pub mod providers;
pub mod secrets;

pub use cascade::run_cascade;
pub use config::{load_config, AppConfig, CascadeConfig, CascadeEntry, DatabaseConfig, FailureConfig, ProviderConfig, ProviderType};
pub use error::{CascadeError, ProviderError};
pub use models::{ContentBlock, Conversation, LlmResponse, Message, MessageRole, ToolDefinition};
