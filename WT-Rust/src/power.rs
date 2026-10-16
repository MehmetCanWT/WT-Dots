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
    // 1. Try KDE Solid PowerManagement on user session bus
    let output = Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.kde.Solid.PowerManagement",
            "/org/kde/Solid/PowerManagement/Actions/PowerProfile",
            "org.kde.Solid.PowerManagement.Actions.PowerProfile",
            "currentProfile",
        ])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(start) = text.find('"') {
                if let Some(end) = text.rfind('"') {
                    if end > start {
                        return text[start + 1..end].to_string();
                    }
                }
            }
        }
    }

    // 2. Fallback: net.hadess.PowerProfiles on system bus
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
    let norm = match profile {
        "performance" | "perf" => "performance",
        "power-saver" | "powersave" | "saver" | "eco" => "power-saver",
        _ => "balanced",
    };

    let mut ok = false;

    // 1. Set via KDE Solid PowerManagement (user session bus)
    let status_kde = Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.kde.Solid.PowerManagement",
            "/org/kde/Solid/PowerManagement/Actions/PowerProfile",
            "org.kde.Solid.PowerManagement.Actions.PowerProfile",
            "setProfile",
            "s",
            norm,
        ])
        .status();

    if let Ok(s) = status_kde {
        if s.success() {
            ok = true;
        }
    }

    // 2. Set via net.hadess.PowerProfiles (system bus)
    let status_hadess = Command::new("busctl")
        .args([
            "set-property",
            "net.hadess.PowerProfiles",
            "/net/hadess/PowerProfiles",
            "net.hadess.PowerProfiles",
            "ActiveProfile",
            "s",
            norm,
        ])
        .status();

    if let Ok(s) = status_hadess {
        if s.success() {
            ok = true;
        }
    }

    // Signal Waybar to refresh the custom/battery module immediately
    let _ = Command::new("pkill").args(["-RTMIN+9", "waybar"]).status();

    ok
}

fn get_battery_info() -> (u32, String, bool) {
    let pwr_dir = Path::new("/sys/class/power_supply");
    let mut capacity = 100u32;
    let mut status = "Full".to_string();
    let mut ac_online = false;

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
            } else if name.starts_with("AC") || name.starts_with("ADP") {
                if let Ok(o_str) = fs::read_to_string(path.join("online")) {
                    ac_online = o_str.trim() == "1";
                }
            }
        }
    }

    (capacity, status, ac_online)
}

fn get_battery_icon(capacity: u32, is_charging: bool) -> &'static str {
    if is_charging {
        return match capacity {
            95..=100 => "󰂅", // charging 100%
            85..=94 => "󰂋",  // charging 90%
            75..=84 => "󰂊",  // charging 80%
            65..=74 => "󰢞",  // charging 70%
            55..=64 => "󰂉",  // charging 60%
            45..=54 => "󰢝",  // charging 50%
            35..=44 => "󰂈",  // charging 40%
            25..=34 => "󰂇",  // charging 30%
            15..=24 => "󰂆",  // charging 20%
            _ => "󰢜",        // charging 10%
        };
    }

    match capacity {
        95..=100 => "󰁹", // 100%
        85..=94 => "󰂂",  // 90%
        75..=84 => "󰂁",  // 80%
        65..=74 => "󰂀",  // 70%
        55..=64 => "󰁿",  // 60%
        45..=54 => "󰁾",  // 50%
        35..=44 => "󰁽",  // 40%
        25..=34 => "󰁼",  // 30%
        15..=24 => "󰁻",  // 20%
        5..=14 => "󰁺",   // 10%
        _ => "󰂎",        // 0% / empty
    }
}

pub fn handle_power(args: &[String]) {
    let subcmd = args.first().map(|s| s.as_str()).unwrap_or("status");

    match subcmd {
        "next" => {
            let cur = get_active_profile();
            let next_profile = match cur.as_str() {
                "power-saver" => "balanced",
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
                "power-saver" => "performance",
                "performance" => "balanced",
                "balanced" => "power-saver",
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
            let (capacity, bat_status, ac_online) = get_battery_info();
            let is_charging = bat_status.eq_ignore_ascii_case("charging");
            let profile = get_active_profile();

            // Distinct badge icon for each power plan
            let (plan_icon, plan_name_tr) = match profile.as_str() {
                "performance" => ("󱐋", "Performans"),
                "power-saver" => ("󰌪", "Güç Tasarrufu"),
                _ => ("󰾅", "Dengeli"),
            };

            let bat_icon = get_battery_icon(capacity, is_charging);

            let text = format!("{} {} {}%", plan_icon, bat_icon, capacity);

            let ac_text = if ac_online { " (AC Bağlı)" } else { "" };
            let tooltip = format!(
                "Batarya: %{} ({}{})\nGüç Planı: {} ({})\n\nSol Tık: Güç planını değiştir",
                capacity, bat_status, ac_text, plan_name_tr, profile
            );

            let p_class = match profile.as_str() {
                "performance" => "profile-performance",
                "power-saver" => "profile-power-saver",
                _ => "profile-balanced",
            };

            let mut class_list = vec!["battery", "custom-battery", p_class];
            if is_charging {
                class_list.push("charging");
            }
            if capacity <= 15 {
                class_list.push("critical");
            } else if capacity <= 30 {
                class_list.push("warning");
            }

            let class = class_list.join(" ");

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
