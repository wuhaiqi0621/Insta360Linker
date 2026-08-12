import AppKit
import Foundation
import SwiftUI

@MainActor
final class AppModel: ObservableObject {
    @Published var section: AppSection = .media
    @Published var host = "192.168.42.1"
    @Published var connectionMessage = "请先连接相机 Wi-Fi，然后连接设备"
    @Published var connected = false
    @Published var controlReady = false
    @Published var recording = false
    @Published var previewing = false
    @Published var captureMode = "photo"
    @Published var zoom = 1.0
    @Published var gimbalSpeed = 3
    @Published var videoFormat = "4k_16_9"
    @Published var videoFPS = 30
    @Published var storage = "all"
    @Published var mediaFilter = "all"
    @Published var media: [MediaItem] = []
    @Published var selectedMedia: Set<String> = []
    @Published var thumbnails: [String: NSImage] = [:]
    @Published var watermarkStyles: [WatermarkStyleOption] = []
    @Published var frameBackgrounds: [FrameBackgroundOption] = []
    @Published var watermarkInput = ""
    @Published var watermarkStyle = "luna-ultra-cn"
    @Published var watermarkPosition = "bottom-center"
    @Published var frameBackground = "black"
    @Published var momentPreset = "official"
    @Published var momentImage = ""
    @Published var watermarkPreview: NSImage?
    @Published var micDevices: [MicDevice] = []
    @Published var micDetails: [String] = []
    @Published var selectedMic: MicDevice?
    @Published var busy: Set<String> = []
    @Published var notice: String?
    @Published var errorMessage: String?
    @Published var confirmingDelete = false

    let backend = BackendClient()

    var visibleMedia: [MediaItem] {
        media.filter { mediaFilter == "all" || $0.kind == mediaFilter }
    }

    var availableVideoFPS: [Int] {
        switch videoFormat {
        case "8k_16_9": [30, 25, 24]
        case "3k_1_1": [60, 50, 48, 30, 25, 24]
        default: [120, 100, 60, 50, 48, 30, 25, 24]
        }
    }

    func syncVideoFPS() {
        if !availableVideoFPS.contains(videoFPS) {
            videoFPS = availableVideoFPS.first ?? 30
        }
    }

    init() {
        loadWatermarkCatalog()
    }

    func perform(_ command: String, payload: [String: Any] = [:], completion: (([String: Any]) -> Void)? = nil) {
        busy.insert(command)
        backend.call(command, payload: payload) { [weak self] result in
            guard let self else { return }
            self.busy.remove(command)
            switch result {
            case .success(let data):
                completion?(data)
            case .failure(let error):
                self.errorMessage = error.localizedDescription
            }
        }
    }

    func connectMedia() {
        perform("detect", payload: ["host": host]) { data in
            let httpOK = data["http_ok"] as? Bool ?? false
            let controlOK = data["control_ok"] as? Bool ?? false
            self.connected = httpOK || controlOK
            self.connectionMessage = data["message"] as? String ?? (self.connected ? "相机已连接" : "连接失败")
            if self.connected { self.reloadMedia() }
        }
    }

    func disconnect() {
        perform("disconnect_luna") { data in
            self.connected = false
            self.controlReady = false
            self.previewing = false
            self.connectionMessage = data["message"] as? String ?? "相机会话已断开"
        }
    }

    func refreshCurrentSection() {
        switch section {
        case .media: reloadMedia()
        case .capture:
            if controlReady { connectControls() }
            else { connectMedia() }
        case .watermark: refreshWatermarkPreview()
        case .mic: scanMic()
        }
    }

    func reloadMedia() {
        perform("list_media", payload: ["host": host, "storage": storage]) { data in
            let rows = data["value"] as? [[String: Any]] ?? []
            self.media = rows.compactMap(MediaItem.init)
            self.selectedMedia = self.selectedMedia.intersection(Set(self.media.map(\.url)))
            self.connected = true
            self.connectionMessage = self.media.isEmpty ? "已连接，相机中暂无素材" : "已读取 \(self.media.count) 个素材"
        }
    }

    func toggleSelection(_ item: MediaItem) {
        if selectedMedia.contains(item.url) { selectedMedia.remove(item.url) }
        else { selectedMedia.insert(item.url) }
    }

    func loadThumbnail(for item: MediaItem) {
        guard thumbnails[item.url] == nil else { return }
        perform("media_thumbnail", payload: [
            "host": host,
            "url": item.url,
            "cache_key": "\(item.date)-\(item.time)-\(item.sizeText)",
            "media_type": item.kind,
        ]) { data in
            guard let base64 = data["data"] as? String,
                  let bytes = Data(base64Encoded: base64),
                  let image = NSImage(data: bytes)
            else { return }
            self.thumbnails[item.url] = image
        }
    }

    func downloadSelected() {
        let items = media.filter { selectedMedia.contains($0.url) }
        guard !items.isEmpty, let folder = chooseFolder() else { return }
        perform("download_batch", payload: [
            "host": host,
            "output_dir": folder,
            "files": items.map { ["url": $0.url, "date": $0.date] },
        ]) { data in
            self.notice = data["message"] as? String ?? "下载完成"
        }
    }

    func deleteSelected() {
        let urls = Array(selectedMedia)
        guard !urls.isEmpty else { return }
        perform("delete_media", payload: ["host": host, "urls": urls]) { data in
            self.notice = data["message"] as? String ?? "删除完成"
            self.selectedMedia.removeAll()
            self.reloadMedia()
        }
    }

    func connectControls() {
        perform("camera_control_connect", payload: ["host": host]) { data in
            self.controlReady = true
            self.connected = true
            self.captureMode = data["mode"] as? String ?? self.captureMode
            self.zoom = data["zoom"] as? Double ?? self.zoom
            self.recording = data["recording"] as? Bool ?? false
            self.connectionMessage = data["message"] as? String ?? "相机控制已就绪"
        }
    }

    func setCaptureMode(_ mode: String) {
        captureMode = mode
        perform("camera_set_capture_mode", payload: ["host": host, "mode": mode]) { data in
            self.captureMode = data["mode"] as? String ?? mode
            self.notice = data["message"] as? String
        }
    }

    func setZoom() {
        perform("camera_set_zoom", payload: ["host": host, "zoom": zoom]) { data in
            self.zoom = data["zoom"] as? Double ?? self.zoom
        }
    }

    func setVideoProfile() {
        perform("camera_set_video_profile", payload: ["host": host, "format": videoFormat, "fps": videoFPS]) { data in
            self.notice = data["message"] as? String
        }
    }

    func togglePreview() {
        let command = previewing ? "camera_stop_preview" : "camera_start_preview"
        perform(command, payload: ["host": host]) { data in
            self.previewing.toggle()
            self.notice = data["message"] as? String
        }
    }

    func triggerCapture() {
        let command: String
        if captureMode == "photo" { command = "camera_take_photo" }
        else { command = recording ? "camera_stop_record" : "camera_start_record" }
        perform(command, payload: ["host": host]) { data in
            if command == "camera_start_record" { self.recording = true }
            if command == "camera_stop_record" { self.recording = false }
            self.notice = data["message"] as? String
        }
    }

    func moveGimbal(x: Int, y: Int) {
        perform("camera_gimbal_move", payload: ["host": host, "x": x, "y": y])
    }

    func setGimbalSpeed() {
        perform("camera_set_gimbal_speed", payload: ["host": host, "level": gimbalSpeed])
    }

    func loadWatermarkCatalog() {
        perform("watermark_styles") { data in
            self.watermarkStyles = (data["value"] as? [[String: Any]] ?? []).compactMap(WatermarkStyleOption.init)
        }
        perform("watermark_frame_backgrounds") { data in
            self.frameBackgrounds = (data["value"] as? [[String: Any]] ?? []).compactMap(FrameBackgroundOption.init)
        }
    }

    func chooseWatermarkInput() {
        if let path = chooseOpenFile(types: ["jpg", "jpeg", "png", "webp", "mp4", "mov", "mkv", "m4v"]) {
            watermarkInput = path
            refreshWatermarkPreview()
        }
    }

    func chooseMomentImage() {
        if let path = chooseOpenFile(types: ["jpg", "jpeg", "png", "webp"]) {
            momentImage = path
            momentPreset = "custom"
            refreshWatermarkPreview()
        }
    }

    func refreshWatermarkPreview() {
        guard !watermarkInput.isEmpty else { return }
        perform("watermark_preview", payload: watermarkPayload(output: nil)) { data in
            guard let base64 = data["data"] as? String,
                  let bytes = Data(base64Encoded: base64),
                  let image = NSImage(data: bytes)
            else { return }
            self.watermarkPreview = image
        }
    }

    func exportWatermark() {
        guard !watermarkInput.isEmpty else {
            errorMessage = "请先选择原始文件"
            return
        }
        let source = URL(fileURLWithPath: watermarkInput)
        let isVideo = ["mp4", "mov", "mkv", "m4v"].contains(source.pathExtension.lowercased())
        let ext = isVideo ? "mp4" : (["png", "webp"].contains(source.pathExtension.lowercased()) ? source.pathExtension : "jpg")
        let suggested = source.deletingPathExtension().lastPathComponent + "_watermarked." + ext
        guard let output = chooseSavePath(suggestedName: suggested) else { return }
        perform("watermark", payload: watermarkPayload(output: output)) { data in
            self.notice = data["message"] as? String ?? "水印文件已导出"
        }
    }

    private func watermarkPayload(output: String?) -> [String: Any] {
        var payload: [String: Any] = [
            "input": watermarkInput,
            "position": watermarkPosition,
            "style": watermarkStyle,
            "frame_background": frameBackground,
            "moment_preset": momentPreset,
            "moment_image": momentImage,
        ]
        if let output { payload["output"] = output }
        return payload
    }

    func scanMic() {
        perform("scan_ble") { data in
            self.micDevices = (data["value"] as? [[String: Any]] ?? []).compactMap(MicDevice.init)
            self.notice = self.micDevices.isEmpty ? "没有发现附近的 Insta360 麦克风" : "发现 \(self.micDevices.count) 个设备"
        }
    }

    func inspectMic(_ device: MicDevice) {
        selectedMic = device
        perform("inspect_ble", payload: ["address": device.address]) { data in
            let rows = data["value"] as? [[String: Any]] ?? []
            self.micDetails = rows.map { row in
                let uuid = row["uuid"] as? String ?? "未知特征"
                let properties = (row["properties"] as? [String] ?? []).joined(separator: " · ")
                return "\(uuid)\n\(properties)"
            }
        }
    }
}
