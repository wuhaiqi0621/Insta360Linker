import AppKit
import Foundation
import SwiftUI

enum BackendError: LocalizedError {
    case unavailable(String)
    case invalidResponse
    case command(String)

    var errorDescription: String? {
        switch self {
        case .unavailable(let message), .command(let message): message
        case .invalidResponse: "后端返回了无法识别的数据"
        }
    }
}
@MainActor
final class BackendClient: ObservableObject {
    typealias Reply = Result<[String: Any], Error>

    @Published var previewImage: NSImage?
    @Published var previewError: String?
    @Published var isRunning = false

    private var process: Process?
    private var input: FileHandle?
    private var outputBuffer = Data()
    private var nextID = 1
    private var pending: [Int: (Reply) -> Void] = [:]

    init() {
        start()
    }

    deinit {
        process?.terminate()
    }

    func start() {
        guard process == nil else { return }
        guard let helperURL = Bundle.main.resourceURL?.appendingPathComponent("Insta360LinkerBackend"),
              FileManager.default.isExecutableFile(atPath: helperURL.path)
        else {
            previewError = "应用包缺少 Insta360LinkerBackend"
            return
        }

        let process = Process()
        let inputPipe = Pipe()
        let outputPipe = Pipe()
        let errorPipe = Pipe()
        process.executableURL = helperURL
        process.arguments = ["--native-backend"]
        process.standardInput = inputPipe
        process.standardOutput = outputPipe
        process.standardError = errorPipe

        outputPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty else { return }
            Task { @MainActor [weak self] in
                self?.consume(data)
            }
        }
        errorPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard let message = String(data: data, encoding: .utf8), !message.isEmpty else { return }
            Task { @MainActor [weak self] in
                self?.previewError = message.trimmingCharacters(in: .whitespacesAndNewlines)
            }
        }
        process.terminationHandler = { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.backendDidTerminate()
            }
        }

        do {
            try process.run()
            self.process = process
            input = inputPipe.fileHandleForWriting
            isRunning = true
        } catch {
            previewError = "无法启动相机后端：\(error.localizedDescription)"
        }
    }

    func stop() {
        process?.terminate()
        process = nil
        input = nil
        isRunning = false
    }

    func call(
        _ command: String,
        payload: [String: Any] = [:],
        completion: @escaping (Reply) -> Void
    ) {
        guard isRunning, let input else {
            completion(.failure(BackendError.unavailable("相机后端未运行")))
            return
        }

        let id = nextID
        nextID += 1
        let request: [String: Any] = ["id": id, "command": command, "payload": payload]
        guard JSONSerialization.isValidJSONObject(request),
              var data = try? JSONSerialization.data(withJSONObject: request)
        else {
            completion(.failure(BackendError.invalidResponse))
            return
        }
        data.append(0x0A)
        pending[id] = completion
        do {
            try input.write(contentsOf: data)
        } catch {
            pending.removeValue(forKey: id)
            completion(.failure(error))
        }
    }

    private func consume(_ data: Data) {
        outputBuffer.append(data)
        while let newline = outputBuffer.firstIndex(of: 0x0A) {
            let line = outputBuffer[..<newline]
            outputBuffer.removeSubrange(...newline)
            guard !line.isEmpty,
                  let object = try? JSONSerialization.jsonObject(with: Data(line)),
                  let json = object as? [String: Any]
            else { continue }

            if let event = json["event"] as? String {
                handleEvent(event, json: json)
                continue
            }
            guard let id = json["id"] as? Int,
                  let completion = pending.removeValue(forKey: id)
            else { continue }
            if json["ok"] as? Bool == true {
                completion(.success(json["data"] as? [String: Any] ?? ["value": json["data"] ?? NSNull()]))
            } else {
                completion(.failure(BackendError.command(json["error"] as? String ?? "后端命令失败")))
            }
        }
    }

    private func handleEvent(_ event: String, json: [String: Any]) {
        switch event {
        case "previewImage":
            guard let base64 = json["data"] as? String,
                  let data = Data(base64Encoded: base64),
                  let image = NSImage(data: data)
            else { return }
            previewImage = image
            previewError = nil
        case "previewError":
            previewError = json["message"] as? String
        default:
            break
        }
    }

    private func backendDidTerminate() {
        isRunning = false
        process = nil
        input = nil
        let callbacks = pending.values
        pending.removeAll()
        callbacks.forEach { $0(.failure(BackendError.unavailable("相机后端已停止"))) }
    }
}
