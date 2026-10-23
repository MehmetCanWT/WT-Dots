mod btop;
mod keybinds;
mod theme;
mod wallpaper;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: wt-rust <wallpaper|theme|keybinds|btop-stats>");
        return;
    }

    match args[1].as_str() {
        "wallpaper" => wallpaper::handle_wallpaper(&args[2..]),
        "theme" => theme::handle_theme(&args[2..]),
        "keybinds" => keybinds::handle_keybinds(&args[2..]),
        "btop-stats" => btop::handle_stats(),
        _ => eprintln!("Unknown command: {}", args[1]),
    }
}
