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
    client_dict = {c["id"]: c for c in clients}

    # 1. If STATE_FILE exists: We previously showed desktop -> RESTORE
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE, "r") as f:
                saved_ids = json.load(f)
        except Exception:
            saved_ids = []

        try:
            os.remove(STATE_FILE)
        except OSError:
            pass

        restored_count = 0
        for item in saved_ids:
            cid = item["id"] if isinstance(item, dict) else item
            if cid in client_dict and client_dict[cid].get("is_minimized", False):
                subprocess.run(["mmsg", "dispatch", "restore_minimized", f"client,{cid}"], check=False)
                restored_count += 1

        # If any window was restored, return
        if restored_count > 0:
            return

    # 2. If STATE_FILE did NOT exist (or was empty/invalid):
    # Only minimize windows currently visible on the active workspace
    visible_clients = [c for c in clients if c.get("is_visible", False) and not c.get("is_minimized", False)]
    if not visible_clients:
        visible_clients = [c for c in clients if not c.get("is_minimized", False)]

    if not visible_clients:
        return

    saved_ids = [c["id"] for c in visible_clients]
    try:
        with open(STATE_FILE, "w") as f:
            json.dump(saved_ids, f)
    except Exception:
        pass

    for cid in saved_ids:
        subprocess.run(["mmsg", "dispatch", "minimized", f"client,{cid}"], check=False)


if __name__ == "__main__":
    main()
