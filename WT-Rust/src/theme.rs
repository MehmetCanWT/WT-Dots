use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bg: String,
    pub border: String,
    pub accent: String,
    pub accent_rgb: (u8, u8, u8),
}

pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta == 0.0 {
        return (0.0, 0.0, l);
    }

    let s = if l > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let mut h = if max == rf {
        (gf - bf) / delta + (if gf < bf { 6.0 } else { 0.0 })
    } else if max == gf {
        (bf - rf) / delta + 2.0
    } else {
        (rf - gf) / delta + 4.0
    };
    h *= 60.0;

    (h, s, l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s <= 0.0 {
        let v = (l.clamp(0.0, 1.0) * 255.0).round() as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - (l * s)
    };
    let p = 2.0 * l - q;
    let hk = (h % 360.0 + 360.0) % 360.0 / 360.0;

    let to_c = |tc: f32| -> u8 {
        let t = tc.rem_euclid(1.0);
        let c = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (c.clamp(0.0, 1.0) * 255.0).round() as u8
    };

    (to_c(hk + 1.0 / 3.0), to_c(hk), to_c(hk - 1.0 / 3.0))
}

pub fn extract_palette(path: &Path) -> Palette {
    let default_palette = Palette {
        bg: "#111116".to_string(),
        border: "rgba(255, 255, 255, 0.08)".to_string(),
        accent: "#567ae6".to_string(),
        accent_rgb: (86, 122, 230),
    };

    // Fast-path: check if cached thumbnail exists first for ultra-fast decoding (<1ms)
    let home = env::var_os("HOME").map(PathBuf::from);
    let thumb_candidate = home.and_then(|h| {
        let stem = path.file_stem()?.to_string_lossy();
        let p = h.join(format!(".cache/wt-rust/thumbnails/{}.png", stem));
        if p.exists() {
            Some(p)
        } else {
            None
        }
    });

    let img = match thumb_candidate {
        Some(tp) => image::open(tp).or_else(|_| image::open(path)),
        None => image::open(path),
    };

    let img = match img {
        Ok(i) => i,
        Err(_) => return default_palette,
    };

    let small = img.thumbnail_exact(48, 48);
    let mut hue_bins: Vec<(f32, f32, f32, usize)> = vec![(0.0, 0.0, 0.0, 0); 16];

    for (_, _, pixel) in small.pixels() {
        let (h, s, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        if l > 0.20 && l < 0.85 && s > 0.22 {
            let bin = (((h / 22.5) as usize) % 16).min(15);
            hue_bins[bin].0 += s;
            hue_bins[bin].1 += l;
            hue_bins[bin].2 += h;
            hue_bins[bin].3 += 1;
        }
    }

    let mut best_bin = None;
    let mut best_score = -1.0f32;

    for (_i, (sum_s, sum_l, sum_h, count)) in hue_bins.iter().enumerate() {
        if *count > 0 {
            let avg_s = sum_s / (*count as f32);
            let avg_l = sum_l / (*count as f32);
            let avg_h = sum_h / (*count as f32);
            let score = (*count as f32) * avg_s * (1.0 - (avg_l - 0.55).abs());
            if score > best_score {
                best_score = score;
                best_bin = Some((avg_h, avg_s, avg_l));
            }
        }
    }

    if let Some((h, _, _)) = best_bin {
        let accent_rgb = hsl_to_rgb(h, 0.70, 0.62);
        let accent_hex = format!("#{:02x}{:02x}{:02x}", accent_rgb.0, accent_rgb.1, accent_rgb.2);

        let bg_rgb = hsl_to_rgb(h, 0.12, 0.07);
        let bg_hex = format!("#{:02x}{:02x}{:02x}", bg_rgb.0, bg_rgb.1, bg_rgb.2);

        Palette {
            bg: bg_hex,
            border: format!("rgba({}, {}, {}, 0.25)", accent_rgb.0, accent_rgb.1, accent_rgb.2),
            accent: accent_hex,
            accent_rgb,
        }
    } else {
        default_palette
    }
}

pub fn apply_theme(pal: &Palette, abs_wallpaper_path: &Path) {
    let abs_str = abs_wallpaper_path.to_string_lossy();
    if let Some(home) = env::var_os("HOME") {
        let home_path = PathBuf::from(home);

        // 1. Waybar CSS Theme
        let waybar_theme = home_path.join(".config/waybar/theme.css");
        let theme_css = format!(
            "/* Dynamic Palette extracted by wt-rust */\n\
             @define-color base-bg {};\n\
             @define-color base-border {};\n\
             @define-color text-color #e6e7ea;\n\
             @define-color text-dim #9aa2b6;\n\
             @define-color text-muted #545868;\n\
             @define-color accent-color {};\n\
             @define-color warning-color #e0a352;\n\
             @define-color critical-color #e05f52;\n",
            pal.bg, pal.border, pal.accent
        );
        let _ = fs::write(&waybar_theme, theme_css);

        // 2. Rofi Theme
        let rofi_theme = home_path.join(".config/rofi/theme.rasi");
        let rasi_content = format!(
            "* {{\n\
             \x20   bg: {}ee;\n\
             \x20   fg: #e6e7eaff;\n\
             \x20   accent: {}ff;\n\
             \x20   active-fg: #ffffffff;\n\
             \x20   border-col: {}ff;\n\n\
             \x20   background-color: transparent;\n\
             \x20   text-color: @fg;\n\
             \x20   font: \"0xProto Nerd Font 11\";\n\
             \x20   margin: 0;\n\
             \x20   padding: 0;\n\
             \x20   spacing: 0;\n\
             }}\n\n\
             window {{\n\
             \x20   width: 440px;\n\
             \x20   border: 2px;\n\
             \x20   border-radius: 12px;\n\
             \x20   border-color: @border-col;\n\
             \x20   background-color: @bg;\n\
             \x20   padding: 10px;\n\
             }}\n\n\
             mainbox {{\n\
             \x20   children: [ inputbar, message, listview, mode-switcher ];\n\
             \x20   spacing: 8px;\n\
             }}\n\n\
             inputbar {{\n\
             \x20   children: [ prompt, entry ];\n\
             \x20   background-color: @bg;\n\
             \x20   border-radius: 8px;\n\
             \x20   padding: 8px 12px;\n\
             }}\n\n\
             prompt {{\n\
             \x20   text-color: @accent;\n\
             \x20   margin: 0 8px 0 0;\n\
             }}\n\n\
             entry {{\n\
             \x20   text-color: @fg;\n\
             \x20   placeholder: \"Search...\";\n\
             \x20   placeholder-color: #545868;\n\
             }}\n\n\
             listview {{\n\
             \x20   lines: 8;\n\
             \x20   columns: 1;\n\
             \x20   scrollbar: false;\n\
             \x20   spacing: 4px;\n\
             }}\n\n\
             element {{\n\
             \x20   padding: 8px 12px;\n\
             \x20   border-radius: 8px;\n\
             }}\n\n\
             element selected {{\n\
             \x20   background-color: @accent;\n\
             \x20   text-color: @active-fg;\n\
             }}\n\n\
             element-icon {{\n\
             \x20   size: 20px;\n\
             \x20   margin: 0 8px 0 0;\n\
             }}\n\n\
             element-text selected {{\n\
             \x20   text-color: @active-fg;\n\
             }}\n",
            pal.bg, pal.accent, pal.accent
        );
        let _ = fs::write(&rofi_theme, rasi_content);

        // 3. Record current wallpaper
        let cache_current = home_path.join(".cache/wallpaper-picker/current");
        let _ = fs::create_dir_all(home_path.join(".cache/wallpaper-picker"));
        let _ = fs::write(cache_current, abs_str.as_bytes());
    }

    // 4. Reload Waybar CSS seamlessly without notification
    let _ = Command::new("pkill").args(["-SIGUSR2", "waybar"]).status();

    println!("Wallpaper applied: {}", abs_str);
    println!("Palette: Accent={}, BG={}", pal.accent, pal.bg);
}

pub fn handle_theme(args: &[String]) {
    if args.len() >= 2 && args[0] == "get" {
        let path = Path::new(&args[1]);
        let pal = extract_palette(path);
        if let Ok(json) = serde_json::to_string(&pal) {
            println!("{}", json);
        }
    } else {
        eprintln!("Usage: wt-rust theme get <image_path>");
    }
}
