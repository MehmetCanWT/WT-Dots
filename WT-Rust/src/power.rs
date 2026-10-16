use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
struct PowerOutput {
    text: String,
    tooltip: String,
    class: String,
    percentage: u32,
}

fn get_active_profile() -> String {
    let output = Command::new("busctl")
        .args([
            "get-property",
            "net.hadess.PowerProfiles",
            "/net/hadess/PowerProfiles",
            "net.hadess.PowerProfiles",
            "ActiveProfile",
        ])
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        // Output format: s "balanced"
        if let Some(start) = text.find('"') {
            if let Some(end) = text.rfind('"') {
                if end > start {
                    return text[start + 1..end].to_string();
                }
            }
        }
    }
    "balanced".to_string()
}

fn set_active_profile(profile: &str) -> bool {
    let status = Command::new("busctl")
        .args([
            "set-property",
            "net.hadess.PowerProfiles",
            "/net/hadess/PowerProfiles",
            "net.hadess.PowerProfiles",
            "ActiveProfile",
            "s",
            profile,
        ])
        .status();

    if let Ok(s) = status {
        if s.success() {
            // Signal Waybar to refresh the power/battery module instantly
            let _ = Command::new("pkill").args(["-RTMIN+9", "waybar"]).status();
            return true;
        }
    }
    false
}

fn get_battery_info() -> (u32, String) {
    let pwr_dir = Path::new("/sys/class/power_supply");
    let mut capacity = 100u32;
    let mut status = "Full".to_string();

    if let Ok(entries) = fs::read_dir(pwr_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with("BAT") {
                if let Ok(c_str) = fs::read_to_string(path.join("capacity")) {
                    capacity = c_str.trim().parse().unwrap_or(100);
                }
                if let Ok(s_str) = fs::read_to_string(path.join("status")) {
                    status = s_str.trim().to_string();
                }
                break;
            }
        }
    }

    (capacity, status)
}

fn get_battery_icon(capacity: u32, is_charging: bool) -> &'static str {
    if is_charging {
        return "󰂄";
    }

    match capacity {
        90..=100 => "󰁹",
        80..=89 => "󰂂",
        70..=79 => "󰂁",
        60..=69 => "󰂀",
        50..=59 => "󰁿",
        40..=49 => "󰁾",
        30..=39 => "󰁽",
        20..=29 => "󰁼",
        10..=19 => "󰁻",
        _ => "󰂎",
    }
}

pub fn handle_power(args: &[String]) {
    let subcmd = args.first().map(|s| s.as_str()).unwrap_or("status");

    match subcmd {
        "next" => {
            let cur = get_active_profile();
            let next_profile = match cur.as_str() {
                "balanced" => "performance",
                "performance" => "power-saver",
                _ => "balanced",
            };
            set_active_profile(next_profile);
            println!("{}", next_profile);
        }
        "prev" => {
            let cur = get_active_profile();
            let prev_profile = match cur.as_str() {
                "balanced" => "power-saver",
                "power-saver" => "performance",
                _ => "balanced",
            };
            set_active_profile(prev_profile);
            println!("{}", prev_profile);
        }
        "set" => {
            if let Some(target) = args.get(1) {
                set_active_profile(target);
                println!("{}", target);
            } else {
                eprintln!("Usage: wt-rust power set <performance|balanced|power-saver>");
            }
        }
        "get" => {
            println!("{}", get_active_profile());
        }
        "status" => {
            let (capacity, bat_status) = get_battery_info();
            let is_charging = bat_status.eq_ignore_ascii_case("charging");
            let profile = get_active_profile();

            // Distinct badge icon for each power plan
            let (plan_icon, plan_name_tr) = match profile.as_str() {
                "performance" => ("󱐋", "Performans"),
                "power-saver" => ("󰌪", "Güç Tasarrufu"),
                _ => ("󰾅", "Dengeli"),
            };

            let bat_icon = get_battery_icon(capacity, is_charging);

            // Visible battery indicator clearly communicating the active power plan
            let text = format!("{}  {} {}%", plan_icon, bat_icon, capacity);

            let tooltip = format!(
                "Batarya: %{} ({})\nGüç Planı: {} ({})\n\nTıklayarak planı değiştir",
                capacity, bat_status, plan_name_tr, profile
            );

            let class = format!(
                "battery profile-{} status-{}",
                profile,
                bat_status.to_ascii_lowercase()
            );

            let resp = PowerOutput {
                text,
                tooltip,
                class,
                percentage: capacity,
            };

            if let Ok(json_str) = serde_json::to_string(&resp) {
                println!("{}", json_str);
            }
        }
        _ => {
            eprintln!("Usage: wt-rust power <status|next|prev|set <profile>|get>");
        }
    }
}
