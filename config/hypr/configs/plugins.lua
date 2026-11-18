-- Hyprland Plugin Configurations (loaded via `hyprpm reload -n`)

hl.config({
    plugin = {
        hypr_edgehover = {
            enabled             = 1,
            edges               = "lrtb",
            inset               = 1,
            max_distance        = 0,
            keyboard_focus      = -1,
            gap_pass            = "hover,click,scroll,keyboard",
            layer_pass          = "hover,keyboard",
            layer_namespaces    = "",
            overhang_pass       = "hover,keyboard",
            overhang_threshold  = 8,
            overhang_edge_width = 0,
            steal_edge_width    = 2,
            zones_top           = "0-100",
            zones_bottom        = "0-100",
            zones_left          = "0-100",
            zones_right         = "0-100",
        },
    },
})

if hl.plugin.confined_floats ~= nil then
    hl.window_rule({
        match = { float = true },
        ["confined-floats:confine"] = true,
    })
end

if hl.plugin.hyprcapture ~= nil then
    hl.config({
        plugin = {
            hyprcapture = {
                screenshot_notification = false,
            },
        },
    })
end
