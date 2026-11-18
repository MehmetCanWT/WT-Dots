use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn default_accent_secondary() -> String {
    "#8ab4f8".to_string()
}
fn default_text_color() -> String {
    "#e6e7ea".to_string()
}
fn default_text_dim() -> String {
    "#9aa2b6".to_string()
}
fn default_text_muted() -> String {
    "#545868".to_string()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub bg: String,
    pub border: String,
    pub accent: String,
    pub accent_rgb: (u8, u8, u8),
    #[serde(default = "default_accent_secondary")]
    pub accent_secondary: String,
    #[serde(default = "default_text_color")]
    pub text_color: String,
    #[serde(default = "default_text_dim")]
    pub text_dim: String,
    #[serde(default = "default_text_muted")]
    pub text_muted: String,
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

pub fn default_palette() -> Palette {
    Palette {
        bg: "#111116".to_string(),
        border: "rgba(86, 122, 230, 0.22)".to_string(),
        accent: "#567ae6".to_string(),
        accent_rgb: (86, 122, 230),
        accent_secondary: "#8ab4f8".to_string(),
        text_color: "#e6e7ea".to_string(),
        text_dim: "#9aa2b6".to_string(),
        text_muted: "#545868".to_string(),
    }
}

pub fn extract_palette_from_image(img: &image::DynamicImage) -> Palette {
    let small = img.thumbnail_exact(48, 48);
    let mut hue_bins: Vec<(f32, f32, f32, usize)> = vec![(0.0, 0.0, 0.0, 0); 16];

    for (_, _, pixel) in small.pixels() {
        let (h, s, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        if l > 0.20 && l < 0.85 && s > 0.22 {
            let bin = (((h + 11.25).rem_euclid(360.0) / 22.5) as usize).min(15);
            let h_unwrapped = if h >= 348.75 { h - 360.0 } else { h };
            hue_bins[bin].0 += s;
            hue_bins[bin].1 += l;
            hue_bins[bin].2 += h_unwrapped;
            hue_bins[bin].3 += 1;
        }
    }

    let mut best_bin = None;
    let mut best_score = -1.0f32;

    for (sum_s, sum_l, sum_h, count) in &hue_bins {
        if *count > 0 {
            let c = *count as f32;
            let avg_s = sum_s / c;
            let avg_l = sum_l / c;
            let avg_h = (sum_h / c).rem_euclid(360.0);
            let score = c * avg_s * (1.0 - (avg_l - 0.55).abs());
            if score > best_score {
                best_score = score;
                best_bin = Some((avg_h, avg_s, avg_l));
            }
        }
    }

    if let Some((h, _, _)) = best_bin {
        let accent_rgb = hsl_to_rgb(h, 0.72, 0.62);
        let accent_hex = format!(
            "#{:02x}{:02x}{:02x}",
            accent_rgb.0, accent_rgb.1, accent_rgb.2
        );

        let accent_sec_rgb = hsl_to_rgb((h + 35.0).rem_euclid(360.0), 0.65, 0.66);
        let accent_sec_hex = format!(
            "#{:02x}{:02x}{:02x}",
            accent_sec_rgb.0, accent_sec_rgb.1, accent_sec_rgb.2
        );

        let text_rgb = hsl_to_rgb(h, 0.18, 0.94);
        let text_hex = format!("#{:02x}{:02x}{:02x}", text_rgb.0, text_rgb.1, text_rgb.2);

        let dim_rgb = hsl_to_rgb(h, 0.22, 0.74);
        let dim_hex = format!("#{:02x}{:02x}{:02x}", dim_rgb.0, dim_rgb.1, dim_rgb.2);

        let muted_rgb = hsl_to_rgb(h, 0.18, 0.46);
        let muted_hex = format!("#{:02x}{:02x}{:02x}", muted_rgb.0, muted_rgb.1, muted_rgb.2);

        let bg_rgb = hsl_to_rgb(h, 0.16, 0.08);
        let bg_hex = format!("#{:02x}{:02x}{:02x}", bg_rgb.0, bg_rgb.1, bg_rgb.2);

        Palette {
            bg: bg_hex,
            border: format!(
                "rgba({}, {}, {}, 0.22)",
                accent_rgb.0, accent_rgb.1, accent_rgb.2
            ),
            accent: accent_hex,
            accent_rgb,
            accent_secondary: accent_sec_hex,
            text_color: text_hex,
            text_dim: dim_hex,
            text_muted: muted_hex,
        }
    } else {
        default_palette()
    }
}

pub fn wallpapers_dir(home: &Path) -> PathBuf {
    let tr = home.join("Resimler/Wallpapers");
    if tr.exists() {
        return tr;
    }
    let en = home.join("Pictures/Wallpapers");
    if en.exists() {
        return en;
    }
    tr
}

pub fn extract_palette(path: &Path) -> Palette {
    // Fast-path: check if cached thumbnail exists first for ultra-fast decoding (<1ms)
    let home = env::var_os("HOME").map(PathBuf::from);
    let thumb_path = home.as_ref().and_then(|h| {
        let name = path.file_name()?.to_string_lossy();
        Some(h.join(format!(".cache/wt-rust/thumbnails/{}.png", name)))
    });

    let img = match thumb_path.as_ref().filter(|p| p.exists()) {
        Some(tp) => image::open(tp).or_else(|_| image::open(path)),
        None => {
            let opened = image::open(path);
            if let (Ok(decoded), Some(h), Some(tp)) = (&opened, &home, &thumb_path)
                && (path.parent() == Some(&h.join("Resimler/Wallpapers"))
                    || path.parent() == Some(&h.join("Pictures/Wallpapers")))
            {
                if let Some(parent) = tp.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = decoded.thumbnail_exact(320, 180).save(tp);
            }
            opened
        }
    };

    let Ok(img) = img else {
        return default_palette();
    };

    extract_palette_from_image(&img)
}

pub(crate) fn parse_hex_rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 {
        return (17, 19, 24);
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(17);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(19);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(24);
    (r, g, b)
}

struct TerminalColors {
    bg: String,
    fg: String,
    accent: String,
    surface: String,
    muted: String,
    c1: String,
    c9: String,
    c2: String,
    c10: String,
    c3: String,
    c11: String,
    c12: String,
    c5: String,
    c13: String,
    c6: String,
    c14: String,
}

impl TerminalColors {
    fn from_palette(pal: &Palette) -> Self {
        let (ar, ag, ab) = pal.accent_rgb;
        let (h, s, l) = rgb_to_hsl(ar, ag, ab);

        let (br, bg_g, bb) = hsl_to_rgb(h, 0.15, 0.14);
        let surface = format!("#{:02x}{:02x}{:02x}", br, bg_g, bb);
        let (mr, mg, mb) = hsl_to_rgb(h, 0.10, 0.48);
        let muted = format!("#{:02x}{:02x}{:02x}", mr, mg, mb);

        let to_hex = |rgb: (u8, u8, u8)| format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2);

        Self {
            bg: pal.bg.clone(),
            fg: "#e6e7ea".to_string(),
            accent: pal.accent.clone(),
            surface,
            muted,
            c1: to_hex(hsl_to_rgb(355.0, 0.55, 0.62)),
            c9: to_hex(hsl_to_rgb(355.0, 0.60, 0.72)),
            c2: to_hex(hsl_to_rgb(130.0, 0.48, 0.58)),
            c10: to_hex(hsl_to_rgb(130.0, 0.55, 0.68)),
            c3: to_hex(hsl_to_rgb(45.0, 0.60, 0.62)),
            c11: to_hex(hsl_to_rgb(45.0, 0.68, 0.74)),
            c12: to_hex(hsl_to_rgb(h, s, (l + 0.10).min(0.85))),
            c5: to_hex(hsl_to_rgb((h + 280.0).rem_euclid(360.0), 0.55, 0.66)),
            c13: to_hex(hsl_to_rgb((h + 280.0).rem_euclid(360.0), 0.60, 0.76)),
            c6: to_hex(hsl_to_rgb((h + 175.0).rem_euclid(360.0), 0.50, 0.62)),
            c14: to_hex(hsl_to_rgb((h + 175.0).rem_euclid(360.0), 0.55, 0.72)),
        }
    }
}

fn generate_ghostty_theme(pal: &Palette) -> String {
    let tc = TerminalColors::from_palette(pal);
    format!(
        "# Dynamic palette generated by wt-rust\n\
         background = {bg}\n\
         foreground = {fg}\n\
         cursor-color = {c4}\n\
         cursor-text = {bg}\n\
         selection-background = {c4}\n\
         selection-foreground = {fg}\n\
         palette = 0={c0}\n\
         palette = 1={c1}\n\
         palette = 2={c2}\n\
         palette = 3={c3}\n\
         palette = 4={c4}\n\
         palette = 5={c5}\n\
         palette = 6={c6}\n\
         palette = 7={fg}\n\
         palette = 8={c8}\n\
         palette = 9={c9}\n\
         palette = 10={c10}\n\
         palette = 11={c11}\n\
         palette = 12={c12}\n\
         palette = 13={c13}\n\
         palette = 14={c14}\n\
         palette = 15=#ffffff\n",
        bg = tc.bg,
        fg = tc.fg,
        c0 = tc.surface,
        c8 = tc.muted,
        c1 = tc.c1,
        c9 = tc.c9,
        c2 = tc.c2,
        c10 = tc.c10,
        c3 = tc.c3,
        c11 = tc.c11,
        c4 = tc.accent,
        c12 = tc.c12,
        c5 = tc.c5,
        c13 = tc.c13,
        c6 = tc.c6,
        c14 = tc.c14,
    )
}

fn generate_waybar_colors(pal: &Palette) -> String {
    let (br, bg_g, bb) = parse_hex_rgb(&pal.bg);
    let (ar, ag, ab) = pal.accent_rgb;
    let (h, _, _) = rgb_to_hsl(ar, ag, ab);
    let (sr, sg, sb) = hsl_to_rgb(h, 0.16, 0.16);
    format!(
        "/* Dynamic palette generated by wt-rust */\n\
         @define-color wt_bg rgba({br}, {bg_g}, {bb}, 0.90);\n\
         @define-color wt_surface rgba({sr}, {sg}, {sb}, 0.72);\n\
         @define-color wt_border rgba({ar}, {ag}, {ab}, 0.28);\n\
         @define-color wt_accent {accent};\n\
         @define-color wt_accent_bg rgba({ar}, {ag}, {ab}, 0.24);\n\
         @define-color wt_accent_hover rgba({ar}, {ag}, {ab}, 0.36);\n\
         @define-color wt_accent_sec {sec};\n\
         @define-color wt_text {text};\n\
         @define-color wt_text_dim {dim};\n",
        accent = pal.accent,
        sec = pal.accent_secondary,
        text = pal.text_color,
        dim = pal.text_dim,
    )
}

fn generate_rofi_colors(pal: &Palette) -> String {
    let (ar, ag, ab) = pal.accent_rgb;
    let (h, _, _) = rgb_to_hsl(ar, ag, ab);
    let (sr, sg, sb) = hsl_to_rgb(h, 0.24, 0.24);
    let bg = pal.bg.trim_start_matches('#');
    let text = pal.text_color.trim_start_matches('#');
    let dim = pal.text_dim.trim_start_matches('#');
    let accent = pal.accent.trim_start_matches('#');
    let sel = format!("{sr:02x}{sg:02x}{sb:02x}");
    format!(
        "/* Dynamic colors generated by wt-rust */\n\
         * {{\n\
             main-bg:   #{bg}ee;\n\
             main-fg:   #{text}ff;\n\
             main-br:   #{accent}99;\n\
             select-bg: #{sel}dd;\n\
             select-fg: #{text}ff;\n\
             dim-fg:    #{dim}ff;\n\
         }}\n"
    )
}

fn generate_hypr_theme(pal: &Palette) -> String {
    let hex_accent = pal.accent.trim_start_matches('#');
    let hex_sec = pal.accent_secondary.trim_start_matches('#');
    let hex_muted = pal.text_muted.trim_start_matches('#');
    format!(
        "-- Dynamic border theme generated by wt-rust\n\
         hl.config({{\n\
             general = {{\n\
                 col = {{\n\
                     active_border   = {{ colors = {{ \"rgba({hex_accent}ee)\", \"rgba({hex_sec}ee)\" }}, angle = 45 }},\n\
                     inactive_border = \"rgba({hex_muted}88)\",\n\
                 }},\n\
             }},\n\
         }})\n"
    )
}

fn generate_hyprlock_colors(pal: &Palette) -> String {
    let (br, bg_g, bb) = parse_hex_rgb(&pal.bg);
    let (tr, tg, tb) = parse_hex_rgb(&pal.text_color);
    let (dr, dg, db) = parse_hex_rgb(&pal.text_dim);
    let hex_accent = pal.accent.trim_start_matches('#');
    let hex_sec = pal.accent_secondary.trim_start_matches('#');
    format!(
        "# Dynamic colors generated by wt-rust\n\
         $wt_inner = rgba({br}, {bg_g}, {bb}, 0.78)\n\
         $wt_outer = rgba({hex_accent}ee) rgba({hex_sec}ee) 45deg\n\
         $wt_font  = rgb({tr}, {tg}, {tb})\n\
         $wt_dim   = rgb({dr}, {dg}, {db})\n"
    )
}

pub fn set_hyprland_wallpaper(abs_str: &str) {
    if let Some(home) = env::var_os("HOME") {
        let conf_path = PathBuf::from(home).join(".config/hypr/hyprpaper.conf");
        let content = format!(
            "# Hyprpaper 0.8+ Block Syntax Configuration\n\nwallpaper {{\n    monitor =\n    path = {}\n    fit_mode = cover\n}}\n",
            abs_str
        );
        let _ = fs::write(conf_path, content);
    }

    // Hyprpaper 0.8+ requires explicit monitor names when updating via IPC.
    let mut applied = false;
    if let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output()
        && let Ok(monitors) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
    {
        for m in monitors {
            if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                let arg = format!("{},{},cover", name, abs_str);
                if Command::new("hyprctl")
                    .args(["hyprpaper", "wallpaper", &arg])
                    .status()
                    .is_ok_and(|s| s.success())
                {
                    applied = true;
                }
            }
        }
    }
    if !applied {
        let arg = format!(",{},cover", abs_str);
        let _ = Command::new("hyprctl")
            .args(["hyprpaper", "wallpaper", &arg])
            .status();
    }
}

pub fn apply_theme(pal: &Palette, abs_wallpaper_path: &Path) {
    let abs_str = abs_wallpaper_path.to_string_lossy();
    if let Some(home) = env::var_os("HOME") {
        let home_path = PathBuf::from(home);

        // Fast-access palette & current wallpaper cache
        let cache_dir = home_path.join(".cache/wt-rust");
        let _ = fs::create_dir_all(&cache_dir);
        if let Ok(json) = serde_json::to_string(pal) {
            let _ = fs::write(cache_dir.join("palette.json"), json);
        }
        let _ = fs::write(cache_dir.join("current"), abs_str.as_bytes());

        // 2. GTK 3.0 & GTK 4.0 User CSS
        let gtk3_dir = home_path.join(".config/gtk-3.0");
        let _ = fs::create_dir_all(&gtk3_dir);
        let gtk3_file = gtk3_dir.join("gtk.css");
        let hide_controls_css = "\nheaderbar windowcontrols,\nheaderbar .titlebutton,\n.titlebar .titlebutton,\nwindowcontrols {\n    min-width: 0;\n    min-height: 0;\n    padding: 0;\n    margin: 0;\n    border: none;\n    opacity: 0;\n}\n";
        let gtk3_content = format!(
            "@define-color accent_color {accent};\n\
             @define-color accent_bg_color {accent};\n\
             @define-color accent_fg_color #ffffff;\n\
             @define-color theme_selected_bg_color {accent};\n\
             @define-color theme_selected_fg_color #ffffff;\n\
             @define-color window_bg_color {bg};\n\
             @define-color window_fg_color #eff0f4;\n\
             @define-color view_bg_color {bg};\n\
             @define-color view_fg_color #eff0f4;\n{hide}",
            accent = pal.accent,
            bg = pal.bg,
            hide = hide_controls_css
        );
        let _ = fs::write(gtk3_file, gtk3_content);

        let gtk4_dir = home_path.join(".config/gtk-4.0");
        let _ = fs::create_dir_all(&gtk4_dir);
        let gtk4_file = gtk4_dir.join("gtk.css");
        let gtk4_content = format!(
            "@define-color accent_color {accent};\n\
             @define-color accent_bg_color {accent};\n\
             @define-color accent_fg_color #ffffff;\n\
             @define-color window_bg_color {bg};\n\
             @define-color window_fg_color #eff0f4;\n\
             @define-color view_bg_color {bg};\n\
             @define-color view_fg_color #eff0f4;\n\
             @define-color headerbar_bg_color {bg};\n\
             @define-color headerbar_fg_color #eff0f4;\n\
             @define-color card_bg_color {bg};\n\
             @define-color card_fg_color #eff0f4;\n\
             @define-color popover_bg_color {bg};\n\
             @define-color popover_fg_color #eff0f4;\n\n\
             .navigation-sidebar > row:selected {{\n\
                 background-color: @accent_bg_color;\n\
                 color: @accent_fg_color;\n\
                 border-radius: 6px;\n\
             }}\n{hide}",
            accent = pal.accent,
            bg = pal.bg,
            hide = hide_controls_css
        );
        let _ = fs::write(gtk4_file, gtk4_content);

        // 4. Ghostty Terminal Theme
        let ghostty_dir = home_path.join(".config/ghostty");
        if ghostty_dir.exists() {
            let auto_dir = ghostty_dir.join("auto");
            let _ = fs::create_dir_all(&auto_dir);
            let _ = fs::write(auto_dir.join("theme.ghostty"), generate_ghostty_theme(pal));
            let _ = Command::new("pkill").args(["-SIGUSR2", "ghostty"]).status();
        }

        // 5. Set Hyprland Wallpaper via hyprpaper & persist hyprpaper.conf
        set_hyprland_wallpaper(&abs_str);

        // 6. Persist and live-apply Hyprland borders & Hyprlock colors
        let hypr_cfg_dir = home_path.join(".config/hypr/configs");
        let _ = fs::create_dir_all(&hypr_cfg_dir);
        let _ = fs::write(hypr_cfg_dir.join("theme.lua"), generate_hypr_theme(pal));
        let _ = fs::write(
            hypr_cfg_dir.join("hyprlock-colors.conf"),
            generate_hyprlock_colors(pal),
        );

        let _ = Command::new("hyprctl")
            .args(["reload", "config-only"])
            .status();

        // 7. Sync Waybar CSS colors and hot-reload Waybar
        let waybar_dir = home_path.join(".config/waybar");
        if waybar_dir.exists() {
            let _ = fs::write(waybar_dir.join("colors.css"), generate_waybar_colors(pal));
            let _ = Command::new("pkill").args(["-SIGUSR2", "waybar"]).status();
        }

        // 8. Sync Rofi colors & fast pre-scaled wallpaper thumbnail (~5ms load in Rofi)
        let rofi_thumb = cache_dir.join("rofi_wall.jpg");
        if let Ok(img) = image::open(abs_wallpaper_path) {
            let thumb = img.resize_to_fill(480, 270, image::imageops::FilterType::Triangle);
            let _ = thumb.to_rgb8().save(&rofi_thumb);
        }
        let rofi_dir = home_path.join(".config/rofi");
        if rofi_dir.exists() {
            let _ = fs::write(rofi_dir.join("theme.rasi"), generate_rofi_colors(pal));
        }

        // 9. Sync SwayNC notification colors and reload CSS
        let swaync_dir = home_path.join(".config/swaync");
        if swaync_dir.exists() {
            let _ = fs::write(swaync_dir.join("colors.css"), generate_waybar_colors(pal));
            let _ = Command::new("swaync-client").arg("-rs").status();
        }
    }

    println!("Wallpaper applied: {}", abs_str);
    println!("Palette: Accent={}, BG={}", pal.accent, pal.bg);
}

pub fn handle_theme(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("get") => {
            if let Some(path_str) = args.get(1) {
                let path = Path::new(path_str);
                let pal = extract_palette(path);
                if let Ok(json) = serde_json::to_string(&pal) {
                    println!("{}", json);
                }
            } else {
                eprintln!("Usage: wt-rust theme get <image_path>");
            }
        }
        Some("apply") => {
            let path_buf = if let Some(path_str) = args.get(1) {
                PathBuf::from(path_str)
            } else {
                let home = env::var_os("HOME").map(PathBuf::from);
                let cur = home.map(|h| h.join(".cache/wt-rust/current"));
                if let Some(cur_file) = cur.filter(|p| p.exists()) {
                    let s = fs::read_to_string(cur_file).unwrap_or_default();
                    PathBuf::from(s.trim())
                } else {
                    eprintln!("No wallpaper specified and no current wallpaper found in cache.");
                    return;
                }
            };
            if !path_buf.exists() {
                eprintln!("Image not found: {:?}", path_buf);
                return;
            }
            let pal = extract_palette(&path_buf);
            apply_theme(&pal, &path_buf);
        }
        _ => {
            eprintln!("Usage: wt-rust theme [get <image_path> | apply [image_path]]");
        }
    }
}

pub fn get_current_palette() -> Palette {
    if let Some(home) = env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        let pal_file = home_path.join(".cache/wt-rust/palette.json");
        if let Ok(content) = fs::read_to_string(&pal_file)
            && let Ok(pal) = serde_json::from_str::<Palette>(&content)
        {
            return pal;
        }

        let cur_file = home_path.join(".cache/wt-rust/current");
        if let Ok(s) = fs::read_to_string(&cur_file) {
            let wp_path = PathBuf::from(s.trim());
            if wp_path.exists() {
                let pal = extract_palette(&wp_path);
                let _ = fs::write(pal_file, serde_json::to_string(&pal).unwrap_or_default());
                return pal;
            }
        }
    }
    default_palette()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsl_roundtrip_and_wrap() {
        let (h, s, l) = rgb_to_hsl(220, 30, 46);
        assert!(h > 348.0 && h < 360.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r as i16 - 220).abs() <= 1);
        assert!((g as i16 - 30).abs() <= 1);
        assert!((b as i16 - 46).abs() <= 1);
    }

    #[test]
    fn test_extract_palette_red_wrap_around() {
        let tmp = env::temp_dir().join("wt_test_red_wrap.png");
        let mut img = image::RgbImage::new(48, 48);
        for (x, _, p) in img.enumerate_pixels_mut() {
            // Half ~355° crimson red, half ~5° scarlet red
            *p = if x < 24 {
                image::Rgb([220, 30, 46])
            } else {
                image::Rgb([220, 46, 30])
            };
        }
        img.save(&tmp).unwrap();
        let pal = extract_palette(&tmp);
        let _ = fs::remove_file(&tmp);
        // Dominant accent must be red (R >> G and R >> B), not cyan
        assert!(
            pal.accent_rgb.0 > 200,
            "Expected red accent, got {:?}",
            pal.accent_rgb
        );
        assert!(
            pal.accent_rgb.1 < 120,
            "Expected low green, got {:?}",
            pal.accent_rgb
        );
        assert!(
            pal.accent_rgb.2 < 120,
            "Expected low blue, got {:?}",
            pal.accent_rgb
        );
    }

    #[test]
    fn test_generate_hyprland_component_themes() {
        let pal = default_palette();
        let ghostty = generate_ghostty_theme(&pal);
        assert!(ghostty.contains(&format!("background = {}", pal.bg)));
        assert!(ghostty.contains(&format!("cursor-color = {}", pal.accent)));

        let waybar = generate_waybar_colors(&pal);
        assert!(waybar.contains(&format!("@define-color wt_accent {};", pal.accent)));
        assert!(waybar.contains("@define-color wt_bg rgba("));

        let hypr = generate_hypr_theme(&pal);
        assert!(hypr.contains("active_border"));
        assert!(hypr.contains("inactive_border"));

        let lock = generate_hyprlock_colors(&pal);
        assert!(lock.contains("$wt_inner = rgba("));
        assert!(lock.contains("$wt_outer = rgba("));
    }
}
