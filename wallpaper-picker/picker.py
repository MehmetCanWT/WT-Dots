#!/usr/bin/env python3
import hashlib, os, subprocess, sys, threading
from pathlib import Path
from PIL import Image
from PySide6.QtCore import QObject, Property, QTimer, QUrl, Signal, Slot
from PySide6.QtGui import QGuiApplication, QSurfaceFormat
from PySide6.QtQml import QQmlApplicationEngine

sys.path.insert(0, str(Path(__file__).parent))
import theme

DIR = Path.home() / "Resimler/Wallpapers"
EXT = {".png", ".jpg", ".jpeg", ".webp"} | theme.VIDEO


def thumb(f):
    out = theme.CACHE / (hashlib.sha1(f"{f}{os.path.getmtime(f)}".encode()).hexdigest() + ".thumb2.png")
    if not out.exists():
        im = Image.open(theme.still(f)).convert("RGB")
        im.thumbnail((1280, 720))
        im.save(out)
    return out


class Bridge(QObject):
    paletteChanged = Signal()

    def __init__(self, files):
        super().__init__()
        self.files = files
        self._pal = theme.palette(files[0])
        self.orig = (theme.CACHE / "current").read_text() if (theme.CACHE / "current").exists() else None
        self.pending = None
        self.timer = QTimer(self)
        self.timer.setSingleShot(True)
        self.timer.setInterval(40)
        self.timer.timeout.connect(self.fire)

    @Property("QVariantMap", notify=paletteChanged)
    def pal(self):
        return self._pal

    @Property("QVariantList", constant=True)
    def items(self):
        return [{"name": Path(f).stem, "thumb": QUrl.fromLocalFile(str(thumb(f))).toString(),
                 "video": Path(f).suffix.lower() in theme.VIDEO} for f in self.files]

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
        QGuiApplication.quit()

    @Slot()
    def cancel(self):
        self.timer.stop()
        QGuiApplication.quit()


def main():
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
    sys.exit(app.exec())


main()
