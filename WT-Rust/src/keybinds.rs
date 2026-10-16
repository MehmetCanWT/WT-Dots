use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct KeybindAction {
    pub id: String,
    pub title: String,
    pub key: String,
    pub cmd: String,
}

pub fn normalize_key(raw: &str) -> String {
    let parts: Vec<String> = raw
        .split('+')
        .map(|p| {
            let trimmed = p.trim();
            if trimmed.eq_ignore_ascii_case("enter") {
                "Return".to_string()
            } else if trimmed.eq_ignore_ascii_case("meta") || trimmed.eq_ignore_ascii_case("super") {
                "Meta".to_string()
            } else if trimmed.eq_ignore_ascii_case("ctrl") || trimmed.eq_ignore_ascii_case("control") {
                "Ctrl".to_string()
            } else if trimmed.eq_ignore_ascii_case("alt") {
                "Alt".to_string()
            } else if trimmed.eq_ignore_ascii_case("shift") {
                "Shift".to_string()
            } else {
                trimmed.to_string()
            }
        })
        .collect();

    parts.join("+")
}

pub fn derive_action_info(key: &str, cmd: &str) -> (String, String) {
    let cmd_lower = cmd.to_lowercase();
    if cmd_lower.starts_with("kitty") || cmd_lower.contains("terminal") {
        ("Terminal".to_string(), "Terminal (Kitty)".to_string())
    } else if cmd_lower.contains("wallpaper select") {
        ("Wallpaper".to_string(), "Wallpaper Selector".to_string())
    } else if cmd_lower.contains("clipboard") {
        ("Clipboard".to_string(), "Clipboard History".to_string())
    } else {
        let base_id = cmd
            .split_whitespace()
            .next()
            .unwrap_or("App")
            .split('/')
            .next_back()
            .unwrap_or("App")
            .replace(['-', '_'], " ");
        let mut chars = base_id.chars();
        let cap_id = match chars.next() {
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            None => "App".to_string(),
        };
        (
            format!("Action{}", key.replace('+', "")),
            format!("{} ({})", cap_id, key),
        )
    }
}

pub fn key_to_qt_code(key: &str) -> Option<i32> {
    let mut modifier = 0i32;
    let mut base_code = None;

    for part in key.split('+') {
        let p = part.trim();
        if p.eq_ignore_ascii_case("meta") || p.eq_ignore_ascii_case("super") {
            modifier |= 0x1000_0000;
        } else if p.eq_ignore_ascii_case("ctrl") || p.eq_ignore_ascii_case("control") {
            modifier |= 0x0400_0000;
        } else if p.eq_ignore_ascii_case("alt") {
            modifier |= 0x0800_0000;
        } else if p.eq_ignore_ascii_case("shift") {
            modifier |= 0x0200_0000;
        } else if p.eq_ignore_ascii_case("return") || p.eq_ignore_ascii_case("enter") {
            base_code = Some(0x0100_0004);
        } else if p.eq_ignore_ascii_case("space") {
            base_code = Some(0x20);
        } else if p.eq_ignore_ascii_case("tab") {
            base_code = Some(0x0100_0001);
        } else if p.eq_ignore_ascii_case("escape") || p.eq_ignore_ascii_case("esc") {
            base_code = Some(0x0100_0000);
        } else if p.len() == 1 {
            let ch = p.chars().next()?.to_ascii_uppercase();
            base_code = Some(ch as i32);
        } else if (p.starts_with('F') || p.starts_with('f'))
            && let Ok(num) = p[1..].parse::<i32>()
            && (1..=35).contains(&num)
        {
            base_code = Some(0x0100_0030 + (num - 1));
        }
    }

    base_code.map(|b| modifier | b)
}

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

    let mut actions = Vec::new();
    let mut seen_ids = HashSet::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }

        if let Some((raw_key, raw_cmd)) = line.split_once('=') {
            let raw_key = raw_key.trim();
            let raw_cmd = raw_cmd.trim();
            if raw_key.is_empty() || raw_cmd.is_empty() {
                continue;
            }

            let key = normalize_key(raw_key);
            let (mut id, title) = derive_action_info(&key, raw_cmd);
            if seen_ids.contains(&id) {
                id = format!("{}{}", id, actions.len());
            }
            seen_ids.insert(id.clone());

            actions.push(KeybindAction {
                id,
                title,
                key,
                cmd: raw_cmd.to_string(),
            });
        }
    }

    // Build consolidated WTDots desktop entry with actions
    let action_list = actions
        .iter()
        .map(|a| a.id.as_str())
        .collect::<Vec<&str>>()
        .join(";");

    let mut desktop_content = format!(
        "[Desktop Entry]\n\
         Name=WTDots\n\
         Comment=WT-Dots System Tools & Shortcuts\n\
         Exec=wt-rust\n\
         Icon=preferences-desktop-keyboard-shortcuts\n\
         Type=Application\n\
         Categories=Utility;\n\
         StartupNotify=false\n\
         Actions={};\n\n",
        action_list
    );

    for act in &actions {
        desktop_content.push_str(&format!(
            "[Desktop Action {}]\n\
             Name={}\n\
             Exec={}\n\
             X-KDE-Shortcuts={}\n\n",
            act.id, act.title, act.cmd, act.key
        ));
    }

    let wtdots_desktop = apps_dir.join("wtdots.desktop");
    let _ = fs::write(&wtdots_desktop, desktop_content);
    // Sync to KDE global shortcuts under [services][wtdots.desktop]
    let _ = Command::new("kwriteconfig6")
        .args([
            "--file",
            "kglobalshortcutsrc",
            "--group",
            "services",
            "--group",
            "wtdots.desktop",
            "--key",
            "_launch",
            "none,none,WTDots",
        ])
        .status();

    for act in &actions {
        let action_val = format!("{},none,{}", act.key, act.title);
        let _ = Command::new("kwriteconfig6")
            .args([
                "--file",
                "kglobalshortcutsrc",
                "--group",
                "services",
                "--group",
                "wtdots.desktop",
                "--key",
                &act.id,
                &action_val,
            ])
            .status();
    }

    // Unbind plasmashell default clipboard shortcut if Meta+V is used by WTDots
    let has_meta_v = actions.iter().any(|a| a.key == "Meta+V");
    if has_meta_v {
        let _ = Command::new("kwriteconfig6")
            .args([
                "--file",
                "kglobalshortcutsrc",
                "--group",
                "plasmashell",
                "--key",
                "show-on-mouse-pos",
                "none,none,Fare Konumunda Pano Ögelerini Göster",
            ])
            .status();
    }

    // Unbind KWin Overview shortcut if Meta+W is used by WTDots
    let has_meta_w = actions.iter().any(|a| a.key == "Meta+W");
    if has_meta_w {
        let _ = Command::new("kwriteconfig6")
            .args([
                "--file",
                "kglobalshortcutsrc",
                "--group",
                "kwin",
                "--key",
                "Overview",
                "none,Meta+W,Genel Görünümü Aç/Kapat",
            ])
            .status();
    }

    // Ensure KWin window rule for wt-clipboard opens under mouse cursor
    ensure_clipboard_kwin_rule();

    // Clean up any legacy separate desktop shortcut files
    if let Ok(entries) = fs::read_dir(&apps_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if (fname.starts_with("wt-bind-") || fname == "wt-wallpaper-picker.desktop")
                && fname.ends_with(".desktop")
            {
                let _ = fs::remove_file(entry.path());
                let _ = Command::new("kwriteconfig6")
                    .args([
                        "--file",
                        "kglobalshortcutsrc",
                        "--group",
                        "services",
                        "--group",
                        &fname,
                        "--key",
                        "_launch",
                        "--delete",
                    ])
                    .status();
            }
        }
    }

    let _ = Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();

    // Rebuild KDE system configuration cache so KSycoca discovers wtdots.desktop immediately
    let _ = Command::new("kbuildsycoca6").status();

    // Register actions with KDE kglobalaccel DBus interface and bind active shortcut keys
    for act in &actions {
        let _ = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.kde.kglobalaccel",
                "/kglobalaccel",
                "org.kde.KGlobalAccel",
                "doRegister",
                "as",
                "4",
                "wtdots.desktop",
                &act.id,
                "WTDots",
                &act.title,
            ])
            .status();

        if let Some(qt_code) = key_to_qt_code(&act.key) {
            let _ = Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.kde.kglobalaccel",
                    "/kglobalaccel",
                    "org.kde.KGlobalAccel",
                    "setForeignShortcut",
                    "asai",
                    "4",
                    "wtdots.desktop",
                    &act.id,
                    "WTDots",
                    &act.title,
                    "1",
                    &qt_code.to_string(),
                ])
                .status();
        }
    }

    let _ = Command::new("qdbus-qt6")
        .args(["org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"])
        .status();

    println!("WTDots keybindings synchronized with KDE Plasma 6.");
}

pub fn ensure_clipboard_kwin_rule() {
    let _ = Command::new("kwriteconfig6")
        .args([
            "--file",
            "kwinrulesrc",
            "--group",
            "1",
            "--key",
            "Description",
            "WT-Clipboard",
        ])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "placement", "UnderMouse"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "placementrule", "2"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "noborder", "true"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "noborderrule", "2"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "above", "true"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "aboverule", "2"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "skiptaskbar", "true"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "skiptaskbarrule", "2"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "skipswitcher", "true"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "skipswitcherrule", "2"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "wmclass", "wt-clipboard"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "1", "--key", "wmclassmatch", "1"])
        .status();
    let _ = Command::new("kwriteconfig6")
        .args(["--file", "kwinrulesrc", "--group", "General", "--key", "count", "1"])
        .status();
    let _ = Command::new("qdbus-qt6")
        .args(["org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"])
        .status();
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
        if let Ok(meta) = fs::metadata(&conf_path)
            && let Ok(mtime) = meta.modified()
            && mtime > last_mtime
        {
            last_mtime = mtime;
            println!("Changes detected in keybinds.conf. Re-syncing...");
            sync_keybinds();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("Meta+Enter"), "Meta+Return");
        assert_eq!(normalize_key("meta+enter"), "Meta+Return");
        assert_eq!(normalize_key("super+enter"), "Meta+Return");
        assert_eq!(normalize_key("Meta+W"), "Meta+W");
        assert_eq!(normalize_key("ctrl+alt+t"), "Ctrl+Alt+t");
    }

    #[test]
    fn test_derive_action_info() {
        let (id, title) = derive_action_info("Meta+Return", "kitty");
        assert_eq!(id, "Terminal");
        assert_eq!(title, "Terminal (Kitty)");

        let (id2, title2) = derive_action_info("Meta+W", "wt-rust wallpaper select");
        assert_eq!(id2, "Wallpaper");
        assert_eq!(title2, "Wallpaper Selector");

        let (id3, title3) = derive_action_info("Meta+V", "wt-rust clipboard");
        assert_eq!(id3, "Clipboard");
        assert_eq!(title3, "Clipboard History");
    }

    #[test]
    fn test_key_to_qt_code() {
        assert_eq!(key_to_qt_code("Meta+Return"), Some(285212676));
        assert_eq!(key_to_qt_code("Meta+Enter"), Some(285212676));
        assert_eq!(key_to_qt_code("Meta+W"), Some(268435543));
        assert_eq!(key_to_qt_code("Meta+V"), Some(268435542));
        assert_eq!(key_to_qt_code("Ctrl+Alt+T"), Some(201326676));
    }
}
