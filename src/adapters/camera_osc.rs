use anyhow::{Context, anyhow};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone)]
pub struct OscClient {
    base_url: String,
    client: Client,
}

/// 相机设置选项，从 APK 中提取的 Luna Ultra 支持的参数
#[derive(Debug, Clone, Default)]
pub struct CameraSettings {
    pub capture_mode: Option<String>,       // "image" 或 "video"
    pub video_type: Option<String>, // VIDEO_NORMAL, VIDEO_PURE, VIDEO_SLOW_MOTION, VIDEO_TIMESHIFT, VIDEO_TIMELAPSE
    pub exposure_compensation: Option<f64>, // 曝光补偿 -4.0 ~ +4.0
    pub white_balance: Option<String>, // auto, daylight, cloudy, incandescent, fluorescent, etc.
    pub iso: Option<u32>,           // ISO 值
    pub exposure_time: Option<String>, // 快门速度
    pub resolution: Option<String>, // 视频分辨率
    pub frame_rate: Option<u32>,    // 帧率
    pub bitrate: Option<u32>,       // 码率
    pub quality: Option<String>,    // 画质
    pub timer: Option<u32>,         // 自拍倒计时(秒)
    pub interval: Option<f64>,      // 间隔拍摄(秒)
    pub burst: Option<u32>,         // 连拍张数
    pub zoom: Option<f64>,          // 变焦倍数
}

impl OscClient {
    pub fn new(host: &str, port: u16) -> anyhow::Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(12)).build()?;
        Ok(Self {
            base_url: format!("http://{}:{}", host.trim(), port),
            client,
        })
    }

    /// 获取相机信息
    pub fn info(&self) -> anyhow::Result<Value> {
        self.client
            .get(format!("{}/osc/info", self.base_url))
            .header("X-XSRF-Protected", "1")
            .send()?
            .error_for_status()?
            .json()
            .context("failed to decode /osc/info")
    }

    /// 获取相机状态
    pub fn state(&self) -> anyhow::Result<Value> {
        self.post("/osc/state", json!({}))
    }

    /// 执行 OSC 命令
    pub fn execute(&self, name: &str, parameters: Option<Value>) -> anyhow::Result<Value> {
        let mut payload = json!({ "name": name });
        if let Some(params) = parameters {
            payload["parameters"] = params;
        }
        self.post("/osc/commands/execute", payload)
    }

    /// 设置相机选项
    pub fn set_options(&self, options: Value) -> anyhow::Result<Value> {
        self.execute("camera.setOptions", Some(json!({ "options": options })))
    }

    /// 获取相机选项
    pub fn get_options(&self, option_names: &[&str]) -> anyhow::Result<Value> {
        self.execute(
            "camera.getOptions",
            Some(json!({ "optionNames": option_names })),
        )
    }

    /// 拍照
    pub fn take_picture(&self) -> anyhow::Result<Value> {
        self.execute("camera.takePicture", None)
    }

    /// 开始录像
    pub fn start_capture(&self) -> anyhow::Result<Value> {
        self.execute("camera.startCapture", None)
    }

    /// 停止录像
    pub fn stop_capture(&self) -> anyhow::Result<Value> {
        self.execute("camera.stopCapture", None)
    }

    /// 切换到照片模式
    pub fn switch_to_photo_mode(&self) -> anyhow::Result<Value> {
        self.set_options(json!({"captureMode": "image"}))
    }

    /// 切换到视频模式
    pub fn switch_to_video_mode(&self) -> anyhow::Result<Value> {
        self.set_options(json!({"captureMode": "video"}))
    }

    /// 设置视频类型
    pub fn set_video_type(&self, video_type: &str) -> anyhow::Result<Value> {
        self.set_options(json!({"_videoType": video_type}))
    }

    /// 设置曝光补偿
    pub fn set_exposure_compensation(&self, ev: f64) -> anyhow::Result<Value> {
        self.set_options(json!({"exposureCompensation": ev}))
    }

    /// 设置白平衡
    pub fn set_white_balance(&self, wb: &str) -> anyhow::Result<Value> {
        self.set_options(json!({"whiteBalance": wb}))
    }

    /// 设置 ISO
    pub fn set_iso(&self, iso: u32) -> anyhow::Result<Value> {
        self.set_options(json!({"iso": iso}))
    }

    /// 设置快门速度
    pub fn set_exposure_time(&self, time: &str) -> anyhow::Result<Value> {
        self.set_options(json!({"exposureTime": time}))
    }

    /// 设置视频分辨率
    pub fn set_resolution(&self, resolution: &str) -> anyhow::Result<Value> {
        self.set_options(json!({"resolution": resolution}))
    }

    /// 设置帧率
    pub fn set_frame_rate(&self, fps: u32) -> anyhow::Result<Value> {
        self.set_options(json!({"frameRate": fps}))
    }

    /// 设置自拍倒计时
    pub fn set_timer(&self, seconds: u32) -> anyhow::Result<Value> {
        self.set_options(json!({"timer": seconds}))
    }

    /// 设置间隔拍摄
    pub fn set_interval(&self, seconds: f64) -> anyhow::Result<Value> {
        self.set_options(json!({"interval": seconds}))
    }

    /// 设置变焦
    pub fn set_zoom(&self, zoom: f64) -> anyhow::Result<Value> {
        self.set_options(json!({"zoom": zoom}))
    }

    /// 应用多个设置
    pub fn apply_settings(&self, settings: &CameraSettings) -> anyhow::Result<Value> {
        let mut options = json!({});

        if let Some(ref mode) = settings.capture_mode {
            options["captureMode"] = json!(mode);
        }
        if let Some(ref vt) = settings.video_type {
            options["_videoType"] = json!(vt);
        }
        if let Some(ev) = settings.exposure_compensation {
            options["exposureCompensation"] = json!(ev);
        }
        if let Some(ref wb) = settings.white_balance {
            options["whiteBalance"] = json!(wb);
        }
        if let Some(iso) = settings.iso {
            options["iso"] = json!(iso);
        }
        if let Some(ref et) = settings.exposure_time {
            options["exposureTime"] = json!(et);
        }
        if let Some(ref res) = settings.resolution {
            options["resolution"] = json!(res);
        }
        if let Some(fps) = settings.frame_rate {
            options["frameRate"] = json!(fps);
        }
        if let Some(br) = settings.bitrate {
            options["bitrate"] = json!(br);
        }
        if let Some(ref q) = settings.quality {
            options["quality"] = json!(q);
        }
        if let Some(t) = settings.timer {
            options["timer"] = json!(t);
        }
        if let Some(i) = settings.interval {
            options["interval"] = json!(i);
        }
        if let Some(b) = settings.burst {
            options["burst"] = json!(b);
        }
        if let Some(z) = settings.zoom {
            options["zoom"] = json!(z);
        }

        self.set_options(options)
    }

    /// 获取常用相机状态
    pub fn get_camera_status(&self) -> anyhow::Result<Value> {
        self.get_options(&[
            "captureMode",
            "_videoType",
            "remainingSpace",
            "_batteryCapacity",
            "exposureCompensation",
            "whiteBalance",
            "iso",
            "exposureTime",
            "resolution",
            "frameRate",
            "timer",
            "interval",
            "zoom",
        ])
    }

    /// 陀螺仪校准
    pub fn calibrate_gyro(&self) -> anyhow::Result<Value> {
        self.execute("camera.calibrateGyro", None)
    }

    /// 格式化存储
    pub fn format_storage(&self) -> anyhow::Result<Value> {
        self.execute("camera.formatStorage", None)
    }

    /// 获取存储信息
    pub fn get_storage_info(&self) -> anyhow::Result<Value> {
        self.get_options(&["remainingSpace", "storageLocation"])
    }

    /// 获取电池信息
    pub fn get_battery_info(&self) -> anyhow::Result<Value> {
        self.get_options(&["_batteryCapacity", "batteryLevel"])
    }

    fn post(&self, path: &str, payload: Value) -> anyhow::Result<Value> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .header("X-XSRF-Protected", "1")
            .json(&payload)
            .send()
            .with_context(|| format!("request failed: {path}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("OSC request failed with HTTP {status}"));
        }
        response.json().context("failed to decode OSC response")
    }
}
