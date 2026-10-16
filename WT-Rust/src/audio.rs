use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
struct AudioOutput {
    text: String,
    tooltip: String,
    class: String,
    percentage: u32,
}

fn get_current_volume_and_mute() -> (f32, bool) {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        // Format: "Volume: 0.55" or "Volume: 1.00 [MUTED]"
        let is_muted = text.contains("[MUTED]");
        if let Some(vol) = text
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<f32>().ok())
        {
            return (vol, is_muted);
        }
    }
    (0.0, false)
}

pub fn handle_volume(args: &[String]) {
    let subcmd = args.first().map(|s| s.as_str()).unwrap_or("status");
    let step_str = args.get(1).map(|s| s.as_str()).unwrap_or("2%");
    let step_val = step_str.trim_end_matches('%').parse::<f32>().unwrap_or(2.0) / 100.0;

    let (cur_vol, is_muted) = get_current_volume_and_mute();

    match subcmd {
        "inc" => {
            // Strictly cap at 1.00 (100% max volume - never 150%)
            let target = (cur_vol + step_val).min(1.00);
            let pct_str = format!("{:.0}%", target * 100.0);
            let _ = Command::new("wpctl")
                .args(["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &pct_str])
                .status();
        }
        "dec" => {
            let target = (cur_vol - step_val).max(0.00);
            let pct_str = format!("{:.0}%", target * 100.0);
            let _ = Command::new("wpctl")
                .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &pct_str])
                .status();
        }
        "mute" | "toggle" => {
            let _ = Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                .status();
        }
        "status" => {
            let pct = (cur_vol * 100.0).round() as u32;
            let icon = if is_muted {
                "󰝟"
            } else if pct > 60 {
                "󰕾"
            } else if pct > 20 {
                "󰖀"
            } else {
                "󰕿"
            };

            let text = if is_muted {
                format!("󰝟 {}%", pct)
            } else {
                format!("{} {}%", icon, pct)
            };

            let resp = AudioOutput {
                text,
                tooltip: format!("Ses: %{} {}", pct, if is_muted { "(Sessiz)" } else { "" }),
                class: if is_muted {
                    "muted".to_string()
                } else {
                    format!("vol-{}", pct)
                },
                percentage: pct,
            };

            if let Ok(json_str) = serde_json::to_string(&resp) {
                println!("{}", json_str);
            }
        }
        _ => {
            eprintln!("Usage: wt-rust volume <inc [step%]|dec [step%]|mute|status>");
        }
    }
}
