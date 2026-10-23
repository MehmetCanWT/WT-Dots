use serde::Serialize;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
struct StatVfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    __f_spare: [i32; 6],
}

unsafe extern "C" {
    fn statvfs(path: *const std::ffi::c_char, buf: *mut StatVfs) -> std::ffi::c_int;
}

#[derive(Serialize)]
pub struct CpuData {
    pub total: f64,
    pub cores: Vec<f64>,
    pub temp: i32,
    pub freq: f64,
}

#[derive(Serialize)]
pub struct MemData {
    pub used: f64,
    pub total: f64,
    pub pct: f64,
    pub swap_used: f64,
    pub swap_total: f64,
    pub swap_pct: f64,
}

#[derive(Serialize)]
pub struct GpuData {
    pub name: String,
    pub usage: f64,
    pub temp: f64,
    pub vram_used: f64,
    pub vram_total: f64,
    pub vram_pct: f64,
}

#[derive(Serialize)]
pub struct NetData {
    pub iface: String,
    pub rx_rate: String,
    pub tx_rate: String,
    pub rx_raw: u64,
    pub tx_raw: u64,
}

#[derive(Serialize)]
pub struct DiskData {
    pub name: String,
    pub mount: String,
    pub used: f64,
    pub total: f64,
    pub pct: f64,
}

#[derive(Serialize)]
pub struct ProcData {
    pub pid: String,
    pub cpu: String,
    pub mem: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct BatteryData {
    pub pct: i32,
    pub status: String,
}

#[derive(Serialize)]
pub struct FullStats {
    pub cpu: CpuData,
    pub mem: MemData,
    pub gpu: GpuData,
    pub net: NetData,
    pub disks: Vec<DiskData>,
    pub procs: Vec<ProcData>,
    pub battery: BatteryData,
    pub uptime: String,
    pub uptime_secs: u64,
    pub is_turkish: bool,
    pub palette: Option<serde_json::Value>,
}

fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

pub fn handle_stats() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let state_file = "/tmp/.btop_widget_state.json";
    let prev_state: serde_json::Value = fs::read_to_string(state_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    // 1. CPU
    let mut cpu_stat: HashMap<String, (f64, f64)> = HashMap::new();
    if let Ok(stat_content) = fs::read_to_string("/proc/stat") {
        for line in stat_content.lines() {
            let mut parts = line.split_whitespace();
            if let Some(key) = parts.next() {
                if key == "cpu" || (key.starts_with("cpu") && key[3..].chars().all(|c| c.is_ascii_digit())) {
                    let vals: Vec<f64> = parts.filter_map(|s| s.parse().ok()).collect();
                    if vals.len() >= 7 {
                        let idle = vals[3] + vals[4];
                        let total: f64 = vals.iter().take(7).sum();
                        cpu_stat.insert(key.to_string(), (idle, total));
                    }
                }
            }
        }
    }

    let mut cpu_total_pct = 0.0;
    if let (Some(cur), Some(prev)) = (cpu_stat.get("cpu"), prev_state.get("cpu").and_then(|v| v.get("cpu"))) {
        if let (Some(p_idle), Some(p_tot)) = (prev.get(0).and_then(|v| v.as_f64()), prev.get(1).and_then(|v| v.as_f64())) {
            let d_tot = (cur.1 - p_tot).max(1.0);
            let d_idle = cur.0 - p_idle;
            cpu_total_pct = ((1.0 - (d_idle / d_tot)) * 100.0).clamp(0.0, 100.0);
        }
    }

    let mut core_keys: Vec<String> = cpu_stat.keys().filter(|k| *k != "cpu").cloned().collect();
    core_keys.sort_by_key(|k| k[3..].parse::<usize>().unwrap_or(0));

    let mut cores_pct: Vec<f64> = Vec::new();
    for k in &core_keys {
        if let (Some(cur), Some(prev)) = (cpu_stat.get(k), prev_state.get("cpu").and_then(|v| v.get(k))) {
            if let (Some(p_idle), Some(p_tot)) = (prev.get(0).and_then(|v| v.as_f64()), prev.get(1).and_then(|v| v.as_f64())) {
                let d_tot = (cur.1 - p_tot).max(1.0);
                let d_idle = cur.0 - p_idle;
                cores_pct.push(((1.0 - (d_idle / d_tot)) * 100.0).clamp(0.0, 100.0));
            }
        }
    }

    // CPU Temp
    let mut cpu_temp = -1;
    for tp in [
        "/sys/class/thermal/thermal_zone0/temp",
        "/sys/class/hwmon/hwmon1/temp1_input",
        "/sys/class/hwmon/hwmon6/temp1_input",
    ] {
        if let Ok(s) = fs::read_to_string(tp) {
            if let Ok(v) = s.trim().parse::<i32>() {
                cpu_temp = if v > 1000 { (v as f64 / 1000.0).round() as i32 } else { v };
                break;
            }
        }
    }

    // CPU Frequency
    let mut cpu_freq = 0.0;
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        for l in cpuinfo.lines() {
            if l.starts_with("cpu MHz") {
                if let Some(mhz_str) = l.split(':').nth(1) {
                    if let Ok(mhz) = mhz_str.trim().parse::<f64>() {
                        cpu_freq = (mhz / 1000.0 * 100.0).round() / 100.0;
                        break;
                    }
                }
            }
        }
    }

    // 2. Memory
    let mut mem_total_kb = 1.0;
    let mut mem_avail_kb = 0.0;
    let mut swap_total_kb = 0.0;
    let mut swap_free_kb = 0.0;

    if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
        for l in meminfo.lines() {
            let mut parts = l.split_whitespace();
            if let (Some(key), Some(val_str)) = (parts.next(), parts.next()) {
                if let Ok(val) = val_str.parse::<f64>() {
                    match key {
                        "MemTotal:" => mem_total_kb = val,
                        "MemAvailable:" => mem_avail_kb = val,
                        "SwapTotal:" => swap_total_kb = val,
                        "SwapFree:" => swap_free_kb = val,
                        _ => {}
                    }
                }
            }
        }
    }

    let mem_total_gib = mem_total_kb / (1024.0 * 1024.0);
    let mem_used_gib = (mem_total_kb - mem_avail_kb) / (1024.0 * 1024.0);
    let mem_pct = ((mem_used_gib / mem_total_gib.max(0.1)) * 100.0).clamp(0.0, 100.0);

    let swap_total_gib = swap_total_kb / (1024.0 * 1024.0);
    let swap_used_gib = (swap_total_kb - swap_free_kb) / (1024.0 * 1024.0);
    let swap_pct = if swap_total_gib > 0.0 {
        ((swap_used_gib / swap_total_gib) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    // 3. Network
    let mut net_stat: HashMap<String, (u64, u64)> = HashMap::new();
    if let Ok(net_content) = fs::read_to_string("/proc/net/dev") {
        for l in net_content.lines() {
            if let Some((iface, data)) = l.split_once(':') {
                let iface = iface.trim();
                if iface == "lo" {
                    continue;
                }
                let nums: Vec<u64> = data.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                if nums.len() >= 9 {
                    net_stat.insert(iface.to_string(), (nums[0], nums[8]));
                }
            }
        }
    }

    let prev_time = prev_state.get("time").and_then(|v| v.as_f64()).unwrap_or(now - 2.0);
    let dt = (now - prev_time).max(0.1);

    let mut total_rx_rate = 0.0;
    let mut total_tx_rate = 0.0;
    let mut active_iface = String::from("--");
    let mut max_bytes = 0;

    if let Some(prev_net) = prev_state.get("net").and_then(|v| v.as_object()) {
        for (iface, &(rx, tx)) in &net_stat {
            if let Some(prev_pair) = prev_net.get(iface).and_then(|v| v.as_array()) {
                if let (Some(p_rx), Some(p_tx)) = (prev_pair.get(0).and_then(|v| v.as_u64()), prev_pair.get(1).and_then(|v| v.as_u64())) {
                    let d_rx = rx.saturating_sub(p_rx) as f64 / dt;
                    let d_tx = tx.saturating_sub(p_tx) as f64 / dt;
                    total_rx_rate += d_rx;
                    total_tx_rate += d_tx;
                    if (rx + tx) > max_bytes {
                        max_bytes = rx + tx;
                        active_iface = iface.clone();
                    }
                }
            }
        }
    }

    // 4. Disks via statvfs
    let mut disks: Vec<DiskData> = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    let candidates = [
        ("home", "/var/home"),
        ("home", "/home"),
        ("root", "/sysroot"),
        ("root", "/"),
    ];

    for &(name, mount) in &candidates {
        if seen_names.contains(name) {
            continue;
        }
        if let Ok(c_path) = CString::new(mount) {
            let mut buf = std::mem::MaybeUninit::<StatVfs>::uninit();
            let res = unsafe { statvfs(c_path.as_ptr(), buf.as_mut_ptr()) };
            if res == 0 {
                let s = unsafe { buf.assume_init() };
                let tot = (s.f_blocks * s.f_frsize) as f64 / (1024.0 * 1024.0 * 1024.0);
                let used = ((s.f_blocks - s.f_bfree) * s.f_frsize) as f64 / (1024.0 * 1024.0 * 1024.0);
                if tot >= 1.0 {
                    let pct = ((used / tot.max(0.1)) * 100.0).clamp(0.0, 100.0);
                    disks.push(DiskData {
                        name: name.to_string(),
                        mount: mount.to_string(),
                        used: (used * 10.0).round() / 10.0,
                        total: (tot * 10.0).round() / 10.0,
                        pct: (pct * 10.0).round() / 10.0,
                    });
                    seen_names.insert(name);
                }
            }
        }
    }

    // 5. GPU (NVIDIA)
    let mut gpu = GpuData {
        name: "N/A".to_string(),
        usage: -1.0,
        temp: -1.0,
        vram_used: -1.0,
        vram_total: -1.0,
        vram_pct: 0.0,
    };

    if let Ok(output) = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
            if parts.len() >= 5 {
                gpu.name = parts[0].to_string();
                gpu.usage = parts[1].parse::<f64>().unwrap_or(-1.0);
                gpu.temp = parts[2].parse::<f64>().unwrap_or(-1.0);
                let u_gib = parts[3].parse::<f64>().unwrap_or(0.0) / 1024.0;
                let t_gib = parts[4].parse::<f64>().unwrap_or(1.0) / 1024.0;
                gpu.vram_used = (u_gib * 100.0).round() / 100.0;
                gpu.vram_total = (t_gib * 100.0).round() / 100.0;
                gpu.vram_pct = ((u_gib / t_gib.max(0.1)) * 100.0).clamp(0.0, 100.0);
            }
        }
    }

    // 6. Top Processes
    let mut procs: Vec<ProcData> = Vec::new();
    if let Ok(output) = Command::new("ps")
        .args(["-eo", "pid,pcpu,pmem,comm", "--sort=-pcpu", "--no-headers"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for l in text.lines() {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[3..].join(" ");
                    if name == "ps" || name == "wt-rust" {
                        continue;
                    }
                    procs.push(ProcData {
                        pid: parts[0].to_string(),
                        cpu: parts[1].to_string(),
                        mem: parts[2].to_string(),
                        name,
                    });
                    if procs.len() >= 6 {
                        break;
                    }
                }
            }
        }
    }

    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_default()
        .to_lowercase();
    let is_turkish = lang.starts_with("tr");

    // 7. Battery & Uptime
    let mut uptime = String::from("--");
    let mut uptime_secs: u64 = 0;
    if let Ok(s) = fs::read_to_string("/proc/uptime") {
        if let Some(sec_str) = s.split_whitespace().next() {
            if let Ok(secs) = sec_str.parse::<f64>() {
                uptime_secs = secs as u64;
                let h = uptime_secs / 3600;
                let m = (uptime_secs % 3600) / 60;
                if is_turkish {
                    uptime = format!("{} sa {} dk", h, m);
                } else {
                    uptime = format!("{}h {}m", h, m);
                }
            }
        }
    }

    let mut bat = BatteryData {
        pct: -1,
        status: String::from("N/A"),
    };
    for bp in ["/sys/class/power_supply/BAT1", "/sys/class/power_supply/BAT0"] {
        let p = Path::new(bp);
        if p.exists() {
            if let Ok(cap) = fs::read_to_string(p.join("capacity")) {
                if let Ok(pct) = cap.trim().parse::<i32>() {
                    bat.pct = pct;
                }
            }
            if let Ok(st) = fs::read_to_string(p.join("status")) {
                bat.status = st.trim().to_string();
            }
            break;
        }
    }

    // Save State
    let next_state = serde_json::json!({
        "cpu": cpu_stat,
        "net": net_stat,
        "time": now
    });
    let _ = fs::write(state_file, next_state.to_string());

    let pal_file = format!("{}/.cache/wt-rust/palette.json", std::env::var("HOME").unwrap_or_default());
    let palette: Option<serde_json::Value> = fs::read_to_string(pal_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    let full_stats = FullStats {
        cpu: CpuData {
            total: (cpu_total_pct * 10.0).round() / 10.0,
            cores: cores_pct.into_iter().map(|c| (c * 10.0).round() / 10.0).collect(),
            temp: cpu_temp,
            freq: cpu_freq,
        },
        mem: MemData {
            used: (mem_used_gib * 10.0).round() / 10.0,
            total: (mem_total_gib * 10.0).round() / 10.0,
            pct: (mem_pct * 10.0).round() / 10.0,
            swap_used: (swap_used_gib * 10.0).round() / 10.0,
            swap_total: (swap_total_gib * 10.0).round() / 10.0,
            swap_pct: (swap_pct * 10.0).round() / 10.0,
        },
        gpu,
        net: NetData {
            iface: active_iface,
            rx_rate: format_rate(total_rx_rate),
            tx_rate: format_rate(total_tx_rate),
            rx_raw: total_rx_rate as u64,
            tx_raw: total_tx_rate as u64,
        },
        disks,
        procs,
        battery: bat,
        uptime,
        uptime_secs,
        is_turkish,
        palette,
    };

    if let Ok(json_out) = serde_json::to_string(&full_stats) {
        println!("{}", json_out);
    }
}
