#!/usr/bin/env python3
"""ricemood / node-vibrant inspired color extractor & theme applier."""
import colorsys, hashlib, json, os, subprocess, sys
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


# ricemood color modifier helpers
def lighten(hex_str, amount):
    h, s, l = hex_to_hsl(hex_str)
    return hsl_to_hex(h, s, l + (amount / 100.0))


def darken(hex_str, amount):
    h, s, l = hex_to_hsl(hex_str)
    return hsl_to_hex(h, s, l - (amount / 100.0))


def saturate(hex_str, amount):
    h, s, l = hex_to_hsl(hex_str)
    return hsl_to_hex(h, s + (amount / 100.0), l)


def desaturate(hex_str, amount):
    h, s, l = hex_to_hsl(hex_str)
    return hsl_to_hex(h, s - (amount / 100.0), l)


def extract_vibrant_swatches(image_path):
    """node-vibrant / Android Palette implementation using PIL median-cut quantization."""
    im = Image.open(still(image_path)).convert("RGB")
    im.thumbnail((128, 128))
    q = im.quantize(16, method=Image.Quantize.MEDIANCUT)
    pal = q.getpalette()
    colors = q.getcolors()

    swatches = []
    max_pop = max((cnt for cnt, _ in colors), default=1)
    for cnt, idx in colors:
        r, g, b = (pal[idx * 3 + i] / 255.0 for i in range(3))
        h, s, l = rgb_to_hsl(r, g, b)
        swatches.append({'r': r, 'g': g, 'b': b, 'h': h, 's': s, 'l': l, 'pop': cnt})

    profiles = {
        'Vibrant':      {'target_s': 1.0,  'min_s': 0.35, 'max_s': 1.0, 'target_l': 0.50, 'min_l': 0.30, 'max_l': 0.70},
        'LightVibrant': {'target_s': 1.0,  'min_s': 0.35, 'max_s': 1.0, 'target_l': 0.74, 'min_l': 0.55, 'max_l': 1.00},
        'DarkVibrant':  {'target_s': 1.0,  'min_s': 0.35, 'max_s': 1.0, 'target_l': 0.26, 'min_l': 0.00, 'max_l': 0.45},
        'Muted':        {'target_s': 0.30, 'min_s': 0.00, 'max_s': 0.40, 'target_l': 0.50, 'min_l': 0.30, 'max_l': 0.70},
        'LightMuted':   {'target_s': 0.30, 'min_s': 0.00, 'max_s': 0.40, 'target_l': 0.74, 'min_l': 0.55, 'max_l': 1.00},
        'DarkMuted':    {'target_s': 0.30, 'min_s': 0.00, 'max_s': 0.40, 'target_l': 0.26, 'min_l': 0.00, 'max_l': 0.45},
    }

    results = {}
    for name, prof in profiles.items():
        best, best_score = None, -1.0
        for sw in swatches:
            s, l, pop = sw['s'], sw['l'], sw['pop']
            if not (prof['min_s'] <= s <= prof['max_s'] and prof['min_l'] <= l <= prof['max_l']):
                continue
            score = (1.0 - abs(s - prof['target_s'])) * 0.35 + (1.0 - abs(l - prof['target_l'])) * 0.52 + (pop / max_pop) * 0.13
            if score > best_score:
                best_score = score
                best = sw
        if best:
            results[name] = rgb_to_hex(best['r'], best['g'], best['b'])

    # Fallback to dominant colors if any profile is missing
    fallback = rgb_to_hex(swatches[0]['r'], swatches[0]['g'], swatches[0]['b']) if swatches else "#7798d9"
    results.setdefault('Vibrant', fallback)
    results.setdefault('LightVibrant', lighten(results['Vibrant'], 25))
    results.setdefault('DarkVibrant', darken(results['Vibrant'], 25))
    results.setdefault('Muted', desaturate(results['Vibrant'], 35))
    results.setdefault('LightMuted', lighten(results['Muted'], 25))
    results.setdefault('DarkMuted', darken(results['Muted'], 25))
    return results


def palette(path):
    key = hashlib.sha1(f"{path}{os.path.getmtime(path)}".encode()).hexdigest()
    cf = CACHE / (key + ".vibrant.json")
    if cf.exists():
        try:
            return json.loads(cf.read_text())
        except Exception:
            pass

    sw = extract_vibrant_swatches(path)

    # ricemood style color scheme synthesis
    h, s, _ = hex_to_hsl(sw['DarkVibrant'] if 'DarkVibrant' in sw else sw['Vibrant'])
    # Deep solid dark theme background subtly tinted by the image
    bg = hsl_to_hex(h, min(0.30, s * 0.4), 0.08)
    bg2 = hsl_to_hex(h, min(0.35, s * 0.5), 0.14)
    fg = lighten(sw.get('LightMuted', sw.get('LightVibrant', "#e0dcd4")), 10)
    accent = sw['Vibrant']
    accent2 = sw.get('LightVibrant', lighten(accent, 15))
    muted = sw.get('Muted', desaturate(accent, 40))
    urgent = "#e05f52"

    p = {
        "bg": bg,
        "bg2": bg2,
        "fg": fg,
        "muted": muted,
        "accent": accent,
        "accent2": accent2,
        "urgent": urgent,
        "vibrant": sw['Vibrant'],
        "dark_vibrant": sw.get('DarkVibrant', bg),
        "light_vibrant": sw.get('LightVibrant', accent2),
        "dark_muted": sw.get('DarkMuted', bg2),
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


def apply(p, restart_bar=True):
    # mangobar: 3-island floating setup with solid dark color
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

#tags {{
    padding: 0px 8px;
    margin: 3px 1px;
    background-color: {p['bg']};
}}

#tags.active {{
    color: {p['bg']};
    background-color: {p['accent']};
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

#tray, #network, #volume, #battery {{
    background-color: {p['bg']};
    color: {p['fg']};
    padding: 0px 8px;
    margin: 3px 1px;
    border-radius: 8px;
}}
""")
    if restart_bar:
        sh("pkill -x mangobar; sleep 0.05; setsid mangobar -c ~/.config/mangobar/config.jsonc -s ~/.config/mangobar/style.css")

    # mango renkleri
    tc = Path("~/.config/mango/theme.conf").expanduser()
    vals = {"root_color": p["bg"], "border_color": p["bg2"], "focus_color": p["accent"],
            "urgent_color": p["urgent"], "maximized_screen_color": p["accent2"]}
    lines = []
    for ln in tc.read_text().splitlines():
        k = ln.split("=")[0].strip()
        lines.append(f"{k}={argb(vals[k])}" if k in vals else ln)
    tc.write_text("\n".join(lines) + "\n")
    sh("mmsg dispatch reload_config")

    # fuzzel, mako, kitty
    write("~/.config/fuzzel/fuzzel.ini", f"""[main]
font=0xProto Nerd Font:size=13
terminal=kitty
width=40
lines=12
horizontal-pad=20
vertical-pad=14
inner-pad=8
line-height=24

[colors]
background={p['bg'][1:]}ee
text={p['fg'][1:]}ff
match={p['accent'][1:]}ff
selection={p['accent'][1:]}ff
selection-text={p['bg'][1:]}ff
border={p['accent'][1:]}ff

[border]
width=2
radius=8
""")
    write("~/.config/mako/config", f"""font=0xProto Nerd Font 11
background-color={p['bg']}
text-color={p['fg']}
border-color={p['accent']}
border-radius=8
default-timeout=5000
""")
    sh("makoctl reload")

    write("~/.config/kitty/current-theme.conf", f"""foreground {p['fg']}
background {p['bg']}
cursor {p['accent']}
selection_background {p['accent']}
selection_foreground {p['bg']}
color0 {p['bg2']}
color8 {p['muted']}
color1 {p['urgent']}
color4 {p['accent']}
color6 {p['accent2']}
color7 {p['fg']}
""")
    kc = Path("~/.config/kitty/kitty.conf").expanduser()
    if not kc.exists() or "current-theme.conf" not in kc.read_text():
        with kc.open("a") as f:
            f.write("include current-theme.conf\n")
    sh("pkill -USR1 -x kitty")


def wallpaper(path):
    p = Path(path)
    new = (f"mpvpaper -o 'no-audio loop' ALL '{p}'" if p.suffix.lower() in VIDEO
           else f"swaybg -i '{p}' -m fill")
    sh(f"old=$(pgrep -x 'swaybg|mpvpaper'); {new} & sleep 0.3; kill $old 2>/dev/null")
    (CACHE / "current").write_text(str(p))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--restore":
        cur = CACHE / "current"
        if cur.exists() and Path(cur.read_text()).exists():
            sys.argv[1:] = [cur.read_text(), "--wall"]
        else:
            sh("swaybg -c 201b14")
            sys.exit()
    if len(sys.argv) > 1:
        f = sys.argv[1]
        apply(palette(f))
        if "--wall" in sys.argv:
            wallpaper(f)
