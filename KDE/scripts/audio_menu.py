#!/usr/bin/env python3
import atexit
import os
import re
import subprocess
import sys

MENU_TAG = "audio"
PID_TAG_FILE = "/tmp/active_rofi_menu"

# Singleton & Toggle handling
try:
    pids = subprocess.check_output(["pgrep", "-x", "rofi"], text=True).strip().split()
    if pids:
        cur_menu = ""
        if os.path.exists(PID_TAG_FILE):
            with open(PID_TAG_FILE) as f:
                cur_menu = f.read().strip()
        subprocess.run(["pkill", "-x", "rofi"])
        if cur_menu == MENU_TAG:
            if os.path.exists(PID_TAG_FILE):
                os.remove(PID_TAG_FILE)
            sys.exit(0)
except Exception:
    pass

try:
    with open(PID_TAG_FILE, "w") as f:
        f.write(MENU_TAG)
except Exception:
    pass


def cleanup_tag():
    if os.path.exists(PID_TAG_FILE):
        try:
            with open(PID_TAG_FILE) as f:
                if f.read().strip() == MENU_TAG:
                    os.remove(PID_TAG_FILE)
        except Exception:
            pass


atexit.register(cleanup_tag)


def get_volume_info(target):
    try:
        out = subprocess.check_output(["wpctl", "get-volume", target], text=True).strip()
        muted = "[MUTED]" in out
        m = re.search(r"Volume:\s+([0-9.]+)", out)
        vol = int(float(m.group(1)) * 100) if m else 0
        return vol, muted
    except Exception:
        return 0, False


def get_audio_devices():
    try:
        out = subprocess.run(["wpctl", "status"], capture_output=True, text=True, check=True).stdout
    except Exception:
        return [], []

    sinks = []
    sources = []
    in_audio = False
    section = None

    for line in out.splitlines():
        line_s = line.strip()
        if line_s == "Audio":
            in_audio = True
            continue
        elif line_s in ("Video", "Settings"):
            in_audio = False
            section = None
            continue

        if in_audio:
            if "Sinks:" in line:
                section = "sinks"
                continue
            elif "Sources:" in line:
                section = "sources"
                continue
            elif any(k in line for k in ("Filters:", "Streams:")):
                section = None
                continue

            if section in ("sinks", "sources"):
                m = re.search(r"([*]?)\s+(\d+)\.\s+([^\[\n]+)", line)
                if m:
                    is_def = bool(m.group(1))
                    dev_id = m.group(2)
                    name = m.group(3).strip()
                    if section == "sinks":
                        sinks.append((dev_id, name, is_def))
                    else:
                        sources.append((dev_id, name, is_def))
    return sinks, sources


def show_rofi_menu(items, selected_row=0, width=440):
    lines_count = max(1, min(12, len(items)))
    cmd = [
        "rofi",
        "-dmenu",
        "-click-to-exit",
        "-markup-rows",
        "-selected-row",
        str(selected_row),
        "-theme-str",
        f'* {{ font: "0xProto Nerd Font 11"; }} window {{ location: north east; anchor: north east; y-offset: 38px; x-offset: -50px; width: {width}px; border: 2px; border-color: #567ae6; border-radius: 12px; }} mainbox {{ children: [ listview ]; }} listview {{ lines: {lines_count}; require-input: false; }}',
    ]
    proc = subprocess.run(cmd, input="\n".join(items), capture_output=True, text=True)
    if proc.returncode != 0:
        return None, -1

    choice = proc.stdout.strip()
    if not choice:
        return None, -1

    idx = -1
    for i, it in enumerate(items):
        if it == choice:
            idx = i
            break
    return choice, idx


def main():
    view = "main"
    last_idx = 0

    while True:
        sinks, sources = get_audio_devices()

        if view == "main":
            active_sink_name = "Bulunamadı"
            for _, name, is_def in sinks:
                if is_def:
                    active_sink_name = name
                    break

            active_source_name = "Bulunamadı"
            for _, name, is_def in sources:
                if is_def:
                    active_source_name = name
                    break

            sink_vol, sink_muted = get_volume_info("@DEFAULT_AUDIO_SINK@")
            source_vol, source_muted = get_volume_info("@DEFAULT_AUDIO_SOURCE@")

            sink_status = "Sessiz" if sink_muted else f"%{sink_vol}"
            source_status = "Sessiz" if source_muted else f"%{source_vol}"

            items = [
                f"󰓃 <b>Hoparlör (Çıkış):</b> {active_sink_name}  <span foreground='#888899'>[{sink_status}]</span>  ❯",
                f"󰍬 <b>Mikrofon (Giriş):</b> {active_source_name}  <span foreground='#888899'>[{source_status}]</span>  ❯",
                f"󰝟 Hoparlör Sessize Al / Aç ({'Sessiz' if sink_muted else 'Açık'})",
                f"󰍭 Mikrofon Sessize Al / Aç ({'Sessiz' if source_muted else 'Açık'})",
            ]

            choice, idx = show_rofi_menu(items, selected_row=last_idx, width=540)
            if choice is None:
                break

            last_idx = max(0, idx)

            if "Hoparlör (Çıkış):" in choice:
                view = "sinks"
                last_idx = 1
            elif "Mikrofon (Giriş):" in choice:
                view = "sources"
                last_idx = 1
            elif "Hoparlör Sessize" in choice:
                subprocess.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            elif "Mikrofon Sessize" in choice:
                subprocess.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])

        elif view in ("sinks", "sources"):
            is_sink = view == "sinks"
            devices = sinks if is_sink else sources
            icon = "󰓃" if is_sink else "󰍬"

            items = ["⬅ <b>Geri</b> (Ana Menü)"]
            active_idx = 1
            for i, (dev_id, name, is_def) in enumerate(devices):
                if is_def:
                    active_idx = i + 1
                    items.append(f"<b><span foreground='#ffffff'>● {icon} {name}</span>  <span foreground='#567ae6'>[AKTİF]</span></b> [ID:{dev_id}]")
                else:
                    items.append(f"<span foreground='#888899'>○ {icon} {name}</span> [ID:{dev_id}]")

            choice, idx = show_rofi_menu(items, selected_row=last_idx if last_idx < len(items) else active_idx, width=540)
            if choice is None:
                break

            if "Geri" in choice:
                view = "main"
                last_idx = 0 if is_sink else 1
                continue

            last_idx = max(1, idx)
            m = re.search(r"\[ID:(\d+)\]", choice)
            if m:
                subprocess.run(["wpctl", "set-default", m.group(1)])


if __name__ == "__main__":
    main()
