#!/usr/bin/env python3
import atexit, fcntl, hashlib, os, subprocess, sys
from pathlib import Path
from PIL import Image
from PySide6.QtCore import QObject, Property, QTimer, QUrl, Signal, Slot
from PySide6.QtGui import QGuiApplication, QSurfaceFormat
from PySide6.QtQml import QQmlApplicationEngine

# Singleton check: allow only one instance at a time
LOCK_PATH = "/tmp/wallpaper-picker.lock"
try:
    lock_file = open(LOCK_PATH, "w")
    fcntl.flock(lock_file, fcntl.LOCK_EX | fcntl.LOCK_NB)
except (BlockingIOError, OSError):
    sys.exit(0)

sys.path.insert(0, str(Path(__file__).parent))
import theme

DIR = Path.home() / "Resimler/Wallpapers"
EXT = {".png", ".jpg", ".jpeg", ".webp"} | theme.VIDEO


def restore_bar():
    res = subprocess.run(["pgrep", "-x", "mangobar"], capture_output=True)
    if res.returncode != 0:
        subprocess.Popen([
            "mangobar",
            "-c", str(Path.home() / ".config/mangobar/config.jsonc"),
            "-s", str(Path.home() / ".config/mangobar/style.css")
        ], start_new_session=True)


atexit.register(restore_bar)


def thumb(f):
    out = theme.CACHE / (hashlib.sha1(f"{f}{os.path.getmtime(f)}".encode()).hexdigest() + ".thumb2.png")
    if not out.exists():
        im = Image.open(theme.still(f)).convert("RGB")
        im.thumbnail((640, 360))
        im.save(out)
    return out


class Bridge(QObject):
    paletteChanged = Signal()

    def __init__(self, files):
        super().__init__()
        self.files = files
        cur_file = (theme.CACHE / "current").read_text().strip() if (theme.CACHE / "current").exists() else ""
        self.initial_index = self.files.index(cur_file) if cur_file in self.files else 0
        self._pal = theme.palette(self.files[self.initial_index])
        self.pending = None
        self.timer = QTimer(self)
        self.timer.setSingleShot(True)
        self.timer.setInterval(40)
        self.timer.timeout.connect(self.fire)

    @Property(int, constant=True)
    def initialIndex(self):
        return self.initial_index

    @Property("QVariantMap", notify=paletteChanged)
    def pal(self):
        return self._pal

    @Property("QVariantList", constant=True)
    def items(self):
        return [{
            "name": Path(f).stem,
            "path": QUrl.fromLocalFile(str(theme.still(f))).toString(),
            "thumb": QUrl.fromLocalFile(str(thumb(f))).toString(),
            "video": Path(f).suffix.lower() in theme.VIDEO
        } for f in self.files]

    @Slot(int)
    def preview(self, i):
        self.pending = self.files[i]
        self.timer.start()

    def fire(self):
        self._pal = theme.palette(self.pending)
        self.paletteChanged.emit()

    @Slot(int)
    def commit(self, i):
        self.timer.stop()
        theme.apply(theme.palette(self.files[i]))
        theme.wallpaper(self.files[i])
        restore_bar()
        QGuiApplication.quit()

    @Slot()
    def cancel(self):
        self.timer.stop()
        restore_bar()
        QGuiApplication.quit()


def main():
    # Hide status bar while picker is active
    subprocess.run(["pkill", "-x", "mangobar"], check=False)

    theme.CACHE.mkdir(parents=True, exist_ok=True)
    files = sorted((str(p) for p in DIR.iterdir() if p.suffix.lower() in EXT), key=os.path.getmtime, reverse=True)
    if not files:
        sys.exit("wallpaper yok: " + str(DIR))
    fmt = QSurfaceFormat()
    fmt.setAlphaBufferSize(8)
    QSurfaceFormat.setDefaultFormat(fmt)
    app = QGuiApplication(sys.argv)
    app.setDesktopFileName("wallpaper-picker")
    b = Bridge(files)
    eng = QQmlApplicationEngine()
    eng.rootContext().setContextProperty("bridge", b)
    eng.load(QUrl.fromLocalFile(str(Path(__file__).with_name("picker.qml"))))
    exit_code = app.exec()
    restore_bar()
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
