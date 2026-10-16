use serde::Serialize;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct WaybarAgyOutput {
    pub text: String,
    pub tooltip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

fn get_app_dir() -> PathBuf {
    if let Ok(app_dir) = env::var("ANTIGRAVITY_APP_DATA_DIR") {
        PathBuf::from(app_dir)
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".gemini").join("antigravity-cli")
    }
}

pub fn clean_model_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "AGY".to_string();
    }

    // Strip out (Medium), (High), (Low) etc.
    let base = if let Some(idx) = trimmed.find('(') {
        trimmed[..idx].trim()
    } else {
        trimmed
    };

    // Strip prefixes like "Gemini " to make it compact in Waybar
    if let Some(rest) = base.strip_prefix("Gemini ") {
        rest.to_string()
    } else {
        base.to_string()
    }
}

pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

pub fn extract_quota_reset(log_content: &str) -> Option<String> {
    for line in log_content.lines().rev() {
        if line.contains("RESOURCE_EXHAUSTED") || line.contains("Individual quota reached") {
            if let Some(idx) = line.find("Resets in ") {
                let rest = &line[idx + "Resets in ".len()..];
                let reset_time = rest.split(['.', ' ']).next().unwrap_or("soon");
                return Some(reset_time.to_string());
            }
            return Some("exceeded".to_string());
        }
    }
    None
}

fn read_last_log_chunk(log_path: &Path, max_bytes: u64) -> Option<String> {
    let mut file = File::open(log_path).ok()?;
    let metadata = file.metadata().ok()?;
    let len = metadata.len();
    if len > max_bytes {
        file.seek(SeekFrom::Start(len - max_bytes)).ok()?;
    }
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn get_active_conversation_id(app_dir: &Path) -> Option<String> {
    // 1. Try history.jsonl (last non-empty line)
    let history_path = app_dir.join("history.jsonl");
    if let Ok(content) = fs::read_to_string(&history_path) {
        for line in content.lines().rev() {
            let line = line.trim();
            if !line.is_empty()
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(conv_id) = v.get("conversationId").and_then(|c| c.as_str())
            {
                return Some(conv_id.to_string());
            }
        }
    }

    // 2. Try last_conversations.json
    let last_conv_path = app_dir.join("cache").join("last_conversations.json");
    if let Ok(content) = fs::read_to_string(&last_conv_path)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(obj) = v.as_object()
        && let Some((_, val)) = obj.iter().next()
        && let Some(conv_id) = val.as_str()
    {
        return Some(conv_id.to_string());
    }

    None
}

struct TokenStats {
    input_tokens: u64,
    output_tokens: u64,
    cache_tokens: u64,
}

fn read_token_stats(app_dir: &Path, conv_id: &str) -> TokenStats {
    let mut stats = TokenStats {
        input_tokens: 0,
        output_tokens: 0,
        cache_tokens: 0,
    };

    let transcript_path = app_dir
        .join("brain")
        .join(conv_id)
        .join(".system_generated")
        .join("logs")
        .join("transcript.jsonl");

    if let Ok(content) = fs::read_to_string(transcript_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(in_tok) = v.get("input_tokens").and_then(|t| t.as_u64()) {
                    stats.input_tokens += in_tok;
                }
                if let Some(out_tok) = v.get("output_tokens").and_then(|t| t.as_u64()) {
                    stats.output_tokens += out_tok;
                }
                if let Some(cache_tok) = v.get("cache_read_tokens").and_then(|t| t.as_u64()) {
                    stats.cache_tokens += cache_tok;
                }
            }
        }
    }

    stats
}

pub fn handle_agy(_args: &[String]) {
    let app_dir = get_app_dir();
    let settings_path = app_dir.join("settings.json");

    let mut full_model = "Antigravity".to_string();
    if let Ok(content) = fs::read_to_string(&settings_path)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(m) = v.get("model").and_then(|s| s.as_str())
    {
        full_model = m.to_string();
    }

    let short_model = clean_model_name(&full_model);

    // Check quota / status from cli.log
    let cli_log_path = app_dir.join("cli.log");
    let quota_reset = if cli_log_path.exists() {
        read_last_log_chunk(&cli_log_path, 8192).and_then(|chunk| extract_quota_reset(&chunk))
    } else {
        None
    };

    let conv_id = get_active_conversation_id(&app_dir);
    let stats = if let Some(ref cid) = conv_id {
        read_token_stats(&app_dir, cid)
    } else {
        TokenStats {
            input_tokens: 0,
            output_tokens: 0,
            cache_tokens: 0,
        }
    };

    let total_session_tokens = stats.input_tokens + stats.output_tokens;

    let (text, class, quota_str) = match quota_reset {
        Some(reset_time) => (
            format!("󰚩 {} 󰅚 Quota ({})", short_model, reset_time),
            Some("critical".to_string()),
            format!("Limited: Resets in {}", reset_time),
        ),
        None => {
            if total_session_tokens > 0 {
                (
                    format!("󰚩 {} · {}", short_model, format_tokens(total_session_tokens)),
                    Some("normal".to_string()),
                    "Active (Normal)".to_string(),
                )
            } else {
                (
                    format!("󰚩 {}", short_model),
                    Some("normal".to_string()),
                    "Active (Normal)".to_string(),
                )
            }
        }
    };

    let tooltip = format!(
        "<b>Antigravity CLI</b>\n\
         Model: {}\n\
         Quota: {}\n\
         Session Usage:\n\
           • In: {}\n\
           • Out: {}\n\
           • Cache: {}\n\
         Conversation: {}",
        full_model,
        quota_str,
        format_tokens(stats.input_tokens),
        format_tokens(stats.output_tokens),
        format_tokens(stats.cache_tokens),
        conv_id.as_deref().unwrap_or("none")
    );

    let output = WaybarAgyOutput {
        text,
        tooltip,
        class,
    };

    if let Ok(json) = serde_json::to_string(&output) {
        println!("{}", json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_model_name() {
        assert_eq!(clean_model_name("Gemini 3.8 Flash (Medium)"), "3.8 Flash");
        assert_eq!(clean_model_name("Gemini 4 Argon (High)"), "4 Argon");
        assert_eq!(clean_model_name("Claude Sonnet 3.7 (Medium)"), "Claude Sonnet 3.7");
        assert_eq!(clean_model_name(""), "AGY");
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(450), "450");
        assert_eq!(format_tokens(1_500), "1k");
        assert_eq!(format_tokens(45_000), "45k");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_extract_quota_reset() {
        let sample = "agent executor error: generating and executing: RESOURCE_EXHAUSTED (code 429): Individual quota reached. Please upgrade your subscription to increase your limits. Resets in 166h45m57s.\nSome other info";
        assert_eq!(extract_quota_reset(sample), Some("166h45m57s".to_string()));

        let normal = "info: all good";
        assert_eq!(extract_quota_reset(normal), None);
    }
}
