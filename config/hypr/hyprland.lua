-- Hyprland 0.57+ Native Lua Configuration
-- Modularized under ~/.config/hypr/configs/

-----------------------------
---- DEFAULT APPLICATIONS ---
-----------------------------

WT_DEFAULTS = {
    mod              = "SUPER",
    terminal         = "ghostty",
    file_manager     = "dolphin",
    browser          = "zen-browser",
    launcher         = "pkill rofi || rofi -show drun",
    wallpaper_picker = "wt-rust wallpaper select",
}

-----------------
---- MODULES ----
-----------------

require("configs.monitors")
require("configs.env")
require("configs.autostart")
require("configs.appearance")
require("configs.input")
require("configs.animations")
require("configs.keybinds")
require("configs.rules")
require("configs.theme")
require("configs.plugins")
