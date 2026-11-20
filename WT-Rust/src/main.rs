mod btop;
mod common;
mod music;
mod popup;
mod theme;
mod wallpaper;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: wt-rust <wallpaper|theme|popup|music|btop-stats|battery>");
        return;
    }

    match args[1].as_str() {
        "wallpaper" => wallpaper::handle_wallpaper(&args[2..]),
        "theme" => theme::handle_theme(&args[2..]),
        "popup" => popup::handle_popup(&args[2..]),
        "btop-stats" => btop::handle_stats(&args[2..]),
        "battery" => popup::handle_battery_waybar(),
        "music" => music::handle_music(&args[2..]),
        _ => eprintln!("Unknown command: {}", args[1]),
    }
}
