mod keybinds;
mod theme;
mod wallpaper;
mod waybar;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: wt-rust <gpu|workspaces|brightness|wallpaper|theme|keybinds>");
        return;
    }

    match args[1].as_str() {
        "gpu" => waybar::get_gpu(),
        "workspaces" => waybar::handle_workspaces(args.get(2).map(|s| s.as_str())),
        "brightness" => waybar::handle_brightness(args.get(2).map(|s| s.as_str())),
        "wallpaper" => wallpaper::handle_wallpaper(&args[2..]),
        "theme" => theme::handle_theme(&args[2..]),
        "keybinds" => keybinds::sync_keybinds(),
        _ => eprintln!("Unknown command: {}", args[1]),
    }
}
