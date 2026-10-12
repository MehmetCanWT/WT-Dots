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
    property color bg: (pal && pal.bg) ? pal.bg : "#0a0c10"
    Behavior on accent { ColorAnimation { duration: 250; easing.type: Easing.OutCubic } }

    property int cur: bridge.initialIndex
    property bool bgToggle: false
    property string srcA: (bridge.items.length > 0 && bridge.initialIndex < bridge.items.length) ? bridge.items[bridge.initialIndex].path : ""
    property string srcB: srcA

    function updateBg(idx) {
        if (bridge.items.length === 0 || idx < 0 || idx >= bridge.items.length) return;
        var p = bridge.items[idx].path;
        if (!bgToggle) {
            srcB = p;
            bgToggle = true;
        } else {
            srcA = p;
            bgToggle = false;
        }
    }

    // -----------------------------------------------------
    // LOSSLESS FULL-SCREEN DYNAMIC BACKGROUND PREVIEW
    // -----------------------------------------------------
    Rectangle {
        anchors.fill: parent
        color: "#0a0c10"
    }

    Item {
        id: bgContainer
        anchors.fill: parent

        Image {
            id: bgA
            anchors.fill: parent
            source: srcA
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: true
            smooth: true
            opacity: bgToggle ? 0.0 : 1.0
            Behavior on opacity { NumberAnimation { duration: 320; easing.type: Easing.InOutQuad } }
        }

        Image {
            id: bgB
            anchors.fill: parent
            source: srcB
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: true
            smooth: true
            opacity: bgToggle ? 1.0 : 0.0
            Behavior on opacity { NumberAnimation { duration: 320; easing.type: Easing.InOutQuad } }
        }
    }

    // Subtle dark gradient along bottom dock to enhance card contrast
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 380
        gradient: Gradient {
            GradientStop { position: 0.0; color: "transparent" }
            GradientStop { position: 0.4; color: Qt.rgba(0, 0, 0, 0.40) }
            GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.85) }
        }
    }

    // Top Right Close Button (✕)
    Rectangle {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 28
        width: 38
        height: 38
        radius: 19
        color: closeMa.containsMouse ? Qt.rgba(1, 1, 1, 0.25) : Qt.rgba(0, 0, 0, 0.55)
        border.color: Qt.rgba(1, 1, 1, 0.25)
        border.width: 1
        z: 200

        Text {
            anchors.centerIn: parent
            text: "✕"
            color: "white"
            font.pixelSize: 16
        }

        MouseArea {
            id: closeMa
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: bridge.cancel()
        }
    }

    // -----------------------------------------------------
    // HYPRQUICKPAPER BOTTOM DOCK CAROUSEL (PATHVIEW)
    // -----------------------------------------------------
    PathView {
        id: pv
        anchors.fill: parent
        focus: true
        interactive: false

        model: bridge.items
        pathItemCount: 13
        preferredHighlightBegin: 0.5
        preferredHighlightEnd: 0.5
        highlightRangeMode: PathView.StrictlyEnforceRange
        highlightMoveDuration: 280
        snapMode: PathView.SnapToItem

        currentIndex: bridge.initialIndex

        onCurrentIndexChanged: {
            updateBg(currentIndex);
            bridge.preview(currentIndex);
        }

        path: Path {
            startX: -win.width * 0.12
            startY: win.height - 150

            PathAttribute { name: "itemScale"; value: 1.0 }
            PathAttribute { name: "itemOpacity"; value: 0.92 }
            PathAttribute { name: "itemZ"; value: 1 }
            PathPercent { value: 0.0 }

            PathLine { x: win.width * 0.5; y: win.height - 150 }
            PathAttribute { name: "itemScale"; value: 1.28 }
            PathAttribute { name: "itemOpacity"; value: 1.0 }
            PathAttribute { name: "itemZ"; value: 100 }
            PathPercent { value: 0.5 }

            PathLine { x: win.width * 1.12; y: win.height - 150 }
            PathAttribute { name: "itemScale"; value: 1.0 }
            PathAttribute { name: "itemOpacity"; value: 0.92 }
            PathAttribute { name: "itemZ"; value: 1 }
            PathPercent { value: 1.0 }
        }

        delegate: Item {
            id: card
            width: 310
            height: 190

            property bool isCurrent: PathView.isCurrentItem

            scale: PathView.itemScale
            opacity: PathView.itemOpacity
            z: isCurrent ? 100 : PathView.itemZ

            Behavior on scale { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }

            // Parallelogram shear transformation (HyprQuickPaper style)
            transform: Matrix4x4 {
                matrix: Qt.matrix4x4(
                    1, -0.22, 0, 0.22 * 190 / 2,
                    0, 1, 0, 0,
                    0, 0, 1, 0,
                    0, 0, 0, 1
                )
            }

            Rectangle {
                id: cardFrame
                anchors.fill: parent
                radius: 14
                color: "#12141a"
                clip: true

                Image {
                    anchors.fill: parent
                    source: modelData.thumb
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    cache: true
                }

                // Video indicator badge
                Rectangle {
                    visible: modelData.video
                    anchors.top: parent.top
                    anchors.left: parent.left
                    anchors.margins: 10
                    width: 28
                    height: 28
                    radius: 14
                    color: Qt.rgba(0, 0, 0, 0.70)
                    border.color: win.accent
                    border.width: 1

                    Text {
                        anchors.centerIn: parent
                        text: "▶"
                        color: "white"
                        font.pixelSize: 12
                    }
                }
            }

            // Outline highlight border
            Rectangle {
                anchors.fill: parent
                radius: 14
                color: "transparent"
                border.width: card.isCurrent ? 2.5 : 1.0
                border.color: card.isCurrent ? win.accent : Qt.rgba(1, 1, 1, 0.20)
                Behavior on border.color { ColorAnimation { duration: 200 } }
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
            else if (e.key === Qt.Key_Up || e.key === Qt.Key_U) {
                for (var i = 0; i < 4; i++) pv.incrementCurrentIndex();
            } else if (e.key === Qt.Key_Down) {
                for (var j = 0; j < 4; j++) pv.decrementCurrentIndex();
            } else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter || e.key === Qt.Key_Space)
                bridge.commit(pv.currentIndex);
            else if (e.key === Qt.Key_Escape)
                bridge.cancel();
        }
    }

    // -----------------------------------------------------
    // PAGINATION / POSITION INDICATOR DOTS
    // -----------------------------------------------------
    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        spacing: 6
        z: 150

        Repeater {
            model: Math.min(6, bridge.items.length)
            Rectangle {
                property int activeDot: bridge.items.length > 0 ? (pv.currentIndex % Math.min(6, bridge.items.length)) : 0
                width: index === activeDot ? 18 : 6
                height: 6
                radius: 3
                color: index === activeDot ? win.accent : Qt.rgba(1, 1, 1, 0.35)
                Behavior on width { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }
                Behavior on color { ColorAnimation { duration: 200 } }
            }
        }
    }

    // -----------------------------------------------------
    // LEFT & RIGHT NAVIGATION CHEVRON BUTTONS
    // -----------------------------------------------------
    Rectangle {
        anchors.left: parent.left
        anchors.leftMargin: 32
        anchors.verticalCenter: parent.verticalCenter
        width: 48
        height: 48
        radius: 24
        color: leftMa.containsMouse ? Qt.rgba(1, 1, 1, 0.25) : Qt.rgba(0, 0, 0, 0.45)
        border.color: Qt.rgba(1, 1, 1, 0.20)
        border.width: 1
        z: 200

        Text {
            anchors.centerIn: parent
            text: "‹"
            font.pixelSize: 32
            color: "white"
        }

        MouseArea {
            id: leftMa
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: pv.decrementCurrentIndex()
        }
    }

    Rectangle {
        anchors.right: parent.right
        anchors.rightMargin: 32
        anchors.verticalCenter: parent.verticalCenter
        width: 48
        height: 48
        radius: 24
        color: rightMa.containsMouse ? Qt.rgba(1, 1, 1, 0.25) : Qt.rgba(0, 0, 0, 0.45)
        border.color: Qt.rgba(1, 1, 1, 0.20)
        border.width: 1
        z: 200

        Text {
            anchors.centerIn: parent
            text: "›"
            font.pixelSize: 32
            color: "white"
        }

        MouseArea {
            id: rightMa
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: pv.incrementCurrentIndex()
        }
    }

    // Mouse wheel handler to scroll anywhere on screen
    WheelHandler {
        onWheel: (e) => (e.angleDelta.y + e.angleDelta.x) < 0 ? pv.incrementCurrentIndex() : pv.decrementCurrentIndex()
    }
}
