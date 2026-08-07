use std::sync::Arc;

pub struct FrameStore;

impl FrameStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn update_jpeg(&self, _jpeg: &[u8]) {}
}

pub struct VirtualCameraController;

impl VirtualCameraController {
    pub fn new(_frames: Arc<FrameStore>) -> anyhow::Result<Arc<Self>> {
        anyhow::bail!("macOS 版本暂不支持虚拟摄像机")
    }

    pub fn start(&self) -> anyhow::Result<String> {
        anyhow::bail!("macOS 版本暂不支持虚拟摄像机")
    }

    pub fn stop(&self) -> anyhow::Result<String> {
        Ok("虚拟摄像机未运行".to_string())
    }

    pub fn is_started(&self) -> bool {
        false
    }
}

pub fn handle_installer_mode() -> Option<i32> {
    None
}
