use eframe::egui;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::theme::{apply_theme, extract_palette, Palette};

pub struct WallpaperItem {
    pub path: PathBuf,
    pub thumb_path: PathBuf,
    pub preview_path: PathBuf,
}

struct BgResult {
    path: PathBuf,
    img: egui::ColorImage,
    palette: Palette,
}

pub struct WallpaperApp {
    items: Vec<WallpaperItem>,
    current_index: usize,
    bg_textures: HashMap<PathBuf, egui::TextureHandle>,
    thumb_textures: HashMap<PathBuf, egui::TextureHandle>,
    palette_cache: HashMap<PathBuf, Palette>,
    palette: Palette,
    scroll_transition: f32,
    scroll_accum: f32,
    should_close: bool,
    committed: bool,
    bg_rx: Option<std::sync::mpsc::Receiver<BgResult>>,
    bg_loading_path: Option<PathBuf>,
}

fn load_texture_from_file(
    ctx: &egui::Context,
    name: &str,
    path: &Path,
    max_dims: Option<(u32, u32)>,
) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?;
    let img = if let Some((w, h)) = max_dims {
        img.thumbnail_exact(w, h)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(name, color_img, egui::TextureOptions::LINEAR))
}

impl WallpaperApp {
    pub fn new(items: Vec<WallpaperItem>, initial_index: usize) -> Self {
        let initial_palette = Palette {
            bg: "#111116".to_string(),
            border: "rgba(255, 255, 255, 0.08)".to_string(),
            accent: "#567ae6".to_string(),
            accent_rgb: (86, 122, 230),
        };

        Self {
            items,
            current_index: initial_index,
            bg_textures: HashMap::new(),
            thumb_textures: HashMap::new(),
            palette_cache: HashMap::new(),
            palette: initial_palette,
            scroll_transition: 0.0,
            scroll_accum: 0.0,
            should_close: false,
            committed: false,
            bg_rx: None,
            bg_loading_path: None,
        }
    }

    fn navigate(&mut self, ctx: &egui::Context, delta: isize) {
        let n = self.items.len();
        if n == 0 {
            return;
        }

        self.current_index = (self.current_index as isize + delta).rem_euclid(n as isize) as usize;
        self.scroll_transition += delta as f32;
        self.on_selection_changed(ctx);
    }

    fn on_selection_changed(&mut self, ctx: &egui::Context) {
        if self.items.is_empty() {
            return;
        }

        let path = self.items[self.current_index].path.clone();
        let preview_path = self.items[self.current_index].preview_path.clone();

        // Check if palette is already known
        if let Some(pal) = self.palette_cache.get(&path) {
            self.palette = pal.clone();
        } else {
            let pal = extract_palette(&path);
            self.palette_cache.insert(path.clone(), pal.clone());
            self.palette = pal;
        }

        // Check if background texture is already in RAM cache
        if !self.bg_textures.contains_key(&path) {
            self.request_bg_load(ctx, path, preview_path);
        }

        ctx.request_repaint();
    }

    fn request_bg_load(&mut self, ctx: &egui::Context, path: PathBuf, preview_path: PathBuf) {
        if self.bg_loading_path.as_ref() == Some(&path) {
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.bg_rx = Some(rx);
        self.bg_loading_path = Some(path.clone());

        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            let pal = extract_palette(&path);

            // Fast path: load preview from disk cache (<5ms)
            let img_opt = if preview_path.exists() {
                image::open(&preview_path).ok()
            } else if let Ok(orig) = image::open(&path) {
                // Generate 1080p preview using fast thumbnail subsampling (~15ms)
                let preview = orig.thumbnail(1920, 1080);
                let _ = preview.save(&preview_path);
                Some(preview)
            } else {
                None
            };

            if let Some(img) = img_opt {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                let _ = tx.send(BgResult {
                    path,
                    img: color_img,
                    palette: pal,
                });
                ctx_clone.request_repaint();
            }
        });
    }
}

impl eframe::App for WallpaperApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let screen_rect = ui.max_rect();
        let screen_w = screen_rect.width();
        let screen_h = screen_rect.height();
        let n = self.items.len();

        if n == 0 {
            return;
        }

        // 1. Process asynchronous background loader results
        if let Some(ref rx) = self.bg_rx {
            if let Ok(result) = rx.try_recv() {
                let tex = ctx.load_texture(
                    format!("bg_{}", result.path.to_string_lossy()),
                    result.img,
                    egui::TextureOptions::LINEAR,
                );
                self.palette_cache.insert(result.path.clone(), result.palette.clone());
                if self.bg_loading_path.as_ref() == Some(&result.path) {
                    self.palette = result.palette;
                }
                self.bg_textures.insert(result.path, tex);
                self.bg_loading_path = None;
            }
        }

        // 2. Pre-load all card thumbnails on frame 1 (instant into VRAM)
        for (i, item) in self.items.iter().enumerate() {
            if !self.thumb_textures.contains_key(&item.thumb_path) {
                if let Some(tex) = load_texture_from_file(
                    &ctx,
                    &format!("thumb_{}", i),
                    &item.thumb_path,
                    None,
                ) {
                    self.thumb_textures.insert(item.thumb_path.clone(), tex);
                }
            }
        }

        // 3. Handle Keyboard & Scroll Inputs
        let mut delta_nav: isize = 0;
        let mut commit = false;
        let mut cancel = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D) {
                delta_nav += 1;
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A) {
                delta_nav -= 1;
            }
            if i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space) {
                commit = true;
            }
            if i.key_pressed(egui::Key::Escape) {
                cancel = true;
            }

            // Mouse wheel / Touchpad horizontal/vertical scroll handling with accumulator
            let scroll_val = if i.smooth_scroll_delta.x.abs() > i.smooth_scroll_delta.y.abs() {
                -i.smooth_scroll_delta.x
            } else {
                -i.smooth_scroll_delta.y
            };

            if scroll_val.abs() > 0.0 {
                self.scroll_accum += scroll_val;
            }
        });

        if self.scroll_accum > 25.0 {
            delta_nav += 1;
            self.scroll_accum = 0.0;
        } else if self.scroll_accum < -25.0 {
            delta_nav -= 1;
            self.scroll_accum = 0.0;
        }

        if delta_nav != 0 {
            self.navigate(&ctx, delta_nav);
        }

        if commit {
            self.committed = true;
            self.should_close = true;
        }

        if cancel {
            self.should_close = true;
        }

        if self.should_close {
            if self.committed {
                apply_wallpaper(&self.items[self.current_index].path);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Trigger initial background load if needed
        let cur_path = self.items[self.current_index].path.clone();
        let cur_thumb_path = self.items[self.current_index].thumb_path.clone();
        let cur_preview_path = self.items[self.current_index].preview_path.clone();

        if !self.bg_textures.contains_key(&cur_path) && self.bg_loading_path.is_none() {
            self.request_bg_load(&ctx, cur_path.clone(), cur_preview_path);
        }

        // 4. Smooth continuous transition decay
        if self.scroll_transition.abs() > 0.002 {
            self.scroll_transition += (0.0 - self.scroll_transition) * 0.25;
            ctx.request_repaint();
        } else {
            self.scroll_transition = 0.0;
        }

        // 5. Render Fullscreen Background Wallpaper
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen_rect, 0.0, egui::Color32::from_rgb(10, 12, 16));

        let mut drawn_bg = false;
        if let Some(bg_tex) = self.bg_textures.get(&cur_path) {
            let img_size = bg_tex.size_vec2();
            if img_size.x > 0.0 && img_size.y > 0.0 {
                let scale = (screen_w / img_size.x).max(screen_h / img_size.y);
                let draw_w = img_size.x * scale;
                let draw_h = img_size.y * scale;
                let draw_x = screen_rect.center().x - draw_w / 2.0;
                let draw_y = screen_rect.center().y - draw_h / 2.0;
                let dest = egui::Rect::from_min_size(egui::pos2(draw_x, draw_y), egui::vec2(draw_w, draw_h));
                painter.image(
                    bg_tex.id(),
                    dest,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                drawn_bg = true;
            }
        }

        // Fallback: draw thumbnail during quick initial load so screen is never blank
        if !drawn_bg {
            if let Some(thumb_tex) = self.thumb_textures.get(&cur_thumb_path) {
                let img_size = thumb_tex.size_vec2();
                if img_size.x > 0.0 && img_size.y > 0.0 {
                    let scale = (screen_w / img_size.x).max(screen_h / img_size.y);
                    let draw_w = img_size.x * scale;
                    let draw_h = img_size.y * scale;
                    let draw_x = screen_rect.center().x - draw_w / 2.0;
                    let draw_y = screen_rect.center().y - draw_h / 2.0;
                    let dest = egui::Rect::from_min_size(egui::pos2(draw_x, draw_y), egui::vec2(draw_w, draw_h));
                    painter.image(
                        thumb_tex.id(),
                        dest,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
        }

        // 6. Render Infinite Continuous Carousel Cards
        let center_x = screen_rect.center().x;
        let cards_y = screen_h - 130.0;
        let card_step = 265.0f32;
        let rounding = egui::CornerRadius::same(16);

        let accent_c = egui::Color32::from_rgb(
            self.palette.accent_rgb.0,
            self.palette.accent_rgb.1,
            self.palette.accent_rgb.2,
        );

        let mut clicked_slot: Option<isize> = None;

        // Render virtual slots centered around the active position (-4..=4)
        for slot in -4..=4 {
            let item_idx = (self.current_index as isize + slot).rem_euclid(n as isize) as usize;
            let item = &self.items[item_idx];

            let visual_offset = slot as f32 + self.scroll_transition;
            let card_center_x = center_x + visual_offset * card_step;

            // Frustum culling
            if card_center_x + card_step < -100.0 || card_center_x - card_step > screen_w + 100.0 {
                continue;
            }

            let dist = visual_offset.abs();
            let factor = (1.0 - dist.min(1.0)).max(0.0);

            // Active card is noticeably larger (~270x152 vs ~210x118, ~28% larger)
            let w = 210.0 + 60.0 * factor;
            let h = 118.0 + 34.0 * factor;

            let card_rect = egui::Rect::from_center_size(
                egui::pos2(card_center_x, cards_y),
                egui::vec2(w, h),
            );

            // Floating Drop Shadow with matching rounded corners
            let shadow_offset = 6.0 + 4.0 * factor;
            let shadow_alpha = (65.0 + 95.0 * factor) as u8;
            painter.rect_filled(
                card_rect.translate(egui::vec2(0.0, shadow_offset)),
                rounding,
                egui::Color32::from_black_alpha(shadow_alpha),
            );

            // Render Card Image with true 16px corner radius and anti-aliasing
            if let Some(tex) = self.thumb_textures.get(&item.thumb_path) {
                let img_alpha = (145.0 + 110.0 * factor) as u8;
                let img = egui::Image::new(tex)
                    .corner_radius(16.0)
                    .fit_to_exact_size(card_rect.size())
                    .tint(egui::Color32::from_white_alpha(img_alpha));
                ui.put(card_rect, img);
            } else {
                painter.rect_filled(card_rect, rounding, egui::Color32::from_black_alpha(160));
            }

            // Smooth dynamic border highlight
            if dist < 0.5 {
                let stroke_alpha = ((1.0 - dist * 2.0) * 255.0) as u8;
                let stroke_col = egui::Color32::from_rgba_unmultiplied(
                    self.palette.accent_rgb.0,
                    self.palette.accent_rgb.1,
                    self.palette.accent_rgb.2,
                    stroke_alpha,
                );
                painter.rect_stroke(
                    card_rect,
                    rounding,
                    egui::Stroke::new(3.0, stroke_col),
                    egui::StrokeKind::Outside,
                );
            } else {
                painter.rect_stroke(
                    card_rect,
                    rounding,
                    egui::Stroke::new(1.0, egui::Color32::from_white_alpha(35)),
                    egui::StrokeKind::Outside,
                );
            }

            // Click handling
            let response = ui.interact(
                card_rect,
                ui.scope_id().with(format!("card_slot_{}", slot)),
                egui::Sense::click(),
            );
            if response.clicked() {
                clicked_slot = Some(slot);
            }
        }

        if let Some(slot) = clicked_slot {
            if slot == 0 {
                self.committed = true;
                self.should_close = true;
            } else {
                self.navigate(&ctx, slot);
            }
        }

        // 7. Minimal Floating Pagination Dots
        if n > 1 {
            let total_dots_w = (n as f32 * 14.0).min(screen_w * 0.4);
            let dot_step = total_dots_w / n as f32;
            let start_dot_x = center_x - total_dots_w / 2.0 + dot_step / 2.0;

            for i in 0..n {
                let is_cur = i == self.current_index;
                let dot_x = start_dot_x + i as f32 * dot_step;
                let dot_w = if is_cur { 18.0 } else { 6.0 };
                let dot_rect = egui::Rect::from_center_size(
                    egui::pos2(dot_x, screen_h - 22.0),
                    egui::vec2(dot_w, 4.0),
                );
                painter.rect_filled(
                    dot_rect,
                    egui::CornerRadius::same(2),
                    if is_cur { accent_c } else { egui::Color32::from_white_alpha(65) },
                );
            }
        }
    }
}

pub fn apply_wallpaper(wallpaper_path: &Path) {
    let abs_path = match wallpaper_path.canonicalize() {
        Ok(p) => p,
        Err(_) => wallpaper_path.to_path_buf(),
    };
    let abs_str = abs_path.to_string_lossy();

    // 1. Set KDE Plasma Wallpaper via DBus
    let script = format!(
        r#"var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {{
    var d = allDesktops[i];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
    d.writeConfig("Image", "file://{}");
}}"#,
        abs_str
    );

    let _ = Command::new("qdbus-qt6")
        .args([
            "org.kde.plasmashell",
            "/PlasmaShell",
            "org.kde.PlasmaShell.evaluateScript",
            &script,
        ])
        .status();

    // 2. Extract Palette & update themes
    let pal = extract_palette(&abs_path);
    apply_theme(&pal, &abs_path);
}

pub fn select_wallpaper() {
    let home = match env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => return,
    };

    let wp_dir = home.join("Resimler/Wallpapers");
    if !wp_dir.exists() {
        eprintln!("Wallpapers directory not found: {:?}", wp_dir);
        return;
    }

    let thumb_cache_dir = home.join(".cache/wt-rust/thumbnails");
    let preview_cache_dir = home.join(".cache/wt-rust/previews");
    let _ = fs::create_dir_all(&thumb_cache_dir);
    let _ = fs::create_dir_all(&preview_cache_dir);

    let mut files = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&wp_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_ascii_lowercase();
                if matches!(ext_lower.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                    files.push(path);
                }
            }
        }
    }

    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    if files.is_empty() {
        eprintln!("No wallpapers found in {:?}", wp_dir);
        return;
    }

    let mut items = Vec::new();
    for path in &files {
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let thumb_path = thumb_cache_dir.join(format!("{}.png", name));
        let preview_path = preview_cache_dir.join(format!("{}.jpg", name));

        let need_thumb = if !thumb_path.exists() {
            true
        } else if let (Ok(m_src), Ok(m_dst)) = (fs::metadata(path), fs::metadata(&thumb_path)) {
            m_src.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                > m_dst.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        } else {
            false
        };

        if need_thumb {
            if let Ok(img) = image::open(path) {
                let thumb = img.thumbnail_exact(320, 180);
                let _ = thumb.save(&thumb_path);
            }
        }

        items.push(WallpaperItem {
            path: path.clone(),
            thumb_path,
            preview_path,
        });
    }

    // Determine initial index from current wallpaper marker
    let cur_marker = home.join(".cache/wallpaper-picker/current");
    let mut initial_idx = 0;
    if let Ok(cur_str) = fs::read_to_string(&cur_marker) {
        let trimmed = cur_str.trim();
        for (i, it) in items.iter().enumerate() {
            if it.path.to_string_lossy() == trimmed {
                initial_idx = i;
                break;
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_decorations(false)
            .with_transparent(true)
            .with_title("WT Wallpaper Selector"),
        ..Default::default()
    };

    let app = WallpaperApp::new(items, initial_idx);
    let _ = eframe::run_native(
        "WT Wallpaper Selector",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}

pub fn handle_wallpaper(args: &[String]) {
    if args.is_empty() || args[0] == "select" {
        select_wallpaper();
    } else if args[0] == "apply" {
        if let Some(path_str) = args.get(1) {
            apply_wallpaper(Path::new(path_str));
        } else {
            eprintln!("Usage: wt-rust wallpaper apply <path>");
        }
    } else {
        eprintln!("Usage: wt-rust wallpaper [select|apply <path>]");
    }
}
