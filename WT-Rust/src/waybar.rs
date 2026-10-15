use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::process::Command;

#[derive(Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

pub fn get_gpu() {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                let util = parts[0];
                let temp = parts[1];
                let resp = WaybarOutput {
                    text: format!("󰢮 {}%  <span color='#9aa2b6'>{}°C</span>", util, temp),
                    tooltip: Value::Bool(false),
                    class: None,
                };
                if let Ok(json_str) = serde_json::to_string(&resp) {
                    println!("{}", json_str);
                    return;
                }
            }
        }
    }

    let fallback = WaybarOutput {
        text: "󰢮 N/A".to_string(),
        tooltip: Value::Bool(false),
        class: None,
    };
    println!("{}", serde_json::to_string(&fallback).unwrap());
}

fn get_desktops_state() -> (Vec<(String, String)>, usize, String) {
    let desktops_out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "get-property",
            "org.kde.KWin",
            "/VirtualDesktopManager",
            "org.kde.KWin.VirtualDesktopManager",
            "desktops",
        ])
        .output();

    let cur_out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "get-property",
            "org.kde.KWin",
            "/VirtualDesktopManager",
            "org.kde.KWin.VirtualDesktopManager",
            "current",
        ])
        .output();

    let mut desktops = Vec::new();
    if let Ok(out) = desktops_out {
        if let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) {
            if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
                for item in arr {
                    if let Some(entry) = item.as_array() {
                        if entry.len() >= 3 {
                            let did = entry[1].as_str().unwrap_or("").to_string();
                            let name = entry[2].as_str().unwrap_or("").to_string();
                            desktops.push((did, name));
                        }
                    }
                }
            }
        }
    }

    let mut cur_id = String::new();
    if let Ok(out) = cur_out {
        if let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) {
            if let Some(s) = v.get("data").and_then(|d| d.as_str()) {
                cur_id = s.to_string();
            }
        }
    }

    let mut cur_idx = 0;
    let mut cur_name = "Desktop 1".to_string();
    for (i, (did, name)) in desktops.iter().enumerate() {
        if did == &cur_id {
            cur_idx = i;
            cur_name = name.clone();
            break;
        }
    }

    (desktops, cur_idx, cur_name)
}

pub fn handle_workspaces(arg: Option<&str>) {
    let (desktops, cur_idx, cur_name) = get_desktops_state();
    let count = desktops.len();

    match arg {
        Some("next") => {
            if count > 1 {
                let next_idx = (cur_idx + 1) % count;
                let target_id = &desktops[next_idx].0;
                let _ = Command::new("busctl")
                    .args([
                        "--user",
                        "set-property",
                        "org.kde.KWin",
                        "/VirtualDesktopManager",
                        "org.kde.KWin.VirtualDesktopManager",
                        "current",
                        "s",
                        target_id,
                    ])
                    .status();
                let _ = Command::new("pkill").args(["-RTMIN+8", "waybar"]).status();
            }
        }
        Some("prev") => {
            if count > 1 {
                let prev_idx = (cur_idx + count - 1) % count;
                let target_id = &desktops[prev_idx].0;
                let _ = Command::new("busctl")
                    .args([
                        "--user",
                        "set-property",
                        "org.kde.KWin",
                        "/VirtualDesktopManager",
                        "org.kde.KWin.VirtualDesktopManager",
                        "current",
                        "s",
                        target_id,
                    ])
                    .status();
                let _ = Command::new("pkill").args(["-RTMIN+8", "waybar"]).status();
            }
        }
        _ => {
            if count <= 1 {
                let resp = WaybarOutput {
                    text: "".to_string(),
                    tooltip: Value::String("".to_string()),
                    class: None,
                };
                println!("{}", serde_json::to_string(&resp).unwrap());
                return;
            }

            let mut dots = Vec::new();
            for i in 0..count {
                if i == cur_idx {
                    dots.push("<span color='#ffffff'>●</span>");
                } else {
                    dots.push("<span color='#545868'>○</span>");
                }
            }

            let text = format!("<span color='#545868'>|</span>   {}", dots.join("   "));
            let resp = WaybarOutput {
                text,
                tooltip: Value::String(format!("{} ({}/{})", cur_name, cur_idx + 1, count)),
                class: Some(format!("workspace-{}", cur_idx + 1)),
            };
            println!("{}", serde_json::to_string(&resp).unwrap());
        }
    }
}

pub fn handle_brightness(arg: Option<&str>) {
    let mut cur = 0u64;
    let mut max = 0u64;

    if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
        for entry in entries.flatten() {
            let p = entry.path();
            if let (Ok(c_str), Ok(m_str)) = (
                fs::read_to_string(p.join("brightness")),
                fs::read_to_string(p.join("max_brightness")),
            ) {
                cur = c_str.trim().parse().unwrap_or(0);
                max = m_str.trim().parse().unwrap_or(0);
                break;
            }
        }
    }

    if max == 0 {
        return;
    }

    let step = (max * 5 / 100).max(1);
    let min_val = (max / 100).max(1); // Hard lower floor of 1%

    match arg {
        Some("toggle") => {
            if cur >= (max * 75 / 100) {
                let target = (max * 50 / 100).max(min_val);
                let _ = Command::new("brightnessctl").args(["set", &target.to_string()]).status();
            } else {
                let _ = Command::new("brightnessctl").args(["set", &max.to_string()]).status();
            }
        }
        Some("inc") => {
            let target = (cur + step).min(max);
            let _ = Command::new("brightnessctl").args(["set", &target.to_string()]).status();
        }
        Some("dec") => {
            let target = cur.saturating_sub(step).max(min_val);
            let _ = Command::new("brightnessctl").args(["set", &target.to_string()]).status();
        }
        _ => {}
    }
}
