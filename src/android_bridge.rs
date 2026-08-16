#[path = "adapters/luna_local.rs"]
mod luna_local;
#[path = "adapters/watermark.rs"]
mod watermark;

use anyhow::{Context, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jbyteArray, jint, jlong, jstring};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use watermark::WatermarkOptions;

struct AndroidState {
    media_session: Mutex<Option<luna_local::LunaAuthSession>>,
    camera_control: Mutex<Option<luna_local::CameraControlSession>>,
    preview_tx: SyncSender<luna_local::LivePreviewChunk>,
    preview_rx: Mutex<Receiver<luna_local::LivePreviewChunk>>,
    task_event_tx: SyncSender<String>,
    task_event_rx: Mutex<Receiver<String>>,
    camera_connected: AtomicBool,
    connection_epoch: AtomicU64,
    transfer_controls: Mutex<HashMap<u64, Arc<TransferControl>>>,
}

impl AndroidState {
    fn new() -> Self {
        let (preview_tx, preview_rx) = mpsc::sync_channel(24);
        let (task_event_tx, task_event_rx) = mpsc::sync_channel(128);
        Self {
            media_session: Mutex::new(None),
            camera_control: Mutex::new(None),
            preview_tx,
            preview_rx: Mutex::new(preview_rx),
            task_event_tx,
            task_event_rx: Mutex::new(task_event_rx),
            camera_connected: AtomicBool::new(false),
            connection_epoch: AtomicU64::new(0),
            transfer_controls: Mutex::new(HashMap::new()),
        }
    }
}

const TRANSFER_RUNNING: u8 = 0;
const TRANSFER_PAUSED: u8 = 1;
const TRANSFER_CANCELLED: u8 = 2;

struct TransferControl {
    state: AtomicU8,
}

impl TransferControl {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(TRANSFER_RUNNING),
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
    #[serde(default)]
    bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PrepareWatermarkMediaPayload {
    host: String,
    url: String,
    #[serde(default)]
    output_dir: Option<String>,
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
pub extern "system" fn Java_studio_insta360_linker_NativeBridge_nativeInit(
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
pub extern "system" fn Java_studio_insta360_linker_NativeBridge_nativeHandle(
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

#[derive(Debug, Deserialize)]
struct VideoWatermarkPlanPayload {
    width: u32,
    height: u32,
    position: String,
    style: Option<String>,
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_insta360_linker_NativeBridge_nativePollPreview(
    env: JNIEnv,
    _class: JClass,
    timeout_ms: jint,
) -> jbyteArray {
    let state = STATE.get_or_init(AndroidState::new);
    let timeout = Duration::from_millis(timeout_ms.clamp(10, 1000) as u64);
    let data = state
        .preview_rx
        .lock()
        .expect("preview lock")
        .recv_timeout(timeout)
        .map(|chunk| chunk.data)
        .unwrap_or_default();
    env.byte_array_from_slice(&data)
        .map(|array| array.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_insta360_linker_NativeBridge_nativePollTaskEvent(
    env: JNIEnv,
    _class: JClass,
    timeout_ms: jint,
) -> jstring {
    let state = STATE.get_or_init(AndroidState::new);
    let timeout = Duration::from_millis(timeout_ms.clamp(10, 1000) as u64);
    let event = state
        .task_event_rx
        .lock()
        .expect("task event lock")
        .recv_timeout(timeout)
        .unwrap_or_default();
    env.new_string(event)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_studio_insta360_linker_NativeBridge_nativeControlTransfer(
    _env: JNIEnv,
    _class: JClass,
    task_id: jlong,
    action: jint,
) -> jboolean {
    let state = STATE.get_or_init(AndroidState::new);
    let task_id = task_id.max(0) as u64;
    let mut controls = state
        .transfer_controls
        .lock()
        .expect("transfer control lock");
    match action {
        0 => {
            controls.insert(task_id, Arc::new(TransferControl::new()));
            JNI_TRUE
        }
        1 => controls
            .get(&task_id)
            .map(|control| {
                control.state.store(TRANSFER_PAUSED, Ordering::SeqCst);
                JNI_TRUE
            })
            .unwrap_or(JNI_FALSE),
        2 => controls
            .get(&task_id)
            .map(|control| {
                control.state.store(TRANSFER_RUNNING, Ordering::SeqCst);
                JNI_TRUE
            })
            .unwrap_or(JNI_FALSE),
        3 => controls
            .get(&task_id)
            .map(|control| {
                control.state.store(TRANSFER_CANCELLED, Ordering::SeqCst);
                JNI_TRUE
            })
            .unwrap_or(JNI_FALSE),
        4 => {
            controls.remove(&task_id);
            JNI_TRUE
        }
        _ => JNI_FALSE,
    }
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
    let task_id = req.id;
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
            let files = list_media_for(state, &payload.host, &payload.storage)?;
            mark_camera_connected(state);
            Ok(serde_json::to_value(files)?)
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
            state.camera_connected.store(false, Ordering::SeqCst);
            state.connection_epoch.fetch_add(1, Ordering::SeqCst);
            cancel_all_transfers(state);
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
            mark_camera_connected(state);
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
        "camera_start_preview" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            drain_preview_queue(state);
            let guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_ref()
                .context("相机控制会话尚未建立")?
                .start_preview()?;
            Ok(json!({"message": "实时预览已开启", "response": response}))
        }
        "camera_stop_preview" => {
            let payload: HostPayload = serde_json::from_value(req.payload)?;
            let guard = camera_control_for(state, &payload.host)?;
            let response = guard
                .as_ref()
                .context("相机控制会话尚未建立")?
                .stop_preview()?;
            drain_preview_queue(state);
            Ok(json!({"message": "实时预览已关闭", "response": response}))
        }
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
            let transfer_control = transfer_control_for(state, task_id);
            wait_for_transfer_permission(&transfer_control)?;
            let connection_epoch = active_camera_transfer_epoch(state, &payload.host)?;
            ensure_media_session(state, &payload.host)?;
            let camera_host = payload.host.clone();
            let batch_total = payload
                .files
                .iter()
                .map(|item| item.bytes.filter(|value| *value > 0))
                .collect::<Option<Vec<_>>>()
                .map(|values| values.into_iter().sum::<u64>());
            let root = payload
                .output_dir
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("downloads"));
            let mut completed = Vec::new();
            let mut failed = Vec::new();
            let item_count = payload.files.len();
            let mut completed_download_bytes = 0_u64;
            for (item_index, item) in payload.files.into_iter().enumerate() {
                let name = download_filename(&item.url);
                let output = root.join(safe_component(&item.date, "未分类")).join(&name);
                let expected_item_bytes = item.bytes.filter(|value| *value > 0);
                let started_at = Instant::now();
                let mut initial_bytes = None;
                let download = luna_local::resume_download_authenticated_with_progress(
                    &item.url,
                    &output,
                    |downloaded, total| {
                        wait_for_transfer_permission(&transfer_control)?;
                        ensure_camera_transfer_active(state, &camera_host, connection_epoch)?;
                        let initial = *initial_bytes.get_or_insert(downloaded);
                        let transferred = downloaded.saturating_sub(initial);
                        let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
                        let speed_bps = (transferred as f64 / elapsed) as u64;
                        let item_total = total.or(expected_item_bytes);
                        let display_completed = batch_total
                            .map(|_| completed_download_bytes.saturating_add(downloaded))
                            .unwrap_or(downloaded);
                        let display_total = batch_total.or(item_total);
                        let progress = display_total
                            .filter(|value| *value > 0)
                            .map(|value| {
                                ((display_completed as f64 / value as f64).clamp(0.0, 1.0) * 100.0)
                                    .round() as u8
                            })
                            .unwrap_or(0);
                        let eta_seconds = display_total.and_then(|value| {
                            (speed_bps > 0)
                                .then(|| value.saturating_sub(display_completed) / speed_bps)
                        });
                        emit_task_event(
                            state,
                            json!({
                                "id": task_id,
                                "command": "download_batch",
                                "phase": "正在从相机读取文件",
                                "progress": progress,
                                "completed_bytes": display_completed,
                                "total_bytes": display_total,
                                "speed_bps": speed_bps,
                                "eta_seconds": eta_seconds,
                                "item_index": item_index + 1,
                                "item_count": item_count,
                                "item_name": name,
                            }),
                        );
                        Ok(())
                    },
                );
                match download {
                    Ok(()) => {
                        completed_download_bytes = completed_download_bytes.saturating_add(
                            output
                                .metadata()
                                .map(|metadata| metadata.len())
                                .unwrap_or(0),
                        );
                        completed
                            .push(json!({"name": name, "output": output.display().to_string()}));
                    }
                    Err(error) => {
                        failed.push(json!({"name": name, "error": error.to_string()}));
                        if ensure_camera_transfer_active(state, &camera_host, connection_epoch)
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            Ok(json!({
                "message": format!("批量下载完成：{} 个成功，{} 个失败", completed.len(), failed.len()),
                "completed": completed,
                "failed": failed
            }))
        }
        "prepare_watermark_media" => {
            let payload: PrepareWatermarkMediaPayload = serde_json::from_value(req.payload)?;
            let transfer_control = transfer_control_for(state, task_id);
            wait_for_transfer_permission(&transfer_control)?;
            luna_local::camera_path_from_url(&payload.host, &payload.url)?;
            let output = cached_watermark_source_path(&payload.url, payload.output_dir.as_deref())?;
            if !output.is_file() || output.metadata().map(|meta| meta.len()).unwrap_or(0) == 0 {
                let connection_epoch = active_camera_transfer_epoch(state, &payload.host)?;
                ensure_media_session(state, &payload.host)?;
                let started_at = Instant::now();
                let mut initial_bytes = None;
                luna_local::resume_download_authenticated_with_progress(
                    &payload.url,
                    &output,
                    |downloaded, total| {
                        wait_for_transfer_permission(&transfer_control)?;
                        ensure_camera_transfer_active(state, &payload.host, connection_epoch)?;
                        let initial = *initial_bytes.get_or_insert(downloaded);
                        let transferred = downloaded.saturating_sub(initial);
                        let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
                        let speed_bps = (transferred as f64 / elapsed) as u64;
                        let progress = total
                            .filter(|value| *value > 0)
                            .map(|value| {
                                ((downloaded as f64 / value as f64).clamp(0.0, 1.0) * 100.0).round()
                                    as u8
                            })
                            .unwrap_or(0);
                        let eta_seconds = total.and_then(|value| {
                            (speed_bps > 0).then(|| value.saturating_sub(downloaded) / speed_bps)
                        });
                        emit_task_event(
                            state,
                            json!({
                                "id": task_id,
                                "command": "prepare_watermark_media",
                                "phase": "正在从相机读取水印原片",
                                "progress": progress,
                                "completed_bytes": downloaded,
                                "total_bytes": total,
                                "speed_bps": speed_bps,
                                "eta_seconds": eta_seconds,
                                "item_index": 1,
                                "item_count": 1,
                                "item_name": download_filename(&payload.url),
                            }),
                        );
                        Ok(())
                    },
                )?;
            } else {
                emit_task_event(
                    state,
                    json!({
                        "id": task_id,
                        "command": "prepare_watermark_media",
                        "phase": "使用已下载的本地原片",
                        "progress": 100,
                        "completed_bytes": output.metadata().map(|meta| meta.len()).unwrap_or(0),
                        "total_bytes": output.metadata().map(|meta| meta.len()).unwrap_or(0),
                        "speed_bps": 0,
                        "eta_seconds": 0,
                        "item_index": 1,
                        "item_count": 1,
                        "item_name": download_filename(&payload.url),
                    }),
                );
            }
            Ok(json!({
                "message": "相机原片已载入水印工作区",
                "path": output.display().to_string(),
                "name": download_filename(&payload.url),
            }))
        }
        "watermark_styles" => Ok(serde_json::to_value(watermark::styles())?),
        "watermark_frame_backgrounds" => Ok(serde_json::to_value(watermark::frame_backgrounds())?),
        "watermark_video_plan" => {
            let payload: VideoWatermarkPlanPayload = serde_json::from_value(req.payload)?;
            let plan = watermark::video_watermark_plan(
                payload.width,
                payload.height,
                payload.style.as_deref().unwrap_or("luna-ultra-cn"),
                &payload.position,
            )?;
            Ok(json!({
                "mime": "image/png",
                "data": BASE64_STANDARD.encode(plan.image),
                "width_ratio": plan.width_ratio,
                "x_ratio": plan.x_ratio,
                "bottom_ratio": plan.bottom_ratio,
            }))
        }
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

fn emit_task_event(state: &AndroidState, event: Value) {
    let _ = state.task_event_tx.try_send(event.to_string());
}

fn transfer_control_for(state: &AndroidState, task_id: u64) -> Arc<TransferControl> {
    let mut controls = state
        .transfer_controls
        .lock()
        .expect("transfer control lock");
    controls
        .entry(task_id)
        .or_insert_with(|| Arc::new(TransferControl::new()))
        .clone()
}

fn cancel_all_transfers(state: &AndroidState) {
    let controls = state
        .transfer_controls
        .lock()
        .expect("transfer control lock");
    for control in controls.values() {
        control.state.store(TRANSFER_CANCELLED, Ordering::SeqCst);
    }
}

fn wait_for_transfer_permission(control: &TransferControl) -> anyhow::Result<()> {
    loop {
        match control.state.load(Ordering::SeqCst) {
            TRANSFER_RUNNING => return Ok(()),
            TRANSFER_PAUSED => std::thread::sleep(Duration::from_millis(100)),
            TRANSFER_CANCELLED => return Err(anyhow!("任务已停止，未完成文件已删除")),
            _ => return Err(anyhow!("传输控制状态无效")),
        }
    }
}

fn mark_camera_connected(state: &AndroidState) {
    if !state.camera_connected.swap(true, Ordering::SeqCst) {
        state.connection_epoch.fetch_add(1, Ordering::SeqCst);
    }
}

fn active_camera_transfer_epoch(state: &AndroidState, host: &str) -> anyhow::Result<u64> {
    let epoch = state.connection_epoch.load(Ordering::SeqCst);
    ensure_camera_transfer_active(state, host, epoch)?;
    Ok(epoch)
}

fn ensure_camera_transfer_active(
    state: &AndroidState,
    host: &str,
    epoch: u64,
) -> anyhow::Result<()> {
    if !state.camera_connected.load(Ordering::SeqCst)
        || state.connection_epoch.load(Ordering::SeqCst) != epoch
    {
        return Err(anyhow!("相机连接已断开，下载已取消"));
    }
    let control_active = {
        let control = state.camera_control.lock().expect("control lock");
        control
            .as_ref()
            .map(|session| session.host() == host && session.is_active())
            .unwrap_or(false)
    };
    let media_active = {
        let media = state.media_session.lock().expect("media lock");
        media
            .as_ref()
            .map(|session| session.host() == host && session.is_active())
            .unwrap_or(false)
    };
    let active = control_active || media_active;
    if !active {
        return Err(anyhow!("相机控制连接已失效，下载已取消"));
    }
    Ok(())
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

fn drain_preview_queue(state: &AndroidState) {
    let receiver = state.preview_rx.lock().expect("preview lock");
    while receiver.try_recv().is_ok() {}
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

fn cached_watermark_source_path(url: &str, output_dir: Option<&str>) -> anyhow::Result<PathBuf> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let root = output_dir
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cache").join("watermark-sources"));
    std::fs::create_dir_all(&root)?;
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    Ok(root.join(format!(
        "{:016x}-{}",
        hasher.finish(),
        download_filename(url)
    )))
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
