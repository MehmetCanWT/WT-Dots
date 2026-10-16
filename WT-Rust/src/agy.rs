use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct WaybarAgyOutput {
    pub text: String,
    pub tooltip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct CachedUsage {
    timestamp: u64,
    tokens_5h: u64,
    tokens_7d: u64,
    earliest_5h_ts: u64,
    earliest_7d_ts: u64,
}

fn get_app_dir() -> PathBuf {
    if let Ok(app_dir) = env::var("ANTIGRAVITY_APP_DATA_DIR") {
        PathBuf::from(app_dir)
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".gemini").join("antigravity-cli")
    }
}

pub fn is_agy_running() -> Option<u32> {
    let proc_dir = fs::read_dir("/proc").ok()?;
    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<u32>() {
            let comm_path = entry.path().join("comm");
            if let Ok(comm) = fs::read_to_string(comm_path)
                && comm.trim() == "agy"
            {
                return Some(pid);
            }
        }
    }
    None
}

pub fn clean_model_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "AGY".to_string();
    }

    let base = if let Some(idx) = trimmed.find('(') {
        trimmed[..idx].trim()
    } else {
        trimmed
    };

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

pub fn format_duration_tr(seconds: u64) -> String {
    if seconds >= 86400 {
        format!("{}g {}s", seconds / 86400, (seconds % 86400) / 3600)
    } else if seconds >= 3600 {
        format!("{}s {}d", seconds / 3600, (seconds % 3600) / 60)
    } else {
        format!("{}d", (seconds / 60).max(1))
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

fn parse_iso8601_to_unix(s: &str) -> Option<u64> {
    // Expected format: YYYY-MM-DDTHH:MM:SSZ
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let parts: Vec<&str> = s[..19].split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let dparts: Vec<u64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let tparts: Vec<u64> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();
    if dparts.len() != 3 || tparts.len() != 3 {
        return None;
    }

    let year = dparts[0];
    let month = dparts[1];
    let day = dparts[2];
    let hour = tparts[0];
    let minute = tparts[1];
    let second = tparts[2];

    // Compute days since 1970-01-01
    let mut y = year as i64;
    let m = month as i64;
    let d = day as i64;
    if m <= 2 {
        y -= 1;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as u64 + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe as i64 - 719468;

    if days < 0 {
        return None;
    }
    let secs = (days as u64) * 86400 + hour * 3600 + minute * 60 + second;
    Some(secs)
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

struct SessionTokens {
    input: u64,
    output: u64,
    cache: u64,
}

fn read_session_tokens(app_dir: &Path, conv_id: &str) -> SessionTokens {
    let mut stats = SessionTokens {
        input: 0,
        output: 0,
        cache: 0,
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
                    stats.input += in_tok;
                }
                if let Some(out_tok) = v.get("output_tokens").and_then(|t| t.as_u64()) {
                    stats.output += out_tok;
                }
                if let Some(cache_tok) = v.get("cache_read_tokens").and_then(|t| t.as_u64()) {
                    stats.cache += cache_tok;
                }
            }
        }
    }

    stats
}

fn get_or_update_usage_cache(app_dir: &Path, now_secs: u64) -> CachedUsage {
    let cache_path = Path::new("/tmp/wt-rust-agy-cache.json");

    if let Ok(content) = fs::read_to_string(cache_path)
        && let Ok(cached) = serde_json::from_str::<CachedUsage>(&content)
        && now_secs.saturating_sub(cached.timestamp) < 20
    {
        return cached;
    }

    let mut usage = CachedUsage {
        timestamp: now_secs,
        tokens_5h: 0,
        tokens_7d: 0,
        earliest_5h_ts: now_secs,
        earliest_7d_ts: now_secs,
    };

    let brain_dir = app_dir.join("brain");
    if let Ok(entries) = fs::read_dir(brain_dir) {
        let five_hours_ago = now_secs.saturating_sub(5 * 3600);
        let seven_days_ago = now_secs.saturating_sub(7 * 86400);

        for entry in entries.flatten() {
            let tpath = entry
                .path()
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");

            if !tpath.exists() {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&tpath) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                        let created_at = v.get("created_at").and_then(|c| c.as_str());
                        if let Some(cat) = created_at
                            && let Some(ts) = parse_iso8601_to_unix(cat)
                        {
                            let toks = v.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0)
                                + v.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0);

                            if ts >= five_hours_ago {
                                usage.tokens_5h += toks;
                                if ts < usage.earliest_5h_ts {
                                    usage.earliest_5h_ts = ts;
                                }
                            }
                            if ts >= seven_days_ago {
                                usage.tokens_7d += toks;
                                if ts < usage.earliest_7d_ts {
                                    usage.earliest_7d_ts = ts;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Ok(json) = serde_json::to_string(&usage) {
        let _ = fs::write(cache_path, json);
    }

    usage
}

pub fn handle_agy(_args: &[String]) {
    let agy_pid = is_agy_running();
    let app_dir = get_app_dir();
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    // If agy is not running, output offline indicator
    let Some(pid) = agy_pid else {
        let output = WaybarAgyOutput {
            text: "󰚩 Kapalı".to_string(),
            tooltip: "<b>Antigravity CLI</b>\nDurum: Çalışmıyor (İnaktif)".to_string(),
            class: Some("offline".to_string()),
        };
        if let Ok(json) = serde_json::to_string(&output) {
            println!("{}", json);
        }
        return;
    };

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

    let usage = get_or_update_usage_cache(&app_dir, now_secs);

    // Model quota capacity pools (tokens)
    // 1. Gemini 3.8 Flash (En Yüksek Limit)
    let flash_cap_5h: u64 = 10_000_000;
    let flash_cap_7d: u64 = 50_000_000;
    let flash_rem_5h = 100u64.saturating_sub((usage.tokens_5h * 100 / flash_cap_5h).min(100));
    let flash_rem_7d = 100u64.saturating_sub((usage.tokens_7d * 100 / flash_cap_7d).min(100));

    // 2. Gemini 4 Argon (Orta Limit)
    let argon_cap_5h: u64 = 4_000_000;
    let argon_cap_7d: u64 = 20_000_000;
    let argon_rem_5h = 100u64.saturating_sub((usage.tokens_5h * 100 / argon_cap_5h).min(100));
    let argon_rem_7d = 100u64.saturating_sub((usage.tokens_7d * 100 / argon_cap_7d).min(100));

    // 3. Claude (Tüm Modeller Ortak Limit)
    let claude_cap_5h: u64 = 2_000_000;
    let claude_cap_7d: u64 = 10_000_000;
    let claude_rem_5h = 100u64.saturating_sub((usage.tokens_5h * 100 / claude_cap_5h).min(100));
    let claude_rem_7d = 100u64.saturating_sub((usage.tokens_7d * 100 / claude_cap_7d).min(100));

    // Reset durations
    let reset_5h_secs = (usage.earliest_5h_ts + 5 * 3600).saturating_sub(now_secs);
    let reset_5h_str = format_duration_tr(reset_5h_secs.max(60));

    let reset_7d_str = if let Some(ref r) = quota_reset {
        r.clone()
    } else {
        let secs = (usage.earliest_7d_ts + 7 * 86400).saturating_sub(now_secs);
        format_duration_tr(secs.max(3600))
    };

    // Determine active model quota for bar display
    let (active_rem_5h, active_rem_7d) = if full_model.contains("Argon") {
        (argon_rem_5h, argon_rem_7d)
    } else if full_model.contains("Claude") {
        (claude_rem_5h, claude_rem_7d)
    } else {
        (flash_rem_5h, flash_rem_7d)
    };

    let conv_id = get_active_conversation_id(&app_dir);
    let session = if let Some(ref cid) = conv_id {
        read_session_tokens(&app_dir, cid)
    } else {
        SessionTokens {
            input: 0,
            output: 0,
            cache: 0,
        }
    };

    let (text, class) = match quota_reset {
        Some(ref reset_time) => (
            format!("󰚩 {} 󰅚 Limit: {}", short_model, reset_time),
            Some("critical".to_string()),
        ),
        None => (
            format!("󰚩 {} · 5s: %{} · H: %{}", short_model, active_rem_5h, active_rem_7d),
            Some("normal".to_string()),
        ),
    };

    let tooltip = format!(
        "<b>Antigravity CLI</b> (Aktif · PID {})\n\
         Aktif Model: {}\n\n\
         <b>Model Kalan Limitleri & Yenilenme:</b>\n\
         1. <b>Gemini 3.8 Flash</b> (En Yüksek Limit)\n\
            • 5 Saatlik : %{} kalan (Yenilenme: {})\n\
            • Haftalık  : %{} kalan (Yenilenme: {})\n\n\
         2. <b>Gemini 4 Argon</b> (Orta Limit)\n\
            • 5 Saatlik : %{} kalan (Yenilenme: {})\n\
            • Haftalık  : %{} kalan (Yenilenme: {})\n\n\
         3. <b>Claude</b> (Tüm Modeller Ortak Limit)\n\
            • 5 Saatlik : %{} kalan (Yenilenme: {})\n\
            • Haftalık  : %{} kalan (Yenilenme: {})\n\
         ────────────────────────────\n\
         Oturum Kullanımı:\n\
           • Girdi: {}  • Çıktı: {}\n\
           • Önbellek: {}\n\
         Konuşma: {}",
        pid,
        full_model,
        flash_rem_5h,
        reset_5h_str,
        flash_rem_7d,
        reset_7d_str,
        argon_rem_5h,
        reset_5h_str,
        argon_rem_7d,
        reset_7d_str,
        claude_rem_5h,
        reset_5h_str,
        claude_rem_7d,
        reset_7d_str,
        format_tokens(session.input),
        format_tokens(session.output),
        format_tokens(session.cache),
        conv_id.as_deref().unwrap_or("yok")
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
    fn test_format_duration_tr() {
        assert_eq!(format_duration_tr(120), "2d");
        assert_eq!(format_duration_tr(3660), "1s 1d");
        assert_eq!(format_duration_tr(90000), "1g 1s");
    }

    #[test]
    fn test_parse_iso8601() {
        let ts = parse_iso8601_to_unix("2026-10-16T12:00:00Z");
        assert!(ts.is_some());
        assert!(ts.unwrap() > 1700000000);
    }

    #[test]
    fn test_extract_quota_reset() {
        let sample = "agent executor error: generating and executing: RESOURCE_EXHAUSTED (code 429): Individual quota reached. Please upgrade your subscription to increase your limits. Resets in 166h45m57s.\nSome other info";
        assert_eq!(extract_quota_reset(sample), Some("166h45m57s".to_string()));

        let normal = "info: all good";
        assert_eq!(extract_quota_reset(normal), None);
    }
}
