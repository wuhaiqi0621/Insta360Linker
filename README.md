# Luna Studio

Rust 原生桌面应用。Windows 使用 WebView2，macOS 使用系统 WebKit，当前适配 Insta360 Luna Ultra 与 Mic Pro。

## 当前版本

日用版入口：

```text
F:/Insta360onWin/run_release.bat
```

当前新版本程序：

```text
F:/Insta360onWin/LunaStudio.exe
```

项目根目录只保留当前日用版本；`run_release.bat` 会直接启动它。

窗口使用 Windows 11 原生 Mica 系统背景。HTML 界面采用透明 WebView2、半透明表面和系统明暗主题，并避免在大量相册卡片上使用逐项模糊。

## Luna Ultra 功能

- 连接 Luna Ultra 的 Wi-Fi 控制会话。
- 同时读取内部存储与 SD 卡，或单独切换其中一个存储。
- 相册分批显示素材，并按可见区域加载缓存缩略图。
- 视频卡片按可见区域生成并缓存静态首帧，优先使用对应的低码率 LRV 文件。
- 只有点击视频卡片后才加载播放器，浏览相册时不会批量建立视频连接。
- 点击素材进入大图或视频预览。
- 右键素材进行选择、下载或多选永久删除。
- 浏览、筛选并批量下载相机媒体。
- 桌面端只在最终点击“下载所选”或水印“导出”时显示保存位置，不需要提前选择导出目录。
- Android 端不显示保存位置选择，照片和视频通过系统媒体库直接保存到相册的 `DCIM/Insta360Linker`。
- 独立的“拍摄控制”页面，不再把相册和拍摄操作挤在同一页。
- 先切换到拍照模式才能拍照，先切换到录像模式才能开始录像。
- 开始录像与停止录像，后端同样执行模式门禁，不能绕过界面误操作。
- 显示录像计时和停止录像后返回的媒体路径。
- 开启与关闭实时画面。
- 使用方向键或二维触控盘连续控制相机云台，松开后自动停止。
- 使用 APK 提取的多套官方水印资源导出照片或视频。

拍照、录像和实时预览命令来自 APK/官方应用 PCAP 证据，并按请求 ID 与相机响应完成配对。应用不使用 OSC `/osc/info` 接口。

## 实时预览

```text
UCD2 0x0001 启动预览
  -> UCD2 type 01 复用流
  -> subtype 0x20 Annex-B HEVC/H.265
  -> FFmpeg 低延迟解码
  -> JPEG 帧
  -> WebView2 Canvas
UCD2 0x0002 停止预览
```

解码组件位于 `assets/ffmpeg/ffmpeg.exe`。运行程序时请保留 `assets` 目录与 EXE 的相对位置。

## 使用

1. 在 Windows Wi-Fi 设置中连接 Luna Ultra 热点。
2. 运行 `run_release.bat`。
3. 打开“拍摄控制”，点击“连接相机”。
4. 等待“控制已就绪”，选择“拍照”或“录像”模式。
5. 使用圆形快门按钮拍照，或开始、停止录像；云台控制位于页面下方。
6. 打开“相机媒体”，在工具栏选择“全部存储”“内部”或“SD 卡”。
7. 拍照或停止录像后，相册会自动延迟刷新。

默认相机地址为 `192.168.42.1`，控制端口为 `6666`，媒体使用 HTTP 端口 `80`。

## 构建

```powershell
cd F:\Insta360onWin
cargo build --release --bin html_app --target-dir target_daily
```

构建产物为 `target_daily/release/html_app.exe`。

### Android

在 GitHub 仓库的 Actions 页面手动运行 `Build Android ARM64 APK`，流程会使用 Java 17、Android API/Build Tools 36、NDK 28.2.13676358 和 Rust `aarch64-linux-android` 目标构建并验证 Debug APK。完成后从运行页面下载 `Insta360Linker-android-arm64-debug` artifact；其中包含 APK 和 `SHA256SUMS.txt`。

本地 Windows 构建方法见 [`android/README.md`](android/README.md)。

### macOS

需要 Rust 工具链和 Xcode Command Line Tools。在 Apple Silicon 或 Intel Mac 上运行：

```bash
./build_macos.sh
```

应用产物为 `dist/Luna Studio.app`。构建脚本会按当前 Mac 架构下载 FFmpeg，并将 FFmpeg、官方水印资源以及 Xcode 27 编译的原生 Liquid Glass 图标一起打包进应用。图标的 `Assets.car` 保留 Icon Composer 分层、玻璃高光以及浅色/深色/着色外观，`.icns` 用作旧版系统兼容回退。FFmpeg 用于实时监看、视频缩略图和视频水印；官方 PNG 资源用于照片与视频水印。macOS 版本禁用 Windows Media Foundation 虚拟摄像机功能，但保留应用内 HEVC 实时监看。

连接 Luna Ultra 前，请先让 Mac 加入相机热点。macOS 版会把相机的 TCP、媒体下载和缩略图请求绑定到与 `192.168.42.1` 同网段的物理网卡，避免 VPN/代理的 `utun` 路由误接管相机地址；找不到正确网卡时，界面会直接提示检查相机 Wi-Fi 和 VPN/代理。

## 关键结构

```text
src/adapters/luna_local.rs  UCD2 会话、控制命令、响应配对与 HEVC 拆流
src/bin/html_app.rs         WebView2 主程序、IPC 与 FFmpeg 实时解码
web/index.html              中文日用界面
assets/apk_watermark/       APK 水印资源
assets/ffmpeg/ffmpeg.exe    实时画面解码器
reverse_apk/                APK/PCAP 证据、分析工具与交接记录
```

## 协议概要

UCD2 帧头为 12 字节：

```text
55 43 44 32 | 01 | 0c | type | sequence | payload_len_le_u32
```

帧末尾附带 4 字节校验值。`sequence` 和内部 `request_id` 都是动态字段。类型 `04` 承载控制请求/响应，类型 `01` 承载实时数据，类型 `05` 用于会话控制/心跳。

```text
0x0001  开启实时预览
0x0002  关闭实时预览
0x0003  拍照
0x0004  开始录像
0x0005  停止录像
0x0007  切换拍照/录像模式
0x00E2  云台移动/释放
```

完整证据见：

```text
reverse_apk/pcap_analysis/20260722_192838/CONTROL_FINDINGS.md
reverse_apk/CAPTURE_CONTROL_CONTINUE.md
reverse_apk/REVERSE_CONTINUE_LOG.md
```
