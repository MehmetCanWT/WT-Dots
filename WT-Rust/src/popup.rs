//! Pure-Rust GPU-rendered Volume Mixer & Wi-Fi Control Popup (`eframe` / `egui`).
//! Synchronizes its accent/surface palette live from `~/.cache/wt-rust/palette.json`.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::btop::{FullStats, collect_stats};
use crate::common::{SocketGuard, bind_single_instance_socket, install_nerd_font, runtime_path};
use crate::theme::{Palette, get_current_palette, parse_hex_rgb};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PopupMode {
    Volume,
    Wifi,
    Bluetooth,
    Battery,
    System,
    Power,
}

impl PopupMode {
    fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            Some("wifi") | Some("network") => Self::Wifi,
            Some("bluetooth") | Some("bt") => Self::Bluetooth,
            Some("battery") | Some("bat") => Self::Battery,
            Some("system") | Some("sys") | Some("monitor") | Some("cpu") | Some("gpu") => {
                Self::System
            }
            Some("power") => Self::Power,
            _ => Self::Volume,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Volume => "volume",
            Self::Wifi => "wifi",
            Self::Bluetooth => "bluetooth",
            Self::Battery => "battery",
            Self::System => "system",
            Self::Power => "power",
        }
    }

    fn size(self) -> [f32; 2] {
        match self {
            Self::Power => [236.0, 260.0],
            Self::Volume => [380.0, 410.0],
            Self::Wifi => [380.0, 440.0],
            Self::Bluetooth => [380.0, 420.0],
            Self::Battery => [380.0, 376.0],
            Self::System => [380.0, 456.0],
        }
    }
}

enum PopupEvent {
    AudioLoaded(AudioState),
    WifiLoaded(WifiState),
    WifiStatus(WifiState, String, bool),
    BtLoaded(BtState),
    BtStatus(BtState, String, bool),
    BatteryLoaded(BatteryState),
    SystemLoaded(Box<FullStats>),
    Trigger(PopupMode),
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct BatteryState {
    present: bool,
    capacity: u8,
    status: String,
    power_watts: Option<f32>,
    time_mins: Option<u32>,
    health_pct: Option<u8>,
    cycles: Option<u32>,
    profile: String,
    has_profiles: bool,
    #[serde(default)]
    keep_awake: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct BtDevice {
    mac: String,
    name: String,
    connected: bool,
    paired: bool,
    battery: Option<u8>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct BtState {
    powered: bool,
    devices: Vec<BtDevice>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct AudioChannelItem {
    id: String,
    label: String,
    percent: u8,
    muted: bool,
    is_default: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct AudioState {
    default_sink_name: String,
    sinks: Vec<AudioChannelItem>,
    sources: Vec<AudioChannelItem>,
    streams: Vec<AudioChannelItem>,
}

#[derive(Clone, Serialize, Deserialize)]
struct WifiNetwork {
    ssid: String,
    in_use: bool,
    security: String,
    signal: u8,
    saved: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct WifiState {
    enabled: bool,
    wifi_device: Option<String>,
    ip_addr: String,
    eth_line: Option<String>,
    networks: Vec<WifiNetwork>,
}

#[derive(Default, Serialize, Deserialize)]
struct PopupCache {
    audio: Option<AudioState>,
    wifi: Option<WifiState>,
    bt: Option<BtState>,
    battery: Option<BatteryState>,
}

fn load_popup_cache() -> PopupCache {
    std::fs::read_to_string(runtime_path("wt-rust-popup-cache.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_popup_cache(cache: &PopupCache) {
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(runtime_path("wt-rust-popup-cache.json"), json);
    }
}

fn snap_five(raw: f32) -> u8 {
    ((raw / 5.0).round() as i32 * 5).clamp(0, 100) as u8
}

fn split_nmcli_line(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&':') {
            current.push(':');
            chars.next();
        } else if c == ':' {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts
}

fn parse_pactl_percent(vol_obj: Option<&serde_json::Value>) -> u8 {
    let Some(map) = vol_obj.and_then(|v| v.as_object()) else {
        return 0;
    };
    for ch in map.values() {
        if let Some(raw) = ch.get("value_percent").and_then(|v| v.as_str())
            && let Ok(num) = raw.trim().trim_end_matches('%').parse::<f32>()
        {
            return snap_five(num);
        }
    }
    0
}

fn run_cmd_stdout(bin: &str, args: &[&str]) -> String {
    Command::new(bin)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

fn run_cmd_quiet(bin: &str, args: &[&str]) -> Result<(), String> {
    match Command::new(bin).args(args).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
            Err(if !err.is_empty() {
                err
            } else if !out.is_empty() {
                out
            } else {
                format!("{} exited with status {:?}", bin, o.status.code())
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

fn query_audio_state() -> AudioState {
    let info_raw = run_cmd_stdout("pactl", &["-f", "json", "info"]);
    let sinks_raw = run_cmd_stdout("pactl", &["-f", "json", "list", "sinks"]);
    let sources_raw = run_cmd_stdout("pactl", &["-f", "json", "list", "sources"]);
    let inputs_raw = run_cmd_stdout("pactl", &["-f", "json", "list", "sink-inputs"]);

    let info: serde_json::Value = serde_json::from_str(&info_raw).unwrap_or_default();
    let default_sink_name = info
        .get("default_sink_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let default_source_name = info
        .get("default_source_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let parse_devices =
        |raw: &str, default_name: &str, skip_monitors: bool| -> Vec<AudioChannelItem> {
            let arr: Vec<serde_json::Value> = serde_json::from_str(raw).unwrap_or_default();
            arr.into_iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    if skip_monitors && name.ends_with(".monitor") {
                        return None;
                    }
                    let desc = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&name)
                        .to_string();
                    let muted = item.get("mute").and_then(|v| v.as_bool()).unwrap_or(false);
                    let percent = parse_pactl_percent(item.get("volume"));
                    Some(AudioChannelItem {
                        is_default: name == default_name,
                        id: name,
                        label: desc,
                        percent,
                        muted,
                    })
                })
                .collect()
        };

    let sinks = parse_devices(&sinks_raw, &default_sink_name, false);
    let sources = parse_devices(&sources_raw, &default_source_name, true);

    let streams_arr: Vec<serde_json::Value> = serde_json::from_str(&inputs_raw).unwrap_or_default();
    let streams = streams_arr
        .into_iter()
        .filter_map(|item| {
            let idx = item.get("index")?.as_u64()?;
            let props = item.get("properties");
            let label = props
                .and_then(|p| p.get("application.name").or_else(|| p.get("media.name")))
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("Stream #{}", idx));
            let muted = item.get("mute").and_then(|v| v.as_bool()).unwrap_or(false);
            let percent = parse_pactl_percent(item.get("volume"));
            Some(AudioChannelItem {
                id: idx.to_string(),
                label,
                percent,
                muted,
                is_default: true,
            })
        })
        .collect();

    AudioState {
        default_sink_name,
        sinks,
        sources,
        streams,
    }
}

fn parse_wifi_scan_lines(raw: &str, saved_ssids: &HashSet<String>) -> Vec<WifiNetwork> {
    let mut dedup: HashMap<String, WifiNetwork> = HashMap::new();
    for line in raw.lines() {
        let parts = split_nmcli_line(line);
        if parts.len() < 4 {
            continue;
        }
        let in_use = parts[0].trim() == "*";
        let ssid = parts[1].trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        let security = parts[2].trim().to_string();
        let signal = parts[3].trim().parse::<u8>().unwrap_or(0);
        let replace = match dedup.get(&ssid) {
            None => true,
            Some(prev) => in_use || (!prev.in_use && signal > prev.signal),
        };
        if replace {
            dedup.insert(
                ssid.clone(),
                WifiNetwork {
                    saved: saved_ssids.contains(&ssid),
                    ssid,
                    in_use,
                    security,
                    signal,
                },
            );
        }
    }

    let mut list: Vec<WifiNetwork> = dedup.into_values().collect();
    list.sort_by(|a, b| {
        b.in_use
            .cmp(&a.in_use)
            .then_with(|| b.signal.cmp(&a.signal))
            .then_with(|| a.ssid.to_lowercase().cmp(&b.ssid.to_lowercase()))
    });
    list
}

fn query_wifi_state(rescan: bool) -> WifiState {
    if rescan {
        let _ = run_cmd_quiet("nmcli", &["dev", "wifi", "rescan"]);
    }

    let enabled = run_cmd_stdout("nmcli", &["radio", "wifi"]).trim() == "enabled";
    let dev_out = run_cmd_stdout(
        "nmcli",
        &["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "dev", "status"],
    );

    let mut wifi_device = None;
    let mut connected_ssid = None;
    let mut eth_line = None;
    for line in dev_out.lines() {
        let parts = split_nmcli_line(line);
        if parts.len() < 4 {
            continue;
        }
        let (dev, dtype, state, conn) = (&parts[0], &parts[1], &parts[2], &parts[3]);
        if dtype == "wifi" && wifi_device.is_none() {
            wifi_device = Some(dev.clone());
            if state == "connected" && !conn.is_empty() && conn != "--" {
                connected_ssid = Some(conn.clone());
            }
        } else if dtype == "ethernet" && state == "connected" && eth_line.is_none() {
            eth_line = Some(format!("{}: {}", dev, conn));
        }
    }

    let mut ip_addr = String::new();
    if let Some(ref dev) = wifi_device
        && connected_ssid.is_some()
    {
        let ip_out = run_cmd_stdout("nmcli", &["-t", "-f", "IP4.ADDRESS", "dev", "show", dev]);
        for line in ip_out.lines() {
            let parts = split_nmcli_line(line);
            if parts.len() >= 2 {
                ip_addr = parts[1].split('/').next().unwrap_or_default().to_string();
                break;
            }
        }
    }

    let saved_raw = run_cmd_stdout("nmcli", &["-t", "-f", "NAME,TYPE", "connection", "show"]);
    let mut saved_ssids = HashSet::new();
    for line in saved_raw.lines() {
        let parts = split_nmcli_line(line);
        if parts.len() >= 2 && parts[1] == "802-11-wireless" {
            saved_ssids.insert(parts[0].clone());
        }
    }

    let networks = if enabled {
        let mut wifi_args = vec![
            "-t",
            "-f",
            "IN-USE,SSID,SECURITY,SIGNAL",
            "dev",
            "wifi",
            "list",
        ];
        if !rescan {
            wifi_args.extend(["--rescan", "no"]);
        }
        let wifi_raw = run_cmd_stdout("nmcli", &wifi_args);
        parse_wifi_scan_lines(&wifi_raw, &saved_ssids)
    } else {
        Vec::new()
    };

    WifiState {
        enabled,
        wifi_device,
        ip_addr,
        eth_line,
        networks,
    }
}

fn c32(hex: &str, alpha: u8) -> egui::Color32 {
    let (r, g, b) = parse_hex_rgb(hex);
    egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

fn mix_c32(a: (u8, u8, u8), b: (u8, u8, u8), weight_b: f32, alpha: u8) -> egui::Color32 {
    let w = weight_b.clamp(0.0, 1.0);
    egui::Color32::from_rgba_unmultiplied(
        (a.0 as f32 * (1.0 - w) + b.0 as f32 * w).round() as u8,
        (a.1 as f32 * (1.0 - w) + b.1 as f32 * w).round() as u8,
        (a.2 as f32 * (1.0 - w) + b.2 as f32 * w).round() as u8,
        alpha,
    )
}

fn query_bt_state() -> BtState {
    let mut state = BtState::default();
    if let Ok(out) = Command::new("bluetoothctl").arg("show").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        state.powered = s.lines().any(|l| l.trim() == "Powered: yes");
    }
    if !state.powered {
        return state;
    }

    if let Ok(out) = Command::new("bluetoothctl").arg("devices").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("Device ") {
                let mut parts = rest.splitn(2, ' ');
                let mac = parts.next().unwrap_or("").trim().to_string();
                let name = parts.next().unwrap_or("").trim().to_string();
                if mac.is_empty() || name.is_empty() {
                    continue;
                }
                let mut connected = false;
                let mut paired = false;
                let mut battery = None;
                if let Ok(info_out) = Command::new("bluetoothctl").args(["info", &mac]).output() {
                    let info = String::from_utf8_lossy(&info_out.stdout);
                    for il in info.lines() {
                        let t = il.trim();
                        if t == "Connected: yes" {
                            connected = true;
                        } else if t == "Paired: yes" {
                            paired = true;
                        } else if let Some(b) = t.strip_prefix("Battery Percentage:")
                            && let Some(open) = b.find('(')
                            && let Some(close) = b.find(')')
                        {
                            battery = b[open + 1..close].trim().parse::<u8>().ok();
                        }
                    }
                }
                state.devices.push(BtDevice {
                    mac,
                    name,
                    connected,
                    paired,
                    battery,
                });
            }
        }
    }

    state.devices.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| b.paired.cmp(&a.paired))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    state
}

fn parse_battery_uevent(raw: &str) -> BatteryState {
    let mut kv: HashMap<&str, &str> = HashMap::new();
    for line in raw.lines() {
        if let Some((k, v)) = line.split_once('=') {
            kv.insert(k.trim(), v.trim());
        }
    }
    let get_u64 = |k: &str| kv.get(k).and_then(|v| v.parse::<u64>().ok());

    let capacity = get_u64("POWER_SUPPLY_CAPACITY").unwrap_or(0).min(100) as u8;
    let status = kv
        .get("POWER_SUPPLY_STATUS")
        .copied()
        .unwrap_or("Unknown")
        .to_string();

    let power_uw = get_u64("POWER_SUPPLY_POWER_NOW").or_else(|| {
        let v = get_u64("POWER_SUPPLY_VOLTAGE_NOW")? as u128;
        let c = get_u64("POWER_SUPPLY_CURRENT_NOW")? as u128;
        Some(((v * c) / 1_000_000) as u64)
    });
    let power_watts = power_uw
        .filter(|&w| w > 100_000)
        .map(|w| w as f32 / 1_000_000.0);

    let time_mins = if status == "Discharging" {
        if let (Some(en), Some(pw)) = (get_u64("POWER_SUPPLY_ENERGY_NOW"), power_uw)
            && pw > 100_000
        {
            Some(((en as f64 / pw as f64) * 60.0).round() as u32)
        } else if let (Some(cn), Some(cur)) = (
            get_u64("POWER_SUPPLY_CHARGE_NOW"),
            get_u64("POWER_SUPPLY_CURRENT_NOW"),
        ) && cur > 10_000
        {
            Some(((cn as f64 / cur as f64) * 60.0).round() as u32)
        } else {
            None
        }
    } else if status == "Charging" {
        if let (Some(ef), Some(en), Some(pw)) = (
            get_u64("POWER_SUPPLY_ENERGY_FULL"),
            get_u64("POWER_SUPPLY_ENERGY_NOW"),
            power_uw,
        ) && pw > 100_000
            && ef > en
        {
            Some((((ef - en) as f64 / pw as f64) * 60.0).round() as u32)
        } else if let (Some(cf), Some(cn), Some(cur)) = (
            get_u64("POWER_SUPPLY_CHARGE_FULL"),
            get_u64("POWER_SUPPLY_CHARGE_NOW"),
            get_u64("POWER_SUPPLY_CURRENT_NOW"),
        ) && cur > 10_000
            && cf > cn
        {
            Some((((cf - cn) as f64 / cur as f64) * 60.0).round() as u32)
        } else {
            None
        }
    } else {
        None
    };

    let health_pct = get_u64("POWER_SUPPLY_ENERGY_FULL")
        .zip(get_u64("POWER_SUPPLY_ENERGY_FULL_DESIGN"))
        .or_else(|| {
            get_u64("POWER_SUPPLY_CHARGE_FULL").zip(get_u64("POWER_SUPPLY_CHARGE_FULL_DESIGN"))
        })
        .filter(|&(_, design)| design > 0)
        .map(|(full, design)| ((full * 100) / design).min(100) as u8);

    let cycles = get_u64("POWER_SUPPLY_CYCLE_COUNT").map(|c| c as u32);

    BatteryState {
        present: true,
        capacity,
        status,
        power_watts,
        time_mins,
        health_pct,
        cycles,
        profile: String::new(),
        has_profiles: false,
        keep_awake: false,
    }
}

fn query_battery_state() -> BatteryState {
    let mut state = BatteryState::default();
    if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
        let mut bat_paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("BAT"))
            })
            .collect();
        bat_paths.sort();
        if let Some(bat) = bat_paths.first()
            && let Ok(raw) = std::fs::read_to_string(bat.join("uevent"))
        {
            state = parse_battery_uevent(&raw);
        }
    }

    let profile = run_cmd_stdout("powerprofilesctl", &["get"])
        .trim()
        .to_string();
    if !profile.is_empty() {
        state.profile = profile;
        state.has_profiles = true;
    }
    state.keep_awake = runtime_path("wt-rust-keep-awake").exists();
    state
}

fn wake_ui(slot: &Arc<Mutex<Option<egui::Context>>>) {
    if let Ok(guard) = slot.lock()
        && let Some(ref ctx) = *guard
    {
        ctx.request_repaint();
    }
}

fn wifi_signal_icon(sig: u8) -> &'static str {
    match sig {
        80..=100 => "󰤨",
        60..=79 => "󰤥",
        40..=59 => "󰤢",
        20..=39 => "󰤟",
        _ => "󰤯",
    }
}

fn battery_icon(cap: u8, charging: bool) -> &'static str {
    if charging {
        return "󰂄";
    }
    match cap {
        95..=100 => "󰁹",
        85..=94 => "󰂂",
        75..=84 => "󰂁",
        65..=74 => "󰂀",
        55..=64 => "󰁿",
        45..=54 => "󰁾",
        35..=44 => "󰁽",
        25..=34 => "󰁼",
        15..=24 => "󰁻",
        5..=14 => "󰁺",
        _ => "󰂎",
    }
}

fn format_duration(mins: u32) -> String {
    let h = mins / 60;
    let m = mins % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

fn configure_visuals(ctx: &egui::Context, pal: &Palette) {
    let bg_rgb = parse_hex_rgb(&pal.bg);
    let accent = c32(&pal.accent, 255);
    let text = c32(&pal.text_color, 255);
    let surface = mix_c32(bg_rgb, (255, 255, 255), 0.07, 255);
    let hover = mix_c32(bg_rgb, (255, 255, 255), 0.12, 255);

    let mut v = egui::Visuals::dark();
    v.window_fill = egui::Color32::TRANSPARENT;
    v.panel_fill = egui::Color32::TRANSPARENT;
    v.extreme_bg_color = mix_c32(bg_rgb, (0, 0, 0), 0.25, 255);
    v.selection.bg_fill = c32(&pal.accent, 75);
    v.selection.stroke = egui::Stroke::new(1.0, accent);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text);
    v.widgets.inactive.bg_fill = surface;
    v.widgets.inactive.weak_bg_fill = surface;
    v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, c32(&pal.accent, 80));
    v.widgets.active.bg_fill = c32(&pal.accent, 65);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
    ctx.set_visuals(v);
}

fn draw_toggle_switch(
    ui: &mut egui::Ui,
    on: bool,
    accent: egui::Color32,
    off_bg: egui::Color32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(38.0, 20.0), egui::Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let fill = if on { accent } else { off_bg };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(10), fill);
    let thumb_x = if on {
        rect.right() - 10.0
    } else {
        rect.left() + 10.0
    };
    ui.painter().circle_filled(
        egui::pos2(thumb_x, rect.center().y),
        7.0,
        egui::Color32::WHITE,
    );
    resp.clicked()
}

fn draw_icon_btn(
    ui: &mut egui::Ui,
    icon: &str,
    fg: egui::Color32,
    bg: egui::Color32,
    hover_bg: egui::Color32,
    enabled: bool,
) -> bool {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(26.0, 24.0), sense);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let fill = if enabled && resp.hovered() {
        hover_bg
    } else {
        bg
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(7), fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(13.0),
        fg,
    );
    enabled && resp.clicked()
}

struct PopupApp {
    mode: PopupMode,
    pal: Palette,
    tx: Sender<PopupEvent>,
    rx: Receiver<PopupEvent>,
    audio: AudioState,
    wifi: WifiState,
    wifi_loaded: bool,
    wifi_loading: bool,
    wifi_status: Option<(String, bool)>,
    bt: BtState,
    bt_loaded: bool,
    bt_loading: bool,
    bt_status: Option<(String, bool)>,
    battery: BatteryState,
    sys: FullStats,
    last_sys_refresh: std::time::Instant,
    pending_pw_ssid: Option<String>,
    password_buf: String,
    show_password: bool,
    ctx_slot: Arc<Mutex<Option<egui::Context>>>,
    saw_focus: bool,
    opened_at: std::time::Instant,
    _sock_guard: SocketGuard,
}

impl PopupApp {
    fn new(
        mode: PopupMode,
        tx: Sender<PopupEvent>,
        rx: Receiver<PopupEvent>,
        ctx_slot: Arc<Mutex<Option<egui::Context>>>,
        sock_guard: SocketGuard,
    ) -> Self {
        let cache = load_popup_cache();
        let wifi_loaded = cache.wifi.is_some();
        let bt_loaded = cache.bt.is_some();
        let battery = if mode == PopupMode::Battery {
            query_battery_state()
        } else {
            cache.battery.unwrap_or_default()
        };
        let sys = if mode == PopupMode::System {
            collect_stats(".wt_btop_popup_state.json", true)
        } else {
            FullStats::default()
        };

        let app = Self {
            mode,
            pal: get_current_palette(),
            tx,
            rx,
            audio: cache.audio.unwrap_or_default(),
            wifi: cache.wifi.unwrap_or_default(),
            wifi_loaded,
            wifi_loading: false,
            wifi_status: None,
            bt: cache.bt.unwrap_or_default(),
            bt_loaded,
            bt_loading: false,
            bt_status: None,
            battery,
            sys,
            last_sys_refresh: std::time::Instant::now(),
            pending_pw_ssid: None,
            password_buf: String::new(),
            show_password: false,
            ctx_slot,
            saw_focus: false,
            opened_at: std::time::Instant::now(),
            _sock_guard: sock_guard,
        };
        app.trigger_refresh(false);
        app
    }

    fn persist_cache(&self) {
        save_popup_cache(&PopupCache {
            audio: (!self.audio.sinks.is_empty()).then(|| self.audio.clone()),
            wifi: self.wifi_loaded.then(|| self.wifi.clone()),
            bt: self.bt_loaded.then(|| self.bt.clone()),
            battery: self.battery.present.then(|| self.battery.clone()),
        });
    }

    fn trigger_refresh(&self, rescan: bool) {
        let tx = self.tx.clone();
        let ctx_slot = Arc::clone(&self.ctx_slot);
        match self.mode {
            PopupMode::Volume => {
                thread::spawn(move || {
                    let _ = tx.send(PopupEvent::AudioLoaded(query_audio_state()));
                    wake_ui(&ctx_slot);
                });
            }
            PopupMode::Wifi => {
                thread::spawn(move || {
                    let _ = tx.send(PopupEvent::WifiLoaded(query_wifi_state(rescan)));
                    wake_ui(&ctx_slot);
                });
            }
            PopupMode::Bluetooth => {
                thread::spawn(move || {
                    if rescan {
                        let _ = Command::new("bluetoothctl")
                            .args(["--timeout", "5", "scan", "on"])
                            .status();
                    }
                    let _ = tx.send(PopupEvent::BtLoaded(query_bt_state()));
                    wake_ui(&ctx_slot);
                });
            }
            PopupMode::Battery => {
                thread::spawn(move || {
                    let _ = tx.send(PopupEvent::BatteryLoaded(query_battery_state()));
                    wake_ui(&ctx_slot);
                });
            }
            PopupMode::System => {
                thread::spawn(move || {
                    let _ = tx.send(PopupEvent::SystemLoaded(Box::new(collect_stats(
                        ".wt_btop_popup_state.json",
                        true,
                    ))));
                    wake_ui(&ctx_slot);
                });
            }
            PopupMode::Power => {}
        }
    }

    fn switch_mode(&mut self, next: PopupMode, ctx: &egui::Context) {
        if self.mode == next {
            return;
        }
        self.mode = next;
        self.pal = get_current_palette();
        configure_visuals(ctx, &self.pal);
        self.pending_pw_ssid = None;
        self.wifi_status = None;
        self.bt_status = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next.size().into()));
        self.trigger_refresh(false);
    }

    fn draw_tab_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let bar_bg = mix_c32(bg_rgb, (0, 0, 0), 0.28, 220);
        let accent = c32(&self.pal.accent, 255);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 210);

        let tabs = [
            (PopupMode::Volume, "󰕾", "Audio"),
            (PopupMode::Wifi, "󰤨", "Wi-Fi"),
            (PopupMode::Bluetooth, "󰂯", "BT"),
            (PopupMode::Battery, "󰁹", "Bat"),
            (PopupMode::System, "󰍛", "Sys"),
        ];

        egui::Frame::new()
            .fill(bar_bg)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(3))
            .show(ui, |ui| {
                let w = ((ui.available_width() - 12.0) / 5.0).max(52.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for (tab_mode, icon, label) in tabs {
                        let active = self.mode == tab_mode;
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(w, 26.0), egui::Sense::click());
                        if resp.hovered() {
                            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let fill = if active {
                            c32(&self.pal.accent, 46)
                        } else if resp.hovered() {
                            mix_c32(bg_rgb, (255, 255, 255), 0.08, 180)
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(7), fill);
                        if active {
                            ui.painter().rect_stroke(
                                rect,
                                egui::CornerRadius::same(7),
                                egui::Stroke::new(1.0, c32(&self.pal.accent, 90)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let fg = if active {
                            accent
                        } else if resp.hovered() {
                            text
                        } else {
                            dim
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{} {}", icon, label),
                            egui::FontId::proportional(11.0),
                            fg,
                        );
                        if resp.clicked() {
                            self.switch_mode(tab_mode, ctx);
                        }
                    }
                });
            });
    }

    fn draw_pill_slider(
        pal: &Palette,
        ui: &mut egui::Ui,
        label: &str,
        pct: &mut u8,
        muted: bool,
        icon: &str,
        muted_icon: &str,
    ) -> (bool, Option<u8>) {
        let bg_rgb = parse_hex_rgb(&pal.bg);
        let track_bg = mix_c32(bg_rgb, (255, 255, 255), 0.07, 235);
        let text = c32(&pal.text_color, 255);
        let dim = c32(&pal.text_dim, 220);
        let accent = c32(&pal.accent, 255);
        let danger = egui::Color32::from_rgb(243, 139, 168);

        let mut toggle_mute = false;
        let mut new_pct = None;

        ui.horizontal(|ui| {
            ui.add(egui::Label::new(egui::RichText::new(label).size(11.5).color(text)).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let readout = if muted {
                    "Muted".to_string()
                } else {
                    format!("{}%", *pct)
                };
                ui.label(
                    egui::RichText::new(readout)
                        .size(11.0)
                        .strong()
                        .color(if muted { danger } else { dim }),
                );
            });
        });

        ui.add_space(3.0);

        let total_w = ui.available_width();
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(total_w, 28.0), egui::Sense::click_and_drag());
        let icon_zone_w = 32.0;
        let bar_left = rect.left() + icon_zone_w;
        let bar_w = (rect.width() - icon_zone_w).max(40.0);

        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 1.0 {
                let step = if scroll_y > 0.0 { 5i16 } else { -5i16 };
                let next = snap_five((*pct as i16 + step).clamp(0, 100) as f32);
                if next != *pct {
                    *pct = next;
                    new_pct = Some(next);
                }
            }
        }

        if let Some(pos) = resp.interact_pointer_pos() {
            if resp.clicked() && pos.x < bar_left {
                toggle_mute = true;
            } else if resp.clicked() || resp.dragged() {
                let ratio = ((pos.x - bar_left) / bar_w).clamp(0.0, 1.0);
                let snapped = snap_five(ratio * 100.0);
                if snapped != *pct {
                    *pct = snapped;
                    new_pct = Some(snapped);
                }
            }
        }

        let p = ui.painter();
        p.rect_filled(rect, egui::CornerRadius::same(9), track_bg);

        let frac = (*pct as f32 / 100.0).clamp(0.0, 1.0);
        let fill_color = if muted {
            egui::Color32::from_rgba_unmultiplied(243, 139, 168, 65)
        } else {
            accent
        };
        if frac > 0.0 {
            let fill_w = (icon_zone_w + bar_w * frac).min(rect.width());
            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height()));
            p.rect_filled(fill_rect, egui::CornerRadius::same(9), fill_color);
        }

        let icon_fg = if muted {
            danger
        } else if frac > 0.0 {
            egui::Color32::from_rgb(17, 17, 27)
        } else {
            dim
        };
        p.text(
            egui::pos2(rect.left() + icon_zone_w * 0.5, rect.center().y),
            egui::Align2::CENTER_CENTER,
            if muted { muted_icon } else { icon },
            egui::FontId::proportional(13.5),
            icon_fg,
        );

        (toggle_mute, new_pct)
    }

    fn draw_section_label(ui: &mut egui::Ui, label: &str, dim: egui::Color32) {
        ui.label(egui::RichText::new(label).size(10.0).strong().color(dim));
        ui.add_space(4.0);
    }

    fn render_volume_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 195);
        let accent = c32(&self.pal.accent, 255);
        let pal = self.pal.clone();

        egui::ScrollArea::vertical()
            .max_height(334.0)
            .show(ui, |ui| {
                // 1. Output (Speaker / Headphones)
                Self::draw_section_label(ui, "OUTPUT", dim);
                let default_sink_name = self.audio.default_sink_name.clone();
                if let Some(sink) = self
                    .audio
                    .sinks
                    .iter_mut()
                    .find(|s| s.id == default_sink_name)
                {
                    let sink_id = sink.id.clone();
                    let sink_label = sink.label.clone();
                    let mut pct = sink.percent;
                    let muted = sink.muted;
                    let (toggled, changed) =
                        Self::draw_pill_slider(&pal, ui, &sink_label, &mut pct, muted, "󰕾", "󰖁");
                    sink.percent = pct;
                    if toggled {
                        let _ = run_cmd_quiet("pactl", &["set-sink-mute", &sink_id, "toggle"]);
                        self.trigger_refresh(false);
                    }
                    if let Some(p) = changed {
                        let arg = format!("{}%", p);
                        let _ = run_cmd_quiet("pactl", &["set-sink-volume", &sink_id, &arg]);
                    }
                } else {
                    ui.label(
                        egui::RichText::new("No active output device")
                            .size(11.5)
                            .color(dim),
                    );
                }

                if self.audio.sinks.len() > 1 {
                    ui.add_space(5.0);
                    let sinks_copy = self.audio.sinks.clone();
                    for s in sinks_copy {
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 26.0),
                            egui::Sense::click(),
                        );
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        let fill = if s.is_default {
                            c32(&self.pal.accent, 34)
                        } else if resp.hovered() {
                            mix_c32(bg_rgb, (255, 255, 255), 0.07, 210)
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(7), fill);
                        let dot = if s.is_default { "●" } else { "○" };
                        let fg = if s.is_default { accent } else { text };
                        ui.painter().text(
                            egui::pos2(rect.left() + 10.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{}  {}", dot, s.label),
                            egui::FontId::proportional(11.0),
                            fg,
                        );
                        if resp.clicked() && !s.is_default {
                            let _ = run_cmd_quiet("pactl", &["set-default-sink", &s.id]);
                            self.trigger_refresh(false);
                        }
                    }
                }

                // 2. Input (Microphone)
                ui.add_space(12.0);
                Self::draw_section_label(ui, "INPUT", dim);
                if let Some(src) = self.audio.sources.iter_mut().find(|s| s.is_default) {
                    let src_id = src.id.clone();
                    let src_label = src.label.clone();
                    let mut pct = src.percent;
                    let muted = src.muted;
                    let (toggled, changed) =
                        Self::draw_pill_slider(&pal, ui, &src_label, &mut pct, muted, "󰍬", "󰍭");
                    src.percent = pct;
                    if toggled {
                        let _ = run_cmd_quiet("pactl", &["set-source-mute", &src_id, "toggle"]);
                        self.trigger_refresh(false);
                    }
                    if let Some(p) = changed {
                        let arg = format!("{}%", p);
                        let _ = run_cmd_quiet("pactl", &["set-source-volume", &src_id, &arg]);
                    }
                } else {
                    ui.label(
                        egui::RichText::new("No active microphone")
                            .size(11.5)
                            .color(dim),
                    );
                }

                // 3. Applications
                if !self.audio.streams.is_empty() {
                    ui.add_space(12.0);
                    Self::draw_section_label(ui, "APPS", dim);
                    for idx in 0..self.audio.streams.len() {
                        let sid = self.audio.streams[idx].id.clone();
                        let label = self.audio.streams[idx].label.clone();
                        let mut pct = self.audio.streams[idx].percent;
                        let muted = self.audio.streams[idx].muted;
                        let (toggled, changed) =
                            Self::draw_pill_slider(&pal, ui, &label, &mut pct, muted, "󰎈", "󰖁");
                        self.audio.streams[idx].percent = pct;
                        if toggled {
                            let _ =
                                run_cmd_quiet("pactl", &["set-sink-input-mute", &sid, "toggle"]);
                            self.trigger_refresh(false);
                        }
                        if let Some(p) = changed {
                            let arg = format!("{}%", p);
                            let _ = run_cmd_quiet("pactl", &["set-sink-input-volume", &sid, &arg]);
                        }
                        ui.add_space(6.0);
                    }
                }
            });
    }

    fn connect_wifi(&mut self, ssid: String, password: Option<String>) {
        self.wifi_loading = true;
        self.wifi_status = Some((format!("Connecting to {}...", ssid), false));
        let tx = self.tx.clone();
        thread::spawn(move || {
            let mut args = vec!["dev", "wifi", "connect", &ssid];
            if let Some(ref pw) = password {
                args.push("password");
                args.push(pw);
            }
            let res = run_cmd_quiet("nmcli", &args);
            let st = query_wifi_state(false);
            let (msg, is_err) = match res {
                Ok(()) => (format!("Connected to {}", ssid), false),
                Err(e) => (e, true),
            };
            let _ = tx.send(PopupEvent::WifiStatus(st, msg, is_err));
        });
    }

    fn disconnect_wifi(&mut self, device: String) {
        self.wifi_loading = true;
        self.wifi_status = Some(("Disconnecting...".to_string(), false));
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = run_cmd_quiet("nmcli", &["dev", "disconnect", &device]);
            let st = query_wifi_state(false);
            let _ = tx.send(PopupEvent::WifiStatus(
                st,
                "Disconnected".to_string(),
                false,
            ));
        });
    }

    fn render_wifi_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 200);
        let accent = c32(&self.pal.accent, 255);
        let surface = mix_c32(bg_rgb, (255, 255, 255), 0.07, 235);
        let hover = mix_c32(bg_rgb, (255, 255, 255), 0.12, 235);

        // Header row: Wi-Fi + IP pill | Rescan + Toggle Switch
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Wi-Fi").size(13.0).strong().color(text));
            if !self.wifi.ip_addr.is_empty() {
                ui.label(
                    egui::RichText::new(&self.wifi.ip_addr)
                        .size(10.5)
                        .color(dim),
                );
            } else if let Some(ref eth) = self.wifi.eth_line {
                ui.label(egui::RichText::new(eth).size(10.5).color(dim));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if draw_toggle_switch(ui, self.wifi.enabled, accent, surface) {
                    let target = if self.wifi.enabled { "off" } else { "on" };
                    let _ = run_cmd_quiet("nmcli", &["radio", "wifi", target]);
                    self.wifi_loading = true;
                    self.trigger_refresh(false);
                }
                if self.wifi.enabled {
                    ui.add_space(6.0);
                    let icon = if self.wifi_loading { "󰔟" } else { "󰑐" };
                    if draw_icon_btn(ui, icon, dim, surface, hover, !self.wifi_loading) {
                        self.wifi_loading = true;
                        self.wifi_status = Some(("Scanning networks...".to_string(), false));
                        self.trigger_refresh(true);
                    }
                }
            });
        });

        if let Some((ref msg, is_err)) = self.wifi_status {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(msg).size(10.5).color(if is_err {
                egui::Color32::from_rgb(243, 139, 168)
            } else {
                c32(&self.pal.accent_secondary, 255)
            }));
        }

        ui.add_space(8.0);

        if !self.wifi_loaded {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Loading networks...")
                        .size(12.0)
                        .color(dim),
                );
            });
        } else if !self.wifi.enabled {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("󰤭").size(28.0).color(dim));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Wi-Fi is turned off")
                        .size(12.0)
                        .color(dim),
                );
            });
        } else {
            let wifi_dev = self.wifi.wifi_device.clone();
            egui::ScrollArea::vertical()
                .max_height(286.0)
                .show(ui, |ui| {
                    let nets = self.wifi.networks.clone();
                    if nets.is_empty() {
                        ui.label(
                            egui::RichText::new("No networks found — click refresh")
                                .size(11.5)
                                .color(dim),
                        );
                    }
                    for net in nets {
                        let is_sec = !net.security.is_empty() && net.security != "--";
                        let expanded = self.pending_pw_ssid.as_deref() == Some(&net.ssid);
                        let row_bg = if net.in_use {
                            c32(&self.pal.accent, 34)
                        } else if expanded {
                            surface
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let frame = egui::Frame::new()
                            .fill(row_bg)
                            .corner_radius(egui::CornerRadius::same(9))
                            .inner_margin(egui::Margin::symmetric(8, 6));

                        frame.show(ui, |ui| {
                            let row = ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(wifi_signal_icon(net.signal))
                                        .size(14.0)
                                        .color(if net.in_use { accent } else { dim }),
                                );
                                ui.add(
                                    egui::Label::new(if net.in_use {
                                        egui::RichText::new(&net.ssid)
                                            .size(12.0)
                                            .strong()
                                            .color(text)
                                    } else {
                                        egui::RichText::new(&net.ssid).size(12.0).color(text)
                                    })
                                    .truncate(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if net.in_use {
                                            if let Some(ref dev) = wifi_dev
                                                && ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new("Disconnect")
                                                                .size(10.5)
                                                                .color(text),
                                                        )
                                                        .fill(surface)
                                                        .corner_radius(egui::CornerRadius::same(6)),
                                                    )
                                                    .clicked()
                                            {
                                                self.disconnect_wifi(dev.clone());
                                            }
                                        } else {
                                            ui.label(
                                                egui::RichText::new(format!("{}%", net.signal))
                                                    .size(10.5)
                                                    .color(dim),
                                            );
                                            if is_sec {
                                                ui.label(
                                                    egui::RichText::new("󰌾").size(11.0).color(dim),
                                                );
                                            }
                                            if net.saved {
                                                ui.label(
                                                    egui::RichText::new("●")
                                                        .size(8.0)
                                                        .color(accent),
                                                );
                                            }
                                        }
                                    },
                                );
                            });

                            let click = ui.interact(
                                row.response.rect,
                                ui.scope_id().with(&net.ssid),
                                egui::Sense::click(),
                            );
                            if !net.in_use && click.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                if !expanded {
                                    ui.painter().rect_filled(
                                        row.response.rect.expand2(egui::vec2(6.0, 4.0)),
                                        egui::CornerRadius::same(8),
                                        mix_c32(bg_rgb, (255, 255, 255), 0.06, 70),
                                    );
                                }
                            }
                            if click.clicked() && !net.in_use {
                                if net.saved || !is_sec {
                                    self.pending_pw_ssid = None;
                                    self.connect_wifi(net.ssid.clone(), None);
                                } else if expanded {
                                    self.pending_pw_ssid = None;
                                } else {
                                    self.pending_pw_ssid = Some(net.ssid.clone());
                                    self.password_buf.clear();
                                }
                            }

                            if expanded {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let pw_edit = ui.add(
                                        egui::TextEdit::singleline(&mut self.password_buf)
                                            .password(!self.show_password)
                                            .hint_text("Password...")
                                            .desired_width(
                                                (ui.available_width() - 68.0).max(120.0),
                                            ),
                                    );
                                    if pw_edit.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        && !self.password_buf.is_empty()
                                    {
                                        let pw = std::mem::take(&mut self.password_buf);
                                        self.pending_pw_ssid = None;
                                        self.connect_wifi(net.ssid.clone(), Some(pw));
                                    }
                                    let eye = if self.show_password { "󰈉" } else { "󰈈" };
                                    if draw_icon_btn(ui, eye, dim, surface, hover, true) {
                                        self.show_password = !self.show_password;
                                    }
                                    let can_go = !self.password_buf.is_empty();
                                    if draw_icon_btn(
                                        ui,
                                        "󰁔",
                                        egui::Color32::from_rgb(17, 17, 27),
                                        accent,
                                        accent,
                                        can_go,
                                    ) {
                                        let pw = std::mem::take(&mut self.password_buf);
                                        self.pending_pw_ssid = None;
                                        self.connect_wifi(net.ssid.clone(), Some(pw));
                                    }
                                });
                            }
                        });
                        ui.add_space(2.0);
                    }
                });
        }

        ui.add_space(6.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let link = ui.add(
                egui::Label::new(
                    egui::RichText::new("Network Settings ↗")
                        .size(10.5)
                        .color(dim),
                )
                .sense(egui::Sense::click()),
            );
            if link.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if link.clicked() {
                let _ = Command::new("nm-connection-editor").spawn();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn render_bt_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 200);
        let accent = c32(&self.pal.accent, 255);
        let surface = mix_c32(bg_rgb, (255, 255, 255), 0.07, 235);
        let hover = mix_c32(bg_rgb, (255, 255, 255), 0.12, 235);

        let connected_cnt = self.bt.devices.iter().filter(|d| d.connected).count();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Bluetooth")
                    .size(13.0)
                    .strong()
                    .color(text),
            );
            if self.bt.powered && connected_cnt > 0 {
                ui.label(
                    egui::RichText::new(format!("{} connected", connected_cnt))
                        .size(10.5)
                        .color(dim),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if draw_toggle_switch(ui, self.bt.powered, accent, surface) && !self.bt_loading {
                    self.bt_loading = true;
                    let next = if self.bt.powered { "off" } else { "on" };
                    let tx = self.tx.clone();
                    thread::spawn(move || {
                        let _ = Command::new("bluetoothctl").args(["power", next]).status();
                        let _ = tx.send(PopupEvent::BtLoaded(query_bt_state()));
                    });
                }
                if self.bt.powered {
                    ui.add_space(6.0);
                    let icon = if self.bt_loading { "󰔟" } else { "󰑐" };
                    if draw_icon_btn(ui, icon, dim, surface, hover, !self.bt_loading) {
                        self.bt_loading = true;
                        self.bt_status = Some(("Scanning nearby devices...".to_string(), false));
                        self.trigger_refresh(true);
                    }
                }
            });
        });

        if let Some((ref msg, is_err)) = self.bt_status {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(msg).size(10.5).color(if is_err {
                egui::Color32::from_rgb(243, 139, 168)
            } else {
                c32(&self.pal.accent_secondary, 255)
            }));
        }

        ui.add_space(8.0);

        if !self.bt_loaded {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Loading devices...")
                        .size(12.0)
                        .color(dim),
                );
            });
            return;
        }

        if !self.bt.powered {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("󰂲").size(28.0).color(dim));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Bluetooth is turned off")
                        .size(12.0)
                        .color(dim),
                );
            });
            return;
        }

        if self.bt.devices.is_empty() {
            ui.label(
                egui::RichText::new("No devices found — click refresh to scan")
                    .size(11.5)
                    .color(dim),
            );
            return;
        }

        let devices = self.bt.devices.clone();
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for dev in devices {
                    let row_bg = if dev.connected {
                        c32(&self.pal.accent, 34)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    egui::Frame::new()
                        .fill(row_bg)
                        .corner_radius(egui::CornerRadius::same(9))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(if dev.connected {
                                        "󰂱"
                                    } else {
                                        "󰂯"
                                    })
                                    .size(14.0)
                                    .color(if dev.connected { accent } else { dim }),
                                );
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&dev.name).size(12.0).color(text),
                                    )
                                    .truncate(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let act = if dev.connected {
                                            "Disconnect"
                                        } else {
                                            "Connect"
                                        };
                                        let btn = egui::Button::new(
                                            egui::RichText::new(act).size(10.5).color(text),
                                        )
                                        .fill(if dev.connected {
                                            surface
                                        } else {
                                            c32(&self.pal.accent, 52)
                                        })
                                        .corner_radius(egui::CornerRadius::same(6));
                                        if ui.add_enabled(!self.bt_loading, btn).clicked() {
                                            self.bt_loading = true;
                                            let mac = dev.mac.clone();
                                            let name = dev.name.clone();
                                            let is_conn = dev.connected;
                                            let is_paired = dev.paired;
                                            let tx = self.tx.clone();
                                            thread::spawn(move || {
                                                let ok = if is_conn {
                                                    Command::new("bluetoothctl")
                                                        .args(["disconnect", &mac])
                                                        .status()
                                                        .is_ok_and(|s| s.success())
                                                } else {
                                                    if !is_paired {
                                                        let _ = Command::new("bluetoothctl")
                                                            .args(["pair", &mac])
                                                            .status();
                                                        let _ = Command::new("bluetoothctl")
                                                            .args(["trust", &mac])
                                                            .status();
                                                    }
                                                    Command::new("bluetoothctl")
                                                        .args(["connect", &mac])
                                                        .status()
                                                        .is_ok_and(|s| s.success())
                                                };
                                                let msg = if ok {
                                                    if is_conn {
                                                        format!("Disconnected {}", name)
                                                    } else {
                                                        format!("Connected to {}", name)
                                                    }
                                                } else {
                                                    format!("Failed for {}", name)
                                                };
                                                let _ = tx.send(PopupEvent::BtStatus(
                                                    query_bt_state(),
                                                    msg,
                                                    !ok,
                                                ));
                                            });
                                        }
                                        if let Some(bat) = dev.battery {
                                            ui.label(
                                                egui::RichText::new(format!("󰥉 {}%", bat))
                                                    .size(10.5)
                                                    .color(dim),
                                            );
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(2.0);
                }
            });
    }

    fn render_battery_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 200);
        let accent = c32(&self.pal.accent, 255);
        let surface = mix_c32(bg_rgb, (255, 255, 255), 0.06, 235);
        let track_bg = mix_c32(bg_rgb, (0, 0, 0), 0.30, 235);

        let charging = self.battery.status == "Charging";
        let cap = self.battery.capacity;
        let state_color = if charging {
            egui::Color32::from_rgb(166, 227, 161)
        } else if cap <= 15 {
            egui::Color32::from_rgb(243, 139, 168)
        } else if cap <= 30 {
            egui::Color32::from_rgb(250, 179, 135)
        } else {
            accent
        };

        // Hero Charge Card
        egui::Frame::new()
            .fill(surface)
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(battery_icon(cap, charging))
                            .size(28.0)
                            .color(state_color),
                    );
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(if self.battery.present {
                                format!("{}%", cap)
                            } else {
                                "AC Power".to_string()
                            })
                            .size(20.0)
                            .strong()
                            .color(text),
                        );
                        let detail = match (self.battery.status.as_str(), self.battery.time_mins) {
                            ("Discharging", Some(m)) => {
                                format!("Discharging · ~{} left", format_duration(m))
                            }
                            ("Charging", Some(m)) => {
                                format!("Charging · ~{} to full", format_duration(m))
                            }
                            (s, _) if !s.is_empty() => s.to_string(),
                            _ => "Desktop / AC".to_string(),
                        };
                        ui.label(egui::RichText::new(detail).size(11.0).color(dim));
                    });
                });

                ui.add_space(10.0);

                let (bar_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 8.0),
                    egui::Sense::hover(),
                );
                let p = ui.painter();
                p.rect_filled(bar_rect, egui::CornerRadius::same(4), track_bg);
                let frac = (cap as f32 / 100.0).clamp(0.0, 1.0);
                if frac > 0.0 {
                    let fw = (bar_rect.width() * frac).max(8.0);
                    p.rect_filled(
                        egui::Rect::from_min_size(bar_rect.min, egui::vec2(fw, bar_rect.height())),
                        egui::CornerRadius::same(4),
                        state_color,
                    );
                }
            });

        // Power Profile Segmented Selector
        if self.battery.has_profiles {
            ui.add_space(12.0);
            Self::draw_section_label(ui, "POWER PROFILE", dim);

            let profiles = [
                ("power-saver", "󰌪", "Saver"),
                ("balanced", "󰗑", "Balanced"),
                ("performance", "󱐋", "Perf"),
            ];
            egui::Frame::new()
                .fill(track_bg)
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::same(3))
                .show(ui, |ui| {
                    let w = ((ui.available_width() - 6.0) / 3.0).max(70.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        for (id, icon, label) in profiles {
                            let active = self.battery.profile == id;
                            let (rect, resp) =
                                ui.allocate_exact_size(egui::vec2(w, 30.0), egui::Sense::click());
                            if resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            let fill = if active {
                                accent
                            } else if resp.hovered() {
                                surface
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter()
                                .rect_filled(rect, egui::CornerRadius::same(7), fill);
                            let fg = if active {
                                egui::Color32::from_rgb(17, 17, 27)
                            } else if resp.hovered() {
                                text
                            } else {
                                dim
                            };
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{} {}", icon, label),
                                egui::FontId::proportional(11.5),
                                fg,
                            );
                            if resp.clicked() && !active {
                                self.battery.profile = id.to_string();
                                let target = id.to_string();
                                let tx = self.tx.clone();
                                let ctx_slot = Arc::clone(&self.ctx_slot);
                                thread::spawn(move || {
                                    let _ = run_cmd_quiet("powerprofilesctl", &["set", &target]);
                                    let _ =
                                        tx.send(PopupEvent::BatteryLoaded(query_battery_state()));
                                    wake_ui(&ctx_slot);
                                });
                            }
                        }
                    });
                });
        }

        // Compact 3-Stat Strip (Power / Health / Cycles)
        ui.add_space(10.0);
        let power_txt = self
            .battery
            .power_watts
            .map(|w| format!("{:.1} W", w))
            .unwrap_or_else(|| "—".to_string());
        let health_txt = self
            .battery
            .health_pct
            .map(|h| format!("{}%", h))
            .unwrap_or_else(|| "—".to_string());
        let cycles_txt = self
            .battery
            .cycles
            .map(|c| c.to_string())
            .unwrap_or_else(|| "—".to_string());

        let stat_w = ((ui.available_width() - 12.0) / 3.0).max(70.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (k, v) in [
                ("POWER", power_txt.as_str()),
                ("HEALTH", health_txt.as_str()),
                ("CYCLES", cycles_txt.as_str()),
            ] {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(stat_w, 48.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(10), surface);
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.top() + 14.0),
                    egui::Align2::CENTER_CENTER,
                    k,
                    egui::FontId::proportional(9.5),
                    dim,
                );
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.bottom() - 15.0),
                    egui::Align2::CENTER_CENTER,
                    v,
                    egui::FontId::proportional(13.0),
                    text,
                );
            }
        });

        // Hold Wake Toggle (prevents hypridle dim/lock/dpms/suspend)
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(surface)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("󰅶")
                            .size(15.0)
                            .color(if self.battery.keep_awake { accent } else { dim }),
                    );
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Hold Wake")
                                .size(12.0)
                                .strong()
                                .color(text),
                        );
                        ui.label(
                            egui::RichText::new("Keep screen unlocked")
                                .size(10.0)
                                .color(dim),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if draw_toggle_switch(ui, self.battery.keep_awake, accent, track_bg) {
                            let next = !self.battery.keep_awake;
                            self.battery.keep_awake = next;
                            let flag = runtime_path("wt-rust-keep-awake");
                            if next {
                                let _ = std::fs::write(&flag, b"1");
                            } else {
                                let _ = std::fs::remove_file(&flag);
                            }
                            self.persist_cache();
                            let _ = run_cmd_quiet("pkill", &["-RTMIN+8", "waybar"]);
                        }
                    });
                });
            });
    }

    fn render_system_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 200);
        let accent = c32(&self.pal.accent, 255);
        let surface = mix_c32(bg_rgb, (255, 255, 255), 0.06, 235);
        let track_bg = mix_c32(bg_rgb, (0, 0, 0), 0.30, 235);

        let load_col = |pct: f64| {
            if pct >= 80.0 {
                egui::Color32::from_rgb(243, 139, 168)
            } else if pct >= 55.0 {
                egui::Color32::from_rgb(250, 179, 135)
            } else {
                accent
            }
        };

        let draw_bar = |ui: &mut egui::Ui, pct: f64, col: egui::Color32| {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(rect, egui::CornerRadius::same(3), track_bg);
            let frac = (pct as f32 / 100.0).clamp(0.0, 1.0);
            if frac > 0.0 {
                let fw = (rect.width() * frac).max(6.0);
                p.rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(fw, rect.height())),
                    egui::CornerRadius::same(3),
                    col,
                );
            }
        };

        // 1. CPU Card
        let cpu = &self.sys.cpu;
        let cpu_col = load_col(cpu.total);
        egui::Frame::new()
            .fill(surface)
            .corner_radius(egui::CornerRadius::same(11))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("󰍛  CPU")
                            .size(12.0)
                            .strong()
                            .color(text),
                    );
                    let meta = if cpu.temp >= 0 {
                        format!("{:.2} GHz · {}°C", cpu.freq, cpu.temp)
                    } else {
                        format!("{:.2} GHz", cpu.freq)
                    };
                    ui.label(egui::RichText::new(meta).size(10.5).color(dim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.0}%", cpu.total))
                                .size(14.0)
                                .strong()
                                .color(cpu_col),
                        );
                    });
                });
                ui.add_space(6.0);
                draw_bar(ui, cpu.total, cpu_col);

                if !cpu.cores.is_empty() {
                    ui.add_space(7.0);
                    let n = cpu.cores.len();
                    let gap = 3.0;
                    let cw = ((ui.available_width() - gap * (n.saturating_sub(1) as f32))
                        / n as f32)
                        .max(4.0);
                    let (cores_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 18.0),
                        egui::Sense::hover(),
                    );
                    let p = ui.painter();
                    for (i, &c) in cpu.cores.iter().enumerate() {
                        let x = cores_rect.left() + i as f32 * (cw + gap);
                        let slot = egui::Rect::from_min_size(
                            egui::pos2(x, cores_rect.top()),
                            egui::vec2(cw, cores_rect.height()),
                        );
                        p.rect_filled(slot, egui::CornerRadius::same(2), track_bg);
                        let h = ((c as f32 / 100.0).clamp(0.0, 1.0) * slot.height()).max(2.0);
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(x, slot.bottom() - h),
                            egui::vec2(cw, h),
                        );
                        p.rect_filled(bar, egui::CornerRadius::same(2), load_col(c));
                    }
                }
            });

        ui.add_space(7.0);

        // 2. GPU Card
        let gpu = &self.sys.gpu;
        let gpu_awake = gpu.vram_total > 0.0 || gpu.usage > 0.0 || gpu.temp > 0.0;
        let gpu_col = load_col(gpu.usage);
        egui::Frame::new()
            .fill(surface)
            .corner_radius(egui::CornerRadius::same(11))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let gname = if gpu.name.is_empty() || gpu.name == "N/A" {
                        "GPU".to_string()
                    } else {
                        gpu.name
                            .replace("NVIDIA GeForce ", "")
                            .replace(" Laptop GPU", "")
                    };
                    ui.label(
                        egui::RichText::new(format!("󰢮  {}", gname))
                            .size(12.0)
                            .strong()
                            .color(text),
                    );
                    if gpu_awake && gpu.temp > 0.0 {
                        ui.label(
                            egui::RichText::new(format!("{:.0}°C", gpu.temp))
                                .size(10.5)
                                .color(dim),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let val = if gpu_awake {
                            format!("{:.0}%", gpu.usage)
                        } else {
                            "Idle / Sleep".to_string()
                        };
                        ui.label(
                            egui::RichText::new(val)
                                .size(if gpu_awake { 14.0 } else { 11.0 })
                                .strong()
                                .color(if gpu_awake { gpu_col } else { dim }),
                        );
                    });
                });
                if gpu_awake {
                    ui.add_space(6.0);
                    draw_bar(ui, gpu.usage, gpu_col);
                    if gpu.vram_total > 0.0 {
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("VRAM").size(10.0).color(dim));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.1} / {:.1} GiB ({:.0}%)",
                                            gpu.vram_used, gpu.vram_total, gpu.vram_pct
                                        ))
                                        .size(10.0)
                                        .color(dim),
                                    );
                                },
                            );
                        });
                    }
                }
            });

        ui.add_space(7.0);

        // 3. RAM & Disk Row (2 Equal Cards)
        let half_w = ((ui.available_width() - 7.0) * 0.5).max(120.0);
        let mem = &self.sys.mem;
        let root_disk = self.sys.disks.first();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            ui.allocate_ui_with_layout(
                egui::vec2(half_w, 60.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(half_w);
                    egui::Frame::new()
                        .fill(surface)
                        .corner_radius(egui::CornerRadius::same(11))
                        .inner_margin(egui::Margin::same(9))
                        .show(ui, |ui| {
                            ui.set_width((half_w - 18.0).max(80.0));
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("󰘚 RAM").size(11.0).strong().color(text),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", mem.pct))
                                                .size(11.5)
                                                .strong()
                                                .color(load_col(mem.pct)),
                                        );
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            draw_bar(ui, mem.pct, load_col(mem.pct));
                            ui.add_space(3.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.1} / {:.1} GiB",
                                    mem.used, mem.total
                                ))
                                .size(10.0)
                                .color(dim),
                            );
                        });
                },
            );

            ui.allocate_ui_with_layout(
                egui::vec2(half_w, 60.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(half_w);
                    egui::Frame::new()
                        .fill(surface)
                        .corner_radius(egui::CornerRadius::same(11))
                        .inner_margin(egui::Margin::same(9))
                        .show(ui, |ui| {
                            ui.set_width((half_w - 18.0).max(80.0));
                            let (dpct, dtxt) = match root_disk {
                                Some(d) => (d.pct, format!("{:.0} / {:.0} GiB", d.used, d.total)),
                                None => (0.0, "—".to_string()),
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("󰋊 DISK")
                                        .size(11.0)
                                        .strong()
                                        .color(text),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", dpct))
                                                .size(11.5)
                                                .strong()
                                                .color(load_col(dpct)),
                                        );
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            draw_bar(ui, dpct, load_col(dpct));
                            ui.add_space(3.0);
                            ui.label(egui::RichText::new(dtxt).size(10.0).color(dim));
                        });
                },
            );
        });

        // 4. Top Processes + Uptime
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("TOP PROCESSES")
                    .size(10.0)
                    .strong()
                    .color(dim),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("Up {}", self.sys.uptime))
                        .size(10.0)
                        .color(dim),
                );
            });
        });
        ui.add_space(4.0);
        egui::Frame::new()
            .fill(surface)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(9, 6))
            .show(ui, |ui| {
                if self.sys.procs.is_empty() {
                    ui.label(egui::RichText::new("No process data").size(11.0).color(dim));
                }
                for proc in self.sys.procs.iter().take(5) {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&proc.name).size(11.0).color(text),
                            )
                            .truncate(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{}% MEM", proc.mem))
                                    .size(10.0)
                                    .color(dim),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!("{}% CPU", proc.cpu))
                                    .size(10.5)
                                    .strong()
                                    .color(accent),
                            );
                        });
                    });
                }
            });
    }

    fn draw_power_row(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        icon: &str,
        label: &str,
        tint: egui::Color32,
        cmd: &[&str],
    ) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
        if resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(9),
                mix_c32(bg_rgb, (tint.r(), tint.g(), tint.b()), 0.18, 230),
            );
        }

        let badge = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 20.0, rect.center().y),
            egui::vec2(26.0, 26.0),
        );
        ui.painter().rect_filled(
            badge,
            egui::CornerRadius::same(8),
            egui::Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), 32),
        );
        ui.painter().text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(14.0),
            tint,
        );
        ui.painter().text(
            egui::pos2(badge.right() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            text,
        );

        if resp.clicked() {
            let _ = Command::new(cmd[0]).args(&cmd[1..]).spawn();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn render_power_ui(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let accent = c32(&self.pal.accent, 255);
        let sleep_col = egui::Color32::from_rgb(137, 220, 235);
        let logout_col = c32(&self.pal.accent_secondary, 255);
        let reboot_col = egui::Color32::from_rgb(250, 179, 135);
        let danger_col = egui::Color32::from_rgb(243, 139, 168);

        self.draw_power_row(ui, ctx, "󰌾", "Lock", accent, &["hyprlock"]);
        ui.add_space(2.0);
        self.draw_power_row(
            ui,
            ctx,
            "󰤄",
            "Suspend",
            sleep_col,
            &["systemctl", "suspend"],
        );
        ui.add_space(2.0);
        self.draw_power_row(
            ui,
            ctx,
            "󰍃",
            "Log Out",
            logout_col,
            &["hyprshutdown", "-t", "Logging out..."],
        );

        ui.add_space(6.0);
        let (sep, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(
            sep.shrink2(egui::vec2(8.0, 0.0)),
            egui::CornerRadius::ZERO,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24),
        );
        ui.add_space(6.0);

        self.draw_power_row(
            ui,
            ctx,
            "󰜉",
            "Reboot",
            reboot_col,
            &[
                "hyprshutdown",
                "-t",
                "Rebooting...",
                "-p",
                "systemctl reboot",
            ],
        );
        ui.add_space(2.0);
        self.draw_power_row(
            ui,
            ctx,
            "⏻",
            "Shut Down",
            danger_col,
            &[
                "hyprshutdown",
                "-t",
                "Shutting down...",
                "-p",
                "systemctl poweroff",
            ],
        );
    }
}

impl eframe::App for PopupApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Ok(mut slot) = self.ctx_slot.lock()
            && slot.is_none()
        {
            configure_visuals(&ctx, &self.pal);
            *slot = Some(ctx.clone());
        }

        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                PopupEvent::AudioLoaded(st) => {
                    self.audio = st;
                    self.persist_cache();
                }
                PopupEvent::WifiLoaded(st) => {
                    self.wifi = st;
                    self.wifi_loaded = true;
                    self.wifi_loading = false;
                    self.persist_cache();
                }
                PopupEvent::WifiStatus(st, msg, is_err) => {
                    self.wifi = st;
                    self.wifi_loaded = true;
                    self.wifi_loading = false;
                    self.wifi_status = Some((msg, is_err));
                    self.persist_cache();
                }
                PopupEvent::BtLoaded(st) => {
                    self.bt = st;
                    self.bt_loaded = true;
                    self.bt_loading = false;
                    self.persist_cache();
                }
                PopupEvent::BtStatus(st, msg, is_err) => {
                    self.bt = st;
                    self.bt_loaded = true;
                    self.bt_loading = false;
                    self.bt_status = Some((msg, is_err));
                    self.persist_cache();
                }
                PopupEvent::BatteryLoaded(st) => {
                    self.battery = st;
                    self.persist_cache();
                }
                PopupEvent::SystemLoaded(st) => {
                    self.sys = *st;
                }
                PopupEvent::Trigger(requested) => {
                    if requested == self.mode {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                    self.switch_mode(requested, &ctx);
                }
            }
        }

        if self.mode == PopupMode::System {
            if self.last_sys_refresh.elapsed() >= std::time::Duration::from_millis(1500) {
                self.last_sys_refresh = std::time::Instant::now();
                self.trigger_refresh(false);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(1500));
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let focused = ctx.input(|i| i.viewport().focused);
        if focused == Some(true) {
            self.saw_focus = true;
        } else if self.saw_focus
            && focused == Some(false)
            && self.opened_at.elapsed() > std::time::Duration::from_millis(200)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let bg = c32(&self.pal.bg, 242);
        let border = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24);
        let margin = if self.mode == PopupMode::Power {
            10
        } else {
            14
        };

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.0, border))
                    .corner_radius(egui::CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(margin)),
            )
            .show(ui, |ui| {
                if self.mode == PopupMode::Power {
                    self.render_power_ui(ui, &ctx);
                } else {
                    self.draw_tab_bar(ui, &ctx);
                    ui.add_space(10.0);
                    match self.mode {
                        PopupMode::Volume => self.render_volume_ui(ui),
                        PopupMode::Wifi => self.render_wifi_ui(ui),
                        PopupMode::Bluetooth => self.render_bt_ui(ui),
                        PopupMode::Battery => self.render_battery_ui(ui),
                        PopupMode::System => self.render_system_ui(ui),
                        PopupMode::Power => unreachable!(),
                    }
                }
            });
    }
}

pub fn handle_battery_waybar() {
    let st = query_battery_state();
    let charging = st.status == "Charging";
    let icon = if st.keep_awake {
        "󰅶"
    } else if st.status == "Full" || st.status == "Not charging" {
        "󰚥"
    } else {
        battery_icon(st.capacity, charging)
    };
    let text = if st.present {
        format!("{} {}%", icon, st.capacity)
    } else {
        format!("{} AC", icon)
    };
    let class = if st.keep_awake {
        "awake"
    } else if charging {
        "charging"
    } else if st.capacity <= 15 {
        "critical"
    } else if st.capacity <= 30 {
        "warning"
    } else {
        "normal"
    };
    let mut tip = vec![format!("{} ({}%)", st.status, st.capacity)];
    if let Some(m) = st.time_mins {
        tip.push(format!("Time: ~{}", format_duration(m)));
    }
    if let Some(w) = st.power_watts {
        tip.push(format!("Power: {:.1} W", w));
    }
    if st.has_profiles {
        tip.push(format!("Profile: {}", st.profile));
    }
    if st.keep_awake {
        tip.push("Hold Wake: ON".to_string());
    }
    println!(
        "{}",
        serde_json::json!({
            "text": text,
            "tooltip": tip.join("\n"),
            "class": class,
        })
    );
}

pub fn handle_popup(args: &[String]) {
    let mode = PopupMode::from_arg(args.first().map(|s| s.as_str()));
    let sock_path = match mode {
        PopupMode::Power => runtime_path("wt-rust-power.sock"),
        _ => runtime_path("wt-rust-popup.sock"),
    };
    let app_id = match mode {
        PopupMode::Power => "wt-popup-power",
        _ => "wt-popup",
    };
    let Some((listener, sock_guard)) = bind_single_instance_socket(&sock_path, |mut stream| {
        let _ = stream.write_all(mode.as_str().as_bytes());
    }) else {
        return;
    };

    let (tx, rx) = mpsc::channel::<PopupEvent>();
    let ctx_slot: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

    let tx_ipc = tx.clone();
    let ctx_ipc = Arc::clone(&ctx_slot);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut buf = String::new();
            let mut s = stream;
            if s.read_to_string(&mut buf).is_ok() {
                let req = PopupMode::from_arg(Some(buf.trim()));
                let _ = tx_ipc.send(PopupEvent::Trigger(req));
                if let Ok(guard) = ctx_ipc.lock()
                    && let Some(ref ctx) = *guard
                {
                    ctx.request_repaint();
                }
            }
        }
    });

    let init_size = mode.size();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(app_id)
            .with_title("WT Control Center")
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_inner_size(init_size),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "WT Control Center",
        native_options,
        Box::new(move |cc| {
            install_nerd_font(&cc.egui_ctx);
            Ok(Box::new(PopupApp::new(mode, tx, rx, ctx_slot, sock_guard)))
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_five() {
        assert_eq!(snap_five(0.0), 0);
        assert_eq!(snap_five(47.0), 45);
        assert_eq!(snap_five(48.0), 50);
        assert_eq!(snap_five(104.0), 100);
    }

    #[test]
    fn test_split_nmcli_escaped_colon() {
        let parts = split_nmcli_line("*:My\\:Home\\:WiFi:WPA2:88");
        assert_eq!(parts, vec!["*", "My:Home:WiFi", "WPA2", "88"]);
    }

    #[test]
    fn test_parse_wifi_dedup_and_sort() {
        let raw = " :Cafe:WPA2:40\n*:Home5G:WPA2 WPA3:82\n :Home5G:WPA2:95\n :Guest:--:70\n";
        let mut saved = HashSet::new();
        saved.insert("Guest".to_string());
        let nets = parse_wifi_scan_lines(raw, &saved);
        assert_eq!(nets.len(), 3);
        assert_eq!(nets[0].ssid, "Home5G");
        assert!(nets[0].in_use);
        assert_eq!(nets[1].ssid, "Guest");
        assert!(nets[1].saved);
    }

    #[test]
    fn test_parse_battery_uevent() {
        let raw = "\
POWER_SUPPLY_STATUS=Discharging
POWER_SUPPLY_CAPACITY=53
POWER_SUPPLY_CYCLE_COUNT=148
POWER_SUPPLY_VOLTAGE_NOW=14000000
POWER_SUPPLY_CURRENT_NOW=2000000
POWER_SUPPLY_CHARGE_FULL_DESIGN=4000000
POWER_SUPPLY_CHARGE_FULL=3200000
POWER_SUPPLY_CHARGE_NOW=2000000
";
        let st = parse_battery_uevent(raw);
        assert!(st.present);
        assert_eq!(st.capacity, 53);
        assert_eq!(st.status, "Discharging");
        assert_eq!(st.cycles, Some(148));
        assert_eq!(st.health_pct, Some(80));
        assert_eq!(st.time_mins, Some(60));
        assert!((st.power_watts.unwrap() - 28.0).abs() < 0.1);
    }
}
