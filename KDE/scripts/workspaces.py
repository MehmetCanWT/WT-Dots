#!/usr/bin/env python3
import json
import re
import subprocess
import sys


def get_desktops_state():
    try:
        out = subprocess.check_output(
            [
                "busctl",
                "--user",
                "get-property",
                "org.kde.KWin",
                "/VirtualDesktopManager",
                "org.kde.KWin.VirtualDesktopManager",
                "desktops",
            ],
            text=True,
        )
        matches = re.findall(r'(\d+)\s+"([^"]+)"\s+"([^"]+)"', out)
        desktops = [(int(pos), did, name) for pos, did, name in matches]

        cur_out = subprocess.check_output(
            [
                "busctl",
                "--user",
                "get-property",
                "org.kde.KWin",
                "/VirtualDesktopManager",
                "org.kde.KWin.VirtualDesktopManager",
                "current",
            ],
            text=True,
        )
        m_cur = re.search(r'"([^"]+)"', cur_out)
        cur_id = m_cur.group(1) if m_cur else ""

        cur_idx = 0
        cur_name = "Desktop 1"
        for i, (pos, did, name) in enumerate(desktops):
            if did == cur_id:
                cur_idx = i
                cur_name = name
                break
        return desktops, cur_idx, cur_name
    except Exception:
        return [(0, "default", "Desktop 1")], 0, "Desktop 1"


def switch_to_index(target_idx):
    desktops, cur_idx, _ = get_desktops_state()
    count = len(desktops)

    if target_idx >= count and target_idx < 6:
        for i in range(count, target_idx + 1):
            subprocess.run(
                [
                    "busctl",
                    "--user",
                    "call",
                    "org.kde.KWin",
                    "/VirtualDesktopManager",
                    "org.kde.KWin.VirtualDesktopManager",
                    "createDesktop",
                    "us",
                    str(i),
                    f"Masaüstü {i+1}",
                ]
            )
        desktops, _, _ = get_desktops_state()
        count = len(desktops)

    if target_idx < count:
        target_id = desktops[target_idx][1]
        subprocess.run(
            [
                "busctl",
                "--user",
                "set-property",
                "org.kde.KWin",
                "/VirtualDesktopManager",
                "org.kde.KWin.VirtualDesktopManager",
                "current",
                "s",
                target_id,
            ]
        )
    # Signal waybar to refresh immediately
    subprocess.run(["pkill", "-RTMIN+8", "waybar"])


def handle_action(action):
    desktops, cur_idx, _ = get_desktops_state()
    count = max(1, len(desktops))

    if action == "next":
        next_idx = (cur_idx + 1) % 6
        switch_to_index(next_idx)
    elif action == "prev":
        prev_idx = (cur_idx - 1) % count
        switch_to_index(prev_idx)
    elif action == "overview":
        subprocess.run(
            [
                "qdbus-qt6",
                "org.kde.KWin",
                "/Effects",
                "org.kde.kwin.Effects.toggleEffect",
                "overview",
            ]
        )
    elif action.isdigit():
        switch_to_index(int(action) - 1)


def print_status():
    desktops, cur_idx, cur_name = get_desktops_state()
    total_dots = 6
    dots = []
    for i in range(total_dots):
        if i == cur_idx:
            dots.append("<span color='#ffffff'>●</span>")
        else:
            dots.append("<span color='#545868'>○</span>")

    text = " ".join(dots)
    data = {
        "text": text,
        "tooltip": f"{cur_name} ({cur_idx + 1}/{len(desktops)})",
        "class": f"workspace-{cur_idx+1}",
    }
    print(json.dumps(data))


if __name__ == "__main__":
    if len(sys.argv) > 1:
        handle_action(sys.argv[1])
    else:
        print_status()
