-- Hyprland Keybindings (defaults sourced from WT_DEFAULTS at top of hyprland.lua)

local d           = rawget(_G, "WT_DEFAULTS") or {}
local mainMod     = d.mod or "SUPER"
local terminal    = d.terminal or "ghostty"
local fileManager = d.file_manager or "dolphin"
local browser     = d.browser or "zen-browser"
local launcher    = d.launcher or "pkill rofi || rofi -show drun"
local wallpaper   = d.wallpaper_picker or "wt-rust wallpaper select"

-- Applications & System
hl.bind(mainMod .. " + space",  hl.dsp.exec_cmd(launcher))
hl.bind(mainMod .. " + Return", hl.dsp.exec_cmd(terminal))
hl.bind(mainMod .. " + E",      hl.dsp.exec_cmd(fileManager))
hl.bind(mainMod .. " + B",      hl.dsp.exec_cmd(browser))
hl.bind(mainMod .. " + W",      hl.dsp.exec_cmd(wallpaper))
hl.bind(mainMod .. " + V",      hl.dsp.exec_cmd("~/.config/hypr/scripts/cliphist-rofi.sh"))
hl.bind(mainMod .. " + L",      hl.dsp.exec_cmd("hyprlock"))
hl.bind(mainMod .. " + M",      hl.dsp.exec_cmd("hyprshutdown -t 'Logging out...'"))

-- Window Management
hl.bind(mainMod .. " + Q", hl.dsp.window.close())
hl.bind(mainMod .. " + C", hl.dsp.window.float({ action = "toggle" }))
hl.bind(mainMod .. " + F", hl.dsp.window.fullscreen(0))
hl.bind(mainMod .. " + P", hl.dsp.window.pseudo())
hl.bind(mainMod .. " + J", hl.dsp.layout("togglesplit"))

-- Focus Movement (Arrow Keys)
hl.bind(mainMod .. " + left",  hl.dsp.focus({ direction = "left" }))
hl.bind(mainMod .. " + right", hl.dsp.focus({ direction = "right" }))
hl.bind(mainMod .. " + up",    hl.dsp.focus({ direction = "up" }))
hl.bind(mainMod .. " + down",  hl.dsp.focus({ direction = "down" }))

-- Swap Windows (ALT + SHIFT + Arrow Keys)
hl.bind(mainMod .. " + SHIFT + left",  hl.dsp.window.swap({ direction = "left" }))
hl.bind(mainMod .. " + SHIFT + right", hl.dsp.window.swap({ direction = "right" }))
hl.bind(mainMod .. " + SHIFT + up",    hl.dsp.window.swap({ direction = "up" }))
hl.bind(mainMod .. " + SHIFT + down",  hl.dsp.window.swap({ direction = "down" }))

-- Workspaces (1-10)
for i = 1, 10 do
    local key = i % 10
    hl.bind(mainMod .. " + " .. key,         hl.dsp.focus({ workspace = i }))
    hl.bind(mainMod .. " + SHIFT + " .. key, hl.dsp.window.move({ workspace = i }))
end

-- Scratchpad (Special Workspace)
hl.bind(mainMod .. " + S",         hl.dsp.workspace.toggle_special("magic"))
hl.bind(mainMod .. " + SHIFT + S", hl.dsp.window.move({ workspace = "special:magic" }))

-- Show Desktop Toggle (SUPER + D)
hl.bind(mainMod .. " + D", hl.dsp.exec_cmd("~/.config/hypr/scripts/show-desktop.sh"))

-- Scroll through workspaces
hl.bind(mainMod .. " + mouse_down", hl.dsp.focus({ workspace = "e+1" }))
hl.bind(mainMod .. " + mouse_up",   hl.dsp.focus({ workspace = "e-1" }))

-- Mouse Move / Resize
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind(mainMod .. " + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- Screenshots (HyprCapture with fallback to screenshot.sh)
if hl.plugin.hyprcapture ~= nil then
    hl.bind("Print",         hl.plugin.hyprcapture.open_editor)
    hl.bind("SHIFT + Print", function() hl.plugin.hyprcapture.open("fullscreen") end)
    hl.bind("ALT + Print",   hl.plugin.hyprcapture.record_toggle)
else
    hl.bind("Print",         hl.dsp.exec_cmd("~/.config/hypr/scripts/screenshot.sh region"))
    hl.bind("SHIFT + Print", hl.dsp.exec_cmd("~/.config/hypr/scripts/screenshot.sh full"))
end

-- Audio & Brightness (Laptop Keys - strict 5% steps)
hl.bind("XF86AudioRaiseVolume",  hl.dsp.exec_cmd("~/.config/hypr/scripts/media-step.sh volume up"),     { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume",  hl.dsp.exec_cmd("~/.config/hypr/scripts/media-step.sh volume down"),   { locked = true, repeating = true })
hl.bind("XF86AudioMute",         hl.dsp.exec_cmd("~/.config/hypr/scripts/media-step.sh mute"),          { locked = true })
hl.bind("XF86AudioMicMute",      hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"),       { locked = true })
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("~/.config/hypr/scripts/media-step.sh brightness up"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("~/.config/hypr/scripts/media-step.sh brightness down"), { locked = true, repeating = true })

-- Media Controls
hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   { locked = true })

--------------------------------------------------------------------------------
-- EXAMPLE KEYBINDS (Commented out - Lua uses "--" for comments)
-- Uncomment any line below to enable it.
--------------------------------------------------------------------------------

-- -- Vim-style Focus (HJKL)
-- hl.bind(mainMod .. " + H", hl.dsp.focus({ direction = "left" }))
-- hl.bind(mainMod .. " + L", hl.dsp.focus({ direction = "right" }))
-- hl.bind(mainMod .. " + K", hl.dsp.focus({ direction = "up" }))
-- hl.bind(mainMod .. " + J", hl.dsp.focus({ direction = "down" }))

-- -- Vim-style Window Swap (ALT + SHIFT + HJKL)
-- hl.bind(mainMod .. " + SHIFT + H", hl.dsp.window.swap({ direction = "left" }))
-- hl.bind(mainMod .. " + SHIFT + L", hl.dsp.window.swap({ direction = "right" }))
-- hl.bind(mainMod .. " + SHIFT + K", hl.dsp.window.swap({ direction = "up" }))
-- hl.bind(mainMod .. " + SHIFT + J", hl.dsp.window.swap({ direction = "down" }))

-- -- Resize Active Window with Keyboard (ALT + CTRL + Arrows)
-- hl.bind(mainMod .. " + CTRL + left",  hl.dsp.window.resize({ x = -40, y = 0 }), { repeating = true })
-- hl.bind(mainMod .. " + CTRL + right", hl.dsp.window.resize({ x = 40,  y = 0 }), { repeating = true })
-- hl.bind(mainMod .. " + CTRL + up",    hl.dsp.window.resize({ x = 0,   y = -40 }), { repeating = true })
-- hl.bind(mainMod .. " + CTRL + down",  hl.dsp.window.resize({ x = 0,   y = 40 }), { repeating = true })

-- -- Window Extras (Center, Pin, Kill, Cycle)
-- hl.bind(mainMod .. " + SHIFT + C", hl.dsp.window.center())                  -- Center floating window
-- hl.bind(mainMod .. " + SHIFT + P", hl.dsp.window.pin())                     -- Pin floating window across workspaces
-- hl.bind(mainMod .. " + SHIFT + Q", hl.dsp.window.kill())                    -- Force kill unresponsive window
-- hl.bind("ALT + Tab",               hl.dsp.window.cycle_next())              -- Cycle through windows
-- hl.bind("ALT + SHIFT + Tab",       hl.dsp.window.cycle_next({ prev = true }))

-- -- Window Groups / Tabs
-- hl.bind(mainMod .. " + G",         hl.dsp.group.toggle())                   -- Toggle window group (tabbed)
-- hl.bind(mainMod .. " + Tab",       hl.dsp.group.next())                     -- Next tab in group
-- hl.bind(mainMod .. " + SHIFT + Tab", hl.dsp.group.prev())                   -- Previous tab in group
-- hl.bind(mainMod .. " + SHIFT + G", hl.dsp.group.lock_active())              -- Lock/unlock active group

-- -- Workspace Navigation
-- hl.bind(mainMod .. " + BracketRight", hl.dsp.focus({ workspace = "e+1" }))  -- Next non-empty workspace
-- hl.bind(mainMod .. " + BracketLeft",  hl.dsp.focus({ workspace = "e-1" }))  -- Previous non-empty workspace
-- hl.bind(mainMod .. " + Tab",          hl.dsp.focus({ workspace = "previous" })) -- Toggle last workspace

-- -- Hyprland & Bar Controls
-- hl.bind(mainMod .. " + SHIFT + R", hl.dsp.reload_config())                  -- Reload Hyprland config
-- hl.bind(mainMod .. " + SHIFT + W", hl.dsp.exec_cmd("pkill -SIGUSR2 waybar")) -- Reload Waybar CSS/Config
-- hl.bind(mainMod .. " + W",         hl.dsp.exec_cmd("pkill waybar || waybar")) -- Toggle Waybar visibility

-- -- Useful Tools & Apps (install if needed: cliphist, hyprpicker, btop, etc.)
-- hl.bind(mainMod .. " + T",         hl.dsp.exec_cmd("ghostty -e btop"))        -- System monitor
-- hl.bind(mainMod .. " + N",         hl.dsp.exec_cmd("makoctl dismiss -a"))   -- Dismiss all notifications
-- hl.bind(mainMod .. " + SHIFT + V", hl.dsp.exec_cmd("cliphist list | fuzzel --dmenu | cliphist decode | wl-copy")) -- Clipboard history
-- hl.bind(mainMod .. " + SHIFT + X", hl.dsp.exec_cmd("hyprpicker -a"))        -- Color picker to clipboard
-- hl.bind(mainMod .. " + SHIFT + M", hl.dsp.exec_cmd("systemctl suspend"))    -- Suspend PC

