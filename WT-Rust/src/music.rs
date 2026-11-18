use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct LyricLine {
    time: f64,
    text: String,
}

struct PlaybackClock {
    track_id: String,
    elapsed: f64,
    last_tick: Instant,
    last_mpris_pos: f64,
    mpris_works: bool,
}

impl PlaybackClock {
    fn new() -> Self {
        Self {
            track_id: String::new(),
            elapsed: 0.0,
            last_tick: Instant::now(),
            last_mpris_pos: -1.0,
            mpris_works: false,
        }
    }

    fn update(&mut self, track_id: &str, status: &str, mpris_pos: f64, reset_offset: bool) -> f64 {
        let now = Instant::now();
        if track_id != self.track_id {
            self.track_id = track_id.to_string();
            self.elapsed = if mpris_pos > 0.5 { mpris_pos } else { 0.0 };
            self.last_tick = now;
            self.last_mpris_pos = mpris_pos;
            self.mpris_works = mpris_pos > 0.5;
            if reset_offset {
                let _ = fs::write(crate::common::runtime_path("waybar_lyrics_offset"), "0");
            }
            return self.elapsed;
        }

        let pos_changed = (mpris_pos - self.last_mpris_pos).abs() > 0.01;
        if mpris_pos > 0.5 && (mpris_pos - self.last_mpris_pos).abs() > 0.15 {
            self.mpris_works = true;
        }
        self.last_mpris_pos = mpris_pos;

        if self.mpris_works && mpris_pos >= 0.0 {
            if status == "Playing" {
                self.elapsed += now.duration_since(self.last_tick).as_secs_f64();
                // Only resync when MPRIS actually ticks or user seeks (> 1.2s jump), never pull back on stale integer seconds
                if pos_changed && (mpris_pos - self.elapsed).abs() > 1.2 {
                    self.elapsed = mpris_pos;
                } else if pos_changed && (mpris_pos - self.elapsed).abs() > 0.35 {
                    self.elapsed = self.elapsed * 0.5 + mpris_pos * 0.5;
                }
            } else {
                self.elapsed = mpris_pos;
            }
            self.last_tick = now;
        } else if status == "Playing" {
            self.elapsed += now.duration_since(self.last_tick).as_secs_f64();
            self.last_tick = now;
        } else {
            self.last_tick = now;
        }

        self.elapsed
    }
}

fn run_playerctl(args: &[&str]) -> String {
    Command::new("playerctl")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn pick_player() -> Option<String> {
    let raw = run_playerctl(&["-l"]);
    if raw.is_empty() {
        return None;
    }
    let players: Vec<String> = raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if players.is_empty() {
        return None;
    }

    let bitchord: Vec<&String> = players
        .iter()
        .filter(|p| p.to_lowercase().contains("bitchord"))
        .collect();

    for p in &bitchord {
        if run_playerctl(&["-p", p, "status"]) == "Playing" {
            return Some((*p).clone());
        }
    }
    for p in &players {
        if run_playerctl(&["-p", p, "status"]) == "Playing" {
            return Some(p.clone());
        }
    }
    if let Some(p) = bitchord.first() {
        return Some((*p).clone());
    }
    players.into_iter().next()
}

fn cache_dir() -> PathBuf {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let dir = home.join(".cache/waybar-lyrics");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn track_hash(artist: &str, title: &str) -> String {
    // Simple FNV-1a 64-bit hex hash for cache filename
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in format!("{}::{}", artist.to_lowercase(), title.to_lowercase()).bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn clean_query(text: &str) -> String {
    let mut s = text.to_string();
    for suffix in [" - YouTube Music", " - YouTube", " | YouTube", " - Topic"] {
        if let Some(idx) = s.to_lowercase().rfind(&suffix.to_lowercase())
            && idx + suffix.len() == s.len()
        {
            s.truncate(idx);
        }
    }
    // Strip trailing (...) or [...] tags like (Official Video), [Explicit]
    loop {
        let trimmed = s.trim_end();
        let last = trimmed.chars().last();
        let (open, close) = match last {
            Some(')') => ('(', ')'),
            Some(']') => ('[', ']'),
            _ => break,
        };
        if let Some(pos) = trimmed.rfind(open) {
            let inner = trimmed[pos + 1..trimmed.len() - 1].to_lowercase();
            if inner.contains("official")
                || inner.contains("lyric")
                || inner.contains("audio")
                || inner.contains("video")
                || inner.contains("explicit")
                || inner.contains("remaster")
                || inner.contains("feat")
                || inner.contains("ft.")
            {
                let _ = close;
                s = trimmed[..pos].trim_end().to_string();
                continue;
            }
        }
        break;
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut timestamps = Vec::new();
        while rest.starts_with('[') {
            if let Some(end) = rest.find(']') {
                let tag = &rest[1..end];
                if let Some((m_str, s_str)) = tag.split_once(':')
                    && let (Ok(m), Ok(s)) = (m_str.parse::<f64>(), s_str.parse::<f64>())
                {
                    timestamps.push(m * 60.0 + s);
                }
                rest = rest[end + 1..].trim_start();
            } else {
                break;
            }
        }
        let text = rest.trim();
        if !text.is_empty() && !timestamps.is_empty() {
            for t in timestamps {
                lines.push(LyricLine {
                    time: t,
                    text: text.to_string(),
                });
            }
        }
    }
    lines.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    lines
}

fn fetch_lrclib(artist: &str, title: &str) -> Vec<LyricLine> {
    let c_artist = clean_query(artist);
    let c_title = clean_query(title);
    if c_title.is_empty() {
        return Vec::new();
    }

    let mut url = format!(
        "https://lrclib.net/api/get?track_name={}",
        url_encode(&c_title)
    );
    if !c_artist.is_empty() {
        url.push_str(&format!("&artist_name={}", url_encode(&c_artist)));
    }

    if let Ok(out) = Command::new("curl")
        .args([
            "-sf",
            "--proto",
            "=https",
            "-m",
            "5",
            "-A",
            "WT-Dots-Waybar/2.0",
            &url,
        ])
        .output()
        && out.status.success()
        && let Ok(v) = serde_json::from_slice::<Value>(&out.stdout)
        && let Some(synced) = v.get("syncedLyrics").and_then(|x| x.as_str())
    {
        let parsed = parse_lrc(synced);
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let q = if c_artist.is_empty() {
        c_title
    } else {
        format!("{} {}", c_artist, c_title)
    };
    let search_url = format!("https://lrclib.net/api/search?q={}", url_encode(&q));
    if let Ok(out) = Command::new("curl")
        .args([
            "-sf",
            "--proto",
            "=https",
            "-m",
            "5",
            "-A",
            "WT-Dots-Waybar/2.0",
            &search_url,
        ])
        .output()
        && out.status.success()
        && let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&out.stdout)
    {
        for item in arr {
            if let Some(synced) = item.get("syncedLyrics").and_then(|x| x.as_str()) {
                let parsed = parse_lrc(synced);
                if !parsed.is_empty() {
                    return parsed;
                }
            }
        }
    }

    Vec::new()
}

fn pango_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
}

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    (
        lerp_u8(a.0, b.0, t),
        lerp_u8(a.1, b.1, t),
        lerp_u8(a.2, b.2, t),
    )
}

/// Smooth character-level sweeping glow wave across the current lyric line.
fn render_glow_line(line: &str, pos_s: f64, t_start: f64, t_end: f64) -> String {
    let text = truncate_chars(line, 48);
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    // Singers finish most LRC lines in ~88% of the interval before the next line's timestamp
    let sing_dur = ((t_end - t_start) * 0.88).clamp(0.7, 6.0);
    let progress = ((pos_s - t_start) / sing_dur).clamp(0.0, 1.0) as f32;
    let cursor = progress * (chars.len() as f32);

    // Palette: dimmed (#6c7086) -> sung (#cdd6f4) -> glowing peak (#ffffff / #89dceb)
    let col_dim = (0x6c_u8, 0x70_u8, 0x86_u8);
    let col_sung = (
        0xcdd6f4_u32 >> 16 & 0xff,
        0xcdd6f4_u32 >> 8 & 0xff,
        0xcdd6f4_u32 & 0xff,
    );
    let col_sung = (col_sung.0 as u8, col_sung.1 as u8, col_sung.2 as u8);
    let col_glow = (0x89_u8, 0xdc_u8, 0xeb_u8); // bright cyan-white glow

    let mut out = String::with_capacity(chars.len() * 24);
    let mut cur_hex: Option<String> = None;
    let mut cur_buf = String::new();

    for (i, &ch) in chars.iter().enumerate() {
        let d = (i as f32) - cursor;
        let rgb = if d < -2.5 {
            col_sung
        } else if d > 2.5 {
            col_dim
        } else if d <= 0.0 {
            // Trailing edge of glow wave -> fade from glow to sung
            let t = ((d + 2.5) / 2.5).clamp(0.0, 1.0);
            lerp_rgb(col_sung, col_glow, t * t)
        } else {
            // Leading edge of glow wave -> fade from glow to dimmed
            let t = (1.0 - d / 2.5).clamp(0.0, 1.0);
            lerp_rgb(col_dim, col_glow, t * t)
        };

        let hex = format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2);
        if cur_hex.as_deref() == Some(&hex) {
            cur_buf.push(ch);
        } else {
            if let Some(prev) = cur_hex.take() {
                out.push_str(&format!(
                    "<span foreground=\"{}\">{}</span>",
                    prev,
                    pango_escape(&cur_buf)
                ));
                cur_buf.clear();
            }
            cur_hex = Some(hex);
            cur_buf.push(ch);
        }
    }

    if let Some(prev) = cur_hex {
        out.push_str(&format!(
            "<span foreground=\"{}\">{}</span>",
            prev,
            pango_escape(&cur_buf)
        ));
    }

    out
}

fn read_offset() -> f64 {
    fs::read_to_string(crate::common::runtime_path("waybar_lyrics_offset"))
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

#[derive(Clone, Default)]
struct PlayerState {
    player: Option<String>,
    status: String,
    title: String,
    artist: String,
    mpris_pos: f64,
    sonata_lyric: Option<String>,
}

fn sonata_now_playing_paths() -> [PathBuf; 2] {
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("sonata/now_playing.json");
    let cache = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cache/sonata/now_playing.json");
    [runtime, cache]
}

fn parse_sonata_json(raw: &str) -> Option<PlayerState> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let raw_status = v.get("status")?.as_str()?.trim().to_lowercase();
    let status = match raw_status.as_str() {
        "playing" => "Playing",
        "paused" => "Paused",
        _ => return None,
    };
    let title = clean_query(v.get("title").and_then(|x| x.as_str()).unwrap_or("").trim());
    let artist = v
        .get("artist")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if title.is_empty() && artist.is_empty() {
        return None;
    }
    let mpris_pos = v
        .get("position_secs")
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let sonata_lyric = v
        .get("lyric")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Some(PlayerState {
        player: Some("sonata".to_string()),
        status: status.to_string(),
        title,
        artist,
        mpris_pos,
        sonata_lyric,
    })
}

fn is_sonata_running() -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(comm) = fs::read_to_string(entry.path().join("comm"))
            && comm.trim() == "sonata"
        {
            return true;
        }
    }
    false
}

fn read_sonata_state() -> Option<PlayerState> {
    if !is_sonata_running() {
        return None;
    }
    for path in sonata_now_playing_paths() {
        if let Ok(raw) = fs::read_to_string(path)
            && let Some(st) = parse_sonata_json(&raw)
        {
            return Some(st);
        }
    }
    None
}

fn parse_and_select_state() -> PlayerState {
    let sonata = read_sonata_state();
    if let Some(ref st) = sonata
        && st.status == "Playing"
    {
        return st.clone();
    }

    let Some(p) = pick_player() else {
        return sonata.unwrap_or_default();
    };
    let status = run_playerctl(&["-p", &p, "status"]);
    if status != "Playing" && status != "Paused" {
        return sonata.unwrap_or_default();
    }
    if status != "Playing"
        && let Some(st) = sonata
    {
        return st;
    }
    let meta = run_playerctl(&["-p", &p, "metadata", "--format", "{{title}}\t{{artist}}"]);
    let mut parts = meta.splitn(2, '\t');
    let raw_title = parts.next().unwrap_or("").trim();
    let raw_artist = parts.next().unwrap_or("").trim();

    let (artist, title) = if raw_artist.is_empty() && raw_title.contains(" - ") {
        if let Some((a, t)) = raw_title.split_once(" - ") {
            (a.trim().to_string(), clean_query(t))
        } else {
            (String::new(), clean_query(raw_title))
        }
    } else {
        (raw_artist.to_string(), clean_query(raw_title))
    };

    let mpris_pos = run_playerctl(&["-p", &p, "position"])
        .parse::<f64>()
        .unwrap_or(0.0);

    PlayerState {
        player: Some(p),
        status,
        title,
        artist,
        mpris_pos,
        sonata_lyric: None,
    }
}

fn run_waybar_loop() {
    let mem: Arc<Mutex<HashMap<String, Vec<LyricLine>>>> = Arc::new(Mutex::new(HashMap::new()));
    let fetching: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let shared_state = Arc::new(Mutex::new(parse_and_select_state()));
    let mut clock = PlaybackClock::new();

    // Event-driven MPRIS listener via `playerctl -a -F` (0ms reaction, zero polling)
    let state_listener = Arc::clone(&shared_state);
    let running = Arc::new(AtomicBool::new(true));
    let running_listener = Arc::clone(&running);
    let active_child: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
    let child_slot = Arc::clone(&active_child);
    thread::spawn(move || {
        while running_listener.load(Ordering::Relaxed) {
            if let Ok(mut child) = Command::new("playerctl")
                .args([
                    "-a",
                    "-F",
                    "metadata",
                    "--format",
                    "{{playerName}}\t{{status}}\t{{title}}\t{{artist}}",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                let stdout = child.stdout.take();
                *child_slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
                if let Some(stdout) = stdout {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        if !running_listener.load(Ordering::Relaxed) {
                            break;
                        }
                        let _ = line;
                        let next = parse_and_select_state();
                        *state_listener.lock().unwrap_or_else(|e| e.into_inner()) = next;
                    }
                }
                if let Some(mut c) = child_slot.lock().unwrap_or_else(|e| e.into_inner()).take() {
                    let _ = c.wait();
                }
            }
            if running_listener.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(2));
            }
        }
    });

    let mut last_pos_sync = Instant::now();
    let mut last_out = String::new();
    let mut sonata_line_text = String::new();
    let mut sonata_line_start = 0.0_f64;

    let stdout = io::stdout();
    let mut out_lock = io::BufWriter::new(stdout.lock());

    loop {
        let mut st = shared_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        // Poll Sonata's now_playing.json (no MPRIS) and resync MPRIS position every 500ms while playing
        if let Some(sonata_st) = read_sonata_state() {
            if sonata_st.status == "Playing"
                || st.player.as_deref() == Some("sonata")
                || st.player.is_none()
            {
                st = sonata_st;
                *shared_state.lock().unwrap_or_else(|e| e.into_inner()) = st.clone();
            }
        } else if st.player.as_deref() == Some("sonata") {
            st = parse_and_select_state();
            *shared_state.lock().unwrap_or_else(|e| e.into_inner()) = st.clone();
        } else if st.status == "Playing" && last_pos_sync.elapsed() >= Duration::from_millis(500) {
            last_pos_sync = Instant::now();
            if let Some(ref p) = st.player {
                let pos = run_playerctl(&["-p", p, "position"])
                    .parse::<f64>()
                    .unwrap_or(st.mpris_pos);
                st.mpris_pos = pos;
                shared_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .mpris_pos = pos;
            }
        }

        let is_playing = st.status == "Playing";
        let mut has_lyrics = false;
        let payload = if let Some(ref p) = st.player {
            let status = st.status;
            let title = st.title;
            let artist = st.artist;
            let mpris_pos = st.mpris_pos;
            if (status == "Playing" || status == "Paused")
                && (!title.is_empty() || !artist.is_empty())
            {
                let track_id = format!("{}::{}::{}", p, artist, title);
                let base_pos = clock.update(&track_id, &status, mpris_pos, true);
                let pos_s = (base_pos + read_offset()).max(0.0);

                let key = track_hash(&artist, &title);
                let lines = {
                    let mut m = mem.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(cached) = m.get(&key) {
                        cached.clone()
                    } else {
                        let cf = cache_dir().join(format!("{}.lrc.json", key));
                        if let Ok(raw) = fs::read_to_string(&cf) {
                            if let Ok(arr) = serde_json::from_str::<Vec<(f64, String)>>(&raw) {
                                let parsed: Vec<LyricLine> = arr
                                    .into_iter()
                                    .map(|(time, text)| LyricLine { time, text })
                                    .collect();
                                m.insert(key.clone(), parsed.clone());
                                parsed
                            } else {
                                Vec::new()
                            }
                        } else {
                            let mut f = fetching.lock().unwrap_or_else(|e| e.into_inner());
                            if !f.contains(&key) {
                                f.insert(key.clone());
                                let mem_c = Arc::clone(&mem);
                                let fetch_c = Arc::clone(&fetching);
                                let key_c = key.clone();
                                let a_c = artist.clone();
                                let t_c = title.clone();
                                thread::spawn(move || {
                                    let parsed = fetch_lrclib(&a_c, &t_c);
                                    if !parsed.is_empty() {
                                        let pairs: Vec<(f64, &str)> = parsed
                                            .iter()
                                            .map(|l| (l.time, l.text.as_str()))
                                            .collect();
                                        if let Ok(j) = serde_json::to_string(&pairs) {
                                            let _ = fs::write(
                                                cache_dir().join(format!("{}.lrc.json", key_c)),
                                                j,
                                            );
                                        }
                                    }
                                    {
                                        let mut m = mem_c.lock().unwrap_or_else(|e| e.into_inner());
                                        if m.len() >= 64 {
                                            m.clear();
                                        }
                                        m.insert(key_c.clone(), parsed);
                                    }
                                    fetch_c
                                        .lock()
                                        .unwrap_or_else(|e| e.into_inner())
                                        .remove(&key_c);
                                });
                            }
                            Vec::new()
                        }
                    }
                };

                has_lyrics = !lines.is_empty() || st.sonata_lyric.is_some();
                let mut idx: isize = -1;
                for (i, l) in lines.iter().enumerate() {
                    if l.time <= pos_s {
                        idx = i as isize;
                    } else {
                        break;
                    }
                }

                let track_label = if artist.is_empty() {
                    title.clone()
                } else {
                    format!("{} - {}", artist, title)
                };

                let display = if status == "Playing" && idx >= 0 {
                    let i = idx as usize;
                    let t_start = lines[i].time;
                    let t_end = if i + 1 < lines.len() {
                        lines[i + 1].time
                    } else {
                        t_start + 4.5
                    };
                    let glow = render_glow_line(&lines[i].text, pos_s, t_start, t_end);
                    format!("<span foreground=\"#cba6f7\">󰎈</span>  {}", glow)
                } else if status == "Playing"
                    && let Some(ref lyric) = st.sonata_lyric
                {
                    if *lyric != sonata_line_text {
                        sonata_line_text = lyric.clone();
                        sonata_line_start = pos_s;
                    }
                    let glow =
                        render_glow_line(lyric, pos_s, sonata_line_start, sonata_line_start + 4.0);
                    format!("<span foreground=\"#cba6f7\">󰎈</span>  {}", glow)
                } else {
                    let icon = if status == "Playing" { "󰎈" } else { "󰏤" };
                    format!(
                        "{}  {}",
                        icon,
                        pango_escape(&truncate_chars(&track_label, 40))
                    )
                };

                let pname = p.split('.').next().unwrap_or(p);
                let mut tip = vec![format!("{} · {}", pname, status), track_label];
                if !lines.is_empty() {
                    tip.push(String::new());
                    let start = if idx > 0 { (idx - 1) as usize } else { 0 };
                    let end = (start + 4).min(lines.len());
                    for (i, line) in lines.iter().enumerate().take(end).skip(start) {
                        let prefix = if i as isize == idx { "▶ " } else { "  " };
                        tip.push(format!("{}{}", prefix, line.text));
                    }
                }

                serde_json::json!({
                    "text": display,
                    "tooltip": tip.join("\n"),
                    "class": status.to_lowercase()
                })
                .to_string()
            } else {
                serde_json::json!({"text": "", "class": "empty"}).to_string()
            }
        } else {
            serde_json::json!({"text": "", "class": "empty"}).to_string()
        };

        if payload != last_out {
            if writeln!(out_lock, "{}", payload).is_err() || out_lock.flush().is_err() {
                break;
            }
            last_out = payload;
        }

        let sleep_ms = if is_playing && has_lyrics {
            60
        } else if is_playing {
            300
        } else {
            400
        };
        thread::sleep(Duration::from_millis(sleep_ms));
    }

    running.store(false, Ordering::Relaxed);
    if let Some(mut c) = active_child
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        let _ = c.kill();
        let _ = c.wait();
    }
}

pub fn handle_music(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("--waybar") => run_waybar_loop(),
        Some("--action") => {
            if let Some(act) = args.get(1)
                && let Some(p) = pick_player()
            {
                let _ = Command::new("playerctl").args(["-p", &p, act]).status();
            }
        }
        Some("--offset") => {
            if let Some(delta_str) = args.get(1)
                && let Ok(delta) = delta_str.parse::<f64>()
            {
                let cur = read_offset();
                let _ = fs::write(
                    crate::common::runtime_path("waybar_lyrics_offset"),
                    format!("{}", cur + delta),
                );
            }
        }
        _ => eprintln!("Usage: wt-rust music <--waybar|--action <cmd>|--offset <sec>>"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_sonata_json;

    #[test]
    fn test_parse_sonata_now_playing() {
        let raw = r#"{
            "status": "Playing",
            "title": "Numb (Official Video)",
            "artist": "Linkin Park",
            "position_secs": 42.5,
            "lyric": "I've become so numb"
        }"#;
        let st = parse_sonata_json(raw).expect("valid sonata json");
        assert_eq!(st.player.as_deref(), Some("sonata"));
        assert_eq!(st.status, "Playing");
        assert_eq!(st.title, "Numb");
        assert_eq!(st.artist, "Linkin Park");
        assert!((st.mpris_pos - 42.5).abs() < 0.01);
        assert_eq!(st.sonata_lyric.as_deref(), Some("I've become so numb"));
    }
}
