use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn sync_keybinds() {
    let home = match env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => return,
    };

    let conf_path = home.join(".config/keybinds.conf");
    if !conf_path.exists() {
        eprintln!("Keybinds config not found: {:?}", conf_path);
        return;
    }

    let content = match fs::read_to_string(&conf_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let apps_dir = home.join(".local/share/applications");
    let _ = fs::create_dir_all(&apps_dir);

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }

        if let Some((key, cmd)) = line.split_once('=') {
            let key = key.trim();
            let cmd = cmd.trim();
            if key.is_empty() || cmd.is_empty() {
                continue;
            }

            let slug = format!("wt-bind-{}", key.to_ascii_lowercase().replace('+', "-"));
            let desktop_file = format!("{}.desktop", slug);
            let desktop_path = apps_dir.join(&desktop_file);

            let desktop_content = format!(
                "[Desktop Entry]\n\
                 Name=WT Shortcut {}\n\
                 Exec={}\n\
                 Type=Application\n\
                 NoDisplay=true\n\
                 Categories=Utility;\n\
                 StartupNotify=false\n\
                 X-KDE-Shortcuts={}\n",
                key, cmd, key
            );

            let _ = fs::write(&desktop_path, desktop_content);

            let action_val = format!("{},none,WT Shortcut {}", key, key);
            let _ = Command::new("kwriteconfig6")
                .args([
                    "--file", "kglobalshortcutsrc",
                    "--group", "services",
                    "--group", &desktop_file,
                    "--key", "_launch",
                    &action_val,
                ])
                .status();
        }
    }

    let _ = Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();

    let _ = Command::new("systemctl")
        .args(["--user", "restart", "plasma-kglobalaccel.service"])
        .status();

    println!("Keybinds synchronized with KDE Plasma 6.");
}

pub fn watch_keybinds() {
    let home = match env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => return,
    };

    let conf_path = home.join(".config/keybinds.conf");
    println!("Watching {:?} for changes...", conf_path);

    // Initial sync
    sync_keybinds();

    let mut last_mtime = fs::metadata(&conf_path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(800));
        if let Ok(meta) = fs::metadata(&conf_path) {
            if let Ok(mtime) = meta.modified() {
                if mtime > last_mtime {
                    last_mtime = mtime;
                    println!("Changes detected in keybinds.conf. Re-syncing...");
                    sync_keybinds();
                }
            }
        }
    }
}

pub fn handle_keybinds(args: &[String]) {
    if args.first().map(|s| s.as_str()) == Some("watch") {
        watch_keybinds();
    } else {
        sync_keybinds();
    }
}

