#!/usr/bin/env python3
import re
import subprocess
import sys

def main():
    try:
        out = subprocess.run(["wpctl", "status"], capture_output=True, text=True, check=True).stdout
    except Exception as e:
        sys.exit(f"wpctl error: {e}")

    sinks = []
    sources = []
    section = None

    for line in out.splitlines():
        if "Sinks:" in line:
            section = "sinks"
            continue
        elif "Sources:" in line:
            section = "sources"
            continue
        elif any(k in line for k in ("Filters:", "Streams:", "Video", "Settings")):
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

    menu_lines = []
    for dev_id, name, is_def in sinks:
        check = "● " if is_def else "○ "
        menu_lines.append(f"{check}󰓃 Hoparlör: {name} [ID:{dev_id}]")

    for dev_id, name, is_def in sources:
        check = "● " if is_def else "○ "
        menu_lines.append(f"{check}󰍬 Mikrofon: {name} [ID:{dev_id}]")

    menu_lines.append("󰝟 Hoparlör Sessize Al / Aç (Toggle)")
    menu_lines.append("󰍭 Mikrofonu Sessize Al / Aç (Toggle)")

    rofi_input = "\n".join(menu_lines)
    lines_count = min(10, len(menu_lines))
    proc = subprocess.run(
        [
            "rofi", "-dmenu",
            "-theme-str", f"mainbox {{ children: [ listview ]; }} listview {{ lines: {lines_count}; }} window {{ width: 520px; }}"
        ],
        input=rofi_input,
        capture_output=True,
        text=True
    )

    choice = proc.stdout.strip()
    if not choice:
        return

    if "Hoparlör Sessize" in choice:
        subprocess.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
    elif "Mikrofonu Sessize" in choice:
        subprocess.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
    else:
        m_id = re.search(r"\[ID:(\d+)\]", choice)
        if m_id:
            target_id = m_id.group(1)
            subprocess.run(["wpctl", "set-default", target_id])

if __name__ == "__main__":
    main()
