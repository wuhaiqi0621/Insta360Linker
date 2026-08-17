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
    let bytes: UInt64?
    let kind: String
    let storageID: String
    let storageLabel: String

    var id: String { url }
    var fileExtension: String { URL(fileURLWithPath: name).pathExtension.lowercased() }
    var isVideo: Bool { ["mp4", "mov", "insv", "lrv", "m4v"].contains(fileExtension) }
    var mediaCategory: String { isVideo ? "video" : "photo" }
    var isLowResolutionPreview: Bool { kind.uppercased() == "LRV" || fileExtension == "lrv" }
    var sortKey: String { "\(date) \(time) \(name)" }
    var mediaPairKey: String? {
        let base = URL(fileURLWithPath: name).deletingPathExtension().lastPathComponent
        let lower = base.lowercased()
        let suffix: String
        if lower.hasPrefix("vid_") || lower.hasPrefix("lrv_") {
            suffix = String(lower.dropFirst(4))
        } else {
            return nil
        }
        let directory = String(url.prefix(upTo: url.lastIndex(of: "/").map { url.index(after: $0) } ?? url.startIndex))
        return "\(storageID)|\(directory)|\(suffix)"
    }
    var supportsWatermark: Bool {
        ["jpg", "jpeg", "png", "webp", "mp4", "mov", "m4v"].contains(fileExtension)
    }
    var supportsPreview: Bool {
        ["jpg", "jpeg", "png", "webp", "insp", "mp4", "mov", "m4v", "insv", "lrv"].contains(fileExtension)
    }

    init?(_ json: [String: Any]) {
        guard let name = json["name"] as? String, let url = json["url"] as? String else { return nil }
        self.name = name
        self.url = url
        date = json["date"] as? String ?? ""
        time = json["time"] as? String ?? ""
        sizeText = json["size_text"] as? String ?? ""
        bytes = (json["bytes"] as? NSNumber)?.uint64Value
        kind = json["kind"] as? String ?? "photo"
        storageID = json["storage_id"] as? String ?? ""
        storageLabel = json["storage_label"] as? String ?? ""
    }
}

enum MediaSortOrder: String, CaseIterable, Identifiable {
    case newest
    case oldest

    var id: String { rawValue }
    var label: String { self == .newest ? "最新优先" : "最早优先" }
}

enum MediaDensity: String, CaseIterable, Identifiable {
    case comfortable
    case medium
    case compact

    var id: String { rawValue }
    var label: String {
        switch self {
        case .comfortable: "大"
        case .medium: "中"
        case .compact: "小"
        }
    }

    var minimumCardWidth: CGFloat {
        switch self {
        case .comfortable: 190
        case .medium: 150
        case .compact: 118
        }
    }

    var maximumCardWidth: CGFloat {
        switch self {
        case .comfortable: 260
        case .medium: 210
        case .compact: 170
        }
    }

    var previewHeight: CGFloat {
        switch self {
        case .comfortable: 125
        case .medium: 100
        case .compact: 78
        }
    }
}

struct MediaPreview: Identifiable {
    let item: MediaItem
    let localURL: URL
    var id: String { item.id }
}

struct VideoFormatOption: Identifiable, Hashable {
    let id: String
    let label: String
    let fpsValues: [Int]

    static let lunaUltra: [VideoFormatOption] = [
        .init(id: "8k_16_9", label: "8K · 16:9 · 7680×4320", fpsValues: [30, 25, 24]),
        .init(id: "8k_2_35_1", label: "8K · 2.35:1 · 7680×3264", fpsValues: [30, 25, 24]),
        .init(id: "4k_16_9", label: "4K · 16:9 · 3840×2160", fpsValues: [120, 100, 60, 50, 48, 30, 25, 24]),
        .init(id: "4k_2_35_1", label: "4K · 2.35:1 · 3840×1632", fpsValues: [120, 100, 60, 50, 48, 30, 25, 24]),
        .init(id: "3k_1_1", label: "3K · 1:1 · 3072×3072", fpsValues: [60, 50, 48, 30, 25, 24]),
        .init(id: "3k_9_16", label: "3K · 9:16 · 1728×3072", fpsValues: [60, 50, 48, 30, 25, 24]),
        .init(id: "2_7k_16_9", label: "2.7K · 16:9 · 2688×1520", fpsValues: [120, 100, 60, 50, 48, 30, 25, 24]),
        .init(id: "2_7k_9_16", label: "2.7K · 9:16 · 1520×2688", fpsValues: [60, 50, 48, 30, 25, 24]),
        .init(id: "1080p_16_9", label: "1080p · 16:9 · 1920×1080", fpsValues: [240, 200, 120, 100, 60, 50, 48, 30, 25, 24]),
        .init(id: "1080p_9_16", label: "1080p · 9:16 · 1080×1920", fpsValues: [60, 50, 48, 30, 25, 24]),
    ]
}

struct WatermarkStyleOption: Identifiable, Hashable {
    let id: String
    let label: String
    let kind: String
    let model: String
    let profile: String
    let imageFile: String?
    let videoFile: String?

    init?(_ json: [String: Any]) {
        guard let id = json["id"] as? String, let label = json["label"] as? String else { return nil }
        self.id = id
        self.label = label
        kind = json["kind"] as? String ?? "mark"
        model = json["model"] as? String ?? "Luna Ultra"
        profile = json["profile"] as? String ?? ""
        imageFile = json["image_file"] as? String
        videoFile = json["video_file"] as? String
    }

    func supports(_ sourceKind: WatermarkSourceKind) -> Bool {
        switch sourceKind {
        case .image: imageFile != nil
        case .video: videoFile != nil
        }
    }
}

enum WatermarkSourceKind {
    case image
    case video
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

struct MicCharacteristic: Identifiable, Hashable {
    let serviceUUID: String
    let uuid: String
    let properties: [String]

    var id: String { "\(serviceUUID)|\(uuid)" }
    var propertyText: String { properties.joined(separator: " · ") }
    var canWrite: Bool { properties.contains { $0.uppercased().contains("WRITE") } }

    init?(_ json: [String: Any]) {
        guard let uuid = json["uuid"] as? String else { return nil }
        serviceUUID = json["service_uuid"] as? String ?? ""
        self.uuid = uuid
        properties = json["properties"] as? [String] ?? []
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
