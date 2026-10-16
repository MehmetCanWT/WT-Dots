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

pub fn load_current_palette() -> Palette {
    if let Ok(home) = env::var("HOME") {
        let cache_path = PathBuf::from(home).join(".cache/wt-rust/palette.json");
        if let Ok(content) = fs::read_to_string(&cache_path)
            && let Ok(pal) = serde_json::from_str::<Palette>(&content)
        {
            return pal;
        }
    }
    default_palette()
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

        Palette {
            bg: "#111116".to_string(),
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
                && path.parent() == Some(&h.join("Resimler/Wallpapers"))
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

fn generate_kitty_theme(pal: &Palette) -> String {
    let (ar, ag, ab) = pal.accent_rgb;
    let (h, s, l) = rgb_to_hsl(ar, ag, ab);

    let bg = &pal.bg;
    let fg = "#e6e7ea";
    let (br, bg_g, bb) = hsl_to_rgb(h, 0.15, 0.14);
    let color0 = format!("#{:02x}{:02x}{:02x}", br, bg_g, bb);
    let (mr, mg, mb) = hsl_to_rgb(h, 0.10, 0.48);
    let color8 = format!("#{:02x}{:02x}{:02x}", mr, mg, mb);

    let (r1, g1, b1) = hsl_to_rgb(355.0, 0.55, 0.62);
    let color1 = format!("#{:02x}{:02x}{:02x}", r1, g1, b1);
    let (r9, g9, b9) = hsl_to_rgb(355.0, 0.60, 0.72);
    let color9 = format!("#{:02x}{:02x}{:02x}", r9, g9, b9);

    let (r2, g2, b2) = hsl_to_rgb(130.0, 0.48, 0.58);
    let color2 = format!("#{:02x}{:02x}{:02x}", r2, g2, b2);
    let (r10, g10, b10) = hsl_to_rgb(130.0, 0.55, 0.68);
    let color10 = format!("#{:02x}{:02x}{:02x}", r10, g10, b10);

    let (r3, g3, b3) = hsl_to_rgb(45.0, 0.60, 0.62);
    let color3 = format!("#{:02x}{:02x}{:02x}", r3, g3, b3);
    let (r11, g11, b11) = hsl_to_rgb(45.0, 0.68, 0.74);
    let color11 = format!("#{:02x}{:02x}{:02x}", r11, g11, b11);

    let color4 = &pal.accent;
    let (r12, g12, b12) = hsl_to_rgb(h, s, (l + 0.10).min(0.85));
    let color12 = format!("#{:02x}{:02x}{:02x}", r12, g12, b12);

    let (r5, g5, b5) = hsl_to_rgb((h + 280.0).rem_euclid(360.0), 0.55, 0.66);
    let color5 = format!("#{:02x}{:02x}{:02x}", r5, g5, b5);
    let (r13, g13, b13) = hsl_to_rgb((h + 280.0).rem_euclid(360.0), 0.60, 0.76);
    let color13 = format!("#{:02x}{:02x}{:02x}", r13, g13, b13);

    let (r6, g6, b6) = hsl_to_rgb((h + 175.0).rem_euclid(360.0), 0.50, 0.62);
    let color6 = format!("#{:02x}{:02x}{:02x}", r6, g6, b6);
    let (r14, g14, b14) = hsl_to_rgb((h + 175.0).rem_euclid(360.0), 0.55, 0.72);
    let color14 = format!("#{:02x}{:02x}{:02x}", r14, g14, b14);

    format!(
        "# Dynamic palette generated by wt-rust\n\
         foreground            {fg}\n\
         background            {bg}\n\
         selection_foreground  {fg}\n\
         selection_background  {color4}\n\
         cursor                {color4}\n\
         cursor_text_color     {bg}\n\
         url_color             {color6}\n\n\
         active_border_color   {color4}\n\
         inactive_border_color {color0}\n\
         bell_border_color     {color1}\n\n\
         active_tab_foreground   {fg}\n\
         active_tab_background   {color4}\n\
         inactive_tab_foreground {color8}\n\
         inactive_tab_background {bg}\n\n\
         wayland_titlebar_color  {bg}\n\n\
         color0  {color0}\n\
         color8  {color8}\n\
         color1  {color1}\n\
         color9  {color9}\n\
         color2  {color2}\n\
         color10 {color10}\n\
         color3  {color3}\n\
         color11 {color11}\n\
         color4  {color4}\n\
         color12 {color12}\n\
         color5  {color5}\n\
         color13 {color13}\n\
         color6  {color6}\n\
         color14 {color14}\n\
         color7  {fg}\n\
         color15 #ffffff\n"
    )
}

pub fn apply_theme(pal: &Palette, abs_wallpaper_path: &Path) {
    let abs_str = abs_wallpaper_path.to_string_lossy();
    if let Some(home) = env::var_os("HOME") {
        let home_path = PathBuf::from(home);

        // 1. Waybar CSS Theme (config + repository sync)
        let waybar_theme = home_path.join(".config/waybar/theme.css");
        let theme_css = format!(
            "/* Dynamic Palette extracted by wt-rust */\n\
             @define-color base-bg {};\n\
             @define-color base-border {};\n\
             @define-color text-color {};\n\
             @define-color text-dim {};\n\
             @define-color text-muted {};\n\
             @define-color accent-color {};\n\
             @define-color accent-secondary {};\n\
             @define-color warning-color #e0a352;\n\
             @define-color critical-color #e05f52;\n",
            pal.bg,
            pal.border,
            pal.accent,
            pal.accent,
            pal.text_muted,
            pal.accent,
            pal.accent_secondary
        );
        let _ = fs::write(&waybar_theme, &theme_css);
        let repo_waybar = home_path.join("Code/WT-Dots/waybar/theme.css");
        if repo_waybar.exists() {
            let _ = fs::write(repo_waybar, &theme_css);
        }

        // Fast-access palette cache for waybar subcommands
        let cache_dir = home_path.join(".cache/wt-rust");
        let _ = fs::create_dir_all(&cache_dir);
        if let Ok(json) = serde_json::to_string(pal) {
            let _ = fs::write(cache_dir.join("palette.json"), json);
        }

        // 2. Rofi Theme (config + repository sync)
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
             message {{\n\
             \x20   padding: 6px 10px;\n\
             \x20   background-color: transparent;\n\
             \x20   border: 0px;\n\
             }}\n\n\
             textbox {{\n\
             \x20   text-color: @active-fg;\n\
             }}\n\n\
             mode-switcher {{\n\
             \x20   spacing: 8px;\n\
             }}\n\n\
             button {{\n\
             \x20   padding: 4px 8px;\n\
             \x20   border-radius: 6px;\n\
             \x20   background-color: transparent;\n\
             \x20   text-color: @fg;\n\
             }}\n\n\
             button selected {{\n\
             \x20   background-color: @accent;\n\
             \x20   text-color: @active-fg;\n\
             }}\n\n\
             inputbar {{\n\
             \x20   children: [ entry ];\n\
             \x20   padding: 6px 10px;\n\
             \x20   border: 1px;\n\
             \x20   border-radius: 6px;\n\
             \x20   border-color: @border-col;\n\
             \x20   background-color: transparent;\n\
             }}\n\n\
             entry {{\n\
             \x20   placeholder: \"\";\n\
             \x20   text-color: @fg;\n\
             }}\n\n\
             listview {{\n\
             \x20   lines: 6;\n\
             \x20   columns: 1;\n\
             \x20   fixed-height: false;\n\
             \x20   dynamic: true;\n\
             \x20   require-input: false;\n\
             \x20   spacing: 4px;\n\
             }}\n\n\
             element {{\n\
             \x20   padding: 6px 10px;\n\
             \x20   border-radius: 6px;\n\
             \x20   spacing: 10px;\n\
             }}\n\n\
             element selected {{\n\
             \x20   background-color: @accent;\n\
             \x20   text-color: @active-fg;\n\
             }}\n\n\
             element-icon {{\n\
             \x20   size: 20px;\n\
             }}\n\n\
             element-text {{\n\
             \x20   vertical-align: 0.5;\n\
             \x20   text-color: inherit;\n\
             }}\n",
            pal.bg, pal.accent, pal.accent
        );
        let _ = fs::write(&rofi_theme, &rasi_content);
        let repo_rofi = home_path.join("Code/WT-Dots/rofi/theme.rasi");
        if repo_rofi.exists() {
            let _ = fs::write(repo_rofi, &rasi_content);
        }

        // 3. GTK 3.0 & GTK 4.0 User CSS
        let gtk3_file = home_path.join(".config/gtk-3.0/gtk.css");
        if gtk3_file.exists() {
            let gtk3_content = format!(
                "@define-color accent_color {};\n\
                 @define-color accent_bg_color {};\n\
                 @define-color accent_fg_color #ffffff;\n\
                 @define-color theme_selected_bg_color {};\n\
                 @define-color theme_selected_fg_color #ffffff;\n\
                 @import 'colors.css';\n",
                pal.accent, pal.accent, pal.accent
            );
            let _ = fs::write(gtk3_file, gtk3_content);
        }

        let gtk4_file = home_path.join(".config/gtk-4.0/gtk.css");
        if gtk4_file.exists() {
            let gtk4_content = format!(
                "@define-color accent_color {};\n\
                 @define-color accent_bg_color {};\n\
                 @define-color accent_fg_color #ffffff;\n\
                 @define-color window_bg_color {};\n\
                 @define-color window_fg_color #e6e7ea;\n\
                 @define-color view_bg_color {};\n\
                 @define-color view_fg_color #e6e7ea;\n\
                 @define-color headerbar_bg_color {};\n\
                 @define-color headerbar_fg_color #e6e7ea;\n\
                 @define-color card_bg_color {};\n\
                 @define-color card_fg_color #e6e7ea;\n\
                 @define-color popover_bg_color {};\n\
                 @define-color popover_fg_color #e6e7ea;\n\n\
                 .navigation-sidebar > row:selected {{\n\
                 \x20   background-color: @accent_bg_color;\n\
                 \x20   color: @accent_fg_color;\n\
                 \x20   border-radius: 6px;\n\
                 }}\n\
                 @import 'colors.css';\n",
                pal.accent, pal.accent, pal.bg, pal.bg, pal.bg, pal.bg, pal.bg
            );
            let _ = fs::write(gtk4_file, gtk4_content);
        }

        // 4. Kitty Terminal Theme
        let kitty_dir = home_path.join(".config/kitty");
        if kitty_dir.exists() {
            let kitty_theme_content = generate_kitty_theme(pal);
            let _ = fs::write(kitty_dir.join("current-theme.conf"), kitty_theme_content);
            let _ = Command::new("pkill").args(["-SIGUSR1", "kitty"]).status();
        }

        // 4. Fastfetch Logo Color Sync
        let fastfetch_color = home_path.join(".config/fastfetch/logos/current.png.color");
        if fastfetch_color.parent().is_some_and(|p| p.exists()) {
            let _ = fs::write(fastfetch_color, &pal.accent);
        }

        // 5. KDE Plasma 6 System Accent Color (cascades to GTK 3 & GTK 4 via kde-gtk-config)
        let _ = Command::new("plasma-apply-colorscheme")
            .args(["-a", &pal.accent, "BreezeDark"])
            .status();

        let rgb_str = format!(
            "{},{},{}",
            pal.accent_rgb.0, pal.accent_rgb.1, pal.accent_rgb.2
        );
        let _ = Command::new("kwriteconfig6")
            .args([
                "--file",
                "kdeglobals",
                "--group",
                "General",
                "--key",
                "AccentColor",
                &rgb_str,
            ])
            .status();
        let _ = Command::new("kwriteconfig6")
            .args([
                "--file",
                "kdeglobals",
                "--group",
                "General",
                "--key",
                "LastUsedCustomAccentColor",
                &rgb_str,
            ])
            .status();

        let (r, g, b) = pal.accent_rgb;
        let qcolor_uint: u32 = 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        let _ = Command::new("qdbus-qt6")
            .args([
                "org.kde.plasmashell.accentColor",
                "/AccentColor",
                "org.kde.plasmashell.accentColor.setAccentColor",
                &qcolor_uint.to_string(),
            ])
            .status();

        let _ = Command::new("qdbus-qt6")
            .args(["org.kde.KWin", "/KWin", "reconfigure"])
            .status();

        // 6. Record current wallpaper
        let cache_dir = home_path.join(".cache/wt-rust");
        let _ = fs::create_dir_all(&cache_dir);
        let _ = fs::write(cache_dir.join("current"), abs_str.as_bytes());
    }

    // 7. Reload Waybar CSS seamlessly without notification
    let _ = Command::new("pkill").args(["-SIGUSR2", "waybar"]).status();

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
    fn test_generate_kitty_theme() {
        let pal = default_palette();
        let kitty = generate_kitty_theme(&pal);
        assert!(kitty.contains("foreground            #e6e7ea"));
        assert!(kitty.contains(&format!("active_border_color   {}", pal.accent)));
        assert!(kitty.contains("color15 #ffffff"));
    }
}
