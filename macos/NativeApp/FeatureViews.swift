import AppKit
import AVKit
import SwiftUI

struct MediaLibraryView: View {
    @ObservedObject var model: AppModel
    private var columns: [GridItem] {
        [GridItem(
            .adaptive(
                minimum: model.mediaDensity.minimumCardWidth,
                maximum: model.mediaDensity.maximumCardWidth
            ),
            spacing: 14
        )]
    }

    var body: some View {
        VStack(spacing: 12) {
            GroupBox {
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 3) {
                        Text("Luna Ultra").font(.headline)
                        Text(model.connectionMessage).font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    if model.connected {
                        LabeledContent("地址", value: model.host)
                        Button("断开") { model.disconnect() }
                            .buttonStyle(.glass)
                    } else {
                        TextField("相机地址", text: $model.host)
                            .frame(width: 170)
                        Button { model.connectMedia() } label: {
                            BusyLabel(title: "连接相机", busy: model.busy.contains("detect"))
                        }
                        .buttonStyle(.glassProminent)
                    }
                }
            } label: {
                Label("相机连接", systemImage: "wifi")
            }

            if model.connected {
                GroupBox {
                    VStack(spacing: 14) {
                        HStack {
                            Picker("存储", selection: $model.storage) {
                                Text("全部存储").tag("all")
                                Text("内部").tag("storage_internal")
                                Text("SD 卡").tag("sdcard")
                            }
                            .pickerStyle(.segmented)
                            .frame(width: 260)
                            .onChange(of: model.storage) { _, _ in model.reloadMedia() }

                            Picker("类型", selection: $model.mediaFilter) {
                                Text("全部").tag("all")
                                Text("照片").tag("photo")
                                Text("视频").tag("video")
                            }
                            .pickerStyle(.segmented)
                            .frame(width: 220)

                            Picker("排序", selection: $model.mediaSort) {
                                ForEach(MediaSortOrder.allCases) { order in
                                    Text(order.label).tag(order)
                                }
                            }
                            .frame(width: 120)

                            Picker("大小", selection: $model.mediaDensity) {
                                ForEach(MediaDensity.allCases) { density in
                                    Text(density.label).tag(density)
                                }
                            }
                            .pickerStyle(.segmented)
                            .frame(width: 110)

                            Spacer()
                            Text("\(model.visibleMedia.count) 个素材")
                                .font(.caption)
                                .foregroundStyle(.secondary)

                            if model.selectedMedia.isEmpty {
                                if !model.visibleMedia.isEmpty {
                                    Button("全选") { model.selectedMedia = Set(model.visibleMedia.map(\.url)) }
                                }
                            } else {
                                Text("已选择 \(model.selectedMedia.count) 个")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                ControlGroup {
                                    Button("取消选择") { model.selectedMedia.removeAll() }
                                    if let item = model.selectedWatermarkMedia {
                                        Button("添加水印") { model.prepareWatermark(from: item) }
                                            .disabled(model.busy.contains("prepare_watermark_media"))
                                    }
                                    Button { model.downloadSelected() } label: {
                                        BusyLabel(title: "下载所选", busy: model.busy.contains("download_batch"))
                                    }
                                    .disabled(model.busy.contains("download_batch"))
                                    Button(role: .destructive) { model.confirmingDelete = true } label: {
                                        Label("删除所选", systemImage: "trash")
                                    }
                                }
                            }
                        }

                        Divider()

                        if model.busy.contains("list_media") {
                            ContentUnavailableView {
                                Label("正在读取相机媒体", systemImage: "photo.stack")
                            } description: {
                                ProgressView()
                            }
                            .frame(maxHeight: .infinity)
                        } else if model.visibleMedia.isEmpty {
                            ContentUnavailableView(
                                "相册为空",
                                systemImage: "photo.on.rectangle.angled",
                                description: Text("相机中没有符合当前筛选条件的照片或视频")
                            )
                            .frame(maxHeight: .infinity)
                        } else {
                            ScrollView {
                                LazyVGrid(columns: columns, spacing: 14) {
                                    ForEach(model.visibleMedia) { item in
                                        MediaCard(model: model, item: item)
                                    }
                                }
                                .padding(2)
                            }
                        }
                    }
                } label: {
                    Label("相机媒体", systemImage: "photo.stack")
                }
                .frame(maxHeight: .infinity)
            } else {
                ContentUnavailableView(
                    "尚未连接相机",
                    systemImage: "wifi.slash",
                    description: Text("连接 Luna Ultra 后才能浏览和管理相机媒体")
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .confirmationDialog("永久删除所选相机文件？", isPresented: $model.confirmingDelete) {
            Button("删除 \(model.selectedMedia.count) 个文件", role: .destructive) { model.deleteSelected() }
            Button("取消", role: .cancel) {}
        } message: {
            Text("此操作会直接删除相机存储中的原文件，无法撤销。")
        }
        .sheet(item: $model.mediaPreview) { preview in
            MediaPreviewSheet(model: model, preview: preview)
        }
    }
}

struct MediaCard: View {
    @ObservedObject var model: AppModel
    let item: MediaItem

    private var selected: Bool { model.selectedMedia.contains(item.url) }

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            ZStack {
                if let image = model.thumbnails[item.url] {
                    Image(nsImage: image)
                        .resizable()
                        .scaledToFill()
                } else {
                    Rectangle().fill(.quaternary)
                    Image(systemName: item.isVideo ? "video.fill" : "photo.fill")
                        .font(.largeTitle)
                        .foregroundStyle(.secondary)
                }
                if item.isVideo {
                    Image(systemName: "play.circle.fill")
                        .font(.system(size: 34))
                        .foregroundStyle(.white)
                        .shadow(radius: 5)
                }
                if model.preparingPreviewURL == item.url {
                    Rectangle().fill(.black.opacity(0.45))
                    ProgressView().controlSize(.large)
                }
            }
            .frame(height: model.mediaDensity.previewHeight)
            .clipShape(.rect(cornerRadius: 8))

            Text(item.name)
                .font(.subheadline.weight(.semibold))
                .lineLimit(1)
            HStack {
                Text(item.date + " " + item.time)
                Spacer()
                Text(item.sizeText)
            }
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
        .padding(10)
        .contentShape(.rect)
        .onTapGesture { model.previewMedia(item) }
        .background(
            selected ? Color.accentColor.opacity(0.14) : Color(nsColor: .controlBackgroundColor),
            in: .rect(cornerRadius: 10)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(selected ? Color.accentColor : Color(nsColor: .separatorColor), lineWidth: selected ? 2 : 1)
        }
        .overlay(alignment: .topTrailing) {
            Button { model.toggleSelection(item) } label: {
                Image(systemName: selected ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(selected ? .white : .white, selected ? Color.accentColor : .black.opacity(0.45))
                    .font(.title3)
            }
            .buttonStyle(.borderless)
            .padding(8)
            .help(selected ? "取消选择" : "选择")
        }
        .onAppear { model.loadThumbnail(for: item) }
        .contextMenu {
            Button { model.previewMedia(item) } label: {
                Label("预览", systemImage: "eye")
            }
            Button { model.toggleSelection(item) } label: {
                Label(selected ? "取消选择" : "选择", systemImage: selected ? "checkmark.circle.fill" : "circle")
            }
            Divider()
            if item.supportsWatermark {
                Button { model.prepareWatermark(from: item) } label: {
                    Label("添加水印", systemImage: "signature")
                }
            }
            Button {
                model.selectOnly(item)
                model.downloadSelected()
            } label: {
                Label("下载", systemImage: "arrow.down.circle")
            }
            Divider()
            Button(role: .destructive) {
                model.selectOnly(item)
                model.confirmingDelete = true
            } label: {
                Label("删除", systemImage: "trash")
            }
        }
    }
}

struct MediaPreviewSheet: View {
    @ObservedObject var model: AppModel
    let preview: MediaPreview

    var body: some View {
        HSplitView {
            ZStack {
                Color.black
                if preview.item.isVideo {
                    NativeVideoPreview(url: preview.localURL)
                } else if let image = NSImage(contentsOf: preview.localURL) {
                    Image(nsImage: image)
                        .resizable()
                        .scaledToFit()
                        .padding(18)
                } else {
                    ContentUnavailableView(
                        "无法显示预览",
                        systemImage: "photo.badge.exclamationmark",
                        description: Text("缓存文件不是可识别的图片")
                    )
                    .foregroundStyle(.secondary)
                }
            }
            .frame(minWidth: 560, maxWidth: .infinity, maxHeight: .infinity)

            Form {
                Section("素材信息") {
                    LabeledContent("文件名", value: preview.item.name)
                    LabeledContent("类型", value: preview.item.isVideo ? "视频" : "照片")
                    LabeledContent("存储", value: preview.item.storageLabel)
                    LabeledContent("拍摄时间", value: preview.item.date + " " + preview.item.time)
                    if !preview.item.sizeText.isEmpty {
                        LabeledContent("大小", value: preview.item.sizeText)
                    }
                }

                Section("操作") {
                    if preview.item.supportsWatermark {
                        Button {
                            model.mediaPreview = nil
                            model.prepareWatermark(from: preview.item)
                        } label: {
                            Label("添加水印", systemImage: "signature")
                        }
                    }
                    Button {
                        model.mediaPreview = nil
                        model.selectOnly(preview.item)
                        model.downloadSelected()
                    } label: {
                        Label("下载原文件…", systemImage: "arrow.down.circle")
                    }
                    Button {
                        NSWorkspace.shared.activateFileViewerSelecting([preview.localURL])
                    } label: {
                        Label("在 Finder 中显示缓存", systemImage: "folder")
                    }
                }

                Section {
                    Button("关闭") { model.mediaPreview = nil }
                        .keyboardShortcut(.cancelAction)
                }
            }
            .formStyle(.grouped)
            .frame(minWidth: 300, idealWidth: 340, maxWidth: 380)
        }
        .frame(minWidth: 900, minHeight: 600)
    }
}

private struct NativeVideoPreview: NSViewRepresentable {
    let url: URL

    func makeNSView(context _: Context) -> AVPlayerView {
        let view = AVPlayerView()
        view.controlsStyle = .floating
        view.videoGravity = .resizeAspect
        view.player = AVPlayer(url: url)
        view.player?.play()
        return view
    }

    func updateNSView(_ view: AVPlayerView, context _: Context) {
        if (view.player?.currentItem?.asset as? AVURLAsset)?.url != url {
            view.player = AVPlayer(url: url)
            view.player?.play()
        }
    }

    static func dismantleNSView(_ view: AVPlayerView, coordinator _: ()) {
        view.player?.pause()
        view.player = nil
    }
}

struct CaptureControlView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        HSplitView {
            GroupBox {
                VStack(spacing: 14) {
                    HStack {
                        Label(model.previewing ? "实时取景中" : "实时取景", systemImage: "viewfinder")
                            .font(.headline)
                        Spacer()
                        if model.recording, let startedAt = model.recordingStartedAt {
                            RecordingElapsedView(startedAt: startedAt)
                        }
                        if model.controlReady {
                            Button(model.previewing ? "停止监看" : "开始监看") { model.togglePreview() }
                                .buttonStyle(.glassProminent)
                                .disabled(model.busy.contains("camera_start_preview") || model.busy.contains("camera_stop_preview"))
                        }
                    }

                    ZStack {
                        Rectangle().fill(.black)
                        if let image = model.backend.previewImage {
                            Image(nsImage: image)
                                .resizable()
                                .scaledToFit()
                        } else {
                            ContentUnavailableView(
                                "等待实时画面",
                                systemImage: "camera.viewfinder",
                                description: Text(model.backend.previewError ?? "连接相机后开启实时监看")
                            )
                            .foregroundStyle(.secondary)
                        }
                    }
                    .clipShape(.rect(cornerRadius: 8))
                    .aspectRatio(16 / 9, contentMode: .fit)

                    if model.controlReady {
                        HStack(spacing: 12) {
                            Picker("模式", selection: $model.captureMode) {
                                Text("拍照").tag("photo")
                                Text("录像").tag("video")
                            }
                            .pickerStyle(.segmented)
                            .disabled(model.recording)
                            .onChange(of: model.captureMode) { _, value in model.setCaptureMode(value) }

                            Button { model.triggerCapture() } label: {
                                Label(
                                    model.captureMode == "photo" ? "拍照" : (model.recording ? "停止录像" : "开始录像"),
                                    systemImage: model.captureMode == "photo" ? "camera.fill" : (model.recording ? "stop.fill" : "record.circle")
                                )
                                .frame(minWidth: 105)
                            }
                            .buttonStyle(.glassProminent)
                            .tint(model.recording ? .red : .accentColor)
                            .controlSize(.large)
                        }
                    }
                }
            } label: {
                Label("取景与拍摄", systemImage: "camera.aperture")
            }
            .frame(minWidth: 480, maxWidth: .infinity, maxHeight: .infinity)

            Form {
                Section("相机连接") {
                    if model.controlReady {
                        LabeledContent("状态", value: "控制已就绪")
                        LabeledContent("地址", value: model.host)
                        Button("断开连接") { model.disconnect() }
                    } else {
                        TextField("相机地址", text: $model.host)
                        Button("连接控制") { model.connectControls() }
                            .buttonStyle(.glassProminent)
                            .frame(maxWidth: .infinity)
                    }
                }

                if model.controlReady {
                    Section("镜头") {
                        LabeledContent("变焦", value: String(format: "%.1f×", model.zoom))
                        Slider(value: $model.zoom, in: 1 ... 12, step: 0.1) { editing in
                            if !editing { model.setZoom() }
                        }
                        .disabled(model.busy.contains("camera_set_zoom"))
                    }
                }

                if model.controlReady && model.captureMode == "video" {
                    Section("录像规格") {
                        Picker("分辨率", selection: $model.videoFormat) {
                            ForEach(model.videoFormats) { format in
                                Text(format.label).tag(format.id)
                            }
                        }
                        .disabled(model.recording)
                        .onChange(of: model.videoFormat) { _, _ in model.syncVideoFPS() }
                        Picker("帧率", selection: $model.videoFPS) {
                            ForEach(model.availableVideoFPS, id: \.self) { Text("\($0) fps").tag($0) }
                        }
                        .disabled(model.recording)
                        Button("应用录像规格") { model.setVideoProfile() }
                            .disabled(model.recording)
                    }
                }

                if model.controlReady {
                    Section("云台") {
                        GimbalPad(model: model)
                            .frame(maxWidth: .infinity)
                        Picker("移动速度", selection: $model.gimbalSpeed) {
                            Text("慢").tag(1)
                            Text("中").tag(2)
                            Text("快").tag(3)
                        }
                        .pickerStyle(.segmented)
                        .disabled(model.busy.contains("camera_set_gimbal_speed"))
                        .onChange(of: model.gimbalSpeed) { _, _ in model.setGimbalSpeed() }
                    }
                }
            }
            .formStyle(.grouped)
            .frame(minWidth: 300, idealWidth: 320, maxWidth: 360)
        }
    }
}

struct GimbalPad: View {
    @ObservedObject var model: AppModel

    var body: some View {
        Grid(horizontalSpacing: 8, verticalSpacing: 8) {
            GridRow { Color.clear.frame(width: 42); moveButton("chevron.up", 0, -72); Color.clear.frame(width: 42) }
            GridRow { moveButton("chevron.left", -72, 0); moveButton("stop.fill", 0, 0); moveButton("chevron.right", 72, 0) }
            GridRow { Color.clear.frame(width: 42); moveButton("chevron.down", 0, 72); Color.clear.frame(width: 42) }
        }
    }

    private func moveButton(_ symbol: String, _ x: Int, _ y: Int) -> some View {
        Button { model.moveGimbal(x: x, y: y) } label: {
            Image(systemName: symbol).frame(width: 42, height: 34)
        }
        .buttonStyle(.glass)
        .disabled(!model.controlReady || model.busy.contains("camera_gimbal_move"))
    }
}

private struct RecordingElapsedView: View {
    let startedAt: Date

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            let elapsed = max(0, Int(context.date.timeIntervalSince(startedAt)))
            Label(
                String(format: "%02d:%02d", elapsed / 60, elapsed % 60),
                systemImage: "record.circle.fill"
            )
            .font(.caption.monospacedDigit().weight(.semibold))
            .foregroundStyle(.red)
        }
    }
}

struct WatermarkView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        HSplitView {
            GroupBox {
                VStack(spacing: 14) {
                    HStack {
                        Text("实时预览").font(.headline)
                        Spacer()
                        if model.canRenderWatermark {
                            Button("刷新预览") { model.refreshWatermarkPreview() }
                        }
                    }
                    ZStack {
                        Rectangle().fill(.black)
                        if let image = model.watermarkPreview {
                            Image(nsImage: image)
                                .resizable()
                                .scaledToFit()
                                .padding(12)
                        } else {
                            ContentUnavailableView(
                                "选择原始文件",
                                systemImage: "photo.badge.plus",
                                description: Text("这里会显示 Rust 官方水印渲染器生成的真实预览")
                            )
                            .foregroundStyle(.secondary)
                        }
                    }
                    .clipShape(.rect(cornerRadius: 8))
                }
            } label: {
                Label("水印预览", systemImage: "photo")
            }
            .frame(minWidth: 460, maxWidth: .infinity, maxHeight: .infinity)

            Form {
                Section("原始文件") {
                    LabeledContent("文件") {
                        Text(model.watermarkInput.isEmpty ? "尚未选择" : URL(fileURLWithPath: model.watermarkInput).lastPathComponent)
                            .lineLimit(1)
                            .foregroundStyle(.secondary)
                    }
                    Button("选择照片或视频…") { model.chooseWatermarkInput() }
                        .buttonStyle(.glassProminent)
                }

                if !model.watermarkInput.isEmpty {
                    Section("官方样式") {
                        Picker("水印", selection: $model.watermarkStyle) {
                            ForEach(model.compatibleWatermarkStyles) { Text($0.label).tag($0.id) }
                        }
                        if model.watermarkSupportsPosition {
                            Picker("位置", selection: $model.watermarkPosition) {
                                Text("底部居中").tag("bottom-center")
                                Text("左下角").tag("bottom-left")
                                Text("右下角").tag("bottom-right")
                            }
                        }
                    }
                }

                if model.watermarkUsesFrame {
                    Section("外框") {
                        Picker("背景", selection: $model.frameBackground) {
                            ForEach(model.frameBackgrounds) { option in
                                Text(option.label).tag(option.id)
                            }
                        }
                    }

                    Section("Luna Moment") {
                        Picker("图案", selection: $model.momentPreset) {
                            Text("官方 Luna Moment").tag("official")
                            Text("深深的巡演").tag("shenshen-concert")
                            Text("自定义图片").tag("custom")
                        }
                        if model.watermarkUsesCustomMoment {
                            LabeledContent("图片") {
                                Text(model.momentImage.isEmpty ? "尚未选择" : URL(fileURLWithPath: model.momentImage).lastPathComponent)
                                    .lineLimit(1)
                                    .foregroundStyle(.secondary)
                            }
                            Button("选择自定义图片…") { model.chooseMomentImage() }
                        }
                    }
                }

                if !model.watermarkInput.isEmpty {
                    Section {
                        Button("导出水印文件…") { model.exportWatermark() }
                            .buttonStyle(.glassProminent)
                            .controlSize(.large)
                            .frame(maxWidth: .infinity)
                            .disabled(!model.canRenderWatermark)
                    }
                }
            }
            .formStyle(.grouped)
            .frame(minWidth: 340, idealWidth: 360, maxWidth: 400)
            .onChange(of: model.watermarkStyle) { _, _ in model.watermarkConfigurationDidChange() }
            .onChange(of: model.watermarkPosition) { _, _ in model.watermarkConfigurationDidChange() }
            .onChange(of: model.frameBackground) { _, _ in model.watermarkConfigurationDidChange() }
            .onChange(of: model.momentPreset) { _, _ in model.watermarkConfigurationDidChange() }
        }
    }
}

struct MicProView: View {
    @ObservedObject var model: AppModel

    var body: some View {
        VStack(spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Mic Pro").font(.title3.bold())
                    Text("扫描附近已开启的 Insta360 麦克风和接收器")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button { model.scanMic() } label: {
                    BusyLabel(title: "扫描设备", busy: model.busy.contains("scan_ble"))
                }
                .buttonStyle(.glassProminent)
            }

            HSplitView {
                if model.busy.contains("scan_ble") {
                    ProgressView("正在扫描附近设备…")
                        .frame(minWidth: 260, maxWidth: .infinity, maxHeight: .infinity)
                } else if model.micDevices.isEmpty {
                    ContentUnavailableView(
                        "尚未发现设备",
                        systemImage: "mic.badge.plus",
                        description: Text("打开 Mic Pro 后点击“扫描设备”")
                    )
                    .frame(minWidth: 260, maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    List(model.micDevices, selection: $model.selectedMic) { device in
                        Button { model.inspectMic(device) } label: {
                            VStack(alignment: .leading, spacing: 4) {
                                Label(device.name, systemImage: "mic.fill")
                                Text(device.address)
                                    .font(.caption.monospaced())
                                    .foregroundStyle(.secondary)
                            }
                            .padding(.vertical, 5)
                        }
                        .buttonStyle(.plain)
                    }
                    .frame(minWidth: 260)
                }

                if let device = model.selectedMic {
                    Form {
                        Section("设备") {
                            LabeledContent("名称", value: device.name)
                            LabeledContent("地址", value: device.address)
                        }
                        Section("GATT 特征") {
                            if model.busy.contains("inspect_ble") {
                                ProgressView("正在读取 GATT 特征…")
                            }
                            ForEach(model.micDetails) { characteristic in
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(characteristic.uuid)
                                        .font(.caption.monospaced())
                                    Text(characteristic.propertyText.isEmpty ? "无属性信息" : characteristic.propertyText)
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                    if !characteristic.serviceUUID.isEmpty {
                                        Text("服务 \(characteristic.serviceUUID)")
                                            .font(.caption2.monospaced())
                                            .foregroundStyle(.tertiary)
                                    }
                                }
                                .padding(.vertical, 2)
                            }
                        }

                        if model.micDetails.contains(where: \.canWrite) {
                            Section("写入 GATT") {
                                Picker("特征", selection: $model.selectedMicCharacteristic) {
                                    Text("请选择").tag(nil as MicCharacteristic?)
                                    ForEach(model.micDetails.filter(\.canWrite)) { characteristic in
                                        Text(characteristic.uuid).tag(Optional(characteristic))
                                    }
                                }
                                TextField("十六进制数据，例如 01 FF", text: $model.micWriteHex)
                                    .textFieldStyle(.roundedBorder)
                                Button { model.writeMicCharacteristic() } label: {
                                    BusyLabel(title: "写入特征", busy: model.busy.contains("write_ble"))
                                }
                                .buttonStyle(.glassProminent)
                                .disabled(
                                    model.selectedMicCharacteristic == nil
                                        || model.micWriteHex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                        || model.busy.contains("write_ble")
                                )
                            }
                        }
                    }
                    .formStyle(.grouped)
                    .frame(minWidth: 420)
                } else {
                    ContentUnavailableView(
                        "尚未选择设备",
                        systemImage: "mic.badge.plus",
                        description: Text("扫描后选择设备以读取原生蓝牙特征")
                    )
                    .frame(minWidth: 420, maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        }
    }
}
