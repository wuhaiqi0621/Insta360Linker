# Luna Ultra 相机控制台

> 历史逆向记录：其中部分协议布局和“已支持”项目已经过时。当前实现请以 `README.md`、`reverse_apk/pcap_analysis/20260722_192838/CONTROL_FINDINGS.md` 和 `reverse_apk/REVERSE_CONTINUE_LOG.md` 为准。

## 📋 项目概述

本项目是一个 Windows 桌面应用程序，用于控制 Insta360 Luna Ultra 相机。通过分析 Insta360 官方 APK（v2.28.0），实现了与相机的原生通信协议。

### 核心特性

- ✅ **原生协议** - 完全基于APK逆向分析
- ✅ **端口 6666** - 直接连接相机控制端口
- ✅ **4字节长度前缀** - 使用小端序长度前缀
- ✅ **拍摄模式切换** - 支持多种录像模式
- ✅ **实时参数调整** - 曝光、白平衡、ISO 等

---

## 🔧 协议说明

### 通信方式

APK 使用 **TCP + 自定义协议** 与 Luna Ultra 相机通信：

- **端口**: 6666
- **消息格式**: 4字节小端长度前缀 + 消息内容
- **握手流程**: ShakeHandInfo → CheckAuthorization

### 消息格式

```
+----------------+----------------+
| 4字节长度前缀  | 消息内容        |
| (小端序)       | (UTF-8/二进制)  |
+----------------+----------------+
```

### 握手流程

```
1. 发送: ShakeHandInfo
2. 接收: ShakeHandInfoResp
3. 发送: CheckAuthorization
4. 接收: CheckAuthorizationResp
5. 建立会话
```

### 支持的消息类型

| 消息 | 说明 |
|------|------|
| `ShakeHandInfo` | 握手信息 |
| `CheckAuthorization` | 授权检查 |
| `SetOptions` | 设置选项 |
| `GetOptions` | 获取选项 |
| `SetSyncCaptureMode` | 设置拍摄模式 |
| `StartCapture` | 开始拍摄 |
| `StopCapture` | 停止拍摄 |
| `TakePicture` | 拍照 |

---

## 🎥 支持的拍摄模式

| 模式 | 说明 | APK 配置键 |
|------|------|-----------|
| `VIDEO_NORMAL` | 普通录像 | camera_enable_highlight_capture_mode_list |
| `VIDEO_PURE` | 纯净录像 | 无防抖，后期处理 |
| `VIDEO_SLOW_MOTION` | 慢动作 | 高帧率拍摄 |
| `VIDEO_TIMESHIFT` | 移动延时 | 运动延时摄影 |
| `VIDEO_TIMELAPSE` | 延时摄影 | 固定延时摄影 |

---

## 🎛️ 可调参数

### 曝光控制
- **曝光补偿 (EV)**: -4.0 ~ +4.0
- **ISO**: 100 / 200 / 400 / 800 / 1600 / 3200 / 6400 / 自动
- **快门速度**: 1/8000 ~ 30秒

### 白平衡
- 自动 / 日光 / 阴天 / 白炽灯 / 荧光灯 / 阴影

### 视频设置
- **分辨率**: 8K / 5.7K / 4K / 2.7K / 1080p / 720p
- **帧率**: 24 / 25 / 30 / 50 / 60 / 100 / 120 / 200 / 240 fps

---

## 🏗️ 技术架构

### 项目结构

```
luna_control_apk_only_ui/
├── Cargo.toml
├── src/
│   ├── adapters/
│   │   ├── mod.rs
│   │   ├── luna_client.rs    # 原生协议客户端 ⭐
│   │   ├── luna_local.rs     # 文件会话管理
│   │   ├── mic_ble.rs        # BLE 蓝牙控制
│   │   └── watermark.rs      # 水印处理
│   └── bin/
│       └── html_app.rs       # WebView2 主程序
├── web/
│   └── index.html            # HTML UI
└── assets/
    └── apk_watermark/        # APK 提取的水印资源
```

### 技术栈

| 组件 | 技术 | 用途 |
|------|------|------|
| 窗口管理 | tao + wry | WebView2 容器 |
| UI | HTML/CSS/JS | 响应式界面 |
| 通信 | TCP + 自定义协议 | 相机控制 |
| 蓝牙 | bbleplug | Mic Pro BLE |

---

## 🚀 快速开始

### 环境要求

- Windows 10/11
- Rust 1.70+
- Luna Ultra 相机 Wi-Fi 热点

### 编译运行

```powershell
cd F:\luna_control_apk_only_ui
cargo build --release --bin html_app
cargo run --release --bin html_app
```

### 使用流程

1. **连接相机 Wi-Fi**
   - 在 Windows Wi-Fi 设置中连接 Luna Ultra 热点
   - 默认网关：192.168.42.1

2. **启动应用**
   - 运行 `html_app.exe`

3. **控制相机**
   - 选择拍摄模式
   - 调整参数
   - 点击拍照/录像按钮

---

## 📝 从 APK 提取的关键信息

### 设备配置 (z03)

```json
{
  "project_name": "Z03",
  "device_type": "Insta360 Luna Ultra",
  "ucd2": true,
  "connection_channels": ["wifi", "usb", "bluetooth"]
}
```

### 支持的录像模式

```json
{
  "camera_enable_highlight_capture_mode_list": [
    "VIDEO_NORMAL",
    "VIDEO_PURE",
    "VIDEO_SLOW_MOTION",
    "VIDEO_TIMESHIFT",
    "VIDEO_TIMELAPSE"
  ]
}
```

### 握手消息类型

```
type.googleapis.com/insta360.messages.ShakeHandInfo
type.googleapis.com/insta360.messages.ShakeHandInfoResp
type.googleapis.com/insta360.messages.CheckAuthorization
type.googleapis.com/insta360.messages.CheckAuthorizationResp
```

---

## ⚠️ 注意事项

1. **网络连接**：必须连接 Luna Ultra 的 Wi-Fi 热点
2. **端口**：控制端口 6666，HTTP 端口 80
3. **协议**：使用4字节长度前缀，不是OSC
4. **编码**：源代码使用 UTF-8 + CRLF 编码
5. **参考**：完全基于APK逆向，不参考任何公开OSC协议

---

## 📚 参考资料

- Insta360 APK v2.28.0 (418696)
- APK 逆向分析
- Netty 框架实现

---

## 📄 许可

本项目仅供学习和研究使用，请勿用于商业用途。

---

**项目路径**: `F:\luna_control_apk_only_ui`

**最后更新**: 2026-07-04
