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
    if std::path::Path::new("/proc/driver/nvidia/version").exists() {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output();

        if let Ok(out) = output
            && out.status.success()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                let util = parts[0];
                let temp = parts[1];
                let resp = WaybarOutput {
                    text: format!("󰢮 {}%  {}°C", util, temp),
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

    // Sysfs AMD/Intel GPU busy percent + hwmon temp (zero process spawn)
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let dev = entry.path().join("device");
            if let Ok(busy_str) = fs::read_to_string(dev.join("gpu_busy_percent"))
                && let Ok(util) = busy_str.trim().parse::<u32>()
            {
                let mut temp_str = String::new();
                if let Ok(hwmons) = fs::read_dir(dev.join("hwmon")) {
                    for hw in hwmons.flatten() {
                        if let Ok(t_str) = fs::read_to_string(hw.path().join("temp1_input"))
                            && let Ok(t_milli) = t_str.trim().parse::<u32>()
                            && t_milli > 0
                        {
                            temp_str = format!("  {}°C", t_milli / 1000);
                            break;
                        }
                    }
                }
                let resp = WaybarOutput {
                    text: format!("󰢮 {}%{}", util, temp_str),
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
    let combined_out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "get-property",
            "org.kde.KWin",
            "/VirtualDesktopManager",
            "org.kde.KWin.VirtualDesktopManager",
            "desktops",
            "current",
        ])
        .output();

    let mut desktops = Vec::new();
    let mut cur_id = String::new();

    if let Ok(out) = combined_out {
        let stdout_str = String::from_utf8_lossy(&out.stdout);
        let mut lines = stdout_str.lines();
        if let Some(first_line) = lines.next()
            && let Ok(v) = serde_json::from_str::<Value>(first_line)
            && let Some(arr) = v.get("data").and_then(|d| d.as_array())
        {
            for item in arr {
                if let Some(entry) = item.as_array()
                    && entry.len() >= 3
                {
                    let did = entry[1].as_str().unwrap_or("").to_string();
                    let name = entry[2].as_str().unwrap_or("").to_string();
                    desktops.push((did, name));
                }
            }
        }
        if let Some(second_line) = lines.next()
            && let Ok(v) = serde_json::from_str::<Value>(second_line)
            && let Some(s) = v.get("data").and_then(|d| d.as_str())
        {
            cur_id = s.to_string();
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
        Some(dir @ ("next" | "prev")) => {
            if count > 1 {
                let target_idx = if dir == "next" {
                    (cur_idx + 1) % count
                } else {
                    (cur_idx + count - 1) % count
                };
                let target_id = &desktops[target_idx].0;
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

            let pal = crate::theme::get_current_palette();
            let mut dots = Vec::new();
            for i in 0..count {
                if i == cur_idx {
                    dots.push(format!("<span color='{}'>●</span>", pal.accent));
                } else {
                    dots.push(format!("<span color='{}'>○</span>", pal.text_muted));
                }
            }

            let text = format!(
                "<span color='{}'>|</span>   {}",
                pal.text_muted,
                dots.join("   ")
            );
            let resp = WaybarOutput {
                text,
                tooltip: Value::String(format!("{} ({}/{})", cur_name, cur_idx + 1, count)),
                class: Some(format!("workspace-{}", cur_idx + 1)),
            };
            println!("{}", serde_json::to_string(&resp).unwrap());
        }
    }
}

fn calc_brightness_target(cur: i64, max: i64, arg: Option<&str>) -> Option<i64> {
    if max <= 0 {
        return None;
    }
    let step = (max * 5 / 100).max(1);
    let min_val = (max / 100).max(1); // 1% minimum hard floor

    match arg {
        Some("toggle") => Some(if cur >= max * 75 / 100 {
            (max * 50 / 100).max(min_val)
        } else {
            max
        }),
        Some("inc") => Some((cur + step).min(max)),
        Some("dec") => Some((cur - step).max(min_val)),
        _ => None,
    }
}

pub fn handle_brightness(arg: Option<&str>) {
    // 1. Try KDE PowerManagement DBus (syncs with KWin, kwinoutputconfig.json, PowerDevil, and sysfs)
    let cur_out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "call",
            "org.kde.Solid.PowerManagement",
            "/org/kde/Solid/PowerManagement/Actions/BrightnessControl",
            "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
            "brightness",
        ])
        .output();

    let max_out = Command::new("busctl")
        .args([
            "--user",
            "--json=short",
            "call",
            "org.kde.Solid.PowerManagement",
            "/org/kde/Solid/PowerManagement/Actions/BrightnessControl",
            "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
            "brightnessMax",
        ])
        .output();

    let mut dbus_cur: Option<i64> = None;
    let mut dbus_max: Option<i64> = None;

    if let (Ok(c_res), Ok(m_res)) = (cur_out, max_out)
        && let (Ok(c_val), Ok(m_val)) = (
            serde_json::from_slice::<Value>(&c_res.stdout),
            serde_json::from_slice::<Value>(&m_res.stdout),
        )
    {
        dbus_cur = c_val
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_i64());
        dbus_max = m_val
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_i64());
    }

    if let (Some(cur_val), Some(max_val)) = (dbus_cur, dbus_max)
        && let Some(target) = calc_brightness_target(cur_val, max_val, arg)
    {
        let _ = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.kde.Solid.PowerManagement",
                "/org/kde/Solid/PowerManagement/Actions/BrightnessControl",
                "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
                "setBrightness",
                "i",
                &target.to_string(),
            ])
            .status();

        return;
    }

    // 2. Fallback: Direct sysfs + brightnessctl
    let mut cur = 0i64;
    let mut max = 0i64;

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

    if let Some(target) = calc_brightness_target(cur, max, arg) {
        let _ = Command::new("brightnessctl")
            .args(["set", &target.to_string()])
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_brightness_target() {
        assert_eq!(calc_brightness_target(50, 0, Some("inc")), None);
        assert_eq!(calc_brightness_target(50, 100, None), None);
        assert_eq!(calc_brightness_target(98, 100, Some("inc")), Some(100));
        assert_eq!(calc_brightness_target(3, 100, Some("dec")), Some(1)); // 1% floor
        assert_eq!(calc_brightness_target(80, 100, Some("toggle")), Some(50));
        assert_eq!(calc_brightness_target(50, 100, Some("toggle")), Some(100));
    }
}
