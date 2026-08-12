import AppKit
import SwiftUI

@main
struct Insta360LinkerNativeApp: App {
    @StateObject private var model = AppModel()

    var body: some Scene {
        WindowGroup {
            RootView(model: model)
                .frame(minWidth: 900, minHeight: 620)
                .containerBackground(Color(nsColor: .windowBackgroundColor), for: .window)
        }
        .windowToolbarStyle(.unified)
        .defaultSize(width: 1180, height: 780)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandMenu("相机") {
                Button("连接相机") { model.connectMedia() }
                    .keyboardShortcut("k", modifiers: [.command])
                Button("断开连接") { model.disconnect() }
                Divider()
                Button("刷新当前页面") { model.refreshCurrentSection() }
                    .keyboardShortcut("r", modifiers: [.command])
            }
        }
    }
}

struct RootView: View {
    @ObservedObject var model: AppModel

    private var sectionSelection: Binding<AppSection?> {
        Binding(
            get: { model.section },
            set: { if let section = $0 { model.section = section } }
        )
    }

    var body: some View {
        NavigationSplitView {
            List(selection: sectionSelection) {
                Section("工作区") {
                    ForEach(AppSection.allCases) { section in
                        Label(section.title, systemImage: section.symbol)
                            .tag(section)
                    }
                }

                Section("相机") {
                    LabeledContent {
                        Text(model.connected ? "已连接" : "未连接")
                            .foregroundStyle(model.connected ? .green : .secondary)
                    } label: {
                        Label("状态", systemImage: model.connected ? "checkmark.circle.fill" : "wifi.slash")
                    }
                    Text(model.host)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                    Text(model.connectionMessage)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Insta360Linker")
            .navigationSplitViewColumnWidth(min: 190, ideal: 220, max: 280)
        } detail: {
            Group {
                switch model.section {
                case .media: MediaLibraryView(model: model)
                case .capture: CaptureControlView(model: model)
                case .watermark: WatermarkView(model: model)
                case .mic: MicProView(model: model)
                }
            }
            .padding(16)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .windowBackgroundColor))
            .navigationTitle(model.section.title)
            .navigationSubtitle(model.section.subtitle)
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    if !model.backend.isRunning {
                        Label("后端未运行", systemImage: "exclamationmark.triangle.fill")
                            .foregroundStyle(.orange)
                    }
                    if let notice = model.notice {
                        Label(notice, systemImage: "checkmark.circle.fill")
                            .help(notice)
                    }
                    Button {
                        model.refreshCurrentSection()
                    } label: {
                        Label("刷新", systemImage: "arrow.clockwise")
                    }
                    .disabled(model.busy.contains("list_media"))
                }
            }
        }
        .navigationSplitViewStyle(.balanced)
        .alert("Insta360Linker", isPresented: Binding(
            get: { model.errorMessage != nil },
            set: { if !$0 { model.errorMessage = nil } }
        )) {
            Button("好") { model.errorMessage = nil }
        } message: {
            Text(model.errorMessage ?? "")
        }
    }
}

struct BusyLabel: View {
    let title: String
    let busy: Bool

    var body: some View {
        HStack(spacing: 7) {
            if busy { ProgressView().controlSize(.small) }
            Text(title)
        }
    }
}
