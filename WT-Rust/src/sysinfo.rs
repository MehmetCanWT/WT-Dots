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
    let Some(cur) = read_cpu_stat() else {
        return 0;
    };

    if let Ok(prev_str) = fs::read_to_string(state_file) {
        let parts: Vec<u64> = prev_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 2 {
            let diff_total = cur.0.saturating_sub(parts[0]);
            let diff_idle = cur.1.saturating_sub(parts[1]);

            let _ = fs::write(state_file, format!("{} {}", cur.0, cur.1));
            if diff_total > 0 {
                let usage = (diff_total.saturating_sub(diff_idle) as f64 / diff_total as f64
                    * 100.0)
                    .round() as u32;
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
            let usage = (diff_total.saturating_sub(diff_idle) as f64 / diff_total as f64 * 100.0)
                .round() as u32;
            return usage.min(100);
        }
    }

    let _ = fs::write(state_file, format!("{} {}", cur.0, cur.1));
    0
}

fn get_cpu_temp() -> Option<u32> {
    let cache_file = Path::new("/tmp/wt-rust-hwmon.cache");
    if let Ok(cached_path) = fs::read_to_string(cache_file) {
        let p = Path::new(cached_path.trim());
        if let Ok(val_str) = fs::read_to_string(p)
            && let Ok(val) = val_str.trim().parse::<u32>()
            && val > 0
            && val < 130000
        {
            return Some(val / 1000);
        }
    }

    let hwmon_dir = Path::new("/sys/class/hwmon");
    let Ok(entries) = fs::read_dir(hwmon_dir) else {
        return None;
    };

    let mut candidate_temp = None;
    let mut candidate_path = None;
    for entry in entries.flatten() {
        let p = entry.path();
        let name = fs::read_to_string(p.join("name"))
            .unwrap_or_default()
            .trim()
            .to_string();

        if matches!(
            name.as_str(),
            "coretemp" | "k10temp" | "zenpower" | "cpu_thermal" | "acpitz"
        ) {
            for i in 1..=8 {
                let temp_path = p.join(format!("temp{}_input", i));
                if let Ok(val_str) = fs::read_to_string(&temp_path)
                    && let Ok(val) = val_str.trim().parse::<u32>()
                    && val > 0
                    && val < 130000
                {
                    let _ = fs::write(cache_file, temp_path.to_string_lossy().as_bytes());
                    return Some(val / 1000);
                }
            }
        } else if candidate_temp.is_none() {
            for i in 1..=3 {
                let temp_path = p.join(format!("temp{}_input", i));
                if let Ok(val_str) = fs::read_to_string(&temp_path)
                    && let Ok(val) = val_str.trim().parse::<u32>()
                    && (20000..130000).contains(&val)
                {
                    candidate_temp = Some(val / 1000);
                    candidate_path = Some(temp_path);
                    break;
                }
            }
        }
    }

    if let Some(p) = candidate_path {
        let _ = fs::write(cache_file, p.to_string_lossy().as_bytes());
    }
    candidate_temp
}

fn parse_proc_stat_ticks(stat: &str) -> Option<u64> {
    let end_comm = stat.rfind(')')?;
    let mut after = stat[end_comm + 1..].split_whitespace();
    let utime: u64 = after.nth(11)?.parse().ok()?;
    let stime: u64 = after.next()?.parse().ok()?;
    Some(utime + stime)
}

fn scan_top_processes(by_memory: bool) -> Vec<(String, u64)> {
    let cache_path = if by_memory {
        Path::new("/tmp/wt-rust-top-mem.cache")
    } else {
        Path::new("/tmp/wt-rust-top-cpu.cache")
    };

    if let Ok(meta) = fs::metadata(cache_path)
        && let Ok(modified) = meta.modified()
        && modified.elapsed().unwrap_or(Duration::from_secs(999)) < Duration::from_secs(10)
        && let Ok(cached) = fs::read_to_string(cache_path)
    {
        let parsed: Vec<(String, u64)> = cached
            .lines()
            .filter_map(|line| {
                let (comm, val) = line.split_once('\t')?;
                Some((comm.to_string(), val.parse().ok()?))
            })
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    let mut procs = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
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

        if by_memory {
            if let Ok(statm) = fs::read_to_string(pid_path.join("statm"))
                && let Some(rss_str) = statm.split_whitespace().nth(1)
                && let Ok(rss_pages) = rss_str.parse::<u64>()
            {
                procs.push((comm, rss_pages * 4096));
            }
        } else if let Ok(stat) = fs::read_to_string(pid_path.join("stat"))
            && let Some(ticks) = parse_proc_stat_ticks(&stat)
        {
            procs.push((comm, ticks));
        }
    }

    procs.sort_by_key(|a| std::cmp::Reverse(a.1));
    procs.truncate(3);

    let cache_body: String = procs
        .iter()
        .map(|(c, v)| format!("{c}\t{v}"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(cache_path, cache_body);

    procs
}

pub fn get_cpu() {
    let usage = get_cpu_usage();
    let temp_opt = get_cpu_temp();
    let temp_str = temp_opt.map(|t| format!("  {}°C", t)).unwrap_or_default();

    let text = format!(" {}%{}", usage, temp_str);

    let top_procs = scan_top_processes(false);
    let mut tooltip_lines = vec![format!(
        "İşlemci Kullanımı: %{}\nSıcaklık: {}°C\n",
        usage,
        temp_opt.unwrap_or(0)
    )];
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

    let text = format!("󰍛 {}%", pct);

    let top_procs = scan_top_processes(true);
    let mut tooltip_lines = vec![format!(
        "Bellek Kullanımı: %{} ({} / {})\n",
        pct, used_str, total_str
    )];

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

    #[test]
    fn test_parse_proc_stat_ticks_with_space_in_comm() {
        let sample = "1234 (Web Content) S 1 1234 1234 0 -1 4194304 100 0 0 0 450 150 0 0 20 0 10 0 1000 1000000 500";
        assert_eq!(parse_proc_stat_ticks(sample), Some(600));
    }
}
