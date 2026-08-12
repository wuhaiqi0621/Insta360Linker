import AppKit
import Foundation
import SwiftUI
import UniformTypeIdentifiers

enum AppSection: String, CaseIterable, Identifiable {
    case media, capture, watermark, mic

    var id: String { rawValue }
    var title: String {
        switch self {
        case .media: "相机媒体"
        case .capture: "拍摄控制"
        case .watermark: "水印导出"
        case .mic: "Mic Pro"
        }
    }
    var subtitle: String {
        switch self {
        case .media: "浏览、预览并下载 Luna Ultra 中的照片和视频"
        case .capture: "实时取景、拍照、录像与云台控制"
        case .watermark: "使用官方样式生成带水印的照片和视频"
        case .mic: "查找附近的 Insta360 麦克风和接收器"
        }
    }
    var symbol: String {
        switch self {
        case .media: "photo.on.rectangle.angled"
        case .capture: "camera.aperture"
        case .watermark: "signature"
        case .mic: "mic.fill"
        }
    }
}

struct MediaItem: Identifiable, Hashable {
    let name: String
    let url: String
    let date: String
    let time: String
    let sizeText: String
    let kind: String
    let storageID: String
    let storageLabel: String

    var id: String { url }
    var isVideo: Bool { kind == "video" }

    init?(_ json: [String: Any]) {
        guard let name = json["name"] as? String, let url = json["url"] as? String else { return nil }
        self.name = name
        self.url = url
        date = json["date"] as? String ?? ""
        time = json["time"] as? String ?? ""
        sizeText = json["size_text"] as? String ?? ""
        kind = json["kind"] as? String ?? "photo"
        storageID = json["storage_id"] as? String ?? ""
        storageLabel = json["storage_label"] as? String ?? ""
    }
}

struct WatermarkStyleOption: Identifiable, Hashable {
    let id: String
    let label: String
    let kind: String
    let model: String
    let profile: String

    init?(_ json: [String: Any]) {
        guard let id = json["id"] as? String, let label = json["label"] as? String else { return nil }
        self.id = id
        self.label = label
        kind = json["kind"] as? String ?? "mark"
        model = json["model"] as? String ?? "Luna Ultra"
        profile = json["profile"] as? String ?? ""
    }
}

struct FrameBackgroundOption: Identifiable, Hashable {
    let id: String
    let label: String
    let startHex: String
    let endHex: String

    init?(_ json: [String: Any]) {
        guard let id = json["id"] as? String, let label = json["label"] as? String else { return nil }
        self.id = id
        self.label = label
        startHex = json["start_hex"] as? String ?? "#222222"
        endHex = json["end_hex"] as? String ?? startHex
    }
}

struct MicDevice: Identifiable, Hashable {
    let name: String
    let address: String
    var id: String { address }

    init?(_ json: [String: Any]) {
        guard let address = json["address"] as? String else { return nil }
        name = json["name"] as? String ?? "Insta360 Mic"
        self.address = address
    }
}

extension Color {
    init(hex: String) {
        let value = UInt64(hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted), radix: 16) ?? 0
        self.init(
            red: Double((value >> 16) & 0xFF) / 255,
            green: Double((value >> 8) & 0xFF) / 255,
            blue: Double(value & 0xFF) / 255
        )
    }
}

func chooseOpenFile(types: [String]) -> String? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = false
    panel.canChooseFiles = true
    panel.allowsMultipleSelection = false
    panel.allowedContentTypes = types.compactMap { UTType(filenameExtension: $0) }
    return panel.runModal() == .OK ? panel.url?.path : nil
}

func chooseFolder() -> String? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.canCreateDirectories = true
    return panel.runModal() == .OK ? panel.url?.path : nil
}

func chooseSavePath(suggestedName: String) -> String? {
    let panel = NSSavePanel()
    panel.nameFieldStringValue = suggestedName
    panel.canCreateDirectories = true
    return panel.runModal() == .OK ? panel.url?.path : nil
}
