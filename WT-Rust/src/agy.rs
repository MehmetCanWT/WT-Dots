use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct WaybarAgyOutput {
    pub text: String,
    pub tooltip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct QuotaResponse {
    pub command: Option<QuotaCommand>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct QuotaCommand {
    pub name: Option<String>,
    pub data: Option<QuotaData>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct QuotaData {
    pub groups: Option<Vec<QuotaGroup>>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct QuotaGroup {
    pub name: String,
    pub description: Option<String>,
    pub buckets: Option<Vec<QuotaBucket>>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct QuotaBucket {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub window: Option<String>,
    pub remaining_fraction: Option<f64>,
    pub reset_time: Option<String>,
    pub disabled: Option<bool>,
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

pub fn escape_pango(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn clean_model_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Gemini".to_string();
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

pub fn parse_iso8601_to_unix(s: &str) -> Option<u64> {
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
    Some((days as u64) * 86400 + hour * 3600 + minute * 60 + second)
}

fn format_reset_time_tr(iso_time: Option<&str>, now_secs: u64) -> String {
    let Some(iso) = iso_time else {
        return "Bilinmiyor".to_string();
    };
    if let Some(unix) = parse_iso8601_to_unix(iso) {
        if unix > now_secs {
            return format_duration_tr(unix - now_secs);
        }
        return "Yenileniyor".to_string();
    }
    "Bilinmiyor".to_string()
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

fn load_quota_summary() -> Option<QuotaResponse> {
    let cache_path = Path::new("/tmp/wt-rust-agy-quota.json");
    let lock_path = Path::new("/tmp/wt-rust-agy-quota.lock");

    let needs_refresh = if let Ok(meta) = fs::metadata(cache_path) {
        if let Ok(modified) = meta.modified() {
            modified.elapsed().unwrap_or(Duration::from_secs(999)) > Duration::from_secs(60)
        } else {
            true
        }
    } else {
        true
    };

    // If cache is stale or missing, trigger background refresh
    if needs_refresh {
        let lock_stale = if let Ok(lmeta) = fs::metadata(lock_path) {
            lmeta.modified().ok().and_then(|m| m.elapsed().ok()).unwrap_or(Duration::from_secs(999)) > Duration::from_secs(30)
        } else {
            true
        };

        if lock_stale {
            let _ = fs::write(lock_path, "1");
            thread::spawn(|| {
                if let Ok(output) = Command::new("agy")
                    .args(["-p", "/quota", "--output-format", "json"])
                    .output()
                    && output.status.success()
                    && !output.stdout.is_empty()
                {
                    let _ = fs::write("/tmp/wt-rust-agy-quota.json", &output.stdout);
                }
                let _ = fs::remove_file("/tmp/wt-rust-agy-quota.lock");
            });
        }
    }

    if let Ok(mut f) = File::open(cache_path) {
        let mut buf = String::new();
        if f.read_to_string(&mut buf).is_ok()
            && let Ok(parsed) = serde_json::from_str::<QuotaResponse>(&buf)
        {
            return Some(parsed);
        }
    }

    None
}

pub fn handle_agy(_args: &[String]) {
    let agy_pid = is_agy_running();
    let app_dir = get_app_dir();
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    let settings_path = app_dir.join("settings.json");
    let mut full_model = "Gemini".to_string();
    if let Ok(content) = fs::read_to_string(&settings_path)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(m) = v.get("model").and_then(|s| s.as_str())
    {
        full_model = m.to_string();
    }

    let short_model = clean_model_name(&full_model);

    // Rule: If agy is not running, show ONLY "Gemini". If running, show active model name!
    let (text, class, menu_title) = match agy_pid {
        Some(pid) => (
            format!("󰚩 {}", short_model),
            Some("normal".to_string()),
            format!("<b>{}</b> (Aktif · PID {})", escape_pango(&full_model), pid),
        ),
        None => (
            "󰚩 Gemini".to_string(),
            Some("offline".to_string()),
            "<b>Gemini</b>".to_string(),
        ),
    };

    let quota_data = load_quota_summary();
    let groups = quota_data
        .and_then(|r| r.command)
        .and_then(|c| c.data)
        .and_then(|d| d.groups)
        .unwrap_or_default();

    // Group 1: Gemini Models (Highest Limit - Flash/Pro)
    let gemini_grp = groups.iter().find(|g| g.name.contains("Gemini Models"));
    let mut gemini_5h_str = "Kullanılabilir".to_string();
    let mut gemini_weekly_str = "Kullanılabilir".to_string();

    if let Some(grp) = gemini_grp && let Some(ref buckets) = grp.buckets {
        for b in buckets {
            let win = b.window.as_deref().unwrap_or("");
            let frac = b.remaining_fraction.unwrap_or(1.0);
            let pct = (frac * 100.0).round() as u64;
            let reset_dur = format_reset_time_tr(b.reset_time.as_deref(), now_secs);

            if win == "5h" {
                gemini_5h_str = format!("%{} kalan (Yenilenme: {})", pct, reset_dur);
            } else if win == "weekly" {
                gemini_weekly_str = format!("%{} kalan (Yenilenme: {})", pct, reset_dur);
            }
        }
    }

    // Group 2: Gemini 4 Argon (Medium Limit)
    let argon_grp = groups.iter().find(|g| g.name.contains("Argon"));
    let mut argon_weekly_str = "Kullanılabilir".to_string();

    if let Some(grp) = argon_grp && let Some(ref buckets) = grp.buckets {
        for b in buckets {
            let frac = b.remaining_fraction.unwrap_or(1.0);
            let pct = (frac * 100.0).round() as u64;
            let reset_dur = format_reset_time_tr(b.reset_time.as_deref(), now_secs);

            if pct == 0 {
                argon_weekly_str = format!("%0 · Limit Doldu (Yenilenme: {})", reset_dur);
            } else {
                argon_weekly_str = format!("%{} kalan (Yenilenme: {})", pct, reset_dur);
            }
        }
    }

    // Group 3: Claude (Tüm Modeller Ortak Limit)
    let claude_grp = groups.iter().find(|g| g.name.contains("Claude") || g.name.contains("3p"));
    let mut claude_5h_str = "Kullanılabilir".to_string();
    let mut claude_weekly_str = "Kullanılabilir".to_string();

    if let Some(grp) = claude_grp && let Some(ref buckets) = grp.buckets {
        for b in buckets {
            let win = b.window.as_deref().unwrap_or("");
            let frac = b.remaining_fraction.unwrap_or(0.0);
            let pct = (frac * 100.0).round() as u64;
            let reset_dur = format_reset_time_tr(b.reset_time.as_deref(), now_secs);
            let is_disabled = b.disabled.unwrap_or(false);

            if win == "5h" {
                if is_disabled {
                    claude_5h_str = "Devre Dışı (Haftalık limit aşıldı)".to_string();
                } else {
                    claude_5h_str = format!("%{} kalan (Yenilenme: {})", pct, reset_dur);
                }
            } else if win == "weekly" {
                if pct == 0 {
                    claude_weekly_str = format!("%0 · Limit Doldu (Yenilenme: {})", reset_dur);
                } else {
                    claude_weekly_str = format!("%{} kalan (Yenilenme: {})", pct, reset_dur);
                }
            }
        }
    }

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

    let tooltip = format!(
        "{}\n\
         ────────────────────────────\n\
         <b>Model Kotaları ve Yenilenme Süreleri:</b>\n\
         1. <b>Gemini Modelleri</b> (En Yüksek Limit · Flash / Pro)\n\
            • 5 Saatlik : {}\n\
            • Haftalık  : {}\n\n\
         2. <b>Gemini 4 Argon</b> (Orta Limit)\n\
            • Haftalık  : {}\n\n\
         3. <b>Claude</b> (Tüm Modeller Ortak Limit)\n\
            • Haftalık  : {}\n\
            • 5 Saatlik : {}\n\
         ────────────────────────────\n\
         Oturum Kullanımı:\n\
            • Girdi: {}  • Çıktı: {}\n\
            • Önbellek: {}\n\
         Konuşma: {}",
        menu_title,
        escape_pango(&gemini_5h_str),
        escape_pango(&gemini_weekly_str),
        escape_pango(&argon_weekly_str),
        escape_pango(&claude_weekly_str),
        escape_pango(&claude_5h_str),
        format_tokens(session.input),
        format_tokens(session.output),
        format_tokens(session.cache),
        escape_pango(conv_id.as_deref().unwrap_or("Yok"))
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
        assert_eq!(clean_model_name(""), "Gemini");
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
}
