use std::io::Write;
use std::path::PathBuf;

use chrono::Local;
use serde::Serialize;

use crate::config::expand_tilde;
use crate::models::Conversation;

#[derive(Serialize)]
struct FailedConversationEnvelope<'a> {
    cascade_name: &'a str,
    timestamp: String,
    conversation: &'a Conversation,
}

pub fn save_failed_conversation(
    conversation: &Conversation,
    dir: &str,
    cascade_name: &str,
) -> PathBuf {
    let expanded_dir = expand_tilde(dir);
    std::fs::create_dir_all(&expanded_dir).ok();

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.json", cascade_name, timestamp);
    let filepath = expanded_dir.join(&filename);

    let envelope = FailedConversationEnvelope {
        cascade_name,
        timestamp: Local::now().to_rfc3339(),
        conversation,
    };

    if let Ok(mut file) = std::fs::File::create(&filepath) {
        let content = serde_json::to_string_pretty(&envelope).unwrap_or_default();
        file.write_all(content.as_bytes()).ok();
    }

    filepath
}
