//! Pure-Rust GPU-rendered Volume Mixer & Wi-Fi Control Popup (`eframe` / `egui`).
//! Synchronizes its accent/surface palette live from `~/.cache/wt-rust/palette.json`.

use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::theme::{Palette, get_current_palette, parse_hex_rgb};

const POPUP_SOCK: &str = "/tmp/wt-rust-popup.sock";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PopupMode {
    Volume,
    Wifi,
    Bluetooth,
    Power,
}

impl PopupMode {
    fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            Some("wifi") => Self::Wifi,
            Some("bluetooth") | Some("bt") => Self::Bluetooth,
            Some("power") => Self::Power,
            _ => Self::Volume,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Volume => "volume",
            Self::Wifi => "wifi",
            Self::Bluetooth => "bluetooth",
            Self::Power => "power",
        }
    }

    fn size(self) -> [f32; 2] {
        match self {
            Self::Power => [320.0, 256.0],
            Self::Volume => [380.0, 440.0],
            Self::Wifi => [380.0, 460.0],
            Self::Bluetooth => [380.0, 460.0],
        }
    }
}

fn install_nerd_font(ctx: &egui::Context) {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let candidates = [
        home.join(".local/share/fonts/0xProto/0xProtoNerdFont-Regular.ttf"),
        home.join("WT-Dots/fonts/0xProto/0xProtoNerdFont-Regular.ttf"),
    ];
    for path in candidates {
        if let Ok(bytes) = fs::read(&path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "nerd_proto".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("nerd_proto".to_owned());
            ctx.set_fonts(fonts);
            break;
        }
    }
}

enum PopupEvent {
    AudioLoaded(AudioState),
    WifiLoaded(WifiState),
    WifiStatus(WifiState, String, bool),
    BtLoaded(BtState),
    BtStatus(BtState, String, bool),
    Trigger(PopupMode),
}

#[derive(Clone)]
struct BtDevice {
    mac: String,
    name: String,
    connected: bool,
    paired: bool,
    battery: Option<u8>,
}

#[derive(Clone, Default)]
struct BtState {
    powered: bool,
    devices: Vec<BtDevice>,
}

#[derive(Clone, Default)]
struct AudioChannelItem {
    id: String,
    label: String,
    percent: u8,
    muted: bool,
    is_default: bool,
}

#[derive(Clone, Default)]
struct AudioState {
    default_sink_name: String,
    sinks: Vec<AudioChannelItem>,
    sources: Vec<AudioChannelItem>,
    streams: Vec<AudioChannelItem>,
}

#[derive(Clone)]
struct WifiNetwork {
    ssid: String,
    in_use: bool,
    security: String,
    signal: u8,
    saved: bool,
}

#[derive(Clone, Default)]
struct WifiState {
    enabled: bool,
    wifi_device: Option<String>,
    connected_ssid: Option<String>,
    ip_addr: String,
    eth_line: Option<String>,
    networks: Vec<WifiNetwork>,
}

pub(crate) struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Bind a single-instance Unix socket; invokes `on_existing` and returns `None` if an instance is active.
pub(crate) fn bind_single_instance_socket<F>(
    path: &str,
    on_existing: F,
) -> Option<(UnixListener, SocketGuard)>
where
    F: FnOnce(UnixStream),
{
    let sock_path = Path::new(path);
    match UnixListener::bind(sock_path) {
        Ok(l) => Some((l, SocketGuard(sock_path.to_path_buf()))),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            if let Ok(stream) = UnixStream::connect(sock_path) {
                on_existing(stream);
                return None;
            }
            let _ = fs::remove_file(sock_path);
            UnixListener::bind(sock_path)
                .ok()
                .map(|l| (l, SocketGuard(sock_path.to_path_buf())))
        }
        Err(_) => None,
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

    let parse_devices = |raw: &str, default_name: &str, skip_monitors: bool| -> Vec<AudioChannelItem> {
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
        let wifi_raw = run_cmd_stdout(
            "nmcli",
            &[
                "-t",
                "-f",
                "IN-USE,SSID,SECURITY,SIGNAL",
                "dev",
                "wifi",
                "list",
            ],
        );
        parse_wifi_scan_lines(&wifi_raw, &saved_ssids)
    } else {
        Vec::new()
    };

    WifiState {
        enabled,
        wifi_device,
        connected_ssid,
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
                        } else if let Some(b) = t.strip_prefix("Battery Percentage:") {
                            if let Some(open) = b.find('(')
                                && let Some(close) = b.find(')')
                            {
                                battery = b[open + 1..close].trim().parse::<u8>().ok();
                            }
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

struct PopupApp {
    mode: PopupMode,
    pal: Palette,
    tx: Sender<PopupEvent>,
    rx: Receiver<PopupEvent>,
    audio: AudioState,
    wifi: WifiState,
    wifi_loading: bool,
    wifi_status: Option<(String, bool)>,
    bt: BtState,
    bt_loading: bool,
    bt_status: Option<(String, bool)>,
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
        let app = Self {
            mode,
            pal: get_current_palette(),
            tx,
            rx,
            audio: AudioState::default(),
            wifi: WifiState::default(),
            wifi_loading: false,
            wifi_status: None,
            bt: BtState::default(),
            bt_loading: false,
            bt_status: None,
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

    fn trigger_refresh(&self, rescan_wifi: bool) {
        let tx = self.tx.clone();
        match self.mode {
            PopupMode::Volume => {
                thread::spawn(move || {
                    let _ = tx.send(PopupEvent::AudioLoaded(query_audio_state()));
                });
            }
            PopupMode::Wifi => {
                thread::spawn(move || {
                    let _ = tx.send(PopupEvent::WifiLoaded(query_wifi_state(rescan_wifi)));
                });
            }
            PopupMode::Bluetooth => {
                thread::spawn(move || {
                    if rescan_wifi {
                        let _ = Command::new("bluetoothctl")
                            .args(["--timeout", "5", "scan", "on"])
                            .status();
                    }
                    let _ = tx.send(PopupEvent::BtLoaded(query_bt_state()));
                });
            }
            PopupMode::Power => {}
        }
    }

    fn draw_slider_row(
        pal: &Palette,
        ui: &mut egui::Ui,
        title: &str,
        pct: &mut u8,
        muted: bool,
        icon: &str,
    ) -> (bool, Option<u8>) {
        let bg_rgb = parse_hex_rgb(&pal.bg);
        let surface = mix_c32(bg_rgb, (255, 255, 255), 0.06, 235);
        let text = c32(&pal.text_color, 255);
        let accent = c32(&pal.accent, 255);
        let sec = c32(&pal.accent_secondary, 255);
        let muted_c = c32(&pal.text_muted, 255);

        let mut toggle_mute = false;
        let mut new_pct: Option<u8> = None;

        egui::Frame::new()
            .fill(surface)
            .stroke(egui::Stroke::new(1.0, c32(&pal.accent, 44)))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mute_btn = egui::Button::new(
                        egui::RichText::new(if muted { "MUTED" } else { icon })
                            .size(11.5)
                            .strong()
                            .color(if muted {
                                egui::Color32::from_rgb(243, 139, 168)
                            } else {
                                accent
                            }),
                    )
                    .fill(if muted {
                        egui::Color32::from_rgba_unmultiplied(243, 139, 168, 30)
                    } else {
                        c32(&pal.accent, 34)
                    })
                    .corner_radius(egui::CornerRadius::same(7));
                    if ui.add(mute_btn).clicked() {
                        toggle_mute = true;
                    }

                    ui.add(
                        egui::Label::new(egui::RichText::new(title).size(12.5).color(text))
                            .truncate(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let badge_txt = if muted {
                            "Muted".to_string()
                        } else {
                            format!("{}%", *pct)
                        };
                        egui::Frame::new()
                            .fill(c32(&pal.accent, 36))
                            .corner_radius(egui::CornerRadius::same(9))
                            .inner_margin(egui::Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(badge_txt)
                                        .size(11.0)
                                        .strong()
                                        .color(if muted { muted_c } else { accent }),
                                );
                            });
                    });
                });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let step_btn_fill = mix_c32(bg_rgb, (255, 255, 255), 0.10, 255);
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("−5").size(11.0).color(text))
                                .fill(step_btn_fill)
                                .corner_radius(egui::CornerRadius::same(7))
                                .min_size(egui::vec2(30.0, 24.0)),
                        )
                        .clicked()
                    {
                        let next = pct.saturating_sub(5);
                        let snapped = snap_five(next as f32);
                        *pct = snapped;
                        new_pct = Some(snapped);
                    }

                    let slider_w = (ui.available_width() - 38.0).max(80.0);
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(slider_w, 22.0),
                        egui::Sense::click_and_drag(),
                    );

                    if (resp.clicked() || resp.dragged())
                        && let Some(pos) = resp.interact_pointer_pos()
                    {
                        let ratio = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                        let snapped = snap_five(ratio * 100.0);
                        if snapped != *pct {
                            *pct = snapped;
                            new_pct = Some(snapped);
                        }
                    }

                    if resp.hovered() {
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

                    let track = egui::Rect::from_center_size(
                        rect.center(),
                        egui::vec2(rect.width(), 8.0),
                    );
                    let track_bg = mix_c32(bg_rgb, (0, 0, 0), 0.25, 255);
                    let p = ui.painter();
                    p.rect_filled(track, egui::CornerRadius::same(4), track_bg);

                    let frac = (*pct as f32 / 100.0).clamp(0.0, 1.0);
                    let fill_w = (track.width() * frac).max(if frac > 0.0 { 8.0 } else { 0.0 });
                    let fill_rect = egui::Rect::from_min_size(
                        track.min,
                        egui::vec2(fill_w, track.height()),
                    );
                    p.rect_filled(
                        fill_rect,
                        egui::CornerRadius::same(4),
                        if muted { muted_c } else { accent },
                    );

                    let knob_x = track.left() + track.width() * frac;
                    p.circle(
                        egui::pos2(knob_x, track.center().y),
                        7.0,
                        text,
                        egui::Stroke::new(2.0, if muted { muted_c } else { sec }),
                    );

                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("+5").size(11.0).color(text))
                                .fill(step_btn_fill)
                                .corner_radius(egui::CornerRadius::same(7))
                                .min_size(egui::vec2(30.0, 24.0)),
                        )
                        .clicked()
                    {
                        let next = (*pct + 5).min(100);
                        let snapped = snap_five(next as f32);
                        *pct = snapped;
                        new_pct = Some(snapped);
                    }
                });
            });

        (toggle_mute, new_pct)
    }

    fn render_volume_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 255);
        let accent = c32(&self.pal.accent, 255);
        let pal = self.pal.clone();

        egui::ScrollArea::vertical().max_height(355.0).show(ui, |ui| {
            // 1. OUTPUT (Speakers / Headphones)
            ui.label(egui::RichText::new("OUTPUT (SPEAKER)").size(10.5).strong().color(dim));
            ui.add_space(4.0);
            let default_sink_name = self.audio.default_sink_name.clone();
            if let Some(sink) = self.audio.sinks.iter_mut().find(|s| s.id == default_sink_name) {
                let sink_id = sink.id.clone();
                let sink_label = sink.label.clone();
                let mut pct = sink.percent;
                let muted = sink.muted;
                let (toggled, changed) =
                    Self::draw_slider_row(&pal, ui, &sink_label, &mut pct, muted, "󰕾");
                sink.percent = pct;
                if toggled {
                    let _ = run_cmd_quiet("pactl", &["set-sink-mute", &sink_id, "toggle"]);
                    self.trigger_refresh(false);
                }
                if let Some(p) = changed {
                    let arg = format!("{}%", p);
                    let _ = run_cmd_quiet("pactl", &["set-sink-volume", &sink_id, &arg]);
                }
            } else if self.audio.sinks.is_empty() {
                ui.label(egui::RichText::new("No active audio output found.").color(dim));
            }

            if self.audio.sinks.len() > 1 {
                ui.add_space(4.0);
                let sinks_copy = self.audio.sinks.clone();
                for s in sinks_copy {
                    let fill = if s.is_default {
                        c32(&self.pal.accent, 42)
                    } else {
                        mix_c32(bg_rgb, (255, 255, 255), 0.05, 210)
                    };
                    let prefix = if s.is_default { "●  " } else { "○  " };
                    let btn = egui::Button::new(
                        egui::RichText::new(format!("{}{}", prefix, s.label))
                            .size(12.0)
                            .color(if s.is_default { accent } else { text }),
                    )
                    .fill(fill)
                    .stroke(egui::Stroke::new(
                        1.0,
                        if s.is_default { c32(&self.pal.accent, 110) } else { egui::Color32::TRANSPARENT },
                    ))
                    .corner_radius(egui::CornerRadius::same(9))
                    .min_size(egui::vec2(ui.available_width(), 26.0));

                    if ui.add(btn).clicked() && !s.is_default {
                        let _ = run_cmd_quiet("pactl", &["set-default-sink", &s.id]);
                        self.trigger_refresh(false);
                    }
                }
            }

            // 2. INPUT (Microphone)
            ui.add_space(10.0);
            ui.label(egui::RichText::new("INPUT (MICROPHONE)").size(10.5).strong().color(dim));
            ui.add_space(4.0);
            if let Some(src) = self.audio.sources.iter_mut().find(|s| s.is_default) {
                let src_id = src.id.clone();
                let src_label = src.label.clone();
                let mut pct = src.percent;
                let muted = src.muted;
                let (toggled, changed) =
                    Self::draw_slider_row(&pal, ui, &src_label, &mut pct, muted, "󰍬");
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
                ui.label(egui::RichText::new("No active microphone found.").color(dim));
            }

            // 3. APPLICATIONS
            if !self.audio.streams.is_empty() {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("APPLICATIONS").size(10.5).strong().color(dim));
                ui.add_space(4.0);
                for idx in 0..self.audio.streams.len() {
                    let sid = self.audio.streams[idx].id.clone();
                    let label = self.audio.streams[idx].label.clone();
                    let mut pct = self.audio.streams[idx].percent;
                    let muted = self.audio.streams[idx].muted;
                    let (toggled, changed) =
                        Self::draw_slider_row(&pal, ui, &label, &mut pct, muted, "󰎈");
                    self.audio.streams[idx].percent = pct;
                    if toggled {
                        let _ = run_cmd_quiet("pactl", &["set-sink-input-mute", &sid, "toggle"]);
                        self.trigger_refresh(false);
                    }
                    if let Some(p) = changed {
                        let arg = format!("{}%", p);
                        let _ = run_cmd_quiet("pactl", &["set-sink-input-volume", &sid, &arg]);
                    }
                    ui.add_space(5.0);
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
                "Disconnected.".to_string(),
                false,
            ));
        });
    }

    fn render_wifi_ui(&mut self, ui: &mut egui::Ui) {
        let bg_rgb = parse_hex_rgb(&self.pal.bg);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 255);
        let accent = c32(&self.pal.accent, 255);
        let surface = mix_c32(bg_rgb, (255, 255, 255), 0.06, 235);

        // Quick Radio Bar + Rescan
        ui.horizontal(|ui| {
            let toggle_txt = if self.wifi.enabled {
                "Wi-Fi: ON"
            } else {
                "Wi-Fi: OFF"
            };
            let toggle_btn = egui::Button::new(
                egui::RichText::new(toggle_txt)
                    .size(11.5)
                    .strong()
                    .color(if self.wifi.enabled {
                        egui::Color32::WHITE
                    } else {
                        dim
                    }),
            )
            .fill(if self.wifi.enabled { accent } else { surface })
            .corner_radius(egui::CornerRadius::same(9));

            if ui.add(toggle_btn).clicked() {
                let target = if self.wifi.enabled { "off" } else { "on" };
                let _ = run_cmd_quiet("nmcli", &["radio", "wifi", target]);
                self.wifi_loading = true;
                self.trigger_refresh(false);
            }

            if self.wifi.enabled {
                let rescan_btn = egui::Button::new(
                    egui::RichText::new(if self.wifi_loading {
                        "Scanning..."
                    } else {
                        "Rescan"
                    })
                    .size(11.5)
                    .color(text),
                )
                .fill(surface)
                .stroke(egui::Stroke::new(1.0, c32(&self.pal.accent, 45)))
                .corner_radius(egui::CornerRadius::same(9));
                if ui.add_enabled(!self.wifi_loading, rescan_btn).clicked() {
                    self.wifi_loading = true;
                    self.wifi_status = Some(("Scanning nearby networks...".to_string(), false));
                    self.trigger_refresh(true);
                }
            }
        });

        if let Some((ref msg, is_err)) = self.wifi_status {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(msg)
                    .size(11.5)
                    .color(if is_err {
                        egui::Color32::from_rgb(243, 139, 168)
                    } else {
                        c32(&self.pal.accent_secondary, 255)
                    }),
            );
        }

        if self.wifi.enabled
            && let (Some(ssid), Some(dev)) = (
                self.wifi.connected_ssid.clone(),
                self.wifi.wifi_device.clone(),
            )
        {
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(c32(&self.pal.accent, 34))
                .stroke(egui::Stroke::new(1.0, c32(&self.pal.accent, 110)))
                .corner_radius(egui::CornerRadius::same(11))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Connected: {}", ssid))
                                    .size(12.5)
                                    .strong()
                                    .color(text),
                            );
                            if !self.wifi.ip_addr.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!("IP {}", self.wifi.ip_addr))
                                        .size(11.0)
                                        .color(dim),
                                );
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let disc_btn = egui::Button::new(
                                egui::RichText::new("Disconnect").size(11.0).color(text),
                            )
                            .fill(surface)
                            .corner_radius(egui::CornerRadius::same(8));
                            if ui.add(disc_btn).clicked() {
                                self.disconnect_wifi(dev);
                            }
                        });
                    });
                });
        }

        ui.add_space(8.0);
        if !self.wifi.enabled {
            ui.label(
                egui::RichText::new("Enable Wi-Fi above to view nearby networks.")
                    .size(12.0)
                    .color(dim),
            );
        } else {
            ui.label(
                egui::RichText::new("AVAILABLE NETWORKS")
                    .size(10.5)
                    .strong()
                    .color(dim),
            );
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .max_height(245.0)
                .show(ui, |ui| {
                    let nets = self.wifi.networks.clone();
                    if nets.is_empty() {
                        ui.label(
                            egui::RichText::new("No networks listed yet — tap Rescan.")
                                .size(12.0)
                                .color(dim),
                        );
                    }
                    for net in nets {
                        let is_sec = !net.security.is_empty() && net.security != "--";
                        let row_fill = if net.in_use {
                            c32(&self.pal.accent, 40)
                        } else {
                            surface
                        };
                        egui::Frame::new()
                            .fill(row_fill)
                            .stroke(egui::Stroke::new(
                                1.0,
                                if net.in_use {
                                    c32(&self.pal.accent, 110)
                                } else {
                                    c32(&self.pal.accent, 24)
                                },
                            ))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .show(ui, |ui| {
                                let row_resp = ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&net.ssid)
                                                .size(12.5)
                                                .color(text),
                                        )
                                        .truncate(),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{}%", net.signal))
                                                    .size(11.0)
                                                    .color(dim),
                                            );
                                            if is_sec {
                                                ui.label(
                                                    egui::RichText::new("SEC")
                                                        .size(9.5)
                                                        .color(dim),
                                                );
                                            }
                                            if net.in_use {
                                                ui.label(
                                                    egui::RichText::new("Active")
                                                        .size(10.5)
                                                        .strong()
                                                        .color(accent),
                                                );
                                            } else if net.saved {
                                                ui.label(
                                                    egui::RichText::new("Saved")
                                                        .size(10.5)
                                                        .color(dim),
                                                );
                                            }
                                        },
                                    );
                                });

                                let click_resp = ui.interact(
                                    row_resp.response.rect,
                                    ui.scope_id().with(&net.ssid),
                                    egui::Sense::click(),
                                );
                                if click_resp.clicked() && !net.in_use {
                                    if net.saved || !is_sec {
                                        self.pending_pw_ssid = None;
                                        self.connect_wifi(net.ssid.clone(), None);
                                    } else if self.pending_pw_ssid.as_deref() == Some(&net.ssid) {
                                        self.pending_pw_ssid = None;
                                    } else {
                                        self.pending_pw_ssid = Some(net.ssid.clone());
                                        self.password_buf.clear();
                                    }
                                }

                                if self.pending_pw_ssid.as_deref() == Some(&net.ssid) {
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        let pw_edit = ui.add(
                                            egui::TextEdit::singleline(&mut self.password_buf)
                                                .password(!self.show_password)
                                                .hint_text("Wi-Fi password...")
                                                .desired_width(ui.available_width() - 122.0),
                                        );
                                        if pw_edit.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                            && !self.password_buf.is_empty()
                                        {
                                            let pw = std::mem::take(&mut self.password_buf);
                                            self.pending_pw_ssid = None;
                                            self.connect_wifi(net.ssid.clone(), Some(pw));
                                        }
                                        if ui
                                            .button(if self.show_password { "Hide" } else { "Show" })
                                            .clicked()
                                        {
                                            self.show_password = !self.show_password;
                                        }
                                        let go_btn = egui::Button::new(
                                            egui::RichText::new("Connect")
                                                .size(11.5)
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        )
                                        .fill(accent)
                                        .corner_radius(egui::CornerRadius::same(7));
                                        if ui.add(go_btn).clicked()
                                            && !self.password_buf.is_empty()
                                        {
                                            let pw = std::mem::take(&mut self.password_buf);
                                            self.pending_pw_ssid = None;
                                            self.connect_wifi(net.ssid.clone(), Some(pw));
                                        }
                                    });
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
        }

        ui.add_space(8.0);
        let adv_btn = egui::Button::new(
            egui::RichText::new("Advanced Network Settings")
                .size(11.5)
                .color(dim),
        )
        .fill(surface)
        .stroke(egui::Stroke::new(1.0, c32(&self.pal.accent, 36)))
        .corner_radius(egui::CornerRadius::same(9))
        .min_size(egui::vec2(ui.available_width(), 28.0));
        if ui.add(adv_btn).clicked() {
            let _ = Command::new("nm-connection-editor").spawn();
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
            *slot = Some(ctx.clone());
        }

        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                PopupEvent::AudioLoaded(st) => self.audio = st,
                PopupEvent::WifiLoaded(st) => {
                    self.wifi = st;
                    self.wifi_loading = false;
                }
                PopupEvent::WifiStatus(st, msg, is_err) => {
                    self.wifi = st;
                    self.wifi_loading = false;
                    self.wifi_status = Some((msg, is_err));
                }
                PopupEvent::BtLoaded(st) => {
                    self.bt = st;
                    self.bt_loading = false;
                }
                PopupEvent::BtStatus(st, msg, is_err) => {
                    self.bt = st;
                    self.bt_loading = false;
                    self.bt_status = Some((msg, is_err));
                }
                PopupEvent::Trigger(requested) => {
                    if requested == self.mode {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                    self.mode = requested;
                    self.pal = get_current_palette();
                    self.pending_pw_ssid = None;
                    self.wifi_status = None;
                    self.bt_status = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                        self.mode.size().into(),
                    ));
                    self.trigger_refresh(false);
                }
            }
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

        let bg = c32(&self.pal.bg, 244);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 255);
        let border = c32(&self.pal.accent, 95);

        let title = match self.mode {
            PopupMode::Volume => "Audio Mixer",
            PopupMode::Wifi => "Wi-Fi & Network",
            PopupMode::Bluetooth => "Bluetooth",
            PopupMode::Power => "Power & Session",
        };
        let subtitle = match self.mode {
            PopupMode::Volume => "PipeWire • 5% precision steps".to_string(),
            PopupMode::Wifi => {
                if !self.wifi.ip_addr.is_empty() {
                    format!("Connected • {}", self.wifi.ip_addr)
                } else if let Some(ref eth) = self.wifi.eth_line {
                    eth.clone()
                } else if self.wifi.enabled {
                    "Wi-Fi Enabled".to_string()
                } else {
                    "Wi-Fi Disabled".to_string()
                }
            }
            PopupMode::Bluetooth => {
                if self.bt.powered {
                    let count = self.bt.devices.iter().filter(|d| d.connected).count();
                    if count > 0 {
                        format!("Enabled • {} connected", count)
                    } else {
                        "Bluetooth Enabled".to_string()
                    }
                } else {
                    "Bluetooth Disabled".to_string()
                }
            }
            PopupMode::Power => "Hyprland • Graceful Shutdown".to_string(),
        };

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.2, border))
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).size(15.5).strong().color(text));
                    ui.label(egui::RichText::new(subtitle).size(11.0).color(dim));
                });

                ui.add_space(10.0);

                match self.mode {
                    PopupMode::Volume => self.render_volume_ui(ui),
                    PopupMode::Wifi => self.render_wifi_ui(ui),
                    PopupMode::Bluetooth => self.render_bt_ui(ui),
                    PopupMode::Power => self.render_power_ui(ui, &ctx),
                }
            });
    }
}

impl PopupApp {
    fn draw_bento_tile(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        width: f32,
        height: f32,
        icon: &str,
        label: &str,
        desc: &str,
        accent_col: egui::Color32,
        fill_col: egui::Color32,
        cmd: &[&str],
    ) {
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 255);
        let border_col = egui::Color32::from_rgba_unmultiplied(
            accent_col.r(),
            accent_col.g(),
            accent_col.b(),
            85,
        );

        let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
        let bg = if resp.hovered() {
            mix_c32(
                (fill_col.r(), fill_col.g(), fill_col.b()),
                (accent_col.r(), accent_col.g(), accent_col.b()),
                0.22,
                250,
            )
        } else {
            fill_col
        };

        ui.painter().rect(
            rect,
            egui::CornerRadius::same(14),
            bg,
            egui::Stroke::new(if resp.hovered() { 1.4 } else { 1.0 }, border_col),
            egui::StrokeKind::Inside,
        );

        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(10.0)));
        child_ui.vertical_centered(|ui| {
            ui.add_space((height - 64.0).max(2.0) * 0.5);
            ui.label(egui::RichText::new(icon).size(24.0).color(accent_col));
            ui.add_space(4.0);
            ui.label(egui::RichText::new(label).size(13.5).strong().color(text));
            ui.label(egui::RichText::new(desc).size(10.5).color(dim));
        });

        if resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            let _ = Command::new(cmd[0]).args(&cmd[1..]).spawn();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn render_power_ui(&self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let base_rgb = parse_hex_rgb(&self.pal.bg);
        let accent_rgb = parse_hex_rgb(&self.pal.accent);
        let accent = c32(&self.pal.accent, 255);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 255);

        let danger_col = egui::Color32::from_rgb(243, 139, 168);
        let reboot_col = egui::Color32::from_rgb(250, 179, 135);
        let danger_bg = mix_c32(base_rgb, (243, 139, 168), 0.14, 242);
        let reboot_bg = mix_c32(base_rgb, (250, 179, 135), 0.12, 242);
        let logout_bg = mix_c32(base_rgb, accent_rgb, 0.14, 242);

        let avail_w = ui.available_width();
        let gap = 10.0;
        let tile_w = (avail_w - gap) * 0.5;

        // Top row: 2 Bento square tiles (Shut Down + Reboot)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            self.draw_bento_tile(
                ui,
                ctx,
                tile_w,
                104.0,
                "⏻",
                "Shut Down",
                "Power off PC",
                danger_col,
                danger_bg,
                &["hyprshutdown", "-t", "Shutting down...", "-p", "systemctl poweroff"],
            );
            self.draw_bento_tile(
                ui,
                ctx,
                tile_w,
                104.0,
                "󰜉",
                "Reboot",
                "Restart PC",
                reboot_col,
                reboot_bg,
                &["hyprshutdown", "-t", "Rebooting...", "-p", "systemctl reboot"],
            );
        });

        ui.add_space(gap);

        // Bottom row: full-width Bento tile (Log Out)
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(avail_w, 58.0), egui::Sense::click());
        let bg = if resp.hovered() {
            mix_c32(base_rgb, accent_rgb, 0.28, 250)
        } else {
            logout_bg
        };
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(14),
            bg,
            egui::Stroke::new(
                if resp.hovered() { 1.4 } else { 1.0 },
                c32(&self.pal.accent, 90),
            ),
            egui::StrokeKind::Inside,
        );

        let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(16.0, 10.0))));
        row_ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new("󰍃").size(22.0).color(accent));
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Log Out").size(14.0).strong().color(text));
                ui.label(egui::RichText::new("Exit Hyprland session").size(10.5).color(dim));
            });
        });

        if resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            let _ = Command::new("hyprshutdown")
                .args(["-t", "Logging out..."])
                .spawn();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn render_bt_ui(&mut self, ui: &mut egui::Ui) {
        let base_rgb = parse_hex_rgb(&self.pal.bg);
        let accent = c32(&self.pal.accent, 255);
        let text = c32(&self.pal.text_color, 255);
        let dim = c32(&self.pal.text_dim, 255);
        let card_bg = mix_c32(base_rgb, (255, 255, 255), 0.05, 235);
        let active_bg = mix_c32(base_rgb, parse_hex_rgb(&self.pal.accent), 0.16, 240);

        ui.horizontal(|ui| {
            let toggle_txt = if self.bt.powered { "󰂯  Bluetooth ON" } else { "󰂲  Bluetooth OFF" };
            let toggle_btn = egui::Button::new(egui::RichText::new(toggle_txt).size(12.5).strong().color(text))
                .fill(if self.bt.powered { active_bg } else { card_bg })
                .corner_radius(egui::CornerRadius::same(10))
                .min_size(egui::vec2(140.0, 32.0));
            if ui.add(toggle_btn).clicked() && !self.bt_loading {
                self.bt_loading = true;
                let next = if self.bt.powered { "off" } else { "on" };
                let tx = self.tx.clone();
                thread::spawn(move || {
                    let _ = Command::new("bluetoothctl").args(["power", next]).status();
                    let _ = tx.send(PopupEvent::BtLoaded(query_bt_state()));
                });
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let scan_txt = if self.bt_loading { "Scanning..." } else { "󰑐  Scan" };
                let scan_btn = egui::Button::new(egui::RichText::new(scan_txt).size(12.0).color(accent))
                    .fill(card_bg)
                    .corner_radius(egui::CornerRadius::same(10))
                    .min_size(egui::vec2(86.0, 32.0));
                if ui.add(scan_btn).clicked() && !self.bt_loading {
                    self.bt_loading = true;
                    self.trigger_refresh(true);
                }
            });
        });

        if let Some((ref msg, is_err)) = self.bt_status {
            ui.add_space(6.0);
            let col = if is_err {
                egui::Color32::from_rgb(243, 139, 168)
            } else {
                egui::Color32::from_rgb(166, 227, 161)
            };
            ui.label(egui::RichText::new(msg).size(11.5).color(col));
        }

        ui.add_space(10.0);

        if !self.bt.powered {
            ui.label(egui::RichText::new("Turn on Bluetooth to see devices.").size(12.5).color(dim));
            return;
        }

        if self.bt.devices.is_empty() {
            ui.label(egui::RichText::new("No devices found. Click Scan.").size(12.5).color(dim));
            return;
        }

        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for dev in self.bt.devices.clone() {
                egui::Frame::new()
                    .fill(if dev.connected { active_bg } else { card_bg })
                    .stroke(egui::Stroke::new(1.0, c32(&self.pal.accent, if dev.connected { 110 } else { 40 })))
                    .corner_radius(egui::CornerRadius::same(11))
                    .inner_margin(egui::Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(if dev.connected { "󰂱" } else { "󰂯" })
                                    .size(16.0)
                                    .color(if dev.connected { accent } else { dim }),
                            );
                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                ui.add(egui::Label::new(egui::RichText::new(&dev.name).size(13.0).strong().color(text)).truncate());
                                let sub = match (dev.connected, dev.paired, dev.battery) {
                                    (true, _, Some(b)) => format!("Connected • {}%", b),
                                    (true, _, None) => "Connected".to_string(),
                                    (false, true, _) => "Paired".to_string(),
                                    (false, false, _) => dev.mac.clone(),
                                };
                                ui.label(egui::RichText::new(sub).size(10.5).color(dim));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let act_label = if dev.connected {
                                    "Disconnect"
                                } else if dev.paired {
                                    "Connect"
                                } else {
                                    "Pair"
                                };
                                let btn = egui::Button::new(egui::RichText::new(act_label).size(11.5).color(text))
                                    .fill(c32(&self.pal.accent, if dev.connected { 35 } else { 60 }))
                                    .corner_radius(egui::CornerRadius::same(8));
                                if ui.add(btn).clicked() && !self.bt_loading {
                                    self.bt_loading = true;
                                    let mac = dev.mac.clone();
                                    let name = dev.name.clone();
                                    let is_conn = dev.connected;
                                    let is_paired = dev.paired;
                                    let tx = self.tx.clone();
                                    thread::spawn(move || {
                                        let ok = if is_conn {
                                            Command::new("bluetoothctl").args(["disconnect", &mac]).status().is_ok_and(|s| s.success())
                                        } else {
                                            if !is_paired {
                                                let _ = Command::new("bluetoothctl").args(["pair", &mac]).status();
                                                let _ = Command::new("bluetoothctl").args(["trust", &mac]).status();
                                            }
                                            Command::new("bluetoothctl").args(["connect", &mac]).status().is_ok_and(|s| s.success())
                                        };
                                        let msg = if ok {
                                            if is_conn { format!("Disconnected {}", name) } else { format!("Connected to {}", name) }
                                        } else {
                                            format!("Failed action for {}", name)
                                        };
                                        let _ = tx.send(PopupEvent::BtStatus(query_bt_state(), msg, !ok));
                                    });
                                }
                            });
                        });
                    });
            }
        });
    }
}

pub fn handle_popup(args: &[String]) {
    let mode = PopupMode::from_arg(args.first().map(|s| s.as_str()));
    let sock_path = match mode {
        PopupMode::Power => "/tmp/wt-rust-power.sock",
        _ => POPUP_SOCK,
    };
    let app_id = match mode {
        PopupMode::Power => "wt-popup-power",
        _ => "wt-popup",
    };
    let Some((listener, sock_guard)) = bind_single_instance_socket(sock_path, |mut stream| {
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
}
