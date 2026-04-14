use std::path::PathBuf;

use clap::Parser;

use llm_cascade::{config, db, run_cascade, Conversation};

#[derive(Parser, Debug)]
#[command(name = "llm-cascade", about = "Cascading LLM inference across multiple providers")]
struct Cli {
    /// Name of the cascade to use (defined in config)
    #[arg(short = 'C', long)]
    cascade: String,

    /// Text prompt to send
    #[arg(short, long, conflicts_with = "file")]
    prompt: Option<String>,

    /// Path to a JSON file containing the conversation
    #[arg(short, long, conflicts_with = "prompt")]
    file: Option<PathBuf>,

    /// Path to config file (default: ~/.config/llm-cascade/config.toml)
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let config_path = cli.config.unwrap_or_else(config::default_config_path);
    let app_config = match config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            std::process::exit(1);
        }
    };

    let conn = match db::init_db(&app_config.database.path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error initializing database: {}", e);
            std::process::exit(1);
        }
    };

    let conversation = if let Some(file_path) = cli.file {
        match std::fs::read_to_string(&file_path) {
            Ok(content) => match serde_json::from_str::<Conversation>(&content) {
                Ok(conv) => conv,
                Err(e) => {
                    eprintln!("Error parsing conversation JSON from '{}': {}", file_path.display(), e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error reading file '{}': {}", file_path.display(), e);
                std::process::exit(1);
            }
        }
    } else if let Some(prompt) = cli.prompt {
        Conversation::single_user_prompt(prompt)
    } else {
        eprintln!("Error: either --prompt or --file must be provided");
        std::process::exit(1);
    };

    match run_cascade(&cli.cascade, &conversation, &app_config, &conn).await {
        Ok(response) => {
            let has_tool_calls = response.content.iter().any(|b| matches!(b, llm_cascade::ContentBlock::ToolCall { .. }));
            if has_tool_calls {
                let output = serde_json::to_string_pretty(&response).unwrap_or_default();
                println!("{}", output);
            } else {
                println!("{}", response.text_only());
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
