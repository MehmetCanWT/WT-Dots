#!/usr/bin/env python3
import sys
import re
from pathlib import Path
from PySide6.QtCore import Qt
from PySide6.QtGui import QFont, QColor, QPalette
from PySide6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QHBoxLayout, QVBoxLayout,
    QListWidget, QListWidgetItem, QLabel, QScrollArea, QFrame
)

DATA = {
    "🚀 Sistem & Başlatıcı": [
        ("Alt + Space", "Uygulama Başlatıcı (Rofi)"),
        ("Alt + Enter", "Terminal Aç (Kitty)"),
        ("Super + E", "Dosya Yöneticisi (Dolphin)"),
        ("Super + V", "Pano Geçmişi (Cliphist + Rofi)"),
        ("Super + W", "Duvar Kağıdı ve Tema Seçici"),
        ("Super + K", "Bu Kısayol Rehberi"),
        ("Super + R", "MangoWM Yapılandırmasını Yenile"),
        ("Super + M", "MangoWM'den Çıkış Yap (Oturumu Kapat)"),
        ("Print", "Tam Ekran Ekran Görüntüsü Al (Grim + Pano)"),
        ("Shift + Print", "Bölge Seçerek Ekran Görüntüsü Al (Slurp + Grim)"),
    ],
    "🪟 Pencere Kontrolleri": [
        ("Alt + Q", "Seçili Pencereyi Kapat (Kill Client)"),
        ("Alt + \\", "Pencereyi Çalışma Alanında Tam Ekran Yap (Bar Kalır)"),
        ("Alt + A", "Pencereyi Serbest/Yüzen Yap (Toggle Floating)"),
        ("Alt + C", "Yüzen Pencereyi Ekranın Ortasına Hizala"),
        ("Alt + Orta Tık", "Tam Ekran Yap (Monitörün Tamamı)"),
        ("Super + D", "Masaüstünü Göster (Pencereleri Gizle/Geri Getir)"),
        ("Alt + Tab", "Hızlı Pencere Değiştirici (Jump)"),
        ("Super + Tab", "Sonraki Pencereye Odaklan"),
        ("Alt + Yön Tuşları", "Pencereler Arasında Yön Odaklama"),
        ("Super + Shift + Yön Tuşları", "Pencerenin Yerini Yön Tuşlarıyla Değiştir"),
        ("Super + Sol Tık", "Pencereyi Fareyle Sürükle / Taşı"),
        ("Super + Sağ Tık", "Pencereyi Fareyle Boyutlandır"),
    ],
    "📐 Boyutlandırma & Düzen": [
        ("Alt + X", "Pencere Genişlik Oranını Değiştir (%50 / %80 / %100)"),
        ("Alt + E", "Pencereyi Tam Genişliğe Yay (%100)"),
        ("Ctrl + Alt + Yön Tuşları", "Yüzen Pencere Boyutunu 50px Genişlet/Daralt"),
        ("Ctrl + Shift + Yön Tuşları", "Yüzen Pencereyi 50px Adımlarla Taşı"),
        ("Super + N", "Dizilim Düzenini Değiştir (Tile / Scroller / Dwindle)"),
        ("Alt + Shift + X", "Pencere Boşluklarını (Gaps) Artır"),
        ("Alt + Shift + Z", "Pencere Boşluklarını (Gaps) Azalt"),
        ("Alt + Shift + R", "Boşlukları Aç / Kapat"),
    ],
    "🏷️ Çalışma Alanları (Tags)": [
        ("Alt + 1..9", "1'den 9'a Kadar Çalışma Alanına Geç"),
        ("Ctrl + 1..9", "Seçili Pencereyi 1..9 Numaralı Alana Gönder"),
        ("Super + Sol / Sağ", "Önceki / Sonraki Çalışma Alanına Geç"),
        ("Ctrl + Super + Sol / Sağ", "Pencereyi Önceki / Sonraki Alana Taşı"),
    ],
    "🔊 Ses, Parlaklık & Medya": [
        ("Ses Açma Tuşu", "Sesi %5 Artır (Maks %100)"),
        ("Ses Kısma Tuşu", "Sesi %5 Azalt"),
        ("Sessize Alma Tuşu", "Sesi Aç / Kapat (Mute Toggle)"),
        ("Mangobar Ses Modülü", "Tekerlek: Ses Ayarı | Tık: Ses Aygıtı Menüsü"),
        ("Mangobar Parlaklık", "Tekerlek: Parlaklık | Tık: Parlaklık Menüsü"),
    ],
    "🎮 Oyun Modu & MangoHud": [
        ("Super + L", "Oyun Modunu Aç/Kapat (Compositor Tuşlarını Kilitler)"),
        ("Sağ Shift + F12", "MangoHud Performans Göstergesini Aç/Kapat"),
        ("Steam Başlatma", "mangohud %command%"),
    ]
}

def get_accent():
    try:
        cfg = Path("~/.config/mango/theme.conf").expanduser().read_text()
        m = re.search(r"focus_color=0x([0-9a-fA-F]{6})", cfg)
        if m:
            return f"#{m.group(1)}"
    except Exception:
        pass
    return "#567ae6"

class KeybindsWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("MangoWM Kısayol Rehberi")
        self.resize(760, 480)
        self.accent = get_accent()
        self.init_ui()

    def init_ui(self):
        central = QWidget()
        self.setCentralWidget(central)
        layout = QHBoxLayout(central)
        layout.setContentsMargins(16, 16, 16, 16)
        layout.setSpacing(16)

        # Left Sidebar: Categories
        left_box = QVBoxLayout()
        cat_title = QLabel("KATEGORİLER")
        cat_title.setStyleSheet("font-weight: 800; font-size: 11px; color: #888899; letter-spacing: 1px;")
        left_box.addWidget(cat_title)

        self.cat_list = QListWidget()
        self.cat_list.setFixedWidth(230)
        self.cat_list.setFocusPolicy(Qt.StrongFocus)
        self.cat_list.setStyleSheet(f"""
            QListWidget {{
                background-color: #12141a;
                border: 1px solid #1f232e;
                border-radius: 8px;
                padding: 6px;
                color: #dcdde5;
                font-size: 13px;
                font-weight: 500;
            }}
            QListWidget::item {{
                padding: 10px 12px;
                border-radius: 6px;
                margin-bottom: 2px;
            }}
            QListWidget::item:selected {{
                background-color: {self.accent};
                color: #ffffff;
                font-weight: 700;
            }}
            QListWidget::item:hover:!selected {{
                background-color: #1c202a;
            }}
        """)
        for cat in DATA.keys():
            self.cat_list.addItem(QListWidgetItem(cat))
        self.cat_list.currentRowChanged.connect(self.display_category)
        left_box.addWidget(self.cat_list)

        hint = QLabel("Kapatmak için: ESC veya Q")
        hint.setStyleSheet("font-size: 11px; color: #666677;")
        left_box.addWidget(hint)
        layout.addLayout(left_box)

        # Right Panel: Items inside scroll area
        right_box = QVBoxLayout()
        self.header_lbl = QLabel("")
        self.header_lbl.setStyleSheet(f"font-size: 16px; font-weight: 800; color: {self.accent}; padding-bottom: 4px;")
        right_box.addWidget(self.header_lbl)

        self.scroll = QScrollArea()
        self.scroll.setWidgetResizable(True)
        self.scroll.setStyleSheet("""
            QScrollArea {
                background: #12141a;
                border: 1px solid #1f232e;
                border-radius: 8px;
            }
            QScrollBar:vertical {
                border: none;
                background: transparent;
                width: 6px;
                margin: 4px;
            }
            QScrollBar::handle:vertical {
                background: #2a3040;
                border-radius: 3px;
                min-height: 20px;
            }
        """)

        self.content_widget = QWidget()
        self.content_layout = QVBoxLayout(self.content_widget)
        self.content_layout.setContentsMargins(16, 16, 16, 16)
        self.content_layout.setSpacing(10)
        self.scroll.setWidget(self.content_widget)

        right_box.addWidget(self.scroll)
        layout.addLayout(right_box)

        self.setStyleSheet("""
            QMainWindow {
                background-color: #0b0d12;
            }
        """)

        self.cat_list.setCurrentRow(0)

    def display_category(self, idx):
        cat_name = list(DATA.keys())[idx]
        self.header_lbl.setText(cat_name)

        # Clear layout
        while self.content_layout.count():
            item = self.content_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        items = DATA[cat_name]
        for key, desc in items:
            row = QFrame()
            row.setStyleSheet("""
                QFrame {
                    background-color: #171a22;
                    border: 1px solid #202430;
                    border-radius: 6px;
                    padding: 4px;
                }
            """)
            rl = QHBoxLayout(row)
            rl.setContentsMargins(10, 6, 10, 6)
            rl.setSpacing(14)

            k_lbl = QLabel(key)
            k_lbl.setStyleSheet(f"""
                QLabel {{
                    background-color: #232736;
                    color: #ffffff;
                    font-family: "0xProto Nerd Font", monospace;
                    font-size: 12px;
                    font-weight: 700;
                    padding: 4px 10px;
                    border-radius: 4px;
                    border: 1px solid #32384a;
                }}
            """)
            k_lbl.setFixedWidth(230)

            d_lbl = QLabel(desc)
            d_lbl.setStyleSheet("color: #d8dae2; font-size: 13px;")

            rl.addWidget(k_lbl)
            rl.addWidget(d_lbl, 1)
            self.content_layout.addWidget(row)

        self.content_layout.addStretch()

    def keyPressEvent(self, event):
        if event.key() in (Qt.Key_Escape, Qt.Key_Q):
            self.close()
        else:
            super().keyPressEvent(event)

def main():
    app = QApplication(sys.argv)
    app.setApplicationName("keybinds-cheatsheet")
    app.setDesktopFileName("keybinds-cheatsheet")
    w = KeybindsWindow()
    w.show()
    sys.exit(app.exec())

if __name__ == "__main__":
    main()
