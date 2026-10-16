use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::theme::{
    Palette, apply_theme, default_palette, extract_palette, extract_palette_from_image, hsl_to_rgb,
    rgb_to_hsl,
};

pub type ThemeSwatches = [(u8, u8, u8); 4];

pub struct WallpaperItem {
    pub path: PathBuf,
    pub thumb_path: PathBuf,
    pub preview_path: PathBuf,
}

struct BgResult {
    path: PathBuf,
    img: egui::ColorImage,
    palette: Palette,
    swatches: ThemeSwatches,
}

pub struct PreloadedData {
    thumbs: Vec<(PathBuf, egui::ColorImage)>,
    palettes: HashMap<PathBuf, Palette>,
    swatches: HashMap<PathBuf, ThemeSwatches>,
    initial_bg: Option<(PathBuf, egui::ColorImage)>,
}

pub struct WallpaperApp {
    items: Vec<WallpaperItem>,
    current_index: usize,
    bg_textures: HashMap<PathBuf, egui::TextureHandle>,
    thumb_textures: HashMap<PathBuf, egui::TextureHandle>,
    thumb_failed: HashSet<PathBuf>,
    pending_thumbs: Vec<(PathBuf, egui::ColorImage)>,
    pending_initial_bg: Option<(PathBuf, egui::ColorImage)>,
    palette_cache: HashMap<PathBuf, Palette>,
    swatch_cache: HashMap<PathBuf, ThemeSwatches>,
    palette: Palette,
    swatches: ThemeSwatches,
    anim_accent: [f32; 3],
    anim_swatches: [[f32; 3]; 4],
    hover_anim: [f32; 9],
    intro_anim: f32,
    prev_bg_path: Option<PathBuf>,
    bg_crossfade: f32,
    scroll_transition: f32,
    scroll_accum: f32,
    should_close: bool,
    committed: bool,
    bg_rx: Option<std::sync::mpsc::Receiver<BgResult>>,
    bg_loading_path: Option<PathBuf>,
    bg_failed: HashSet<PathBuf>,
}

fn default_swatches_for_palette(pal: &Palette) -> ThemeSwatches {
    let (r, g, b) = pal.accent_rgb;
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let c1 = hsl_to_rgb(
        (h + 330.0).rem_euclid(360.0),
        (s * 0.95).clamp(0.45, 0.85),
        (l * 0.88).clamp(0.42, 0.68),
    );
    let c2 = hsl_to_rgb(
        (h + 35.0).rem_euclid(360.0),
        (s * 0.90).clamp(0.45, 0.85),
        (l * 0.96).clamp(0.48, 0.72),
    );
    let c3 = hsl_to_rgb(
        (h + 180.0).rem_euclid(360.0),
        (s * 0.65).clamp(0.30, 0.70),
        (l * 0.75).clamp(0.35, 0.62),
    );
    [c1, c2, c3, pal.accent_rgb]
}

fn extract_swatches_from_image(img: &image::DynamicImage, pal: &Palette) -> ThemeSwatches {
    use image::GenericImageView;

    let small = img.thumbnail_exact(48, 48);
    let mut hue_bins: Vec<(f32, f32, f32, usize)> = vec![(0.0, 0.0, 0.0, 0); 16];

    for (_, _, pixel) in small.pixels() {
        let (h, s, l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        if l > 0.16 && l < 0.88 && s > 0.18 {
            let bin = (((h + 11.25).rem_euclid(360.0) / 22.5) as usize).min(15);
            let h_unwrapped = if h >= 348.75 { h - 360.0 } else { h };
            hue_bins[bin].0 += s;
            hue_bins[bin].1 += l;
            hue_bins[bin].2 += h_unwrapped;
            hue_bins[bin].3 += 1;
        }
    }

    let mut ranked: Vec<(usize, f32, (u8, u8, u8))> = hue_bins
        .iter()
        .enumerate()
        .filter_map(|(idx, &(sum_s, sum_l, sum_h, count))| {
            if count == 0 {
                return None;
            }
            let c = count as f32;
            let avg_s = (sum_s / c).clamp(0.42, 0.82);
            let avg_l = (sum_l / c).clamp(0.46, 0.68);
            let avg_h = (sum_h / c).rem_euclid(360.0);
            let score = c * avg_s * (1.0 - (avg_l - 0.56).abs() * 0.8);
            Some((idx, score, hsl_to_rgb(avg_h, avg_s, avg_l)))
        })
        .collect();

    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut picked_bins: Vec<usize> = Vec::with_capacity(3);
    let mut colors: Vec<(u8, u8, u8)> = Vec::with_capacity(3);

    for &(bin_idx, _, rgb) in &ranked {
        let distinct = picked_bins.iter().all(|&prev| {
            let diff = (bin_idx as isize - prev as isize).abs();
            let wrap_diff = 16 - diff;
            diff.min(wrap_diff) >= 2
        });
        if distinct {
            picked_bins.push(bin_idx);
            colors.push(rgb);
            if colors.len() == 3 {
                break;
            }
        }
    }

    let fallback = default_swatches_for_palette(pal);
    while colors.len() < 3 {
        colors.push(fallback[colors.len()]);
    }

    [colors[0], colors[1], colors[2], pal.accent_rgb]
}

fn load_texture_from_file(
    ctx: &egui::Context,
    name: &str,
    path: &Path,
    max_dims: Option<(u32, u32)>,
) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?;
    let img = if let Some((w, h)) = max_dims {
        img.resize_to_fill(w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(name, color_img, egui::TextureOptions::LINEAR))
}

fn paint_cover_image(
    painter: &egui::Painter,
    screen_rect: egui::Rect,
    tex: &egui::TextureHandle,
    tint: egui::Color32,
) {
    let img_size = tex.size_vec2();
    if img_size.x > 0.0 && img_size.y > 0.0 {
        let scale = (screen_rect.width() / img_size.x).max(screen_rect.height() / img_size.y);
        let dest = egui::Rect::from_center_size(screen_rect.center(), img_size * scale);
        painter.image(
            tex.id(),
            dest,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let c = t.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Computes the unscaled (w, h, bottom_lift) for a card at continuous distance `dist` from center.
/// Matches the stepped silhouette in the user's sketch:
/// - Center card (dist = 0) is significantly larger and taller
/// - Neighboring cards step down smoothly toward the screen edges
fn card_metrics_at_dist(dist: f32) -> (f32, f32, f32) {
    // Keyframes at dist = 0, 1, 2, 3, 4+
    const W: [f32; 5] = [368.0, 236.0, 198.0, 166.0, 142.0];
    const H: [f32; 5] = [207.0, 133.0, 111.0, 93.0, 80.0];
    const LIFT: [f32; 5] = [8.0, 18.0, 11.0, 4.0, 0.0];

    let clamped = dist.clamp(0.0, 4.0);
    let seg = (clamped.floor() as usize).min(3);
    let frac = smoothstep(clamped - seg as f32);

    let w = lerp(W[seg], W[seg + 1], frac);
    let h = lerp(H[seg], H[seg + 1], frac);
    let lift = lerp(LIFT[seg], LIFT[seg + 1], frac);
    (w, h, lift)
}

/// Computes the unscaled horizontal center offset from screen center for a continuous `visual_offset`,
/// preserving clean, non-overlapping gaps between variable-width cards at all fractional scroll states.
fn card_x_offset(visual_offset: f32) -> f32 {
    // Cumulative centers for offsets 0, 1, 2, 3, 4:
    // gap between 0 & 1 = 28px -> (368 + 236)/2 + 28 = 330
    // gap between 1 & 2 = 24px -> 330 + (236 + 198)/2 + 24 = 571
    // gap between 2 & 3 = 22px -> 571 + (198 + 166)/2 + 22 = 775
    // gap between 3 & 4 = 20px -> 775 + (166 + 142)/2 + 20 = 949
    const X_POS: [f32; 5] = [0.0, 330.0, 571.0, 775.0, 949.0];

    let sign = visual_offset.signum();
    let abs_v = visual_offset.abs().min(4.0);
    let seg = (abs_v.floor() as usize).min(3);
    let frac = smoothstep(abs_v - seg as f32);

    sign * lerp(X_POS[seg], X_POS[seg + 1], frac)
}

impl WallpaperApp {
    pub fn new(items: Vec<WallpaperItem>, initial_index: usize) -> Self {
        let safe_index = if items.is_empty() {
            0
        } else {
            initial_index.min(items.len() - 1)
        };
        let initial_palette = default_palette();
        let initial_swatches = default_swatches_for_palette(&initial_palette);

        Self {
            items,
            current_index: safe_index,
            bg_textures: HashMap::new(),
            thumb_textures: HashMap::new(),
            thumb_failed: HashSet::new(),
            pending_thumbs: Vec::new(),
            pending_initial_bg: None,
            palette_cache: HashMap::new(),
            swatch_cache: HashMap::new(),
            anim_accent: [
                initial_palette.accent_rgb.0 as f32,
                initial_palette.accent_rgb.1 as f32,
                initial_palette.accent_rgb.2 as f32,
            ],
            anim_swatches: initial_swatches.map(|(r, g, b)| [r as f32, g as f32, b as f32]),
            hover_anim: [0.0; 9],
            intro_anim: 0.0,
            palette: initial_palette,
            swatches: initial_swatches,
            prev_bg_path: None,
            bg_crossfade: 1.0,
            scroll_transition: 0.0,
            scroll_accum: 0.0,
            should_close: false,
            committed: false,
            bg_rx: None,
            bg_loading_path: None,
            bg_failed: HashSet::new(),
        }
    }

    pub fn with_preloaded(
        items: Vec<WallpaperItem>,
        initial_index: usize,
        preloaded: PreloadedData,
    ) -> Self {
        let mut app = Self::new(items, initial_index);
        app.pending_thumbs = preloaded.thumbs;
        app.pending_initial_bg = preloaded.initial_bg;
        app.palette_cache = preloaded.palettes;
        app.swatch_cache = preloaded.swatches;

        if let Some(it) = app.items.get(app.current_index) {
            if let Some(pal) = app.palette_cache.get(&it.path).cloned() {
                app.anim_accent = [
                    pal.accent_rgb.0 as f32,
                    pal.accent_rgb.1 as f32,
                    pal.accent_rgb.2 as f32,
                ];
                app.palette = pal;
            }
            if let Some(&sw) = app.swatch_cache.get(&it.path) {
                app.swatches = sw;
                app.anim_swatches = sw.map(|(r, g, b)| [r as f32, g as f32, b as f32]);
            } else {
                let sw = default_swatches_for_palette(&app.palette);
                app.swatches = sw;
                app.anim_swatches = sw.map(|(r, g, b)| [r as f32, g as f32, b as f32]);
            }
        }
        app
    }

    fn shift_hover_slots(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        let old = self.hover_anim;
        self.hover_anim = [0.0; 9];
        for new_slot in -4isize..=4 {
            let old_slot = new_slot + delta;
            if (-4..=4).contains(&old_slot) {
                self.hover_anim[(new_slot + 4) as usize] = old[(old_slot + 4) as usize];
            }
        }
    }

    fn navigate(&mut self, ctx: &egui::Context, delta: isize) {
        if self.items.is_empty() || delta == 0 {
            return;
        }

        let prev_idx = self.current_index;
        let len = self.items.len() as isize;
        let next_idx = (prev_idx as isize + delta).rem_euclid(len) as usize;

        if next_idx != prev_idx {
            self.prev_bg_path = Some(self.items[prev_idx].path.clone());
            self.bg_crossfade = 0.0;
            self.shift_hover_slots(delta);
        }

        self.current_index = next_idx;
        self.scroll_transition = (self.scroll_transition + delta as f32).clamp(-3.5, 3.5);
        self.on_selection_changed(ctx);
    }

    fn select_exact_index(&mut self, ctx: &egui::Context, target_idx: usize) {
        if target_idx >= self.items.len() || target_idx == self.current_index {
            return;
        }
        let len = self.items.len() as isize;
        let cur = self.current_index as isize;
        let tgt = target_idx as isize;
        // Shortest wrapped signed delta so Home/End/Random also animate the carousel smoothly
        let mut delta = (tgt - cur).rem_euclid(len);
        if delta > len / 2 {
            delta -= len;
        }
        let visual_delta = delta.clamp(-3, 3);

        self.prev_bg_path = Some(self.items[self.current_index].path.clone());
        self.bg_crossfade = 0.0;
        self.shift_hover_slots(visual_delta);
        self.current_index = target_idx;
        self.scroll_transition = (self.scroll_transition + visual_delta as f32).clamp(-3.5, 3.5);
        self.on_selection_changed(ctx);
    }

    fn select_random(&mut self, ctx: &egui::Context) {
        let len = self.items.len();
        if len <= 1 {
            return;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(1);
        let step = 1 + (nanos % (len - 1));
        let target_idx = (self.current_index + step) % len;
        self.select_exact_index(ctx, target_idx);
    }

    fn on_selection_changed(&mut self, ctx: &egui::Context) {
        if self.items.is_empty() {
            return;
        }

        let path = self.items[self.current_index].path.clone();
        let preview_path = self.items[self.current_index].preview_path.clone();

        if let Some(pal) = self.palette_cache.get(&path) {
            self.palette = pal.clone();
        }
        if let Some(&sw) = self.swatch_cache.get(&path) {
            self.swatches = sw;
        } else {
            self.swatches = default_swatches_for_palette(&self.palette);
        }

        if !self.bg_textures.contains_key(&path) && !self.bg_failed.contains(&path) {
            self.request_bg_load(ctx, path, preview_path);
        }

        ctx.request_repaint();
    }

    fn request_bg_load(&mut self, ctx: &egui::Context, path: PathBuf, preview_path: PathBuf) {
        if self.bg_loading_path.as_ref() == Some(&path) || self.bg_failed.contains(&path) {
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.bg_rx = Some(rx);
        self.bg_loading_path = Some(path.clone());

        let cached_pal = self.palette_cache.get(&path).cloned();
        let cached_sw = self.swatch_cache.get(&path).copied();
        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            let need_preview = is_cache_stale(&path, &preview_path);
            let img_opt = if !need_preview {
                image::open(&preview_path).ok()
            } else {
                None
            }
            .or_else(|| {
                image::open(&path).ok().map(|orig| {
                    let preview = orig.thumbnail(1920, 1080);
                    let _ = preview.save(&preview_path);
                    preview
                })
            });

            if let Some(img) = img_opt {
                let pal = cached_pal.unwrap_or_else(|| extract_palette_from_image(&img));
                let swatches = cached_sw.unwrap_or_else(|| extract_swatches_from_image(&img, &pal));
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                let _ = tx.send(BgResult {
                    path,
                    img: color_img,
                    palette: pal,
                    swatches,
                });
                ctx_clone.request_repaint();
            }
        });
    }

    fn poll_bg_result(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.bg_rx {
            match rx.try_recv() {
                Ok(result) => {
                    let tex = ctx.load_texture(
                        format!("bg_{}", result.path.to_string_lossy()),
                        result.img,
                        egui::TextureOptions::LINEAR,
                    );
                    self.palette_cache
                        .insert(result.path.clone(), result.palette.clone());
                    self.swatch_cache
                        .insert(result.path.clone(), result.swatches);
                    if self.items[self.current_index].path == result.path {
                        self.palette = result.palette;
                        self.swatches = result.swatches;
                    }
                    self.bg_textures.insert(result.path, tex);
                    self.bg_loading_path = None;
                    self.bg_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if let Some(failed_path) = self.bg_loading_path.take() {
                        self.bg_failed.insert(failed_path);
                    }
                    self.bg_rx = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
    }

    fn prefetch_neighbors(&mut self, ctx: &egui::Context) {
        if self.bg_loading_path.is_some() || self.items.len() <= 1 {
            return;
        }
        let n = self.items.len() as isize;
        for offset in [1isize, -1isize, 2isize, -2isize] {
            let idx = (self.current_index as isize + offset).rem_euclid(n) as usize;
            let path = self.items[idx].path.clone();
            if !self.bg_textures.contains_key(&path) && !self.bg_failed.contains(&path) {
                let preview_path = self.items[idx].preview_path.clone();
                self.request_bg_load(ctx, path, preview_path);
                break;
            }
        }
    }

    fn step_animations(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
        let mut animating = false;

        // 1. Entrance animation on open
        if self.intro_anim < 1.0 {
            self.intro_anim = (self.intro_anim + dt * 5.5).min(1.0);
            animating = true;
        }

        // 2. Carousel scroll spring decay (smooth exponential ease)
        if self.scroll_transition.abs() > 0.0015 {
            let decay = (-13.5 * dt).exp();
            self.scroll_transition *= decay;
            animating = true;
        } else {
            self.scroll_transition = 0.0;
        }

        // 3. Fullscreen background crossfade
        if self.bg_crossfade < 1.0 {
            self.bg_crossfade = (self.bg_crossfade + dt * 5.8).min(1.0);
            if self.bg_crossfade >= 1.0 {
                self.prev_bg_path = None;
            }
            animating = true;
        }

        // 4. Smooth dynamic palette & 4-swatch interpolation
        let target_acc = [
            self.palette.accent_rgb.0 as f32,
            self.palette.accent_rgb.1 as f32,
            self.palette.accent_rgb.2 as f32,
        ];
        let color_lerp = 1.0 - (-12.0 * dt).exp();

        for (ch, &target_val) in target_acc.iter().enumerate() {
            let diff_a = target_val - self.anim_accent[ch];
            if diff_a.abs() > 0.35 {
                self.anim_accent[ch] += diff_a * color_lerp;
                animating = true;
            } else {
                self.anim_accent[ch] = target_val;
            }
        }

        for (sw_idx, &(r, g, b)) in self.swatches.iter().enumerate() {
            let target_sw = [r as f32, g as f32, b as f32];
            for (ch, &target_val) in target_sw.iter().enumerate() {
                let diff = target_val - self.anim_swatches[sw_idx][ch];
                if diff.abs() > 0.35 {
                    self.anim_swatches[sw_idx][ch] += diff * color_lerp;
                    animating = true;
                } else {
                    self.anim_swatches[sw_idx][ch] = target_val;
                }
            }
        }

        if animating {
            ctx.request_repaint();
        }
    }

    fn current_accent_color(&self, alpha: u8) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            self.anim_accent[0].round() as u8,
            self.anim_accent[1].round() as u8,
            self.anim_accent[2].round() as u8,
            alpha,
        )
    }

    fn current_swatch_color(&self, index: usize, alpha: u8) -> egui::Color32 {
        let rgb = self.anim_swatches[index.min(3)];
        egui::Color32::from_rgba_unmultiplied(
            rgb[0].round() as u8,
            rgb[1].round() as u8,
            rgb[2].round() as u8,
            alpha,
        )
    }

    fn render_background(&self, painter: &egui::Painter, screen_rect: egui::Rect) {
        painter.rect_filled(screen_rect, 0.0, egui::Color32::from_rgb(10, 12, 16));

        let cur_item = &self.items[self.current_index];
        let cur_tex = self
            .bg_textures
            .get(&cur_item.path)
            .or_else(|| self.thumb_textures.get(&cur_item.thumb_path));

        if self.bg_crossfade < 1.0
            && let Some(ref prev_path) = self.prev_bg_path
            && let Some(prev_tex) = self.bg_textures.get(prev_path).or_else(|| {
                self.items
                    .iter()
                    .find(|it| &it.path == prev_path)
                    .and_then(|it| self.thumb_textures.get(&it.thumb_path))
            })
        {
            paint_cover_image(painter, screen_rect, prev_tex, egui::Color32::WHITE);
            if let Some(tex) = cur_tex {
                let eased = smoothstep(self.bg_crossfade);
                let alpha = (eased * 255.0).round() as u8;
                paint_cover_image(
                    painter,
                    screen_rect,
                    tex,
                    egui::Color32::from_white_alpha(alpha),
                );
            }
            return;
        }

        if let Some(tex) = cur_tex {
            paint_cover_image(painter, screen_rect, tex, egui::Color32::WHITE);
        }
    }

    fn render_theme_swatches(
        &self,
        painter: &egui::Painter,
        active_card_rect: egui::Rect,
        scale: f32,
        focus_strength: f32,
    ) {
        if focus_strength <= 0.02 {
            return;
        }

        let alpha_f = smoothstep(focus_strength);
        let alpha = (alpha_f * 255.0).round() as u8;
        let shadow_alpha = (alpha_f * 130.0).round() as u8;
        let border_alpha = (alpha_f * 95.0).round() as u8;

        // Positioned centered slightly above the active center wallpaper card
        let swatch_h = (13.0 * scale).max(9.0);
        let gap = (7.0 * scale).max(4.0);
        let y_bottom = active_card_rect.top() - (10.0 * scale).max(7.0);
        let y_top = y_bottom - swatch_h;
        let rounding = egui::CornerRadius::same((4.0 * scale).round().clamp(3.0, 6.0) as u8);

        let widths = [
            (18.0 * scale).max(12.0),
            (18.0 * scale).max(12.0),
            (18.0 * scale).max(12.0),
            (30.0 * scale).max(20.0),
        ];
        let total_w: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
        let mut cur_x = active_card_rect.center().x - total_w * 0.5;

        for (idx, &w) in widths.iter().enumerate() {
            let rect =
                egui::Rect::from_min_max(egui::pos2(cur_x, y_top), egui::pos2(cur_x + w, y_bottom));

            // Soft drop shadow so swatches stand out cleanly on any wallpaper without a bar
            painter.rect_filled(
                rect.translate(egui::vec2(0.0, 2.5 * scale)).expand(1.0),
                rounding,
                egui::Color32::from_black_alpha(shadow_alpha),
            );

            painter.rect_filled(rect, rounding, self.current_swatch_color(idx, alpha));
            painter.rect_stroke(
                rect,
                rounding,
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(border_alpha)),
                egui::StrokeKind::Inside,
            );

            cur_x += w + gap;
        }
    }

    fn render_carousel(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        painter: &egui::Painter,
        screen_rect: egui::Rect,
    ) {
        let screen_w = screen_rect.width();
        let screen_h = screen_rect.height();
        let center_x = screen_rect.center().x;
        let n = self.items.len();

        // Proportional scale so all 7 cards (-3..=3) fit gracefully on any resolution
        let scale = (screen_w / 1920.0).clamp(0.62, 1.15);
        let intro_ease = smoothstep(self.intro_anim);
        let intro_y_offset = (1.0 - intro_ease) * 36.0 * scale;
        let base_bottom_y = screen_h - 22.0 * scale + intro_y_offset;

        let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.05);
        let hover_lerp = 1.0 - (-16.0 * dt).exp();
        let mut hover_animating = false;
        let mut clicked_slot: Option<isize> = None;

        // Sort slots back-to-front by distance from center so the active center card always draws on top
        let mut slots: Vec<isize> = (-4..=4).collect();
        slots.sort_by(|&a, &b| {
            let da = (a as f32 + self.scroll_transition).abs();
            let db = (b as f32 + self.scroll_transition).abs();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_slots = 9usize;
        let mut center_slot_rect: Option<egui::Rect> = None;

        for slot in slots {
            if n == 1 && slot != 0 {
                continue;
            }
            if n < total_slots {
                let half = (n as isize) / 2;
                let min_s = -half;
                let max_s = (n as isize - 1) - half;
                if slot < min_s || slot > max_s {
                    continue;
                }
            }

            let item_idx = (self.current_index as isize + slot).rem_euclid(n as isize) as usize;
            let item = &self.items[item_idx];

            let visual_offset = slot as f32 + self.scroll_transition;
            let dist = visual_offset.abs();
            if dist > 3.85 {
                continue;
            }

            let (raw_w, raw_h, raw_lift) = card_metrics_at_dist(dist);
            let w = raw_w * scale;
            let h = raw_h * scale;
            let bottom_y = base_bottom_y - raw_lift * scale;
            let card_center_x = center_x + card_x_offset(visual_offset) * scale;

            if card_center_x + w * 0.5 < -40.0 || card_center_x - w * 0.5 > screen_w + 40.0 {
                continue;
            }

            let probe_rect = egui::Rect::from_center_size(
                egui::pos2(card_center_x, bottom_y - h * 0.5),
                egui::vec2(w, h),
            );

            let response = ui.interact(
                probe_rect,
                ui.scope_id().with(("wallpaper_card_slot", slot)),
                egui::Sense::click(),
            );

            let slot_idx = (slot + 4) as usize;
            let target_hover = if response.hovered() && slot != 0 {
                1.0
            } else {
                0.0
            };
            let h_diff = target_hover - self.hover_anim[slot_idx];
            if h_diff.abs() > 0.005 {
                self.hover_anim[slot_idx] += h_diff * hover_lerp;
                hover_animating = true;
            } else {
                self.hover_anim[slot_idx] = target_hover;
            }
            let hover_t = self.hover_anim[slot_idx];

            let card_rect = probe_rect
                .translate(egui::vec2(0.0, -5.0 * scale * hover_t))
                .expand(3.5 * scale * hover_t);

            if slot == 0 {
                center_slot_rect = Some(card_rect);
            }

            let focus_factor = smoothstep(1.0 - dist.min(1.0));
            let edge_fade = smoothstep(1.0 - ((dist - 2.8) / 1.05).clamp(0.0, 1.0));
            let corner_px = (12.0 + 4.0 * focus_factor).round().clamp(10.0, 18.0);
            let rounding = egui::CornerRadius::same(corner_px as u8);

            // Multi-layered floating drop shadow (no container box behind cards)
            let shadow_y = (7.0 + 6.0 * focus_factor + 3.0 * hover_t) * scale;
            let ambient_alpha =
                ((75.0 + 85.0 * focus_factor + 25.0 * hover_t) * edge_fade * intro_ease) as u8;
            painter.rect_filled(
                card_rect
                    .translate(egui::vec2(0.0, shadow_y))
                    .expand(2.0 * scale),
                rounding,
                egui::Color32::from_black_alpha(ambient_alpha),
            );

            // Render card thumbnail
            if let Some(tex) = self.thumb_textures.get(&item.thumb_path) {
                let brightness = (155.0 + 100.0 * focus_factor + 45.0 * hover_t).min(255.0);
                let img_alpha = (brightness * edge_fade * intro_ease)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                egui::Image::new(tex)
                    .corner_radius(corner_px)
                    .tint(egui::Color32::from_white_alpha(img_alpha))
                    .paint_at(ui, card_rect);
            } else {
                painter.rect_filled(
                    card_rect,
                    rounding,
                    egui::Color32::from_black_alpha((170.0 * edge_fade * intro_ease) as u8),
                );
            }

            // Sleek border highlight (dynamic accent on active/hovered card, subtle glass rim on others)
            if dist < 0.85 {
                let glow_t = smoothstep(1.0 - dist / 0.85) * intro_ease;
                painter.rect_stroke(
                    card_rect.expand(2.0),
                    egui::CornerRadius::same((corner_px as u8).saturating_add(2)),
                    egui::Stroke::new(3.5, self.current_accent_color((68.0 * glow_t) as u8)),
                    egui::StrokeKind::Outside,
                );
                painter.rect_stroke(
                    card_rect,
                    rounding,
                    egui::Stroke::new(2.4, self.current_accent_color((255.0 * glow_t) as u8)),
                    egui::StrokeKind::Outside,
                );
            } else {
                let rim_alpha = ((38.0 + 110.0 * hover_t) * edge_fade * intro_ease) as u8;
                let rim_col = if hover_t > 0.05 {
                    self.current_accent_color(
                        ((80.0 + 145.0 * hover_t) * edge_fade * intro_ease) as u8,
                    )
                } else {
                    egui::Color32::from_white_alpha(rim_alpha)
                };
                painter.rect_stroke(
                    card_rect,
                    rounding,
                    egui::Stroke::new(1.2 + 0.6 * hover_t, rim_col),
                    egui::StrokeKind::Outside,
                );
            }

            if response.clicked() {
                clicked_slot = Some(slot);
            }
        }

        // Render the 4 simple theme color swatches anchored slightly above the active center card
        // so they never vanish during rapid keyboard or wheel scrolling
        if let Some(active_rect) = center_slot_rect {
            let settle = 1.0 - (self.scroll_transition.abs() / 3.8).clamp(0.0, 0.35);
            self.render_theme_swatches(painter, active_rect, scale, intro_ease * settle);
        }

        if hover_animating {
            ctx.request_repaint();
        }

        if let Some(slot) = clicked_slot {
            if slot == 0 {
                self.committed = true;
                self.should_close = true;
            } else {
                self.navigate(ctx, slot);
            }
        }
    }
}

fn crop_thumbnail_16_9(img: &image::DynamicImage) -> image::DynamicImage {
    img.resize_to_fill(320, 180, image::imageops::FilterType::Triangle)
}

fn is_cache_stale(src: &Path, dst: &Path) -> bool {
    if !dst.exists() {
        return true;
    }
    if let (Ok(m_src), Ok(m_dst)) = (fs::metadata(src), fs::metadata(dst)) {
        m_src.modified().unwrap_or(UNIX_EPOCH) > m_dst.modified().unwrap_or(UNIX_EPOCH)
    } else {
        false
    }
}

impl eframe::App for WallpaperApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let screen_rect = ui.max_rect();
        let n = self.items.len();

        if n == 0 {
            return;
        }

        // 1. Upload pre-decoded RAM images to GPU textures on first frame
        if !self.pending_thumbs.is_empty() {
            for (i, (thumb_path, color_img)) in self.pending_thumbs.drain(..).enumerate() {
                let tex = ctx.load_texture(
                    format!("thumb_pre_{}", i),
                    color_img,
                    egui::TextureOptions::LINEAR,
                );
                self.thumb_textures.insert(thumb_path, tex);
            }
        }
        if let Some((bg_path, bg_img)) = self.pending_initial_bg.take() {
            let tex = ctx.load_texture(
                format!("bg_{}", bg_path.to_string_lossy()),
                bg_img,
                egui::TextureOptions::LINEAR,
            );
            self.bg_textures.insert(bg_path, tex);
        }

        // 2. Process asynchronous background loader results
        self.poll_bg_result(&ctx);

        // 3. Fallback lazy load for any thumbnails not preloaded
        for (i, item) in self.items.iter().enumerate() {
            if !self.thumb_textures.contains_key(&item.thumb_path)
                && !self.thumb_failed.contains(&item.thumb_path)
            {
                if let Some(tex) =
                    load_texture_from_file(&ctx, &format!("thumb_{}", i), &item.thumb_path, None)
                {
                    self.thumb_textures.insert(item.thumb_path.clone(), tex);
                } else {
                    self.thumb_failed.insert(item.thumb_path.clone());
                }
            }
        }

        // 4. Handle Keyboard & Scroll Inputs
        let mut delta_nav: isize = 0;
        let mut commit = false;
        let mut cancel = false;
        let mut jump_home = false;
        let mut jump_end = false;
        let mut jump_random = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D) {
                delta_nav += 1;
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A) {
                delta_nav -= 1;
            }
            if i.key_pressed(egui::Key::Home) {
                jump_home = true;
            }
            if i.key_pressed(egui::Key::End) {
                jump_end = true;
            }
            if i.key_pressed(egui::Key::R) {
                jump_random = true;
            }
            if i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space) {
                commit = true;
            }
            if i.key_pressed(egui::Key::Escape) {
                cancel = true;
            }

            let scroll_val = if i.smooth_scroll_delta.x.abs() > i.smooth_scroll_delta.y.abs() {
                -i.smooth_scroll_delta.x
            } else {
                -i.smooth_scroll_delta.y
            };

            if scroll_val.abs() > 0.0 {
                self.scroll_accum += scroll_val;
            }
        });

        if jump_home && !self.items.is_empty() {
            self.select_exact_index(&ctx, 0);
        }
        if jump_end && !self.items.is_empty() {
            self.select_exact_index(&ctx, self.items.len() - 1);
        }
        if jump_random {
            self.select_random(&ctx);
        }

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
                let chosen = &self.items[self.current_index].path;
                let cached_pal = self.palette_cache.get(chosen);
                apply_wallpaper_with_palette(chosen, cached_pal);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // 5. Ensure current background texture is loaded, then prefetch neighbors
        let cur_path = self.items[self.current_index].path.clone();
        let cur_preview_path = self.items[self.current_index].preview_path.clone();

        if !self.bg_textures.contains_key(&cur_path) && self.bg_loading_path.is_none() {
            self.request_bg_load(&ctx, cur_path, cur_preview_path);
        } else if self.bg_loading_path.is_none() {
            self.prefetch_neighbors(&ctx);
        }

        // 6. Step smooth framerate-independent animations
        self.step_animations(&ctx);

        // 7. Render Fullscreen Wallpaper & Floating Carousel (Zero Text)
        let painter = ctx.layer_painter(egui::LayerId::background());
        self.render_background(&painter, screen_rect);
        self.render_carousel(ui, &ctx, &painter, screen_rect);
    }
}

pub fn apply_wallpaper_with_palette(wallpaper_path: &Path, cached_palette: Option<&Palette>) {
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

    // 2. Extract Palette (or reuse pre-computed palette) & update themes
    let owned_pal;
    let pal = match cached_palette {
        Some(p) => p,
        None => {
            owned_pal = extract_palette(&abs_path);
            &owned_pal
        }
    };
    apply_theme(pal, &abs_path);
}

pub fn apply_wallpaper(wallpaper_path: &Path) {
    apply_wallpaper_with_palette(wallpaper_path, None);
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

    let items: Vec<WallpaperItem> = files
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            WallpaperItem {
                thumb_path: thumb_cache_dir.join(format!("{}.png", name)),
                preview_path: preview_cache_dir.join(format!("{}.jpg", name)),
                path,
            }
        })
        .collect();

    // Determine initial index from current wallpaper marker
    let cur_marker = home.join(".cache/wt-rust/current");
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

    // Bounded parallel startup pipeline: ensure thumbnails exist, decode all thumbnails + palettes + swatches
    // in RAM, and pre-decode initial 1080p background preview across CPU cores.
    let next_idx = AtomicUsize::new(0);
    let preloaded = std::thread::scope(|s| {
        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(items.len())
            .max(1);
        let items_ref = &items;
        let next_idx_ref = &next_idx;

        let worker_handles: Vec<_> = (0..worker_count)
            .map(|_| {
                s.spawn(move || {
                    let mut local_out = Vec::new();
                    loop {
                        let i = next_idx_ref.fetch_add(1, Ordering::Relaxed);
                        if i >= items_ref.len() {
                            break;
                        }
                        let it = &items_ref[i];
                        let need_thumb = is_cache_stale(&it.path, &it.thumb_path);

                        let thumb_img = if need_thumb {
                            image::open(&it.path).ok().map(|orig| {
                                let t = crop_thumbnail_16_9(&orig);
                                let _ = t.save(&it.thumb_path);
                                t
                            })
                        } else {
                            image::open(&it.thumb_path)
                                .or_else(|_| image::open(&it.path).map(|o| crop_thumbnail_16_9(&o)))
                                .ok()
                        };

                        if let Some(img) = thumb_img {
                            let pal = extract_palette_from_image(&img);
                            let sw = extract_swatches_from_image(&img, &pal);
                            let rgba = img.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let color_img =
                                egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                            local_out.push((
                                it.path.clone(),
                                it.thumb_path.clone(),
                                color_img,
                                pal,
                                sw,
                            ));
                        }
                    }
                    local_out
                })
            })
            .collect();

        let initial_item = &items[initial_idx];
        let init_bg_handle = s.spawn(move || {
            let need_preview = is_cache_stale(&initial_item.path, &initial_item.preview_path);
            let img_opt = if !need_preview {
                image::open(&initial_item.preview_path).ok()
            } else {
                None
            }
            .or_else(|| {
                image::open(&initial_item.path).ok().map(|orig| {
                    let preview = orig.thumbnail(1920, 1080);
                    let _ = preview.save(&initial_item.preview_path);
                    preview
                })
            });
            img_opt.map(|img| {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
                (initial_item.path.clone(), color_img)
            })
        });

        let mut thumbs = Vec::with_capacity(items.len());
        let mut palettes = HashMap::with_capacity(items.len());
        let mut swatches = HashMap::with_capacity(items.len());
        for h in worker_handles {
            if let Ok(batch) = h.join() {
                for (path, thumb_path, color_img, pal, sw) in batch {
                    thumbs.push((thumb_path, color_img));
                    palettes.insert(path.clone(), pal);
                    swatches.insert(path, sw);
                }
            }
        }
        let initial_bg = init_bg_handle.join().ok().flatten();

        PreloadedData {
            thumbs,
            palettes,
            swatches,
            initial_bg,
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_fullscreen(true)
            .with_decorations(false)
            .with_transparent(true)
            .with_title("WT Wallpaper Selector"),
        ..Default::default()
    };

    let app = WallpaperApp::with_preloaded(items, initial_idx, preloaded);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stale_bg_load_does_not_overwrite_active_palette_and_disconnect_clears_state() {
        let ctx = egui::Context::default();
        let path_a = PathBuf::from("/tmp/wall_a.png");
        let path_b = PathBuf::from("/tmp/wall_b.png");

        let items = vec![
            WallpaperItem {
                path: path_a.clone(),
                thumb_path: PathBuf::from("/tmp/thumb_a.png"),
                preview_path: PathBuf::from("/tmp/prev_a.jpg"),
            },
            WallpaperItem {
                path: path_b.clone(),
                thumb_path: PathBuf::from("/tmp/thumb_b.png"),
                preview_path: PathBuf::from("/tmp/prev_b.jpg"),
            },
        ];

        let mut app = WallpaperApp::new(items, 0);
        let pal_a = Palette {
            bg: "#141011".to_string(),
            border: "rgba(226, 90, 120, 0.25)".to_string(),
            accent: "#e25a78".to_string(),
            accent_rgb: (226, 90, 120),
            ..default_palette()
        };
        let pal_b = Palette {
            bg: "#101410".to_string(),
            border: "rgba(90, 226, 120, 0.25)".to_string(),
            accent: "#5ae278".to_string(),
            accent_rgb: (90, 226, 120),
            ..default_palette()
        };
        let sw_b = default_swatches_for_palette(&pal_b);

        let dummy_img = egui::ColorImage::new([2, 2], vec![egui::Color32::RED; 4]);
        let tex_a = ctx.load_texture("bg_a", dummy_img.clone(), egui::TextureOptions::LINEAR);
        app.bg_textures.insert(path_a.clone(), tex_a);
        app.palette_cache.insert(path_a.clone(), pal_a.clone());
        app.palette = pal_a.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        app.bg_rx = Some(rx);
        app.bg_loading_path = Some(path_b.clone());
        app.current_index = 0;

        tx.send(BgResult {
            path: path_b.clone(),
            img: dummy_img,
            palette: pal_b.clone(),
            swatches: sw_b,
        })
        .unwrap();

        app.poll_bg_result(&ctx);

        assert_eq!(app.palette.accent, pal_a.accent);
        assert_eq!(app.palette_cache.get(&path_b).unwrap().accent, pal_b.accent);
        assert_eq!(app.swatch_cache.get(&path_b).copied(), Some(sw_b));
        assert!(app.bg_textures.contains_key(&path_b));
        assert!(app.bg_loading_path.is_none());

        let (tx_fail, rx_fail) = std::sync::mpsc::channel::<BgResult>();
        drop(tx_fail);
        app.bg_rx = Some(rx_fail);
        app.bg_loading_path = Some(PathBuf::from("/tmp/corrupt.png"));

        app.poll_bg_result(&ctx);
        assert!(app.bg_loading_path.is_none());
        assert!(app.bg_rx.is_none());
        assert!(app.bg_failed.contains(Path::new("/tmp/corrupt.png")));
    }

    #[test]
    fn test_navigation_and_non_overlapping_carousel_geometry() {
        let ctx = egui::Context::default();
        let items = vec![
            WallpaperItem {
                path: PathBuf::from("/tmp/w1.png"),
                thumb_path: PathBuf::from("/tmp/t1.png"),
                preview_path: PathBuf::from("/tmp/p1.jpg"),
            },
            WallpaperItem {
                path: PathBuf::from("/tmp/w2.jpg"),
                thumb_path: PathBuf::from("/tmp/t2.png"),
                preview_path: PathBuf::from("/tmp/p2.jpg"),
            },
            WallpaperItem {
                path: PathBuf::from("/tmp/w3.webp"),
                thumb_path: PathBuf::from("/tmp/t3.png"),
                preview_path: PathBuf::from("/tmp/p3.jpg"),
            },
        ];

        let mut app = WallpaperApp::new(items, 0);
        let dummy = egui::ColorImage::new([2, 2], vec![egui::Color32::BLACK; 4]);
        for (i, it) in app.items.iter().enumerate() {
            let tex = ctx.load_texture(format!("t_{}", i), dummy.clone(), Default::default());
            app.bg_textures.insert(it.path.clone(), tex);
        }

        app.navigate(&ctx, -1);
        assert_eq!(app.current_index, 2);
        assert_eq!(app.prev_bg_path.as_deref(), Some(Path::new("/tmp/w1.png")));

        // Verify center card is strictly larger than neighbor cards and adjacent slots never overlap
        let (w0, h0, _) = card_metrics_at_dist(0.0);
        let (w1, h1, _) = card_metrics_at_dist(1.0);
        let (w2, h2, _) = card_metrics_at_dist(2.0);
        let (w3, h3, _) = card_metrics_at_dist(3.0);
        assert!(w0 > w1 && w1 > w2 && w2 > w3);
        assert!(h0 > h1 && h1 > h2 && h2 > h3);

        for slot in 0..3 {
            let x_a = card_x_offset(slot as f32);
            let x_b = card_x_offset((slot + 1) as f32);
            let (wa, _, _) = card_metrics_at_dist(slot as f32);
            let (wb, _, _) = card_metrics_at_dist((slot + 1) as f32);
            let gap = (x_b - wb * 0.5) - (x_a + wa * 0.5);
            assert!(
                gap >= 18.0,
                "Expected clean gap between slot {slot} and {}, got {gap}",
                slot + 1
            );
        }

        // Verify adjacent cards also maintain exact non-overlapping gaps at fractional scroll offsets
        for step in 0..=20 {
            let frac = step as f32 * 0.05;
            for slot in -3..3 {
                let v_a = slot as f32 + frac;
                let v_b = (slot + 1) as f32 + frac;
                let x_a = card_x_offset(v_a);
                let x_b = card_x_offset(v_b);
                let (wa, _, _) = card_metrics_at_dist(v_a.abs());
                let (wb, _, _) = card_metrics_at_dist(v_b.abs());
                let gap = (x_b - wb * 0.5) - (x_a + wa * 0.5);
                assert!(
                    gap >= 18.0,
                    "Expected clean mid-scroll gap at frac={frac} between slot {slot} and {}, got {gap}",
                    slot + 1
                );
            }
        }

        // Verify select_exact_index triggers smooth scroll_transition and shifts hover slots cleanly
        app.scroll_transition = 0.0;
        app.hover_anim[5] = 0.85; // slot +1 hovered
        app.select_exact_index(&ctx, 0);
        assert_eq!(app.current_index, 0);
        assert!(
            (app.scroll_transition - 1.0).abs() < 1e-5,
            "select_exact_index should trigger smooth carousel scroll_transition (+1.0 from 2 -> 0 in 3-item list)"
        );
        assert!(
            (app.hover_anim[4] - 0.85).abs() < 1e-5,
            "hover state from slot +1 should transfer to slot 0 when advancing by +1"
        );
    }

    #[test]
    fn test_extract_swatches_returns_four_colors_ending_with_accent() {
        let mut raw = image::RgbImage::new(48, 48);
        for (x, y, p) in raw.enumerate_pixels_mut() {
            *p = if x < 16 {
                image::Rgb([220, 50, 70])
            } else if x < 32 {
                image::Rgb([40, 190, 110])
            } else if y < 24 {
                image::Rgb([55, 110, 230])
            } else {
                image::Rgb([180, 90, 220])
            };
        }
        let dyn_img = image::DynamicImage::ImageRgb8(raw);
        let pal = extract_palette_from_image(&dyn_img);
        let swatches = extract_swatches_from_image(&dyn_img, &pal);
        assert_eq!(swatches[3], pal.accent_rgb);
        assert_ne!(swatches[0], swatches[1]);
        assert_ne!(swatches[1], swatches[2]);
    }

    #[test]
    fn test_crop_thumbnail_16_9_preserves_aspect_without_distortion() {
        let mut raw = image::RgbImage::new(500, 100);
        for (x, _, p) in raw.enumerate_pixels_mut() {
            *p = if (160..340).contains(&x) {
                image::Rgb([0, 255, 0])
            } else {
                image::Rgb([255, 0, 0])
            };
        }
        let dyn_img = image::DynamicImage::ImageRgb8(raw);
        let thumb = crop_thumbnail_16_9(&dyn_img).to_rgb8();
        assert_eq!(thumb.dimensions(), (320, 180));
        let left_px = thumb.get_pixel(10, 90);
        let right_px = thumb.get_pixel(310, 90);
        assert!(left_px[1] > 200 && left_px[0] < 50);
        assert!(right_px[1] > 200 && right_px[0] < 50);
    }

    fn contains_text_shape(shapes: &[egui::epaint::ClippedShape]) -> bool {
        fn check_shape(s: &egui::epaint::Shape) -> bool {
            match s {
                egui::epaint::Shape::Text(_) => true,
                egui::epaint::Shape::Vec(v) => v.iter().any(check_shape),
                _ => false,
            }
        }
        shapes.iter().any(|cs| check_shape(&cs.shape))
    }

    #[test]
    fn test_headless_carousel_renders_with_zero_text_shapes() {
        let ctx = egui::Context::default();
        let items: Vec<WallpaperItem> = (0..7)
            .map(|i| WallpaperItem {
                path: PathBuf::from(format!("/tmp/wall_{i}.png")),
                thumb_path: PathBuf::from(format!("/tmp/thumb_{i}.png")),
                preview_path: PathBuf::from(format!("/tmp/prev_{i}.jpg")),
            })
            .collect();

        let mut app = WallpaperApp::new(items, 3);
        app.intro_anim = 1.0;
        let dummy = egui::ColorImage::new([16, 9], vec![egui::Color32::from_rgb(40, 80, 160); 144]);
        for (i, it) in app.items.iter().enumerate() {
            let tex = ctx.load_texture(format!("tb_{i}"), dummy.clone(), Default::default());
            app.thumb_textures
                .insert(it.thumb_path.clone(), tex.clone());
            app.bg_textures.insert(it.path.clone(), tex);
        }

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(1920.0, 1080.0),
            )),
            ..Default::default()
        };

        let mut full_output = ctx.run_ui(raw_input.clone(), |ui| {
            let inner_ctx = ui.ctx().clone();
            let screen_rect = ui.max_rect();
            let painter = inner_ctx.layer_painter(egui::LayerId::background());
            app.step_animations(&inner_ctx);
            app.render_background(&painter, screen_rect);
            app.render_carousel(ui, &inner_ctx, &painter, screen_rect);
        });
        full_output.textures_delta.clear();

        assert!(
            !full_output.shapes.is_empty(),
            "Expected background, carousel cards, and theme swatches to be painted"
        );
        assert!(
            !contains_text_shape(&full_output.shapes),
            "Wallpaper selector must not render any text shapes"
        );

        // Also verify mid-scroll state (scroll_transition = 0.5) renders swatches and zero text shapes
        app.scroll_transition = 0.5;
        let mut mid_scroll_output = ctx.run_ui(raw_input, |ui| {
            let inner_ctx = ui.ctx().clone();
            let screen_rect = ui.max_rect();
            let painter = inner_ctx.layer_painter(egui::LayerId::background());
            app.render_background(&painter, screen_rect);
            app.render_carousel(ui, &inner_ctx, &painter, screen_rect);
        });
        mid_scroll_output.textures_delta.clear();
        assert!(
            mid_scroll_output.shapes.len() >= 20,
            "Mid-scroll state should still render all visible cards and the 4 theme swatches"
        );
        assert!(
            !contains_text_shape(&mid_scroll_output.shapes),
            "Mid-scroll state must not render any text shapes"
        );
    }
}
