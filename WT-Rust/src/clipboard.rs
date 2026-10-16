use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use crate::theme::{Palette, load_current_palette};

#[derive(Clone, Debug)]
pub struct ClipItem {
    pub raw_line: String,
    pub id: u64,
    pub is_image: bool,
    pub display_text: String,
    pub details: String,
}

pub struct DecodedThumb {
    pub id: u64,
    pub image: egui::ColorImage,
}

pub struct ClipboardApp {
    items: Vec<ClipItem>,
    filtered_indices: Vec<usize>,
    search_query: String,
    selected_index: usize,
    palette: Palette,
    textures: HashMap<u64, egui::TextureHandle>,
    loading_ids: HashSet<u64>,
    thumb_tx: Sender<DecodedThumb>,
    thumb_rx: Receiver<DecodedThumb>,
    frame_count: u32,
    should_close: bool,
    search_focused: bool,
}

impl ClipboardApp {
    pub fn new(items: Vec<ClipItem>, palette: Palette) -> Self {
        let (thumb_tx, thumb_rx) = channel();
        let filtered_indices = (0..items.len()).collect();
        Self {
            items,
            filtered_indices,
            search_query: String::new(),
            selected_index: 0,
            palette,
            textures: HashMap::new(),
            loading_ids: HashSet::new(),
            thumb_tx,
            thumb_rx,
            frame_count: 0,
            should_close: false,
            search_focused: false,
        }
    }

    fn filter_items(&mut self) {
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if item.is_image {
                        q.contains("resim")
                            || q.contains("görsel")
                            || q.contains("gorsel")
                            || q.contains("image")
                            || q.contains("png")
                            || q.contains("jpg")
                            || item.details.to_lowercase().contains(&q)
                    } else {
                        item.display_text.to_lowercase().contains(&q)
                            || item.details.to_lowercase().contains(&q)
                    }
                })
                .map(|(idx, _)| idx)
                .collect();
        }

        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = 0;
        }
    }

    fn copy_item_and_exit(&mut self, item: &ClipItem) {
        copy_clip_item(item);
        self.should_close = true;
    }

    fn poll_thumbnails(&mut self, ctx: &egui::Context) {
        while let Ok(thumb) = self.thumb_rx.try_recv() {
            let handle = ctx.load_texture(
                format!("clip_thumb_{}", thumb.id),
                thumb.image,
                egui::TextureOptions::LINEAR,
            );
            self.textures.insert(thumb.id, handle);
            self.loading_ids.remove(&thumb.id);
            ctx.request_repaint();
        }
    }
}

impl eframe::App for ClipboardApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frame_count = self.frame_count.saturating_add(1);
        self.poll_thumbnails(&ctx);

        if self.should_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Close on escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Close when focus is lost after initial startup grace period
        if self.frame_count > 15 && ctx.input(|i| !i.focused) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Keyboard navigation
        let nav_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
        let nav_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
        let press_enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));

        if nav_down && !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
        }
        if nav_up && !self.filtered_indices.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_indices.len().saturating_sub(1);
            } else {
                self.selected_index -= 1;
            }
        }

        if press_enter && !self.filtered_indices.is_empty() {
            let actual_idx = self.filtered_indices[self.selected_index];
            let item = self.items[actual_idx].clone();
            self.copy_item_and_exit(&item);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let (ar, ag, ab) = self.palette.accent_rgb;
        let accent_color = egui::Color32::from_rgb(ar, ag, ab);
        let accent_border = egui::Color32::from_rgba_unmultiplied(ar, ag, ab, 90);
        let accent_subtle = egui::Color32::from_rgba_unmultiplied(ar, ag, ab, 45);
        let bg_color = egui::Color32::from_rgb(17, 17, 22);
        let text_main = egui::Color32::from_rgb(240, 242, 237);
        let text_muted = egui::Color32::from_rgb(130, 142, 120);

        ui.style_mut().interaction.selectable_labels = false;

        let container_frame = egui::Frame::new()
            .fill(bg_color)
            .corner_radius(12.0)
            .stroke(egui::Stroke::new(1.2, accent_border))
            .inner_margin(egui::Margin::same(10));

        container_frame.show(ui, |ui| {
            ui.style_mut().interaction.selectable_labels = false;

            // 1. Header with search input
            ui.horizontal(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new("󰍉")
                            .color(accent_color)
                            .size(16.0),
                    )
                    .selectable(false),
                );

                let search_id = egui::IdSalt::new("clip_search_input");
                let search_response = ui.add_sized(
                    [ui.available_width() - 70.0, 26.0],
                    egui::TextEdit::singleline(&mut self.search_query)
                        .id_salt(search_id)
                        .hint_text(
                            egui::RichText::new("Pano geçmişinde ara...")
                                .color(text_muted)
                                .size(13.0),
                        )
                        .text_color(text_main)
                        .font(egui::FontId::monospace(13.0))
                        .margin(egui::Margin::symmetric(8, 4)),
                );

                if !self.search_focused {
                    search_response.request_focus();
                    self.search_focused = true;
                }

                if search_response.changed() {
                    self.filter_items();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{} öge", self.filtered_indices.len()))
                                .color(text_muted)
                                .size(11.0),
                        )
                        .selectable(false),
                    );
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // 2. Clipboard list view
            let mut clicked_item: Option<ClipItem> = None;

            // Pre-request thumbnails for visible image items
            for &idx in &self.filtered_indices {
                let it = &self.items[idx];
                if it.is_image
                    && !self.textures.contains_key(&it.id)
                    && !self.loading_ids.contains(&it.id)
                {
                    self.loading_ids.insert(it.id);
                    let tx = self.thumb_tx.clone();
                    let raw_line = it.raw_line.clone();
                    let id = it.id;
                    thread::spawn(move || {
                        if let Some(bytes) = decode_cliphist(&raw_line)
                            && let Ok(dyn_img) = image::load_from_memory(&bytes)
                        {
                            let thumb = dyn_img.thumbnail(120, 80);
                            let rgba = thumb.to_rgba8();
                            let size = [thumb.width() as usize, thumb.height() as usize];
                            let color_img = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                            let _ = tx.send(DecodedThumb { id, image: color_img });
                        }
                    });
                }
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.filtered_indices.is_empty() {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("󰅍")
                                    .color(text_muted)
                                    .size(28.0),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Eşleşen pano ögesi bulunamadı")
                                    .color(text_muted)
                                    .size(13.0),
                            );
                        });
                        return;
                    }

                    for (display_idx, &actual_idx) in self.filtered_indices.iter().enumerate() {
                        let item = &self.items[actual_idx];
                        let is_selected = display_idx == self.selected_index;

                        let row_bg = if is_selected {
                            accent_subtle
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let row_frame = egui::Frame::new()
                            .fill(row_bg)
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(8, 6));

                        let response = row_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if item.is_image {
                                    if let Some(texture) = self.textures.get(&item.id) {
                                        let aspect = texture.size_vec2().x
                                            / texture.size_vec2().y.max(1.0);
                                        let thumb_h = 38.0;
                                        let thumb_w = (thumb_h * aspect).clamp(24.0, 68.0);
                                        ui.add(
                                            egui::Image::new(texture)
                                                .fit_to_exact_size(egui::vec2(thumb_w, thumb_h))
                                                .corner_radius(6.0),
                                        );
                                    } else {
                                        ui.add_sized(
                                            [42.0, 38.0],
                                            egui::Label::new(
                                                egui::RichText::new("󰋩")
                                                    .color(accent_color)
                                                    .size(20.0),
                                            ),
                                        );
                                    }

                                        let text_w = (ui.available_width() - 8.0).max(80.0);
                                        ui.vertical(|ui| {
                                            ui.set_max_width(text_w);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&item.display_text)
                                                        .color(accent_color)
                                                        .strong()
                                                        .size(13.0),
                                                )
                                                .selectable(false)
                                                .wrap_mode(egui::TextWrapMode::Wrap),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&item.details)
                                                        .color(text_muted)
                                                        .size(11.0),
                                                )
                                                .selectable(false)
                                                .wrap_mode(egui::TextWrapMode::Truncate),
                                            );
                                        });
                                    } else {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new("󰅍")
                                                    .color(if is_selected {
                                                        accent_color
                                                    } else {
                                                        text_muted
                                                    })
                                                    .size(15.0),
                                            )
                                            .selectable(false),
                                        );

                                        let text_w = (ui.available_width() - 8.0).max(80.0);
                                        ui.vertical(|ui| {
                                            ui.set_max_width(text_w);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&item.display_text)
                                                        .color(text_main)
                                                        .font(egui::FontId::monospace(12.5)),
                                                )
                                                .selectable(false)
                                                .wrap_mode(egui::TextWrapMode::Wrap),
                                            );
                                            if !item.details.is_empty() {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(&item.details)
                                                            .color(text_muted)
                                                            .size(10.5),
                                                    )
                                                    .selectable(false)
                                                    .wrap_mode(egui::TextWrapMode::Truncate),
                                                );
                                            }
                                        });
                                    }
                            });
                        });

                        let row_rect = response.response.rect;
                        let interact = ui.interact(
                            row_rect,
                            ui.scope_id().with(actual_idx),
                            egui::Sense::click(),
                        );

                        if interact.hovered() && !is_selected {
                            ui.painter().rect_filled(
                                row_rect,
                                8.0,
                                egui::Color32::from_rgba_unmultiplied(ar, ag, ab, 20),
                            );
                        }

                        if interact.clicked() {
                            clicked_item = Some(item.clone());
                        }
                    }
                });

            if let Some(item) = clicked_item {
                self.copy_item_and_exit(&item);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // 3. Footer shortcuts hint
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new("↵ Kopyala")
                            .color(accent_color)
                            .size(10.5),
                    )
                    .selectable(false),
                );
                ui.add(egui::Label::new(egui::RichText::new("·").color(text_muted).size(10.5)).selectable(false));
                ui.add(
                    egui::Label::new(
                        egui::RichText::new("↑↓ Gezin")
                            .color(text_muted)
                            .size(10.5),
                    )
                    .selectable(false),
                );
                ui.add(egui::Label::new(egui::RichText::new("·").color(text_muted).size(10.5)).selectable(false));
                ui.add(
                    egui::Label::new(
                        egui::RichText::new("Esc Kapat")
                            .color(text_muted)
                            .size(10.5),
                    )
                    .selectable(false),
                );
            });
        });
    }
}

pub fn format_preview_max_lines(raw: &str, max_chars_per_line: usize, max_lines: usize) -> String {
    let clean = raw.trim().replace(['\r', '\t'], " ");
    let flat = clean.replace('\n', " ");
    let trimmed = flat.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut result_lines = Vec::new();
    let mut remaining = trimmed;

    while !remaining.is_empty() && result_lines.len() < max_lines {
        let char_count = remaining.chars().count();
        if char_count <= max_chars_per_line {
            result_lines.push(remaining.to_string());
            break;
        }

        // On the final allowed line, cut off and append ellipsis
        if result_lines.len() == max_lines - 1 {
            let cutoff = max_chars_per_line.saturating_sub(3);
            let chunk: String = remaining.chars().take(cutoff).collect();
            result_lines.push(format!("{}...", chunk.trim_end()));
            break;
        }

        // Search for a word boundary before max_chars_per_line
        let line_chars: Vec<char> = remaining.chars().take(max_chars_per_line).collect();
        let last_space = line_chars.iter().rposition(|&c| c == ' ');

        let split_char_idx = match last_space {
            Some(idx) if idx > 12 => idx,
            _ => max_chars_per_line,
        };

        let chunk: String = remaining.chars().take(split_char_idx).collect();
        result_lines.push(chunk.trim().to_string());

        let skip_chars = split_char_idx + if remaining.chars().nth(split_char_idx) == Some(' ') { 1 } else { 0 };
        remaining = remaining.char_indices().nth(skip_chars).map(|(i, _)| &remaining[i..]).unwrap_or("");
    }

    result_lines.join("\n")
}

pub fn parse_cliphist_list(output: &str) -> Vec<ClipItem> {
    let mut items = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, '\t');
        let id_str = parts.next().unwrap_or("0");
        let content = parts.next().unwrap_or("");
        let id = id_str.trim().parse::<u64>().unwrap_or(0);

        if content.starts_with("[[ binary data ") {
            let details = content
                .trim_start_matches("[[ binary data ")
                .trim_end_matches(" ]]")
                .trim()
                .to_string();
            items.push(ClipItem {
                raw_line: trimmed.to_string(),
                id,
                is_image: true,
                display_text: "Görsel (Ekran / Resim)".to_string(),
                details,
            });
        } else {
            let preview = format_preview_max_lines(content, 34, 2);
            let char_count = content.chars().count();
            let details = if char_count > 60 {
                format!("{} karakter", char_count)
            } else {
                String::new()
            };

            items.push(ClipItem {
                raw_line: trimmed.to_string(),
                id,
                is_image: false,
                display_text: preview,
                details,
            });
        }
    }

    items
}

pub fn fetch_clipboard_history() -> Vec<ClipItem> {
    let output = Command::new("cliphist").arg("list").output();

    if let Ok(out) = output
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        let items = parse_cliphist_list(&text);
        if !items.is_empty() {
            return items;
        }
    }

    // Fallback: Check KDE Klipper sqlite directly if cliphist is empty
    fetch_klipper_sqlite_history()
}

fn fetch_klipper_sqlite_history() -> Vec<ClipItem> {
    let home = std::env::var("HOME").unwrap_or_default();
    let db_path = std::path::Path::new(&home).join(".local/share/klipper/history3.sqlite");
    if !db_path.exists() {
        return Vec::new();
    }

    let output = Command::new("sqlite3")
        .args([
            db_path.to_str().unwrap_or(""),
            "SELECT substr(text, 1, 100), length(text) FROM main WHERE text IS NOT NULL AND text != '' ORDER BY last_used_time DESC LIMIT 40;",
        ])
        .output();

    let mut items = Vec::new();
    if let Ok(out) = output
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
            for (idx, line) in text.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mut parts = trimmed.split('|');
                let content = parts.next().unwrap_or("").replace(['\n', '\r', '\t'], " ");
                let preview = format_preview_max_lines(&content, 34, 2);
                let len_str = parts.next().unwrap_or("");
                let details = if let Ok(l) = len_str.parse::<usize>() {
                    if l > 60 {
                        format!("{} karakter", l)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                items.push(ClipItem {
                    raw_line: content.clone(),
                    id: (idx + 1) as u64,
                    is_image: false,
                    display_text: preview,
                    details,
                });
            }
        }

    items
}

pub fn decode_cliphist(raw_line: &str) -> Option<Vec<u8>> {
    let mut child = Command::new("cliphist")
        .arg("decode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(raw_line.as_bytes());
    }

    let output = child.wait_with_output().ok()?;
    if output.status.success() {
        Some(output.stdout)
    } else {
        None
    }
}

pub fn copy_clip_item(item: &ClipItem) {
    if item.is_image {
        if let Some(bytes) = decode_cliphist(&item.raw_line) {
            let child = Command::new("wl-copy")
                .args(["--type", "image/png"])
                .stdin(Stdio::piped())
                .spawn();
            if let Ok(mut c) = child {
                if let Some(mut stdin) = c.stdin.take() {
                    let _ = stdin.write_all(&bytes);
                }
                let _ = c.wait();
            }
        }
    } else {
        let text_bytes = if let Some(decoded) = decode_cliphist(&item.raw_line) {
            decoded
        } else {
            item.raw_line.as_bytes().to_vec()
        };

        let child = Command::new("wl-copy").stdin(Stdio::piped()).spawn();
        if let Ok(mut c) = child {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(&text_bytes);
            }
            let _ = c.wait();
        }
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let font_candidates = [
        "/home/mehmetcan/.fonts/0xProtoNerdFont-Regular.ttf",
        "/home/mehmetcan/.local/share/fonts/0/0xProtoNerdFont_Regular.ttf",
    ];

    for path in font_candidates {
        if let Ok(font_data) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "0xProtoNerdFont".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(font_data)),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "0xProtoNerdFont".to_owned());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "0xProtoNerdFont".to_owned());

            ctx.set_fonts(fonts);
            break;
        }
    }
}

pub fn open_clipboard_menu() {
    let items = fetch_clipboard_history();
    let palette = load_current_palette();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 480.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_title("wt-clipboard"),
        ..Default::default()
    };

    let app = ClipboardApp::new(items, palette);
    let _ = eframe::run_native(
        "wt-clipboard",
        native_options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    );
}

pub fn handle_clipboard(args: &[String]) {
    if args.is_empty() || args[0] == "menu" {
        open_clipboard_menu();
    } else if args[0] == "list" {
        for item in fetch_clipboard_history() {
            println!("{}\t{}", item.id, item.display_text);
        }
    } else {
        eprintln!("Usage: wt-rust clipboard [menu|list]");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cliphist_text_and_image() {
        let input = "489\ttest clipboard copy 12345\n488\t[[ binary data 186 KiB png 1920x1080 ]]\n";
        let items = parse_cliphist_list(input);
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, 489);
        assert!(!items[0].is_image);
        assert_eq!(items[0].display_text, "test clipboard copy 12345");

        assert_eq!(items[1].id, 488);
        assert!(items[1].is_image);
        assert_eq!(items[1].display_text, "Görsel (Ekran / Resim)");
        assert_eq!(items[1].details, "186 KiB png 1920x1080");
    }

    #[test]
    fn test_filter_clipboard_search() {
        let input = "1\tfirst note\n2\tsecond cargo build\n3\t[[ binary data 12 KiB png 200x200 ]]\n";
        let items = parse_cliphist_list(input);
        let mut app = ClipboardApp::new(items, crate::theme::default_palette());

        assert_eq!(app.filtered_indices.len(), 3);

        app.search_query = "cargo".to_string();
        app.filter_items();
        assert_eq!(app.filtered_indices.len(), 1);
        assert_eq!(app.filtered_indices[0], 1);

        app.search_query = "görsel".to_string();
        app.filter_items();
        assert_eq!(app.filtered_indices.len(), 1);
        assert_eq!(app.filtered_indices[0], 2);
    }

    #[test]
    fn test_format_preview_max_lines() {
        let short = "simple text";
        assert_eq!(format_preview_max_lines(short, 40, 2), "simple text");

        let long = "this is a very long line that exceeds forty characters and should definitely wrap to line two and truncate";
        let formatted = format_preview_max_lines(long, 40, 2);
        let lines: Vec<&str> = formatted.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with("..."));
    }
}
