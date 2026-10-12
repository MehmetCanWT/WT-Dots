#!/usr/bin/env python3
"""palette cikar + tema uygula. Kullanim: theme.py <dosya> [--wall]"""
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


def hexc(h, s, v):
    r, g, b = colorsys.hsv_to_rgb(h % 1.0, max(0, min(1, s)), max(0, min(1, v)))
    return "#%02x%02x%02x" % (round(r * 255), round(g * 255), round(b * 255))


def palette(path):
    key = hashlib.sha1(f"{path}{os.path.getmtime(path)}".encode()).hexdigest()
    cf = CACHE / (key + ".json")
    if cf.exists():
        return json.loads(cf.read_text())
    im = Image.open(still(path)).convert("RGB")
    im.thumbnail((96, 96))
    q = im.quantize(10, method=Image.Quantize.MEDIANCUT)
    pal = q.getpalette()
    best, score = (0.6, 0.5, 0.8), -1
    for n, idx in q.getcolors():
        r, g, b = (pal[idx * 3 + i] / 255 for i in range(3))
        h, s, v = colorsys.rgb_to_hsv(r, g, b)
        sc = (n ** 0.5) * (s + 0.15) * (1 if 0.25 < v < 0.95 else 0.3)
        if sc > score:
            best, score = (h, s, v), sc
    h, s, _ = best
    s = max(s, 0.35)
    p = {
        "bg": hexc(h, 0.30 * s + 0.1, 0.09), "bg2": hexc(h, 0.30 * s + 0.1, 0.16),
        "fg": hexc(h, 0.12, 0.88), "muted": hexc(h, 0.18, 0.50),
        "accent": hexc(h, min(0.65, s + 0.1), 0.85), "accent2": hexc(h + 0.08, 0.5, 0.80),
        "urgent": "#e06c5a",
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


def apply(p):
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
    margin: 0px 1px;
    background-color: {p['bg']};
}}

#tags.active {{
    color: {p['bg']};
    background-color: {p['accent']};
    border-radius: 6px;
    margin: 3px 2px;
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
    border-radius: 8px;
}}

#tray, #network, #volume, #battery {{
    background-color: {p['bg']};
    color: {p['fg']};
    padding: 0px 8px;
    border-radius: 8px;
}}
""")
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
    if sys.argv[1] == "--restore":  # acilista son duvar kagidi
        cur = CACHE / "current"
        if cur.exists() and Path(cur.read_text()).exists():
            sys.argv[1:] = [cur.read_text(), "--wall"]
        else:
            sh("swaybg -c 201b14")
            sys.exit()
    f = sys.argv[1]
    apply(palette(f))
    if "--wall" in sys.argv:
        wallpaper(f)
