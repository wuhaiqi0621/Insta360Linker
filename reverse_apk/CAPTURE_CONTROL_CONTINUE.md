# Luna Studio 拍摄控制与云台交接

日期：2026-07-23

## 本轮目标

- 将拍摄控制从相册中拆成独立页面。
- 录像前必须先切换到录像模式，拍照前必须先切换到拍照模式。
- 将用户 PCAP 中已有的 Luna Ultra 云台移动能力做成日用控件。
- 保留完整客户端 Mica，不恢复旧调试界面。
- 只使用本地 APK、反编译产物和用户 PCAP 证据确认相机协议。

## 证据边界

本轮相机协议实现没有引用公开协议说明或第三方项目。证据来自：

- `Insta360_v2.28.0_418696.apk` 的本地反编译产物。
- `reverse_apk/pcap_analysis/20260722_192838` 的用户 PCAP 分析。
- 当前仓库已经恢复的 UCD2 帧、校验和、内部请求 ID 和响应配对实现。

界面布局与 Rust 工程结构沿用当前 Luna Studio，不改变协议证据来源。

## 协议结论

### 拍摄模式

- 内部命令号：`0x0007`。
- 内部 method：`0x02`。
- 模式字段：外层 field 1 为选项 `0x28`，field 2 承载模式值。
- APK 枚举字符串包含 `SYNC_MODE_NORMAL_IMAGE` 和 `SYNC_MODE_NORMAL_VIDEO`。
- 普通拍照枚举值为默认值 `0`。
- 普通录像枚举值为 `1`。

已实现的命令 body：

```text
普通拍照  08 28 12 00
普通录像  08 28 12 03 c0 02 01
```

用户 PCAP 中还观察到同一字段布局的值 `8`：

```text
08 28 12 03 c0 02 08
```

它用于证明字段编码方式，不被本轮日用界面暴露为未知模式。

### 拍照与录像门禁

- 拍照命令：`0x0003`，body `30 03`。
- 开始录像：`0x0004`，body `08 01`。
- 停止录像：`0x0005`，body `10 01`。
- 前端只有在模式切换成功后才激活对应快门行为。
- Rust 后端再次检查模式，因此绕过前端也不能在错误模式下拍照或开始录像。
- 录像中禁止切换模式；停止录像成功后才清除后端录像状态。
- 所有状态只在相机命令返回成功后更新。

### 云台

- 内部命令号：`0x00E2`。
- 内部 method：`0x02`。
- 移动 body：`08 01 12 <nested_len> <nested_axes>`。
- PCAP 中观察到 body `08 02`，但它不属于普通手势松手流程。唯一一帧 `08 02` 出现在最后一个零向量约 1.78 秒之后，并紧跟 `0x00EE` 命令，因此应视为控制器退出/销毁流程。
- 当前应用的普通云台操作绝不发送 `08 02`。
- 静止点 body：`08 01 12 00`。
- x/y 范围均为 `-100..100`。
- nested field 1 为 x，field 2 为 y。
- 两轴不是普通无符号坐标，而是 protobuf ZigZag 有符号整数。

从 PCAP 逐字节复现的样本：

```text
x=36,  y=0   -> 08 01 12 02 08 48
x=59,  y=0   -> 08 01 12 02 08 76
x=90,  y=10  -> 08 01 12 05 08 b4 01 10 14
x=98,  y=13  -> 08 01 12 05 08 c4 01 10 1a
x=-21, y=96  -> 08 01 12 05 08 29 10 c0 01
```

PCAP 中共有 282 个 `0x00E2` 请求。三个可分离的普通手势段分别是：

```text
165 帧 / 7162 ms，平均 43.7 ms，x/y 覆盖 -99..99
 37 帧 /  726 ms，平均 20.2 ms，x 0..85，y -55..0
 79 帧 / 2863 ms，平均 36.7 ms，x -80..0，y -82..0
```

三个手势都从 `(0,0)` 开始，经连续中间值移动，并在松手后平滑回到 `(0,0)`。请求到成功响应的中位延迟为 9 ms，90 分位为 27 ms，99 分位为 43 ms，最大 47 ms。由此确认坐标是连续速度/摇杆向量，零向量用于停止运动，而不是物理方位角。

当前前端按 25 ms 周期运行控制器，每周期最多改变 18 个单位；输入结束后平滑回到 `(0,0)`，连续保留 4 个零向量后停止定时器。IPC 使用单飞最新点队列：在途请求未返回时只覆盖保存最新坐标，不累积历史轨迹。这样既接近官方手势节奏，又不会因响应抖动形成过期命令队列。

### 2026-07-23 真机方向校正

旧版把 UI 坐标直接作为设备 x/y 发送，用户实测结果是：

```text
按上 -> 相机向左
按下 -> 相机向右
按左 -> 相机向下
```

由此确认 UI 轴和设备轴需要交换并旋转：

```text
device_x = -ui_vertical
device_y =  ui_horizontal
```

当前四个方向的映射为：

```text
UI 上    (0, -72) -> device ( 72,   0)
UI 下    (0,  72) -> device (-72,   0)
UI 左  (-72,   0) -> device (  0, -72)
UI 右  ( 72,   0) -> device (  0,  72)
```

### 2026-07-23 录像状态确认

开始录像命令的 `0x00C8` 响应只代表命令被接收，不能证明相机已经开始写文件。官方 PCAP 在成功开始录像后还返回：

```text
event 0x2010 body 08 01 10 00 38 00  -> 正在录像
event 0x2010 body 08 00 10 00 38 00  -> 已停止
```

停止过程还出现 `08 01 10 00 38 02`，其 field 1 仍为 `1`，随后才变为 `0`。当前后端会解析 `0x2010` 的 field 1，并在看到目标状态后才更新计时器和录像状态；三秒内没有看到目标状态则返回“相机没有进入预期的录像状态”，不再出现相机没拍但界面假装正在录像的情况。

### 2026-07-23 状态事件订阅修正

用户真机触发“相机没有进入预期的录像状态”后，重新检查成功录像前的完整 PCAP 时间线：

```text
19:29:51.192  phone -> camera  command 0x0011, body 08 01
19:29:51.362  camera -> phone  status 0x00C8, body 0a 05 08 00 10 e8 07
19:30:10.578  phone -> camera  command 0x0004, body 08 01
19:30:11.481  camera -> phone  status 0x00C8
19:30:11.605  camera -> phone  event 0x2010, body 08 01 10 00 38 00
```

`0x0011 / 08 01` 是官方应用进入控制流程后使用的相机状态事件订阅。旧实现只解析 `0x2010`，却没有先完成该订阅，因此可能出现录像命令被接受、应用却永远收不到状态确认的情况。

Step 161 曾在认证完成后直接发送 `0x0011 / 08 01`；真机证明仅有订阅仍不足以进入录像。该顺序已被下方“完整会话初始化修正”取代。录像开始/停止始终必须收到 `0x2010` 真正状态，不使用命令响应制造假成功。

模式切换现在按 APK/PCAP 证据执行两步：

```text
普通拍照  command 0x0007: 08 28 12 00
拍照上下文 command 0x000A: 08 63 10 06
普通录像  command 0x0007: 08 28 12 03 c0 02 01
录像上下文 command 0x000A: 08 63 10 07
```

分析过程中临时试过模式值 `8` 和从状态响应中误读出的 `100`；二者都没有产生录像文件，且值 `8` 在 PCAP 中对应上下文 `0x43`，不是普通录像。最终实现已恢复 APK 枚举值 `1`，这两个临时候选均已排除。

### 2026-07-23 完整会话初始化修正

用户真机确认仅增加 `0x0011` 订阅后仍不能进入录像。重新对齐官方应用连接后的第一组请求，发现旧实现认证成功后直接订阅事件，跳过了官方链路中的会话初始化、客户端登记、时间同步和状态读取。

当前 `CameraControlSession::open` 按以下顺序准备控制会话：

```text
认证/设备信息  command 0x0008，request id 0x80000001
会话初始化     command 0x000F，empty body
客户端登记     command 0x0027，UUID + role 2
时间同步       command 0x0007，epoch seconds + UTC+8 + Asia/Shanghai
状态读取       command 0x0008，body 08 0b 08 55 08 b4 01
事件订阅       command 0x0011，body 08 01
```

每一步都必须收到匹配 request ID 的 `0x00C8` 才报告“控制已就绪”。开始录像仍发送已确认的 `0x0004 / 08 01`，并继续等待真实 `0x2010 / field 1 = 1`；本次没有改回虚假乐观状态。

### 2026-07-23 硬件摇杆速度分档

用户说明速度设置会同步到相机硬件摇杆，因此“只缩放 PC 端 `0x00E2` 坐标”的初步判断无效，相关百分比滑杆已完整撤销。

PCAP 中定位到两次明确的设备参数写入：

```text
19:31:17.911  command 0x0009
body 08 55 12 05 aa 05 02 10 02 18 06
目标档位 2

19:31:20.984  command 0x0009
body 08 55 12 05 aa 05 02 10 01 18 06
目标档位 1
```

相机随后分别发送 `0x206A / 08 00 10 02` 和 `0x206A / 08 00 10 01`。第一次写入前还观察到 `0x206A / 08 00 10 03`，证明设备原档位为 `3`。当前日用界面使用三段按钮：

```text
1 = 慢
2 = 中
3 = 快（抓包初始档位）
```

设置通过 `camera_set_gimbal_speed` IPC 直接发送给相机。真机证明上一版根据当前拍摄模式改写上下文无效；现已严格固定为 PCAP 唯一证明的 `0x06`，三档 body 只有目标档位字节不同。软件摇杆仍发送原始归一化坐标，不用速度档位缩放坐标上限。

速度设置不再把 `0x00C8` 当作最终成功。worker 会解析相机的 `0x206A`：

```text
08 00 10 01  -> 慢
08 00 10 02  -> 中
08 00 10 03  -> 快
```

只有收到与目标档位相同的 `0x206A` 才更新界面选中状态；三秒内没有确认则报告相机没有应用该档位。连接后界面不再假定“快”已经生效，每次点击慢/中/快都会真实发包。

### 2026-07-26 录像准备链修正

用户真机证明完整会话初始化后仍不能录像。再次与官方成功录像前的时间线对齐，旧实现仍缺少拍摄页的预览准备：

```text
command 0x00BF  body 58 0a
command 0x00C6  empty body
command 0x0001  body 10 01 30 28 38 2c 40 01 48 28 50 22
```

当前点击录像快门时，如果预览尚未开启，前端会自动完成上述预览准备和启动；收到预览成功后才继续开始录像。用户不再需要先手动点击“开启预览”。

开始录像前后端还会执行：

```text
command 0x000A  body 08 63 10 07                  刷新录像上下文
command 0x0008  body 08 14 08 b0 01 08 b1 01      读取录像/存储准备状态
command 0x0004  body 08 01                         开始录像
event   0x2010 body 08 01 10 00 38 00              真正进入录像
```

Step 163 曾把 `0x0008 / option 0x28` 的响应误当作当前拍摄模式，并用它拒绝模式切换和录像。用户真机证明该判断错误；相关查询解析和严格比较已在 Step 164 全部移除。

### 2026-07-26 模式确认与相册回归修正

仅依据 APK 重建 DEX 和用户官方应用抓包重新确认：

```text
seg06.dex code item 0x427b74, type @1884
SYNC_MODE_UNKNOWN          = 0
SYNC_MODE_NORMAL_IMAGE     = 1
SYNC_MODE_NORMAL_VIDEO     = 2
SYNC_MODE_HDR_IMAGE        = 3
SYNC_MODE_HDR_VIDEO        = 4
SYNC_MODE_INTERVAL_IMAGE   = 5
SYNC_MODE_TIMELAPSE_VIDEO  = 6
SYNC_MODE_BURST_PHOTO      = 7
SYNC_MODE_BULLETTIME_VIDEO = 8
SYNC_MODE_TIMESHIFT_VIDEO  = 9
SYNC_MODE_AEB_NIGHT_PHOTO  = 10
```

应用枚举包含 `UNKNOWN`，UCD2 拍摄模式嵌套值不包含该占位，因此普通拍照为 wire `0`、普通录像为 wire `1`。普通录像 body 仍为：

```text
08 28 12 03 c0 02 01
```

官方抓包的模式切换不是“发模式包、等一秒响应、再发上下文包”，而是：

```text
0x0007 模式包
约 3 ms
0x000A 上下文包
随后 event 0x2053 确认当前模式
最后才陆续收到两个 0x00C8
```

当前 worker 会先登记一次新的 `0x2053` 等待，再连续写出模式包和上下文包。只有收到两个成功响应并看到 `0x2053 / field 1` 等于目标 wire 值，才更新应用中的拍照/录像模式。

相册回归来自 Step 163 把列相册、缩略图和下载全部改成完整 `CameraControlSession`。现在已恢复两级会话：

```text
相册列表 / 照片和视频查看 / 缩略图 / 下载
  -> 持久轻量 LunaAuthSession

拍摄控制 / 相机端删除
  -> 完整 CameraControlSession
```

升级到完整控制前会先关闭轻量会话；已有完整控制时，相册直接复用该认证状态，不会再创建第二个 `6666` 连接。相册页检测成功后只加载素材，不再自动连接拍摄控制。

### 2026-07-26 照片与视频预览代理

Step 164 只修复了会话冲突，但照片和视频弹窗仍让 WebView 直接请求：

```text
http://192.168.42.1/storage_internal/...
http://192.168.42.1/sdcard/...
```

该请求绕开了 Rust 已建立的媒体链路，并受到 WebView 请求头、页面来源、系统代理和相机 MIME 的共同影响。真机 UI 验证结果是照片、视频都无法正常打开。

当前应用改为从随机本机回环端口加载 HTML，并提供同源媒体代理：

```text
WebView -> http://127.0.0.1:<random>/media/<UTF-8 hex camera URL>
Rust    -> 验证相机路径 -> 保持 UCD2 认证 -> 直连相机 HTTP
```

视频请求会把浏览器的 `Range` 原样转发给相机，并流式返回 `206 Partial Content`、`Content-Range`、`Content-Length` 和 `Accept-Ranges`。不会为了预览先下载完整视频。

所有相机 HTTP 客户端现已：

```text
禁用配置的 HTTP 代理
Accept-Encoding: identity
保持现有 LunaAuthSession / CameraControlSession
```

照片原图失败时退回相册缩略图；视频先试关联 LRV，再自动尝试原视频。最终失败会在预览弹窗内显示中文错误，不再留空。

本机代理根页面已验证返回 HTTP 200。当前电脑无法完成真实媒体字节验证，因为 `192.168.42.1` 被 FlClash 以 `198.18.0.1` 作为源地址接管，WLAN 的 `192.168.42.50` 已是 `Deprecated`；需重新连接真实 LunaU Wi-Fi 后验证。

### 2026-07-26 媒体会话断开修正

用户重新连接真实 LunaU Wi-Fi 后，Windows 现场状态显示：

```text
应用唯一 6666 会话：CloseWait
本地地址：192.168.42.50
远端地址：192.168.42.1:6666
路由接口：WLAN
```

这说明相机已经主动关闭连接，但旧应用仍保存着会话对象。相册检测此前还会先执行一次未认证的 `6666` TCP connect，然后立即关闭，再建立正式认证会话。该短探测现已删除；相册检测只检查 HTTP，正式认证会话成为唯一的控制端口连接。

官方 PCAP 中手机端 type-05 空帧时间线：

```text
19:28:46.698  05 0f
19:28:48.192  05 17
19:28:49.692  05 18
19:28:51.193  05 19
19:28:52.694  05 1c
```

除少量操作占用外，间隔稳定约 1.5 秒。旧 `LunaAuthSession` 认证后既不发送该心跳，也不持续读取相机帧。

当前轻量媒体会话拥有独立后台 worker：

```text
认证完成
等待约 1.5 秒，不额外插入即时心跳
每 1.5 秒发送 05 <sequence> 空帧
持续读取并排空相机返回帧
EOF / 写入失败 -> worker 结束
下一次媒体操作发现 is_active=false -> 丢弃旧会话并重连
关闭应用 / 切换完整控制 -> Shutdown + join
```

首个保活帧为已在用户会话中观察过的：

```text
55 43 44 32 01 0c 05 11 00 00 00 00 76 20 c6 cb
```

完整控制会话也增加 `is_active` 判断，后台 worker 已结束时不再继续复用失效对象。

## Rust 后端改动

文件：`src/adapters/luna_local.rs`

- 新增 `CameraCaptureMode::{Photo, Video}`。
- `CameraControlSession` 持有当前拍摄模式和录像状态。
- 新增 `switch_capture_mode` 和 `move_gimbal`；删除错误用于普通松手的 `release_gimbal`。
- `take_photo`、`start_recording`、`stop_recording` 增加后端状态门禁。
- 新增普通拍照/录像 body 构造器。
- 新增云台 ZigZag body 构造器和 `-100..100` 范围校验。
- UI 坐标在发送前按 `(-vertical, horizontal)` 映射到设备坐标。
- 云台请求使用 2 秒响应超时，避免连续控制时长时间卡住。
- 新增录像状态等待器；开始/停止录像在收到 `0x2010` 确认事件后才提交状态。
- 控制连接新增 `0x000F` 会话初始化、`0x0027` 客户端登记、动态时间同步、状态读取和事件订阅。
- 新增硬件摇杆速度 body 构造器与 `set_gimbal_speed`；只接受设备已证明的档位 `1..3`。
- 新增 `0x206A` 云台速度状态等待器；速度设置固定使用抓包上下文 `0x06`。
- 删除已被真机否定的 option `0x28` 当前模式查询解析。
- 模式包和上下文包按官方抓包顺序连续入队，并使用 `0x2053` 事件确认真实模式。
- 实时预览启动前补齐 `0x00BF / 58 0a` 与 `0x00C6 / empty`。
- 开始录像前补齐录像上下文和 `0x14/0xB0/0xB1` 准备状态查询。

文件：`src/bin/html_app.rs`

- 新增 `camera_set_capture_mode` IPC。
- 新增 `camera_gimbal_move` IPC；真机否定后已删除 `camera_gimbal_release` IPC。
- 新增 `camera_set_gimbal_speed` IPC，将三档速度直接写入相机。
- 控制连接响应返回当前模式和录像状态。
- 拍照、开始录像、停止录像都改为使用可变控制会话，让后端状态与相机成功响应同步。
- 相册列表、缩略图和下载恢复轻量认证会话；完整控制只在拍摄和删除时按需建立。
- HTML 和照片/视频预览改为同源本机回环服务，媒体代理支持视频 Range 流式转发。
- 相册检测不再短暂连接并关闭 `6666`；轻量媒体会话按官方 PCAP 每 1.5 秒发送 type-05 心跳并持续读取相机帧。

## 前端改动

文件：`web/index.html`

- 左侧导航新增独立“拍摄控制”页面。
- “相机媒体”页面只保留存储、相册、预览、下载和删除。
- 拍摄页包含实时画面、拍照/录像分段选择、上下文快门、录像计时和预览开关。
- 未选模式时快门不可用。
- 拍照模式下快门只执行拍照。
- 录像模式下同一个快门负责开始/停止录像。
- 云台区域包含上、下、左、右、停止按钮和二维触控盘。
- 云台区域新增“慢 / 中 / 快”硬件速度三段按钮，成功响应后才更新选中状态。
- 连接后速度档位不再预选；每次点击都发送，收到 `0x206A` 后才高亮。
- 录像快门在预览关闭时自动启动预览，预览成功后串行继续录像。
- 方向按钮与二维触控盘统一设置目标向量，不再直接按鼠标事件频率发送坐标。
- 控制循环按 25 ms 更新，使用单飞最新点队列和平滑加减速。
- 松手、指针取消和窗口失焦都会将目标向量设为零；控制器降速至零并发送 4 个稳定零向量后停发。
- 停止按钮同样进入平滑零速流程，不发送 `08 02`。
- 云台错误只提示一次，避免连续请求制造通知风暴。
- 页面保持 Mica 透明根层；实时画面区域保持不透明黑色以保证影像准确。
- 1180 x 780 与 760 x 560 均无横向溢出；小窗口使用纵向滚动查看云台区域。

## 连接流程修正

独立拍摄页最初复用了相册页的 `detect` 流程，会先读取整套媒体列表，再排队建立控制会话。素材较多时会让“连接相机”看起来卡住。

现已调整为：

- 拍摄页直接调用 `camera_control_connect`，不读取相册。
- 相册页仍执行设备检测和媒体加载。
- 拍摄页显示“正在连接”“已连接”“控制连接失败”状态。
- 控制会话成功后才启用模式、快门、预览和云台。

## 验证记录

静态与自动化验证：

- 内联 JavaScript 语法检查通过。
- `cargo fmt -- --check` 通过。
- `cargo test --bin html_app`：18 passed，0 failed。
- 新增测试逐字节验证两种普通拍摄模式 body。
- 新增测试逐字节验证五组 PCAP 云台坐标和静止点。
- 新增测试验证四个 UI 方向到设备坐标的换轴映射。
- 新增测试验证抓包中的开始、停止和停止过渡录像状态事件。
- 新增测试逐字节验证相机状态事件订阅内部包 `11 00 02 <request_id> 00 00 08 01`。
- 新增测试逐字节验证 `0x000F` 会话初始化、`0x0027` 客户端登记和抓包时间同步 body。
- 新增测试逐字节验证硬件摇杆速度档位 `1/2/3` 的 `0x0009 / option 0x55` body。
- 新增测试验证 `0x206A` 的 `1/2/3` 档位解析。
- 新增测试验证抓包 `0x2053` 中普通拍照、普通录像和其他模式值的解析。
- 新增测试验证超出 `-100..100` 的云台值会被拒绝。
- 静态检查确认控制循环周期为 25 ms、停止稳定帧为 4，且前后端都不存在 `camera_gimbal_release`/`release_gimbal`。
- release 构建通过；仅保留工程原有的未使用 OSC/dead-code 警告。

视觉验证：

- 拍摄页在 1180 x 780 下无横向溢出。
- 拍摄页在最小 760 x 560 下无横向溢出，云台可通过纵向滚动完整访问。
- Mica 根层、较高可读性前景表面和不透明实时画面区域均保留。

2026-07-23 本轮真机现场：

- 最初的 `192.168.42.1:6666` TCP 探测显示可达，但连接本地地址是代理虚拟网卡 `198.18.0.1`。
- 进一步检查确认物理 `WLAN` 适配器状态是 `Disconnected`，原有 `192.168.42.50/24` 地址处于 `Deprecated`。
- 因此独立控制连接和最小两包认证探测没有收到 UCD2 初始化响应，并不是两阶段模式命令导致相机断开。
- 应用正确保持“控制连接失败”，模式和快门保持禁用。
- 物理 Wi-Fi 未连接时没有发送模式、快门、录像或云台命令。
- 重新连接 LunaU Wi-Fi 后，先确认界面显示“控制已连接”，再依次验证录像模式、开始录像和停止录像；停止响应应返回新 MP4 路径。

## 后续模型入口

1. 先读本文和 `reverse_apk/REVERSE_CONTINUE_LOG.md`。
2. 真机回复 UCD2 初始化帧后，依次验证拍照模式、录像模式；不要在未征得用户同意时触发快门。
3. 云台真机验证先短按四方向，再测试二维触控盘。预期看到连续加速坐标、连续减速坐标和最后的零向量；普通松手绝不能出现 `08 02`。
4. 若模式 body 被相机拒绝，保留响应 JSON/hex，不要猜测新枚举。
5. 不要恢复旧的原始 UCD2 调试 UI。

## 发布

- Step 163/164/165 旧版文件大小和哈希已由下方 Step 166 发布覆盖。
- 文件：`F:/Insta360onWin/LunaStudio.exe`
- 大小：8,968,704 bytes
- SHA-256：`A869366D97147B16BB03309C428855F56B352FD7479096EA61DA4142CB79CC64`
- 发布程序成功创建标题为 `Luna 控制台` 的原生窗口。
- DWM 属性 38 返回值 `2`，属性 20 返回值 `1`，Mica 和深色沉浸仍然生效。
- 本机随机回环页面返回 HTTP 200，并确认发布程序包含媒体代理前端代码。
- 烟雾测试窗口正常退出；随后删除了确认位于项目根目录内的 WebView2 测试配置。
- JavaScript 内联脚本语法检查、UTF-8 CRLF 检查、`cargo fmt -- --check`、20 个 Rust 测试和 release 构建均通过。
- 相册连接按钮现在直接建立认证媒体会话；只有列表成功才显示已连接，失败会明确显示“相机会话连接失败”。
- 本次没有自动触发云台移动或修改硬件速度；三档设备设置需由用户在已连接 Luna Ultra 上完成最终真机验证。
- 本次没有自动触发拍摄模式、拍照或录像；`0x2053` 模式确认和开始/停止录像仍需用户完成最终真机验证。
- 当前电脑的 `192.168.42.1` 经 FlClash `198.18.0.1` 路由，WLAN 地址已失效，因此没有伪装成已经完成真实媒体字节验证；照片原图和 Range 视频播放需用户重新连接 Luna Ultra Wi-Fi 后确认。
- 后续 WLAN 已恢复为 `192.168.42.50`，但相机端当时在 215 ms 内直接关闭第一包 `05 0f` 且返回 0 字节，证明设备仍卡在旧会话状态；必须重启 LunaU 后再验证 Step 166，不能把该设备端拒绝误归因于最终客户端。
- 已删除本轮视觉检查 PNG、JavaScript 临时检查文件、重复 PCAP 分析目录和烟雾测试产生的 WebView2 配置目录。
- 项目根目录只保留 `LunaStudio.exe` 一个 EXE，且没有残留 Luna Studio、Cargo 或 Rust 编译进程。

## Step 167 - 将 Luna 实时画面输出为 Windows 虚拟摄像机

### 用户目标

- 在拍摄控制页一键把 Luna Ultra 返回的实时画面提供给微信、OBS、会议软件等 Windows 应用。
- 保持日用界面，不增加原始包、注册表或媒体源调试页面。
- 项目和发布文件继续只在 `F:/Insta360onWin` 内操作。

### Windows 实现依据

- 相机控制和取流仍只使用本项目已有的 APK/PCAP 逆向结果；本步骤没有引入新的公开相机协议。
- Windows 虚拟摄像机层使用微软官方 `MFCreateVirtualCamera` / `IMFVirtualCamera` 接口。
- 当前系统 build `26200` 高于该 API 要求的 Windows 11 build `22000`。
- 微软官方样例确认自定义媒体源必须是系统可加载的进程内 COM DLL，并注册到：
  `HKLM/Software/Classes/CLSID/{CLSID}/InprocServer32`。
- 参考：
  - `https://learn.microsoft.com/windows/win32/api/mfvirtualcamera/nn-mfvirtualcamera-imfvirtualcamera`
  - `https://learn.microsoft.com/windows/win32/api/mfvirtualcamera/nf-mfvirtualcamera-imfvirtualcamera-start`
  - `https://github.com/microsoft/Windows-Camera/tree/master/Samples/VirtualCamera`

### 早期失败与修正

- 首次原型把媒体源注册为当前用户 COM 类；`IMFVirtualCamera::Start` 返回
  `0x80070003`（系统找不到指定路径）。
- 原因是 Windows Frame Server 服务不读取该当前用户注册。
- 已改为随应用发布 `LunaVirtualCamera.dll`，首次开启时由主程序以 `runas`
  请求一次 UAC，并把 DLL 路径写入 HKLM。
- 非提权安装入口已实测返回 code `1` 且不会污染 HKLM，证明权限门禁生效。
- 之后再次开启不再请求权限；若程序目录变更，应用会检测 DLL 路径并重新安装。

### 后端实现

文件：`Cargo.toml`

- 新增 `cdylib`/`rlib` 库输出。
- 启用 Media Foundation、COM、注册表、进程等待和 Shell 提权所需 Windows API。

文件：`src/lib.rs`

- 导出标准 COM 入口 `DllGetClassObject` 和 `DllCanUnloadNow`。
- 发布 DLL 已解析 PE 导出表确认两个入口均存在。

文件：`src/virtual_camera.rs`

- 实现 `IMFActivate`、`IMFMediaSourceEx`、`IMFMediaStream2` 和 COM class factory。
- 虚拟摄像机名称：`Luna Studio Camera`。
- 媒体源 CLSID：`{670F22B2-30A4-4C92-8B89-CB7D26783509}`。
- 输出格式固定为 `1280 x 720`、`15 fps`、`NV12`，降低消费端兼容风险。
- 主程序将 FFmpeg 解出的 JPEG 帧转换为 NV12。
- 主程序与 Frame Server 内 DLL 通过 `127.0.0.1:38475` 的本机专用帧桥接传递最新画面。
- 桥接帧包含 magic、宽、高、payload 长度和序号；DLL 会校验尺寸和长度。
- 三秒没有新相机画面时输出中性占位帧，不重复显示过期画面。
- 虚拟摄像机使用 Session lifetime；关闭虚拟摄像机、断开 Luna 或退出应用时停止系统相机实例。

文件：`src/bin/html_app.rs`

- 实时预览 FFmpeg 输出统一为 `1280 x 720` letterbox、`15 fps` MJPEG。
- 每个解析成功的 JPEG 同时送往 WebView 实时画面和虚拟摄像机帧存储。
- 新增 `virtual_camera_status`、`virtual_camera_start`、`virtual_camera_stop` 三个内部 IPC 命令。
- 程序启动最早阶段处理提权安装参数，安装进程不会创建窗口、WebView、相机会话或画面端口。
- 断开 Luna 和关闭程序都会先关闭虚拟摄像机。

### 日用 UI

文件：`web/index.html`

- 拍摄控制页新增紧凑的 `Luna Studio Camera` 控制区。
- 状态分为：系统不可用、连接相机后可开启、正在开启、已开启、正在关闭。
- 按钮只在控制连接成功且系统组件可用时启用。
- 如果实时预览尚未开启，点击虚拟摄像机会先自动开启预览，预览成功后串行开启虚拟摄像机。
- 关闭实时预览时会同步关闭虚拟摄像机。
- 第一次开启会出现 Windows UAC；用户确认后后续日常使用不再出现。
- 没有加入任何注册表、COM、端口或原始数据调试 UI。

### 自动化验证

- 内联 JavaScript 语法检查通过。
- `cargo fmt --all` 通过。
- `cargo test --bin html_app`：23 passed，0 failed，1 ignored。
- 新增测试验证 RGBA 到 BGRA/NV12 尺寸和视频色域转换。
- 新增测试逐字节验证帧桥接传输了完整 `1280 x 720 NV12` 帧。
- `cargo build --release --bin html_app --lib` 通过。
- 发布 DLL PE 导出表包含 `DllGetClassObject` 和 `DllCanUnloadNow`。
- 隐藏烟雾启动中：
  - `LunaStudio.exe` 正常响应。
  - `127.0.0.1:38475` 帧桥接处于监听状态。
  - 内嵌页面返回 HTTP 200。
  - 页面包含 `Luna Studio Camera` 和 `toggleVirtualCamera`。
- 当前 Codex 进程未提权，因此没有擅自触发 UAC；完整 `IMFVirtualCamera::Start`
  需要用户在界面第一次点击“开启”并确认 UAC 后完成本机验证。
- 当前未自动触发 Luna 拍摄、录像、删除、云台或速度命令。
- 因相机此前仍会拒绝第一包 UCD2，真实 Luna 画面尚未伪装成已在第三方应用中验证。

### 发布

- 主程序：`F:/Insta360onWin/LunaStudio.exe`
- 大小：`9,221,632 bytes`
- SHA-256：`083DB1C92C93F343A067A99D10F7A8CCF409754DF47645BF8F1869783B75613E`
- Windows 媒体源：`F:/Insta360onWin/LunaVirtualCamera.dll`
- 大小：`225,792 bytes`
- SHA-256：`A41FF5D4E244D6DACC0AA8D3F0C639C5E9AFC94795D3E19A6FD98EC51517684B`
- `LunaVirtualCamera.dll` 必须与 `LunaStudio.exe` 放在同一目录；它不可独立启动，也不是第二个应用。
- 项目根目录仍只有 `LunaStudio.exe` 一个 EXE。
- 烟雾测试后没有残留 Luna Studio、Cargo 或 Rust 编译进程。

### 后续模型/真机验证入口

1. 启动 `LunaStudio.exe`，连接 Luna Ultra，进入“拍摄控制”。
2. 点击 `Luna Studio Camera` 的“开启”；第一次只需在 UAC 点“是”。
3. 在 Windows 相机、OBS、微信或会议软件选择 `Luna Studio Camera`。
4. 验证画面为 `1280 x 720` 且持续更新，再关闭开关并确认设备从第三方应用停止。
5. 若 `Start` 仍失败，先检查 HKLM 的 InprocServer32 默认值是否精确指向
   `F:/Insta360onWin/LunaVirtualCamera.dll`，不要修改 Luna UCD2 协议。
6. 不要删除 DLL、不要恢复当前用户 COM 注册，也不要新增虚拟摄像机调试页。

## Step 168 - 修复虚拟摄像机间歇黑屏并补全模式落地流程

### 用户真机反馈

- Windows 能看到 `Luna Studio Camera`，但最初长时间黑屏。
- 画面偶尔恢复后又会再次变黑，随后又恢复。
- 用户同时指出拍照/录像模式切换仍不像真正完成了设备切换。

### 黑屏根因

真机运行时检查 `127.0.0.1:38475`，同时观察到主程序验证实例和 Windows
相机进程建立了多个 TCP 连接。旧帧服务器虽然允许连接进入 TCP backlog，
但接受第一个连接后会一直在该连接的发送循环中，不能继续为第二个连接发送帧。

结果是：

- 第一个媒体源实例持续收到画面。
- 后续实例显示为已连接，但拿不到任何帧，只能显示黑色占位。
- 当前一个实例断开后，排队实例才突然恢复画面。
- Windows 重新创建媒体源实例时会再次出现黑屏，因此表现为黑、恢复、再黑。

### 虚拟摄像机修正

文件：`src/virtual_camera.rs`

- 每个 Frame Server 客户端连接现在使用独立发送线程。
- 多个验证实例、预览实例和第三方应用可以同时读取同一个最新帧。
- `VirtualCameraController::start` 在请求 Windows Start 之前就启用帧缓存。
- 若 Windows Start 失败，立即关闭帧缓存，不留下错误状态。
- 主程序帧的过期阈值从 3 秒提高到 30 秒；短暂解码抖动时保持最后一帧，
  不再立即切换成黑色占位。
- 新增双客户端测试，确保两个并行消费者都能收到完整 NV12 帧。

文件：`web/index.html`

- 新增 `previewFrameReady` 状态。
- 开启虚拟摄像机必须等待真实 JPEG 首帧成功绘制，不再只等待
  `camera_start_preview` 命令响应。
- 如果预览命令已成功但尚无画面，界面显示“正在等待第一帧实时画面”。
- 首帧到达后才调用 `virtual_camera_start`。

### 模式切换重新核对

只重新读取本地用户 PCAP
`reverse_apk/pcap_analysis/20260722_192838/ucd2_frames.json`。

官方应用切换到普通拍照的完整相关顺序是：

```text
0x0007  08 28 12 00             设置普通拍照
0x000A  08 63 10 06             设置拍照上下文
0x2053  08 00 10 64 18 03       相机确认模式
0x000A  08 63 10 06             刷新上下文
0x0008  08 14 08 b0 01 08 b1 01 读取拍摄准备状态
0x000A  08 57 10 06             读取模式详情
0x000A  08 29 08 63 10 06       读取组合上下文
```

旧实现收到 `0x2053` 后直接提交模式，没有执行后四项模式落地刷新。

文件：`src/adapters/luna_local.rs`

- 模式包和上下文包仍按抓包间隔连续入队。
- 仍必须收到目标 `0x2053` 和两个匹配 `0x00C8`。
- 相机确认目标模式后，新增上下文、准备状态、模式详情和组合上下文四项刷新。
- 四项刷新全部成功后才把后端模式改为 Photo 或 Video。
- Photo 使用抓包 context `0x06`；Video 使用抓包 context `0x07`。
- 普通录像 body 继续使用 APK 恢复值
  `08 28 12 03 c0 02 01`；用户 PCAP 本身没有包含从其他模式切回普通录像的动作，
  因此不能擅自改成未捕获枚举。
- 测试新增逐字节验证：
  - `08 57 10 06/07`
  - `08 29 08 63 10 06/07`

### 验证与发布

- 内联 JavaScript 语法通过。
- `cargo test --bin html_app`：24 passed，0 failed，1 ignored。
- `cargo build --release --bin html_app --lib` 通过。
- 发布版隐藏启动后，同时建立两个独立帧桥接客户端。
- 两个客户端都立即收到：
  - magic：`LVC1`
  - payload：`1,382,400 bytes`
  - 即完整 `1280 x 720 NV12` 帧。
- 没有自动触发相机模式、快门、录像、删除、云台或速度命令。
- 最终仍需用户用 Luna Ultra 验证 `0x2053` 和新增四项刷新是否全部返回成功。

发布文件：

- `F:/Insta360onWin/LunaStudio.exe`
- 大小：`9,221,120 bytes`
- SHA-256：`23253EDECFF0CC3C2E6C0B8685A5880737B84E07B420B0FE3F713E56F7149F43`
- `F:/Insta360onWin/LunaVirtualCamera.dll`
- 大小：`225,792 bytes`
- SHA-256：`2D62C39A2F0DBC2F06EF966140D2AE2F95BD4A14CE36662C40F32553D80A9A32`

## Step 169 - 修复 Windows 相机 CameraSwitchFailed 0x80070057

### 用户现场现象

- Windows 相机能找到 `Luna Studio Camera`。
- 打开设备时提示“无法启动你的相机”。
- 错误码为 `0xA00F4241<CameraSwitchFailed> (0x80070057)`。
- 这说明设备注册已经生效，失败发生在 Windows Frame Server 启动媒体流的阶段，不是驱动未安装。

### 与微软本地参考实现的差异

只对照项目内已经保存的微软 VirtualCamera 官方示例：

`target/microsoft-windows-camera-ref/Samples/VirtualCamera/VirtualCameraMediaSource`

发现旧实现存在以下媒体源契约缺口：

- 只实现了 `IMFMediaSourceEx`，没有实现虚拟相机 Frame Server 使用的
  `IMFSampleAllocatorControl`。
- `MESourceStarted` 和 `MESourceStopped` 事件使用空 `PROPVARIANT`，没有携带
  `MFGetSystemTime()` 的 100ns 系统时间。
- `Start` 没有校验开始位置、流编号、流选择状态、主媒体类型和 NV12 子类型。
- 外部 Presentation Descriptor 选中的流没有同步到内部 Presentation Descriptor。
- `Stop` 没有撤销内部流选择。
- `IMFMediaStream2::SetStreamState` 接受了不合法的 Pause 状态转换。

### 实现修正

文件：`src/virtual_camera.rs`

- 媒体源改为同时实现：
  - `IMFMediaSourceEx`
  - `IMFSampleAllocatorControl`
- 输出流 `0` 明确报告
  `MFSampleAllocatorUsage_UsesCustomAllocator`，与当前源自行创建内存样本的实现一致。
- `SetDefaultAllocator` 接受合法的输出流 `0`；未知流返回 `MF_E_NOT_FOUND`。
- `Start` 现在要求非空 Presentation Descriptor 和开始位置。
- `Start` 校验：
  - 只有一个流；
  - 流已选择；
  - 流 ID 为 `0`；
  - 主类型为 Video；
  - 子类型为 NV12。
- 开始时同步选中内部流描述，停止时撤销选择。
- `MESourceStarted` 和 `MESourceStopped` 现在携带以 `MFGetSystemTime()` 创建的
  `VT_I8` 时间值。
- Pause 只允许从 Running 进入；其它非法状态转换返回
  `MF_E_INVALID_STATE_TRANSITION`。
- 没有修改 Luna UCD2、预览、拍摄、云台、相册或删除协议。
- 没有新增任何调试页面或日用界面入口。

### 新增验证

新增两个媒体源首帧测试：

1. `media_foundation_reader_receives_first_frame`
   - 直接创建 Rust 媒体源；
   - 使用 `IMFSourceReader` 选择 1280 x 720 NV12；
   - 成功读取一个完整的 `1,382,400-byte` 样本。
2. `windows_registered_camera_returns_first_frame`
   - 创建与应用相同的 Session lifetime 虚拟摄像机；
   - 从虚拟摄像机提供的系统 symbolic link 重新调用 `MFCreateDeviceSource`；
   - 通过系统注册的 `F:/Insta360onWin/LunaVirtualCamera.dll` 激活源；
   - 处理首次媒体类型变化事件后继续读取；
   - 成功取得完整的 `1,382,400-byte` NV12 首帧。

验证结果：

- `cargo test --all-targets` 全部通过：
  - lib：5 passed，2 ignored；
  - html_app：25 passed，2 ignored；
  - main：18 passed。
- 系统注册相机测试单独以 `--ignored` 运行：1 passed。
- `cargo build --release --bin html_app --lib` 通过。
- 发布时 Windows Camera 仍占用旧 DLL；关闭 Camera 后，使用一次 UAC 停止
  `FrameServer` 并完成 DLL 替换。
- HKLM `InprocServer32` 仍精确指向：
  `F:/Insta360onWin/LunaVirtualCamera.dll`
- `ThreadingModel` 仍为 `Both`。
- 根目录仍只有一个可启动文件：`LunaStudio.exe`。

### Step 169 发布文件

- `F:/Insta360onWin/LunaStudio.exe`
- 大小：`9,221,120 bytes`
- SHA-256：`6F5315B20BD42AC8CF66D58DF4259429EF8C69E50E41DC2DB778BF224F0F89EF`
- `F:/Insta360onWin/LunaVirtualCamera.dll`
- 大小：`228,352 bytes`
- SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`

### 真实 Luna 最终确认

自动化测试已经证明 Windows 可以启动系统注册的源并读取首帧，但该测试使用本地帧桥中的
占位 NV12 帧，没有连接真实 Luna Ultra。

最终操作：

1. 启动 `LunaStudio.exe`。
2. 连接 Luna Ultra 并进入“拍摄控制”。
3. 开启实时预览，等应用内出现真实画面。
4. 开启 `Luna Studio Camera`。
5. 重新打开 Windows 相机并选择 `Luna Studio Camera`。

预期结果是不再出现 `0x80070057`，并在短暂启动后显示 Luna 实时画面。
## Step 170 - 拍照与录像回归修复

### 用户现象

- 旧版本曾经可以控制拍照，当前版本拍照和录像都无法完成。
- 用户要求重新确认 Wi-Fi 请求是否发错。

### 仅使用本地 APK 与 PCAP 的复核结论

本次重新核对：

- `reverse_apk/pcap_analysis/20260722_192838/ucd2_frames.json`
- 项目内已经拆出的 APK 枚举和控制节点证据
- `src/adapters/luna_local.rs` 当前实现

PCAP 证明日用快门命令本身没有发错：

- 拍照：命令 `0x0003`，body `30 03`
- 开始录像：命令 `0x0004`，body `08 01`
- 停止录像：命令 `0x0005`，body `10 01`

需要纠正一个分析过程中的误判：

- PCAP 后段出现过模式 body `08 28 12 03 c0 02 08` 和 context `0x43`。
- APK 枚举表明 wire value `8` 不是普通录像，而是另一个非日用拍摄模式。
- 普通录像继续使用 APK 恢复出的 wire value `1`：
  `08 28 12 03 c0 02 01`。
- 成功录像时 PCAP 已观察到普通录像 context `0x07`。
- 因此没有把普通录像模式错误替换成 `8`，也没有发送猜测枚举值。

### 确认的两个回归原因

1. 前端收到“预览启动命令响应”后立即发送开始录像，但此时 Luna 的真实预览关键帧可能尚未到达。
   官方 PCAP 中预览画面已经稳定输出后才发生开始录像。
2. Step 168 把模式切换后的四条只读/辅助刷新都当成模式切换的硬成功条件。
   相机即使已经返回模式命令成功并发出 `0x2053` 模式事件，只要任意辅助查询超时，
   应用仍会把模式切换判定为失败，导致拍照按钮无法使用。

### 后端修复

文件：`src/adapters/luna_local.rs`

- 将三条抓包证明过的快门命令提为固定常量，避免后续流程修改时误改字节：
  - `CAMERA_TAKE_PHOTO_COMMAND/BODY`
  - `CAMERA_START_RECORD_COMMAND/BODY`
  - `CAMERA_STOP_RECORD_COMMAND/BODY`
- 模式切换的硬成功条件恢复为：
  - `0x0007` 模式请求返回 `0x00C8`
  - `0x000A` context 请求返回 `0x00C8`
  - 收到匹配目标模式的 `0x2053` 相机事件
- 满足以上条件后立即提交本地 `capture_mode`。
- context、capture-ready、detail、combined-context 四条 APK/PCAP 辅助刷新仍然发送，
  但单条查询超时不再推翻已经由相机确认的模式。
- 如果辅助刷新期间控制 socket 实际断开，仍然向 UI 返回失败，不伪造成功。
- 开始录像前的 Video context 仍要求成功；只读 capture-ready 查询超时不再阻止发送
  已被 PCAP 证明的 `0x0004 / 08 01`。
- 开始录像仍必须收到 `0x2010` 的 recording=true 事件才在应用内进入录像状态。

### 前端修复

文件：`web/index.html`

- 新增 `startPendingRecordingWhenReady()`。
- 自动开始录像现在同时要求：
  - `camera_start_preview` 已经成功返回
  - 第一帧 JPEG 已经真实解码并绘制到预览画布
  - 当前没有另一个开始录像请求正在执行
- 如果预览已经开启但还没有首帧，按录像键只进入等待状态，不会提前发录像命令。
- 首帧到达后自动发送一次开始录像命令，并立即清除 pending 标志，避免重复发送。
- 虚拟摄像机原有首帧门禁保持不变。

### 回归测试

- 新增 `captured_shutter_commands_are_reproducible`：
  - 固定拍照 `0x0003 / 30 03`
  - 固定开始录像 `0x0004 / 08 01`
  - 固定停止录像 `0x0005 / 10 01`
- 模式测试重命名为 `apk_and_pcap_luna_capture_modes_are_reproducible`，
  明确普通录像模式 body 来自 APK，Photo/context 来自 APK 与 PCAP 交叉证据。
- `cargo test --all-targets`：
  - lib：`5 passed / 2 ignored`
  - html_app：`26 passed / 2 ignored`
  - main：`19 passed`
- 两个 ignored 测试是需要主动打开系统虚拟摄像机的既有测试，与本次拍摄控制修改无关。
- 内联 JavaScript 语法检查通过。
- `cargo fmt --all -- --check` 通过。
- `cargo build --release --bin html_app` 通过。
- 新 EXE 启动 5 秒后仍正常运行，窗口标题为 `Luna 控制台`，随后正常关闭。

### Step 170 发布文件

- `F:/Insta360onWin/LunaStudio.exe`
- 大小：`9,222,656 bytes`
- SHA-256：`64C60D71C6716DBCA69A2C577ADFDA95D5705A8F2D8AC914AC7EDA6BEFA3F2B2`
- `LunaVirtualCamera.dll` 没有修改，继续使用 Step 169 已验证版本。
- 根目录仍然只有一个 EXE：`LunaStudio.exe`。
- 本次触及的 Rust、HTML 和 Markdown 文件统一为 UTF-8 无 BOM、CRLF。

### 真机验证顺序

当前开发机没有连接 Luna Ultra，因此没有自动触发真实拍照或录像。
下一次真机只验证日用流程，不需要打开任何调试页面：

1. 完全关闭旧版 `LunaStudio.exe`，再启动根目录新版本。
2. 连接 Luna Ultra，进入“拍摄控制”并点击连接。
3. 选择“拍照”，等待 UI 明确显示拍照模式，再按快门；相机应拍照，相册稍后刷新。
4. 选择“录像”，按快门。
5. 应用会先开启预览并等待真实画面；第一帧出现后才发送录像命令。
6. 只有收到相机录像事件后，UI 才显示正在录像。
7. 再按一次快门停止录像，并在相册中确认新视频。

## Step 171 - 相册 403 与 UCD2 文件列表修复

### 用户现场

相册同时报告：

```text
内部存储：
HTTP 403 /storage_internal/DCIM/
HTTP 403 /storage_internal/DCIM/Camera01/

SD 卡：
HTTP 403 /DCIM/
HTTP 403 /DCIM/Camera01/
```

旧实现把 HTTP 目录页当作相册索引。相机允许请求具体媒体文件，但禁止浏览目录，
因此内部存储和 SD 卡都会返回 `403 Forbidden`。

### PCAP 证据

重新读取用户原始 `PCAPdroid_22_7月_19_28_38.pcap`，仅在 F 盘生成临时完整解析，
提取缺失字节后立即删除临时目录。没有在 C 盘写入项目文件。

官方应用没有发送任何 HTTP 目录 GET。相册文件名来自 UCD2 `command 0x000D`，
method `0x02`。已确认请求 body：

```text
内部存储，第 1 页：08 02 18 64 20 02
SD 卡，第 1 页：   08 02 18 64 20 03
SD 卡，offset 100：08 02 10 64 18 64 20 03
内部存储，类别 3： 08 03 18 64 20 02
```

字段解释以本地 PCAP 为准：

```text
field 1 = 类别 2 / 3
field 2 = offset，仅非零时发送
field 3 = page size 100
field 4 = 存储位置，2 内部存储 / 3 SD 卡
```

响应的 protobuf field 1 是相机文件路径：

```text
/storage_internal/DCIM/Camera01/...
/DCIM/Camera01/...
```

SD 卡第 1 页达到 100 条时，官方应用用 `offset 100` 继续读取。当前实现沿用这一分页方式，
并在不足 100 条、没有新增路径或达到 100 页保护上限时停止。

PCAP 还确认会话初始化在客户端登记前有一条此前遗漏的完整能力读取：

```text
command 0x0008
request id 0x80000003
internal packet 192 bytes
body 183 bytes
```

完整 body 已从原始 PCAP 恢复并加入字节级测试，不使用公开协议猜测。

### 后端实现

文件：`src/adapters/luna_local.rs`

- `MediaStorage` 增加 UCD2 存储 selector。
- 轻量 `LunaAuthSession` worker 支持带动态 request ID 的普通命令执行。
- 认证阶段不再盲目排空 200 ms 数据；必须读到 request `0x80000001` 的真实 `0x00C8`
  设备信息响应后才建立媒体会话。
- TCP 连接失败仍可短暂重试；相机一旦明确关闭认证连接则立即停止，不连续制造多次连接闪烁。
- 轻量媒体会话按以下顺序初始化：

```text
认证 / 设备信息 0x0008 request 1
会话初始化      0x000F request 2
完整能力读取    0x0008 request 3
客户端登记      0x0027 request 4
时间同步        0x0007 request 5
状态读取        0x0008 request 6
文件列表        0x000D request 7...
```

- 轻量媒体会话不提前发送拍摄事件订阅，文件列表前的顺序与 PCAP 一致。
- 完整 `CameraControlSession` 同样补上完整能力读取；已有控制会话时相册复用它，
  不创建第二个 `6666` 连接。
- 新增 UCD2 文件列表分页、protobuf 路径解析、存储路径校验、UTF-8 URL 编码、
  文件类型和文件名时间解析。
- HTTP 目录列表已退出日用相册主流程。HTTP 只在拿到具体文件 URL 后用于缩略图、
  照片/视频预览和下载。

文件：`src/bin/html_app.rs`

- `list_media` 改为调用持久媒体会话或已有完整控制会话的 UCD2 文件列表。
- 不再为每个存储位置另开一次端口 `6666`。
- 连接失败显示具体会话错误，不再把 HTTP 可达误报为相册已连接。
- 没有新增调试页面或原始包 UI。

### 回归测试

新增：

- `captured_camera_session_setup_is_reproducible`
  - 固定 183 字节完整能力 body。
  - 固定 PCAP 的 192 字节 internal packet。
- `captured_ucd2_file_list_requests_are_reproducible`
  - 固定内部、SD、offset 100 和类别 3 body。
- `ucd2_file_list_response_builds_daily_media_items`
  - 验证 protobuf 路径、存储标签、日期、类型和 URL。
- `connected_camera_lists_media_via_ucd2`
  - 显式 ignored 的只读真机测试，不删除、不下载、不拍摄、不移动云台。

最终 `cargo test --all-targets`：

```text
lib：      5 passed / 2 ignored
html_app：28 passed / 3 ignored
main：     21 passed / 1 ignored
```

内联 JavaScript 语法检查、Rust 格式检查和 release 构建均通过。

### 真机边界

在 WLAN `192.168.42.50/24` 仍存在时做了两次只读验证。相机在认证/设备信息阶段主动关闭
新连接，文件列表命令尚未发送，因此不能把这次结果判定为 `0x000D` 失败。

最终代码随后收紧为相机拒绝认证时只尝试一次。为避免继续扰动相机，没有再做第三次真机连接。
下一次验证前先完全退出手机官方应用和旧版 LunaStudio，再重启 Luna Ultra；新版只需在相册页
点一次连接。成功后应直接显示内部存储或 SD 卡内容，不再出现目录 `403`。

### Step 171 发布文件

- `F:/Insta360onWin/LunaStudio.exe`
- 大小：`7,650,816 bytes`
- SHA-256：`F51ADF9ADBB3D4B679A616F72D553198B8AA57D0340B35D2583592B1746FF998`
- `LunaVirtualCamera.dll` 未修改：
  - 大小：`228,352 bytes`
  - SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`
- 隐藏启动检查运行 6 秒，窗口标题为 `Luna 控制台`，进程响应正常并正常关闭。
- 删除了临时 PCAP 完整解析目录和旧 `LunaStudio.exe.WebView2` 运行缓存。
- 根目录只有一个 EXE：`LunaStudio.exe`。
- 本次触及的 Rust 与 Markdown 文件统一为 UTF-8 无 BOM、CRLF。

## Step 172 - 普通录像模式、变焦与 1080p 48fps

### 新抓包

用户提供：

```text
C:/Users/H!Mooo/Downloads/PCAPdroid_28_7月_15_48_50.pcap
```

用户操作顺序：

```text
连接时为拍照模式
3× 变焦
拍照
切换录像
2× / 3× / 6× 变焦
1080p
48fps
开始录像（有重复尝试）
停止录像
```

只读取 C 盘原始 PCAP，所有解析结果写入：

```text
F:/Insta360onWin/reverse_apk/pcap_analysis/20260728_154850/
```

完整逐项证据见同目录 `CONTROL_FINDINGS.md`。

### 关键修复

文件：`src/adapters/luna_local.rs`

- 普通录像模式从错误的特殊模式 body 改为抓包确认的：

```text
command 0x0007
08 29 12 00
```

- 普通拍照模式保持：

```text
command 0x0007
08 28 12 00
```

- 模式事件不再把 field 1 当作 `0/1` 枚举，而是匹配完整 `0x2053`：

```text
Photo  08 00 10 64 18 03
Video  08 64 10 00 18 03
```

- Step 172 曾把控制会话连接后的默认模式设为 Photo；Step 173 已撤销该硬编码，
  改为读取完整状态响应中的真实模式。
- 开始录像移除额外前置请求，只发送：

```text
command 0x0004 / 08 01
```

- 录像状态改为：

```text
08 01 10 00 38 00  准备中，不提交 UI
08 01 10 00 38 02  稳定录像
08 00 10 00 38 00  已停止
```

- 新增变焦构造器，使用 little-endian `f64`：

```text
Photo 3×：08 35 12 0a a9 03 00 00 00 00 00 00 08 40 18 06
Video 2×：08 35 12 0a a9 03 00 00 00 00 00 00 00 40 18 07
Video 3×：08 35 12 0a a9 03 00 00 00 00 00 00 08 40 18 07
Video 6×：08 35 12 0a a9 03 00 00 00 00 00 00 18 40 18 07
```

- 新增已确认录像规格：

```text
1080p：       08 1f 12 03 f8 01 28 18 07
1080p 48fps： 08 1f 12 04 f8 01 84 02 18 07
```

文件：`src/bin/html_app.rs`

- 新增日用命令 `camera_set_zoom`。
- 新增日用命令 `camera_set_video_profile`。
- 输入只接受抓包确认的变焦档位与录像规格。

文件：`web/index.html`

- 拍摄侧栏新增 1×、2×、3×、6× 变焦分段控件。
- 录像模式新增“1080p”和“1080p · 48fps”组合规格。
- 录像按钮不再先开启预览并等待首帧，直接发送设备录像命令。
- 实时预览和虚拟摄像机仍可独立使用。
- 新控件使用固定网格与最小宽度约束，避免侧栏横向溢出。
- 没有恢复任何原始包或调试功能。

### 测试与发布

- 新增模式、事件、变焦与录像规格的字节级回归测试。
- `cargo test --all-targets`：

```text
lib：      5 passed / 2 ignored
html_app：30 passed / 3 ignored
main：     23 passed / 1 ignored
```

- `cargo fmt --all -- --check` 通过。
- 内联 JavaScript 语法检查通过。
- release 构建通过。
- 根目录 EXE 隐藏启动 6 秒后窗口标题为 `Luna 控制台`，进程响应正常并正常关闭。
- 发布：

```text
F:/Insta360onWin/LunaStudio.exe
size: 7,671,808 bytes
SHA-256: 7A200847EECDBC9D134E880154B6012A931758E743D42EDFE61FFC7D2AB4FAB7
```

- `LunaVirtualCamera.dll` 未修改：

```text
size: 228,352 bytes
SHA-256: 19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00
```

- 发布前关闭了仍在运行的旧 `LunaStudio.exe` 与其 FFmpeg 预览子进程。
- 删除了重新生成的 `LunaStudio.exe.WebView2` 运行缓存。
- 根目录仍只有一个 EXE：`LunaStudio.exe`。
- 没有自动连接相机或触发拍照、录像、删除、云台移动。

### 下一次真机验证

1. 启动根目录 `LunaStudio.exe`。
2. 连接 Luna Ultra，确认拍摄页显示“拍照模式”。
3. 点 3× 后拍照，确认照片生成。
4. 切到录像，确认设备进入普通录像模式。
5. 依次验证 2×、3×、6×。
6. 依次选择 1080p、1080p · 48fps。
7. 点一次录像；只在相机稳定录像后 UI 才变为“正在录像”。
8. 点一次停止并确认相册出现新 MP4。

## Step 173 - 连接时同步相机真实拍摄模式

### 用户纠正

用户指出相机连接时应当把当前拍照/录像状态告知应用，
不应由 Luna Studio 硬编码 Photo。

### PCAP 证据

连接初始化已有完整状态查询：

```text
command 0x0008
request id 0x80000003
```

本次 PCAP 的相机响应位于 `15:49:05.969`，internal payload 长度 `1,171 bytes`。
响应 body 顶层 protobuf field 2 是完整状态消息。该嵌套消息偏移 `607`：

```text
c0 02 00 c8 02 64
```

解码后：

```text
field 40 = 0
field 41 = 100
```

用户确认连接时相机为拍照模式，因此直接得到：

```text
Photo = (field40=0, field41=100)
```

模式切换事件 `0x2053` 已确认：

```text
Photo = (field1=0, field2=100)
Video = (field1=100, field2=0)
```

相同二值对交叉确认 Video 状态为：

```text
Video = (field40=100, field41=0)
```

### 后端实现

文件：`src/adapters/luna_local.rs`

- `CameraControlSession` 初始化时不再设置 `Some(Photo)`。
- 完整状态查询响应不再丢弃。
- 新增通用 protobuf length-delimited field 读取器。
- 从响应顶层 field 2 进入嵌套状态，再读取 field 40/41。
- `(0,100)` 映射为 Photo。
- `(100,0)` 映射为 Video。
- 未知组合或结构不完整时保持 `capture_mode=None`，不使用默认值。
- 连接 IPC 原本就返回 `session.capture_mode()`，前端会自动显示真实模式，
  不需要额外请求或调试 UI。

### 测试与发布

- 新增 `captured_full_status_reports_current_capture_mode`：
  - 验证 Photo；
  - 验证 Video；
  - 验证未知组合；
  - 验证缺失状态消息。
- `cargo test --all-targets`：

```text
lib：      5 passed / 2 ignored
html_app：31 passed / 3 ignored
main：     24 passed / 1 ignored
```

- release 构建通过。
- 根目录 EXE 隐藏启动 6 秒后窗口标题为 `Luna 控制台`，进程响应正常。
- 发布：

```text
F:/Insta360onWin/LunaStudio.exe
size: 7,673,856 bytes
SHA-256: AE7F0A7EE90F434D620C5557D15D7E536A17490524AA69F4528A38C75A91E8E6
```

- `LunaVirtualCamera.dll` 未修改：

```text
size: 228,352 bytes
SHA-256: 19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00
```

- 发布前关闭了用户重新打开的旧 `LunaStudio.exe` 与 FFmpeg 预览子进程。
- 删除了启动检查生成的 WebView2 运行缓存。
- 根目录仍只有一个 EXE。
- 没有自动连接 Luna Ultra 或触发任何真机写操作。

### 真机确认

1. 先在相机上切到拍照，连接 Luna Studio；UI 应自动选中“拍照”。
2. 断开应用，在相机上切到录像，再重新连接；UI 应自动选中“录像”。
3. 若完整状态出现未识别组合，UI 会保持“请选择拍摄模式”，而不是误报拍照。

## Step 174 - 完整普通录像规格与联动选择器

### 用户纠正

旧界面只有两个录像规格，并把协议值 `40` 显示成含糊的“1080p”。
用户指出该值实际是 `1920x1080@60fps`，Luna Ultra 的清晰度、画幅和帧率
远不止两个档位。

本步把“普通录像”完整接入；夜景录像、慢动作、延时摄影和照片规格先记录为
待接入能力，不用普通录像命令冒充这些子模式。

### APK 静态证据

只读取本地 APK 重建结果：

```text
reverse_apk/reconstructed_dex/seg02.dex
VideoResolution static initializer code offset: 0x4cf304
code units: 5299
enum entries: 324
constructor: method@0x7d91
```

构造参数包含宽、高、帧率和设备协议值。PCAP 再次交叉确认：

```text
1920x1080@60 -> 40
1920x1080@48 -> 260
```

普通录像 10 种画幅、64 个合法组合的协议值：

```text
8K 16:9      7680x4320  30=154 25=211 24=210
8K 2.35:1    7680x3264  30=213 25=212 24=219
4K 16:9      3840x2160  120=214 100=220 60=23 50=92 48=258 30=24 25=48 24=49
4K 2.35:1    3840x1632  120=433 100=434 60=435 50=436 48=437 30=438 25=439 24=440
3K 1:1       3072x3072  60=446 50=447 48=448 30=121 25=120 24=119
3K 9:16      1728x3072  60=450 50=451 48=452 30=453 25=454 24=455
2.7K 16:9    2688x1520  120=242 100=243 60=244 50=245 48=331 30=246 25=247 24=248
2.7K 9:16    1520x2688  60=441 50=442 48=468 30=443 25=444 24=445
1080p 16:9   1920x1080  240=27 200=26 120=28 100=150 60=40 50=81 48=260 30=29 25=52 24=53
1080p 9:16   1080x1920  60=68 50=82 48=298 30=64 25=71 24=85
```

APK 枚举把 8K 2.35:1 的内部尺寸写作 `7680x3268`，产品规格和日用 UI
按设备显示名称使用 `7680x3264`；协议值直接来自该枚举，不由尺寸字符串推算。

### 后端实现

文件：`src/adapters/luna_local.rs`

- `CameraVideoProfile` 从两个硬编码枚举项改为包含格式、尺寸、画幅、帧率和
  协议值的结构。
- `CAMERA_VIDEO_FORMATS` 保存 10 种普通录像格式和 64 个合法组合。
- `resolve_camera_video_profile(format_id, fps)` 是唯一解析入口。
- 录像规格写入仍使用 PCAP 已确认的 option field `31` 和 video context `7`。
- 前端不能直接提交协议值；IPC 只接收格式 ID 和帧率，后端必须在白名单中解析。
- 非法组合直接返回中文错误。
- 相机不在录像模式或正在录像时，后端拒绝修改规格。

文件：`src/bin/html_app.rs`

- `camera_set_video_profile` 接收 `{ host, format, fps }`。
- 成功响应返回格式、画幅、尺寸、帧率和相机响应，供 UI 同步。

### 日用界面

文件：`web/index.html`

- 删除两个容易误解的规格按钮。
- 使用三个联动选择器：清晰度、画幅、帧率。
- 清晰度决定可选画幅，画幅决定合法帧率。
- 只有选择完整合法组合后才向相机发送一次设置。
- 成功前不把选择显示成相机当前状态。
- 两列清晰度/画幅加一行帧率，适配窄控制栏，不产生横向溢出。
- 未连接、非录像模式、正在录像或控制忙碌时禁用选择器。
- 没有新增调试入口或原始包输入框。

真实 `1196x819` 窗口通过 `PrintWindow` 检查：

```text
target/step174-ui-printwindow.png
target/step174-control.png
target/step174-control-scroll.png
```

相机未连接时录像规格区域按日用逻辑隐藏；页面、控制栏和云台区域均可纵向滚动，
没有横向溢出。

### 尚未伪造的模式

用户提供的能力要求已保留：

- 夜景录像：4K/3K/2.7K/1080p，共 36 个分辨率与帧率组合。
- 慢动作：4K、2.7K、1080p，共 10 个组合。
- 延时摄影：4K、2.7K、1080p，均为 30fps。
- 照片：3700 万、3300 万、900 万、800 万和 2 亿宽幅。

APK 已找到 `VIDEO_NORMAL`、`VIDEO_PURE`、`VIDEO_SLOW_MOTION`、
`VIDEO_TIMELAPSE` 等枚举，但当前还没有从 APK/PCAP 证明 LunaU 各子模式的
完整 `0x0007` 写入 body。照片的 PhotoSize/PhotoResolution 选项值也尚未证明。
因此发布版只开放已确认的普通录像 64 组，不发送猜测模式包。

### 验证与发布

- UTF-8 无 BOM、CRLF 检查通过。
- `cargo fmt --all -- --check` 通过。
- 内联 JavaScript `node --check` 通过。
- `cargo test --all-targets`：

```text
lib:      5 passed / 2 ignored
html_app: 32 passed / 3 ignored
main:     25 passed / 1 ignored
```

- 新测试验证 64 组完整覆盖、代表性 APK 映射、非法组合拒绝，以及 PCAP 中
  `1080p60/48` 请求字节完全一致。
- release 构建通过。
- 根目录发布版启动后窗口标题为 `Luna 控制台`，进程响应正常并正常关闭。
- DWM 检查：

```text
DWMWA_SYSTEMBACKDROP_TYPE (38) = 2
DWMWA_USE_IMMERSIVE_DARK_MODE (20) = 1
```

- 发布：

```text
F:/Insta360onWin/LunaStudio.exe
size: 7,686,656 bytes
SHA-256: 7ED54580DDF2E893EE3CC57DF8DD45CE1A9BCEC7687C5464BF57F21AF38E59FF
```

- `LunaVirtualCamera.dll` 未修改：

```text
size: 228,352 bytes
SHA-256: 19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00
```

- 启动检查产生的 WebView2 缓存已删除。
- 根目录只有 `LunaStudio.exe` 一个 EXE，没有遗留项目进程。
- 没有自动连接相机，也没有触发模式、规格、快门、删除或云台写操作。

### 真机确认

1. 启动根目录 `LunaStudio.exe` 并连接 Luna Ultra。
2. 确认应用自动显示相机当前 Photo/Video 状态。
3. 在录像模式依次测试 `4K 16:9 48fps`、`1080p 16:9 60fps`、
   `1080p 16:9 240fps` 和一个 `9:16` 组合。
4. 每次设置后观察相机屏幕规格是否同步，再开始和停止一次录像。
5. 若某个 APK 枚举值被设备拒绝，保留应用错误提示并记录该组合，不连续发送其他设置。
6. 夜景、慢动作、延时和照片规格要在其模式写入 body 被 APK/PCAP 证明后再接入。

## Step 175 - 变焦真实状态与写后校验

### 问题结论

用户发现变焦显示与设备行为不一致。复核后确认写包本身没有把数值编码错：

```text
Photo 3×  -> option 0x35, fixed64 3.0, context 0x06
Video 1×  -> option 0x35, fixed64 1.0, context 0x07
Video 2×  -> option 0x35, fixed64 2.0, context 0x07
Video 3×  -> option 0x35, fixed64 3.0, context 0x07
Video 6×  -> option 0x35, fixed64 6.0, context 0x07
```

真正的错误位于状态管理：

- 前端在模式切换成功后直接把 `cameraZoom` 写成 `1`。
- 这一步既没有读取相机，也没有真正发送 1×。
- 因为 UI 误以为已经是 1×，用户随后点击 1× 时请求还会被吞掉。
- 连接成功时变焦始终被清空，无法显示相机当前值。
- 设置成功后 UI 显示的是请求值，不是设备实际值。

APK 静态字符串还确认变焦模型是浮点状态，而不是四个整数枚举：

```text
ZoomScaleSetting
ZoomScaleRangeSegment
ZoomModel(minRatio=
ZoomState(currentRatio=
getSupportZoomScaleList
getSupportZoomScaleQuickTabs
getSupportZoomScaleMajorTick
```

因此 1×、2×、3×、6× 只保留为 PCAP 已确认的快捷点，不再宣称是完整变焦档位。

### PCAP 状态读取

只使用本地抓包
`reverse_apk/pcap_analysis/20260728_154850/ucd2_frames.json`。

官方应用通过 command `0x000a` 发送 168 字节拍摄设置查询：

- 83 个 setting ID；
- 最后附加 context `0x06`（Photo）或 `0x07`（Video）；
- 相机响应顶层 protobuf field 2 是拍摄状态；
- 该状态的 field 53、wire type 1 是 little-endian fixed64；
- 抓包实际返回 `1.0`。

录像模式下同一份 83 项查询和 context `0x07` 也在抓包中出现，不能把 Photo
查询上下文硬套给 Video。

### 后端实现

文件：`src/adapters/luna_local.rs`

- 新增抓包原序的 `CAMERA_CAPTURE_SETTING_IDS`。
- 新增 `build_capture_settings_query_body(mode)`，生成 Photo/Video 对应查询。
- 新增通用 `protobuf_fixed64_field`。
- 新增 `zoom_from_capture_settings_body`，只接受有限、正数且合理的设备返回值。
- `CameraControlSession` 保存 `zoom: Option<f64>`。
- 控制连接建立后读取一次真实变焦；读取失败但连接仍存活时不伪造数值。
- `set_zoom` 改用 `f64` 协议语义，收到 option ACK 后立即读取拍摄设置确认。
- 只有读回成功才更新会话变焦状态。
- 模式切换完成上下文刷新后，按官方抓包顺序真正发送 1×，再读回实际值。
- 目前写入白名单仍只有抓包确认的 1×、2×、3×、6×；没有凭空开放任意小数。

文件：`src/bin/html_app.rs`

- 连接、模式切换和变焦 IPC 均返回设备实际 `zoom`。
- 不再把请求参数直接当成设备状态。
- 无实际返回值时操作失败，不向前端伪报成功数值。

### 前端实现

文件：`web/index.html`

- 删除“切换录像模式后直接显示 1×”的硬编码。
- 连接和模式切换只采用 IPC 返回的真实 `zoom`。
- 增加独立当前值显示，支持整数和小数。
- 四个按钮改名为“快捷变焦”。
- 按钮高亮使用浮点容差，不依赖整数严格相等。
- 相机返回未识别值时显示该值，但不会错误高亮某个快捷点。
- 1196×819 拍摄页检查没有横向溢出。

### 测试与发布

- 新增 `captured_zoom_state_query_and_response_are_reproducible`：
  - 168 字节 Photo 查询与 PCAP 逐字节一致；
  - 解析 1.0；
  - 解析 2.5，证明状态不是整数枚举；
  - 拒绝 NaN 和缺失字段。
- 原 1×、2×、3×、6× 写包回归继续通过。
- 内联 JavaScript 语法检查通过。
- `cargo test --all-targets`：

```text
lib:      5 passed / 2 ignored
html_app: 33 passed / 3 ignored
main:     26 passed / 1 ignored
```

- release 构建、根目录启动与窗口响应检查通过。
- UI 截图：
  - `target/step175-ui-printwindow.png`
  - `target/step175-control.png`
- Mica 保持启用：
  - `DWMWA_SYSTEMBACKDROP_TYPE=2`
  - `DWMWA_USE_IMMERSIVE_DARK_MODE=1`
- 发布：

```text
F:/Insta360onWin/LunaStudio.exe
size: 7,693,824 bytes
SHA-256: CB8E2FFA91168D207DB38BB9B823B61D850A3F9030B8E047E0B1D73674D9BEDE
```

- `LunaVirtualCamera.dll` 未修改。
- 启动检查产生的 WebView2 缓存已删除。
- 根目录只有 `LunaStudio.exe` 一个 EXE，没有遗留项目进程。
- 验证过程没有连接相机，也没有发送真机写操作。

### 真机确认

1. 启动根目录 `LunaStudio.exe`，进入“拍摄控制”并连接相机。
2. “快捷变焦”右侧应直接显示相机返回的当前值，而不是固定 1×。
3. 点 2×，等待成功提示；显示值必须来自读回结果。
4. 切到录像模式；应用会按官方顺序真正写入 1×，随后显示读回值。
5. 依次测试 3×、6×，观察相机画面与显示值是否一致。
6. 若提示“读取实际变焦值失败”，不要连续点击；保留提示并重新连接后记录结果。

## Step 176 - Luna Ultra 官方水印目录与外框版式

### 本地 APK 证据

- `assets/apk_watermark/Image_Watermark_Config_Table.txt` 中 Luna Ultra 照片水印只有
  `Leica` 与 `Leica-CN`，并且位置固定为 `BottomCenter`。
- `assets/apk_watermark/Watermark_Config_Table.txt` 中 Luna Ultra 视频水印提供
  `TopLeft`、`TopRight`、`BottomLeft`、`BottomCenter`、`BottomRight` 五个位置。
- 照片和视频不是四种不同样式，而是同一样式对应两套资源：
  - 中文：`ic_watermark_luna_ultra_image_cn.png` / `ic_watermark_luna_ultra_cn.png`
  - 标准：`ic_watermark_luna_ultra_image.png` / `ic_watermark_luna_ultra.png`
- `reverse_apk/assets/ins_config_files/phoneApp/z03/exportSetting/business.json` 明确包含
  `is_zstyle_frame_watermark=true`。
- `Frame_Watermark_Config_Table.txt` 为 Luna Ultra 提供 `ZStyle`、`ZStyle-CN`，
  每种覆盖 15 个照片比例；表头明确给出相框宽高、照片底部距离、品牌资源宽度、
  拍摄参数字号/间距、时间戳字号及两行文字间距。

### 已修改

- `src/adapters/watermark.rs`
  - 水印目录收紧为中文、标准、外框水印（中文）、外框水印四项。
  - 中文/标准按媒体类型自动选择 APK 的照片或视频资源。
  - 普通照片固定使用 APK 的底部居中参数；普通视频按 APK 五位置表导出。
  - 修正配置表纵坐标语义：表内数值是底部距离，不是顶部坐标。
  - 视频导出改用随应用发布的 `assets/ffmpeg/ffmpeg.exe`。
  - 外框读取 `Frame_Watermark_Config_Table.txt`，品牌资源宽度以原照片宽度为基准。
- `web/index.html`
  - 删除 App 中不存在的自定义大小、透明度和无关机型样式。
  - 修复已删除控件仍被绑定而导致页面脚本中断的问题。
  - 照片/外框显示“App 固定版式”；视频才显示五位置选择。
  - 视频自动禁用仅照片可用的外框样式，并增加外框预览状态。
- `src/main.rs`
  - 删除旧入口对已移除大小、透明度参数的引用，保持全目标可编译。

### 证据边界

- 相框几何尺寸和 Logo 资源比例来自 APK 表，可直接实现。
- 表中拍摄参数字号、时间戳字号和行距已经确认，但当前项目没有 APK 对应的
  EXIF 字符串格式化与字体绘制实现；本步骤不会把缺失的文字层伪装成完整复刻。
- 后续若继续补文字层，只能使用 APK/真机导出样本确认格式、位置和字体。

### 真实渲染预览与发布收尾

- `src/adapters/watermark.rs`
  - 将普通图片水印与外框水印拆成可复用的内存像素合成函数；导出和预览调用同一实现。
  - 新增 `preview(...)`：照片按当前样式直接合成；视频通过随应用发布的 FFmpeg
    抽取实际画面，再使用视频水印参数表进行合成。
  - 预览结果按最大 `900 x 650` 等比缩放并编码为 JPEG，仅降低 UI 传输尺寸，
    不改变最终导出的原始分辨率。
  - 补充真实预览测试，确认返回的是可解码 RGB JPEG，且外框合成后的尺寸受预览边界约束。
- `src/bin/html_app.rs`
  - 新增日用 IPC 命令 `watermark_preview`，返回 `image/jpeg` 与 Base64 图像；没有暴露调试参数。
- `web/index.html`
  - 删除原来用文字、定位和 CSS 相框模拟的预览。
  - 选择素材、切换样式或调整视频位置后，自动请求后台真实渲染。
  - 增加 160 ms 防抖、加载状态和请求 ID 校验；旧请求晚返回时不会覆盖当前样式。
  - 照片、外框和视频都只展示实际合成结果；视频仍自动禁用仅照片可用的外框样式。

### 验证结果

- 内联 JavaScript 通过 `node --check`。
- `cargo test --all-targets`：

```text
lib:      5 passed / 2 ignored
html_app: 42 passed / 3 ignored
main:     35 passed / 1 ignored
```

- 已实际启动根目录 Release，在 `1196 x 819` 窗口验证：
  - 空预览状态：`target/step176-watermark-real-empty.png`
  - 普通中文水印真实合成：`target/step176-watermark-real-preview.png`
  - 外框水印真实合成：`target/step176-watermark-real-frame-preview.png`
  - 视频首帧真实合成：`target/step176-watermark-real-video-preview.png`
- 发布文件：

```text
F:/Insta360onWin/LunaStudio.exe
size: 7,810,048 bytes
SHA-256: C4AC4F6BA2CE28996647D98D47302E00BD0B3BFDFFA9D574AB0D8396C1108D2E
```

- 验证结束后已关闭项目进程；没有连接相机，也没有发送真机写操作。

## Step 177 - 按 Luna Ultra 直出原图完成外框水印

### 本步权威参考与结论

- 用户提供的直出原图：
  `C:/Users/H!Mooo/AppData/Local/Temp/codex-clipboard-91f6fdba-b130-4545-b879-fbefa5685a8e.png`。
- 图片为 `3781 x 4209`；其中原照片区域约为 `3520 x 2644`，位于
  `x=130, y=695`。
- 直出图的关键像素包围盒：
  - 顶部联名 Logo：`x=1282..2498, y=252..443`，宽 `1217`。
  - `Luna Moment`：`x=1516..2264, y=3466..3724`，宽 `749`。
  - 拍摄参数：`x=1589..2192, y=3877..3919`，宽 `604`。
  - 时间戳：`x=1450..2329, y=3993..4035`，宽 `880`。
- 右下角没有署名。署名不属于外框模板，应用不得生成。
- 直出文本格式确认为：
  - `F/2.0   1/100   ISO416`
  - `2026-Jun-18 20:08 UTC+08:00`

### APK 参数表与几何修正

- Luna Ultra `ZStyle-CN / 4:3` 参数仍来自
  `assets/apk_watermark/Frame_Watermark_Config_Table.txt`：
  `1.0740##1.5917##0.2065##0.3219##0.0138##0.0165##0.0138##0.0132##0.1983##0.0000##0.1983`。
- 直出图证明这些比例以最终相框画布为基准：
  - `3781 * 0.3219 ~= 1217`，对应顶部联名 Logo 宽度。
  - `3781 * 0.1983 ~= 750`，对应 `Luna Moment` 宽度。
  - `4209 * 0.2065 ~= 869`，对应照片到最终画布底部的距离。
- 修复旧实现把照片底距按源照片高度计算、把顶部 Logo 按源照片宽度计算的问题。
- 最终画布尺寸使用向上取整；参考样本可精确得到 `3781 x 4209`。
- 外框底色改为纯黑 `#000000`。

### 实现

- 新增资源：
  `assets/apk_watermark/choose_logo_photo_moment.png`。
  该资源从本步直出样本的 `Luna Moment` 区域无损裁取，名称对应本地 DEX 中的
  `choose_logo_photo_moment` 资源名；不包含任何署名。
- `src/adapters/watermark.rs`：
  - 预览和导出共用同一个外框像素合成器。
  - 使用 `kamadak-exif` 读取 `FNumber`、`ExposureTime`、
    `PhotographicSensitivity`、`DateTimeOriginal`、`OffsetTimeOriginal`。
  - 光圈格式为 `F/2.0`；快门分数约分；ISO 格式为 `ISO416`；时间按直出图格式化，
    且不显示秒。
  - 使用 Windows Bahnschrift 绘制元数据，Consolas/Segoe UI 作为运行时回退。
  - 根据直出样本将元数据整行横向校准为 `1.16`，不改变字号和纵向位置。
  - 缺少 EXIF 时仍渲染品牌和 `Luna Moment`，不伪造拍摄参数。
- `Cargo.toml` / `Cargo.lock` 新增 `ab_glyph 0.2.32` 与
  `kamadak-exif 0.6.1`。

### 像素校准

- QA 图：`target/step177-synthetic-frame.png`，尺寸 `1719 x 1911`。
- 对直出图按相同画布宽度归一化后：
  - `Luna Moment`：实测 `341px`，参考 `341px`。
  - 拍摄参数：实测 `278px`，参考约 `275px`。
  - 时间戳：实测 `403px`，参考约 `400px`。
- 纵向实测起点为 `1574 / 1760 / 1812`，与参考归一化位置误差约 `1-2px`。

### 验证与发布

- `cargo fmt --all -- --check` 通过。
- `cargo test --all-targets` 全部通过：

```text
lib:      5 passed / 2 ignored
html_app: 44 passed / 3 ignored
main:     37 passed / 1 ignored
```

- 根目录 Release 启动并保持运行，冒烟检查通过；随后已关闭项目进程。
- 未连接相机，也未发送真机写操作。

- 发布文件：

```text
F:/Insta360onWin/LunaStudio.exe
size: 7,992,320 bytes
SHA-256: 97ABDF8C525F456088F191588D3D3A31E154E8511F06778BD2B5881EB3BA977C
```

## Step 178 - 外框背景色与首张参考图格式（完成）

### 用户参考优先级

- 外框版式和文字格式以用户第一次提供的参考图为准。
- 光圈显示为 `F2.0`，不带斜杠。
- 时间显示为 `19-Jun-2026 12:59 UTC+08:00`，即日-英文月份-年。
- 右下角署名不是 App 水印内容，继续明确禁止生成。

### APK 本地证据

- DEX 中存在 `PhotoFrameColor(startHex=...)`、
  `availablePhotoFrameColors`、`getDefaultAutoFrameBg`、
  `backgroundColorStart` 与 `backgroundColorEnd`。
- DEX 中同时存在 `choose_logo_photo_diagram_zstyle_black` 和
  `choose_logo_photo_moment_black`，证明浅色背景会切换深色品牌素材。
- 因此日用界面按 App 模型提供黑色、白色、照片深色、照片浅色和照片渐变；
  不加入任意颜色选择器。照片色由输入照片采样，渐变使用开始/结束两种照片色。

### 当前实现

- `WatermarkOptions`、真实预览 IPC 与导出 IPC 新增 `frame_background`。
- 外框合成器支持纯色与纵向渐变，照片取色使用低成本网格采样。
- 根据背景亮度自动选择浅色/深色 Logo、Moment 和 EXIF 文字；
  Leica 红色标志保持原色。
- `choose_logo_photo_moment.png` 的黑底在运行时转换为透明蒙版，
  因而可用于所有背景颜色。
- HTML 新增 App 风格背景色板，仅在照片外框样式下显示。
- QA 图：
  - `target/step178-frame-black.png`
  - `target/step178-frame-white.png`
  - `target/step178-frame-gradient.png`
- 黑底、白底和照片渐变均完成实际像素检查；白底品牌文字、Leica 红章、
  `Luna Moment` 和 EXIF 文字正确切换为深色，红章内部保持原素材。
- 内联 JavaScript 已使用 `node --check` 验证。
- `cargo fmt --all -- --check` 通过。
- `cargo test --all-targets` 全部通过：

```text
lib:      5 passed / 2 ignored
html_app: 48 passed / 3 ignored
main:     41 passed / 1 ignored
```

- 根目录 Release 已更新：

```text
F:/Insta360onWin/LunaStudio.exe
size: 8,029,184 bytes
SHA-256: 4D02641692F0EBFE2E6A95F6F762392757C88AB856C8CBED803BDD15F272FC57
```

- 隐藏启动冒烟进入了进程启动路径；关闭阶段 Windows 返回一次进程句柄访问拒绝，
  随后按路径和进程名复查，确认没有残留项目进程。
- 未连接相机，也未发送真机写操作。

## Step 179 - 原片 EXIF 快门校正与外框字体复核（完成）

### 原片用途

- 用户提供的 `IMG_20260626_164557_021.jpg` 是无水印的 Luna Ultra 相机直出图。
- 该文件只用于验证 EXIF 读取和作为真实渲染输入，不作为外框水印版式或字体参考。

### 快门证据与修正

- 原片 `ExposureTime` 为 `1211/1000000` 秒，`ShutterSpeedValue` 为
  `9689/1000` APEX；两者描述同一次约 `1/826` 秒的曝光。
- App 应显示相机快门档位而不是把 EXIF 有理数直接约分；用户确认该原片显示为 `1/800`。
- 快门读取以 `ExposureTime` 为主，缺失时才使用 `ShutterSpeedValue`；小于一秒的值按
  对数距离吸附到相机标准快门分母。新增 `1211/1000000 -> 1/800`、
  `1/16 -> 1/16` 和 APEX 后备读取回归测试。

### 字体复核

- 当前 Bahnschrift 加横向拉伸是旧的近似实现，用户已确认不是外框水印字体。
- 本地 APK DEX 中存在 `FontLeagueGothicRegular` 与
  `font/LeagueGothic-Regular.otf`，但真实渲染过窄；进一步反查表明它属于浮动文字素材，
  不是照片外框 EXIF 参数字，已从实现中撤除。
- `libarvbmg.so` 的 `cv::putText/getTextSize` 调用点全部位于 AI 跟踪和调试可视化函数；
  `PrintWaterMark::Process` 只负责图像贴合，不绘制外框参数字，因此 OpenCV Hershey 路线也已排除。
- 当前参数字改为按用户第一张水印参考图逐字形比对得到的方角 SemiBold 字体；
  它是参考图拟合用的 OFL 字体，不冒充 APK 内置资源。字体与许可保存在
  `assets/apk_watermark/FrameMetadata-SemiBold.ttf` 和 `FrameMetadata-OFL.md`。
- 使用用户无水印原片完成真实合成：`target/step179-original-frame.png`，
  底部参数为 `F2.0  1/800  ISO161`。

### 验证与发布

- `cargo fmt --all -- --check` 通过。
- `cargo test --all-targets` 全部通过：`5 passed / 2 ignored`、
  `48 passed / 4 ignored`、`41 passed / 2 ignored`。
- 已删除错误的 League Gothic 文件、候选字体仓库和候选联系表，只保留最终字体、许可、
  真实原片 QA 与参考图局部。
- 根目录 Release 已更新：

```text
F:/Insta360onWin/LunaStudio.exe
size: 8,090,112 bytes
SHA-256: 611018F24B892541C8C94BC8556CDAE4A0EE32BEC2235BA40FEAB65F33205333
```

- 发布后的隐藏启动冒烟由 Windows 返回一次“拒绝访问”；复查确认没有
  `LunaStudio` 或 `html_app` 进程残留，因此本步不把启动冒烟标记为通过。
- 本步未连接相机，也未发送真机写操作。
