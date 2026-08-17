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
    @Published var mediaSort: MediaSortOrder = .newest
    @Published var mediaDensity: MediaDensity = .comfortable
    @Published var media: [MediaItem] = []
    @Published var selectedMedia: Set<String> = []
    @Published var thumbnails: [String: NSImage] = [:]
    @Published var preparingPreviewURL: String?
    @Published var mediaPreview: MediaPreview?
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
    @Published var micDetails: [MicCharacteristic] = []
    @Published var selectedMic: MicDevice?
    @Published var selectedMicCharacteristic: MicCharacteristic?
    @Published var micWriteHex = ""
    @Published var recordingStartedAt: Date?
    @Published var busy: Set<String> = []
    @Published var notice: String?
    @Published var errorMessage: String?
    @Published var confirmingDelete = false

    let backend = BackendClient()

    private var linkedVideoPreviews: [String: MediaItem] {
        media.reduce(into: [:]) { result, item in
            guard item.isLowResolutionPreview, let key = item.mediaPairKey else { return }
            result[key] = item
        }
    }

    private var linkedPreviewURLs: Set<String> {
        let previews = linkedVideoPreviews
        return Set(media.compactMap { item in
            guard item.isVideo,
                  !item.isLowResolutionPreview,
                  let key = item.mediaPairKey,
                  let preview = previews[key]
            else { return nil }
            return preview.url
        })
    }

    var logicalMedia: [MediaItem] {
        let hiddenURLs = linkedPreviewURLs
        return media.filter { !hiddenURLs.contains($0.url) }
    }

    var visibleMedia: [MediaItem] {
        logicalMedia
            .filter { mediaFilter == "all" || $0.mediaCategory == mediaFilter }
            .sorted {
                mediaSort == .newest ? $0.sortKey > $1.sortKey : $0.sortKey < $1.sortKey
            }
    }

    var selectedWatermarkMedia: MediaItem? {
        guard selectedMedia.count == 1,
              let item = media.first(where: { selectedMedia.contains($0.url) }),
              item.supportsWatermark
        else { return nil }
        return item
    }

    var watermarkSourceKind: WatermarkSourceKind? {
        let ext = URL(fileURLWithPath: watermarkInput).pathExtension.lowercased()
        if ["jpg", "jpeg", "png", "webp"].contains(ext) { return .image }
        if ["mp4", "mov", "mkv", "avi", "m4v"].contains(ext) { return .video }
        return nil
    }

    var compatibleWatermarkStyles: [WatermarkStyleOption] {
        guard let watermarkSourceKind else { return watermarkStyles }
        return watermarkStyles.filter { $0.supports(watermarkSourceKind) }
    }

    var selectedWatermarkStyle: WatermarkStyleOption? {
        watermarkStyles.first { $0.id == watermarkStyle }
    }

    var watermarkUsesFrame: Bool {
        selectedWatermarkStyle?.kind == "frame" && watermarkSourceKind != .video
    }

    var watermarkSupportsPosition: Bool {
        selectedWatermarkStyle?.kind != "frame" && watermarkSourceKind == .video
    }

    var watermarkUsesCustomMoment: Bool {
        watermarkUsesFrame && momentPreset == "custom"
    }

    var canRenderWatermark: Bool {
        !watermarkInput.isEmpty
            && selectedWatermarkStyle?.supports(watermarkSourceKind ?? .image) == true
            && (!watermarkUsesCustomMoment || !momentImage.isEmpty)
    }

    let videoFormats = VideoFormatOption.lunaUltra

    var availableVideoFPS: [Int] {
        videoFormats.first { $0.id == videoFormat }?.fpsValues ?? [30, 25, 24]
    }

    func syncVideoFPS() {
        if !availableVideoFPS.contains(videoFPS) {
            videoFPS = availableVideoFPS.first ?? 30
        }
    }

    init() {
        loadWatermarkCatalog()
    }

    func perform(_ command: String, payload: [String: Any] = [:]) {
        execute(command, payload: payload, failure: nil, completion: nil)
    }

    func perform(
        _ command: String,
        payload: [String: Any] = [:],
        completion: @escaping ([String: Any]) -> Void
    ) {
        execute(command, payload: payload, failure: nil, completion: completion)
    }

    func perform(
        _ command: String,
        payload: [String: Any] = [:],
        failure: @escaping (Error) -> Void,
        completion: @escaping ([String: Any]) -> Void
    ) {
        execute(command, payload: payload, failure: failure, completion: completion)
    }

    private func execute(
        _ command: String,
        payload: [String: Any],
        failure: ((Error) -> Void)?,
        completion: (([String: Any]) -> Void)?
    ) {
        busy.insert(command)
        backend.call(command, payload: payload) { [weak self] result in
            guard let self else { return }
            self.busy.remove(command)
            switch result {
            case .success(let data):
                completion?(data)
            case .failure(let error):
                self.errorMessage = error.localizedDescription
                failure?(error)
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
            self.recording = false
            self.recordingStartedAt = nil
            self.mediaPreview = nil
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
            self.selectedMedia = self.selectedMedia.intersection(Set(self.logicalMedia.map(\.url)))
            self.connected = true
            self.connectionMessage = self.media.isEmpty ? "已连接，相机中暂无素材" : "已读取 \(self.media.count) 个素材"
        }
    }

    func toggleSelection(_ item: MediaItem) {
        if selectedMedia.contains(item.url) { selectedMedia.remove(item.url) }
        else { selectedMedia.insert(item.url) }
    }

    func selectOnly(_ item: MediaItem) {
        selectedMedia = [item.url]
    }

    func linkedPreview(for item: MediaItem) -> MediaItem? {
        guard item.isVideo, !item.isLowResolutionPreview, let key = item.mediaPairKey else { return nil }
        return linkedVideoPreviews[key]
    }

    func loadThumbnail(for item: MediaItem) {
        guard thumbnails[item.url] == nil else { return }
        let source = linkedPreview(for: item) ?? item
        perform("media_thumbnail", payload: [
            "host": host,
            "url": source.url,
            "cache_key": "\(source.date)-\(source.time)-\(source.sizeText)",
            "media_type": source.kind,
        ]) { data in
            guard let base64 = data["data"] as? String,
                  let bytes = Data(base64Encoded: base64),
                  let image = NSImage(data: bytes)
            else { return }
            self.thumbnails[item.url] = image
        }
    }

    func previewMedia(_ item: MediaItem) {
        guard item.supportsPreview else {
            errorMessage = "该相机素材格式暂不支持直接预览"
            return
        }
        let source = linkedPreview(for: item) ?? item
        preparingPreviewURL = item.url
        perform(
            "prepare_media_preview",
            payload: ["host": host, "url": source.url],
            failure: { _ in self.preparingPreviewURL = nil }
        ) { data in
            self.preparingPreviewURL = nil
            guard let path = data["path"] as? String, !path.isEmpty else {
                self.errorMessage = "相机素材没有返回可预览文件"
                return
            }
            self.mediaPreview = MediaPreview(item: item, localURL: URL(fileURLWithPath: path))
        }
    }

    func downloadSelected() {
        let items = media.filter { selectedMedia.contains($0.url) }
        guard !items.isEmpty, let folder = chooseFolder() else { return }
        perform("download_batch", payload: [
            "host": host,
            "output_dir": folder,
            "files": items.map { item in
                var row: [String: Any] = ["url": item.url, "date": item.date]
                if let bytes = item.bytes { row["bytes"] = bytes }
                return row
            },
        ]) { data in
            self.notice = data["message"] as? String ?? "下载完成"
            let failedNames = Set((data["failed"] as? [[String: Any]] ?? []).compactMap { $0["name"] as? String })
            self.selectedMedia = Set(self.logicalMedia.filter { failedNames.contains($0.name) }.map(\.url))
        }
    }

    func prepareWatermark(from item: MediaItem) {
        guard item.supportsWatermark else {
            errorMessage = "该相机素材格式暂不支持水印导出"
            return
        }
        perform("prepare_watermark_media", payload: ["host": host, "url": item.url]) { data in
            guard let path = data["path"] as? String, !path.isEmpty else {
                self.errorMessage = "相机原片没有返回可用路径"
                return
            }
            self.watermarkInput = path
            self.normalizeWatermarkConfiguration()
            self.section = .watermark
            self.notice = data["message"] as? String ?? "相机原片已载入水印工作区"
            self.refreshWatermarkPreview()
        }
    }

    func deleteSelected() {
        var urls = Set(selectedMedia)
        for item in logicalMedia where selectedMedia.contains(item.url) {
            if let preview = linkedPreview(for: item) { urls.insert(preview.url) }
        }
        guard !urls.isEmpty else { return }
        perform("delete_media", payload: ["host": host, "urls": Array(urls)]) { data in
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
            self.recordingStartedAt = self.recording ? Date() : nil
            self.connectionMessage = data["message"] as? String ?? "相机控制已就绪"
        }
    }

    func setCaptureMode(_ mode: String) {
        captureMode = mode
        perform("camera_set_capture_mode", payload: ["host": host, "mode": mode]) { data in
            self.captureMode = data["mode"] as? String ?? mode
            self.zoom = data["zoom"] as? Double ?? self.zoom
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
            self.videoFormat = data["format"] as? String ?? self.videoFormat
            self.videoFPS = data["fps"] as? Int ?? self.videoFPS
            self.notice = data["message"] as? String
        }
    }

    func togglePreview() {
        let command = previewing ? "camera_stop_preview" : "camera_start_preview"
        perform(command, payload: ["host": host]) { data in
            self.previewing = command == "camera_start_preview"
            self.notice = data["message"] as? String
        }
    }

    func triggerCapture() {
        let command: String
        if captureMode == "photo" { command = "camera_take_photo" }
        else { command = recording ? "camera_stop_record" : "camera_start_record" }
        perform(command, payload: ["host": host]) { data in
            if command == "camera_start_record" {
                self.recording = true
                self.recordingStartedAt = Date()
            }
            if command == "camera_stop_record" {
                self.recording = false
                self.recordingStartedAt = nil
            }
            self.notice = data["media_path"] as? String ?? data["message"] as? String
            if command == "camera_take_photo" { self.scheduleMediaReload(after: 4.5) }
            if command == "camera_stop_record" { self.scheduleMediaReload(after: 1.8) }
        }
    }

    func moveGimbal(x: Int, y: Int) {
        perform("camera_gimbal_move", payload: ["host": host, "x": x, "y": y])
    }

    func setGimbalSpeed() {
        perform("camera_set_gimbal_speed", payload: ["host": host, "level": gimbalSpeed])
    }

    private func scheduleMediaReload(after seconds: Double) {
        Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
            guard let self, self.connected else { return }
            self.reloadMedia()
        }
    }

    func loadWatermarkCatalog() {
        perform("watermark_styles") { data in
            self.watermarkStyles = (data["value"] as? [[String: Any]] ?? []).compactMap(WatermarkStyleOption.init)
            self.normalizeWatermarkConfiguration()
        }
        perform("watermark_frame_backgrounds") { data in
            self.frameBackgrounds = (data["value"] as? [[String: Any]] ?? []).compactMap(FrameBackgroundOption.init)
        }
    }

    func chooseWatermarkInput() {
        if let path = chooseOpenFile(types: ["jpg", "jpeg", "png", "webp", "mp4", "mov", "mkv", "m4v"]) {
            watermarkInput = path
            normalizeWatermarkConfiguration()
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
        guard canRenderWatermark else {
            watermarkPreview = nil
            return
        }
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
        guard canRenderWatermark else {
            errorMessage = watermarkUsesCustomMoment ? "请先选择自定义 Luna Moment 图片" : "当前水印样式不支持这个文件"
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
            "position": watermarkSupportsPosition ? watermarkPosition : "bottom-center",
            "style": watermarkStyle,
            "frame_background": watermarkUsesFrame ? frameBackground : "black",
            "moment_preset": watermarkUsesFrame ? momentPreset : "official",
        ]
        if watermarkUsesCustomMoment { payload["moment_image"] = momentImage }
        if let output { payload["output"] = output }
        return payload
    }

    func watermarkConfigurationDidChange() {
        normalizeWatermarkConfiguration()
        refreshWatermarkPreview()
    }

    private func normalizeWatermarkConfiguration() {
        if !compatibleWatermarkStyles.contains(where: { $0.id == watermarkStyle }),
           let fallback = compatibleWatermarkStyles.first
        {
            watermarkStyle = fallback.id
        }
        if !watermarkSupportsPosition { watermarkPosition = "bottom-center" }
    }

    func scanMic() {
        perform("scan_ble") { data in
            self.micDevices = (data["value"] as? [[String: Any]] ?? []).compactMap(MicDevice.init)
            self.notice = self.micDevices.isEmpty ? "没有发现附近的 Insta360 麦克风" : "发现 \(self.micDevices.count) 个设备"
        }
    }

    func inspectMic(_ device: MicDevice) {
        selectedMic = device
        selectedMicCharacteristic = nil
        micWriteHex = ""
        perform("inspect_ble", payload: ["address": device.address]) { data in
            let rows = data["value"] as? [[String: Any]] ?? []
            self.micDetails = rows.compactMap(MicCharacteristic.init)
            self.selectedMicCharacteristic = self.micDetails.first(where: \.canWrite)
        }
    }

    func writeMicCharacteristic() {
        guard let device = selectedMic, let characteristic = selectedMicCharacteristic else { return }
        let clean = micWriteHex.replacingOccurrences(of: " ", with: "").replacingOccurrences(of: "-", with: "")
        guard !clean.isEmpty, clean.count.isMultiple(of: 2), clean.allSatisfy({ $0.isHexDigit }) else {
            errorMessage = "请输入偶数位十六进制数据"
            return
        }
        perform("write_ble", payload: [
            "address": device.address,
            "uuid": characteristic.uuid,
            "hex": clean,
        ]) { data in
            self.notice = data["message"] as? String ?? "GATT 写入完成"
        }
    }
}
