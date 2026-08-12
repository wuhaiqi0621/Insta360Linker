#![cfg_attr(target_os = "macos", allow(dead_code, unused_imports))]

#[path = "../adapters/mod.rs"]
mod adapters;

#[path = "../profiles.rs"]
mod profiles;

#[cfg(target_os = "windows")]
#[path = "../virtual_camera.rs"]
mod virtual_camera;

#[cfg(not(target_os = "windows"))]
#[path = "../virtual_camera_unsupported.rs"]
mod virtual_camera;

use adapters::watermark::WatermarkOptions;

use anyhow::Context;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

use serde::{Deserialize, Serialize};

use serde_json::{Value, json};

use std::collections::hash_map::DefaultHasher;

use std::hash::{Hash, Hasher};

use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use std::net::{TcpListener, TcpStream};

use std::path::PathBuf;

use std::process::{Command, Stdio};

use std::sync::{Arc, Mutex, mpsc};

use tao::dpi::LogicalSize;

use tao::event::{Event, WindowEvent};

use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};

#[cfg(windows)]
use tao::window::Icon;

use tao::window::WindowBuilder;

use wry::{WebViewBuilder, http::Request};

const HTML: &str = include_str!("../../web/index.html");
const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/branding/Insta360Linker-glass.png");

#[cfg(windows)]
fn app_window_icon() -> Option<Icon> {
    let image = image::load_from_memory(include_bytes!(
        "../../assets/branding/Insta360Linker-glass.png"
    ))
    .ok()?
    .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

#[derive(Clone)]

struct AppState {
    luna_session: Arc<Mutex<Option<adapters::luna_local::LunaAuthSession>>>,

    ucd2_session: Arc<Mutex<Option<adapters::luna_local::Ucd2RawSession>>>,

    camera_control: Arc<Mutex<Option<adapters::luna_local::CameraControlSession>>>,

    preview_tx: mpsc::SyncSender<adapters::luna_local::LivePreviewChunk>,

    virtual_camera: Option<Arc<virtual_camera::VirtualCameraController>>,

    virtual_camera_error: Option<Arc<String>>,
}

#[derive(Debug)]

enum UserEvent {
    Js(String),
}

#[derive(Debug, Deserialize)]

struct UiRequest {
    id: u64,

    command: String,

    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Serialize)]

struct UiResponse {
    id: u64,

    ok: bool,

    command: String,

    data: Value,

    error: Option<String>,
}

#[derive(Debug, Deserialize)]

struct HostPayload {
    host: String,
}

#[derive(Debug, Deserialize)]
struct CaptureModePayload {
    host: String,

    mode: String,
}

#[derive(Debug, Deserialize)]
struct ZoomPayload {
    host: String,

    zoom: f64,
}

#[derive(Debug, Deserialize)]
struct VideoProfilePayload {
    host: String,

    format: String,

    fps: u16,
}

#[derive(Debug, Deserialize)]
struct GimbalMovePayload {
    host: String,

    x: i16,

    y: i16,
}

#[derive(Debug, Deserialize)]
struct GimbalSpeedPayload {
    host: String,

    level: u8,
}

#[derive(Debug, Deserialize)]
struct MediaListPayload {
    host: String,

    #[serde(default = "default_media_storage")]
    storage: String,
}

#[derive(Debug, Deserialize)]
struct DeleteMediaPayload {
    host: String,

    urls: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MediaThumbnailPayload {
    host: String,

    url: String,

    #[serde(default)]
    cache_key: String,

    #[serde(default)]
    media_type: String,
}

fn default_media_storage() -> String {
    "all".to_string()
}

#[derive(Debug, Deserialize)]

struct DownloadPayload {
    host: String,

    url: String,

    #[serde(default)]
    output_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BatchDownloadPayload {
    host: String,

    files: Vec<BatchDownloadItem>,

    #[serde(default)]
    output_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BatchDownloadItem {
    url: String,

    #[serde(default)]
    date: String,
}

#[derive(Debug, Deserialize)]

struct Ucd2RawPayload {
    host: String,

    hex: String,
}

#[derive(Debug, Deserialize)]

struct Ucd2StopPayload {
    host: String,

    variant: String,
}

#[derive(Debug, Deserialize)]

struct BleInspectPayload {
    address: String,
}

#[derive(Debug, Deserialize)]

struct BleWritePayload {
    address: String,

    uuid: String,

    hex: String,
}

#[derive(Debug, Deserialize)]

struct WatermarkPayload {
    input: String,

    output: String,

    position: String,

    style: Option<String>,

    frame_background: Option<String>,

    moment_preset: Option<String>,

    moment_image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WatermarkPreviewPayload {
    input: String,

    position: String,

    style: Option<String>,

    frame_background: Option<String>,

    moment_preset: Option<String>,

    moment_image: Option<String>,
}

#[cfg(target_os = "windows")]
fn prepare_mica_surface(window: &tao::window::Window) -> bool {
    use tao::platform::windows::WindowExtWindows;

    #[repr(C)]
    struct Margins {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: isize, margins: *const Margins) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn GetStockObject(object: i32) -> isize;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetClassLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
        fn RedrawWindow(
            hwnd: isize,
            update_rect: *const std::ffi::c_void,
            update_region: isize,
            flags: u32,
        ) -> i32;
    }

    const BLACK_BRUSH: i32 = 4;
    const GCLP_HBRBACKGROUND: i32 = -10;
    const RDW_INVALIDATE: u32 = 0x0001;
    const RDW_ERASE: u32 = 0x0004;
    const RDW_FRAME: u32 = 0x0400;

    let hwnd = window.hwnd();
    let full_client = Margins {
        left: -1,
        right: -1,
        top: -1,
        bottom: -1,
    };

    unsafe {
        let frame_result = DwmExtendFrameIntoClientArea(hwnd, &full_client);
        let black_brush = GetStockObject(BLACK_BRUSH);
        if black_brush != 0 {
            let _ = SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, black_brush);
            let _ = RedrawWindow(
                hwnd,
                std::ptr::null(),
                0,
                RDW_INVALIDATE | RDW_ERASE | RDW_FRAME,
            );
        }
        frame_result >= 0 && black_brush != 0
    }
}

#[cfg(not(target_os = "windows"))]
fn prepare_mica_surface(_window: &tao::window::Window) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn apply_mica(window: &tao::window::Window) -> bool {
    use std::ffi::c_void;
    use tao::platform::windows::WindowExtWindows;
    use tao::window::Theme;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            attribute: u32,
            value: *const c_void,
            value_size: u32,
        ) -> i32;
    }

    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
    const DWMWA_MICA_EFFECT_FALLBACK: u32 = 1029;
    const DWMWCP_ROUND: i32 = 2;
    const DWMSBT_MAINWINDOW: i32 = 2;

    let hwnd = window.hwnd();
    let dark_mode = i32::from(matches!(window.theme(), Theme::Dark));
    let enabled = 1i32;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_mode as *const i32).cast(),
            size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&DWMWCP_ROUND as *const i32).cast(),
            size_of::<i32>() as u32,
        );
        let backdrop_result = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&DWMSBT_MAINWINDOW as *const i32).cast(),
            size_of::<i32>() as u32,
        );
        if backdrop_result >= 0 {
            true
        } else {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_MICA_EFFECT_FALLBACK,
                (&enabled as *const i32).cast(),
                size_of::<i32>() as u32,
            ) >= 0
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_mica(_window: &tao::window::Window) -> bool {
    false
}

fn native_surface_class(liquid_glass_enabled: bool, mica_enabled: bool) -> &'static str {
    if liquid_glass_enabled {
        "native-liquid-glass"
    } else if mica_enabled {
        "native-mica"
    } else {
        "no-native-surface"
    }
}

fn native_surface_class_script(liquid_glass_enabled: bool, mica_enabled: bool) -> String {
    let platform_class = if cfg!(target_os = "macos") {
        "macos-host"
    } else {
        "windows-host"
    };
    format!(
        "document.documentElement.classList.remove('native-liquid-glass','native-mica','no-native-surface','macos-host','windows-host');document.documentElement.classList.add('{}','{}');",
        native_surface_class(liquid_glass_enabled, mica_enabled),
        platform_class
    )
}

#[derive(Debug, Default, Deserialize)]
struct PickWatermarkOutputPayload {
    #[serde(default)]
    input: String,
}

fn bundled_ffmpeg_path() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let executable_dir = executable.parent()?;
    let binary_name = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let mut candidates = vec![
        executable_dir
            .join("assets")
            .join("ffmpeg")
            .join(binary_name),
    ];

    // macOS application bundles keep resources in Contents/Resources.
    if cfg!(target_os = "macos") {
        if let Some(contents_dir) = executable_dir.parent() {
            candidates.push(
                contents_dir
                    .join("Resources")
                    .join("ffmpeg")
                    .join(binary_name),
            );
        }
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn main() -> wry::Result<()> {
    #[cfg(target_os = "macos")]
    {
        if std::env::args().any(|argument| argument == "--native-backend") {
            if let Err(error) = run_native_backend() {
                eprintln!("Native backend failed: {error:#}");
                std::process::exit(1);
            }
        } else {
            eprintln!("This executable is the macOS SwiftUI backend helper.");
        }
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    run_desktop_webview()
}

#[cfg(not(target_os = "macos"))]
fn run_desktop_webview() -> wry::Result<()> {
    if let Some(exit_code) = virtual_camera::handle_installer_mode() {
        std::process::exit(exit_code);
    }

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let proxy = event_loop.create_proxy();

    let (preview_tx, preview_rx) = mpsc::sync_channel::<adapters::luna_local::LivePreviewChunk>(16);
    let virtual_camera_frames = virtual_camera::FrameStore::new();
    let (virtual_camera, virtual_camera_error) =
        match virtual_camera::VirtualCameraController::new(virtual_camera_frames.clone()) {
            Ok(controller) => (Some(controller), None),
            Err(error) => (None, Some(Arc::new(error.to_string()))),
        };
    spawn_preview_decoder(preview_rx, proxy.clone(), virtual_camera_frames);

    let state = AppState {
        luna_session: Arc::new(Mutex::new(None)),

        ucd2_session: Arc::new(Mutex::new(None)),

        camera_control: Arc::new(Mutex::new(None)),

        preview_tx,

        virtual_camera,

        virtual_camera_error,
    };

    let window_builder = WindowBuilder::new()
        .with_title("Insta360Linker")
        .with_inner_size(LogicalSize::new(1180.0, 780.0))
        .with_min_inner_size(LogicalSize::new(760.0, 560.0));

    #[cfg(windows)]
    let window_builder = window_builder.with_window_icon(app_window_icon());

    let window = window_builder.build(&event_loop).unwrap();
    let liquid_glass_enabled = false;

    let mica_surface_ready = prepare_mica_surface(&window);
    let mica_enabled = mica_surface_ready && apply_mica(&window);
    let native_class = native_surface_class(liquid_glass_enabled, mica_enabled);
    let html = HTML.replacen(
        "<html lang=\"zh-CN\">",
        &format!("<html lang=\"zh-CN\" class=\"{native_class}\">"),
        1,
    );
    let app_url =
        start_local_app_server(html, state.clone()).expect("failed to start local media server");

    let ipc_state = state.clone();

    let ipc_proxy = proxy.clone();

    let handler = move |request: Request<String>| {
        let body = request.body().to_string();

        let state = ipc_state.clone();

        let proxy = ipc_proxy.clone();

        std::thread::spawn(move || {
            let response = match serde_json::from_str::<UiRequest>(&body) {
                Ok(req) => response_for(req, state),

                Err(err) => UiResponse {
                    id: 0,

                    ok: false,

                    command: "invalid".to_string(),

                    error: Some(format!(
                        "\u{8bf7}\u{6c42}\u{89e3}\u{6790}\u{5931}\u{8d25}\u{ff1a}{err}"
                    )),

                    data: Value::Null,
                },
            };

            send_response(&proxy, response);
        });
    };

    let webview_builder = WebViewBuilder::new()
        .with_url(&app_url)
        .with_ipc_handler(handler)
        .with_transparent(true)
        .with_devtools(false);

    let webview = webview_builder.build(&window)?;

    // WebView2 creation can update the host window's composition state.
    let mica_enabled = mica_surface_ready && apply_mica(&window);
    let _ = webview.evaluate_script(&native_surface_class_script(
        liquid_glass_enabled,
        mica_enabled,
    ));

    let mut webview = Some(webview);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if let Ok(mut session) = state.luna_session.lock() {
                    if let Some(session) = session.as_mut() {
                        session.close();
                    }

                    *session = None;
                }

                if let Ok(mut session) = state.ucd2_session.lock() {
                    *session = None;
                }

                if let Ok(mut session) = state.camera_control.lock() {
                    *session = None;
                }

                if let Some(virtual_camera) = state.virtual_camera.as_ref() {
                    let _ = virtual_camera.stop();
                }

                let _ = webview.take();

                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::ThemeChanged(_),
                ..
            } => {
                let mica_enabled = mica_surface_ready && apply_mica(&window);
                if let Some(webview) = webview.as_ref() {
                    let _ = webview.evaluate_script(&native_surface_class_script(
                        liquid_glass_enabled,
                        mica_enabled,
                    ));
                }
            }

            Event::UserEvent(UserEvent::Js(script)) => {
                if let Some(webview) = webview.as_ref() {
                    let _ = webview.evaluate_script(&script);
                }
            }

            _ => {}
        }
    });
}

#[cfg(target_os = "macos")]
type BackendWriter = Arc<Mutex<BufWriter<std::io::Stdout>>>;

#[cfg(target_os = "macos")]
fn write_backend_value(writer: &BackendWriter, value: &Value) -> anyhow::Result<()> {
    let mut output = writer.lock().expect("native backend output lock poisoned");
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_native_backend() -> anyhow::Result<()> {
    let (preview_tx, preview_rx) = mpsc::sync_channel::<adapters::luna_local::LivePreviewChunk>(16);
    let virtual_camera_frames = virtual_camera::FrameStore::new();
    let (virtual_camera, virtual_camera_error) =
        match virtual_camera::VirtualCameraController::new(virtual_camera_frames.clone()) {
            Ok(controller) => (Some(controller), None),
            Err(error) => (None, Some(Arc::new(error.to_string()))),
        };
    let state = AppState {
        luna_session: Arc::new(Mutex::new(None)),
        ucd2_session: Arc::new(Mutex::new(None)),
        camera_control: Arc::new(Mutex::new(None)),
        preview_tx,
        virtual_camera,
        virtual_camera_error,
    };
    let writer = Arc::new(Mutex::new(BufWriter::new(std::io::stdout())));
    spawn_native_backend_preview_decoder(preview_rx, virtual_camera_frames, writer.clone());

    for line in BufReader::new(std::io::stdin()).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<UiRequest>(&line) {
            Ok(request) => response_for(request, state.clone()),
            Err(error) => UiResponse {
                id: 0,
                ok: false,
                command: "invalid".to_string(),
                data: Value::Null,
                error: Some(format!("请求解析失败：{error}")),
            },
        };
        write_backend_value(&writer, &serde_json::to_value(response)?)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn_native_backend_preview_decoder(
    preview_rx: mpsc::Receiver<adapters::luna_local::LivePreviewChunk>,
    virtual_camera_frames: Arc<virtual_camera::FrameStore>,
    writer: BackendWriter,
) {
    std::thread::spawn(move || {
        let result = decode_preview_stream(preview_rx, virtual_camera_frames, |jpeg| {
            write_backend_value(
                &writer,
                &json!({
                    "event": "previewImage",
                    "data": BASE64_STANDARD.encode(jpeg),
                }),
            )
        });
        if let Err(error) = result {
            let _ = write_backend_value(
                &writer,
                &json!({"event": "previewError", "message": error.to_string()}),
            );
        }
    });
}

fn spawn_preview_decoder(
    preview_rx: mpsc::Receiver<adapters::luna_local::LivePreviewChunk>,
    proxy: EventLoopProxy<UserEvent>,
    virtual_camera_frames: Arc<virtual_camera::FrameStore>,
) {
    std::thread::spawn(move || {
        let result = decode_preview_stream(preview_rx, virtual_camera_frames, |jpeg| {
            let payload = json!({ "data": BASE64_STANDARD.encode(jpeg) });
            let script = format!(
                "window.Insta360LinkerBridge && window.Insta360LinkerBridge.previewImage({payload});"
            );
            proxy
                .send_event(UserEvent::Js(script))
                .map_err(|_| anyhow::anyhow!("应用窗口已关闭"))
        });
        if let Err(error) = result {
            send_preview_error(&proxy, &error.to_string());
        }
    });
}

fn decode_preview_stream<F>(
    preview_rx: mpsc::Receiver<adapters::luna_local::LivePreviewChunk>,
    virtual_camera_frames: Arc<virtual_camera::FrameStore>,
    mut on_frame: F,
) -> anyhow::Result<()>
where
    F: FnMut(&[u8]) -> anyhow::Result<()>,
{
    let ffmpeg_path =
        bundled_ffmpeg_path().context("缺少实时预览解码组件 Contents/Resources/ffmpeg/ffmpeg")?;
    let mut command = Command::new(ffmpeg_path);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-fflags",
        "nobuffer",
        "-flags",
        "low_delay",
        "-f",
        "hevc",
        "-i",
        "pipe:0",
        "-an",
        "-vf",
        "scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2:black,fps=15",
        "-f",
        "image2pipe",
        "-vcodec",
        "mjpeg",
        "-q:v",
        "5",
        "pipe:1",
    ]);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let mut child = command.spawn().context("无法启动实时预览解码器")?;
    let mut decoder_input = child.stdin.take().context("无法打开实时预览解码输入")?;
    let mut decoder_output = child.stdout.take().context("无法打开实时预览解码输出")?;
    let input_thread = std::thread::spawn(move || {
        while let Ok(chunk) = preview_rx.recv() {
            if decoder_input.write_all(&chunk.data).is_err() || decoder_input.flush().is_err() {
                break;
            }
        }
    });

    let result = (|| -> anyhow::Result<()> {
        let mut read_buffer = [0u8; 64 * 1024];
        let mut jpeg_buffer = Vec::with_capacity(256 * 1024);
        loop {
            let count = decoder_output
                .read(&mut read_buffer)
                .context("读取实时预览画面失败")?;
            if count == 0 {
                break;
            }
            jpeg_buffer.extend_from_slice(&read_buffer[..count]);
            while let Some(start) = find_bytes(&jpeg_buffer, &[0xff, 0xd8], 0) {
                if start > 0 {
                    jpeg_buffer.drain(..start);
                }
                let Some(end) = find_bytes(&jpeg_buffer, &[0xff, 0xd9], 2) else {
                    break;
                };
                let jpeg: Vec<u8> = jpeg_buffer.drain(..end + 2).collect();
                virtual_camera_frames.update_jpeg(&jpeg);
                on_frame(&jpeg)?;
            }
            if jpeg_buffer.len() > 8 * 1024 * 1024 {
                jpeg_buffer.clear();
            }
        }
        Ok(())
    })();

    let _ = child.kill();
    drop(decoder_output);
    let _ = input_thread.join();
    result
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    haystack
        .get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|position| position + start)
}

fn send_preview_error(proxy: &EventLoopProxy<UserEvent>, message: &str) {
    let payload = serde_json::to_string(message).unwrap_or_else(|_| "\"实时预览不可用\"".into());
    let script = format!(
        "window.Insta360LinkerBridge && window.Insta360LinkerBridge.previewError({payload});"
    );
    let _ = proxy.send_event(UserEvent::Js(script));
}

fn response_for(req: UiRequest, state: AppState) -> UiResponse {
    let id = req.id;

    let command = req.command.clone();

    match handle_command(req, state) {
        Ok(data) => UiResponse {
            id,

            ok: true,

            command,

            data,

            error: None,
        },

        Err(err) => UiResponse {
            id,

            ok: false,

            command,

            data: Value::Null,

            error: Some(err.to_string()),
        },
    }
}

fn handle_command(req: UiRequest, state: AppState) -> anyhow::Result<Value> {
    match req.command.as_str() {
        "detect" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;

            let status = adapters::luna_local::check_status(&payload.host, false);

            Ok(serde_json::to_value(status)?)
        }

        "list_media" => {
            let payload: MediaListPayload = serde_json::from_value(req.payload)?;

            let files = list_media_for(&state, &payload.host, &payload.storage)?;

            Ok(serde_json::to_value(files)?)
        }

        "media_thumbnail" => {
            let payload: MediaThumbnailPayload = serde_json::from_value(req.payload)?;
            ensure_media_session(&state, &payload.host)?;
            adapters::luna_local::camera_path_from_url(&payload.host, &payload.url)?;

            let thumbnail = load_media_thumbnail(&payload)?;
            Ok(json!({
                "url": payload.url,
                "mime": "image/jpeg",
                "data": BASE64_STANDARD.encode(thumbnail),
            }))
        }

        "delete_media" => {
            let payload: DeleteMediaPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(&state, &payload.host)?;
            let deleted = guard
                .as_ref()
                .expect("camera control session just opened")
                .delete_media_urls(&payload.urls)?;
            Ok(json!({
                "message": format!("已删除 {} 个相机文件", deleted.len()),
                "deleted": deleted,
            }))
        }

        "disconnect_luna" => {
            if let Some(virtual_camera) = state.virtual_camera.as_ref() {
                let _ = virtual_camera.stop();
            }

            let mut guard = state
                .luna_session
                .lock()
                .expect("luna session lock poisoned");

            if let Some(session) = guard.as_mut() {
                session.close();
            }

            *guard = None;

            let mut raw_guard = state
                .ucd2_session
                .lock()
                .expect("ucd2 session lock poisoned");

            *raw_guard = None;

            let mut control_guard = state
                .camera_control
                .lock()
                .expect("camera control session lock poisoned");

            *control_guard = None;

            Ok(json!({"message":"Luna 会话已断开"}))
        }

        "virtual_camera_status" => {
            let available = state.virtual_camera.is_some();
            let active = state
                .virtual_camera
                .as_ref()
                .map(|camera| camera.is_started())
                .unwrap_or(false);
            Ok(json!({
                "available": available,
                "active": active,
                "name": "Insta360Linker Camera",
                "error": state.virtual_camera_error.as_deref().map(|message| message.as_str()),
            }))
        }

        "virtual_camera_start" => {
            let camera = state.virtual_camera.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "{}",
                    state
                        .virtual_camera_error
                        .as_deref()
                        .map(|message| message.as_str())
                        .unwrap_or("当前系统不支持虚拟摄像机")
                )
            })?;
            let message = camera.start()?;
            Ok(json!({
                "message": message,
                "active": true,
                "name": "Insta360Linker Camera",
            }))
        }

        "virtual_camera_stop" => {
            let camera = state.virtual_camera.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "{}",
                    state
                        .virtual_camera_error
                        .as_deref()
                        .map(|message| message.as_str())
                        .unwrap_or("当前系统不支持虚拟摄像机")
                )
            })?;
            let message = camera.stop()?;
            Ok(json!({
                "message": message,
                "active": false,
                "name": "Insta360Linker Camera",
            }))
        }

        "camera_control_connect" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(&state, &payload.host)?;
            let session = guard.as_ref().expect("camera control session just opened");
            Ok(json!({
                "message": "相机控制已就绪",
                "host": session.host(),
                "mode": session.capture_mode().map(|mode| mode.as_str()),
                "zoom": session.zoom(),
                "recording": session.is_recording(),
            }))
        }

        "camera_set_capture_mode" => {
            let payload: CaptureModePayload = serde_json::from_value(req.payload)?;
            let mode = match payload.mode.as_str() {
                "photo" => adapters::luna_local::CameraCaptureMode::Photo,
                "video" => adapters::luna_local::CameraCaptureMode::Video,
                other => anyhow::bail!("不支持的拍摄模式：{other}"),
            };
            let mut guard = camera_control_for(&state, &payload.host)?;
            let session = guard.as_mut().expect("camera control session just opened");
            let response = session.switch_capture_mode(mode)?;
            Ok(json!({
                "message": if mode == adapters::luna_local::CameraCaptureMode::Photo {
                    "已切换到拍照模式"
                } else {
                    "已切换到录像模式"
                },
                "mode": mode.as_str(),
                "zoom": session.zoom(),
                "response": response,
            }))
        }

        "camera_set_zoom" => {
            let payload: ZoomPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(&state, &payload.host)?;
            let session = guard.as_mut().expect("camera control session just opened");
            let response = session.set_zoom(payload.zoom)?;
            let actual_zoom = session.zoom().context("相机没有返回设置后的实际变焦值")?;
            Ok(json!({
                "message": format!("当前变焦 {:.1}×", actual_zoom),
                "zoom": actual_zoom,
                "response": response,
            }))
        }

        "camera_set_video_profile" => {
            let payload: VideoProfilePayload = serde_json::from_value(req.payload)?;
            let profile =
                adapters::luna_local::resolve_camera_video_profile(&payload.format, payload.fps)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Luna Ultra 不支持录像规格 {} @ {} fps",
                            payload.format,
                            payload.fps
                        )
                    })?;
            let guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_ref()
                .expect("camera control session just opened")
                .set_video_profile(profile)?;
            Ok(json!({
                "message": format!("录像规格已切换到 {}", profile.display_label()),
                "format": profile.format_id(),
                "resolution": profile.resolution(),
                "aspect_ratio": profile.aspect_ratio(),
                "width": profile.width(),
                "height": profile.height(),
                "fps": profile.fps(),
                "response": response,
            }))
        }

        "camera_take_photo" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_mut()
                .expect("camera control session just opened")
                .take_photo()?;
            Ok(json!({
                "message": "拍照命令已完成",
                "response": response,
            }))
        }

        "camera_start_record" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_mut()
                .expect("camera control session just opened")
                .start_recording()?;
            Ok(json!({
                "message": "录像已开始",
                "response": response,
            }))
        }

        "camera_stop_record" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_mut()
                .expect("camera control session just opened")
                .stop_recording()?;
            Ok(json!({
                "message": "录像已停止",
                "media_path": response.media_path,
                "response": response,
            }))
        }

        "camera_start_preview" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_ref()
                .expect("camera control session just opened")
                .start_preview()?;
            Ok(json!({
                "message": "实时预览已开启",
                "response": response,
            }))
        }

        "camera_stop_preview" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_ref()
                .expect("camera control session just opened")
                .stop_preview()?;
            Ok(json!({
                "message": "实时预览已关闭",
                "response": response,
            }))
        }

        "camera_gimbal_move" => {
            let payload: GimbalMovePayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_ref()
                .expect("camera control session just opened")
                .move_gimbal(payload.x, payload.y)?;
            Ok(json!({
                "x": payload.x,
                "y": payload.y,
                "response": response,
            }))
        }

        "camera_set_gimbal_speed" => {
            let payload: GimbalSpeedPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(&state, &payload.host)?;
            let response = guard
                .as_ref()
                .expect("camera control session just opened")
                .set_gimbal_speed(payload.level)?;
            Ok(json!({
                "message": "云台速度已同步到相机",
                "level": payload.level,
                "response": response,
            }))
        }

        "download" => {
            let payload: DownloadPayload = serde_json::from_value(req.payload)?;

            if payload.url.trim().is_empty() {
                anyhow::bail!("\u{6587}\u{4ef6} URL \u{4e0d}\u{80fd}\u{4e3a}\u{7a7a}");
            }

            let filename = download_filename(&payload.url);

            let output = download_root(payload.output_dir.as_deref()).join(&filename);

            ensure_media_session(&state, &payload.host)?;

            adapters::luna_local::resume_download_authenticated(&payload.url, &output)?;

            Ok(json!({"message": format!("\u{5df2}\u{4e0b}\u{8f7d}\u{5230} {}", output.display())}))
        }

        "download_batch" => {
            let payload: BatchDownloadPayload = serde_json::from_value(req.payload)?;

            if payload.files.is_empty() {
                anyhow::bail!(
                    "\u{8bf7}\u{5148}\u{9009}\u{62e9}\u{8981}\u{4e0b}\u{8f7d}\u{7684}\u{7d20}\u{6750}"
                );
            }

            ensure_media_session(&state, &payload.host)?;
            let mut completed = Vec::new();
            let mut failed = Vec::new();

            for item in payload.files {
                if item.url.trim().is_empty() {
                    continue;
                }

                let filename = download_filename(&item.url);
                let day = safe_path_component(&item.date, "\u{672a}\u{5206}\u{7c7b}");
                let output = download_root(payload.output_dir.as_deref())
                    .join(day)
                    .join(&filename);

                match adapters::luna_local::resume_download_authenticated(&item.url, &output) {
                    Ok(()) => completed.push(json!({
                        "name": filename,
                        "output": output.display().to_string(),
                    })),
                    Err(err) => failed.push(json!({
                        "name": filename,
                        "error": err.to_string(),
                    })),
                }
            }

            Ok(json!({
                "message": format!("\u{6279}\u{91cf}\u{4e0b}\u{8f7d}\u{5b8c}\u{6210}\u{ff1a}{} \u{4e2a}\u{6210}\u{529f}\u{ff0c}{} \u{4e2a}\u{5931}\u{8d25}", completed.len(), failed.len()),
                "completed": completed,
                "failed": failed,
            }))
        }

        "ucd2_auth_probe" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;

            let mut guard = ucd2_session_for(&state, &payload.host)?;

            let session = guard.as_mut().expect("ucd2 session just opened");

            Ok(serde_json::to_value(session.send_apk_auth()?)?)
        }

        "ucd2_raw_probe" => {
            let payload: Ucd2RawPayload = serde_json::from_value(req.payload)?;

            if payload.hex.trim().is_empty() {
                anyhow::bail!(
                    "\u{539f}\u{59cb} UCD2 \u{5305}\u{4e0d}\u{80fd}\u{4e3a}\u{7a7a}\u{ff1b}\u{82e5}\u{53ea}\u{60f3}\u{8bfb}\u{53d6}\u{79ef}\u{538b}\u{5e27}\u{ff0c}\u{8bf7}\u{4f7f}\u{7528}\u{201c}\u{53ea}\u{8bfb}\u{53d6}\u{79ef}\u{538b}\u{5e27}\u{201d}\u{6309}\u{94ae}"
                );
            }

            let packets = adapters::luna_local::parse_hex_packets(&payload.hex)?;

            let mut guard = ucd2_session_for(&state, &payload.host)?;

            let session = guard.as_mut().expect("ucd2 session just opened");

            Ok(serde_json::to_value(session.send_packets(

                &packets,

                vec![

                    "Persistent UCD2 session: the socket stays open until Disconnect Luna.".to_string(),

                    "Raw UCD2 probe. Use only packets derived from APK or captured from the official app.".to_string(),

                ],

            )?)?)
        }

        "ucd2_poll" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;

            let mut guard = state
                .ucd2_session
                .lock()
                .expect("ucd2 session lock poisoned");

            let Some(session) = guard.as_mut() else {
                anyhow::bail!(
                    "\u{8fd8}\u{6ca1}\u{6709}\u{6253}\u{5f00} UCD2 \u{6301}\u{4e45}\u{4f1a}\u{8bdd}\u{ff1b}\u{8bf7}\u{5148}\u{53d1}\u{9001} APK UCD2 \u{8ba4}\u{8bc1}\u{5305}"
                );
            };

            if session.host() != payload.host {
                anyhow::bail!(
                    "\u{5f53}\u{524d} UCD2 \u{4f1a}\u{8bdd}\u{4e3b}\u{673a}\u{662f} {}\u{ff0c}\u{4e0d}\u{662f} {}",
                    session.host(),
                    payload.host
                );
            }

            Ok(serde_json::to_value(session.poll_pending()?)?)
        }

        "ucd2_collect_heartbeats" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;

            let mut guard = state
                .ucd2_session
                .lock()
                .expect("ucd2 session lock poisoned");

            let Some(session) = guard.as_mut() else {
                anyhow::bail!(
                    "\u{8fd8}\u{6ca1}\u{6709}\u{6253}\u{5f00} UCD2 \u{6301}\u{4e45}\u{4f1a}\u{8bdd}\u{ff1b}\u{8bf7}\u{5148}\u{53d1}\u{9001} APK UCD2 \u{8ba4}\u{8bc1}\u{5305}"
                );
            };

            if session.host() != payload.host {
                anyhow::bail!(
                    "\u{5f53}\u{524d} UCD2 \u{4f1a}\u{8bdd}\u{4e3b}\u{673a}\u{662f} {}\u{ff0c}\u{4e0d}\u{662f} {}",
                    session.host(),
                    payload.host
                );
            }

            Ok(serde_json::to_value(session.collect_heartbeats()?)?)
        }

        "ucd2_device_info" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;

            let mut guard = ucd2_session_for(&state, &payload.host)?;

            let session = guard.as_mut().expect("ucd2 session just opened");

            Ok(serde_json::to_value(session.read_device_info()?)?)
        }

        "ucd2_negotiation_sync" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;

            let mut guard = ucd2_session_for(&state, &payload.host)?;

            let session = guard.as_mut().expect("ucd2 session just opened");

            Ok(serde_json::to_value(session.send_negotiation_sync()?)?)
        }

        "ucd2_stop_candidate" => {
            let payload: Ucd2StopPayload = serde_json::from_value(req.payload)?;

            let mut guard = ucd2_session_for(&state, &payload.host)?;

            let session = guard.as_mut().expect("ucd2 session just opened");

            Ok(serde_json::to_value(
                session.send_stop_capture_candidate(&payload.variant)?,
            )?)
        }

        "scan_ble" => {
            let runtime = tokio::runtime::Runtime::new()?;

            let devices = runtime.block_on(adapters::mic_ble::scan_mic_devices())?;

            Ok(serde_json::to_value(devices)?)
        }

        "inspect_ble" => {
            let payload: BleInspectPayload = serde_json::from_value(req.payload)?;

            let runtime = tokio::runtime::Runtime::new()?;

            let chars = runtime.block_on(adapters::mic_ble::inspect(&payload.address))?;

            Ok(serde_json::to_value(chars)?)
        }

        "write_ble" => {
            let payload: BleWritePayload = serde_json::from_value(req.payload)?;

            let runtime = tokio::runtime::Runtime::new()?;

            runtime.block_on(adapters::mic_ble::write_hex(
                &payload.address,
                &payload.uuid,
                &payload.hex,
            ))?;

            Ok(json!({"message":"GATT \u{5199}\u{5165}\u{5b8c}\u{6210}"}))
        }

        "watermark" => {
            let payload: WatermarkPayload = serde_json::from_value(req.payload)?;

            let options = WatermarkOptions {
                input: payload.input.into(),

                output: payload.output.into(),

                position: payload.position,

                style: payload.style.unwrap_or_else(|| "luna-ultra-cn".to_string()),

                frame_background: payload
                    .frame_background
                    .unwrap_or_else(|| "black".to_string()),

                moment_preset: payload
                    .moment_preset
                    .unwrap_or_else(|| "official".to_string()),

                moment_image: payload
                    .moment_image
                    .filter(|path| !path.trim().is_empty())
                    .map(Into::into),
            };

            adapters::watermark::apply(&options)?;

            Ok(
                json!({"message": format!("\u{6c34}\u{5370}\u{6587}\u{4ef6}\u{5df2}\u{5bfc}\u{51fa}\u{5230} {}", options.output.display())}),
            )
        }

        "watermark_styles" => Ok(serde_json::to_value(adapters::watermark::styles())?),

        "watermark_frame_backgrounds" => Ok(serde_json::to_value(
            adapters::watermark::frame_backgrounds(),
        )?),

        "watermark_preview" => {
            let payload: WatermarkPreviewPayload = serde_json::from_value(req.payload)?;
            let style = payload.style.unwrap_or_else(|| "luna-ultra-cn".to_string());
            let frame_background = payload
                .frame_background
                .unwrap_or_else(|| "black".to_string());
            let moment_image = payload
                .moment_image
                .as_deref()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(std::path::Path::new);
            let moment_preset = payload.moment_preset.as_deref().unwrap_or("official");
            let preview = adapters::watermark::preview(
                std::path::Path::new(&payload.input),
                &style,
                &payload.position,
                &frame_background,
                moment_preset,
                moment_image,
                900,
                650,
            )?;
            Ok(json!({
                "mime": "image/jpeg",
                "data": BASE64_STANDARD.encode(preview)
            }))
        }

        "pick_media_file" => Ok(json!({
            "path": rfd::FileDialog::new()
                .add_filter("\u{7167}\u{7247}\u{548c}\u{89c6}\u{9891}", &["jpg", "jpeg", "png", "webp", "mp4", "mov", "mkv", "avi", "m4v"])
                .pick_file()
                .map(|path| path.display().to_string())
        })),

        "pick_moment_image" => Ok(json!({
            "path": rfd::FileDialog::new()
                .add_filter("图片", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
                .map(|path| path.display().to_string())
        })),

        "pick_watermark_output" => {
            let payload = serde_json::from_value::<PickWatermarkOutputPayload>(req.payload)
                .unwrap_or_default();
            let input = std::path::Path::new(&payload.input);
            let stem = input
                .file_stem()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("Luna");
            let extension = input
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            let video = matches!(extension.as_str(), "mp4" | "mov" | "mkv" | "avi" | "m4v");
            let output_extension = if video {
                "mp4"
            } else if matches!(extension.as_str(), "png" | "webp") {
                extension.as_str()
            } else {
                "jpg"
            };
            let file_name = format!("{stem}_watermarked.{output_extension}");
            let dialog = rfd::FileDialog::new().set_file_name(file_name);
            let dialog = if video {
                dialog.add_filter("MP4 \u{89c6}\u{9891}", &["mp4"])
            } else {
                dialog
                    .add_filter("JPEG \u{56fe}\u{7247}", &["jpg", "jpeg"])
                    .add_filter("PNG \u{56fe}\u{7247}", &["png"])
                    .add_filter("WebP \u{56fe}\u{7247}", &["webp"])
            };
            Ok(json!({
                "path": dialog.save_file().map(|path| path.display().to_string())
            }))
        }

        "pick_download_dir" => Ok(json!({
            "path": rfd::FileDialog::new()
                .pick_folder()
                .map(|path| path.display().to_string())
        })),

        "profiles" => Ok(serde_json::to_value(profiles::summaries()?)?),

        other => anyhow::bail!("\u{672a}\u{77e5}\u{547d}\u{4ee4}\u{ff1a}{other}"),
    }
}

fn ucd2_session_for<'a>(
    state: &'a AppState,

    host: &str,
) -> anyhow::Result<std::sync::MutexGuard<'a, Option<adapters::luna_local::Ucd2RawSession>>> {
    let mut guard = state
        .ucd2_session
        .lock()
        .expect("ucd2 session lock poisoned");

    let should_open = guard
        .as_ref()
        .map(|session| session.host() != host)
        .unwrap_or(true);

    if should_open {
        *guard = Some(adapters::luna_local::Ucd2RawSession::open(host)?);
    }

    Ok(guard)
}

fn camera_control_for<'a>(
    state: &'a AppState,
    host: &str,
) -> anyhow::Result<std::sync::MutexGuard<'a, Option<adapters::luna_local::CameraControlSession>>> {
    {
        let mut media_guard = state
            .luna_session
            .lock()
            .expect("luna session lock poisoned");
        if let Some(session) = media_guard.as_mut() {
            session.close();
        }
        *media_guard = None;
    }

    let mut guard = state
        .camera_control
        .lock()
        .expect("camera control session lock poisoned");

    let should_open = guard
        .as_ref()
        .map(|session| session.host() != host || !session.is_active())
        .unwrap_or(true);

    if should_open {
        *guard = Some(adapters::luna_local::CameraControlSession::open(
            host,
            state.preview_tx.clone(),
        )?);
    }

    Ok(guard)
}

fn ensure_media_session(state: &AppState, host: &str) -> anyhow::Result<()> {
    let mut media_guard = state
        .luna_session
        .lock()
        .expect("luna session lock poisoned");
    let mut control_guard = state
        .camera_control
        .lock()
        .expect("camera control session lock poisoned");

    if let Some(session) = control_guard.as_ref() {
        if session.host() == host && session.is_active() {
            return Ok(());
        }
        if session.host() != host {
            anyhow::bail!("请先断开当前相机，再读取其他设备的媒体");
        }
        *control_guard = None;
    }
    drop(control_guard);

    let should_open = media_guard
        .as_ref()
        .map(|session| session.host() != host || !session.is_active())
        .unwrap_or(true);
    if should_open {
        if let Some(session) = media_guard.as_mut() {
            session.close();
        }
        *media_guard = Some(adapters::luna_local::LunaAuthSession::open(host)?);
    }

    Ok(())
}

fn list_media_for(
    state: &AppState,
    host: &str,
    storage_id: &str,
) -> anyhow::Result<Vec<adapters::luna_local::LunaFile>> {
    ensure_media_session(state, host)?;

    {
        let control_guard = state
            .camera_control
            .lock()
            .expect("camera control session lock poisoned");
        if let Some(session) = control_guard.as_ref() {
            if session.host() == host && session.is_active() {
                return session.list_files_for_storage(storage_id);
            }
        }
    }

    let mut media_guard = state
        .luna_session
        .lock()
        .expect("luna session lock poisoned");
    media_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Luna 媒体会话尚未建立"))?
        .list_files_for_storage(storage_id)
}

fn start_local_app_server(html: String, state: AppState) -> anyhow::Result<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    let html = Arc::new(html.into_bytes());

    std::thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(mut stream) = connection else {
                continue;
            };
            let html = Arc::clone(&html);
            let state = state.clone();
            std::thread::spawn(move || {
                if let Err(error) = handle_local_app_request(&mut stream, &html, &state) {
                    let _ = write_local_response(
                        &mut stream,
                        500,
                        "text/plain; charset=utf-8",
                        format!("媒体代理错误：{error}").as_bytes(),
                    );
                }
            });
        }
    });

    Ok(format!("http://{address}/"))
}

fn handle_local_app_request(
    stream: &mut TcpStream,
    html: &[u8],
    state: &AppState,
) -> anyhow::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(8)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(120)))?;

    let mut request = Vec::with_capacity(4096);
    let mut buffer = [0u8; 4096];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            anyhow::bail!("本机预览连接提前关闭");
        }
        request.extend_from_slice(&buffer[..count]);
        if request.len() > 64 * 1024 {
            anyhow::bail!("本机预览请求头过大");
        }
    }

    let request_text = String::from_utf8_lossy(&request);
    let mut lines = request_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("缺少本机预览请求行"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or("");
    let target = request_parts.next().unwrap_or("/");
    let range = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("range")
            .then(|| value.trim().to_string())
    });

    if method != "GET" && method != "HEAD" {
        return write_local_response(
            stream,
            405,
            "text/plain; charset=utf-8",
            "不支持的请求方法".as_bytes(),
        );
    }

    let path = target.split('?').next().unwrap_or(target);
    match path {
        "/" | "/index.html" => write_local_response(stream, 200, "text/html; charset=utf-8", html),
        "/app-icon.png" => write_local_response(stream, 200, "image/png", APP_ICON_PNG),
        "/favicon.ico" => write_local_response(stream, 204, "image/x-icon", &[]),
        _ if path.starts_with("/media/") => {
            let encoded = path.trim_start_matches("/media/");
            let url = decode_media_proxy_url(encoded)?;
            proxy_camera_media(stream, state, method, &url, range.as_deref())
        }
        _ => write_local_response(
            stream,
            404,
            "text/plain; charset=utf-8",
            "未找到".as_bytes(),
        ),
    }
}

fn decode_media_proxy_url(encoded: &str) -> anyhow::Result<String> {
    if encoded.is_empty() || !encoded.len().is_multiple_of(2) {
        anyhow::bail!("媒体地址编码无效");
    }
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let part = std::str::from_utf8(pair)?;
        decoded.push(u8::from_str_radix(part, 16)?);
    }
    Ok(String::from_utf8(decoded)?)
}

fn proxy_camera_media(
    stream: &mut TcpStream,
    state: &AppState,
    method: &str,
    url: &str,
    range: Option<&str>,
) -> anyhow::Result<()> {
    let parsed = reqwest::Url::parse(url)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("媒体地址缺少相机主机"))?;
    adapters::luna_local::camera_path_from_url(host, url)?;
    if !has_active_camera_session(state, host) {
        anyhow::bail!("请先在应用中连接相机并刷新相册");
    }
    ensure_media_session(state, host)?;

    let client_builder = reqwest::blocking::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(30 * 60));
    let client_builder = adapters::luna_local::bind_camera_http_client(client_builder, host)
        .map_err(anyhow::Error::msg)?;
    let client = client_builder.build()?;
    let request_method = if method == "HEAD" {
        reqwest::Method::HEAD
    } else {
        reqwest::Method::GET
    };
    let mut request = client
        .request(request_method, url)
        .header(reqwest::header::USER_AGENT, "Insta360Linker/0.4")
        .header(reqwest::header::ACCEPT, "*/*")
        .header(reqwest::header::ACCEPT_ENCODING, "identity");
    if let Some(range) = range {
        request = request.header(reqwest::header::RANGE, range);
    }

    let mut response = request.send()?;
    let status = response.status().as_u16();
    let upstream_content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let inferred_content_type = media_content_type(url);
    let content_type = if inferred_content_type == "application/octet-stream" {
        upstream_content_type
    } else {
        inferred_content_type.to_string()
    };
    let forwarded_headers = [
        reqwest::header::CONTENT_LENGTH,
        reqwest::header::CONTENT_RANGE,
        reqwest::header::ACCEPT_RANGES,
        reqwest::header::ETAG,
        reqwest::header::LAST_MODIFIED,
    ]
    .into_iter()
    .filter_map(|name| {
        response
            .headers()
            .get(&name)
            .and_then(|value| value.to_str().ok())
            .map(|value| (name.as_str().to_string(), value.to_string()))
    })
    .collect::<Vec<_>>();

    write!(
        stream,
        "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\n",
        local_status_reason(status)
    )?;
    for (name, value) in forwarded_headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(
        stream,
        "Cache-Control: private, max-age=60\r\nConnection: close\r\n\r\n"
    )?;
    if method != "HEAD" {
        let _ = std::io::copy(&mut response, stream);
    }
    let _ = stream.flush();
    Ok(())
}

fn has_active_camera_session(state: &AppState, host: &str) -> bool {
    let media_matches = state
        .luna_session
        .lock()
        .map(|guard| {
            guard
                .as_ref()
                .map(|session| session.host() == host && session.is_active())
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if media_matches {
        return true;
    }
    state
        .camera_control
        .lock()
        .map(|guard| {
            guard
                .as_ref()
                .map(|session| session.host() == host && session.is_active())
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn media_content_type(url: &str) -> &'static str {
    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".insp")
        || path.ends_with(".dng")
    {
        "image/jpeg"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".mov") {
        "video/quicktime"
    } else if path.ends_with(".mp4") || path.ends_with(".lrv") || path.ends_with(".insv") {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}

fn local_status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        206 => "Partial Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Response",
    }
}

fn write_local_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        local_status_reason(status),
        body.len()
    )?;
    if !body.is_empty() {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}

fn send_response(proxy: &EventLoopProxy<UserEvent>, response: UiResponse) {
    let payload = serde_json::to_string(&response).unwrap_or_else(|err| {
        json!({
            "id": 0,
            "ok": false,
            "command": "serialize",
            "data": null,
            "error": format!("\u{54cd}\u{5e94}\u{5e8f}\u{5217}\u{5316}\u{5931}\u{8d25}\u{ff1a}{err}")
        })
        .to_string()
    });
    let script =
        format!("window.Insta360LinkerBridge && window.Insta360LinkerBridge.receive({payload});");
    let _ = proxy.send_event(UserEvent::Js(script));
}

fn load_media_thumbnail(payload: &MediaThumbnailPayload) -> anyhow::Result<Vec<u8>> {
    let cache_dir = gallery_thumbnail_cache_dir()?;
    std::fs::create_dir_all(&cache_dir)?;

    let mut hasher = DefaultHasher::new();
    payload.host.hash(&mut hasher);
    payload.url.hash(&mut hasher);
    payload.cache_key.hash(&mut hasher);
    payload.media_type.hash(&mut hasher);
    let cache_path = cache_dir.join(format!("{:016x}.jpg", hasher.finish()));

    if let Ok(bytes) = std::fs::read(&cache_path) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }

    let encoded = if payload.media_type == "video" {
        load_video_thumbnail(&payload.url)?
    } else {
        load_image_thumbnail(&payload.url)?
    };

    let temporary = cache_path.with_extension("jpg.tmp");
    std::fs::write(&temporary, &encoded)?;
    if std::fs::rename(&temporary, &cache_path).is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    Ok(encoded)
}

fn gallery_thumbnail_cache_dir() -> anyhow::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    if let Some(user_home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(user_home)
            .join("Library")
            .join("Caches")
            .join("Insta360Linker")
            .join("gallery-thumbnails"));
    }

    Ok(std::env::current_dir()?.join("data/gallery-thumbnails"))
}

fn load_image_thumbnail(url: &str) -> anyhow::Result<Vec<u8>> {
    let parsed = reqwest::Url::parse(url)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("缩略图地址缺少相机主机"))?;
    let client_builder = reqwest::blocking::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(25));
    let client_builder = adapters::luna_local::bind_camera_http_client(client_builder, host)
        .map_err(anyhow::Error::msg)?;
    let mut response = client_builder
        .build()?
        .get(url)
        .header(reqwest::header::USER_AGENT, "Insta360Linker/0.3")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()?
        .error_for_status()?;

    const MAX_SOURCE_BYTES: u64 = 96 * 1024 * 1024;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_SOURCE_BYTES)
    {
        anyhow::bail!("素材过大，无法生成相册缩略图");
    }
    let mut source = Vec::new();
    response
        .by_ref()
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut source)?;
    if source.len() as u64 > MAX_SOURCE_BYTES {
        anyhow::bail!("素材过大，无法生成相册缩略图");
    }

    let image = image::load_from_memory(&source)?;
    let thumbnail = image.thumbnail(480, 320).to_rgb8();
    let mut encoded = Vec::with_capacity(48 * 1024);
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 74).encode(
        thumbnail.as_raw(),
        thumbnail.width(),
        thumbnail.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(encoded)
}

fn load_video_thumbnail(url: &str) -> anyhow::Result<Vec<u8>> {
    let ffmpeg_path = bundled_ffmpeg_path()
        .ok_or_else(|| anyhow::anyhow!("缺少视频缩略图组件 assets/ffmpeg/ffmpeg"))?;

    let mut command = Command::new(ffmpeg_path);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-rw_timeout",
        "15000000",
        "-i",
        url,
        "-ss",
        "0.2",
        "-frames:v",
        "1",
        "-an",
        "-vf",
        "scale=480:320:force_original_aspect_ratio=decrease",
        "-f",
        "image2pipe",
        "-vcodec",
        "mjpeg",
        "-q:v",
        "5",
        "pipe:1",
    ]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("无法读取视频缩略图输出"))?;
    let output_reader = std::thread::spawn(move || {
        let mut stdout = stdout;
        let mut bytes = Vec::with_capacity(64 * 1024);
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });

    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= std::time::Duration::from_secs(25) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = output_reader.join();
            anyhow::bail!("生成视频预览图超时");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };

    let encoded = output_reader
        .join()
        .map_err(|_| anyhow::anyhow!("视频缩略图读取线程异常"))??;
    if !status.success() || encoded.len() < 4 || !encoded.starts_with(&[0xff, 0xd8]) {
        anyhow::bail!("无法从视频中提取预览图");
    }
    Ok(encoded)
}

fn download_filename(url: &str) -> String {
    let raw = url
        .split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("camera_file");

    safe_path_component(raw, "camera_file")
}

fn download_root(value: Option<&str>) -> PathBuf {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("downloads"))
}

fn safe_path_component(value: &str, fallback: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'..='\x1f' => '_',
            _ => ch,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');

    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod local_media_proxy_tests {
    use super::{decode_media_proxy_url, media_content_type};

    fn encode_url(value: &str) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn media_proxy_url_round_trips_utf8_paths() {
        let url = "http://192.168.42.1/storage_internal/DCIM/Camera01/测试 01.jpg";
        assert_eq!(decode_media_proxy_url(&encode_url(url)).unwrap(), url);
        assert!(decode_media_proxy_url("123").is_err());
    }

    #[test]
    fn media_proxy_reports_browser_media_types() {
        assert_eq!(media_content_type("http://camera/photo.JPG"), "image/jpeg");
        assert_eq!(media_content_type("http://camera/preview.LRV"), "video/mp4");
        assert_eq!(
            media_content_type("http://camera/original.MOV"),
            "video/quicktime"
        );
    }
}
