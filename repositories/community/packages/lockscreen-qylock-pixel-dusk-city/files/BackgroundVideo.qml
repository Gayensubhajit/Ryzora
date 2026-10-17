import QtQuick
import QtQuick.Window
import QtMultimedia

Item {
    readonly property real s: Screen.height / 768
    anchors.fill: parent

        MediaPlayer {
        audioOutput: AudioOutput { muted: true; volume: 0.0 }
            id: mediaplayer
            source: "bg.mp4"
            autoPlay: true
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

