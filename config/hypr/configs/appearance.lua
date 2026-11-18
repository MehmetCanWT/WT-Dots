-- General, Decoration, Layout & Core Compositor Settings

hl.config({
    general = {
        gaps_in     = 5,
        gaps_out    = 10,
        border_size = 2,

        col = {
            active_border   = { colors = { "rgba(89b4faee)", "rgba(cba6f7ee)" }, angle = 45 },
            inactive_border = "rgba(45475aaa)",
        },

        resize_on_border = true,
        allow_tearing    = false,
        layout           = "dwindle",

        snap = {
            enabled      = true,
            respect_gaps = true,
            monitor_gap  = 10,
            window_gap   = 10,
        },
    },

    decoration = {
        rounding       = 10,
        rounding_power = 2,

        active_opacity   = 1.0,
        inactive_opacity = 0.95,

        shadow = {
            enabled      = true,
            range        = 8,
            render_power = 3,
            color        = 0xaa11111b,
        },

        blur = {
            enabled  = true,
            size     = 5,
            passes   = 2,
            vibrancy = 0.17,
        },
    },

    dwindle = {
        preserve_split = true,
    },

    master = {
        new_status = "master",
    },

    misc = {
        force_default_wallpaper  = 0,
        disable_hyprland_logo    = true,
        disable_splash_rendering = true,
        background_color         = 0x11111b,
        middle_click_paste       = true,
    },

    layers = {
        allow_new_top_layers_over_existing_fullscreen = true,
    },

    render = {
        direct_scanout = 0,
    },

    xwayland = {
        force_zero_scaling = true,
    },

    cursor = {
        no_hardware_cursors = 2,
    },

    debug = {
        vfr = true,
    },
})
