mod audio;
mod keybinds;
mod power;
mod sysinfo;
mod theme;
mod wallpaper;
mod waybar;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: wt-rust <gpu|workspaces|brightness|wallpaper|theme|keybinds|volume|power|sysinfo>"
        );
        return;
    }

    match args[1].as_str() {
        "gpu" => waybar::get_gpu(),
        "workspaces" => waybar::handle_workspaces(args.get(2).map(|s| s.as_str())),
        "brightness" => waybar::handle_brightness(args.get(2).map(|s| s.as_str())),
        "wallpaper" => wallpaper::handle_wallpaper(&args[2..]),
        "theme" => theme::handle_theme(&args[2..]),
        "keybinds" => keybinds::handle_keybinds(&args[2..]),
        "volume" => audio::handle_volume(&args[2..]),
        "power" => power::handle_power(&args[2..]),
        "sysinfo" => sysinfo::handle_sysinfo(&args[2..]),
        _ => eprintln!("Unknown command: {}", args[1]),
    }
}
