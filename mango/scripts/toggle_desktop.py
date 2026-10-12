#!/usr/bin/env python3
import json, os, subprocess

STATE_FILE = "/tmp/mango_desktop_minimized.json"


def get_clients():
    res = subprocess.run(["mmsg", "get", "all-clients"], capture_output=True, text=True)
    try:
        return json.loads(res.stdout).get("clients", [])
    except Exception:
        return []


def main():
    clients = get_clients()
    visible_ids = [c["id"] for c in clients if not c.get("is_minimized", False)]

    # If any windows are currently visible on screen -> hide all to reveal clean desktop
    if visible_ids:
        try:
            with open(STATE_FILE, "w") as f:
                json.dump(visible_ids, f)
        except Exception:
            pass
        for cid in visible_ids:
            subprocess.run(["mmsg", "dispatch", "minimized", f"client,{cid}"], check=False)
    # If no windows are visible on screen -> restore previously hidden windows
    elif os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE, "r") as f:
                saved_ids = json.load(f)
        except Exception:
            saved_ids = []

        all_ids = {c["id"] for c in clients}
        for cid in saved_ids:
            if cid in all_ids:
                subprocess.run(["mmsg", "dispatch", "restore_minimized", f"client,{cid}"], check=False)

        try:
            os.remove(STATE_FILE)
        except OSError:
            pass


if __name__ == "__main__":
    main()
