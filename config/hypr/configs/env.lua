-- Environment Variables

local d = rawget(_G, "WT_DEFAULTS") or {}

hl.env("BROWSER", d.browser or "zen-browser")

-- Dual-GPU laptop: force Intel iGPU (00:02.0) as primary compositor GPU, NVIDIA dGPU (01:00.0) as secondary fallback
hl.env("AQ_DRM_DEVICES", "/dev/dri/by-path/pci-0000:00:02.0-card:/dev/dri/by-path/pci-0000:01:00.0-card")

hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_SIZE", "24")
hl.env("QT_QPA_PLATFORM", "wayland;xcb")
hl.env("QT_QPA_PLATFORMTHEME", "kde")
hl.env("MOZ_ENABLE_WAYLAND", "1")
hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")
