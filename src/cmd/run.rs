use std::path::PathBuf;

use clap::Args;

use llm_cascade::{Conversation, config, db, run_cascade};

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Name of the cascade to use (defined in config)
    #[arg(short = 'C', long)]
    pub cascade: String,

    /// Text prompt to send
    #[arg(short, long, conflicts_with = "file")]
    pub prompt: Option<String>,

    /// Path to a JSON file containing the conversation
    #[arg(short, long, conflicts_with = "prompt")]
    pub file: Option<PathBuf>,

    /// Path to config file (default: ~/.config/llm-cascade/config.toml)
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

pub fn execute(args: RunArgs) {
    let config_path = args.config.unwrap_or_else(config::default_config_path);
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

    let conversation = if let Some(file_path) = args.file {
        match std::fs::read_to_string(&file_path) {
            Ok(content) => match serde_json::from_str::<Conversation>(&content) {
                Ok(conv) => conv,
                Err(e) => {
                    eprintln!(
                        "Error parsing conversation JSON from '{}': {}",
                        file_path.display(),
                        e
                    );
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error reading file '{}': {}", file_path.display(), e);
                std::process::exit(1);
            }
        }
    } else if let Some(prompt) = args.prompt {
        Conversation::single_user_prompt(prompt)
    } else {
        eprintln!("Error: either --prompt or --file must be provided");
        std::process::exit(1);
    };

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        match run_cascade(&args.cascade, &conversation, &app_config, &conn).await {
            Ok(response) => {
                let has_tool_calls = response
                    .content
                    .iter()
                    .any(|b| matches!(b, llm_cascade::ContentBlock::ToolCall { .. }));
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
    });
}
