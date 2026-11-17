import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami
import org.kde.plasma.core as PlasmaCore

Rectangle {
    id: fullRoot

    implicitWidth: 620
    implicitHeight: 630
    Layout.minimumWidth: 540
    Layout.minimumHeight: 560
    Layout.preferredWidth: 620
    Layout.preferredHeight: 630

    color: Qt.rgba(root.themeBg.r, root.themeBg.g, root.themeBg.b, 0.95)
    radius: 12
    border.color: root.themeBorder
    border.width: 1

    function getLoadColor(pct) {
        if (pct >= 85) return Kirigami.Theme.negativeTextColor
        if (pct >= 60) return Kirigami.Theme.neutralTextColor
        if (pct >= 35) return root.themeAccentSecondary
        return root.themeAccent
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        // ================= HEADER =================
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 26

            Text {
                text: "btop++"
                color: root.themeAccent
                font.family: "monospace"
                font.pixelSize: 15
                font.bold: true
            }

            Rectangle {
                Layout.preferredWidth: 6
                Layout.preferredHeight: 6
                radius: 3
                color: root.themeAccentSecondary
            }

            Text {
                text: (root.isTurkish ? "çalışma: " : "uptime: ") + root.formattedUptime
                color: root.themeTextDim
                font.family: "monospace"
                font.pixelSize: 11
            }

            Item { Layout.fillWidth: true }

            // Launch btop Button
            Rectangle {
                Layout.preferredHeight: 24
                Layout.preferredWidth: btnText.implicitWidth + 16
                radius: 4
                color: btnMouse.containsMouse ? Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.25) : Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.08)
                border.color: root.themeAccent
                border.width: 1

                Text {
                    id: btnText
                    anchors.centerIn: parent
                    text: root.isTurkish ? "Terminalde Aç 󰆍" : "Open in Terminal 󰆍"
                    color: root.themeText
                    font.family: "monospace"
                    font.pixelSize: 10
                    font.bold: true
                }

                MouseArea {
                    id: btnMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.launchBtop()
                }
            }
        }

        // ================= CPU BOX =================
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 115
            radius: 8
            color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.04)
            border.color: Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.22)
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 5

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "┌─ CPU"
                        color: root.themeAccent
                        font.family: "monospace"
                        font.pixelSize: 11
                        font.bold: true
                    }
                    Text {
                        text: "[ " + Math.round(root.cpuUsage) + "% ]"
                        color: fullRoot.getLoadColor(root.cpuUsage)
                        font.family: "monospace"
                        font.pixelSize: 12
                        font.bold: true
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: (root.cpuFreq > 0 ? root.cpuFreq + " GHz  " : "") + (root.cpuTemp >= 0 ? root.cpuTemp + "°C" : "")
                        color: root.themeTextDim
                        font.family: "monospace"
                        font.pixelSize: 11
                    }
                }

                // Canvas Graph
                Canvas {
                    id: cpuCanvas
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    Connections {
                        target: root
                        function onCpuUsageChanged() { cpuCanvas.requestPaint() }
                        function onThemeAccentChanged() { cpuCanvas.requestPaint() }
                    }

                    onPaint: {
                        var ctx = getContext("2d")
                        ctx.clearRect(0, 0, width, height)

                        var points = root.cpuHistory
                        if (!points || points.length < 2) return

                        var step = width / (points.length - 1)

                        // Area Gradient fill
                        var grad = ctx.createLinearGradient(0, 0, 0, height)
                        var col = root.themeAccent
                        grad.addColorStop(0, Qt.rgba(col.r, col.g, col.b, 0.45))
                        grad.addColorStop(1, Qt.rgba(col.r, col.g, col.b, 0.04))

                        ctx.beginPath()
                        ctx.moveTo(0, height)
                        for (var i = 0; i < points.length; i++) {
                            var y = height - (points[i] / 100 * (height - 4))
                            ctx.lineTo(i * step, y)
                        }
                        ctx.lineTo(width, height)
                        ctx.closePath()
                        ctx.fillStyle = grad
                        ctx.fill()

                        // Line
                        ctx.beginPath()
                        for (var j = 0; j < points.length; j++) {
                            var ly = height - (points[j] / 100 * (height - 4))
                            if (j === 0) ctx.moveTo(0, ly)
                            else ctx.lineTo(j * step, ly)
                        }
                        ctx.strokeStyle = col
                        ctx.lineWidth = 1.8
                        ctx.stroke()
                    }
                }

                // Mini Cores Grid
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 3
                    Repeater {
                        model: root.cpuCores
                        delegate: Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 5
                            radius: 1
                            color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.12)
                            Rectangle {
                                width: Math.max(1, parent.width * (modelData / 100))
                                height: parent.height
                                radius: 1
                                color: fullRoot.getLoadColor(modelData)
                            }
                        }
                    }
                }
            }
        }

        // ================= MEMORY & DISKS & GPU & NET (2-COLUMN GRID) =================
        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 165
            spacing: 10

            // Left Box: MEM & DISK
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 8
                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.04)
                border.color: Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.22)
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 5

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: root.isTurkish ? "┌─ BELLEK" : "┌─ MEMORY"
                            color: root.themeAccentSecondary
                            font.family: "monospace"
                            font.pixelSize: 11
                            font.bold: true
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: root.memUsed.toFixed(1) + " / " + root.memTotal.toFixed(1) + " GiB (" + Math.round(root.memPct) + "%)"
                            color: fullRoot.getLoadColor(root.memPct)
                            font.family: "monospace"
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }

                    // RAM Progress Bar
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 6
                        radius: 3
                        color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.12)
                        Rectangle {
                            width: Math.max(3, parent.width * (root.memPct / 100))
                            height: parent.height
                            radius: 3
                            color: fullRoot.getLoadColor(root.memPct)
                        }
                    }

                    // Swap Row (Aligned with Disks)
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Text {
                            Layout.preferredWidth: 46
                            text: (root.isTurkish ? "TAKAS" : "SWAP") + ":"
                            color: root.themeTextDim
                            font.family: "monospace"
                            font.pixelSize: 10
                        }
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 5
                            radius: 2
                            color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.08)
                            Rectangle {
                                width: Math.max(1, parent.width * (root.swapPct / 100))
                                height: parent.height
                                radius: 2
                                color: root.themeAccentSecondary
                            }
                        }
                        Text {
                            Layout.preferredWidth: 125
                            horizontalAlignment: Text.AlignRight
                            text: root.swapUsed.toFixed(1) + "/" + root.swapTotal.toFixed(1) + "G (" + Math.round(root.swapPct) + "%)"
                            color: root.themeTextDim
                            font.family: "monospace"
                            font.pixelSize: 10
                        }
                    }

                    // Disks List (Aligned to same pixel coordinates)
                    Repeater {
                        model: root.disks
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: 6
                            Text {
                                Layout.preferredWidth: 46
                                text: modelData.name + ":"
                                color: root.themeAccent
                                font.family: "monospace"
                                font.pixelSize: 10
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 5
                                radius: 2
                                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.08)
                                Rectangle {
                                    width: Math.max(2, parent.width * (modelData.pct / 100))
                                    height: parent.height
                                    radius: 2
                                    color: root.themeAccent
                                }
                            }
                            Text {
                                Layout.preferredWidth: 125
                                horizontalAlignment: Text.AlignRight
                                text: modelData.used + "/" + modelData.total + "G (" + Math.round(modelData.pct) + "%)"
                                color: root.themeTextDim
                                font.family: "monospace"
                                font.pixelSize: 10
                            }
                        }
                    }
                }
            }

            // Right Box: GPU & NET
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: 8
                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.04)
                border.color: Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.22)
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 4

                    // GPU Sub-group (Compact & tight spacing)
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: "┌─ GPU"
                                color: root.themeAccent
                                font.family: "monospace"
                                font.pixelSize: 11
                                font.bold: true
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: root.gpuUsage >= 0 ? Math.round(root.gpuUsage) + "%  " + (root.gpuTemp >= 0 ? root.gpuTemp + "°C" : "") : "N/A"
                                color: fullRoot.getLoadColor(root.gpuUsage)
                                font.family: "monospace"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: root.gpuName
                            color: root.themeTextDim
                            font.family: "monospace"
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }

                        // VRAM Row (Aligned with disk style)
                        RowLayout {
                            visible: root.vramTotal > 0
                            Layout.fillWidth: true
                            spacing: 6
                            Text {
                                Layout.preferredWidth: 46
                                text: "VRAM:"
                                color: root.themeTextDim
                                font.family: "monospace"
                                font.pixelSize: 10
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 5
                                radius: 2
                                color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.08)
                                Rectangle {
                                    width: Math.max(2, parent.width * (root.vramPct / 100))
                                    height: parent.height
                                    radius: 2
                                    color: root.themeAccent
                                }
                            }
                            Text {
                                Layout.preferredWidth: 100
                                horizontalAlignment: Text.AlignRight
                                text: root.vramUsed.toFixed(1) + "/" + root.vramTotal.toFixed(1) + "G"
                                color: root.themeTextDim
                                font.family: "monospace"
                                font.pixelSize: 10
                            }
                        }
                    }

                    // Spacer between GPU and NET sections
                    Item { Layout.fillHeight: true }

                    // NET Sub-group
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: "┌─ NET: " + root.netIface
                                color: root.themeAccentSecondary
                                font.family: "monospace"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                text: "↓ " + root.rxRate
                                color: root.themeAccent
                                font.family: "monospace"
                                font.pixelSize: 11
                                font.bold: true
                            }
                            Item { Layout.fillWidth: true }
                            Text {
                                text: "↑ " + root.txRate
                                color: root.themeAccentSecondary
                                font.family: "monospace"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }
                    }
                }
            }
        }

        // ================= TOP PROCESSES BOX =================
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 180
            radius: 8
            color: Qt.rgba(root.themeText.r, root.themeText.g, root.themeText.b, 0.04)
            border.color: Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.22)
            border.width: 1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 5

                // Section Title
                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: root.isTurkish ? "┌─ EN ÇOK KULLANANLAR" : "┌─ TOP PROCESSES"
                        color: root.themeAccent
                        font.family: "monospace"
                        font.pixelSize: 11
                        font.bold: true
                    }
                    Item { Layout.fillWidth: true }
                }

                // Table Column Headers (Aligned to data columns!)
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        Layout.preferredWidth: 60
                        text: "PID"
                        color: root.themeTextDim
                        font.family: "monospace"
                        font.pixelSize: 10
                        font.bold: true
                    }
                    Text {
                        Layout.preferredWidth: 55
                        text: "CPU%"
                        color: root.themeTextDim
                        font.family: "monospace"
                        font.pixelSize: 10
                        font.bold: true
                    }
                    Text {
                        Layout.preferredWidth: 55
                        text: "RAM%"
                        color: root.themeTextDim
                        font.family: "monospace"
                        font.pixelSize: 10
                        font.bold: true
                    }
                    Text {
                        Layout.fillWidth: true
                        text: root.isTurkish ? "KOMUT" : "COMMAND"
                        color: root.themeTextDim
                        font.family: "monospace"
                        font.pixelSize: 10
                        font.bold: true
                    }
                }

                // Separator line
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Qt.rgba(root.themeAccent.r, root.themeAccent.g, root.themeAccent.b, 0.15)
                }

                // Processes List
                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: root.procs
                    spacing: 3
                    delegate: RowLayout {
                        width: parent.width
                        spacing: 8

                        Text {
                            Layout.preferredWidth: 60
                            text: modelData.pid
                            color: root.themeTextDim
                            font.family: "monospace"
                            font.pixelSize: 11
                        }
                        Text {
                            Layout.preferredWidth: 55
                            text: modelData.cpu + "%"
                            color: fullRoot.getLoadColor(parseFloat(modelData.cpu) || 0)
                            font.family: "monospace"
                            font.pixelSize: 11
                            font.bold: true
                        }
                        Text {
                            Layout.preferredWidth: 55
                            text: modelData.mem + "%"
                            color: root.themeAccentSecondary
                            font.family: "monospace"
                            font.pixelSize: 11
                        }
                        Text {
                            Layout.fillWidth: true
                            text: modelData.name
                            color: root.themeText
                            font.family: "monospace"
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }
}
