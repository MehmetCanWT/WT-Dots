import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid

Item {
    id: compact

    readonly property bool isVertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical

    Layout.minimumWidth: isVertical ? Kirigami.Units.gridUnit * 2 : rowLayout.implicitWidth + Kirigami.Units.smallSpacing * 2
    Layout.preferredWidth: Layout.minimumWidth
    Layout.minimumHeight: isVertical ? colLayout.implicitHeight + Kirigami.Units.smallSpacing * 2 : Kirigami.Units.gridUnit * 2
    Layout.preferredHeight: Layout.minimumHeight

    function getLoadColor(pct) {
        if (pct >= 85) return Kirigami.Theme.negativeTextColor
        if (pct >= 60) return Kirigami.Theme.neutralTextColor
        if (pct >= 35) return root.themeAccentSecondary
        return root.themeAccent
    }

    Rectangle {
        id: bgHighlight
        anchors.fill: parent
        anchors.margins: 2
        radius: 6
        color: mouseArea.containsMouse ? Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.15) : "transparent"
        border.color: mouseArea.containsMouse ? Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.3) : "transparent"
        border.width: 1
        Behavior on color { ColorAnimation { duration: 150 } }
    }

    // Horizontal Layout (Standard Taskbar / Bottom Panel)
    RowLayout {
        id: rowLayout
        visible: !compact.isVertical
        anchors.centerIn: parent
        spacing: Kirigami.Units.smallSpacing * 2

        // CPU Metric
        RowLayout {
            spacing: 4
            Text {
                text: "CPU"
                color: root.themeTextDim
                font.family: "monospace"
                font.pixelSize: 10
                font.bold: true
            }
            Text {
                text: Math.round(root.cpuUsage) + "%"
                color: compact.getLoadColor(root.cpuUsage)
                font.family: "monospace"
                font.pixelSize: 11
                font.bold: true
            }
            Rectangle {
                Layout.preferredWidth: 26
                Layout.preferredHeight: 5
                radius: 2
                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.15)
                Rectangle {
                    width: Math.max(2, parent.width * (root.cpuUsage / 100))
                    height: parent.height
                    radius: 2
                    color: compact.getLoadColor(root.cpuUsage)
                }
            }
        }

        // Memory Metric
        RowLayout {
            spacing: 4
            Text {
                text: "RAM"
                color: root.themeTextDim
                font.family: "monospace"
                font.pixelSize: 10
                font.bold: true
            }
            Text {
                text: Math.round(root.memPct) + "%"
                color: compact.getLoadColor(root.memPct)
                font.family: "monospace"
                font.pixelSize: 11
                font.bold: true
            }
            Rectangle {
                Layout.preferredWidth: 26
                Layout.preferredHeight: 5
                radius: 2
                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.15)
                Rectangle {
                    width: Math.max(2, parent.width * (root.memPct / 100))
                    height: parent.height
                    radius: 2
                    color: compact.getLoadColor(root.memPct)
                }
            }
        }

        // GPU Metric (if available)
        RowLayout {
            visible: root.gpuUsage >= 0
            spacing: 4
            Text {
                text: "GPU"
                color: root.themeTextDim
                font.family: "monospace"
                font.pixelSize: 10
                font.bold: true
            }
            Text {
                text: Math.round(root.gpuUsage) + "%"
                color: compact.getLoadColor(root.gpuUsage)
                font.family: "monospace"
                font.pixelSize: 11
                font.bold: true
            }
            Rectangle {
                Layout.preferredWidth: 26
                Layout.preferredHeight: 5
                radius: 2
                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.15)
                Rectangle {
                    width: Math.max(2, parent.width * (root.gpuUsage / 100))
                    height: parent.height
                    radius: 2
                    color: compact.getLoadColor(root.gpuUsage)
                }
            }
        }
    }

    // Vertical Layout (Side Panel)
    ColumnLayout {
        id: colLayout
        visible: compact.isVertical
        anchors.centerIn: parent
        spacing: 2

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "C:" + Math.round(root.cpuUsage) + "%"
            color: compact.getLoadColor(root.cpuUsage)
            font.family: "monospace"
            font.pixelSize: 10
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "R:" + Math.round(root.memPct) + "%"
            color: compact.getLoadColor(root.memPct)
            font.family: "monospace"
            font.pixelSize: 10
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton

        onClicked: function(mouse) {
            if (mouse.button === Qt.LeftButton) {
                root.expanded = !root.expanded
            } else if (mouse.button === Qt.MiddleButton) {
                root.launchBtop()
            }
        }
    }
}
