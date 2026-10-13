#!/usr/bin/env python3
"""ricemood / node-vibrant / Material You inspired color extractor & theme applier."""
import colorsys, hashlib, json, math, os, subprocess, sys, time
from pathlib import Path
from PIL import Image

HOME = Path.home()
CACHE = HOME / ".cache/wallpaper-picker"
VIDEO = {".mp4", ".mkv", ".webm", ".mov", ".gif"}


def still(path):
    """Video ise ilk kare PNG, degilse yolun kendisi."""
    p = Path(path)
    if p.suffix.lower() not in VIDEO:
        return p
    out = CACHE / (hashlib.sha1(str(p).encode()).hexdigest() + ".frame.png")
    if not out.exists():
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", "1", "-i", str(p), "-frames:v", "1", str(out)])
        if not out.exists():
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(p), "-frames:v", "1", str(out)])
    return out


def rgb_to_hsl(r, g, b):
    mx, mn = max(r, g, b), min(r, g, b)
    l = (mx + mn) / 2.0
    if mx == mn:
        h = s = 0.0
    else:
        d = mx - mn
        s = d / (2.0 - mx - mn) if l > 0.5 else d / (mx + mn)
        if mx == r:
            h = (g - b) / d + (6.0 if g < b else 0.0)
        elif mx == g:
            h = (b - r) / d + 2.0
        else:
            h = (r - g) / d + 4.0
        h /= 6.0
    return h, s, l


def hsl_to_rgb(h, s, l):
    if s == 0:
        return l, l, l
    def hue2rgb(p, q, t):
        t = t % 1.0
        if t < 1/6: return p + (q - p) * 6 * t
        if t < 1/2: return q
        if t < 2/3: return p + (q - p) * (2/3 - t) * 6
        return p
    q = l * (1 + s) if l < 0.5 else l + s - l * s
    p = 2 * l - q
    return hue2rgb(p, q, h + 1/3), hue2rgb(p, q, h), hue2rgb(p, q, h - 1/3)


def rgb_to_hex(r, g, b):
    return "#%02x%02x%02x" % (round(r * 255), round(g * 255), round(b * 255))


def hex_to_hsl(hex_str):
    h = hex_str.lstrip("#")
    r, g, b = (int(h[i:i+2], 16) / 255.0 for i in (0, 2, 4))
    return rgb_to_hsl(r, g, b)


def hsl_to_hex(h, s, l):
    r, g, b = hsl_to_rgb(h, max(0.0, min(1.0, s)), max(0.0, min(1.0, l)))
    return rgb_to_hex(r, g, b)


def lighten(hex_str, amount):
    h, s, l = hex_to_hsl(hex_str)
    return hsl_to_hex(h, s, l + (amount / 100.0))


def darken(hex_str, amount):
    h, s, l = hex_to_hsl(hex_str)
    return hsl_to_hex(h, s, l - (amount / 100.0))


def normalize_accent(hex_str):
    """Normalize accent to ensure punchy saturation and optimal contrast on dark backgrounds."""
    h, s, l = hex_to_hsl(hex_str)
    if s < 0.16:
        # Monochrome / slate silver for grayscale wallpapers
        return hsl_to_hex(h, 0.12, 0.78)
    # Boost saturation for punchy UI
    s = min(0.95, max(0.55, s * 1.25))
    # Lightness clamped to [0.52, 0.62] for perfect contrast
    l = min(0.62, max(0.52, l))
    return hsl_to_hex(h, s, l)


def luminance(r, g, b):
    """WCAG 2.1 relative luminance."""
    def ch(c):
        return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)


def get_active_fg(accent_hex):
    """Calculate legible foreground (white or near-black) for accent backgrounds."""
    h = accent_hex.lstrip("#")
    r, g, b = (int(h[i:i+2], 16) / 255.0 for i in (0, 2, 4))
    lum = luminance(r, g, b)
    return "#ffffff" if lum < 0.45 else "#101216"


def closest_gnome_accent(h, s):
    """Map HSL hue to nearest GNOME 47 accent color."""
    if s < 0.18:
        return "slate"
    deg = (h * 360.0) % 360.0
    if deg < 15 or deg >= 345:
        return "red"
    elif deg < 45:
        return "orange"
    elif deg < 75:
        return "yellow"
    elif deg < 160:
        return "green"
    elif deg < 195:
        return "teal"
    elif deg < 265:
        return "blue"
    elif deg < 315:
        return "purple"
    else:
        return "pink"


def extract_vibrant_swatches(image_path):
    """Quantize to 64 swatches and select the best chromatic focal color."""
    im = Image.open(still(image_path)).convert("RGB")
    im.thumbnail((256, 256))
    q = im.quantize(64)
    pal = q.getpalette()
    colors = q.getcolors()

    swatches = []
    for cnt, idx in colors:
        r, g, b = (pal[idx * 3 + i] / 255.0 for i in range(3))
        h, s, l = rgb_to_hsl(r, g, b)
        swatches.append({'r': r, 'g': g, 'b': b, 'h': h, 's': s, 'l': l, 'cnt': cnt})

    # Discard extreme blacks, whites and purely desaturated grays
    chroma_candidates = [sw for sw in swatches if 0.12 <= sw['l'] <= 0.88 and sw['s'] >= 0.18]
    if chroma_candidates:
        max_log_pop = max(math.log1p(sw['cnt']) for sw in chroma_candidates)
        def score(sw):
            sat_score = sw['s']
            l_score = max(0.0, 1.0 - abs(sw['l'] - 0.54) * 2.0)
            pop_score = math.log1p(sw['cnt']) / max_log_pop if max_log_pop > 0 else 0
            return sat_score * 0.55 + l_score * 0.30 + pop_score * 0.15

        best = max(chroma_candidates, key=score)
        vibrant = rgb_to_hex(best['r'], best['g'], best['b'])
        dark_vibrant = hsl_to_hex(best['h'], min(1.0, best['s'] * 1.1), 0.22)
        light_vibrant = hsl_to_hex(best['h'], min(1.0, best['s'] * 1.1), 0.72)
    else:
        viable = [sw for sw in swatches if 0.20 <= sw['l'] <= 0.80]
        best = max(viable, key=lambda sw: sw['cnt']) if viable else swatches[0]
        vibrant = rgb_to_hex(best['r'], best['g'], best['b'])
        dark_vibrant = hsl_to_hex(best['h'], 0.10, 0.20)
        light_vibrant = hsl_to_hex(best['h'], 0.15, 0.75)

    return {
        'Vibrant': vibrant,
        'DarkVibrant': dark_vibrant,
        'LightVibrant': light_vibrant,
    }


def palette(path):
    key = hashlib.sha1(f"v5:{path}:{os.path.getmtime(path)}".encode()).hexdigest()
    cf = CACHE / (key + ".vibrant.json")
    if cf.exists():
        try:
            return json.loads(cf.read_text())
        except Exception:
            pass

    sw = extract_vibrant_swatches(path)

    # Base hue and saturation from image
    h, s, _ = hex_to_hsl(sw['DarkVibrant'] if 'DarkVibrant' in sw else sw['Vibrant'])
    # Deep solid dark theme background subtly tinted by the image
    bg = hsl_to_hex(h, min(0.20, s * 0.25), 0.09)
    bg2 = hsl_to_hex(h, min(0.25, s * 0.30), 0.15)
    # High-contrast readable foreground text (91% lightness)
    fg = hsl_to_hex(h, min(0.12, s * 0.15), 0.91)

    # Normalized vibrant accent with guaranteed contrast
    accent = normalize_accent(sw['Vibrant'])
    h_acc, s_acc, l_acc = hex_to_hsl(accent)
    accent2 = hsl_to_hex(h_acc, max(0.50, s_acc * 0.9), min(0.78, l_acc + 0.15))
    muted = hsl_to_hex(h, min(0.15, s * 0.20), 0.55)
    active_fg = get_active_fg(accent)
    urgent = "#e05f52"

    p = {
        "bg": bg,
        "bg2": bg2,
        "fg": fg,
        "muted": muted,
        "accent": accent,
        "accent2": accent2,
        "active_fg": active_fg,
        "urgent": urgent,
        "vibrant": sw['Vibrant'],
        "dark_vibrant": sw.get('DarkVibrant', bg),
        "light_vibrant": sw.get('LightVibrant', accent2),
    }
    cf.write_text(json.dumps(p))
    return p


def sh(cmd):
    subprocess.Popen(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)


def write(path, text):
    p = Path(path).expanduser()
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text)


def argb(c):
    return "0x" + c[1:] + "ff"


def reload_mango_config():
    """Dynamically resolve active MangoWM IPC socket and trigger reload."""
    env = os.environ.copy()
    try:
        out = subprocess.check_output(["pgrep", "-x", "mango"], text=True).strip().split()
        if out:
            sock = f"/run/user/{os.getuid()}/mango-{out[-1]}.sock"
            if os.path.exists(sock):
                env["MANGO_INSTANCE_SIGNATURE"] = sock
    except Exception:
        pass
    subprocess.run(["mmsg", "dispatch", "reload_config"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def apply(p, restart_bar=True):
    # mangobar: 3-island floating setup with solid dark color and readable active tag
    write("~/.config/mangobar/style.css", f"""* {{
    font-family: "0xProto Nerd Font";
    font-size: 13px;
    font-weight: normal;
    color: {p['fg']};
    background-color: {p['bg']};
    padding: 0px 8px;
    margin: 3px 1px;
    border-radius: 8px;
}}

#bar {{
    background: none;
    margin: 2px 8px;
}}

#bar.sel {{
    background: none;
}}

#custom-power {{
    padding: 0px 8px;
    margin: 3px 1px;
    background-color: {p['bg']};
    color: {p['urgent']};
}}

#tags {{
    padding: 0px 8px;
    margin: 3px 1px;
    background-color: {p['bg']};
}}

#tags.active {{
    color: {p['active_fg']};
    background-color: {p['accent']};
    font-weight: bold;
    border-radius: 6px;
    margin: 3px 1px;
}}

#tags.occupied {{
    color: {p['fg']};
    background-color: {p['bg']};
}}

#tags.empty {{
    color: {p['muted']};
    background-color: {p['bg']};
}}

#tags.urgent {{
    color: {p['urgent']};
    background-color: {p['bg']};
}}

#clock {{
    background-color: {p['bg']};
    color: {p['fg']};
    padding: 0px 12px;
    margin: 3px 1px;
    border-radius: 8px;
}}

#tray, #network, #volume, #pulseaudio, #backlight, #battery {{
    background-color: {p['bg']};
    color: {p['fg']};
    padding: 0px 8px;
    margin: 3px 1px;
    border-radius: 8px;
}}
""")
    if restart_bar:
        sh("pkill -9 -x mangobar; sleep 0.08; setsid mangobar -c ~/.config/mangobar/config.jsonc -s ~/.config/mangobar/style.css")

    # mango window colors
    tc = Path("~/.config/mango/theme.conf").expanduser()
    vals = {"root_color": p["bg"], "border_color": p["bg2"], "focus_color": p["accent"],
            "urgent_color": p["urgent"], "maximized_screen_color": p["accent2"]}
    lines = []
    for ln in tc.read_text().splitlines():
        k = ln.split("=")[0].strip()
        lines.append(f"{k}={argb(vals[k])}" if k in vals else ln)
    tc.write_text("\n".join(lines) + "\n")
    reload_mango_config()

    # fuzzel launcher
    write("~/.config/fuzzel/fuzzel.ini", f"""[main]
font=0xProto Nerd Font:size=13
terminal=kitty
width=40
lines=12
horizontal-pad=20
vertical-pad=14
inner-pad=8
line-height=24
filter-desktop=yes

[colors]
background={p['bg'][1:]}ee
text={p['fg'][1:]}ff
match={p['accent'][1:]}ff
selection={p['accent'][1:]}ff
selection-text={p['active_fg'][1:]}ff
border={p['accent'][1:]}ff

[border]
width=2
radius=8
""")
    sh("rm -f ~/.cache/fuzzel")

    # mako notifications
    write("~/.config/mako/config", f"""# Auto-generated by wallpaper theme
font=0xProto Nerd Font 12
background-color={p['bg']}
text-color={p['fg']}
border-color={p['accent']}
border-size=2
border-radius=12
progress-color=over {p['accent']}

# Dimensions and spacing
layer=overlay
anchor=bottom-right
width=380
height=160
padding=14,18
margin=8
outer-margin=0,18,18,0

# Timeout and behavior
default-timeout=8500
ignore-timeout=1
markup=1
actions=1

# Icons
icons=1
max-icon-size=48
icon-border-radius=8
icon-location=left
""")
    sh("makoctl reload")

    # Kitty 16-color soft pastel harmonic terminal palette (Tokyo Night / Catppuccin style)
    h_acc, s_acc, _ = hex_to_hsl(p['accent'])

    k_red = hsl_to_hex(0.98, 0.48, 0.62)
    k_green = hsl_to_hex(0.36, 0.38, 0.60)
    k_yellow = hsl_to_hex(0.12, 0.50, 0.65)
    k_blue = hsl_to_hex(0.60, 0.46, 0.64)
    k_magenta = hsl_to_hex(0.80, 0.42, 0.66)
    k_cyan = hsl_to_hex(0.50, 0.38, 0.62)

    k_sel_bg = hsl_to_hex(h_acc, min(0.35, s_acc * 0.4), 0.28)
    k_cursor = hsl_to_hex(h_acc, min(0.65, max(0.40, s_acc * 0.7)), 0.68)
    k_active_border = hsl_to_hex(h_acc, min(0.70, max(0.45, s_acc * 0.8)), 0.60)

    kitty_theme = f"""# Auto-generated soft harmonic theme
foreground            {p['fg']}
background            {p['bg']}
selection_foreground  {p['fg']}
selection_background  {k_sel_bg}
cursor                {k_cursor}
cursor_text_color     {p['bg']}
url_color             {k_cyan}

active_border_color   {k_active_border}
inactive_border_color {p['bg2']}
bell_border_color     {k_red}

active_tab_foreground   {p['fg']}
active_tab_background   {k_sel_bg}
inactive_tab_foreground {p['muted']}
inactive_tab_background {p['bg']}

wayland_titlebar_color  {p['bg']}

color0  {p['bg2']}
color8  {p['muted']}
color1  {k_red}
color9  {lighten(k_red, 10)}
color2  {k_green}
color10 {lighten(k_green, 10)}
color3  {k_yellow}
color11 {lighten(k_yellow, 10)}
color4  {k_blue}
color12 {lighten(k_blue, 10)}
color5  {k_magenta}
color13 {lighten(k_magenta, 10)}
color6  {k_cyan}
color14 {lighten(k_cyan, 10)}
color7  {p['fg']}
color15 #ffffff
"""
    write("~/.config/kitty/current-theme.conf", kitty_theme)
    sh("pkill -SIGUSR1 -x kitty 2>/dev/null || true")

    # GTK 3 & 4 / Libadwaita colors for Nautilus & GTK apps
    gtk_common = f"@define-color accent_color {p['accent']};\n@define-color accent_bg_color {p['accent']};\n@define-color accent_fg_color {p['active_fg']};"
    write("~/.config/gtk-4.0/gtk.css", f"""{gtk_common}
@define-color window_bg_color {p['bg']};
@define-color window_fg_color {p['fg']};
@define-color view_bg_color {p['bg2']};
@define-color view_fg_color {p['fg']};
@define-color headerbar_bg_color {p['bg']};
@define-color headerbar_fg_color {p['fg']};
@define-color card_bg_color {p['bg2']};
@define-color card_fg_color {p['fg']};
@define-color popover_bg_color {p['bg']};
@define-color popover_fg_color {p['fg']};

.navigation-sidebar > row:selected {{
    background-color: @accent_bg_color;
    color: @accent_fg_color;
    border-radius: 6px;
}}
""")
    write("~/.config/gtk-3.0/gtk.css", f"""{gtk_common}
@define-color theme_selected_bg_color {p['accent']};
@define-color theme_selected_fg_color {p['active_fg']};
""")

    # GNOME / Libadwaita desktop interface integration
    gnome_accent = closest_gnome_accent(h_acc, s_acc)
    subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", "accent-color", gnome_accent], check=False)
    subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", "color-scheme", "prefer-dark"], check=False)


def wallpaper(path):
    p = Path(path).resolve()
    old_pids = []
    try:
        out = subprocess.check_output(["pgrep", "-f", "swaybg|mpvpaper"], text=True)
        old_pids = [int(x) for x in out.strip().split() if x.isdigit()]
    except Exception:
        pass

    cmd = ["mpvpaper", "-o", "no-audio loop", "ALL", str(p)] if p.suffix.lower() in VIDEO else ["swaybg", "-i", str(p), "-m", "fill"]
    subprocess.Popen(cmd, start_new_session=True)

    time.sleep(0.18)
    for pid in old_pids:
        try:
            os.kill(pid, 15)
        except ProcessLookupError:
            pass

    (CACHE / "current").write_text(str(p))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--restore":
        cur = CACHE / "current"
        if cur.exists() and Path(cur.read_text().strip()).exists():
            f = cur.read_text().strip()
            apply(palette(f))
            wallpaper(f)
            sys.exit(0)
        else:
            walls = sorted((p for p in (HOME / "Resimler/Wallpapers").iterdir() if p.suffix.lower() in {".png", ".jpg", ".jpeg", ".webp"} | VIDEO), key=os.path.getmtime, reverse=True)
            if walls:
                f = str(walls[0])
                apply(palette(f))
                wallpaper(f)
                sys.exit(0)
            else:
                sh("swaybg -c 201b14")
                sys.exit(0)
    if len(sys.argv) > 1:
        f = sys.argv[1]
        apply(palette(f))
        if "--wall" in sys.argv:
            wallpaper(f)
