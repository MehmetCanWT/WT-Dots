-- Hyprland Window & Layer Rules

hl.window_rule({
    name  = "suppress-maximize-events",
    match = { class = ".*" },
    suppress_event = "maximize",
})

hl.window_rule({
    name = "fix-xwayland-drags",
    match = {
        class      = "^$",
        title      = "^$",
        xwayland   = true,
        float      = true,
        fullscreen = false,
        pin        = false,
    },
    no_focus = true,
})

-- Floating utility windows
hl.window_rule({
    name  = "float-pavucontrol",
    match = { class = "^(org\\.pulseaudio\\.pavucontrol|pavucontrol)$" },
    float = true,
    size  = "800 500",
    center = true,
})

hl.window_rule({
    name  = "float-nm-editor",
    match = { class = "^(nm-connection-editor)$" },
    float = true,
    center = true,
})

hl.window_rule({
    name  = "float-polkit-agent",
    match = { class = "^(hyprpolkitagent|org\\.kde\\.polkit-kde-authentication-agent-1)$" },
    float = true,
    center = true,
})

hl.window_rule({
    name  = "float-file-dialogs",
    match = { title = "^(Open File|Save File|Select Folder|Dosya Aç|Farklı Kaydet)$" },
    float = true,
    center = true,
})

-- Pure-Rust Volume & Wi-Fi popup anchored top-right cleanly below Waybar
hl.window_rule({
    name        = "wt-popup-overlay",
    match       = { class = "^(wt-popup)$" },
    float       = true,
    pin         = true,
    no_shadow   = true,
    no_anim     = true,
    border_size = 0,
    rounding    = 16,
    move        = "monitor_w-window_w-12 42",
})

-- Pure-Rust Power menu anchored top-left cleanly below the Power button
hl.window_rule({
    name        = "wt-popup-power-overlay",
    match       = { class = "^(wt-popup-power)$" },
    float       = true,
    pin         = true,
    no_shadow   = true,
    no_anim     = true,
    border_size = 0,
    rounding    = 16,
    move        = "12 42",
})

-- Floating btop++ window launched from Waybar CPU/RAM indicator
hl.window_rule({
    name   = "wt-btop-monitor",
    match  = { class = "^(wt-btop)$" },
    float  = true,
    size   = "980 650",
    center = true,
})

-- All Steam / Proton / Linux Games: no blur/shadow/anim for smooth 144Hz PRIME frame pacing
hl.window_rule({
    name      = "games-low-latency",
    match     = { class = "^(steam_app_.*|cs2|dota2|gamescope|Minecraft.*|factorio)$" },
    no_anim   = true,
    no_blur   = true,
    no_shadow = true,
})
