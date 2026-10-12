import QtQuick
import QtQuick.Window
import QtQuick.Effects

Window {
    id: win
    visible: true
    color: "#0a0c10"
    flags: Qt.FramelessWindowHint
    visibility: Window.FullScreen
    title: "wallpaper-picker"

    property var pal: bridge.pal
    property color accent: (pal && pal.accent) ? pal.accent : "#7798d9"
    property color bg: (pal && pal.bg) ? pal.bg : "#101216"
    Behavior on accent { ColorAnimation { duration: 300; easing.type: Easing.OutCubic } }
    Behavior on bg { ColorAnimation { duration: 300; easing.type: Easing.OutCubic } }

    property int cur: pv.currentIndex
    property bool flip: false
    property string srcA: bridge.items.length > 0 ? bridge.items[0].thumb : ""
    property string srcB: bridge.items.length > 0 ? bridge.items[0].thumb : ""
    onCurChanged: {
        if (flip) srcA = bridge.items[cur].thumb;
        else srcB = bridge.items[cur].thumb;
        flip = !flip;
    }

    // 100% OPAQUE base: completely hides underlying windows
    Rectangle {
        anchors.fill: parent
        color: "#0a0c10"
    }

    // Blurred dynamic backdrop matching selected wallpaper
    Item {
        id: backdrop
        anchors.fill: parent
        opacity: 0.40
        layer.enabled: true
        layer.effect: MultiEffect {
            blurEnabled: true
            blur: 1.0
            blurMax: 64
            saturation: 0.45
            brightness: -0.05
        }
        Image {
            anchors.fill: parent
            source: srcA
            fillMode: Image.PreserveAspectCrop
            opacity: flip ? 0 : 1
            Behavior on opacity { NumberAnimation { duration: 350; easing.type: Easing.InOutQuad } }
        }
        Image {
            anchors.fill: parent
            source: srcB
            fillMode: Image.PreserveAspectCrop
            opacity: flip ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: 350; easing.type: Easing.InOutQuad } }
        }
    }

    // Subtle dark vignette for contrast
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.7) }
            GradientStop { position: 0.5; color: Qt.rgba(0, 0, 0, 0.3) }
            GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.8) }
        }
    }

    // Top Category Header
    Row {
        anchors.top: parent.top
        anchors.topMargin: 40
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: 12
        Text {
            text: "WALLPAPER PICKER"
            font.pixelSize: 16
            font.bold: true
            font.letterSpacing: 4
            color: win.accent
        }
    }

    // Main 3D-feel Center Carousel PathView
    PathView {
        id: pv
        anchors.centerIn: parent
        width: win.width
        height: 520
        model: bridge.items
        pathItemCount: 5
        preferredHighlightBegin: 0.5
        preferredHighlightEnd: 0.5
        highlightRangeMode: PathView.StrictlyEnforceRange
        highlightMoveDuration: 300
        snapMode: PathView.SnapToItem
        focus: true

        onCurrentIndexChanged: bridge.preview(currentIndex)
        Component.onCompleted: bridge.preview(currentIndex)

        path: Path {
            startX: -320; startY: 260
            PathAttribute { name: "z"; value: 0 }
            PathAttribute { name: "sc"; value: 0.45 }
            PathAttribute { name: "op"; value: 0.0 }

            PathLine { x: pv.width * 0.18; y: 260 }
            PathAttribute { name: "z"; value: 3 }
            PathAttribute { name: "sc"; value: 0.62 }
            PathAttribute { name: "op"; value: 0.55 }

            PathLine { x: pv.width * 0.5; y: 260 }
            PathAttribute { name: "z"; value: 10 }
            PathAttribute { name: "sc"; value: 1.0 }
            PathAttribute { name: "op"; value: 1.0 }

            PathLine { x: pv.width * 0.82; y: 260 }
            PathAttribute { name: "z"; value: 3 }
            PathAttribute { name: "sc"; value: 0.62 }
            PathAttribute { name: "op"; value: 0.55 }

            PathLine { x: pv.width + 320; y: 260 }
            PathAttribute { name: "z"; value: 0 }
            PathAttribute { name: "sc"; value: 0.45 }
            PathAttribute { name: "op"; value: 0.0 }
        }

        delegate: Item {
            id: card
            width: 760
            height: 428
            z: PathView.z
            scale: PathView.sc
            opacity: PathView.op

            Rectangle {
                id: frame
                anchors.fill: parent
                radius: 18
                color: "#161920"
                border.width: card.PathView.isCurrentItem ? 3 : 1
                border.color: card.PathView.isCurrentItem ? win.accent : Qt.rgba(1, 1, 1, 0.15)
                clip: true

                layer.enabled: true
                layer.effect: MultiEffect {
                    shadowEnabled: true
                    shadowBlur: 1.0
                    shadowOpacity: card.PathView.isCurrentItem ? 0.85 : 0.4
                    shadowVerticalOffset: 12
                    shadowColor: card.PathView.isCurrentItem ? Qt.rgba(0, 0, 0, 0.95) : Qt.rgba(0, 0, 0, 0.6)
                }

                Image {
                    anchors.fill: parent
                    anchors.margins: frame.border.width
                    source: modelData.thumb
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                }

                // Video indicator badge
                Rectangle {
                    visible: modelData.video
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 16
                    width: 44
                    height: 44
                    radius: 22
                    color: Qt.rgba(0, 0, 0, 0.65)
                    border.color: win.accent
                    border.width: 1
                    Text {
                        anchors.centerIn: parent
                        text: "▶"
                        color: "white"
                        font.pixelSize: 18
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    if (pv.currentIndex === index)
                        bridge.commit(index);
                    else
                        pv.currentIndex = index;
                }
            }
        }

        Keys.onPressed: (e) => {
            if (e.key === Qt.Key_Right || e.key === Qt.Key_D || e.key === Qt.Key_L)
                pv.incrementCurrentIndex();
            else if (e.key === Qt.Key_Left || e.key === Qt.Key_A || e.key === Qt.Key_H)
                pv.decrementCurrentIndex();
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space)
                bridge.commit(pv.currentIndex);
            else if (e.key === Qt.Key_Escape)
                bridge.cancel();
        }
    }

    // Left Navigation Arrow Button
    Rectangle {
        anchors.left: parent.left
        anchors.leftMargin: 48
        anchors.verticalCenter: parent.verticalCenter
        width: 52
        height: 52
        radius: 26
        color: lma.containsMouse ? Qt.rgba(1, 1, 1, 0.25) : Qt.rgba(0, 0, 0, 0.45)
        border.color: Qt.rgba(1, 1, 1, 0.2)
        border.width: 1
        Text {
            anchors.centerIn: parent
            text: "‹"
            font.pixelSize: 34
            color: "white"
        }
        MouseArea {
            id: lma
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: pv.decrementCurrentIndex()
        }
    }

    // Right Navigation Arrow Button
    Rectangle {
        anchors.right: parent.right
        anchors.rightMargin: 48
        anchors.verticalCenter: parent.verticalCenter
        width: 52
        height: 52
        radius: 26
        color: rma.containsMouse ? Qt.rgba(1, 1, 1, 0.25) : Qt.rgba(0, 0, 0, 0.45)
        border.color: Qt.rgba(1, 1, 1, 0.2)
        border.width: 1
        Text {
            anchors.centerIn: parent
            text: "›"
            font.pixelSize: 34
            color: "white"
        }
        MouseArea {
            id: rma
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: pv.incrementCurrentIndex()
        }
    }

    WheelHandler {
        onWheel: (e) => (e.angleDelta.y + e.angleDelta.x) < 0 ? pv.incrementCurrentIndex() : pv.decrementCurrentIndex()
    }

    // Bottom Details and Palette Preview
    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 40
        spacing: 12

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            color: "white"
            font.pixelSize: 22
            font.bold: true
            text: bridge.items.length > 0 ? bridge.items[pv.currentIndex].name : ""
        }

        // Color palette preview capsules
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 8
            Repeater {
                model: pal ? [pal.bg, pal.bg2, pal.muted, pal.fg, pal.accent, pal.accent2] : []
                Rectangle {
                    width: 28
                    height: 28
                    radius: 14
                    color: modelData
                    border.color: Qt.rgba(1, 1, 1, 0.35)
                    border.width: 1
                }
            }
        }

        // Instruction info in English
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            color: Qt.rgba(1, 1, 1, 0.75)
            font.pixelSize: 13
            text: (pv.currentIndex + 1) + " of " + bridge.items.length + "   •   A / D or ← / → to navigate   •   Enter to select   •   Esc to cancel"
        }
    }
}
