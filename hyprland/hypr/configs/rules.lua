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

-- Pure-Rust Volume & Wi-Fi popup anchored top-right flush beneath Waybar
hl.window_rule({
    name      = "wt-popup-overlay",
    match     = { class = "^(wt-popup)$" },
    float     = true,
    pin       = true,
    no_shadow = true,
    move      = "(monitor_w-window_w-12) 34",
})

-- Floating btop++ window launched from Waybar CPU/RAM indicator
hl.window_rule({
    name   = "wt-btop-monitor",
    match  = { class = "^(wt-btop)$" },
    float  = true,
    size   = "980 650",
    center = true,
})
