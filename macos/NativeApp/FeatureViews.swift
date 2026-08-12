import AppKit
import SwiftUI

struct MediaLibraryView: View {
    @ObservedObject var model: AppModel
    private let columns = [GridItem(.adaptive(minimum: 190, maximum: 260), spacing: 14)]

    var body: some View {
        VStack(spacing: 12) {
            GroupBox {
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 3) {
                        Text("Luna Ultra").font(.headline)
                        Text(model.connectionMessage).font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    TextField("相机地址", text: $model.host)
                        .frame(width: 170)
                    Button { model.connectMedia() } label: {
                        BusyLabel(title: "连接相机", busy: model.busy.contains("detect"))
                    }
                    .buttonStyle(.glassProminent)
                    Button("断开") { model.disconnect() }
                        .buttonStyle(.glass)
                }
            } label: {
                Label("相机连接", systemImage: "wifi")
            }

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

                        Spacer()
                        Text("\(model.visibleMedia.count) 个素材")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        ControlGroup {
                            Button("全选") { model.selectedMedia = Set(model.visibleMedia.map(\.url)) }
                            Button("取消选择") { model.selectedMedia.removeAll() }
                        }

                        Button("下载所选") { model.downloadSelected() }
                            .buttonStyle(.glassProminent)
                            .disabled(model.selectedMedia.isEmpty)
                        Button(role: .destructive) { model.confirmingDelete = true } label: {
                            Label("删除所选", systemImage: "trash")
                        }
                        .disabled(model.selectedMedia.isEmpty)
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
                            description: Text("连接相机后刷新，照片和视频会显示在这里")
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
        }
        .confirmationDialog("永久删除所选相机文件？", isPresented: $model.confirmingDelete) {
            Button("删除 \(model.selectedMedia.count) 个文件", role: .destructive) { model.deleteSelected() }
            Button("取消", role: .cancel) {}
        } message: {
            Text("此操作会直接删除相机存储中的原文件，无法撤销。")
        }
    }
}

struct MediaCard: View {
    @ObservedObject var model: AppModel
    let item: MediaItem

    private var selected: Bool { model.selectedMedia.contains(item.url) }

    var body: some View {
        Button { model.toggleSelection(item) } label: {
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
                }
                .frame(height: 125)
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
        }
        .buttonStyle(.plain)
        .background(
            selected ? Color.accentColor.opacity(0.14) : Color(nsColor: .controlBackgroundColor),
            in: .rect(cornerRadius: 10)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(selected ? Color.accentColor : Color(nsColor: .separatorColor), lineWidth: selected ? 2 : 1)
        }
        .overlay(alignment: .topTrailing) {
            if selected {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(.white, Color.accentColor)
                    .font(.title3)
                    .padding(8)
            }
        }
        .onAppear { model.loadThumbnail(for: item) }
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
                        Button(model.previewing ? "停止监看" : "开始监看") { model.togglePreview() }
                            .buttonStyle(.glassProminent)
                            .disabled(!model.controlReady)
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

                    HStack(spacing: 12) {
                        Picker("模式", selection: $model.captureMode) {
                            Text("拍照").tag("photo")
                            Text("录像").tag("video")
                        }
                        .pickerStyle(.segmented)
                        .onChange(of: model.captureMode) { _, value in
                            if model.controlReady { model.setCaptureMode(value) }
                        }

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
                        .disabled(!model.controlReady)
                    }
                }
            } label: {
                Label("取景与拍摄", systemImage: "camera.aperture")
            }
            .frame(minWidth: 480, maxWidth: .infinity, maxHeight: .infinity)

            Form {
                Section("相机连接") {
                    TextField("相机地址", text: $model.host)
                    Button("连接控制") { model.connectControls() }
                        .buttonStyle(.glassProminent)
                        .frame(maxWidth: .infinity)
                }

                Section("录像规格") {
                    LabeledContent("变焦", value: String(format: "%.1f×", model.zoom))
                    Slider(value: $model.zoom, in: 1 ... 4, step: 0.1) { editing in
                        if !editing && model.controlReady { model.setZoom() }
                    }
                    Picker("分辨率", selection: $model.videoFormat) {
                        Text("8K 16:9").tag("8k_16_9")
                        Text("4K 16:9").tag("4k_16_9")
                        Text("4K 2.35:1").tag("4k_2_35_1")
                        Text("3K 1:1").tag("3k_1_1")
                        Text("2.7K 16:9").tag("2_7k_16_9")
                        Text("1080p 16:9").tag("1080p_16_9")
                    }
                    .onChange(of: model.videoFormat) { _, _ in model.syncVideoFPS() }
                    Picker("帧率", selection: $model.videoFPS) {
                        ForEach(model.availableVideoFPS, id: \.self) { Text("\($0) fps").tag($0) }
                    }
                    Button("应用录像规格") { model.setVideoProfile() }
                        .disabled(!model.controlReady)
                }

                Section("云台") {
                    LabeledContent("移动速度", value: "\(model.gimbalSpeed)")
                    GimbalPad(model: model)
                        .frame(maxWidth: .infinity)
                    Slider(value: Binding(
                        get: { Double(model.gimbalSpeed) },
                        set: { model.gimbalSpeed = Int($0.rounded()) }
                    ), in: 1 ... 5, step: 1) { editing in
                        if !editing && model.controlReady { model.setGimbalSpeed() }
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
            GridRow { Color.clear.frame(width: 42); moveButton("chevron.up", 0, 1); Color.clear.frame(width: 42) }
            GridRow { moveButton("chevron.left", -1, 0); moveButton("scope", 0, 0); moveButton("chevron.right", 1, 0) }
            GridRow { Color.clear.frame(width: 42); moveButton("chevron.down", 0, -1); Color.clear.frame(width: 42) }
        }
    }

    private func moveButton(_ symbol: String, _ x: Int, _ y: Int) -> some View {
        Button { if x != 0 || y != 0 { model.moveGimbal(x: x, y: y) } } label: {
            Image(systemName: symbol).frame(width: 42, height: 34)
        }
        .buttonStyle(.glass)
        .disabled(!model.controlReady)
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
                        Button("刷新预览") { model.refreshWatermarkPreview() }
                            .disabled(model.watermarkInput.isEmpty)
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

                Section("官方样式") {
                    Picker("水印", selection: $model.watermarkStyle) {
                        ForEach(model.watermarkStyles) { Text($0.label).tag($0.id) }
                    }
                    Picker("位置", selection: $model.watermarkPosition) {
                        Text("底部居中").tag("bottom-center")
                        Text("左下角").tag("bottom-left")
                        Text("右下角").tag("bottom-right")
                    }
                    Picker("外框背景", selection: $model.frameBackground) {
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
                    Button("选择自定义图片…") { model.chooseMomentImage() }
                }

                Section {
                    Button("导出水印文件…") { model.exportWatermark() }
                        .buttonStyle(.glassProminent)
                        .controlSize(.large)
                        .frame(maxWidth: .infinity)
                }
            }
            .formStyle(.grouped)
            .frame(minWidth: 340, idealWidth: 360, maxWidth: 400)
            .onChange(of: model.watermarkStyle) { _, _ in model.refreshWatermarkPreview() }
            .onChange(of: model.watermarkPosition) { _, _ in model.refreshWatermarkPreview() }
            .onChange(of: model.frameBackground) { _, _ in model.refreshWatermarkPreview() }
            .onChange(of: model.momentPreset) { _, _ in model.refreshWatermarkPreview() }
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
                            ForEach(Array(model.micDetails.enumerated()), id: \.offset) { _, detail in
                                Text(detail).font(.caption.monospaced())
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
