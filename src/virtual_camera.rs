#![allow(non_snake_case)]

use std::ffi::c_void;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use image::imageops::FilterType;
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, CloseHandle, E_INVALIDARG, E_NOTIMPL,
    E_POINTER, E_UNEXPECTED, ERROR_CANCELLED, ERROR_SUCCESS, HANDLE, S_FALSE, S_OK, WIN32_ERROR,
};
use windows::Win32::Media::KernelStreaming::PINNAME_VIDEO_CAPTURE;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize, IClassFactory, IClassFactory_Impl,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyW,
    RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{
    BOOL, Error as WinError, GUID, HRESULT, IUnknown, Interface, PCWSTR, PWSTR, Ref,
    Result as WinResult, implement,
};

const FRAME_WIDTH: u32 = 1280;
const FRAME_HEIGHT: u32 = 720;
const FRAME_RATE: u32 = 15;
const FRAME_DURATION_100NS: i64 = 10_000_000 / FRAME_RATE as i64;
const FRAME_STALE_AFTER: Duration = Duration::from_secs(30);
const FRAME_BRIDGE_ADDRESS: &str = "127.0.0.1:38475";
const FRAME_BRIDGE_MAGIC: &[u8; 4] = b"LVC1";
const FRAME_BRIDGE_HEADER_LEN: usize = 24;
const FRAME_BRIDGE_MAX_PAYLOAD: usize = (FRAME_WIDTH * FRAME_HEIGHT * 3 / 2) as usize;
const SOURCE_CLSID_TEXT: &str = "{670F22B2-30A4-4C92-8B89-CB7D26783509}";
const SOURCE_CLSID: GUID = GUID::from_u128(0x670f22b2_30a4_4c92_8b89_cb7d26783509);
const SOURCE_REGISTRY_KEY: &str =
    "Software\\Classes\\CLSID\\{670F22B2-30A4-4C92-8B89-CB7D26783509}\\InprocServer32";

#[derive(Clone)]
struct VideoFrames {
    bgra: Arc<Vec<u8>>,
    nv12: Arc<Vec<u8>>,
    updated_at: Instant,
}

pub struct FrameStore {
    enabled: AtomicBool,
    sequence: AtomicU64,
    latest: Mutex<VideoFrames>,
    fallback: VideoFrames,
}

impl FrameStore {
    pub fn new() -> Arc<Self> {
        let fallback = fallback_frames();
        Arc::new(Self {
            enabled: AtomicBool::new(false),
            sequence: AtomicU64::new(1),
            latest: Mutex::new(fallback.clone()),
            fallback,
        })
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    pub fn update_jpeg(&self, jpeg: &[u8]) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }

        let Ok(image) = image::load_from_memory(jpeg) else {
            return;
        };
        let image = if image.width() != FRAME_WIDTH || image.height() != FRAME_HEIGHT {
            image.resize_exact(FRAME_WIDTH, FRAME_HEIGHT, FilterType::Triangle)
        } else {
            image
        };
        let rgba = image.to_rgba8();
        let (bgra, nv12) = rgba_to_video_frames(rgba.as_raw());
        if let Ok(mut latest) = self.latest.lock() {
            *latest = VideoFrames {
                bgra: Arc::new(bgra),
                nv12: Arc::new(nv12),
                updated_at: Instant::now(),
            };
            self.sequence.fetch_add(1, Ordering::AcqRel);
        }
    }

    fn update_nv12(&self, nv12: Vec<u8>) {
        if nv12.len() != FRAME_BRIDGE_MAX_PAYLOAD {
            return;
        }
        if let Ok(mut latest) = self.latest.lock() {
            *latest = VideoFrames {
                bgra: self.fallback.bgra.clone(),
                nv12: Arc::new(nv12),
                updated_at: Instant::now(),
            };
            self.sequence.fetch_add(1, Ordering::AcqRel);
        }
    }

    fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::Acquire)
    }

    fn frame_for(&self, subtype: &GUID) -> Arc<Vec<u8>> {
        let latest = self.latest.lock().ok();
        let frames = latest
            .as_deref()
            .filter(|frame| frame.updated_at.elapsed() <= FRAME_STALE_AFTER)
            .unwrap_or(&self.fallback);
        if subtype == &MFVideoFormat_NV12 {
            frames.nv12.clone()
        } else {
            frames.bgra.clone()
        }
    }
}

fn spawn_frame_bridge_server(
    frames: Arc<FrameStore>,
    stop: Arc<AtomicBool>,
) -> anyhow::Result<JoinHandle<()>> {
    let listener = TcpListener::bind(FRAME_BRIDGE_ADDRESS)
        .map_err(|error| anyhow::anyhow!("无法启动虚拟摄像机画面通道：{error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| anyhow::anyhow!("无法配置虚拟摄像机画面通道：{error}"))?;
    std::thread::Builder::new()
        .name("insta360linker-virtual-camera-frames".into())
        .spawn(move || {
            while !stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let client_frames = frames.clone();
                        let client_stop = stop.clone();
                        let _ = std::thread::Builder::new()
                            .name("insta360linker-virtual-camera-client".into())
                            .spawn(move || {
                                let _ =
                                    serve_frame_bridge_client(&client_frames, &client_stop, stream);
                            });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(error) => {
                        log::warn!("虚拟摄像机画面通道异常：{error}");
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
            }
        })
        .map_err(|error| anyhow::anyhow!("无法创建虚拟摄像机画面线程：{error}"))
}

fn serve_frame_bridge_client(
    frames: &FrameStore,
    stop: &AtomicBool,
    mut stream: TcpStream,
) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut last_sequence = 0u64;
    let mut last_send = Instant::now() - Duration::from_secs(2);
    while !stop.load(Ordering::Acquire) {
        let sequence = frames.sequence();
        if sequence != last_sequence || last_send.elapsed() >= Duration::from_secs(1) {
            let frame = frames.frame_for(&MFVideoFormat_NV12);
            let mut header = [0u8; FRAME_BRIDGE_HEADER_LEN];
            header[..4].copy_from_slice(FRAME_BRIDGE_MAGIC);
            header[4..8].copy_from_slice(&FRAME_WIDTH.to_le_bytes());
            header[8..12].copy_from_slice(&FRAME_HEIGHT.to_le_bytes());
            header[12..16].copy_from_slice(&(frame.len() as u32).to_le_bytes());
            header[16..24].copy_from_slice(&sequence.to_le_bytes());
            stream.write_all(&header)?;
            stream.write_all(frame.as_slice())?;
            last_sequence = sequence;
            last_send = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn spawn_frame_bridge_client(frames: Arc<FrameStore>) {
    let _ = std::thread::Builder::new()
        .name("insta360linker-virtual-camera-receiver".into())
        .spawn(move || {
            loop {
                match TcpStream::connect(FRAME_BRIDGE_ADDRESS) {
                    Ok(stream) => {
                        let _ = receive_frame_bridge(&frames, stream);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(500)),
                }
            }
        });
}

fn receive_frame_bridge(frames: &FrameStore, mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    loop {
        let mut header = [0u8; FRAME_BRIDGE_HEADER_LEN];
        stream.read_exact(&mut header)?;
        if &header[..4] != FRAME_BRIDGE_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid frame bridge header",
            ));
        }
        let width = u32::from_le_bytes(header[4..8].try_into().unwrap_or_default());
        let height = u32::from_le_bytes(header[8..12].try_into().unwrap_or_default());
        let payload_len =
            u32::from_le_bytes(header[12..16].try_into().unwrap_or_default()) as usize;
        if width != FRAME_WIDTH || height != FRAME_HEIGHT || payload_len != FRAME_BRIDGE_MAX_PAYLOAD
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid virtual camera frame",
            ));
        }
        let mut nv12 = vec![0u8; payload_len];
        stream.read_exact(&mut nv12)?;
        frames.update_nv12(nv12);
    }
}

pub struct VirtualCameraController {
    frames: Arc<FrameStore>,
    command_tx: mpsc::Sender<VirtualCameraCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
    bridge_stop: Arc<AtomicBool>,
    bridge_worker: Mutex<Option<JoinHandle<()>>>,
    started: AtomicBool,
}

enum VirtualCameraCommand {
    Start(mpsc::Sender<Result<String, String>>),
    Stop(mpsc::Sender<Result<String, String>>),
    Shutdown,
}

impl VirtualCameraController {
    pub fn new(frames: Arc<FrameStore>) -> anyhow::Result<Arc<Self>> {
        let bridge_stop = Arc::new(AtomicBool::new(false));
        let bridge_worker = spawn_frame_bridge_server(frames.clone(), bridge_stop.clone())?;
        let (command_tx, command_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker_frames = frames.clone();
        let worker = std::thread::Builder::new()
            .name("insta360linker-virtual-camera".into())
            .spawn(move || virtual_camera_worker(worker_frames, command_rx, ready_tx))
            .map_err(|error| anyhow::anyhow!("创建虚拟摄像机服务失败：{error}"))?;
        match ready_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                bridge_stop.store(true, Ordering::Release);
                let _ = bridge_worker.join();
                let _ = worker.join();
                anyhow::bail!("{error}");
            }
            Err(error) => {
                bridge_stop.store(true, Ordering::Release);
                let _ = bridge_worker.join();
                let _ = worker.join();
                anyhow::bail!("虚拟摄像机服务未能启动：{error}");
            }
        }

        Ok(Arc::new(Self {
            frames,
            command_tx,
            worker: Mutex::new(Some(worker)),
            bridge_stop,
            bridge_worker: Mutex::new(Some(bridge_worker)),
            started: AtomicBool::new(false),
        }))
    }

    pub fn start(&self) -> anyhow::Result<String> {
        if self.started.load(Ordering::Acquire) {
            return Ok("Insta360Linker 虚拟摄像机已开启".into());
        }

        self.frames.set_enabled(true);
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .command_tx
            .send(VirtualCameraCommand::Start(reply_tx))
            .is_err()
        {
            self.frames.set_enabled(false);
            anyhow::bail!("虚拟摄像机服务已停止");
        }
        let response = match reply_rx.recv() {
            Ok(response) => response,
            Err(_) => {
                self.frames.set_enabled(false);
                anyhow::bail!("虚拟摄像机服务没有响应");
            }
        };
        match response {
            Ok(message) => {
                self.started.store(true, Ordering::Release);
                Ok(message)
            }
            Err(error) => {
                self.frames.set_enabled(false);
                anyhow::bail!("{error}")
            }
        }
    }

    pub fn stop(&self) -> anyhow::Result<String> {
        self.frames.set_enabled(false);
        self.started.store(false, Ordering::Release);
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(VirtualCameraCommand::Stop(reply_tx))
            .map_err(|_| anyhow::anyhow!("虚拟摄像机服务已停止"))?;
        match reply_rx
            .recv()
            .map_err(|_| anyhow::anyhow!("虚拟摄像机服务没有响应"))?
        {
            Ok(message) => Ok(message),
            Err(error) => anyhow::bail!("{error}"),
        }
    }

    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }
}

impl Drop for VirtualCameraController {
    fn drop(&mut self) {
        self.frames.set_enabled(false);
        let _ = self.command_tx.send(VirtualCameraCommand::Shutdown);
        if let Ok(worker) = self.worker.get_mut() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
        self.bridge_stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(FRAME_BRIDGE_ADDRESS);
        if let Ok(worker) = self.bridge_worker.get_mut() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

fn virtual_camera_worker(
    _frames: Arc<FrameStore>,
    command_rx: mpsc::Receiver<VirtualCameraCommand>,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
) {
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() };
    if let Err(error) = initialized {
        let _ = ready_tx.send(Err(format!("初始化 Windows COM 失败：{error}")));
        return;
    }
    if let Err(error) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) } {
        let _ = ready_tx.send(Err(format!("初始化 Windows 媒体组件失败：{error}")));
        unsafe {
            CoUninitialize();
        }
        return;
    }
    let _ = ready_tx.send(Ok(()));

    let mut camera: Option<IMFVirtualCamera> = None;
    while let Ok(command) = command_rx.recv() {
        match command {
            VirtualCameraCommand::Start(reply) => {
                let result = ensure_media_source_installed()
                    .and_then(|_| start_system_virtual_camera(&mut camera))
                    .map_err(|error| error.to_string());
                let _ = reply.send(result);
            }
            VirtualCameraCommand::Stop(reply) => {
                stop_system_virtual_camera(&mut camera);
                let _ = reply.send(Ok("虚拟摄像机已关闭".into()));
            }
            VirtualCameraCommand::Shutdown => break,
        }
    }

    stop_system_virtual_camera(&mut camera);
    unsafe {
        let _ = MFShutdown();
        CoUninitialize();
    }
}

pub fn handle_installer_mode() -> Option<i32> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--install-virtual-camera")) {
        return None;
    }
    let Some(path) = args.next() else {
        return Some(2);
    };
    Some(match install_media_source_system(Path::new(&path)) {
        Ok(()) => 0,
        Err(error) => {
            log::error!("安装虚拟摄像机组件失败：{error}");
            1
        }
    })
}

fn ensure_media_source_installed() -> anyhow::Result<()> {
    let dll_path = find_media_source_dll()?;
    if registered_media_source_path()
        .as_deref()
        .is_some_and(|registered| paths_equal(registered, &dll_path))
    {
        return Ok(());
    }

    launch_elevated_installer(&dll_path)?;
    let registered = registered_media_source_path()
        .ok_or_else(|| anyhow::anyhow!("虚拟摄像机组件安装未完成"))?;
    if !paths_equal(&registered, &dll_path) {
        anyhow::bail!("虚拟摄像机组件安装路径不正确");
    }
    Ok(())
}

fn install_media_source_system(dll_path: &Path) -> anyhow::Result<()> {
    if !dll_path.is_file() {
        anyhow::bail!("缺少虚拟摄像机组件：{}", dll_path.display());
    }
    let key_path = wide_string(SOURCE_REGISTRY_KEY);
    let mut key = HKEY::default();
    let create_result =
        unsafe { RegCreateKeyW(HKEY_LOCAL_MACHINE, PCWSTR(key_path.as_ptr()), &mut key) };
    win32_result(create_result)?;

    let set_path_result = set_registry_string(key, PCWSTR::null(), &dll_path.display().to_string());
    let threading_name = wide_string("ThreadingModel");
    let set_threading_result = set_registry_string(key, PCWSTR(threading_name.as_ptr()), "Both");
    unsafe {
        let _ = RegCloseKey(key);
    }
    set_path_result?;
    set_threading_result
}

fn find_media_source_dll() -> anyhow::Result<PathBuf> {
    if crate::embedded_windows::has_embedded_virtual_camera_dll() {
        return crate::embedded_windows::virtual_camera_dll_path();
    }

    let executable =
        std::env::current_exe().map_err(|error| anyhow::anyhow!("无法读取程序路径：{error}"))?;
    let parent = executable
        .parent()
        .ok_or_else(|| anyhow::anyhow!("程序目录不可用"))?;
    let mut candidates = vec![
        parent.join("Insta360LinkerVirtualCamera.dll"),
        parent.join("insta360_linker.dll"),
    ];
    if let Some(build_dir) = parent.parent() {
        candidates.push(build_dir.join("insta360_linker.dll"));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow::anyhow!("缺少 Insta360LinkerVirtualCamera.dll"))
}

fn registered_media_source_path() -> Option<PathBuf> {
    let key_path = wide_string(SOURCE_REGISTRY_KEY);
    let mut key = HKEY::default();
    let open_result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key_path.as_ptr()),
            None,
            KEY_READ,
            &mut key,
        )
    };
    if open_result != ERROR_SUCCESS {
        return None;
    }

    let mut byte_len = 0u32;
    let size_result = unsafe {
        RegGetValueW(
            key,
            PCWSTR::null(),
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut byte_len),
        )
    };
    if size_result != ERROR_SUCCESS || byte_len < 2 {
        unsafe {
            let _ = RegCloseKey(key);
        }
        return None;
    }

    let mut value = vec![0u16; (byte_len as usize + 1) / 2];
    let read_result = unsafe {
        RegGetValueW(
            key,
            PCWSTR::null(),
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut byte_len),
        )
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    if read_result != ERROR_SUCCESS {
        return None;
    }
    let value_len = value
        .iter()
        .position(|item| *item == 0)
        .unwrap_or(value.len());
    Some(PathBuf::from(String::from_utf16_lossy(&value[..value_len])))
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn launch_elevated_installer(dll_path: &Path) -> anyhow::Result<()> {
    let executable =
        std::env::current_exe().map_err(|error| anyhow::anyhow!("无法读取程序路径：{error}"))?;
    let executable = wide_string(&executable.display().to_string());
    let verb = wide_string("runas");
    let parameters = wide_string(&format!(
        "--install-virtual-camera \"{}\"",
        dll_path.display()
    ));
    let mut execute = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(executable.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    if let Err(error) = unsafe { ShellExecuteExW(&mut execute) } {
        if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
            anyhow::bail!("已取消虚拟摄像机组件安装");
        }
        anyhow::bail!("无法启动虚拟摄像机组件安装：{error}");
    }
    wait_for_installer(execute.hProcess)
}

fn wait_for_installer(process: HANDLE) -> anyhow::Result<()> {
    if process.is_invalid() {
        anyhow::bail!("虚拟摄像机组件安装进程不可用");
    }
    unsafe {
        WaitForSingleObject(process, INFINITE);
    }
    let mut exit_code = 1u32;
    let read_result = unsafe { GetExitCodeProcess(process, &mut exit_code) };
    unsafe {
        let _ = CloseHandle(process);
    }
    read_result.map_err(|error| anyhow::anyhow!("无法读取安装结果：{error}"))?;
    if exit_code == 0 {
        Ok(())
    } else {
        anyhow::bail!("虚拟摄像机组件安装失败")
    }
}

fn set_registry_string(key: HKEY, name: PCWSTR, value: &str) -> anyhow::Result<()> {
    let value = wide_string(value);
    let value_bytes = unsafe {
        std::slice::from_raw_parts(
            value.as_ptr().cast::<u8>(),
            value.len() * std::mem::size_of::<u16>(),
        )
    };
    let result = unsafe { RegSetValueExW(key, name, None, REG_SZ, Some(value_bytes)) };
    win32_result(result)
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn win32_result(result: WIN32_ERROR) -> anyhow::Result<()> {
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{}",
            WinError::from_hresult(HRESULT::from_win32(result.0))
        ))
    }
}

fn start_system_virtual_camera(camera: &mut Option<IMFVirtualCamera>) -> anyhow::Result<String> {
    if camera.is_some() {
        return Ok("Insta360Linker 虚拟摄像机已开启".into());
    }
    let friendly_name: Vec<u16> = "Insta360Linker Camera\0".encode_utf16().collect();
    let source_id: Vec<u16> = format!("{SOURCE_CLSID_TEXT}\0").encode_utf16().collect();
    let virtual_camera = unsafe {
        MFCreateVirtualCamera(
            MFVirtualCameraType_SoftwareCameraSource,
            MFVirtualCameraLifetime_Session,
            MFVirtualCameraAccess_CurrentUser,
            PCWSTR(friendly_name.as_ptr()),
            PCWSTR(source_id.as_ptr()),
            None,
        )
        .map_err(|error| virtual_camera_error("创建虚拟摄像机", error))?
    };
    unsafe {
        virtual_camera
            .Start(None)
            .map_err(|error| virtual_camera_error("启动虚拟摄像机", error))?;
    }
    *camera = Some(virtual_camera);
    Ok("Insta360Linker 虚拟摄像机已开启".into())
}

fn stop_system_virtual_camera(camera: &mut Option<IMFVirtualCamera>) {
    if let Some(camera) = camera.take() {
        unsafe {
            let _ = camera.Stop();
            let _ = camera.Shutdown();
        }
    }
}

fn virtual_camera_error(action: &str, error: WinError) -> anyhow::Error {
    let code = error.code();
    if code.0 as u32 == 0x8007_0057 {
        anyhow::anyhow!("{action}失败：当前 Windows 版本不支持系统虚拟摄像机")
    } else if code.0 as u32 == 0x8007_0005 {
        anyhow::anyhow!("{action}失败：请在 Windows 隐私设置中允许桌面应用访问摄像头")
    } else {
        anyhow::anyhow!("{action}失败：{error}")
    }
}

fn fallback_frames() -> VideoFrames {
    let mut bgra = vec![0u8; (FRAME_WIDTH * FRAME_HEIGHT * 4) as usize];
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[22, 25, 30, 255]);
    }

    let y_len = (FRAME_WIDTH * FRAME_HEIGHT) as usize;
    let mut nv12 = vec![128u8; y_len + y_len / 2];
    nv12[..y_len].fill(30);
    VideoFrames {
        bgra: Arc::new(bgra),
        nv12: Arc::new(nv12),
        updated_at: Instant::now() - FRAME_STALE_AFTER,
    }
}

fn rgba_to_video_frames(rgba: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let pixel_count = (FRAME_WIDTH * FRAME_HEIGHT) as usize;
    let mut bgra = vec![0u8; pixel_count * 4];
    let mut nv12 = vec![0u8; pixel_count + pixel_count / 2];

    for (index, source) in rgba.chunks_exact(4).enumerate() {
        let target = &mut bgra[index * 4..index * 4 + 4];
        target.copy_from_slice(&[source[2], source[1], source[0], 255]);
        nv12[index] = rgb_to_y(source[0], source[1], source[2]);
    }

    let uv_offset = pixel_count;
    let width = FRAME_WIDTH as usize;
    let height = FRAME_HEIGHT as usize;
    for row in (0..height).step_by(2) {
        for column in (0..width).step_by(2) {
            let mut u_sum = 0u16;
            let mut v_sum = 0u16;
            for dy in 0..2 {
                for dx in 0..2 {
                    let index = ((row + dy) * width + column + dx) * 4;
                    let r = rgba[index];
                    let g = rgba[index + 1];
                    let b = rgba[index + 2];
                    u_sum += rgb_to_u(r, g, b) as u16;
                    v_sum += rgb_to_v(r, g, b) as u16;
                }
            }
            let uv_index = uv_offset + (row / 2) * width + column;
            nv12[uv_index] = (u_sum / 4) as u8;
            nv12[uv_index + 1] = (v_sum / 4) as u8;
        }
    }
    (bgra, nv12)
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

fn rgb_to_y(r: u8, g: u8, b: u8) -> u8 {
    clamp_u8(((66 * r as i32 + 129 * g as i32 + 25 * b as i32 + 128) >> 8) + 16)
}

fn rgb_to_u(r: u8, g: u8, b: u8) -> u8 {
    clamp_u8(((-38 * r as i32 - 74 * g as i32 + 112 * b as i32 + 128) >> 8) + 128)
}

fn rgb_to_v(r: u8, g: u8, b: u8) -> u8 {
    clamp_u8(((112 * r as i32 - 94 * g as i32 - 18 * b as i32 + 128) >> 8) + 128)
}

static DLL_FRAME_STORE: OnceLock<Arc<FrameStore>> = OnceLock::new();

pub unsafe fn dll_get_class_object(
    clsid: *const GUID,
    riid: *const GUID,
    result: *mut *mut c_void,
) -> HRESULT {
    if clsid.is_null() || riid.is_null() || result.is_null() {
        return E_POINTER;
    }
    unsafe {
        result.write(std::ptr::null_mut());
        if *clsid != SOURCE_CLSID {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
    }

    let frames = DLL_FRAME_STORE
        .get_or_init(|| {
            let frames = FrameStore::new();
            spawn_frame_bridge_client(frames.clone());
            frames
        })
        .clone();
    let factory: IClassFactory = CameraClassFactory { frames }.into();
    let unknown: WinResult<IUnknown> = factory.cast();
    match unknown {
        Ok(unknown) => unsafe { unknown.query(riid, result) },
        Err(error) => error.code(),
    }
}

pub fn dll_can_unload_now() -> HRESULT {
    S_FALSE
}

#[implement(IClassFactory)]
struct CameraClassFactory {
    frames: Arc<FrameStore>,
}

impl IClassFactory_Impl for CameraClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        riid: *const GUID,
        result: *mut *mut c_void,
    ) -> WinResult<()> {
        if !outer.is_null() {
            return Err(WinError::from_hresult(CLASS_E_NOAGGREGATION));
        }
        if result.is_null() {
            return Err(WinError::from_hresult(E_POINTER));
        }
        unsafe {
            result.write(std::ptr::null_mut());
        }

        let activation: IMFActivate = CameraActivation::new(self.frames.clone())?.into();
        let unknown: IUnknown = activation.cast()?;
        unsafe { unknown.query(riid, result).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> WinResult<()> {
        Ok(())
    }
}

#[implement(IMFActivate)]
struct CameraActivation {
    attributes: IMFAttributes,
    frames: Arc<FrameStore>,
    active_source: Mutex<Option<IMFMediaSource>>,
}

impl CameraActivation {
    fn new(frames: Arc<FrameStore>) -> WinResult<Self> {
        let attributes = create_attributes(12)?;
        Ok(Self {
            attributes,
            frames,
            active_source: Mutex::new(None),
        })
    }
}

impl IMFActivate_Impl for CameraActivation_Impl {
    fn ActivateObject(&self, riid: *const GUID, result: *mut *mut c_void) -> WinResult<()> {
        if result.is_null() {
            return Err(WinError::from_hresult(E_POINTER));
        }
        unsafe {
            result.write(std::ptr::null_mut());
        }
        let source = create_media_source(self.frames.clone(), Some(&self.attributes))?;
        let unknown: IUnknown = source.cast()?;
        unsafe {
            unknown.query(riid, result).ok()?;
        }
        if let Ok(mut active) = self.active_source.lock() {
            *active = Some(source);
        }
        Ok(())
    }

    fn ShutdownObject(&self) -> WinResult<()> {
        if let Ok(mut active) = self.active_source.lock() {
            if let Some(source) = active.take() {
                unsafe {
                    let _ = source.Shutdown();
                }
            }
        }
        Ok(())
    }

    fn DetachObject(&self) -> WinResult<()> {
        if let Ok(mut active) = self.active_source.lock() {
            *active = None;
        }
        Ok(())
    }
}

impl IMFAttributes_Impl for CameraActivation_Impl {
    fn GetItem(&self, key: *const GUID, value: *mut PROPVARIANT) -> WinResult<()> {
        unsafe { self.attributes.GetItem(key, Some(value)) }
    }

    fn GetItemType(&self, key: *const GUID) -> WinResult<MF_ATTRIBUTE_TYPE> {
        unsafe { self.attributes.GetItemType(key) }
    }

    fn CompareItem(&self, key: *const GUID, value: *const PROPVARIANT) -> WinResult<BOOL> {
        unsafe { self.attributes.CompareItem(key, value) }
    }

    fn Compare(
        &self,
        theirs: Ref<'_, IMFAttributes>,
        match_type: MF_ATTRIBUTES_MATCH_TYPE,
    ) -> WinResult<BOOL> {
        unsafe { self.attributes.Compare(theirs.as_ref(), match_type) }
    }

    fn GetUINT32(&self, key: *const GUID) -> WinResult<u32> {
        unsafe { self.attributes.GetUINT32(key) }
    }

    fn GetUINT64(&self, key: *const GUID) -> WinResult<u64> {
        unsafe { self.attributes.GetUINT64(key) }
    }

    fn GetDouble(&self, key: *const GUID) -> WinResult<f64> {
        unsafe { self.attributes.GetDouble(key) }
    }

    fn GetGUID(&self, key: *const GUID) -> WinResult<GUID> {
        unsafe { self.attributes.GetGUID(key) }
    }

    fn GetStringLength(&self, key: *const GUID) -> WinResult<u32> {
        unsafe { self.attributes.GetStringLength(key) }
    }

    fn GetString(
        &self,
        key: *const GUID,
        value: PWSTR,
        value_len: u32,
        actual_len: *mut u32,
    ) -> WinResult<()> {
        unsafe {
            let vtable = Interface::vtable(&self.attributes);
            (vtable.GetString)(
                Interface::as_raw(&self.attributes),
                key,
                value,
                value_len,
                actual_len,
            )
            .ok()
        }
    }

    fn GetAllocatedString(
        &self,
        key: *const GUID,
        value: *mut PWSTR,
        actual_len: *mut u32,
    ) -> WinResult<()> {
        unsafe { self.attributes.GetAllocatedString(key, value, actual_len) }
    }

    fn GetBlobSize(&self, key: *const GUID) -> WinResult<u32> {
        unsafe { self.attributes.GetBlobSize(key) }
    }

    fn GetBlob(
        &self,
        key: *const GUID,
        value: *mut u8,
        value_len: u32,
        actual_len: *mut u32,
    ) -> WinResult<()> {
        unsafe {
            let vtable = Interface::vtable(&self.attributes);
            (vtable.GetBlob)(
                Interface::as_raw(&self.attributes),
                key,
                value,
                value_len,
                actual_len,
            )
            .ok()
        }
    }

    fn GetAllocatedBlob(
        &self,
        key: *const GUID,
        value: *mut *mut u8,
        actual_len: *mut u32,
    ) -> WinResult<()> {
        unsafe { self.attributes.GetAllocatedBlob(key, value, actual_len) }
    }

    fn GetUnknown(
        &self,
        key: *const GUID,
        riid: *const GUID,
        result: *mut *mut c_void,
    ) -> WinResult<()> {
        unsafe {
            let vtable = Interface::vtable(&self.attributes);
            (vtable.GetUnknown)(Interface::as_raw(&self.attributes), key, riid, result).ok()
        }
    }

    fn SetItem(&self, key: *const GUID, value: *const PROPVARIANT) -> WinResult<()> {
        unsafe { self.attributes.SetItem(key, value) }
    }

    fn DeleteItem(&self, key: *const GUID) -> WinResult<()> {
        unsafe { self.attributes.DeleteItem(key) }
    }

    fn DeleteAllItems(&self) -> WinResult<()> {
        unsafe { self.attributes.DeleteAllItems() }
    }

    fn SetUINT32(&self, key: *const GUID, value: u32) -> WinResult<()> {
        unsafe { self.attributes.SetUINT32(key, value) }
    }

    fn SetUINT64(&self, key: *const GUID, value: u64) -> WinResult<()> {
        unsafe { self.attributes.SetUINT64(key, value) }
    }

    fn SetDouble(&self, key: *const GUID, value: f64) -> WinResult<()> {
        unsafe { self.attributes.SetDouble(key, value) }
    }

    fn SetGUID(&self, key: *const GUID, value: *const GUID) -> WinResult<()> {
        unsafe { self.attributes.SetGUID(key, value) }
    }

    fn SetString(&self, key: *const GUID, value: &PCWSTR) -> WinResult<()> {
        unsafe { self.attributes.SetString(key, *value) }
    }

    fn SetBlob(&self, key: *const GUID, value: *const u8, value_len: u32) -> WinResult<()> {
        unsafe {
            let vtable = Interface::vtable(&self.attributes);
            (vtable.SetBlob)(Interface::as_raw(&self.attributes), key, value, value_len).ok()
        }
    }

    fn SetUnknown(&self, key: *const GUID, value: Ref<'_, IUnknown>) -> WinResult<()> {
        unsafe { self.attributes.SetUnknown(key, value.as_ref()) }
    }

    fn LockStore(&self) -> WinResult<()> {
        unsafe { self.attributes.LockStore() }
    }

    fn UnlockStore(&self) -> WinResult<()> {
        unsafe { self.attributes.UnlockStore() }
    }

    fn GetCount(&self) -> WinResult<u32> {
        unsafe { self.attributes.GetCount() }
    }

    fn GetItemByIndex(&self, index: u32, key: *mut GUID, value: *mut PROPVARIANT) -> WinResult<()> {
        unsafe { self.attributes.GetItemByIndex(index, key, Some(value)) }
    }

    fn CopyAllItems(&self, destination: Ref<'_, IMFAttributes>) -> WinResult<()> {
        unsafe { self.attributes.CopyAllItems(destination.as_ref()) }
    }
}

struct MediaSourceState {
    frames: Arc<FrameStore>,
    source_queue: IMFMediaEventQueue,
    stream_queue: IMFMediaEventQueue,
    source_attributes: IMFAttributes,
    stream_attributes: IMFAttributes,
    stream_descriptor: IMFStreamDescriptor,
    presentation_descriptor: IMFPresentationDescriptor,
    source: Mutex<Option<IMFMediaSource>>,
    stream: Mutex<Option<IMFMediaStream2>>,
    current_subtype: Mutex<GUID>,
    stream_state: Mutex<MF_STREAM_STATE>,
    source_started: AtomicBool,
    shutdown: AtomicBool,
}

fn create_attributes(initial_size: u32) -> WinResult<IMFAttributes> {
    let mut attributes = None;
    unsafe {
        MFCreateAttributes(&mut attributes, initial_size)?;
    }
    attributes.ok_or_else(unexpected)
}

fn create_media_source(
    frames: Arc<FrameStore>,
    activation_attributes: Option<&IMFAttributes>,
) -> WinResult<IMFMediaSource> {
    unsafe {
        let nv12 = create_video_type(MFVideoFormat_NV12)?;
        let stream_descriptor = MFCreateStreamDescriptor(0, &[Some(nv12.clone())])?;
        let handler = stream_descriptor.GetMediaTypeHandler()?;
        handler.SetCurrentMediaType(&nv12)?;

        let stream_attributes: IMFAttributes = stream_descriptor.cast()?;
        set_stream_attributes(&stream_attributes)?;

        let presentation_descriptor =
            MFCreatePresentationDescriptor(Some(&[Some(stream_descriptor.clone())]))?;
        presentation_descriptor.SelectStream(0)?;

        let source_attributes = create_attributes(12)?;
        if let Some(activation_attributes) = activation_attributes {
            activation_attributes.CopyAllItems(&source_attributes)?;
        }

        let state = Arc::new(MediaSourceState {
            frames,
            source_queue: MFCreateEventQueue()?,
            stream_queue: MFCreateEventQueue()?,
            source_attributes,
            stream_attributes,
            stream_descriptor,
            presentation_descriptor,
            source: Mutex::new(None),
            stream: Mutex::new(None),
            current_subtype: Mutex::new(MFVideoFormat_NV12),
            stream_state: Mutex::new(MF_STREAM_STATE_STOPPED),
            source_started: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
        });

        let source_ex: IMFMediaSourceEx = VirtualMediaSource {
            state: state.clone(),
        }
        .into();
        let source: IMFMediaSource = source_ex.cast()?;
        let stream: IMFMediaStream2 = VirtualMediaStream {
            state: state.clone(),
        }
        .into();
        *state.source.lock().map_err(|_| unexpected())? = Some(source.clone());
        *state.stream.lock().map_err(|_| unexpected())? = Some(stream);
        Ok(source)
    }
}

unsafe fn create_video_type(subtype: GUID) -> WinResult<IMFMediaType> {
    let media_type = unsafe { MFCreateMediaType()? };
    unsafe {
        media_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        media_type.SetGUID(&MF_MT_SUBTYPE, &subtype)?;
        media_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        media_type.SetUINT32(&MF_MT_ALL_SAMPLES_INDEPENDENT, 1)?;
        media_type.SetUINT64(
            &MF_MT_FRAME_SIZE,
            ((FRAME_WIDTH as u64) << 32) | FRAME_HEIGHT as u64,
        )?;
        media_type.SetUINT64(&MF_MT_FRAME_RATE, ((FRAME_RATE as u64) << 32) | 1)?;
        media_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
        media_type.SetUINT32(
            &MF_MT_AVG_BITRATE,
            if subtype == MFVideoFormat_NV12 {
                FRAME_WIDTH * FRAME_HEIGHT * 12 * FRAME_RATE
            } else {
                FRAME_WIDTH * FRAME_HEIGHT * 32 * FRAME_RATE
            },
        )?;
        if subtype == MFVideoFormat_RGB32 {
            media_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, FRAME_WIDTH * 4)?;
        }
    }
    Ok(media_type)
}

unsafe fn set_stream_attributes(attributes: &IMFAttributes) -> WinResult<()> {
    unsafe {
        attributes.SetGUID(&MF_DEVICESTREAM_STREAM_CATEGORY, &PINNAME_VIDEO_CAPTURE)?;
        attributes.SetUINT32(&MF_DEVICESTREAM_STREAM_ID, 0)?;
        attributes.SetUINT32(&MF_DEVICESTREAM_FRAMESERVER_SHARED, 1)?;
        attributes.SetUINT32(
            &MF_DEVICESTREAM_ATTRIBUTE_FRAMESOURCE_TYPES,
            MFFrameSourceTypes_Color.0 as u32,
        )?;
    }
    Ok(())
}

fn unexpected() -> WinError {
    WinError::from_hresult(E_UNEXPECTED)
}

fn shutdown_error() -> WinError {
    WinError::from_hresult(MF_E_SHUTDOWN)
}

fn check_active(state: &MediaSourceState) -> WinResult<()> {
    if state.shutdown.load(Ordering::Acquire) {
        Err(shutdown_error())
    } else {
        Ok(())
    }
}

#[implement(IMFMediaSourceEx, IMFSampleAllocatorControl)]
struct VirtualMediaSource {
    state: Arc<MediaSourceState>,
}

impl IMFMediaEventGenerator_Impl for VirtualMediaSource_Impl {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> WinResult<IMFMediaEvent> {
        check_active(&self.state)?;
        unsafe { self.state.source_queue.GetEvent(flags.0 as u32) }
    }

    fn BeginGetEvent(
        &self,
        callback: Ref<'_, IMFAsyncCallback>,
        callback_state: Ref<'_, IUnknown>,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        unsafe {
            self.state
                .source_queue
                .BeginGetEvent(callback.as_ref(), callback_state.as_ref())
        }
    }

    fn EndGetEvent(&self, result: Ref<'_, IMFAsyncResult>) -> WinResult<IMFMediaEvent> {
        check_active(&self.state)?;
        unsafe { self.state.source_queue.EndGetEvent(result.as_ref()) }
    }

    fn QueueEvent(
        &self,
        event_type: u32,
        extended_type: *const GUID,
        status: HRESULT,
        value: *const PROPVARIANT,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        unsafe {
            self.state
                .source_queue
                .QueueEventParamVar(event_type, extended_type, status, value)
        }
    }
}

impl IMFMediaSource_Impl for VirtualMediaSource_Impl {
    fn GetCharacteristics(&self) -> WinResult<u32> {
        check_active(&self.state)?;
        Ok(MFMEDIASOURCE_IS_LIVE.0 as u32)
    }

    fn CreatePresentationDescriptor(&self) -> WinResult<IMFPresentationDescriptor> {
        check_active(&self.state)?;
        unsafe { self.state.presentation_descriptor.Clone() }
    }

    fn Start(
        &self,
        presentation: Ref<'_, IMFPresentationDescriptor>,
        time_format: *const GUID,
        start_position: *const PROPVARIANT,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        if presentation.is_null() || start_position.is_null() {
            return Err(WinError::from_hresult(E_INVALIDARG));
        }
        if !time_format.is_null() {
            let format = unsafe { *time_format };
            if format != GUID::zeroed() {
                return Err(WinError::from_hresult(MF_E_UNSUPPORTED_TIME_FORMAT));
            }
        }

        let presentation = presentation.ok()?;
        let count = unsafe { presentation.GetStreamDescriptorCount()? };
        if count != 1 {
            return Err(WinError::from_hresult(E_INVALIDARG));
        }

        let mut selected = BOOL(0);
        let mut descriptor = None;
        unsafe {
            presentation.GetStreamDescriptorByIndex(0, &mut selected, &mut descriptor)?;
        }
        if !selected.as_bool() {
            return Err(WinError::from_hresult(E_INVALIDARG));
        }

        let descriptor = descriptor.ok_or_else(|| WinError::from_hresult(E_INVALIDARG))?;
        if unsafe { descriptor.GetStreamIdentifier()? } != 0 {
            return Err(WinError::from_hresult(MF_E_NOT_FOUND));
        }
        let media_type = unsafe { descriptor.GetMediaTypeHandler()?.GetCurrentMediaType()? };
        if unsafe { media_type.GetGUID(&MF_MT_MAJOR_TYPE)? } != MFMediaType_Video {
            return Err(WinError::from_hresult(E_INVALIDARG));
        }
        let subtype = unsafe { media_type.GetGUID(&MF_MT_SUBTYPE)? };
        if subtype != MFVideoFormat_NV12 {
            return Err(WinError::from_hresult(E_INVALIDARG));
        }
        *self
            .state
            .current_subtype
            .lock()
            .map_err(|_| unexpected())? = subtype;
        unsafe {
            self.state
                .stream_descriptor
                .GetMediaTypeHandler()?
                .SetCurrentMediaType(&media_type)?;
            self.state.presentation_descriptor.SelectStream(0)?;
        }

        let stream = self
            .state
            .stream
            .lock()
            .map_err(|_| unexpected())?
            .clone()
            .ok_or_else(unexpected)?;
        let stream_unknown: IUnknown = stream.cast()?;
        let event = if self.state.source_started.load(Ordering::Acquire) {
            MEUpdatedStream
        } else {
            MENewStream
        };
        let start_time = PROPVARIANT::from(unsafe { MFGetSystemTime() });
        unsafe {
            self.state.source_queue.QueueEventParamUnk(
                event.0 as u32,
                &GUID::zeroed(),
                S_OK,
                &stream_unknown,
            )?;
            self.state.stream_queue.QueueEventParamVar(
                MEStreamStarted.0 as u32,
                &GUID::zeroed(),
                S_OK,
                std::ptr::null(),
            )?;
            self.state.source_queue.QueueEventParamVar(
                MESourceStarted.0 as u32,
                &GUID::zeroed(),
                S_OK,
                &start_time,
            )?;
        }
        *self.state.stream_state.lock().map_err(|_| unexpected())? = MF_STREAM_STATE_RUNNING;
        self.state.source_started.store(true, Ordering::Release);
        Ok(())
    }

    fn Stop(&self) -> WinResult<()> {
        check_active(&self.state)?;
        if !self.state.source_started.load(Ordering::Acquire) {
            return Err(WinError::from_hresult(MF_E_INVALID_STATE_TRANSITION));
        }
        *self.state.stream_state.lock().map_err(|_| unexpected())? = MF_STREAM_STATE_STOPPED;
        unsafe {
            self.state.presentation_descriptor.DeselectStream(0)?;
        }
        self.state.source_started.store(false, Ordering::Release);
        let stop_time = PROPVARIANT::from(unsafe { MFGetSystemTime() });
        unsafe {
            self.state.stream_queue.QueueEventParamVar(
                MEStreamStopped.0 as u32,
                &GUID::zeroed(),
                S_OK,
                std::ptr::null(),
            )?;
            self.state.source_queue.QueueEventParamVar(
                MESourceStopped.0 as u32,
                &GUID::zeroed(),
                S_OK,
                &stop_time,
            )
        }
    }

    fn Pause(&self) -> WinResult<()> {
        Err(WinError::from_hresult(MF_E_INVALID_STATE_TRANSITION))
    }

    fn Shutdown(&self) -> WinResult<()> {
        if self.state.shutdown.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        unsafe {
            let _ = self.state.source_queue.Shutdown();
            let _ = self.state.stream_queue.Shutdown();
        }
        if let Ok(mut source) = self.state.source.lock() {
            *source = None;
        }
        if let Ok(mut stream) = self.state.stream.lock() {
            *stream = None;
        }
        Ok(())
    }
}

impl IMFSampleAllocatorControl_Impl for VirtualMediaSource_Impl {
    fn SetDefaultAllocator(
        &self,
        output_stream_id: u32,
        _allocator: Ref<'_, IUnknown>,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        if output_stream_id != 0 {
            return Err(WinError::from_hresult(MF_E_NOT_FOUND));
        }
        Ok(())
    }

    fn GetAllocatorUsage(
        &self,
        output_stream_id: u32,
        input_stream_id: *mut u32,
        usage: *mut MFSampleAllocatorUsage,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        if output_stream_id != 0 {
            return Err(WinError::from_hresult(MF_E_NOT_FOUND));
        }
        if input_stream_id.is_null() || usage.is_null() {
            return Err(WinError::from_hresult(E_POINTER));
        }
        unsafe {
            *input_stream_id = output_stream_id;
            *usage = MFSampleAllocatorUsage_UsesCustomAllocator;
        }
        Ok(())
    }
}

impl IMFMediaSourceEx_Impl for VirtualMediaSource_Impl {
    fn GetSourceAttributes(&self) -> WinResult<IMFAttributes> {
        check_active(&self.state)?;
        Ok(self.state.source_attributes.clone())
    }

    fn GetStreamAttributes(&self, stream_id: u32) -> WinResult<IMFAttributes> {
        check_active(&self.state)?;
        if stream_id != 0 {
            return Err(WinError::from_hresult(MF_E_NOT_FOUND));
        }
        Ok(self.state.stream_attributes.clone())
    }

    fn SetD3DManager(&self, _manager: Ref<'_, IUnknown>) -> WinResult<()> {
        Err(WinError::from_hresult(E_NOTIMPL))
    }
}

#[implement(IMFMediaStream2)]
struct VirtualMediaStream {
    state: Arc<MediaSourceState>,
}

impl IMFMediaEventGenerator_Impl for VirtualMediaStream_Impl {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> WinResult<IMFMediaEvent> {
        check_active(&self.state)?;
        unsafe { self.state.stream_queue.GetEvent(flags.0 as u32) }
    }

    fn BeginGetEvent(
        &self,
        callback: Ref<'_, IMFAsyncCallback>,
        callback_state: Ref<'_, IUnknown>,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        unsafe {
            self.state
                .stream_queue
                .BeginGetEvent(callback.as_ref(), callback_state.as_ref())
        }
    }

    fn EndGetEvent(&self, result: Ref<'_, IMFAsyncResult>) -> WinResult<IMFMediaEvent> {
        check_active(&self.state)?;
        unsafe { self.state.stream_queue.EndGetEvent(result.as_ref()) }
    }

    fn QueueEvent(
        &self,
        event_type: u32,
        extended_type: *const GUID,
        status: HRESULT,
        value: *const PROPVARIANT,
    ) -> WinResult<()> {
        check_active(&self.state)?;
        unsafe {
            self.state
                .stream_queue
                .QueueEventParamVar(event_type, extended_type, status, value)
        }
    }
}

impl IMFMediaStream_Impl for VirtualMediaStream_Impl {
    fn GetMediaSource(&self) -> WinResult<IMFMediaSource> {
        check_active(&self.state)?;
        self.state
            .source
            .lock()
            .map_err(|_| unexpected())?
            .clone()
            .ok_or_else(unexpected)
    }

    fn GetStreamDescriptor(&self) -> WinResult<IMFStreamDescriptor> {
        check_active(&self.state)?;
        Ok(self.state.stream_descriptor.clone())
    }

    fn RequestSample(&self, token: Ref<'_, IUnknown>) -> WinResult<()> {
        check_active(&self.state)?;
        if *self.state.stream_state.lock().map_err(|_| unexpected())? != MF_STREAM_STATE_RUNNING {
            return Err(WinError::from_hresult(MF_E_INVALIDREQUEST));
        }

        let subtype = *self
            .state
            .current_subtype
            .lock()
            .map_err(|_| unexpected())?;
        let frame = self.state.frames.frame_for(&subtype);
        unsafe {
            let buffer = MFCreateMemoryBuffer(frame.len() as u32)?;
            let mut target = std::ptr::null_mut();
            let mut max_len = 0;
            buffer.Lock(&mut target, Some(&mut max_len), None)?;
            if target.is_null() || max_len < frame.len() as u32 {
                let _ = buffer.Unlock();
                return Err(WinError::from_hresult(E_UNEXPECTED));
            }
            std::ptr::copy_nonoverlapping(frame.as_ptr(), target, frame.len());
            buffer.Unlock()?;
            buffer.SetCurrentLength(frame.len() as u32)?;

            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(MFGetSystemTime())?;
            sample.SetSampleDuration(FRAME_DURATION_100NS)?;
            if let Some(token) = token.cloned() {
                sample.SetUnknown(&MFSampleExtension_Token, &token)?;
            }
            let sample_unknown: IUnknown = sample.cast()?;
            self.state.stream_queue.QueueEventParamUnk(
                MEMediaSample.0 as u32,
                &GUID::zeroed(),
                S_OK,
                &sample_unknown,
            )
        }
    }
}

impl IMFMediaStream2_Impl for VirtualMediaStream_Impl {
    fn SetStreamState(&self, state: MF_STREAM_STATE) -> WinResult<()> {
        check_active(&self.state)?;
        let mut current = self.state.stream_state.lock().map_err(|_| unexpected())?;
        if *current == state {
            return Ok(());
        }
        match state {
            MF_STREAM_STATE_PAUSED if *current != MF_STREAM_STATE_RUNNING => {
                return Err(WinError::from_hresult(MF_E_INVALID_STATE_TRANSITION));
            }
            MF_STREAM_STATE_PAUSED | MF_STREAM_STATE_RUNNING | MF_STREAM_STATE_STOPPED => {
                *current = state;
            }
            _ => return Err(WinError::from_hresult(MF_E_INVALID_STATE_TRANSITION)),
        }
        Ok(())
    }

    fn GetStreamState(&self) -> WinResult<MF_STREAM_STATE> {
        check_active(&self.state)?;
        Ok(*self.state.stream_state.lock().map_err(|_| unexpected())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_conversions_have_expected_sizes() {
        let rgba = vec![127u8; (FRAME_WIDTH * FRAME_HEIGHT * 4) as usize];
        let (bgra, nv12) = rgba_to_video_frames(&rgba);
        assert_eq!(bgra.len(), (FRAME_WIDTH * FRAME_HEIGHT * 4) as usize);
        assert_eq!(nv12.len(), (FRAME_WIDTH * FRAME_HEIGHT * 3 / 2) as usize);
    }

    #[test]
    fn yuv_conversion_stays_in_video_range() {
        assert_eq!(rgb_to_y(0, 0, 0), 16);
        assert!(rgb_to_y(255, 255, 255) >= 235);
        assert_eq!(rgb_to_u(0, 0, 0), 128);
        assert_eq!(rgb_to_v(0, 0, 0), 128);
    }

    #[test]
    fn frame_bridge_transfers_nv12_frame() {
        let frames = FrameStore::new();
        let expected = vec![93u8; FRAME_BRIDGE_MAX_PAYLOAD];
        frames.update_nv12(expected.clone());
        let stop = Arc::new(AtomicBool::new(false));
        let server_stop = stop.clone();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            serve_frame_bridge_client(&frames, &server_stop, stream).ok();
        });

        let mut client = TcpStream::connect(address).unwrap();
        let mut header = [0u8; FRAME_BRIDGE_HEADER_LEN];
        client.read_exact(&mut header).unwrap();
        assert_eq!(&header[..4], FRAME_BRIDGE_MAGIC);
        assert_eq!(
            u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize,
            expected.len()
        );
        let mut received = vec![0u8; expected.len()];
        client.read_exact(&mut received).unwrap();
        assert_eq!(received, expected);
        stop.store(true, Ordering::Release);
        server.join().unwrap();
    }

    #[test]
    fn frame_bridge_serves_multiple_camera_instances() {
        let frames = FrameStore::new();
        let expected = vec![61u8; FRAME_BRIDGE_MAX_PAYLOAD];
        frames.update_nv12(expected.clone());
        let stop = Arc::new(AtomicBool::new(false));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_frames = frames.clone();
        let server_stop = stop.clone();
        let server = std::thread::spawn(move || {
            let mut workers = Vec::new();
            for _ in 0..2 {
                let (stream, _) = listener.accept().unwrap();
                let frames = server_frames.clone();
                let stop = server_stop.clone();
                workers.push(std::thread::spawn(move || {
                    serve_frame_bridge_client(&frames, &stop, stream).ok();
                }));
            }
            workers
        });

        let mut first = TcpStream::connect(address).unwrap();
        let mut second = TcpStream::connect(address).unwrap();
        for client in [&mut first, &mut second] {
            let mut header = [0u8; FRAME_BRIDGE_HEADER_LEN];
            client.read_exact(&mut header).unwrap();
            let mut received = vec![0u8; expected.len()];
            client.read_exact(&mut received).unwrap();
            assert_eq!(received, expected);
        }

        stop.store(true, Ordering::Release);
        for worker in server.join().unwrap() {
            worker.join().unwrap();
        }
    }

    #[test]
    fn media_foundation_reader_receives_first_frame() {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
            MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
        }

        let result = (|| -> WinResult<()> {
            let source = create_media_source(FrameStore::new(), None)?;
            let reader = unsafe { MFCreateSourceReaderFromMediaSource(&source, None)? };
            let media_type = unsafe { create_video_type(MFVideoFormat_NV12)? };
            unsafe {
                reader.SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, true)?;
                reader.SetCurrentMediaType(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    None,
                    &media_type,
                )?;
            }

            let mut actual_stream = 0;
            let mut flags = 0;
            let mut timestamp = 0;
            let mut sample = None;
            unsafe {
                reader.ReadSample(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    0,
                    Some(&mut actual_stream),
                    Some(&mut flags),
                    Some(&mut timestamp),
                    Some(&mut sample),
                )?;
            }
            let sample = sample.ok_or_else(unexpected)?;
            let buffer = unsafe { sample.ConvertToContiguousBuffer()? };
            assert_eq!(
                unsafe { buffer.GetCurrentLength()? } as usize,
                FRAME_BRIDGE_MAX_PAYLOAD
            );
            unsafe {
                source.Shutdown()?;
            }
            Ok(())
        })();

        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
        result.unwrap();
    }

    unsafe fn activate_registered_virtual_camera(
        camera: &IMFVirtualCamera,
    ) -> WinResult<IMFMediaSource> {
        let link_length = unsafe {
            camera.GetStringLength(&MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK)?
        };
        let mut link = vec![0u16; link_length as usize + 1];
        unsafe {
            camera.GetString(
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                &mut link,
                None,
            )?;
        }

        let attributes = create_attributes(2)?;
        unsafe {
            attributes.SetGUID(
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
            )?;
            attributes.SetString(
                &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK,
                PCWSTR(link.as_ptr()),
            )?;
            MFCreateDeviceSource(&attributes)
        }
    }

    #[test]
    #[ignore = "opens the system-registered Insta360Linker virtual camera and reads one frame"]
    fn windows_registered_camera_returns_first_frame() {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
            MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
        }
        let frames = FrameStore::new();
        frames.set_enabled(true);
        let stop = Arc::new(AtomicBool::new(false));
        let bridge = spawn_frame_bridge_server(frames, stop.clone()).unwrap();

        let mut camera = None;
        let result = (|| -> anyhow::Result<()> {
            start_system_virtual_camera(&mut camera)
                .map_err(|error| anyhow::anyhow!("start virtual camera: {error}"))?;
            let camera_ref = camera
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("camera missing"))?;
            let source = unsafe { activate_registered_virtual_camera(camera_ref) }
                .map_err(|error| anyhow::anyhow!("activate registered source: {error}"))?;
            let reader = unsafe { MFCreateSourceReaderFromMediaSource(&source, None) }
                .map_err(|error| anyhow::anyhow!("create source reader: {error}"))?;
            unsafe {
                reader.SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, true)?;
                reader.SetCurrentMediaType(
                    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                    None,
                    &create_video_type(MFVideoFormat_NV12)?,
                )?;
            }

            let mut sample = None;
            let mut last_stream = 0;
            let mut last_flags = 0;
            let mut last_timestamp = 0;
            for _ in 0..8 {
                unsafe {
                    reader
                        .ReadSample(
                            MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                            0,
                            Some(&mut last_stream),
                            Some(&mut last_flags),
                            Some(&mut last_timestamp),
                            Some(&mut sample),
                        )
                        .map_err(|error| anyhow::anyhow!("read first sample: {error}"))?;
                }
                if sample.is_some() {
                    break;
                }
            }
            let sample = sample.ok_or_else(|| {
                anyhow::anyhow!(
                    "no sample returned; stream={last_stream}, flags=0x{last_flags:08X}, timestamp={last_timestamp}"
                )
            })?;
            let buffer = unsafe { sample.ConvertToContiguousBuffer()? };
            anyhow::ensure!(
                unsafe { buffer.GetCurrentLength()? } as usize == FRAME_BRIDGE_MAX_PAYLOAD,
                "unexpected frame size"
            );
            Ok(())
        })();

        stop_system_virtual_camera(&mut camera);
        stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(FRAME_BRIDGE_ADDRESS);
        bridge.join().unwrap();
        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
        result.unwrap();
    }

    #[test]
    #[ignore = "registers a temporary Windows virtual camera"]
    fn windows_registration_smoke_test() {
        let frames = FrameStore::new();
        let controller = VirtualCameraController::new(frames).unwrap();
        controller.start().unwrap();
        assert!(controller.is_started());
        assert!(controller.stop().is_ok());
    }
}
