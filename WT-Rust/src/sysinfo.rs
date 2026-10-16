use serde::Serialize;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Serialize)]
pub struct WaybarSysinfoOutput {
    pub text: String,
    pub tooltip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

fn read_cpu_stat() -> Option<(u64, u64)> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let first_line = stat.lines().next()?;
    if !first_line.starts_with("cpu ") {
        return None;
    }
    let parts: Vec<u64> = first_line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() < 4 {
        return None;
    }
    let idle = parts[3] + parts.get(4).copied().unwrap_or(0); // idle + iowait
    let total: u64 = parts.iter().sum();
    Some((total, idle))
}

fn get_cpu_usage() -> u32 {
    let state_file = Path::new("/tmp/wt-rust-cpu.stat");
    let current = read_cpu_stat();

    let Ok(cur) = current.ok_or(()) else {
        return 0;
    };

    if let Ok(prev_str) = fs::read_to_string(state_file) {
        let parts: Vec<u64> = prev_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 2 {
            let prev_total = parts[0];
            let prev_idle = parts[1];
            let diff_total = cur.0.saturating_sub(prev_total);
            let diff_idle = cur.1.saturating_sub(prev_idle);

            let _ = fs::write(state_file, format!("{} {}", cur.0, cur.1));
            if diff_total > 0 {
                let usage = (diff_total.saturating_sub(diff_idle) as f64 / diff_total as f64 * 100.0).round() as u32;
                return usage.min(100);
            }
        }
    }

    // Fallback on initial cold launch: sample over 120ms
    thread::sleep(Duration::from_millis(120));
    if let Some(next) = read_cpu_stat() {
        let diff_total = next.0.saturating_sub(cur.0);
        let diff_idle = next.1.saturating_sub(cur.1);
        let _ = fs::write(state_file, format!("{} {}", next.0, next.1));
        if diff_total > 0 {
            let usage = (diff_total.saturating_sub(diff_idle) as f64 / diff_total as f64 * 100.0).round() as u32;
            return usage.min(100);
        }
    }

    let _ = fs::write(state_file, format!("{} {}", cur.0, cur.1));
    0
}

fn get_cpu_temp() -> Option<u32> {
    let hwmon_dir = Path::new("/sys/class/hwmon");
    let Ok(entries) = fs::read_dir(hwmon_dir) else {
        return None;
    };

    let mut candidate_temp = None;
    for entry in entries.flatten() {
        let p = entry.path();
        let name = fs::read_to_string(p.join("name")).unwrap_or_default().trim().to_string();

        if matches!(name.as_str(), "coretemp" | "k10temp" | "zenpower" | "cpu_thermal" | "acpitz") {
            // Find temp1_input or any temp*_input
            for i in 1..=8 {
                let temp_path = p.join(format!("temp{}_input", i));
                if let Ok(val_str) = fs::read_to_string(temp_path)
                    && let Ok(val) = val_str.trim().parse::<u32>()
                    && val > 0
                    && val < 130000
                {
                    return Some(val / 1000);
                }
            }
        } else if candidate_temp.is_none() {
            for i in 1..=3 {
                let temp_path = p.join(format!("temp{}_input", i));
                if let Ok(val_str) = fs::read_to_string(temp_path)
                    && let Ok(val) = val_str.trim().parse::<u32>()
                    && (20000..130000).contains(&val)
                {
                    candidate_temp = Some(val / 1000);
                    break;
                }
            }
        }
    }

    candidate_temp
}

fn get_top_cpu_processes() -> Vec<(String, u64)> {
    let proc_dir = Path::new("/proc");
    let Ok(entries) = fs::read_dir(proc_dir) else {
        return Vec::new();
    };

    let mut procs = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let pid_path = entry.path();
        let comm = fs::read_to_string(pid_path.join("comm"))
            .unwrap_or_default()
            .trim()
            .to_string();
        if comm.is_empty() {
            continue;
        }

        if let Ok(stat) = fs::read_to_string(pid_path.join("stat")) {
            // utime is field 14, stime is field 15 (1-indexed)
            let fields: Vec<&str> = stat.split_whitespace().collect();
            if fields.len() >= 15 {
                let utime: u64 = fields[13].parse().unwrap_or(0);
                let stime: u64 = fields[14].parse().unwrap_or(0);
                let total_ticks = utime + stime;
                procs.push((comm, total_ticks));
            }
        }
    }

    procs.sort_by_key(|a| std::cmp::Reverse(a.1));
    procs.truncate(3);
    procs
}

pub fn get_cpu() {
    let pal = crate::theme::get_current_palette();
    let usage = get_cpu_usage();
    let temp_opt = get_cpu_temp();
    let temp_str = temp_opt
        .map(|t| format!("  <span color='{}'>{}°C</span>", pal.text_dim, t))
        .unwrap_or_default();

    let text = format!("<span color='{}'></span> {}%{}", pal.accent, usage, temp_str);

    let top_procs = get_top_cpu_processes();
    let mut tooltip_lines = vec![format!("İşlemci Kullanımı: %{}\nSıcaklık: {}°C\n", usage, temp_opt.unwrap_or(0))];
    if !top_procs.is_empty() {
        tooltip_lines.push("En Çok Tutan Süreçler:".to_string());
        for (i, (comm, _)) in top_procs.iter().enumerate() {
            tooltip_lines.push(format!("{}. {}", i + 1, comm));
        }
    }

    let class = if usage >= 85 {
        Some("critical".to_string())
    } else if usage >= 65 {
        Some("warning".to_string())
    } else {
        None
    };

    let resp = WaybarSysinfoOutput {
        text,
        tooltip: tooltip_lines.join("\n"),
        class,
    };

    if let Ok(json) = serde_json::to_string(&resp) {
        println!("{}", json);
    }
}

fn get_top_mem_processes() -> Vec<(String, u64)> {
    let proc_dir = Path::new("/proc");
    let Ok(entries) = fs::read_dir(proc_dir) else {
        return Vec::new();
    };

    let mut procs = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let pid_path = entry.path();
        let comm = fs::read_to_string(pid_path.join("comm"))
            .unwrap_or_default()
            .trim()
            .to_string();
        if comm.is_empty() {
            continue;
        }

        // statm second field is resident set size (pages, typically 4KiB)
        if let Ok(statm) = fs::read_to_string(pid_path.join("statm"))
            && let Some(rss_str) = statm.split_whitespace().nth(1)
            && let Ok(rss_pages) = rss_str.parse::<u64>()
        {
            let rss_bytes = rss_pages * 4096;
            procs.push((comm, rss_bytes));
        }
    }

    procs.sort_by_key(|a| std::cmp::Reverse(a.1));
    procs.truncate(3);
    procs
}

fn format_bytes(bytes: u64) -> String {
    let mib = bytes as f64 / 1024.0 / 1024.0;
    if mib >= 1024.0 {
        format!("{:.1} GiB", mib / 1024.0)
    } else {
        format!("{:.0} MiB", mib)
    }
}

pub fn get_memory() {
    let Ok(meminfo) = fs::read_to_string("/proc/meminfo") else {
        println!("{{\"text\":\"󰍛 N/A\",\"tooltip\":\"\"}}");
        return;
    };

    let mut total_kib = 0u64;
    let mut avail_kib = 0u64;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            total_kib = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            avail_kib = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
    }

    if total_kib == 0 {
        println!("{{\"text\":\"󰍛 N/A\",\"tooltip\":\"\"}}");
        return;
    }

    let used_kib = total_kib.saturating_sub(avail_kib);
    let pct = (used_kib as f64 / total_kib as f64 * 100.0).round() as u32;

    let used_str = format_bytes(used_kib * 1024);
    let total_str = format_bytes(total_kib * 1024);

    let pal = crate::theme::get_current_palette();
    let text = format!("<span color='{}'>󰍛</span> {}%", pal.accent, pct);

    let top_procs = get_top_mem_processes();
    let mut tooltip_lines = vec![format!("Bellek Kullanımı: %{} ({} / {})\n", pct, used_str, total_str)];

    if !top_procs.is_empty() {
        tooltip_lines.push("En Çok Tüketen Süreçler:".to_string());
        for (i, (comm, bytes)) in top_procs.iter().enumerate() {
            tooltip_lines.push(format!("{}. {} ({})", i + 1, comm, format_bytes(*bytes)));
        }
    }

    let class = if pct >= 85 {
        Some("critical".to_string())
    } else if pct >= 70 {
        Some("warning".to_string())
    } else {
        None
    };

    let resp = WaybarSysinfoOutput {
        text,
        tooltip: tooltip_lines.join("\n"),
        class,
    };

    if let Ok(json) = serde_json::to_string(&resp) {
        println!("{}", json);
    }
}

pub fn handle_sysinfo(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("memory" | "mem") => get_memory(),
        _ => get_cpu(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500 * 1024 * 1024), "500 MiB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.0 GiB");
    }

    #[test]
    fn test_read_cpu_stat() {
        let stat = read_cpu_stat();
        assert!(stat.is_some());
        let (total, idle) = stat.unwrap();
        assert!(total > 0 && total >= idle);
    }
}
