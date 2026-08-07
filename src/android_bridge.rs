#[path = "adapters/luna_local.rs"]
mod luna_local;
#[path = "adapters/watermark.rs"]
mod watermark;

use anyhow::{Context, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::sync::{Mutex, OnceLock, mpsc};

use watermark::WatermarkOptions;

struct AndroidState {
    media_session: Mutex<Option<luna_local::LunaAuthSession>>,
    camera_control: Mutex<Option<luna_local::CameraControlSession>>,
    preview_tx: SyncSender<luna_local::LivePreviewChunk>,
}

impl AndroidState {
    fn new() -> Self {
        let (preview_tx, preview_rx) = mpsc::sync_channel(8);
        std::thread::spawn(move || while preview_rx.recv().is_ok() {});
        Self {
            media_session: Mutex::new(None),
            camera_control: Mutex::new(None),
            preview_tx,
        }
    }
}

static STATE: OnceLock<AndroidState> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct UiRequest {
    id: u64,
    command: String,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct HostPayload {
    host: String,
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

fn default_media_storage() -> String {
    "all".to_string()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_luna_linker_NativeBridge_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    files_dir: JString,
) {
    if let Ok(path) = env.get_string(&files_dir) {
        let root = PathBuf::from(path.to_string_lossy().into_owned());
        let _ = std::fs::create_dir_all(&root);
        let _ = std::env::set_current_dir(root);
    }
    let _ = STATE.set(AndroidState::new());
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_luna_linker_NativeBridge_nativeHandle(
    mut env: JNIEnv,
    _class: JClass,
    request: JString,
) -> jstring {
    let request = env
        .get_string(&request)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let response = std::panic::catch_unwind(|| handle_json(&request)).unwrap_or_else(|_| {
        json!({
            "id": 0,
            "command": "native_panic",
            "ok": false,
            "data": null,
            "error": "Android 原生核心发生异常"
        })
        .to_string()
    });
    env.new_string(response)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

fn handle_json(request: &str) -> String {
    let parsed = serde_json::from_str::<UiRequest>(request);
    let Ok(req) = parsed else {
        return json!({
            "id": 0,
            "command": "parse",
            "ok": false,
            "data": null,
            "error": "无法解析前端请求"
        })
        .to_string();
    };
    let id = req.id;
    let command = req.command.clone();
    match handle_command(req) {
        Ok(data) => json!({
            "id": id,
            "command": command,
            "ok": true,
            "data": data,
            "error": null
        })
        .to_string(),
        Err(error) => json!({
            "id": id,
            "command": command,
            "ok": false,
            "data": null,
            "error": error.to_string()
        })
        .to_string(),
    }
}

fn handle_command(req: UiRequest) -> anyhow::Result<Value> {
    let state = STATE.get_or_init(AndroidState::new);
    match req.command.as_str() {
        "detect" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            Ok(serde_json::to_value(luna_local::check_status(
                &payload.host,
                false,
            ))?)
        }
        "list_media" => {
            let payload: MediaListPayload = serde_json::from_value(req.payload)?;
            Ok(serde_json::to_value(list_media_for(
                state,
                &payload.host,
                &payload.storage,
            )?)?)
        }
        "delete_media" => {
            let payload: DeleteMediaPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(state, &payload.host)?;
            let deleted = guard
                .as_ref()
                .context("相机控制会话尚未建立")?
                .delete_media_urls(&payload.urls)?;
            Ok(
                json!({"message": format!("已删除 {} 个相机文件", deleted.len()), "deleted": deleted}),
            )
        }
        "disconnect_luna" => {
            if let Some(session) = state.media_session.lock().expect("media lock").as_mut() {
                session.close();
            }
            *state.media_session.lock().expect("media lock") = None;
            *state.camera_control.lock().expect("control lock") = None;
            Ok(json!({"message": "Luna 会话已断开"}))
        }
        "camera_control_connect" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(state, &payload.host)?;
            let session = guard.as_ref().context("相机控制会话尚未建立")?;
            Ok(json!({
                "message": "相机控制已就绪",
                "host": session.host(),
                "mode": session.capture_mode().map(|mode| mode.as_str()),
                "zoom": session.zoom(),
                "recording": session.is_recording()
            }))
        }
        "camera_set_capture_mode" => {
            let payload: CaptureModePayload = serde_json::from_value(req.payload)?;
            let mode = match payload.mode.as_str() {
                "photo" => luna_local::CameraCaptureMode::Photo,
                "video" => luna_local::CameraCaptureMode::Video,
                other => return Err(anyhow!("不支持的拍摄模式：{other}")),
            };
            let mut guard = camera_control_for(state, &payload.host)?;
            let session = guard.as_mut().context("相机控制会话尚未建立")?;
            let response = session.switch_capture_mode(mode)?;
            Ok(json!({
                "message": if mode == luna_local::CameraCaptureMode::Photo { "已切换到拍照模式" } else { "已切换到录像模式" },
                "mode": mode.as_str(),
                "zoom": session.zoom(),
                "response": response
            }))
        }
        "camera_set_zoom" => {
            let payload: ZoomPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(state, &payload.host)?;
            let session = guard.as_mut().context("相机控制会话尚未建立")?;
            let response = session.set_zoom(payload.zoom)?;
            let zoom = session.zoom().context("相机没有返回实际变焦值")?;
            Ok(
                json!({"message": format!("当前变焦 {zoom:.1}x"), "zoom": zoom, "response": response}),
            )
        }
        "camera_set_video_profile" => {
            let payload: VideoProfilePayload = serde_json::from_value(req.payload)?;
            let profile = luna_local::resolve_camera_video_profile(&payload.format, payload.fps)
                .ok_or_else(|| anyhow!("Luna Ultra 不支持该录像规格"))?;
            let guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_ref()
                .context("相机控制会话尚未建立")?
                .set_video_profile(profile)?;
            Ok(json!({
                "message": format!("录像规格已切换到 {}", profile.display_label()),
                "format": profile.format_id(),
                "resolution": profile.resolution(),
                "aspect_ratio": profile.aspect_ratio(),
                "width": profile.width(),
                "height": profile.height(),
                "fps": profile.fps(),
                "response": response
            }))
        }
        "camera_take_photo" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_mut()
                .context("相机控制会话尚未建立")?
                .take_photo()?;
            Ok(json!({"message": "拍照命令已完成", "response": response}))
        }
        "camera_start_record" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_mut()
                .context("相机控制会话尚未建立")?
                .start_recording()?;
            Ok(json!({"message": "录像已开始", "response": response}))
        }
        "camera_stop_record" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let mut guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_mut()
                .context("相机控制会话尚未建立")?
                .stop_recording()?;
            Ok(
                json!({"message": "录像已停止", "media_path": response.media_path, "response": response}),
            )
        }
        "camera_gimbal_move" => {
            let payload: GimbalMovePayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_ref()
                .context("相机控制会话尚未建立")?
                .move_gimbal(payload.x, payload.y)?;
            Ok(json!({"x": payload.x, "y": payload.y, "response": response}))
        }
        "camera_set_gimbal_speed" => {
            let payload: GimbalSpeedPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_ref()
                .context("相机控制会话尚未建立")?
                .set_gimbal_speed(payload.level)?;
            Ok(
                json!({"message": "云台速度已同步到相机", "level": payload.level, "response": response}),
            )
        }
        "camera_start_preview" | "camera_stop_preview" => Err(anyhow!(
            "Android 实时预览解码器正在适配，当前构建暂不提供实时取景"
        )),
        "virtual_camera_status" => Ok(json!({
            "available": false,
            "active": false,
            "name": "",
            "error": "虚拟摄像头仅适用于 Windows"
        })),
        "virtual_camera_start" | "virtual_camera_stop" => {
            Err(anyhow!("Android 不支持 Windows 虚拟摄像头"))
        }
        "download_batch" => {
            let payload: BatchDownloadPayload = serde_json::from_value(req.payload)?;
            ensure_media_session(state, &payload.host)?;
            let root = payload
                .output_dir
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("downloads"));
            let mut completed = Vec::new();
            let mut failed = Vec::new();
            for item in payload.files {
                let name = download_filename(&item.url);
                let output = root.join(safe_component(&item.date, "未分类")).join(&name);
                match luna_local::resume_download_authenticated(&item.url, &output) {
                    Ok(()) => completed
                        .push(json!({"name": name, "output": output.display().to_string()})),
                    Err(error) => failed.push(json!({"name": name, "error": error.to_string()})),
                }
            }
            Ok(json!({
                "message": format!("批量下载完成：{} 个成功，{} 个失败", completed.len(), failed.len()),
                "completed": completed,
                "failed": failed
            }))
        }
        "watermark_styles" => Ok(serde_json::to_value(watermark::styles())?),
        "watermark_frame_backgrounds" => Ok(serde_json::to_value(watermark::frame_backgrounds())?),
        "watermark_preview" => {
            let payload: WatermarkPreviewPayload = serde_json::from_value(req.payload)?;
            let preview = watermark::preview(
                Path::new(&payload.input),
                payload.style.as_deref().unwrap_or("luna-ultra-cn"),
                &payload.position,
                payload.frame_background.as_deref().unwrap_or("black"),
                payload.moment_preset.as_deref().unwrap_or("official"),
                payload
                    .moment_image
                    .as_deref()
                    .filter(|path| !path.trim().is_empty())
                    .map(Path::new),
                900,
                650,
            )?;
            Ok(json!({"mime": "image/jpeg", "data": BASE64_STANDARD.encode(preview)}))
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
            watermark::apply(&options)?;
            Ok(json!({"message": "水印文件已导出", "path": options.output.display().to_string()}))
        }
        "scan_ble" => Ok(json!([])),
        other => Err(anyhow!("Android 版暂不支持命令：{other}")),
    }
}

fn camera_control_for<'a>(
    state: &'a AndroidState,
    host: &str,
) -> anyhow::Result<std::sync::MutexGuard<'a, Option<luna_local::CameraControlSession>>> {
    {
        let mut media = state.media_session.lock().expect("media lock");
        if let Some(session) = media.as_mut() {
            session.close();
        }
        *media = None;
    }
    let mut control = state.camera_control.lock().expect("control lock");
    let should_open = control
        .as_ref()
        .map(|session| session.host() != host || !session.is_active())
        .unwrap_or(true);
    if should_open {
        *control = Some(luna_local::CameraControlSession::open(
            host,
            state.preview_tx.clone(),
        )?);
    }
    Ok(control)
}

fn ensure_media_session(state: &AndroidState, host: &str) -> anyhow::Result<()> {
    let mut media = state.media_session.lock().expect("media lock");
    let control = state.camera_control.lock().expect("control lock");
    if let Some(session) = control.as_ref() {
        if session.host() == host && session.is_active() {
            return Ok(());
        }
        if session.host() != host {
            return Err(anyhow!("请先断开当前相机，再读取其他设备的媒体"));
        }
    }
    drop(control);
    let should_open = media
        .as_ref()
        .map(|session| session.host() != host || !session.is_active())
        .unwrap_or(true);
    if should_open {
        if let Some(session) = media.as_mut() {
            session.close();
        }
        *media = Some(luna_local::LunaAuthSession::open(host)?);
    }
    Ok(())
}

fn list_media_for(
    state: &AndroidState,
    host: &str,
    storage: &str,
) -> anyhow::Result<Vec<luna_local::LunaFile>> {
    ensure_media_session(state, host)?;
    {
        let control = state.camera_control.lock().expect("control lock");
        if let Some(session) = control.as_ref() {
            if session.host() == host && session.is_active() {
                return session.list_files_for_storage(storage);
            }
        }
    }
    state
        .media_session
        .lock()
        .expect("media lock")
        .as_mut()
        .context("Luna 媒体会话尚未建立")?
        .list_files_for_storage(storage)
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
    safe_component(raw, "camera_file")
}

fn safe_component(value: &str, fallback: &str) -> String {
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
