import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasma5support as Plasma5Support

PlasmoidItem {
    id: root

    // System metrics
    property real cpuUsage: 0
    property var cpuCores: []
    property real cpuTemp: -1
    property real cpuFreq: 0
    property var cpuHistory: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

    property real memUsed: 0
    property real memTotal: 0
    property real memPct: 0
    property real swapUsed: 0
    property real swapTotal: 0
    property real swapPct: 0

    property string gpuName: "N/A"
    property real gpuUsage: -1
    property real gpuTemp: -1
    property real vramUsed: -1
    property real vramTotal: -1
    property real vramPct: 0

    property string netIface: "--"
    property string rxRate: "0 B/s"
    property string txRate: "0 B/s"
    property real rxRaw: 0
    property real txRaw: 0
    property var rxHistory: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    property var txHistory: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

    property var disks: []
    property var procs: []
    property int batteryPct: -1
    property string batteryStatus: ""
    property string uptime: "--"
    property int uptimeSecs: 0
    property bool isTurkish: true

    readonly property string formattedUptime: root.uptime

    // Dynamic theme colors synced to wallpaper palette with Kirigami.Theme fallbacks
    property var themePalette: null
    readonly property color themeBg: themePalette && themePalette.bg ? themePalette.bg : Kirigami.Theme.backgroundColor
    readonly property color themeAccent: themePalette && themePalette.accent ? themePalette.accent : Kirigami.Theme.highlightColor
    readonly property color themeAccentSecondary: themePalette && themePalette.accent_secondary ? themePalette.accent_secondary : Kirigami.Theme.positiveTextColor
    readonly property color themeBorder: themePalette && themePalette.border ? themePalette.border : Qt.rgba(Kirigami.Theme.highlightColor.r, Kirigami.Theme.highlightColor.g, Kirigami.Theme.highlightColor.b, 0.25)
    readonly property color themeBoxBg: Qt.rgba(themeBg.r + 0.05, themeBg.g + 0.05, themeBg.b + 0.05, 0.6)
    readonly property color themeText: themePalette && themePalette.text_color ? themePalette.text_color : Kirigami.Theme.textColor
    readonly property color themeTextDim: themePalette && themePalette.text_dim ? themePalette.text_dim : Kirigami.Theme.disabledTextColor
    readonly property color themeTextMuted: themePalette && themePalette.text_muted ? themePalette.text_muted : Kirigami.Theme.disabledTextColor

    // Configuration / Sizing
    preferredRepresentation: Plasmoid.formFactor === PlasmaCore.Types.Planar ? fullRepresentation : compactRepresentation
    Plasmoid.backgroundHints: PlasmaCore.Types.DefaultBackground | PlasmaCore.Types.ConfigurableBackground

    compactRepresentation: CompactRepresentation {}
    fullRepresentation: FullRepresentation {}

    function getStatsCmd() {
        return "wt-rust btop-stats"
    }

    function launchBtop() {
        launcherSource.connectSource("kitty -e btop || konsole -e btop || btop")
    }

    function updateHistory(histArray, newVal) {
        var arr = histArray.slice()
        arr.shift()
        arr.push(newVal)
        return arr
    }

    function parseStats(jsonStr) {
        if (!jsonStr || jsonStr.trim() === "") return
        try {
            var data = JSON.parse(jsonStr)
            if (data.cpu) {
                root.cpuUsage = data.cpu.total || 0
                root.cpuCores = data.cpu.cores || []
                root.cpuTemp = data.cpu.temp !== undefined ? data.cpu.temp : -1
                root.cpuFreq = data.cpu.freq || 0
                root.cpuHistory = updateHistory(root.cpuHistory, root.cpuUsage)
            }
            if (data.mem) {
                root.memUsed = data.mem.used || 0
                root.memTotal = data.mem.total || 0
                root.memPct = data.mem.pct || 0
                root.swapUsed = data.mem.swap_used || 0
                root.swapTotal = data.mem.swap_total || 0
                root.swapPct = data.mem.swap_pct || 0
            }
            if (data.gpu) {
                root.gpuName = data.gpu.name || "N/A"
                root.gpuUsage = data.gpu.usage !== undefined ? data.gpu.usage : -1
                root.gpuTemp = data.gpu.temp !== undefined ? data.gpu.temp : -1
                root.vramUsed = data.gpu.vram_used !== undefined ? data.gpu.vram_used : -1
                root.vramTotal = data.gpu.vram_total !== undefined ? data.gpu.vram_total : -1
                root.vramPct = data.gpu.vram_pct || 0
            }
            if (data.net) {
                root.netIface = data.net.iface || "--"
                root.rxRate = data.net.rx_rate || "0 B/s"
                root.txRate = data.net.tx_rate || "0 B/s"
                root.rxRaw = data.net.rx_raw || 0
                root.txRaw = data.net.tx_raw || 0
                root.rxHistory = updateHistory(root.rxHistory, root.rxRaw)
                root.txHistory = updateHistory(root.txHistory, root.txRaw)
            }
            if (data.disks) root.disks = data.disks
            if (data.procs) root.procs = data.procs
            if (data.battery) {
                root.batteryPct = data.battery.pct !== undefined ? data.battery.pct : -1
                root.batteryStatus = data.battery.status || ""
            }
            if (data.uptime) root.uptime = data.uptime
            if (data.uptime_secs !== undefined) root.uptimeSecs = data.uptime_secs
            if (data.is_turkish !== undefined) root.isTurkish = data.is_turkish
            if (data.palette) root.themePalette = data.palette
        } catch (e) {
            console.warn("BTOP_Widget: JSON parse error", e)
        }
    }

    // Command Runner DataSource
    Plasma5Support.DataSource {
        id: statsDataSource
        engine: "executable"
        connectedSources: []

        onNewData: function(source, data) {
            var stdout = data["stdout"] || ""
            root.parseStats(stdout)
            disconnectSource(source)
        }
    }

    Plasma5Support.DataSource {
        id: launcherSource
        engine: "executable"
        connectedSources: []
        onNewData: function(source, data) {
            disconnectSource(source)
        }
    }

    Timer {
        id: updateTimer
        interval: 2000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            statsDataSource.connectSource(root.getStatsCmd())
        }
    }

    Plasmoid.contextualActions: [
        PlasmaCore.Action {
            text: root.isTurkish ? "btop'u Terminalde Aç" : "Open btop in Terminal"
            icon.name: "utilities-terminal"
            onTriggered: root.launchBtop()
        }
    ]
}
