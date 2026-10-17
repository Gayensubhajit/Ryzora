import QtQuick
import QtQuick.Window
import QtMultimedia

Item {
    readonly property real s: Screen.height / 768
    anchors.fill: parent

    MediaPlayer {
        id: mediaplayer
        source: "bg.mp4"
        loops: MediaPlayer.Infinite
        videoOutput: videoOutput
        Component.onCompleted: play()
        onMediaStatusChanged: if (mediaStatus === MediaPlayer.LoadedMedia) play()
    }

    VideoOutput {
        id: videoOutput
        anchors.fill: parent
        fillMode: VideoOutput.PreserveAspectCrop
    }
}

