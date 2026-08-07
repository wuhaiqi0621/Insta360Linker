# Reverse Continue Log

This file is the handoff log for continuing the APK-only Luna Ultra reverse engineering work.

## 2026-07-05

### Step 1 - Create persistent handoff log

User requested every step to be written into a Markdown file so another model can continue development.

Files:

- Main evidence notes: `F:\Insta360onWin\reverse_apk\ASHIELD_UNPACK_NOTES.md`
- This step-by-step handoff log: `F:\Insta360onWin\reverse_apk\REVERSE_CONTINUE_LOG.md`

Rule for future work:

- Append every meaningful step here.
- Keep `ASHIELD_UNPACK_NOTES.md` as the compact evidence/facts file.
- Keep files UTF-8 with CRLF line endings.

### Step 2 - Build segment keyword index

Scanned the 17 `adx0` DEX map-list segments and counted protocol/camera keywords per segment.

Important segment conclusions:

- Segment `5` (`bodyrel 0x02bb4a2b..0x0312e1d9`) is the `insta360/messages` protobuf segment.
  - Contains `INS_UCDTYPE_*`.
  - Contains message classes such as `TakePicture`, `TakePictureResponse`, `StopCapture`, `StopCaptureResp`, `CheckAuthorization`, `EncryptKeyExchangeReq`, `EncryptKeyExchangeResp`, `FastDownload*`, `FileMeta`, `Options`.
- Segment `10` (`bodyrel 0x0482ff6b..0x04fa1095`) contains camera command classes.
  - Important strings include `Lcom/arashivision/camera/command/CheckAuthorizationCmd;`, `HdrStopCaptureCmd`, `StopTakePictureCommand`.
  - It also contains `Linsta360/messages/TakePicture;`, `Linsta360/messages/StopCapture;`, `access$isUseUcd2$p`, `setUcd2`, `isUcd2`, `takePicture`, `stopTakePicture`.
- Segment `11` (`bodyrel 0x04fa1095..0x056641cf`) contains base camera/controller business logic.
  - Contains `UCD2-XOR-KEY-001`.
  - Contains `UCD2.0 don't need sync-packet`.
  - Contains `CameraOptionsManager`, `BaseCameraController`, authorization callbacks, and `EncryptKeyExchangeResp`.

Current direction:

- Extract all `com/arashivision/camera/command/*` class names from segment `10`.
- Use command class names plus protobuf message names to infer control commands before full DEX reconstruction.

### Step 3 - Extract camera command class index

Scanned protected segment `10` (`bodyrel 0x0482ff6b..0x04fa1095`) for `com/arashivision/camera/command/*`, `onecamera`, and `onedriver/packet` strings.

High-value command classes found:

- Session/auth/info: `InstaCmdExe`, `CheckAuthorizationCmd`, `RequestAuthorizationCmd`, `CancelAuthorizationCmd`, `CancelRequestAuthorizationCmd`, `SetAppIdCmd`, `SetPhoneInfoCmd`, `GetFirmwareVersionInfoCmd`, `GetFlowStateCmd`, `SetFlowStateCmd`.
- Capture control: `CaptureCommand`, `StartRecordCmd`, `StopRecordCmd`, `PauseRecordCmd`, `CaptureWithoutStorageCmd`, `HdrCaptureCommand`, `HdrStopCaptureCmd`, `StopTakePictureCommand`, `StartBulletTimeCmd`, `StopBulletTimeCmd`, `StartTimeplaseCmd`, `StopTimelapseCmd`, `StartTimeshiftCmd`, `StopTimeShift`.
- Preview/live/stream: `PreviewStreamingCmd`, `StopStreamingCmd`, `PrepareCameraLiveCmd`, `StartCameraLiveCmd`, `SetCameraLiveOptionsAsyncCmd`, `StopCameraLiveCmd`, `StopLiveCmd`, `RequestStreamIframeCmd`.
- File/media: `GetFileListCmd`, `GetFileListIncludeRecordingCmd`, `GetDownloadFileListCmd`, `notifyGetDownloadFileListResultCmd`, `GetFileInfoListCmd`, `GetFileCmd`, `GetFileExtraCmd`, `SetFileExtraCmd`, `DeleteFileListCmd`, `SetFavoriteListCmd`, `UpdateDownloadInfoCmd`, `GetFileFinishCmd`.
- Options/settings: `GetOptionCmd`, `GetOptionSyncCmd`, `SetOptionAsyncCmd`, `GetPhotoOptionCmd`, `GetPhotoOptionsAsyncCmd`, `SetPhotoOptionsCmd`, `SetPhotoOptionsSyncCmd`, `SetTimelapseOptionCmd`, `AyncSetTimelapseOptionCmd`, `SetPreviewEncodeCmd`, `SetTransferStatusCmd`.
- Wi-Fi/USB/BLE: `OpenCameraWifiCmd`, `CloseCameraWifiCmd`, `ResetCameraWifiCmd`, `SetCameraWifiSeizeEnableCmd`, `GetWifi*`, `SetWifi*`, `OpenUsbCmd`, `BleConnectCmd`, `BleScanCmd`, `BleWakeUpCmd`, `ConnectBTCmd`, `DisConnectBTCmd`, `GetConnectBTCmd`, `ScanBTCmd`.

Packet/driver classes found:

- `Lcom/arashivision/onecamera/OneDriver;`
- `Lcom/arashivision/onecamera/OneDriverInfo;`
- `Lcom/arashivision/onecamera/OneDriverInfo$Response;`
- `Lcom/arashivision/onecamera/OneDriverInfo$Notification$TakePictureState;`
- `Lcom/arashivision/onedriver/packet/BufferStream;`
- `Lcom/arashivision/onedriver/packet/Go2BlePacket;`
- `Lcom/arashivision/onedriver/packet/Packet;`
- `Lcom/arashivision/onedriver/packet/PacketDemuxer;`
- `Lcom/arashivision/onedriver/packet/PacketType;`

Next direction:

- Inspect string neighborhoods around `Packet`, `PacketDemuxer`, `PacketType`, `InstaCmdExe`, `CaptureCommand`, `StartRecordCmd`, `StopRecordCmd`, `StopTakePictureCommand`, `GetFileListCmd`, and `CheckAuthorizationCmd`.
- Try to identify method names and constructor field names (`cmd type`, payload builder, response parser) before attempting full DEX reconstruction.

### Step 4 - Map command layer to protobuf message layer

Scanned protected segments `5`, `10`, and `11` for command/message relationships.

Important findings:

- `Segment 10` contains OneDriver execution signatures:
  - `(Lcom/arashivision/onecamera/OneDriver;J[Lcom/arashivision/camera/command/InstaCmdExe;)V`
  - `(Lcom/arashivision/onecamera/OneDriver;J[Lcom/arashivision/camera/command/InstaCmdExe;Lkotlin/coroutines/Continuation;)Ljava/lang/Object;`
  - `(Lcom/arashivision/onecamera/OneDriver;Lcom/arashivision/camera/command/InstaCmdExe;IJLkotlin/coroutines/Continuation;)Ljava/lang/Object;`
- Command wrappers point at protobuf payload classes:
  - `CheckAuthorizationCmd` uses `Linsta360/messages/CheckAuthorization;` plus `Lcom/arashivision/camera/RequestOptions;`.
  - `GetFileListCmd` uses `Linsta360/messages/GetFileList;`.
  - `StopTakePictureCommand` / capture commands reference `Linsta360/messages/TakePicture;` and `Linsta360/messages/StopCapture;`.
- `Segment 5` contains the actual Wire/protobuf message classes and constructors:
  - `CheckAuthorization`: `(Ljava/lang/String;Linsta360/messages/CheckAuthorization$InitiatorType;Ljava/lang/String;Lokio/ByteString;)V`
  - `CheckAuthorizationResp`: `(Linsta360/messages/CheckAuthorizationResp$AuthorizationStatus;Linsta360/messages/CheckAuthorizationResp$FindmyPairStatus;ILjava/lang/String;Lokio/ByteString;)V`
  - `TakePicture`: `(Linsta360/messages/TakePicture$Mode;Linsta360/messages/ExtraMetadata;Ljava/util/List;Linsta360/messages/RawCaptureType;ZLinsta360/messages/SensorDevice;ILinsta360/messages/TriggerSource;Lokio/ByteString;)V`
  - `StopCapture`, `StopCaptureResp`, `GetFileList`, `GetFileListResp`, `StartCapture`, and `StartCaptureResp` are present.
- `Segment 11` contains UCD2/packet/encryption implementation names:
  - `UCD2-XOR-KEY-001`
  - `UCD2.0 don't need sync-packet`
  - `ucd2`, `ucd2Magic`, `ucd2Sequence`
  - `Lcom/arashivision/onedriver/encrypt/AesGcmResult;`
  - `Lcom/arashivision/onedriver/encrypt/EcdhKeyPair;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptResult;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptionManager;`
  - `Lcom/arashivision/onedriver/packet/PacketDecryptParams;`
  - `Lcom/arashivision/onedriver/packet/PacketEncryptResult;`
  - `Lcom/arashivision/onedriver/packet/PacketEncryptionParams;`

Conclusion:

- Camera control should not be guessed as simple numeric OSC-like commands.
- The APK architecture appears to be: `BaseCamera/Controller -> OneDriver -> InstaCmdExe[] -> camera/command/*Cmd -> insta360/messages/* protobuf -> ondriver/packet/Packet -> UCD2 transport/encryption`.
- Generating new commands still requires reconstructing the `Packet` / `EncryptionManager` tail and encryption path, or replaying APK-derived/captured packets.

### Step 5 - Fix app-side UCD2 hint text encoding

Touched `F:\Insta360onWin\src\adapters\luna_local.rs`.

Changed:

- Replaced mojibake Chinese in `message_hint(...)` with Rust Unicode escape strings so the source remains safe UTF-8/ASCII and runtime JSON displays Chinese correctly.
- Ran `cargo fmt --all`.
- Ran `cargo check`.

Verification:

- `cargo check` passed.
- Existing warnings are unused-code warnings for older OSC/probe paths; no build errors.

### Step 6 - Update app UI for APK-only camera control workflow

Touched:

- `F:\Insta360onWin\web\index.html`
- `F:\Insta360onWin\src\bin\html_app.rs`

Changed:

- Rebuilt the HTML UI in Chinese.
- Added a dedicated `鐩告満鎺у埗` panel.
- Kept `璇诲彇璁惧淇℃伅` as the only direct safe control action because its `04 10` packet is already verified by APK/evidence and device responses.
- Added visible placeholders for `鎷嶇収`, `寮€濮嬪綍鍍廯, `鍋滄褰曞儚`, `鍋滄鎷嶇収`, and `鐩稿唽鍛戒护`.
  - These buttons do not send unsafe guessed packets.
  - They show the APK class/message evidence and `绛夊緟 Packet/EncryptionManager` status.
- Reworked UCD2 raw probing copy to clearly say empty input will not send.
- Fixed Rust-side window title and error strings using Rust Unicode escapes so runtime Chinese displays correctly while source stays safe.

Verification:

- Ran `cargo fmt --all`.
- Ran `cargo check`.
- `cargo check` passed; remaining warnings are unused-code warnings from older OSC/probe modules.
- Ran `cargo build --release --bin html_app`.
- Release build succeeded: `F:\Insta360onWin\target\release\html_app.exe`.
- Checked `web/index.html`, `src/bin/html_app.rs`, and this log for `???` / replacement-character residue; none found.

Next:

- Continue APK-only reverse engineering of `com/arashivision/onedriver/packet/Packet`, `PacketEncryptionParams`, `PacketEncryptResult`, and `EncryptionManager`.
- Once Packet tail/encryption generation is known, wire the existing `鐩告満鎺у埗` placeholders to real UCD2 packet builders.

## Step 7 - Packet and EncryptionManager string map

Scope: APK-only reverse work. No public protocol material used.

Segment 11 contains the main packet/encryption layer:

- `Lcom/arashivision/onedriver/packet/Packet;`
- `Lcom/arashivision/onedriver/packet/Packet$Companion;`
- `Lcom/arashivision/onedriver/packet/PacketDemuxer;`
- `Lcom/arashivision/onedriver/packet/PacketEncryptResult;`
- `Lcom/arashivision/onedriver/packet/PacketEncryptionParams;`
- `Lcom/arashivision/onedriver/packet/PacketDecryptParams;`
- `Lcom/arashivision/onedriver/packet/Message;`
- `Lcom/arashivision/onedriver/packet/MessageMuxer;`
- `Lcom/arashivision/onedriver/packet/MessageDemuxer;`
- `Lcom/arashivision/onedriver/encrypt/EncryptionManager;`
- `Lcom/arashivision/onedriver/encrypt/PlatformCrypto;`
- `Lcom/arashivision/onedriver/encrypt/AesGcmResult;`
- `Lcom/arashivision/onedriver/encrypt/EcdhKeyPair;`
- `Lcom/arashivision/onedriver/encrypt/EncryptResult;`

Packet/encryption evidence strings found in segment 11:

- `PacketDecryptParams(decrypt=`
- `PacketEncryptResult(ciphertext=`
- `PacketEncryptionParams(scheme=`
- `AesGcmResult(ciphertext=`
- `EncryptResult(ciphertext=`
- `EcdhKeyPair(publicKey=` / `, privateKey=`
- `nonceHex=` / `tagHex=` / `aadHex=`
- `decrypt[scheme=`
- `fromBytes: encrypted frame but decryptParams is null, scheme=`
- `fromBytes: decrypt failed, scheme=`
- `parseEncryptOption: unknown scheme=`
- `encrypt: cleared`
- `$encrypt: encrypting message, method=`
- `negotiateEncryption` / `negotiateEncryption result=`

Encryption schemes/options observed in APK strings:

- `ENCRYPT_SCHEME_NONE`
- `ENCRYPT_SCHEME_XOR`
- `ENCRYPT_SCHEME_AES_GCM_128`
- `ENCRYPT_OPTION_DATA_LEN_XOR`
- `ENCRYPT_OPTION_DATA_LEN_AES_GCM`
- `XOR_KEY`
- `UCD2-XOR-KEY-001`

Segment 5 contains the protobuf-side encryption negotiation messages:

- `EncryptCapabilityQuery{}`
- `EncryptCapabilityResp{`
- `EncryptKeyExchangeReq{`
- `EncryptKeyExchangeResp{`
- `Linsta360/messages/EncryptScheme;`
- `Linsta360/messages/EncryptDevType;`
- `Linsta360/messages/EncryptErrorCode;`
- constructor descriptors `(Linsta360/messages/EncryptScheme;Lokio/ByteString;Lokio/ByteString;)V` and `(Linsta360/messages/EncryptErrorCode;Lokio/ByteString;Lokio/ByteString;)V`

Current conclusion:

- UCD2 frames are only the transport wrapper. Real camera control needs the inner `Packet -> Message -> insta360/messages/*Cmd` layer.
- The app supports unencrypted, XOR, and AES-GCM packet schemes. The LunaU session appears to expose heartbeat/control frames immediately after the APK-derived handshake, but arbitrary control commands should not be sent until Packet construction and encryption-option parsing are implemented from APK evidence.
- Next target: recover numeric enum values and byte layout for `Packet`, `PacketEncryptionParams`, and `Message` from segment 11, then wire the Windows app to generate only APK-derived packets.
## Step 8 - App UI refresh for Packet/EncryptionManager checkpoint

Updated `web/index.html` as UTF-8 with CRLF because the previous HTML text had replacement/garbled Chinese characters inside the app, not only terminal display issues.

UI changes:

- Rebuilt the HTML interface with proper Chinese labels.
- Kept the layout bounded with responsive grids and internal vertical scrolling, avoiding horizontal overflow.
- Added a visible `Packet / EncryptionManager` card showing the current APK-only evidence:
  - Packet classes: `Packet`, `Packet$Companion`, `PacketEncryptResult`, `PacketEncryptionParams`, `PacketDecryptParams`.
  - Encryption schemes: `ENCRYPT_SCHEME_NONE`, `ENCRYPT_SCHEME_XOR`, `ENCRYPT_SCHEME_AES_GCM_128`.
  - Negotiation messages: `EncryptCapabilityQuery/Resp`, `EncryptKeyExchangeReq/Resp`, `UCD2-XOR-KEY-001`.
- Camera control buttons still do not send guessed packets. They show APK class/message evidence and remain blocked until `Packet / EncryptionManager` byte layout is recovered.
- Restored the Rust WebView callback bridge as `window.LunaBridge.receive(...)`.

Verification:

- UTF-8 decode has zero replacement characters.
- CRLF byte check passed.
- `cargo check --bin html_app` passed with only pre-existing dead-code warnings.
## Step 9 - Encoding hardening for app Chinese text

The first UTF-8 rewrite still passed through a shell/stdin path that converted literal Chinese into `?` characters. Rewrote `web/index.html` again as an ASCII-only HTML/JS source: Chinese UI text is emitted at runtime through JavaScript `\uXXXX` escapes.

Why this matters:

- The source file is valid UTF-8 with CRLF and contains no non-ASCII text that can be damaged by PowerShell code page conversion.
- The app still renders Chinese in the WebView because the browser decodes JS Unicode escapes.
- `window.LunaBridge.receive(...)` is preserved for Rust -> WebView responses.

Verification after hardening:

- UTF-8 decode succeeded.
- Direct non-ASCII source character count is zero by design.
- CRLF byte check passed.
- `cargo check --bin html_app` passed with only pre-existing dead-code warnings.
## Step 10 - Ashield native unpack path checkpoint

Scope: APK-only reverse work. No public protocol material used.

Tool state on this machine:

- `java`, `jadx`, `apktool`, `baksmali`, `objdump`, `readelf`, `nm`, and `strings` were not found in PATH.
- Python is available.
- Python packages `pyelftools` and `capstone` are available, so local ELF + AArch64 disassembly is possible.

`classes.dex` Ashield header:

- Stub DEX is a normal small DEX wrapper.
- Appended container marker: `adx0` at file offset `0x18e4`.
- `adx0` header: magic `adx0`, total appended size `0x072abb5b`, metadata size `0x018c`.
- Metadata bytes are XOR `0x52` encoded.
- Decoded records:
  - `activityName = com.arashivision.insta360akiko.app.SplashActivity`
  - `appName = com.arashivision.insta360akiko.app.AkikoApplication`
  - `fastLevel = 1`
  - `jiaguVersion = 1.5.1.3`
  - `ls = 6372`
  - `opt = 1`
  - `pkg = com.arashivision.insta360akiko`
  - `protectTime = 0`
  - `stubAppName = com/ashield/Stub`
  - `versionCode = 418696`
  - `versionName = 2.28.0`
- Business body begins at `0x1a7c` (`adx + 12 + 0x18c`). First little-endian u32 is `0x11`, matching 17 protected DEX/container sections.

`libashield.so` native evidence:

- ELF64 AArch64, entry `0xe150`.
- Important sections:
  - `.plt` at `0xdb90`, size `0x5c0`
  - `.text` at `0xe150`, size `0xc02cc`
  - `.rodata` at `0xce420`, size `0x6081`
- Needed libs include `libz.so`, confirming zlib path.
- Dynamic symbol `JNI_OnLoad` at `0xe300`.
- Important PLT imports:
  - `lseek64` at `0xdbd0`
  - `open` at `0xdc30`
  - `inflate` at `0xdc50`
  - `inflateInit2_` at `0xdcd0`
  - `read` at `0xdd10`
  - `inflateEnd` at `0xdf20`
  - `mmap` at `0xdf80`

Confirmed unpack/decompress region:

- Around `0x4f500..0x4f930` is the raw-deflate wrapper.
- `0x4f778 -> inflateInit2_`; immediately before it sets `w1 = -15`, proving raw deflate mode.
- `0x4f7f8 -> inflate`.
- `0x4f8c8 -> inflateEnd`.
- `0x4f75c -> 0x4c734` appears to call an internal helper before zlib init.

File/container IO region:

- `0x4c298 -> open`
- `0x4c384 -> lseek64`
- `0x4caec -> read`
- `0x4cc1c -> read`
- `0x4e880 -> lseek64`
- `0x4e934 -> read`
- `0x4ea8c -> lseek64`
- `0x4b644 -> mmap`

Local helper added:

- `reverse_apk/tools/ashield_native_scan.py`
- It parses `libashield.so` with `pyelftools`, disassembles with `capstone`, lists PLT imports, imported calls, and calls inside the `0x4b000..0x50000` unpack cluster.

Current conclusion:

- The protected business DEX sections are not directly stored as complete DEX files. Clear DEX `map_list` and strings survive, but section bodies are transformed/reordered by Ashield.
- The static unpack path is now narrowed to the native cluster around `0x4b000..0x4f930`.
- Next target: reverse call chain `0x4f404 -> 0x4f504` and helpers `0x4edb8`, `0x4ef7c`, `0x4dff4`, `0x4dc00`, `0x4c734` to recover the per-section header/decrypt/deflate input format.
## Step 11 - Inflate wrapper and block-unpack state machine

Scope: APK-only reverse work. No public protocol material used.

Generated detailed AArch64 disassembly for the unpack cluster:

- `reverse_apk/tools/ashield_unpack_cluster_disasm.txt`
- Ranges included: `0x4b000..0x50040`, `0x81800..0x81b40`, `0xa0500..0xa0850`.

Confirmed function signatures / roles:

### `0x4f504` - raw inflate wrapper

Call observed at `0x4f404`:

```text
x0 = output pointer
x1 = input pointer
x2 = output length
x3 = input length
bl 0x4f504
```

Inside `0x4f504`, the function builds a zlib `z_stream`-like struct:

- `[stream + 0x00] = x1` -> `next_in`
- `[stream + 0x08] = w3` -> `avail_in`
- `[stream + 0x18] = x0` -> `next_out`
- `[stream + 0x20] = w2` -> `avail_out`

Then:

- `0x4f778 -> inflateInit2_` with `w1 = -15`, proving raw deflate mode.
- `0x4f7f8 -> inflate` with flush argument `4`.
- `0x4f8c8 -> inflateEnd`.

Current signature:

```text
bool ashield_raw_inflate(void* out_ptr, void* in_ptr, uint32_t out_len, uint32_t in_len)
```

### `0x4ef7c` - upper block unpack state machine

This function stores initial arguments:

- `x0 -> [x29 - 0xa8]`
- `x1 -> [x29 - 0xa0]`
- `x2 -> [x29 - 0x98]`

It later reaches the inflate call at `0x4f404` using:

- `x0 = [x29 - 0x68]`
- `x1 = [x29 - 0x80]`
- `x2 = [x29 - 0x28]`
- `x3 = [x29 - 0x30]`

So the state machine prepares output pointer, compressed input pointer, output length, and compressed input length before calling `0x4f504`.

Important branches / helpers in this state machine:

- `0x4f280 -> 0x4dff4`: parses a header/descriptor and writes multiple output fields to stack pointers.
- `0x4f2d4 -> 0x4ee24`: builds an intermediate object/descriptor from source data.
- `0x4f344 -> 0x4d6ec`: likely reads length/field from the intermediate object.
- `0x4f3ac -> memcpy`: copies decoded data when the block is already clear or after a previous transform.
- `0x4f404 -> 0x4f504`: raw inflate path.
- `0x4f460 -> 0x4b8d8` and `0x4f490 -> 0x4ba08`: cleanup/free paths.

### `0x4d810` / `0x4d87c`

Confirmed little-endian integer helpers:

- `0x4d810(ptr)` reads a little-endian 32-bit integer from four bytes.
- `0x4d87c(ptr)` reads a little-endian 16-bit integer from two bytes.

Current conclusion:

- Ashield block unpacking is now narrowed to a clear chain:

```text
block input
  -> 0x4dff4 header/descriptor parser
  -> optional descriptor/object build via 0x4ee24
  -> optional memcpy or raw deflate
  -> 0x4f504 raw inflate(out, in, out_len, in_len)
```

- Next target: reverse `0x4dff4` fully. It likely identifies per-block mode, compressed offset, compressed length, and uncompressed length. Once those fields are recovered, a Python unpacker for the protected DEX sections should be possible.
## Step 12 - `0x4dff4` identified as APK ZIP entry parser

Scope: APK-only reverse work. No public protocol material used.

`0x4dff4` was inspected with data-flow focus on writes to caller-provided output pointers.

Important discovery:

- `0x4dff4` is not the inner Ashield block parser yet. It is primarily parsing APK ZIP central-directory / local-file-header metadata.

Evidence from the function:

- It receives many pointer arguments and stores them as:
  - `x0 -> [sp + 0x1b0]`
  - `x1 -> [sp + 0x1b8]`
  - `x2 -> [sp + 0x1c0]`
  - `x3 -> [sp + 0x1c8]`
  - `x4 -> [sp + 0x1d0]`
  - `x5 -> [sp + 0x1d8]`
  - `x6 -> [sp + 0x1e0]`
  - `x7 -> [sp + 0x1e8]`
- It selects an entry descriptor from a table/vector at `entry_table + index * 16` and then backs up `0x2e` bytes:
  - `entry = table[index]`
  - `central = entry - 0x2e`
- It reads little-endian fields from central-directory-like offsets:
  - `central + 0x0a` via `0x4d87c` -> compression method candidate
  - `central + 0x0c` via `0x4d810`
  - `central + 0x10` via `0x4d810`
  - `central + 0x14` via `0x4d810` -> compressed size candidate
  - `central + 0x18` via `0x4d810` -> uncompressed size candidate
  - `central + 0x2a` via `0x4d810` -> local header offset candidate
- It then seeks to the local header offset:
  - `0x4e880 -> lseek64(fd, local_header_offset, SEEK_SET)`
- It reads `0x1e` bytes into a stack buffer:
  - `0x4e934 -> read(fd, stack_header, 0x1e)`
- It checks the local file header magic:
  - `0x4ea44 -> 0x4d810(stack_header)`
  - compares against `0x04034b50`

Interpretation:

- `0x4dff4` resolves an APK ZIP entry into data-position/size/method information.
- The raw-deflate wrapper `0x4f504` is used by this ZIP layer when an APK entry is deflated.
- This explains the raw `inflateInit2_(-15)` path: ZIP entries use raw deflate streams.
- Therefore the current unpack chain is two-layered:

```text
APK ZIP entry parser/decompressor
  -> extracted classes.dex bytes
  -> classes.dex `adx0` Ashield container at offset 0x18e4
  -> Ashield protected DEX section transform still to recover
```

Important cross-check:

- `adx0` metadata key `ls = 6372` equals `0x18e4`, the `adx0` offset inside extracted `classes.dex`.

Current conclusion:

- The APK ZIP layer is now understood enough and is probably not the blocker anymore because `reverse_apk/classes.dex` is already extracted.
- Next target should move past the ZIP loader and focus on code that consumes the `adx0` body after `ls=6372`, especially routines that iterate the first body u32 `0x11` sections and transform them into runtime DEX data.
## Step 13 - ZIP verification and DEX map-list breakthrough

Scope: APK-only reverse work. No public protocol material used.

ZIP verification against the original APK:

- APK size: `1588657289` bytes.
- `classes.dex` is a deflated ZIP entry: method `8`, uncompressed `120247368`, compressed `51167112`, local header `0x51fbcf57`, data offset `0x51fbcf80`.
- `lib/arm64-v8a/libashield.so` is also deflated: uncompressed `972616`, compressed `437579`, local header `0x5ad94b09`, data offset `0x5ad94b42`.
- `assets/ashield.png` does not exist in the standard ZIP central directory and the raw APK does not contain the ASCII name `ashield.png`. Native code still has the string `assets/ashield.png`, so that path is likely a fallback/legacy path or built for another packaging mode.

Important correction:

- The 17 previously tracked values are not just random section starts. Their absolute positions inside the `adx0` body contain valid DEX `map_list` records.
- Example at body-relative `0x008bd1bf` starts with `12 00 00 00`, then standard DEX map items: `type=0000`, `0001`, `0002`, `0003`, `0004`, `0005`, `0006`, `2001`, `2003`, `1001`, `2002`, `2004`, `2000`, `2005`, `1003`, `1002`, `2006`, `1000`.
- Similar map lists were confirmed at all 17 tracked body-relative offsets.
- Direct DEX magic strings (`dex\n035`, `dex\n037`, `dex\n038`, `dex\n039`) are not present in the `adx0` body, so headers are removed/obfuscated even though many DEX data sections and strings are clear.

Observed clear strings inside the `adx0` body include:

- `Packet` around body-relative `0x00549dfa` and nearby offsets.
- `EncryptionManager` around body-relative `0x00549d81`, `0x03fe934d`, `0x054025e2`, `0x06f228cf`.
- `Lcom/arashivision/onedriver/packet/Packet;` around body-relative `0x00549e0d`, `0x04d25041`, `0x05402d50`.
- `insta360/messages` around body-relative `0x0054b24b` and later mirrored regions.

Working interpretation:

```text
adx0 body
  -> first u32 = 17
  -> protected/obfuscated header or index region
  -> clear/semi-clear DEX section data
  -> 17 valid DEX map_list regions
  -> DEX headers/magic must be reconstructed or decrypted
```

Next target:

- Reconstruct each DEX start and header from the map_list records and string_id/string_data relationships.
- If a full DEX can be rebuilt, decompile or parse the classes around `Packet`, `EncryptionManager`, `MessageMuxer`, and `MessageDemuxer` to recover UCD2 packet/encryption byte layout.

## Step 14 - Stub JNI entry and `ashieldStub4` native path

Scope: APK-only reverse work. No public protocol material used.

Stub DEX facts:

- The extracted `classes.dex` before `adx0` is a small valid DEX stub.
- It contains one class: `Lcom/ashield/Stub;`.
- Native methods in the stub class:
  - `ashieldStub1` private static native
  - `ashieldStub2` private static native
  - `ashieldStub3` private static native
  - `ashieldStub4` private static native
  - `ashieldStub5` private static native
  - `ashieldStub7` public static native
  - `ashieldStub8` public static native
- `ashieldStub6` is not native; it is a public static bytecode wrapper.

Native registration facts from `libashield.so`:

- A static `JNINativeMethod` table exists at VA `0xfce80`:
  - name pointer `0xd2050` -> `ashieldStub4`
  - signature pointer `0xd205d` -> starts with `(Landroid/content/res/AssetManager;Ljava/lang/String;Ljava/lang/String;...`
  - function pointer `0xa0374`
- Therefore `0xa0374` is confirmed as the native implementation of `com.ashield.Stub.ashieldStub4`.

`ashieldStub4` behavior from disassembly:

- JNI args are saved as:
  - `x0` -> `JNIEnv*`
  - `x2` -> Java `AssetManager` argument
  - `x3` -> Java `String` argument converted through JNIEnv vtable offset `0x548`
  - `x4` -> Java `String` argument converted through JNIEnv vtable offsets `0x540`/`0x548`
- It opens one converted path at `0xa054c -> open(path, 0)`.
- It reads 4 bytes at `0xa0568 -> read(fd, stack+0xbc, 4)`.
- It compares the first 2 bytes with rodata `0xd212a -> "PK"`.
- If the path is an APK/ZIP, it initializes the ZIP parser object at stack `sp+0xd0` and calls the previously identified ZIP helpers:
  - `0xa0658 -> 0x4bdd4`
  - `0xa07b8 -> 0x4dc00` with rodata `0xd20c5 -> assets/ashield.png`
  - `0xa07e4 -> 0x4dff4` ZIP entry resolver
  - `0xa07f8 -> 0x4edb8`
  - `0xa0824 -> 0x4ef7c` ZIP raw-deflate extract
- The original APK does not contain `assets/ashield.png`, so this path may be a fallback/legacy resource path or may be unused in this APK's packaging mode.

Current meaning:

- We now have the Java -> native entry and confirmed the APK ZIP extraction branch.
- This still does not directly unpack the `adx0` protected DEX body. The next native targets are the later calls from `JNI_OnLoad` after registration, especially `0x82a38`, `0x378e0`, `0x93620`, and `0x97084`, because they decide the loader mode and build the runtime loader object after native registration succeeds.

## Step 15 - `0x82780` string decoder recovered

Scope: APK-only reverse work. No public protocol material used.

`0x82780` is a string decoder used heavily by the loader mode detector at `0x82a38`. It contains anti-linear-disassembly junk: the function entry jumps over embedded invalid/SVE-looking bytes, so useful disassembly starts at `0x827c8` and later blocks such as `0x82860` / `0x828dc`.

Recovered core behavior:

```text
real_out = pseudo_out + 0xffffffff809b57fc
real_src = pseudo_src + 0xffffffff809b57fc
for i in 0..len-1:
    real_out[i] = ((real_src[i] ^ 0x6d) + i + 0x25 - key) & 0xff
real_out[len] = key
return real_out
```

Confirmed decoded strings from `0x82a38`:

- `java/lang/System`
- `getProperty`
- `(Ljava/lang/String;)Ljava/lang/String;`
- `java.vm.version`
- `libdl.so`
- `libdvm.so`
- `libart.so`
- `ro.yunos.vm.name`
- `AOC`

Interpretation:

- `0x82a38` is mostly runtime/VM mode detection. It queries `System.getProperty("java.vm.version")`, probes runtime libraries such as `libdvm.so` / `libart.so`, and checks YunOS/AOC markers.
- The return value from `0x82a38` is a loader/runtime mode selector used by `JNI_OnLoad` to choose which C++ loader object is constructed (`0x93620` for one mode, `0x97084` for another).

Next target:

- Run the recovered decoder over all `0x82780` and `0x7f580` call sites to reveal hidden class/method/property names.
- Prioritize decoded strings near `0x378e0`, `0x93620`, `0x97084`, and any functions that mention `classes.dex`, `sourceDir`, `ApplicationInfo`, `DexFile`, or `BaseDexClassLoader`, because those are likely to lead toward the protected DEX reconstruction path.

## Step 16 - `e660` decoder and JNI registration clarified

Scope: APK-only reverse work. No public protocol material used.

A second string decoder was recovered at `0xe660`. Like `0x82780`, it contains embedded anti-linear-disassembly bytes and useful blocks begin after jumps such as `0xe700` / `0xe764` / `0xe808`.

Recovered core behavior:

```text
real_out = pseudo_out + 0xffffffff9fde43f6
real_src = pseudo_src + 0xffffffff9fde43f6
for i in 0..len-1:
    t = (real_src[i] - 0x69) & 0xff
    real_out[i] = ((t ^ 0xad) - i - key) & 0xff
real_out[len] = key
return real_out
```

Confirmed decode from `JNI_OnLoad`:

- `0xf4810 / 0xf4818`, len `0x11`, key `0x08` -> `com/ashield/Stub`.
- `0xf4820 / 0xf4828`, len `0x11`, key `0x08` -> `com/ashield/Stub`.

`JNI_OnLoad` flow clarified:

- `0xe390 -> 0xe900` uses the decoded `com/ashield/Stub` class name.
- `0xe3e8 -> 0xa02a0` also uses decoded `com/ashield/Stub`.
- `0xa02a0` calls JNIEnv vtable offset `0x6b8`, consistent with `RegisterNatives`.
- The native method table passed by `0xa02a0` is still the static table at `0xfce80`, count `1`:
  - name `ashieldStub4`
  - signature `(Landroid/content/res/AssetManager;Ljava/lang/String;Ljava/lang/String;)I`
  - function `0xa0374`

Stub bytecode facts added:

- `ashieldStub4` signature from DEX is exactly `int ashieldStub4(AssetManager, String, String)`.
- `onCreate()` calls `ashieldStub4` in the assets-enabled branch after `getAssets()` and `context.getFilesDir().getAbsolutePath()`.
- `attachBaseContext()` loads `ashield` and calls `ashieldStub5(Context): Application` only on the has-app branch.

Current open question:

- Other native methods (`ashieldStub1/2/3/5/7/8`) are not present as plain strings or dynamic symbols and are not in the one-entry `RegisterNatives` table. They may be resolved by a later mutation/hook path, by loader-specific runtime patching, or by name generation not yet decoded.
- Continue by scanning decoded native strings and loader object constructors (`0x93620`, `0x97084`, `0x378e0`) for references to Application creation, DEX loading, and protected `adx0` reconstruction.

## Step 17 - Decode native strings and locate DEX injection layer

Scope: APK-only reverse work. No public protocol material used.

Added reusable decoder tool:

- `F:\Insta360onWin\reverse_apk\tools\ashield_decode_strings.py`

The tool currently implements the two confirmed native string decoders:

- `0x1a204`
- `0xe660`

Important decoded results:

- `0x1d748` (`ashieldStub5`) decodes and uses:
  - `appName`
  - `java/lang/Class`
  - `getClassLoader`
  - `java/lang/ClassLoader`
  - `loadClass`
  - `(Ljava/lang/String;)Ljava/lang/Class;`
  - `newInstance`
  - `()Ljava/lang/Object;`
- `0x1e560` decodes and uses:
  - `java/lang/Class`
  - `getDex`
  - `()Lcom/android/dex/Dex;`
  - `L`
  - `;`
- APK/source-path helpers decode:
  - `classes.dex`
  - `customization`
  - `applicationInfo`
  - `sourceDir`
  - `getPackageCodePath`
- Runtime Application/LoadedApk patching helpers decode:
  - `android/app/ActivityThread`
  - `currentActivityThread`
  - `mBoundApplication`
  - `mPackageInfo`
  - `mLoadedApk`
  - `mOuterContext`
  - `mApplication`
  - `mInitialApplication`
  - `mAllApplications`

DEX injection layer now has concrete decoded strings:

- Function `0x306ec`:
  - `java/nio/ByteBuffer`
  - `wrap`
  - `([B)Ljava/nio/ByteBuffer;`
  - `dalvik/system/InMemoryDexClassLoader`
  - `<init>`
  - `([Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V`
  - `java/lang/Class`
  - `getClassLoader`
  - `()Ljava/lang/ClassLoader;`
  - `com/ashield/Stub`
  - `dalvik/system/BaseDexClassLoader`
- Function `0x32150` and `0x35a94` reference the older `BaseDexClassLoader` / `DexPathList` / `dexElements` / `DexFile.loadDex` compatibility path.
- Clear rodata references around `0xce988..0xcec20` include:
  - `pathList`
  - `Ldalvik/system/DexPathList;`
  - `dexElements`
  - `[Ldalvik/system/DexPathList$Element;`
  - `dalvik/system/DexFile`
  - `loadDex`
  - `(Ljava/lang/String;Ljava/lang/String;I)Ldalvik/system/DexFile;`
  - `/tmp.dex`
  - `/odex.dex`
  - `mCookie`

Native registration clarification:

- `0xa02a0` calls JNIEnv `RegisterNatives` with table `0xfce80`, count `1`.
  - This is the separate `ashieldStub4` table:
    - name pointer `0xd2050` -> `ashieldStub4`
    - signature pointer `0xd205d` -> `(Landroid/content/res/AssetManager;Ljava/lang/String;Ljava/lang/String;)I`
    - function pointer `0xa0374`
- `0xe900` calls `0x19984` first, then calls helper `0xff7c` with table `0xf9190`, count `5`.
  - `0xf9190` is a 5-row runtime-filled table whose function pointers are:
    - `0x28424`
    - `0x19f50`
    - `0x19f18`
    - `0x1d748`
    - `0x9be60`
  - Its name/signature pointers point to runtime output buffers populated by decoder logic around `0x197c4`.
  - Decoded names observed in `0x197c4` include `ashieldStub1`, `ashieldStub2`, `ashieldStub3`; the remaining entries align with `ashieldStub5` and `ashieldStub7` from the stub DEX signatures, but some signatures still need better data-flow emulation to decode cleanly.

Corrected interpretation:

- There are two registration/bridge paths:
  - static one-entry `RegisterNatives` path for `ashieldStub4` at `0xa02a0 -> 0xfce80`;
  - runtime-filled five-entry table path at `0xe900 -> 0xff7c -> 0xf9190`.
- The real protected DEX recovery path is now likely:

```text
ashieldStub5
  -> load real appName if possible
  -> fallback/recovery path around 0x1e560
  -> obtain com/android/dex/Dex via getDex()
  -> inject reconstructed DEX via 0x306ec / InMemoryDexClassLoader on newer Android
  -> or inject via BaseDexClassLoader/DexPathList/DexFile.loadDex compatibility functions on older Android
```

Next target:

- Reverse `0x1e560` enough to identify where the reconstructed byte array is produced.
- Track the arguments passed into `0x306ec`: it expects a JNIEnv-like wrapper, a vector/list of DEX byte arrays or byte buffers, and a boolean selecting injection mode. The call at `0x31a48 -> 0x3023c` inside `0x306ec` is part of the DEX installation path, not camera protocol logic.
- Once the reconstructed DEX byte array source is identified, dump or rebuild those DEX bytes locally, then parse `Packet`, `Message`, `MessageMuxer`, `PacketEncryptionParams`, and `EncryptionManager` from the real classes.

## Step 18 - Trace loader call chain around `0x8d4a0`

Scope: APK-only reverse work. No public protocol material used.

Call-chain scan results:

- `0x8d4a0` is a high-level loader hub.
  - It calls old/compat injection helpers:
    - `0x8db74 -> 0x35a94`
    - `0x8dbdc -> 0x35a94`
    - `0x8dc18 -> 0x35a94`
  - It calls class/Dex processing:
    - `0x8e060 -> 0x1e560`
    - `0x8e400 -> 0x2e9e0`
  - It calls data/vector helpers:
    - `0x8d840 -> 0x8e760`
    - `0x8d854 -> 0x8f4a0`
    - `0x8dbf8 -> 0x748cc`
    - multiple calls to `0x16414`, `0x1653c`, `0x1164c`, `0x11674`
- `0x8e690` is a thin wrapper over the modern injection path:

```text
0x8e690(x0, x1):
  w2 = -1
  return 0x306ec(x0, x1, -1)
```

- No direct `bl 0x8e690` was found. Direct callers exist for nearby helper `0x8e6d8`:
  - `0x8aa54` and `0x8aaa8` inside `0x880c0`
  - `0x94dd4` and `0x94e1c` inside `0x93f3c`
- Raw 64-bit pointer scans for `0x8e690`, `0x8e6d8`, `0x8e70c`, `0x306ec`, `0x35a94`, `0x32150`, `0x341bc`, `0x1e560`, and `0x2e9e0` found no literal pointer entries in the ELF file. These functions are likely bound through code-generated object slots, relative construction, or indirect C++ method tables rather than plain static 64-bit function pointers.

Important correction to Step 17:

- `0x1e560` may not itself produce the final reconstructed DEX byte array. Its decoded use of `Class.getDex()` plus `L` / `;` string construction suggests it likely processes class descriptors within an existing `com/android/dex/Dex` object.
- The modern byte-array/vector-to-classloader path is still `0x306ec`, because it explicitly decodes and uses:
  - `java/nio/ByteBuffer.wrap`
  - `dalvik/system/InMemoryDexClassLoader`
  - `([Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V`
- The next best target is the object construction path around:
  - `0x93620`
  - `0x93f3c`
  - `0x95f4c`
  - `0x965d0`
  - `0x880c0`

Working interpretation:

```text
JNI_OnLoad
  -> runtime mode detector 0x82a38
  -> constructs loader object, e.g. 0x93620 / 0x97084 path
  -> high-level loader hub 0x8d4a0
  -> tries old BaseDexClassLoader/DexPathList paths via 0x35a94
  -> processes Dex/class descriptors via 0x1e560 and 0x2e9e0
  -> modern InMemoryDexClassLoader injection is wrapped by 0x8e690 -> 0x306ec
```

Next target:

- Decode or annotate the constructor/object-slot code in `0x93620` and `0x93f3c` to find where `0x8d4a0`, `0x8e6d8`, or equivalent method slots are attached.
- Once the slot owner is understood, track its input vector/list back to the `adx0` body section data.

## Step 19 - Confirm `adx0` loader entry and VM-built internal table

Scope: APK-only reverse work. No public protocol material used.

New native string decoders recovered:

- Decoder around `0x8ab44`:

```text
real_ptr = pseudo_ptr + 0xffffffffb5264604
dst[i] = (((src[i] ^ 0x14) - i + key) & 0xff)
```

Confirmed strings include `apk@classes.dex`, `ls`, `opt`, `/classes.dex`, `%d.dex`, `classes`, `.dex`, `protectTime`, `activityName`, and two ART `DexFile::Open*` symbols.

- Decoder around `0x94e64`:

```text
real_ptr = pseudo_ptr + 0xffffffff9d6cf33e
dst[i] = ((((src[i] + 0x46) & 0xff) - i) ^ key) & 0xff
```

Confirmed strings include `apk@classes.dex`, `ls`, `customization`, `edh`, and `activityName`.

Important loader path update:

- `0x880c0` is now a confirmed `adx0/classes.dex` entrypoint.
  - It decodes `apk@classes.dex`.
  - It obtains a candidate pointer to the embedded `classes.dex` buffer.
  - At `0x88370`, it calls an indirect method with:
    - `x1 = candidate_ptr + 0x0c`
    - `w2 = [candidate_ptr + 8]`
  - This matches the known Ashield layout: `adx0` magic, metadata size, then metadata/body bytes after a 12-byte header.
  - It decodes `ls` and compares the metadata offset against the pointer-derived offset. The decoded APK metadata already showed `ls=6372`, which equals the `adx0` offset in the protected `classes.dex`.
  - It checks the body structure by reading `candidate_ptr + [candidate_meta+8] + 0x0c`, then using helper `0x8b3ec` to read a little-endian u32 at that location.
  - On success it calls `0x8b9cc`, a thin wrapper over `0x2a1e0`.

`0x2a1e0` and `0x2a328` findings:

- `0x2a1e0` builds a `0x250`-byte stack object/context.
- It copies many constants from `.data.rel.ro` around `0xf5350..0xf53c0` into that context.
- It then calls:

```text
0x2a328(0x250, 0xcf570, context_ptr)
```

- `0xcf570` is not normal text or a C++ vtable. It is bytecode/data for a custom VM.
- `0x2a328` is a VM runner:
  - allocates an aligned output/work area of size `0x250`;
  - sets `pc = 0`;
  - repeatedly calls `0x2a57c(bytecode, pc, context, &hi, &lo)`;
  - stores the returned `pc`;
  - stops when the returned `pc` no longer advances.
- `0x2a57c` is the VM instruction dispatcher:
  - reads `insn = *(u32 *)(bytecode + pc)`;
  - uses `op_a = (insn >> 10) & 0x3f`;
  - uses `op_b = (insn >> 21) & 0x3f`;
  - reads and writes 64-bit slots in the context with indexes from instruction bitfields;
  - implements register moves, immediate extraction, shifts, multiplication/umulh, and indirect calls through function pointers stored in the context.

Current interpretation:

```text
adx0 pointer verified by 0x880c0
  -> 0x8b9cc
  -> 0x2a1e0
  -> VM context seeded from 0xf5350 table
  -> VM bytecode at 0xcf570
  -> VM likely constructs an internal parser/table used to split or rebuild protected DEX sections
```

Next target:

- Build a small static VM tracer for `0x2a57c` using the `0xcf570` bytecode and `0xf5350` seed context.
- First goal is not full execution; trace instruction classes, target context slots, and indirect function-pointer slots.
- Once VM output slots are known, connect them back to `0x880c0` and the `adx0` body index table to recover reconstructed DEX bytes.

Tool added:

- `reverse_apk/tools/ashield_vm_trace.py`

Usage:

```powershell
$env:PYTHONIOENCODING='utf-8'
py F:\Insta360onWin\reverse_apk\tools\ashield_vm_trace.py --count 160
```

The tool prints:

- non-zero `0x250`-byte seed context slots from `0xf5350`;
- decoded fields for VM bytecode words at `0xcf570`;
- opcode-pair counts using the same bit fields observed in `0x2a57c`.

First run result:

- Seed slots are mostly not normal ELF virtual addresses. Many have repeated high patterns such as `0x64a13xxx` and `0x649e4xxx`, so they are likely encoded constants or VM-decoded function/table entries rather than direct native pointers.
- The first 96 bytecode words are dominated by:
  - `opa=0x2a opb=0x3c`
  - `opa=0x2a opb=0x1e`
  - `opa=0x2a opb=0x21`
  - `opa=0x1a` variants
  - `opa=0x03` variants
- Directly disassembling `0xcf570` as ARM64 does not produce coherent executable code. The VM borrows AArch64-like bitfield shapes but uses its own dispatcher.

## Step 20 - Test direct DEX reconstruction from exposed map lists

Scope: APK-only reverse work. No public protocol material used.

New experiment:

- Parsed all 17 protected section positions as DEX `map_list` records. Every listed position still contains a valid 18-entry DEX-style `map_list`.
- For Segment 11, the map list at `bodyrel 0x04fa1095` reports:
  - `string_id size=59316 off=0x70`
  - `type_id size=9840 off=0x39f40`
  - `class_def size=6420 off=0x11bbc8`
  - `string_data size=59316 off=0x5d37b6`
  - `map_list off=0xa61758`
- `Packet`/`EncryptionManager` strings were found in Segment 11:
  - `Lcom/arashivision/onedriver/packet/Packet;` at `bodyrel 0x5402d50`
  - `PacketEncryptionParams` at `bodyrel 0x5402ebf`
  - `UCD2-XOR-KEY-001` at `bodyrel 0x521c19b`
  - `fromBytes: encrypted frame...` at `bodyrel 0x54941c8`

Attempted reconstruction:

- Used `dex_base = map_bodyrel - map_off`.
- Built a synthetic DEX header and wrote files under:

```text
reverse_apk/reconstructed_dex/segXX.dex
```

Result:

- This direct reconstruction is not valid.
- Example Segment 11:
  - inferred base `0x453f93d`;
  - at reconstructed offset `0x70`, bytes are string data such as `pin(unPin)TopActivityStack is not valid`, not a `string_id` table;
  - parsing `string_ids[0]` immediately fails because the first four bytes are ASCII, not an offset.
- Therefore the exposed map lists preserve original DEX section metadata, but Ashield's protected body is not laid out as a simple original DEX image with a missing header. Section order and/or index tables have been stripped, relocated, or virtualized.

Current conclusion:

- The direct "琛?DEX 澶? path is a dead end for this APK.
- The useful output is that Segment 11 is still the Packet/Encryption target segment, but real reconstruction needs Ashield's native reassembly logic, likely the `0x880c0 -> 0x8b9cc -> 0x2a1e0 -> VM` path.

Next target:

- Continue decoding VM opcode semantics or locate the native routine that materializes `string_id`, `type_id`, `method_id`, and `class_def` tables from the protected body.
- Keep `reverse_apk/reconstructed_dex/segXX.dex` only as failed experiment artifacts; do not treat them as valid DEX files.

## Step 21 - Correct VM context layout

Scope: APK-only reverse work. No public protocol material used.

Correction to the VM tracer:

- `0xf5350` is not copied into VM context slot 0.
- `0x2a1e0` builds the context at `sp+0x30`:
  - `context[0]` is VM output; it is loaded after `0x2a328` and returned by `0x2a1e0`.
  - `context[1]` receives the caller's `x0`.
  - `context[2..16]` receive qwords loaded from `.data.rel.ro` at `0xf5350..0xf53c0`.
  - `context[16]` / offset `0x80` is written separately from `0xf53c0`.
- `0x8b9cc` calls `0x2a1e0` with its second argument:

```text
0x8b9cc:
  x0 = saved x1
  bl 0x2a1e0
```

So the VM input in `context[1]` is the pointer passed as `x1` to `0x8b9cc` from the `0x880c0` validation path.

Updated tool:

- `reverse_apk/tools/ashield_vm_trace.py` now prints:
  - slot 0 as VM output;
  - slot 1 as `<caller x0>`;
  - slots 2..16 from `0xf5350`.

Environment check:

- No local ARM/native emulation libraries are currently installed in Python:
  - `unicorn`: not installed
  - `angr`: not installed
  - `qiling`: not installed

Next target:

- Continue static VM decoding unless an emulator dependency is intentionally added later.
- Track the `x1` argument passed into `0x8b9cc` from `0x880c0`, because that is the VM's protected-body input pointer.

## Step 22 - Unicorn VM emulation and native hook boundary

Scope: APK-only reverse work. No public protocol material used.

Added dynamic emulation tool:

- `reverse_apk/tools/ashield_vm_emulate.py`

Local dependency:

- Installed Python `unicorn` in the user Python environment.
- `capstone` is also available and used for AArch64 disassembly checks.

Emulation setup:

- Maps `libashield.so` PT_LOAD segments into Unicorn.
- Maps the protected `adx0` business body from `classes.dex` at `0x73000000`.
- Seeds VM context exactly as corrected in Step 21:
  - slot 0 output;
  - slot 1 input pointer;
  - slots 2..16 from `.data.rel.ro` `0xf5350..0xf53c0`.
- Starts at `0x2a328(0x250, 0xcf570, context_ptr)`.

Important PLT correction:

- Earlier notes that treated some PLT addresses as memory functions were partly wrong.
- Correct `.plt` order from `.rela.plt`:
  - `0xdc00` = `pthread_create`
  - `0xdc10` = `pthread_join`
  - `0xde30` = `memcmp`
  - `0xdf80` = `mmap`
  - `0xdfb0` = `memmove`
  - `0xe000` = `strlen`
  - `0xe020` = `close`
  - `0xe060` = `munmap`
  - `0xe070` = `malloc`
  - `0xe080` = `memcpy`
  - `0xe090` = `memset`
  - `0xe0f0` = `free`
  - `0xe100` = `strcpy`

Hooks currently implemented:

- allocation: `0xa4570`, `malloc`, `calloc`, `realloc`
- free/noop: `free`, `0xa184c`, `0xa45d8`
- memory/string: `memcpy`, `memmove`, `memset`, `memcmp`, `strlen`, `strcpy`, `strcmp`, `strncmp`, `strcasecmp`
- threading: `pthread_create`, `pthread_join`, mutex/once/cond noops
- mapping: `mmap`, `munmap`, `close`
- temporary body-check bypass: `0x3e1f8 -> return 1`

Observed path:

- VM reaches `0x2afd8 -> 0x2df38`, then allocator hooks.
- `0x2b098 -> 0x2ca28 -> 0x3e1f8` receives:
  - `dst=0x50000020`
  - `src=0x7300000c`
  - `len=0x10adb3`
- Temporary hook returns `1` to advance.
- Large buffers allocated:
  - `0x500000a0 size=0x880840`
  - `0x508808e0 size=0x1101080`
  - copied chunk: `0x51981980 size=0x6766307` from protected body `0x7310adcb`

Heap dump support:

```powershell
$env:PYTHONIOENCODING='utf-8'
py F:\Insta360onWin\reverse_apk\tools\ashield_vm_emulate.py --max-insn 3000000 --sync-pthreads --dump-dir F:\Insta360onWin\reverse_apk\vm_heap_dumps
```

Before fast RC4 hook, the large copied chunk contains clear target strings:

- `EncryptionManager` at heap offset `+0x43efb6`
- `Packet` at heap offset `+0x43f02f`
- `UCD2-XOR-KEY-001` at heap offset `+0x51113d0`

This confirms the VM/native path is operating on the target Packet/Encryption segment, not an unrelated library block.

## Step 23 - RC4-style state-machine loop and current blocker

Scope: APK-only reverse work. No public protocol material used.

Disassembled slow loop around `0x4a9a0..0x4ac54`.

Conclusion:

- The loop is an RC4-style PRGA over a 256-byte state table:
  - increment `i`
  - add `state[i]` into `j`
  - swap `state[i]` and `state[j]`
  - XOR data byte with `state[(state[i] + state[j]) & 0xff]`
- It is flattened as a state machine, not reached as a clean function entry.
- The hot loop entry observed during emulation is `0x4aae8` / `0x4aaec`.

Fast hook added:

- At `0x4aae8` / `0x4aaec`, read stack locals:
  - state pointer: `[sp+0x50]`
  - data pointer: `[sp+0x48]`
  - total length: `[sp+0x40]`
  - current offset: `[sp+0x28]`
  - PRGA `i`: `[sp+0x3c]`
  - PRGA `j`: `[sp+0x38]`
- Process remaining bytes in Python and jump to `0x4ac54`.

Current behavior with `--sync-pthreads`:

- First worker:
  - copies `0x6766307` bytes from `0x7310adcb`;
  - fast PRGA hook runs;
  - then calls `mmap(addr=0, len=0xf99e9efd, prot=7, flags=0x22)`.
- Second worker:
  - copies `0xe0102` bytes from `0x7325afd7`;
  - fast PRGA hook runs;
  - then calls `mmap(addr=0, len=0xd18e2f68, prot=7, flags=0x22)`.
- The `mmap` lengths look invalid, so the hook returns `MAP_FAILED` for huge lengths.
- After two failed workers, the third inline worker reaches `0x2a0f4` and tries to read `[0x144c1e041]`, causing an unmapped read.

Interpretation:

- The copied pre-PRGA chunk definitely contains Packet/Encryption strings.
- The fast PRGA hook may still be slightly wrong, or this PRGA pass may not be the final DEX materialization step.
- Do not treat post-PRGA heap dump as a valid DEX yet.
- Do not remove the pre-PRGA dump path; it is currently the most reliable artifact proving target coverage.

Next target:

- Validate the PRGA hook against native execution on a small prefix:
  - either trace a few native loop iterations before the hook takes over;
  - or add a debug mode that hooks after `N` native bytes and compares Python state/data.
- Inspect where `[sp+0x224]` is set before `0x28e78 -> 0x7c70c`; this value becomes the suspicious `mmap` length.
- If PRGA is correct, next missing piece is the structure parser that turns decrypted chunks into mmap-backed code/data.

## Step 24 - RC4 validation, native body-check fast path, and pthread return bridge

Scope: APK-only reverse work. No public protocol material used.

Updated `reverse_apk/tools/ashield_vm_emulate.py`.

Confirmed:

- The RC4-style PRGA hook is equivalent for at least the tested native prefix.
  - `--rc4-native-prefix 32` lets native code process the first 32 bytes, then compares Python state/data.
  - Result observed: `state_ok=True data_ok=True ij_ok=True`.
- Thread inline execution needed a real return-value bridge.
  - `pthread_create` now records the fake thread id and caller LR.
  - `THREAD_RETURN_ADDR` records the worker's `x0` into `thread_results[tid]`.
  - `pthread_join` writes that stored value to `value_ptr`.
- Added `--native-body-check` mode:
  - `0x3db5c` is hooked as body precheck and returns `0`.
  - `0x3e2a4` is fast-hooked as the XOR `0x52` loop over the temporary body buffer, then resumes at `0x3e2e8`.
  - This avoids the slow byte loop while preserving the later scanner logic.
- `py -m py_compile reverse_apk/tools/ashield_vm_emulate.py` passes.

Observed after the above:

- The run still reaches the same Packet/Encryption chunk area.
- First worker still processes the record at body-relative `0x10adc3`.
- First worker data still contains the target chunk length `0x06766307`.
- The reader still returns `0`, so `0x2848c` computes a bad mmap length:

```text
orig=0x00150208 consumed=0x06766307 reader_ret=0x00000000 remaining=0xf99e9efd total=0xf99e9efd
```

Conclusion:

- The bad mmap length is not caused by the Python PRGA implementation.
- The bad mmap length is also not fixed by running the native `0x3e1f8` body-check path instead of the older full bypass.
- Current blocker moved to the `0x2848c` chunk parser / handler selection path.

## Step 25 - Worker descriptor and chunk-mode trace

Scope: APK-only reverse work. No public protocol material used.

Added `--trace-threads` to `reverse_apk/tools/ashield_vm_emulate.py`.

What it prints:

- `pthread_create` argument structure:
  - `[arg]` output/context pointer
  - `[arg+8]` source record pointer
  - first bytes at the source record
  - body-relative source offset when it points into the mapped `adx0` body
- worker entry `0x2a0bc`.
- the actual call into `0x2848c` at `0x2a118`.
- `0x2848c` chunk mode at `0x289dc`.

Confirmed first worker descriptor:

```text
arg=0x5098b6a0
out=0x50000020
src=0x7310adc3
bodyrel=0x10adc3
src_len=0x00150208
data_u32=0x06766307
```

Actual `0x2848c` call:

```text
data=0x7310adc7
src_len=0x00150208
data_u32=0x06766307
out=0x50000020
```

Inside `0x2848c`:

```text
mode=0x00000000
orig=0x00150208
consumed=0x06766307
data_ptr=0x7310adcb
```

Interpretation:

- `0x2a0bc` treats the source record as:
  - first u32: `src_len`
  - then data pointer passed to `0x2848c`
- `0x2848c` reads the first u32 at that data pointer with `0x2905c`; this becomes `consumed`.
- Because `mode` is `0`, `0x2848c` takes the reader path and later computes:

```text
remaining = orig_len - 4 - consumed
total = reader_ret + remaining
```

- Since `consumed` is bigger than `orig_len`, this underflows and produces the bad mmap length.

Important correction:

- The earlier suspicion that a missing `atoi` hook selected the wrong branch was tested. Hooks for `atoi`, `strtoul`, and `strtol` were added, but `atoi` is not hit on this path. The chunk mode is `0` before the reader branch because local handler/object matching did not set it to `2`.

Next target:

- Reverse the handler selection before `0x289b8`, especially the path that sets `[sp+0x200]`.
- The simple path at `0x289f0` is only taken when `[sp+0x200] == 2`; that path uses `orig_len - 4` instead of the reader/underflow formula.
- Trace the object/function calls around:
  - `0x28500` virtual call into the object at `[sp+0x218]`
  - `0x287a0 -> 0x15270`
  - `0x287ec..0x28980`
  - `0x49ab8` handler lookup
- The likely missing piece is an emulated object/handler side effect, not UCD2 protocol logic.

## Step 26 - Correct native body-check note and parent descriptor trace

Scope: APK-only reverse work. No public protocol material used.

Important correction to Step 24:

- The old note saying "`0x3db5c` is hooked as body precheck and returns `0`" is obsolete.
- Current emulator behavior with `--native-body-check`:
  - `0x3db5c` is allowed to run natively.
  - `0x3dcd4` is fast-hooked for the rolling/checksum loop.
  - `0x3e2a4` is fast-hooked for the XOR `0x52` body-copy loop.

Observed native precheck output for the first body area:

```text
trace body precheck native pc=0x3db5c dst=0x50000020 src=0x7300000c len=0x10adb3
hook body rolling pc=0x3dcd4 src=0x7300000c len=0x10adb3 rolling_ptr=0x5000002a acc_ptr=0x5000003c rolling=a3 27 87 2d 52 63 b5 b1 db d4 b0 02 f5 52 c1 6d acc=0x0010ade9
```

Added `--trace-parent` to `reverse_apk/tools/ashield_vm_emulate.py`.

Purpose:

- Trace the parent-side object used just before `pthread_create`.
- Confirm where worker args come from before inline worker execution mutates state.

Confirmed parent object at `0x2d824`:

```text
obj=0x701ffc10
thread*=0x50215c20
start=0x2a0bc
arg_from_obj20=0x50a96460
```

The object fields around first worker creation:

```text
object+0x08 = 0x50215c20   ; thread pointer area
object+0x18 = 0x0002a0bc   ; worker start function
object+0x20 = 0x50a96460   ; current worker arg
object+0x30 = 0x50a96460   ; arg array base
object+0x38 = 0x01101080   ; arg-array/buffer allocation size candidate
```

First worker arg:

```text
arg=0x50a96460
out=0x50000020
src=0x7310adc3
bodyrel=0x10adc3
src_len=0x00150208
data_u32=0x06766307
```

Second worker arg:

```text
arg=0x50a96470
out=0x50000020
src=0x7325afcf
bodyrel=0x25afcf
src_len=0xd19c306e
data_u32=0x000e0102
```

Third worker arg after the first two worker failures:

```text
arg=0x50a96480
out=0x50000020
src=0x144c1e041
```

Interpretation:

- The first two worker args are real entries from the parent arg array.
- The third `src=0x144c1e041` is a derived/bad pointer after earlier worker failures, not a clean source record from the original body.
- The current crash is downstream of the failed worker processing; it is not the original root cause.

## Step 27 - Handler object field `+0x50` is empty

Scope: APK-only reverse work. No public protocol material used.

Expanded handler object dump to `0x80` bytes.

First worker handler object at `0x284ec`:

```text
trace handler object=0x50000020
vtbl=0x00000000000f57a0
rolling bytes at object+0x0a = a3 27 87 2d 52 63 b5 b1 db d4 b0 02 f5 52 c1 6d
object+0x1c = 0x0010ade9
object+0x28 = 0x50000050
object+0x50..0x77 = all zero
```

`0x40900` uses `object + 0x50`:

```text
0x40928: add x10, x9, #0x50
0x40964: bl 0xa0fd8
...
0x40988: ldr x1, [sp, #0xb8]
0x4098c: bl 0x467e4
```

Because `object+0x50` is currently zero-filled, the virtual call result is an empty local string:

```text
trace handler vcall result local=0x701ff3a0 head=00 ... 48 00 00 50 ...
```

The comparison target `0xd3ff2` is also intentionally an empty C string in `.rodata`; the next bytes are `operator<<`, etc.:

```text
0xd3ff2: 00 6f 70 65 72 61 74 6f 72 3c 3c 00 ...
```

So the current branch is:

```text
compare empty local string against empty rodata string -> match
match-ok=1
mode remains 0
atoi fallback at 0x28994 is skipped
```

This explains the bad chunk-mode path:

```text
mode=0
consumed=first u32 at data_ptr
remaining = orig_len - 4 - consumed
```

For the first worker:

```text
orig=0x00150208
consumed=0x06766307
reader_ret=0
remaining=0xf99e9efd
total=0xf99e9efd
```

Conclusion:

- The root issue is now narrowed to why `out_ctx+0x50` is empty in the emulator.
- Either a still-missing native side effect should populate `out_ctx+0x50`, or this emulator path needs a different object setup before worker `0x2848c`.
- The next target should be the functions that initialize `out_ctx` around `0x3db5c`, `0x3e174`, `0x40900`, and helpers `0xa0fd8` / `0x467e4` / `0x414c8`.

## Step 28 - Trace native writes into `out_ctx`

Scope: APK-only reverse work. No public protocol material used.

Added `--trace-outctx-writes` to `reverse_apk/tools/ashield_vm_emulate.py`.

Purpose:

- Watch native memory writes into the first `0x78`-byte output context allocated at `0x50000020`.
- Specifically verify whether `out_ctx+0x50` is ever populated with a handler/mode string object.

Observed writes before worker execution:

```text
trace outctx write pc=0x49858 off=0x0  size=8 value=0xf5728
trace outctx write pc=0x4977c off=0x0  size=8 value=0xf57a0
trace outctx write pc=0x49a7c off=0x30 size=8 value=0x0
trace outctx write pc=0x49a08 off=0x38 size=8 value=0x0
trace outctx write pc=0x49998 off=0x28 size=8 value=0x50000050
trace outctx write pc=0x497d4 off=0x70 size=8 value=0x0
trace outctx write pc=0x497d8 off=0x68 size=8 value=0x0
trace outctx write pc=0x497dc off=0x60 size=8 value=0x0
trace outctx write pc=0x497e0 off=0x58 size=8 value=0x0
trace outctx write pc=0x497e4 off=0x50 size=8 value=0x0
trace outctx write pc=0x497ec off=0x50 size=4 value=0x0
```

Writes during `0x3db5c` native precheck:

```text
trace outctx write pc=0x3db88 off=0x12 size=8 value=0x0
trace outctx write pc=0x3db8c off=0x0a size=8 value=0x0
trace outctx write pc=0x3db94 off=0x1c size=4 value=0xdc
```

Then the Python fast hook for `0x3dcd4` writes the real rolling bytes and accumulator into:

```text
out_ctx+0x0a
out_ctx+0x1c
```

No native write populates `out_ctx+0x50` before `0x2848c`.

Conclusion:

- `out_ctx+0x50` is not being written and then cleared; it is only initialized to zero.
- The missing piece is earlier than the worker chunk parser: some constructor/config path should decide the handler/mode name, or the current emulation path did not seed the context/object the same way Android runtime does.
- Next target: reverse the constructor functions at `0x4977c..0x49a7c` and the object setup around `0x3dfdc`, `0x3e174`, plus runtime-decoded `.bss` strings at `0xfdb48`, `0xfdb51`, and `0xfdd81`.

## Step 29 - Experimental mode=2 path and fast copy hook

Scope: APK-only reverse work. No public protocol material used.

Added experimental switches to `reverse_apk/tools/ashield_vm_emulate.py`:

- `--force-chunk-mode <n>`
- `--normalize-huge-src-len`

Added fast hook:

- `0x290d4(dst, src, len)` is a heavily flattened copy/memmove state machine.
- Hook now copies `len` bytes from `src` to `dst` and returns `dst`.

Why this was tested:

- For the first worker, forcing `mode=2` avoids the bad reader path.
- The `0x7c70c` allocator then mmaps a reasonable length:

```text
force chunk mode old=0x00000000 new=0x00000002
hook mmap addr=0x0 len=0x150204 prot=0x7 flags=0x22 -> 0x51b98000
hook copy-state-machine 0x290d4 dst=0x51b98000 src=0x7310adcb len=0x150204
```

Second worker observation:

```text
src=0x7325afcf bodyrel=0x25afcf
src_len=0xd19c306e
data_u32=0x000e0102
```

This record has a huge first word but a plausible next u32. With `--normalize-huge-src-len`, the emulator rewrites `x1` at the `0x2a118 -> 0x2848c` call:

```text
normalize huge src_len old=0xd19c306e new=0x000e0102 data=0x7325afd3
force chunk mode old=0x00000000 new=0x00000002
hook mmap addr=0x0 len=0xe00fe prot=0x7 flags=0x22 -> 0x51cea000
hook copy-state-machine 0x290d4 dst=0x51cea000 src=0x7325afd7 len=0xe00fe
```

Dump directory:

```text
F:\Insta360onWin\reverse_apk\vm_heap_dumps\force_mode2_normalized
```

Useful dumped mmap fragments:

```text
heap_51b98000_150204_mmap.bin
heap_51cea000_e00fe_mmap.bin
```

Important caveat:

- These forced-mode experiments are not final protocol evidence.
- They prove the mode-2 branch and fast copy behavior, but after the copy the `rc4` handler can still transform the mmap area in place.
- The two mmap fragments do not contain `dex\n`, `Packet`, `EncryptionManager`, or `UCD2-XOR-KEY-001` strings in their final dumped state.
- The third worker still uses bad pointer `0x144c1e041`, so the parent/result chain is still not fully correct.

Current interpretation:

- `mode=2` is the correct structural path for at least the first worker's `0x150204` fragment.
- The second worker likely has a one-word prefix before its real length (`0x000e0102`), but this is still an experimental normalization.
- The next target is not UCD2. It is the parent/worker result chain after each `0x2848c` return, especially why the third arg becomes `src=0x144c1e041`.

## Step 30 - Worker arg construction is VM-register driven

Scope: APK-only reverse work. No public protocol material used.

Added trace switches to `reverse_apk/tools/ashield_vm_emulate.py`:

- `--trace-worker-array-writes`
- `--trace-parent-stack-writes`

Trace files:

```text
F:\Insta360onWin\reverse_apk\vm_heap_dumps\trace_parent_stack.log
F:\Insta360onWin\reverse_apk\vm_heap_dumps\trace_vm_regs.log
```

Important finding:

- The bad third worker source is not mutated by the worker.
- The worker arg array is written by the VM state-machine block at `0x2c95c`:

```text
0x2c95c: str x11, [x0, x10]
```

- `0x2c95c` copies the value prepared in `sp+0x570` into the worker array.
- `sp+0x570` is filled by `0x2c850`:

```text
0x2c844: ldr x8, [sp, #0x568]
0x2c848: ldr x9, [sp, #0x230]
0x2c84c: ldr x8, [x9, x8, lsl #3]
0x2c850: str x8, [sp, #0x570]
```

Interpretation:

- `sp+0x230` is the VM register table.
- `sp+0x254` is the current VM instruction word.
- For `sp254=0x35000dc5`, the source register index is `6`.
- The bad pointer appears because VM register 6 already contains `0x144c1e041`.

Bad chain:

```text
reg6 = 0xffffffffd19c306e   ; second source first u32
reg11 = 0xffffffffd1b13282  ; derived huge length/advance value
reg6 = 0x144c1e035          ; 0x7310adb3 + 0xd1b13282
reg6 = 0x144c1e041          ; +0x0c
worker[2].src = 0x144c1e041
```

This proves that normalizing only the worker call at `0x2a118` is too late. The parent VM also uses the unnormalized huge length to advance the stream.

## Step 31 - Parent VM normalization reaches the real third section

Added experimental switch:

- `--normalize-vm-huge-length`

First attempt used the current VM register table and was wrong:

```text
normalize vm huge length old=0xd19c306e new=0x00000001 source=0x2
```

That produced `src=0x7325afd4`, which is not correct.

Second attempt uses `last_worker_src`:

```text
normalize vm huge length old=0xd19c306e new=0x000e0102 source=0x7325afcf
```

This changes the third worker source to the expected body-relative position:

```text
src=0x7333b0d5 bodyrel=0x33b0d5
```

The bytes at `classes.dex` file offset `0x33cb51` (`body_off 0x1a7c + bodyrel 0x33b0d5`) begin:

```text
02 30 09 32 06 10 0f 1a 02 30 01 4a 0e 10 16 1a
02 30 09 32 06 10 0f 1a 02 30 10 4a 10 10 17 1a
```

Important conclusion:

- `0x7333b0d5` is not a normal encrypted chunk with a little-endian length word.
- It looks like a structured protobuf-like table or VM IR stream.
- Forcing `mode=2` on this third section is wrong. It treats `0x32093002` as a length, then attempts a huge mmap (`0x1a0f1002`) and fails.

Next target:

- Stop relying on global `--force-chunk-mode 2`.
- Recover the real `0x2848c` handler/mode selection, especially the missing `out_ctx+0x50` configuration path.
- The first two sections can use the mode-2 copy/decrypt experiment, but the third section needs its actual handler.

## Step 32 - Handler/mode trace and constructor trace

Scope: APK-only reverse work. No public protocol material used.

Handler trace without forced mode:

```text
py reverse_apk\tools\ashield_vm_emulate.py --max-insn 1200000 --sync-pthreads --native-body-check --trace-handlers --trace-threads
```

Observed in `0x2848c`:

```text
trace handler object=0x50000020
trace handler object+0x50=0x50000070 head=00 ...
trace handler vtbl=0xf57a0
trace handler vcall object=0x50000020 vtbl=0xf57a0 fn18=0x40900
trace handler compare-call x0=... b'' x3=0xd3ff2 b''
trace handler match-ok=01 mode=0x00000000
trace handler final mode before chunk=0x00000000
```

Interpretation:

- `0x40900` is the vtable `fn18` used by `0x2848c` to get the mode string.
- `0x40900` uses object fields around `object+0x28` and `object+0x50`.
- Constructor trace for `0x49728` shows:

```text
0x49728 entry: x0=0x50000020 x1=0x110 x2=0x701ffd90
0x4977c stores vtable 0xf57a0
0x49784 prepares object+0x28
0x497d0..0x497ec clears object+0x50..0x70
```

No later native write to `object+0x50` has been seen before `0x2848c`, so empty mode is not due to a later clear; it is the constructed state.

Important correction to working theory:

- Global `--force-chunk-mode 2` is only an experimental bypass.
- For the first two sources it makes progress because their leading words behave like length-prefixed chunks.
- The third source reached after parent VM normalization is not length-prefixed chunk data:

```text
src=0x7333b0d5 bodyrel=0x33b0d5
head=02 30 09 32 06 10 0f 1a ...
```

- Treating that third source as mode 2 produces huge fake lengths (`0x32093002`, then `0x1a0f1006`) and is wrong.

Next target:

- Parse the `0x7333b0d5` structured table/IR directly.
- In parallel, continue identifying the default mode0 RC4/key path, because mode0 may be the intended default path rather than a bad mode.
## Step 33 - RC4 preimage capture and UCD2 XOR key evidence

Scope: APK-only reverse work. After the user's warning, all follow-up reads/writes were kept under `F:\Insta360onWin`; no APK/public protocol source was used.

Added `--dump-rc4-preimage-dir` to `reverse_apk/tools/ashield_vm_emulate.py`. It saves PRGA input buffers before the emulator mutates them, but only when they contain target strings such as `Packet`, `EncryptionManager`, or `UCD2-XOR-KEY-001`.

Run result:

```text
dump rc4 preimage F:\Insta360onWin\reverse_apk\vm_heap_dumps\rc4_preimages\rc4_preimage_51b97500_0_6766307.bin hits=Packet@+0x43f02f, EncryptionManager@+0x43efb6, UCD2-XOR-KEY-001@+0x51113d0, Lcom/arashivision/onedriver/packet/Packet;@+0x43f042
```

Important findings:

- The saved preimage contains DEX string_data_item records for the Packet/encryption classes.
- The same preimage contains Dalvik code items. A clean code_item at `0x5111394` has:
  - `registers=1`, `ins=0`, `outs=1`, `insns_size=30`.
  - It creates a new object, sets one static object field, then builds a byte array using a fill-array-data payload.
  - The fill-array-data payload is exactly the ASCII bytes `UCD2-XOR-KEY-001` (16 bytes).

Local disassembly of that code_item:

```text
0000: 22 00 0d 2d        new-instance        type@2d0d
0002: 70 10 35 f6 00 00  invoke-direct       method@f635 {v0}
0005: 69 00 46 70        sput-object         field@7046
0007: 13 00 10 00        const/16            v0, #0x10
0009: 23 00 36 37        new-array           type@3736
000b: 26 00 07 00 00 00  fill-array-data     +7 code units
000e: 69 00 4b 70        sput-object         field@704b
0010: 0e 00              return-void
0011: 00 00              nop/padding
0012: 00 03              array-data payload ident
0013: 01 00              element_width=1
0014: 10 00 00 00        size=16
0016: 55 43 44 32 2d 58 4f 52 2d 4b 45 59 2d 30 30 31
```

Conclusion:

- `UCD2-XOR-KEY-001` is not just a diagnostic string. It is materialized as a static byte-array field (`field@704b`) in the protected Packet/encryption code.
- The previous PRGA path is not the final DEX reconstruction path: PRGA input is already meaningful DEX material, while PRGA output produces invalid chunk lengths. The remaining blocker is handler/mode selection and/or the native reassembly path, not UCD2 itself.

Next target:

- Find all reads of `field@704b` and disassemble those methods. That should reveal the XOR packet encryption/decryption byte loop and how it plugs into `PacketEncryptionParams` / `EncryptionManager`.
## Step 34 - First read of UCD2 XOR key field

Searching for exact field access to `field@704b` found:

```text
0x62 sget-object field@704b at preimage offset 0x56d86af
0x69 sput-object field@704b at preimage offset 0x51113c0
```

The `sget-object` method is a small clean code_item at `0x56d869f`:

```text
registers=2 ins=1 outs=2 tries=0 insns_size=7
0000: 62 00 4b 70        sget-object     field@704b
0002: 70 20 f6 96 01 00  invoke-direct   method@96f6 {p0, v0}
0005: 12 11              const/4         v1, #1
0006: 0f 01              return          v1
```

Interpretation:

- `field@704b` is initialized from the 16-byte ASCII payload `UCD2-XOR-KEY-001` and then passed into `method@96f6`.
- The method returns constant `1`, so this is likely an initialization/availability method for XOR encryption rather than the byte loop itself.
- Next target is `method@96f6` and nearby code_item clusters that contain `aget-byte` / `xor-int` / `aput-byte`.

Correction for Step 34:

The preimage is not a single final DEX image. Field/method indexes can repeat across different protected DEX materials inside the same large buffer. Therefore `field@704b` matches outside the `UCD2-XOR-KEY-001` local window must be treated as possible index collisions until a per-DEX boundary is recovered.

New rule:

- For the XOR key, only trust references inside the local material window around `0x51113d0` unless the surrounding strings/code prove the same Packet/encryption DEX context.

Step 34 correction continued:

A local-window check around `0x511ace0` found no valid covering code_item, so that `4b 70` hit is not trusted as a real field access. For now the only high-confidence UCD2 key fact is the code_item at `0x5111394`, which materializes the 16-byte key array.

Next target:

- Recover the local Packet/encryption DEX material boundary around `0x5111394` and then rebuild enough field/method metadata to link the key initializer to its class and consumers.
## Step 35 - Recovered XOR byte loop

High-confidence local code_item near the UCD2 key material:

```text
code_item offset=0x51112f8
registers=6 ins=2 outs=0 tries=0 insns_size=19
0000: 21 40          array-length      v0, p0
0001: 12 01          const/4           v1, #0
0002: 35 01 10 00    if-ge             v1, v0, +16
0004: 48 02 04 01    aget-byte         v2, p0[v1]
0006: 21 53          array-length      v3, p1
0007: 94 03 01 03    rem-int           v3, v1, v3
0009: 48 03 05 03    aget-byte         v3, p1[v3]
000b: b7 32          xor-int/2addr     v2, v3
000c: 8d 22          int-to-byte       v2, v2
000d: 4f 02 04 01    aput-byte         v2, p0[v1]
000f: d8 01 01 01    add-int/lit8      v1, v1, #1
0011: 28 f1          goto              -15
0012: 0e 00          return-void
```

Recovered pseudocode:

```java
static void xorInPlace(byte[] data, byte[] key) {
    for (int i = 0; i < data.length; i++) {
        data[i] = (byte)(data[i] ^ key[i % key.length]);
    }
}
```

Recovered APK-derived XOR key:

```text
55 43 44 32 2d 58 4f 52 2d 4b 45 59 2d 30 30 31
ASCII: UCD2-XOR-KEY-001
```

This is the first final algorithmic piece needed by the Windows app's UCD2 packet layer. It is APK-derived from the protected Packet/encryption code, not guessed.

Next target:

- Recover Packet byte layout and outer UCD2 frame tail/checksum so the app can generate new camera-control frames instead of replaying captured frames.

## Step 36 - Recovered UCD2 Packet constants and builder entry

Added a reproducible local DEX inspection helper:

```text
F:\Insta360onWin\reverse_apk\tools\dex_inspect.py
```

The protected/reconstructed DEX tables are partially damaged, but code_item regions are usable. `seg12.dex` is the key segment for Packet/UCD2 work:

```text
F:\Insta360onWin\reverse_apk\reconstructed_dex\seg12.dex
```

Relevant raw string hits in `seg12.dex`:

```text
UCD2-XOR-KEY-001                       @ 0x56399c
PacketEncryptionParams                 @ 0x74a6c0
fromBytes: encrypted frame             @ 0x7db9c9
sync-packet                            @ 0x76fb77
Lcom/arashivision/onedriver/packet/Packet; @ 0x6c842
```

Static byte-array initializer at code_item `0x563fe8`:

```text
field@7059 = 73 79 4e 63 65 4e 64 69 6e 53  ASCII "syNceNdinS"
field@705a = 08 00 00 00 b0 00 00 01
field@705b = 07 00 00 00 05 00 00
field@705c = 55 43 44 32                       ASCII "UCD2"
```

`field@705c` is used by code_item `0x563ae8`, which is the high-confidence Packet `fromBytes` / parse entry:

- It receives a byte array in the second parameter (`p1`/`v12` in this code_item).
- It checks `data[0..3]` against `field@705c == "UCD2"`.
- If the frame length is 8, it checks against `field@705a`.
- It also has a legacy/sync path using `field@7059`.

High-confidence UCD2 builder/toBytes entry:

```text
code_item offset = 0x565824
registers=49 ins=9 outs=6 tries=0 insns_size=586
```

The UCD2 header write sequence inside `0x565824` starts at code-unit `pc=0x0f5`:

```text
new byte[header_or_frame_len]
index 0  = 0x55 ('U')
index 1  = 0x43 ('C')
index 2  = 0x44 ('D')
index 3  = 0x32 ('2')
index 4  = 0x01
index 5  = header_len
index 6  = message_type byte 0
index 7  = message_type byte 1
index 8  = payload_len byte 0
index 9  = payload_len byte 1
index 10 = payload_len byte 2
index 11 = payload_len byte 3
```

This exactly matches the observed LunaU frame shape:

```text
55 43 44 32 01 0c 04 10 0f 00 00 00 ...
```

Important: `method@ee8b` calls reached after the header write are likely array copy/slice helpers, not the checksum/tail algorithm. A class_data scan did not find a useful code_item for `method@ee8b`, and the call shape matches byte-array helper behavior.

Tail/checksum status:

- Still not recovered.
- Common zlib CRC32/JAMCRC-style checks on observed headers/payloads did not match the 4-byte tails.
- Observed tails tested:

```text
05 01 empty payload -> 11 28 34 b2
05 02 empty payload -> f6 7b 41 8a
05 0f empty payload -> 37 05 47 7c
04 10 payload       -> 7c 00 8e 7c
```

Next target:

- Continue inside `0x565824` after the UCD2 header write and identify where the final 4 bytes are produced/written.
- Add register/operand decoding for arithmetic opcodes (`and-int/lit16`, `shr-int/lit8`, etc.) to make the final-byte writes readable.

## Step 37 - Recovered final UCD2 tail/checksum algorithm

Final missing piece recovered from `seg12.dex`.

Standard 12-byte UCD2 builder is not `0x565824`; the standard no-encryption frame builder is:

```text
code_item offset = 0x5651f4
registers=15 ins=4 outs=6 tries=0 insns_size=155
```

This method writes:

```text
index 0  = 0x55 ('U')
index 1  = 0x43 ('C')
index 2  = 0x44 ('D')
index 3  = 0x32 ('2')
index 4  = 0x01
index 5  = 0x0c
index 6  = message_type byte 0
index 7  = message_type byte 1
index 8  = payload_len byte 0
index 9  = payload_len byte 1
index 10 = payload_len byte 2
index 11 = payload_len byte 3
```

Then it copies payload and calls:

```text
method@ee8c(frame_bytes, tail_offset)
```

The returned `int` is written little-endian to:

```text
frame[tail_offset + 0]
frame[tail_offset + 1]
frame[tail_offset + 2]
frame[tail_offset + 3]
```

`method@ee8c` implementation was recovered via the local CRC table:

- CRC table initializer: `0x566a70`
- Table field: `field@708b`
- Table size: 256 x u32
- First entries:

```text
00000000 04c11db7 09823b6e 0d4326d9 130476dc 17c56b6b ...
```

This is a non-reflected CRC32 table using polynomial:

```text
0x04c11db7
```

Recovered tail algorithm:

```java
int crc = 0xffffffff;
for (int i = 0; i < tailOffset; i++) {
    crc ^= frame[i] & 0xff;
    for (int j = 0; j < 4; j++) {
        crc = (crc << 8) ^ table[(crc >>> 24) & 0xff];
    }
}
tail = littleEndian(crc);
```

Validation against observed LunaU frames:

```text
UCD2 01 0c 05 01 00 00 00 00 -> crc b2342811 -> tail 11 28 34 b2
UCD2 01 0c 05 02 00 00 00 00 -> crc 8a417bf6 -> tail f6 7b 41 8a
UCD2 01 0c 05 0f 00 00 00 00 -> crc 7c470537 -> tail 37 05 47 7c
04 10 device-info frame payload -> crc 7c8e007c -> tail 7c 00 8e 7c
```

Rust implementation added:

```text
F:\Insta360onWin\src\adapters\luna_local.rs
```

New functions:

```text
build_ucd2_frame(group, code, payload)
ucd2_checksum(bytes)
```

`read_device_info()` now generates the known `04 10` frame from payload + recovered checksum instead of relying on a hard-coded full packet.

## Step 38 - Continue Packet builder analysis

Scope: APK-only reverse work under `F:\Insta360onWin` only. No C-drive project access and no public protocol/reference material.

Disassembled the larger Packet/UCD2 builder candidate:

```text
seg12.dex code_item offset = 0x565824
registers=49 ins=9 outs=6 tries=0 insns_size=586
```

Important observation:

- `0x565824` is not the simple 12-byte standard UCD2 frame builder. That role is already confirmed as `0x5651f4`.
- `0x565824` appears to build a lower/internal Packet header first, then wrap it in an outer UCD2 frame.
- This matches the known device-info payload:

```text
08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b
```

Working split:

```text
internal packet header candidate: 08 00 02 01 00 00 80 00 00
message/protobuf candidate:       08 30 08 0f 08 0b
```

Evidence from `0x565824`:

- It calculates a length and subtracts `9`.
- It writes bytes at internal header indexes `0..8`.
- Index `0..1` are written from a length-like value.
- Index `2` is written from a virtual getter result, likely content/type/direction.
- Index `3..6` are written little-endian from a long/int-like value.
- Index `7` and `8` are written as zero in the observed branch.
- Later code writes a normal outer UCD2 header beginning with:

```text
55 43 44 32 01 ...
```

Next target:

- Finish disassembling the rest of `0x565824`.
- Identify the exact source of the internal header fields and whether the known device-info packet uses content type `02`, sequence/request id `01 00 00 80`, and flags `00 00`.
- Recover command enum values and protobuf bytes for `StartCapture`, `StopCapture`, `TakePicture`, and `SetSyncCaptureMode` from APK-generated message classes.

## Step 39 - Message payload split and Packet header confirmation

Scope: APK-only reverse work under `F:\Insta360onWin` only.

Improved `reverse_apk/tools/dex_inspect.py` so it decodes basic Dalvik `move*`, `move-result*`, and range invoke opcodes and forces UTF-8 console output. This makes the protected Packet code easier to read and avoids Windows console encoding failures.

Re-read `seg12.dex` code item `0x565824` with the improved decoder.

Confirmed shape:

```text
UCD2 04 10 payload:
08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b

internal Packet header candidate:
08 00 02 01 00 00 80 00 00

inner Message protobuf candidate:
08 30 08 0f 08 0b
```

`0x565824` writes the internal Packet header first:

- bytes `0..1`: a little-endian length-like value from a getter.
- byte `2`: a content/type-like enum getter.
- bytes `3..6`: a little-endian id/sequence-like integer/long.
- bytes `7..8`: zero in the known no-encryption branch.

Then it wraps the result in UCD2. The method also has a longer extended-header branch, but the observed LunaU device-info frame uses the standard `01 0c 04 10` UCD2 header path already implemented by `build_ucd2_frame`.

Decoded the observed response Message bytes after the 9-byte internal Packet header:

```text
08 30 08 0f 08 0b 12 2e ...
```

Recursive protobuf interpretation:

```text
field 1 varint: 48
field 1 varint: 15
field 1 varint: 11
field 2 bytes length 46:
  field 11 bytes length 6:
    field 1 varint: 0
    field 2 varint: 82
    field 4 varint: 0
  field 15 bytes length 14: "BTLA3ABESWPJTD"
  field 48 bytes length 19: "Insta360 Luna Ultra"
```

Important correction:

- The three top-level values `48, 15, 11` are not safely interpretable as command-list ordinals.
- The `PHONE_COMMAND_*` raw strings are DEX `string_data_item` records with uleb length + null terminator. Their offsets are not present in normal `string_ids`, so normal class/method reference lookup fails.
- Do not use enum ordinal as final command code until a constructor/value initializer proves it.

Current likely control envelope:

```text
outer UCD2: build_ucd2_frame(0x04, 0x10, inner_packet)
inner Packet: 9-byte Packet header + Wire/protobuf Message bytes
inner Message: repeated field 1 command/message code(s), optional field 2 command payload bytes
```

Still missing:

- APK-proven numeric `MessageCode` values for:
  - `PHONE_COMMAND_START_CAPTURE`
  - `PHONE_COMMAND_STOP_CAPTURE`
  - `PHONE_COMMAND_STOP_TAKE_PICTURE`
  - `PHONE_COMMAND_TAKE_PICTURE`
  - `PHONE_COMMAND_SET_SYNC_CAPTURE_MODE`
- APK-proven protobuf payload defaults for `StartCapture`, `StopCapture`, `TakePicture`, and sync capture mode commands.

## Step 40 - Re-interpret internal Packet first word as status/request code

Scope: APK-only reverse work under `F:\Insta360onWin` only. Avoid C-drive project access.

Important correction to Step 38 wording:

- The first two bytes of the 9-byte internal Packet header are probably not a payload length.
- Known request Packet header begins with:

```text
08 00 02 01 00 00 80 00 00
```

- Known response Packet header begins with:

```text
c8 00 02 01 00 00 80 00 00
```

`0x0008` therefore looks like a request-kind/status word and `0x00c8` looks like a success/status word (`200`) in the APK Packet layer. The remaining known header fields still look like:

```text
byte 2:  content/type-like value = 02
bytes 3-6: little-endian id/session/sequence-like value = 01 00 00 80
bytes 7-8: flags/encryption/reserved = 00 00 in the observed no-encryption path
```

Current safest control-envelope hypothesis remains:

```text
outer UCD2: 04 10
inner Packet header: 08 00 02 01 00 00 80 00 00
inner Message: protobuf-like bytes beginning with repeated field 1 values
tail: APK CRC-like UCD2 checksum already implemented
```

Do not send guessed camera-control commands yet. First recover or prove the numeric `MessageCode` values and payload schemas from APK code, or use only observed read-only values from the known device-info message for live probes.

## Step 41 - Device-info `48/15/11` are probably field numbers, not command codes

Scope: APK-only reverse work under `F:\Insta360onWin` only. A live probe was attempted only with APK-observed read-only values and did not send camera-control guesses.

Parsed the protected raw `PHONE_COMMAND_*` string block from `seg12.dex` starting at the length-prefixed `PHONE_ANDROID` item. This is only a string block, not yet a proven enum value table, but it provides reliable local ordering:

```text
PHONE_COMMAND_GET_SYNC_CAPTURE_MODE  index 56
PHONE_COMMAND_SET_SYNC_CAPTURE_MODE  index 106
PHONE_COMMAND_START_CAPTURE          index 115
PHONE_COMMAND_STOP_CAPTURE           index 122
PHONE_COMMAND_STOP_TAKE_PICTURE      index 125
PHONE_COMMAND_TAKE_PICTURE           index 131
PHONE_COMMAND_PHONE_INFO             index 71
```

This proves the already-observed device-info Message values:

```text
08 30 08 0f 08 0b
```

must not be treated as `PHONE_COMMAND_*` ordinals. Values `48`, `15`, and `11` do not match the relevant command string positions.

Better interpretation:

- The `04 10` packet with inner message `08 30 08 0f 08 0b` is a device-info field request.
- The response top-level repeats the same values and then returns payload fields that match them:

```text
field 48 bytes: "Insta360 Luna Ultra"
field 15 bytes: "BTLA3ABESWPJTD"
field 11 bytes: nested status block
```

Therefore `48/15/11` are most likely requested `PhoneInfo`/device-info field numbers, not global camera command codes.

Live probe note:

- Sent only APK-observed read-only field combinations `[48]`, `[15]`, `[11]`, `[48,15]`, `[48,11]`, `[15,11]`, `[48,15,11]` using the known no-encryption Packet header and UCD2 `04 10`.
- The socket connected but no frames were received in that test run, likely because the camera/session was not in the same responsive state. No destructive control commands were sent.

Next target:

- Stop using the `04 10` device-info field request as a proxy for camera control.
- Continue from `seg06.dex` / `seg07.dex` generated `insta360/messages/*` classes for:
  - `StartCapture`
  - `StopCapture`
  - `TakePicture`
  - `SyncCaptureMode`
- Recover the true MessageCode numeric values from APK enum initialization or from sender-side call sites that combine `PHONE_COMMAND_*` with serialized message payloads.

## Step 42 - Command tables, message constructors, and current blocker

Scope: APK-only reverse work under `F:\Insta360onWin` only. No public protocol references.

Parsed two protected raw command-name blocks:

```text
seg06.dex PHONE_ANDROID block count: 226
  PHONE_COMMAND_GET_SYNC_CAPTURE_MODE       index 87
  PHONE_COMMAND_SET_SYNC_CAPTURE_MODE       index 170
  PHONE_COMMAND_START_CAPTURE               index 186
  PHONE_COMMAND_START_CAPTURE_WITH_PARAM    index 187
  PHONE_COMMAND_STOP_CAPTURE                index 199
  PHONE_COMMAND_STOP_TAKE_PICTURE           index 204
  PHONE_COMMAND_TAKE_PICTURE                index 212
  PHONE_COMMAND_TAKE_PICTURE_WITHOUT_STORING index 213

seg12.dex PHONE_ANDROID block count: 138
  PHONE_COMMAND_GET_SYNC_CAPTURE_MODE       index 56
  PHONE_COMMAND_SET_SYNC_CAPTURE_MODE       index 106
  PHONE_COMMAND_START_CAPTURE               index 115
  PHONE_COMMAND_STOP_CAPTURE                index 122
  PHONE_COMMAND_STOP_TAKE_PICTURE           index 125
  PHONE_COMMAND_TAKE_PICTURE                index 131
  PHONE_COMMAND_TAKE_PICTURE_WITHOUT_STORING index 132
```

Do not assume either index set is the final numeric protocol value. `seg06` looks like a fuller app-level enum, while `seg12` looks closer to the Luna/UCD2 transport module. The final value still needs proof from enum initialization or a sender call site.

Message constructor evidence from `seg06.dex` / `seg07.dex`:

```text
TakePicture constructor:
(Linsta360/messages/TakePicture$Mode;
 Linsta360/messages/ExtraMetadata;
 Ljava/util/List;
 Linsta360/messages/RawCaptureType;
 Z
 Linsta360/messages/SensorDevice;
 I
 Linsta360/messages/TriggerSource;
 Lokio/ByteString;)V

SyncCaptureMode constructor:
(Linsta360/messages/SyncCaptureMode;Lokio/ByteString;)V
```

`StartCapture` appears in the string pool primarily as Builder/Companion/ADAPTER descriptors, so the direct constructor signature was not recovered yet:

```text
Linsta360/messages/StartCapture$Builder;
Linsta360/messages/StartCapture$Companion$ADAPTER$1;
Linsta360/messages/StartCapture$Companion;
Linsta360/messages/StartCapture;
```

Config evidence:

- `assets/ins_config_files/phoneApp/z03/basic/business.json` confirms:

```json
"device_type": "Insta360 Luna Ultra",
"ucd2": true,
"connection_channels": ["wifi", "usb", "bluetooth"]
```

- `captureOperation/business.json` confirms LunaU uses camera JSON capability flags, but the full camera option JSON was not present in the extracted `assets`.
- `exportSetting/business.json` has `is_zstyle_frame_watermark: true`, useful later for watermark defaults, but not for UCD2 camera control packets.

Dynamic read-only probe status:

- Tried a known `05 0f` open packet and known device-info `04 10` packet.
- This run received no frames even for `05 0f`.
- A later read-only candidate probe for `GET_SYNC_CAPTURE_MODE` / `PHONE_INFO` was aborted by the peer.
- Therefore the current live test did not validate or reject candidate codes; it only shows the device/session was not responsive in this run.
- No guessed start/stop/take-picture command was sent.

Current blocker:

- The protected DEX class/method/type tables are damaged enough that normal class lookup (`class_contains`, androguard, full class_data scan) is unreliable.
- Raw command strings have no normal `string_ids` references.
- Need continue with one of:
  1. Recover the sender call site by direct code-item/invoke scanning around the UCD2/Packet builder methods.
  2. Get a responsive LunaU session and test only read-only command-envelope candidates first.
  3. Install/use a local decompiler toolchain under `F:\Insta360onWin` only, if available, to decompile the reconstructed DEX fragments without relying on public protocol info.

Do not mark camera-control packets final yet.

## Step 43 - Packet builder callers and UCD2 dispatch paths

Scope: APK-only reverse work under `F:\Insta360onWin` only. No public protocol references.

Added `reverse_apk/tools/dex_code_scan.py` to scan plausible protected DEX code items for `invoke-*`, string refs, type refs, field refs, and constants. This is needed because normal DEX metadata tables are partly damaged.

Mapped the protected `seg12.dex` Packet/UCD2 code items back to method indices through class-data ULEB walking:

```text
method@ee8b code 0x5651f4  standard UCD2 frame builder
method@ee8c code 0x565d78  checksum helper
method@ee8d code 0x56533c  parser/fromBytes candidate
method@ee91 code 0x565824  standard internal Packet + UCD2 builder
method@ee92 code 0x565cc8  short/event packet builder candidate
method@ee60 code 0x5646b4  extended/encrypted packet builder candidate

method@edc4 code 0x55e378  upper standard/extended build-and-dispatch router
method@edba code 0x55caec  large UCD2/extended router
```

Caller scans:

```text
method@ee91 callers: 0x55e378 only
method@ee60 callers: 0x55e378 only
method@ee50 callers: 0x55caec, 0x55e378, 0x563e48
method@eed2 callers: 0x55caec, 0x55e378, 0x56772c
```

Wrapper evidence:

```text
0x55e34c -> method@edc1 -> method@edc4(0x55e378)
0x55cac0 -> method@edb7 -> method@edba(0x55caec)
```

`0x55e378` is therefore the best current upper Packet/UCD2 build-and-dispatch target. It calls both `ee60` and `ee91`, then sends through `ee50` or `eed2` depending on branch/state.

`0x55caec` is a large UCD2/extended router and includes constants seen in known frames: `85 67 68 50` (`UCD2`), `48`, `15`, `11`, `56`, and checksum/frame helper calls.

## Step 44 - Packet header layout from `method@ee91`

Scope: APK-only reverse work under `F:\Insta360onWin` only.

Disassembled `seg12.dex` code item `0x565824` (`method@ee91`). This is the strongest evidence so far for the 9-byte internal Packet header used before the outer UCD2 wrapper.

Header bytes are written as:

```text
byte 0..1  little-endian value from an object method call on the first Packet parameter
byte 2     one-byte option/value from another object method call
byte 3..6  little-endian 32-bit id/session/sequence value, with a flag mixed into bit 30 in one branch
byte 7..8  zero in the observed no-encryption path
```

This matches the observed device-info request:

```text
08 00 02 01 00 00 80 00 00
```

and explains why the first word should not be treated as a length. The first word is a Packet-level type/command-like value. The payload follows after the 9-byte header, then the whole inner Packet is wrapped by UCD2.

`method@ee8b` (`0x5651f4`) remains the standard outer UCD2 builder:

```text
UCD2 magic
version 01
header length 0c
message type bytes
payload length little-endian u32
payload bytes
4-byte APK-derived checksum tail
```

`method@ee92` (`0x565cc8`) builds a shorter packet-like header:

```text
byte 0..3  little-endian total length-ish value
byte 4     static enum/value from field@709e
byte 5..6  zero
then payload bytes
```

This looks like a separate event/channel packet path and should not be used for camera-control command generation until its caller context is proven.

## Step 45 - Command enum proof from `seg06.dex`

Scope: APK-only reverse work under `F:\Insta360onWin` only.

The full app-level `PHONE_COMMAND_*` enum initializer in `seg06.dex` is at code item `0x43e424`. It constructs enum values with constructor arguments `(name, ordinal, value)`. The enum array initializer is at `0x43cc60`.

Confirmed camera-control values from initializer and array index:

```text
PHONE_COMMAND_START_CAPTURE                 value 186  field@4ee2
PHONE_COMMAND_START_CAPTURE_WITH_PARAM      value 187  field@4ee3
PHONE_COMMAND_STOP_CAPTURE                  value 199  field@4fda
PHONE_COMMAND_STOP_TAKE_PICTURE             value 204  field@5004
PHONE_COMMAND_TAKE_PICTURE                  value 212  field@5083
PHONE_COMMAND_TAKE_PICTURE_WITHOUT_STORING  value 213  field@5084
```

Important caution:

- This proves the full app-level command enum values in `seg06.dex`.
- It does not yet prove how those enum values are encoded into the LunaU UCD2 Packet payload.
- The shorter `seg12.dex` command-name block and the `0x567134` enum-like initializer with values `16,32,33,34,48,64,131,132,133,255` are a different transport/status enum and must not be mixed with the full `PHONE_COMMAND_*` enum.

Next target:

- Recover the sender call site that takes a `PHONE_COMMAND_*` enum plus a serialized `insta360/messages/*` protobuf and passes it into the `seg12` Packet builder/router.
- Recover default protobuf bytes for `StartCapture`, `StopCapture`, `TakePicture`, and `SyncCaptureMode` from the APK-generated Wire message adapters/classes.

## Step 46 - Command construction evidence from `seg08.dex`

Scope: APK-only reverse work under `F:\Insta360onWin` only.

Continued scanning code items for the confirmed app-level command values from Step 45. The strongest new evidence is in `seg08.dex`.

Start capture:

```text
seg08.dex 0x479f54
  const/16 v0, #186
  invoke-direct {v1, v0}, method@9e8c
  stores four instance fields after the base constructor call
```

This looks like a command object constructor that passes `186` into a base command constructor.

```text
seg08.dex 0x4728a4
  const/16 v6, #186
  invoke-virtual/range ..., method@9c6b
  later invokes method@9d11 with the same command value when a state branch sends/executes it
```

This is strong APK-only evidence that `PHONE_COMMAND_START_CAPTURE = 186` is passed into the command builder/send layer.

Start capture with parameters:

```text
seg08.dex 0x473304
  calls method@9c6b with a derived command code
  checks const/16 #187
  later invokes method@9d11
```

This likely handles `PHONE_COMMAND_START_CAPTURE_WITH_PARAM = 187`, but the exact parameter serialization still needs to be recovered.

Stop capture:

```text
seg08.dex 0x47e804
  categorizes constants 167, 198, 199
```

```text
seg08.dex 0x47ed54
seg08.dex 0x47f5a0
  invoke-static method@9cb8
  invoke-virtual method@9d55
  invoke-virtual method@9d57 with #178
  invoke-virtual method@9d5a with #89
  new-instance type@34f7
  invoke-virtual {builder, #199, object}, method@9d5e
  invoke-virtual method@9d5a with #87
  invoke-virtual method@9d58
  invoke-virtual method@9d5f
  invoke-virtual method@9d5a with #176
  invoke-virtual method@9d65
  invoke-virtual method@9d56
```

Both methods push `199` through the same builder family, so this is the current strongest APK-only sender evidence for `PHONE_COMMAND_STOP_CAPTURE = 199`.

Important caution:

- `seg08.dex 0x47b568` passes `197` into `method@9e8c`; this is not StopCapture. The full enum in Step 45 shows StopCapture is `199`.
- `method@9c6b`, `method@9d11`, `method@9cb8`, `method@9d5e`, and `method@9d56` are currently API-shape evidence only. The method definitions still need to be recovered before packet bytes can be generated safely.

Related raw class-name evidence from `seg11.dex`:

```text
Lcom/arashivision/camera/command/StartRecordCmd;
Lcom/arashivision/camera/command/StopRecordCmd;
Lcom/arashivision/camera/command/StopTakePictureCommand;
Lcom/arashivision/camera/command/CaptureCommand;
Lcom/arashivision/camera/command/HdrCaptureCommand;
```

Next target:

- Recover definitions or surrounding wrappers for `method@9c6b`, `method@9d11`, `method@9cb8`, `method@9d5e`, and `method@9d56`.
- Follow that command builder output until it reaches the `seg12.dex` Packet/UCD2 router at `0x55e378` or `0x55caec`.

## Step 47 - `seg08` command execution layer vs `seg12` Packet/UCD2 layer

Scope: APK-only reverse work under `F:\Insta360onWin` only.

Disassembled more `seg08.dex` command construction call sites.

`StartCapture` path:

```text
seg08.dex 0x4728a4
  const/16 #186
  invoke-virtual method@9c6b
  ...
  invoke-virtual method@9d11
```

`StartCaptureWithParam` path:

```text
seg08.dex 0x473304
  invoke-virtual method@9c6b
  checks const/16 #187
  ...
  invoke-virtual method@9d11
```

`StopCapture` path:

```text
seg08.dex 0x47ed54
seg08.dex 0x47f5a0
  invoke-virtual method@9d5e with const/16 #199
  ...
  invoke-virtual method@9d56
```

Additional `method@9d11` caller scan shows it is a shared camera-command execution API, not a one-off:

```text
0x472244, 0x4726f8, 0x47279c, 0x472828, 0x4728a4,
0x472944, 0x472c70, 0x472fbc, 0x473088, 0x473160,
0x473304, 0x47338c
```

Additional `method@9d56` caller scan shows it is the finalize/send method for another command-builder family:

```text
0x47058c, 0x47ed54, 0x47ef9c, 0x47f4fc, 0x47f5a0
```

`seg12.dex` scans for constants `186` and `199` found no direct Packet/UCD2 code item hits. `seg12.dex` still only exposes the lower Packet/UCD2 router:

```text
0x55e378 -> method@ee91 / method@ee60 -> method@ee50 / method@eed2
```

Interpretation:

- `186`, `187`, and `199` are proven at the app camera-command layer in `seg08.dex`.
- They are not directly written by the final Packet/UCD2 builder in `seg12.dex`.
- The missing bridge is therefore likely:

```text
seg08 camera command object/builder
  -> InstaCmdExe[] / camera command execution API
  -> insta360/messages/* Wire/protobuf serialization
  -> OneDriver / ondriver Packet
  -> seg12 Packet/UCD2 router at 0x55e378 or 0x55caec
```

Raw APK string evidence from original `classes.dex` and reconstructed segments also confirms the same class/message names:

```text
Lcom/arashivision/camera/command/InstaCmdExe;
Lcom/arashivision/onedriver/packet/Packet;
Lcom/arashivision/onedriver/encrypt/EncryptionManager;
Linsta360/messages/StartCapture;
Linsta360/messages/StopCapture;
Linsta360/messages/SyncCaptureMode;
```

Important caution:

- The original `classes.dex` header is shell/protector-like and reports only tiny string/type/method counts, while the raw body contains real protected strings. Standard DEX string/type lookup is therefore unreliable for these raw descriptors.
- Continue using code-item evidence, raw context windows, and previously captured RC4 preimage evidence instead of trusting damaged DEX metadata.

Next target:

- Recover the `InstaCmdExe[]` execution bridge and identify which method serializes `StartCapture` / `StopCapture` messages before passing bytes into OneDriver/Packet.
- If metadata stays damaged, infer serialization by extracting Wire message adapter/default-field constants from raw code and validating against APK-only class descriptors.

## Step 48 - StartCapture command object and message/default-field status

Scope: APK-only reverse work under `F:\Insta360onWin` only.

Disassembled the `seg08.dex` constructor-like code item for the StartCapture command object:

```text
seg08.dex 0x479f54
  const/16 #186
  invoke-direct method@9e8c
  iput-object field@46eb
  iput-object field@46ea
  iput-object field@46e8
  iput-object field@46e9
```

Interpretation:

- The object passes app command value `186` to a base command constructor.
- It stores four object fields supplied by the caller.
- No StartCapture protobuf bytes or default constants are hard-coded in this constructor.

Field-use scan for the StartCapture command object's four fields:

```text
0x479f20
  new-instance type@3513
  reads field@46eb/46ea/46e8/46e9
  invoke-direct method@9edd
  invoke-virtual method@9e92

0x479f54
  constructor, command value 186

0x479f80
  reads field@46eb/46ea/46e8/46e9
  invoke-virtual method@9d5d
  invoke-virtual method@9e8e
```

`0x479f80` is the best current execution/serialization candidate for this command object, because it sends the four stored fields into `method@9d5d` and then passes the result to `method@9e8e`.

`method@9de5` is rare and appears only in:

```text
0x46d0e4 wrapper -> method@9de5
0x4728a4 StartCapture path -> method@9de5 -> method@9c6b -> method@9d11
```

`method@9ddb` is much more general and is used by multiple command paths, including the `187` path.

Message constructor signature evidence from original APK raw `classes.dex`:

```text
(Linsta360/messages/SyncCaptureMode;Lokio/ByteString;)V
(Linsta360/messages/TakePicture$Mode;Linsta360/messages/ExtraMetadata;Ljava/util/List;Linsta360/messages/RawCaptureType;ZLinsta360/messages/SensorDevice;ILinsta360/messages/TriggerSource;Lokio/ByteString;)V
```

StartCapture and StopCapture did not appear with multi-field constructor signatures in the same constructor-signature block. Current inference from APK raw evidence:

- `TakePicture` definitely has a non-empty message body with several defaultable fields.
- `SyncCaptureMode` has a one-enum-field body.
- `StartCapture` / `StopCapture` may be empty or builder-only messages, but final packets still require the outer command envelope and cannot be generated from that inference alone.

Packet/UCD2 router confirmation from `seg12.dex 0x55e378`:

```text
0x55e378
  chunks large payloads
  calls method@ee60 for extended/encrypted packet path
  calls method@ee91 for standard internal Packet + UCD2 path
  dispatches through method@ee50 or method@eed2 depending on state
```

Important caution:

- `src/adapters/ucd2_client.rs` still contains older guessed message types like `0x0300` and `0x0301`. These are not APK-proven and should not be used as protocol evidence.
- The only generated frame currently safe from APK proof is the known device-info frame using `build_ucd2_frame(0x04, 0x10, DEVICE_INFO_PAYLOAD)`.

Next target:

- Continue from `0x479f80 -> method@9d5d -> method@9e8e` and `0x4728a4 -> method@9d11` to find where command value `186` becomes the onedriver `Message` envelope.
- Recover the envelope byte layout before enabling Start/Stop/TakePicture in the app.

## Step 49 - RC4 preimage code mirror and raw Packet builder evidence

Scope: APK-only reverse work under `F:\Insta360onWin` only.

Added `reverse_apk/tools/dex_callgraph_slice.py` to inspect code-item slices without trusting damaged DEX metadata. It can:

- scan code items by method/field/type/constant usage
- decode invoke register lists
- raw-disassemble code items from reconstructed DEX files or heap/preimage blobs

Important correction made during this step:

- Non-range invoke register decoding was fixed. For 35c invoke format, the first code-unit high nibble is the argument count and the next nibble is register G.
- This matters for correctly reading calls such as:

```text
0x479f80:
  method@9d5d regs=[v5, v0, v1, v2, v3]
  method@9e8e regs=[v4, v5]

0x4728a4:
  method@9de5 regs=[v0, v3, v4, v5, v6]
  method@9c6b regs=[v4, v6, v5]
  method@9d11 regs=[v4, v6, v5, v3, v0]

0x47ed54:
  method@9d5e regs=[v6, v0, v13] with v0 = 199
  method@9d56 regs=[v6]
```

The RC4 preimage file:

```text
reverse_apk/vm_heap_dumps/rc4_preimages/rc4_preimage_51b97500_0_6766307.bin
size 108421895
```

contains exact byte-for-byte mirrors of reconstructed code items:

```text
seg08.dex 0x479f54  -> preimage 0x3799a10  StartCapture command constructor
seg08.dex 0x479f80  -> preimage 0x3799a3c  StartCapture execute/serialize candidate
seg08.dex 0x4728a4  -> preimage 0x3792360  StartCapture path using 186
seg08.dex 0x47ed54  -> preimage 0x379e810  StopCapture path using 199
```

This proves the preimage is not merely unrelated string material; it preserves the same protected code blobs used by the reconstructed DEX segments.

The preimage also contains the Packet/UCD2 material:

```text
Packet                    @ 0x43f02f
MessageDemuxer            @ 0x52f7e0b
MessageMuxer              @ 0x52f7eb5
MessageContentType        @ 0x52f7d91 / 0x52f7dd3
MessageDirection          @ 0x52f7e3f / 0x52f7e7f
PacketEncryptionParams    @ 0x43f0ca / 0x52f80f4 / 0x5311e7c
UCD2-XOR-KEY-001          @ 0x51113d0
```

Raw code-item scan around `0x5110000..0x5113000` found many plausible Packet/encryption code items, including:

```text
0x5111394  static initializer for UCD2-XOR-KEY-001 byte array
0x511151c  UCD2 parser / fromBytes-style method, same logic as seg12.dex 0x563ae8
0x51120e8  internal Packet + UCD2 builder, same role as seg12.dex method@ee91 / code 0x565824
```

High-confidence raw facts from `0x51120e8`:

- It computes a base internal Packet header length of `9`.
- It writes an internal byte array before the outer UCD2 wrapping.
- It writes bytes 0..8 of the standard Packet header:

```text
byte 0..3  little-endian 32-bit value from a Packet parameter method call
byte 4     byte from static field@709a method@eeb3 result
byte 5..6  short/value derived from an argument
byte 7..8  short/value derived from another Packet parameter method call
```

This is the same standard internal Packet header region previously observed as:

```text
08 00 02 01 00 00 80 00 00
```

but the raw preimage disassembly shows the writer more directly than damaged metadata did.

The same builder also contains an extended-header branch:

```text
bytes 10..13 write a 32-bit value
bytes 14..15 are zero in that path
then bytes are copied/appended before construction of the final Packet object
```

Interpretation:

- The app has at least a standard 9-byte internal Packet header path and an extended 16-byte-ish path.
- The known LunaU device-info request uses the standard path.
- Camera-control commands may still require the onedriver `Message` envelope before entering this Packet builder.

Current bridge status:

- `0x479f80 -> method@9d5d -> method@9e8e` is now confirmed as a StartCapture command-object execution/serialization candidate with correct invoke registers.
- `0x4728a4 -> method@9c6b -> method@9d11` is now confirmed as the controller path that passes command value `186` plus generated objects into a shared execution API.
- `0x47ed54/0x47f5a0 -> method@9d5e(199, object) -> method@9d56` is the strongest StopCapture builder/finalize path.

Still missing:

- The exact onedriver `Message` envelope byte layout that carries command value `186/187/199` and the serialized `insta360/messages/*` body into the Packet payload.
- Without that envelope, generating Start/Stop/TakePicture packets would still be guessing.

Next target:

- Use the preimage code mirror offsets to keep tracing from `method@9d5d`, `method@9e8e`, `method@9d11`, `method@9d5e`, and `method@9d56` toward the Packet builder.
- Search for code items around the preimage `MessageMuxer` / `MessageDemuxer` code material, not just the string pool, and link that to the command execution APIs.

## 2026-07-06 Step 50 - Corrected preimage builder mapping and command-router evidence

Important correction to Step 49:

- `preimage 0x51120e8` is **not** the same code item as `seg12.dex method@ee91 / code 0x565824`.
- Exact byte matching shows:

```text
seg12.dex 0x565824 -> preimage 0x5113258  standard internal Packet + UCD2 builder, method@ee91
seg12.dex 0x5646b4 -> preimage 0x51120e8  extended/encrypted Packet builder candidate, method@ee60
seg12.dex 0x55e378 -> preimage 0x510bdac  upper Packet/UCD2 build-dispatch router
```

The stable mapping deltas are:

```text
seg08.dex -> rc4_preimage_51b97500...  delta 0x331fabc for tested command-layer code items
seg12.dex -> rc4_preimage_51b97500...  delta 0x4bada34 for tested Packet/UCD2 code items
seg06.dex -> rc4_preimage_51b97500...  delta 0x277638e for tested enum initializer code
```

Router evidence:

- `preimage 0x510bdac` / `seg12.dex 0x55e378` is an upper build-dispatch method.
- It invokes `method@ee60` twice in the early branch.
- It invokes `method@ee91` twice in the standard branch.
- It then dispatches through `method@ee50` or `method@eed2`.

Standard builder evidence:

- `preimage 0x5113258` / `seg12.dex 0x565824` is the standard internal Packet + UCD2 builder.
- It writes the `55 43 44 32` UCD2 magic in one branch.
- It uses a base internal header size of `9`.
- It calls `method@ee8b` before returning; this is the lower outer-frame construction call already visible from raw invoke registers.

Extended builder evidence:

- `preimage 0x51120e8` / `seg12.dex 0x5646b4` is the extended/encrypted builder candidate.
- It writes a longer internal header, including fields through bytes 10..15, then constructs a Packet-like object.
- This is still important for encrypted/control paths, but it must not be confused with the standard `ee91` path.

Command-router evidence from `seg08.dex --target-method 0x9d11`:

```text
0x4728a4  invokes 9de5, 9c6b, 9c71, 9dce, 9d11  consts include 186
0x473304  invokes 9ddb, 9c6b, 9d11              consts include 187
0x472828  invokes 9c6b, 9c69, 9d11              consts include 188
0x473088  invokes 9ddb, 9c6b, 9c6d, 9d11        consts include 197
0x4726f8  invokes 9c6d, 9c69, 9c6d, 9c6b, 9c71, 9d11  consts include 132 and 196
```

Interpretation:

- `method@9d11` is a shared camera-command execution API for multiple app commands.
- `0x4728a4` is still the strongest StartCapture controller path because it carries command value `186`.
- `0x473304` is the strongest StartCaptureWithParam controller path because it carries command value `187`.
- The StopCapture path remains strongest through `0x47ed54/0x47f5a0 -> method@9d5e(199, object) -> method@9d56`.

Negative result:

- A code-item scan around the `MessageMuxer`/`MessageDemuxer` string-pool region `0x530b000..0x5313000` found no plausible code items.
- Therefore the code that implements the muxer is not physically adjacent to those strings in this preimage; keep following code references and code-shape matches instead of assuming string locality.

Next target:

- Inspect the shared `method@9d11` caller shapes and adjacent command-builder methods to identify the exact object/envelope passed into the `seg12` Packet build router.
- Continue treating `0x510bdac -> ee91/ee60 -> ee8b` as the confirmed lower Packet/UCD2 construction chain.

## 2026-07-06 Step 51 - Start/Stop command builder context and protected-data negative results

Added helper:

```text
reverse_apk/tools/dex_hit_context.py
```

Purpose:

- Given a known `code_off`, print only the context around target method invokes or constants.
- This avoids dumping multi-thousand-instruction dispatcher methods while preserving the nearby register layout.

Confirmed StartCapture object serialization template:

```text
seg08.dex 0x479f80
  field@46eb -> v0
  field@46ea -> v1
  field@46e8 -> v2
  field@46e9 -> v3
  invoke method@9d5d {v5, v0, v1, v2, v3}
  invoke method@9e8e {v4, v5}
```

Interpretation:

- `0x479f80` builds or mutates a command body through `9d5d`.
- The result is then passed to `9e8e`, which remains the strongest per-object serialize/send/finalize candidate for this command-object family.

Confirmed StopCapture template, second copy:

```text
seg08.dex 0x47f5a0
  const 178
  invoke method@9d57 {v6, v2, v13, v0, v1}
  const 89
  invoke method@9d5a {v6, v13}
  new-instance type@34f7
  invoke method@9d39 {v13}
  const 199
  invoke method@9d5e {v6, v0, v13}
  const 87
  invoke method@9d5a {v6, v0}
  ...
  invoke method@9d65 {v11, v13, v14}
  invoke method@9d56 {v11}
```

This matches the earlier `0x47ed54` StopCapture path, with the same command value `199` and the same finalize method `9d56`.

Small `9d5e -> 9e8e` template:

```text
seg08.dex 0x47a014
  field@46d2 -> v0
  field@46ec -> v1
  invoke method@9eea {v1}
  move-result-object v1
  invoke method@9d5e {v3, v0, v1}
  invoke method@9e8e {v2, v3}
```

This proves `9d5e` is not StopCapture-only. It is a generic command/body writer used by multiple command-object serializers before `9e8e`.

Large dispatcher evidence:

```text
seg08.dex 0x46a838
  pc 0x02a2: const 187; branch into method@9c7e
  pc 0x0521: invoke method@9d5d {v0, v4, v3, v5, v1}
  pc 0x03fc/0x043b/0x043e/0x0446/0x0460/0x0604: invoke method@9d5e ...
```

Interpretation:

- `0x46a838` is a broad command dispatcher/body mapper.
- It includes `187`, so it is relevant to `PHONE_COMMAND_START_CAPTURE_WITH_PARAM`.
- It contains the same `9d5d` and `9d5e` writer family seen in the smaller command-object serializers.
- It did **not** expose command `186` in the tested constant search, so plain StartCapture remains best represented by `0x4728a4` and `0x479f80`.

Protected-data search results:

- `reverse_apk/classes.dex` is a valid DEX at offset 0, but its header reports only `0x79` string ids and `1` class_def while the file is about `0x72ad448` bytes.
- Target packet strings such as `Message(messageMethod=` and `PacketEncryptionParams(scheme=` are in the large appended/protected data region, not in normal DEX metadata.
- Searching for nested DEX magics inside this file found only the root `dex\n035\0` at offset `0x0`.
- Raw code-item scans around these protected string regions found no plausible code items:

```text
classes.dex 0x5418000..0x5421000  no plausible code_items
classes.dex 0x5100000..0x5120000  no plausible code_items
classes.dex 0x52f0000..0x5320000  no plausible code_items
```

Tooling note:

- No local `jadx`, `apktool`, or `java` executable was found from the project environment during this pass.
- Continue with APK-only byte/code scanning tools already in `reverse_apk/tools`.

Current packet-chain model:

```text
camera command API
  -> seg08 command writer family: 9d5d / 9d5e / 9d5a / 9d57 / 9d65
  -> per-object finalizer: 9e8e or builder finalizer 9d56
  -> protected Packet router: seg12 0x55e378 / preimage 0x510bdac
  -> standard builder ee91 at seg12 0x565824 / preimage 0x5113258
  -> extended/encrypted builder ee60 at seg12 0x5646b4 / preimage 0x51120e8
  -> lower UCD2 frame builder ee8b
```

Still missing:

- The exact bytes produced by `9d5d/9d5e/...` for command bodies `186`, `187`, and `199`.
- The exact bridge from `seg08` command writer output into the `seg12` Packet router, because damaged DEX metadata prevents reliable class/method name resolution.

## 2026-07-06 Step 52 - Static final-packet search

Searched for known verified LunaU UCD2 bytes across:

- `reverse_apk/classes.dex`
- `reverse_apk/vm_heap_dumps/rc4_preimages/rc4_preimage_51b97500_0_6766307.bin`
- `reverse_apk/reconstructed_dex/seg*.dex`

Needles:

```text
device_info_payload:
08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b

device_info_frame:
55 43 44 32 01 0c 04 10 0f 00 00 00 08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b 7c 00 8e 7c

auth_frame:
55 43 44 32 01 0c 05 0f 00 00 00 00 37 05 47 7c
```

Results:

- No hit for the full known auth frame.
- No hit for the full known device-info frame.
- No hit for the known 15-byte device-info internal payload.
- `UCD2-XOR-KEY-001` still appears at:

```text
classes.dex                              0x521dc17
rc4_preimage_51b97500_0_6766307.bin      0x51113d0
seg12.dex                                0x56399c
```

Interpretation:

- The known working UCD2 packets are generated dynamically by builder code, not copied from a static byte array.
- Therefore the final camera-control packets for `186/187/199` also need to be recovered by following the writer/envelope code or by instrumenting builder output.

Next target:

- Recover enough writer semantics from `9d5d`, `9d5e`, `9d5a`, `9d57`, `9d65`, `9e8e`, and `9d56` to reproduce the command body bytes.
- Then feed those bytes into the confirmed `seg12 0x55e378 -> ee91/ee60 -> ee8b` Packet/UCD2 chain.

## 2026-07-06 Step 53 - Writer family evidence points to Wire/protobuf command envelopes

Added helper:

```text
reverse_apk/tools/dex_safe_method_defs.py
```

Purpose:

- Safely scan normal `class_def` / `class_data` mappings with strict bounds.
- Avoid the previous issue where damaged class metadata could make a class-data scan hang.

Definition scan results:

```text
seg08 method@9d5a/9d5d/9d5e/9d57/9d65/9d56/9e8e  hits=0
seg12 method@ee91/ee60/ee8b/edc4                         hits=0
seg08 limited scan 0x460000..0x482000 for writer ids       hits=0
```

Interpretation:

- These method ids cannot currently be resolved through normal class metadata in the reconstructed DEX files.
- Continue using code-item/call-shape evidence rather than class/method names for this protected area.

Wire/protobuf evidence from APK-only strings:

```text
classes.dex contains:
Lcom/squareup/wire/ProtoAdapter;
Lcom/squareup/wire/ProtoWriter;
(Lcom/squareup/wire/ProtoWriter;ILjava/lang/Object;)V
(Lcom/squareup/wire/ReverseProtoWriter;ILjava/lang/Object;)V
unknownFields
Message(messageMethod=
Message(type=
```

Important caveat:

- `seg08.dex` itself does not expose readable `ProtoWriter` descriptors.
- Therefore `method@9d5e` cannot yet be directly named as `ProtoAdapter.encodeWithTag`.
- However, the app contains Wire/protobuf runtime signatures, and the command serializers use the same pattern of integer tags plus typed values, so Wire/protobuf remains the strongest body-envelope model.

StopCapture writer template, refined:

```text
seg08.dex 0x47ed54 / 0x47f5a0
  create builder/object via method@9cb8 with const 4106
  method@9d55(builder)
  const 178
  method@9d57(builder, 178, object/string-like args)
  const 89
  method@9d5a(builder, 89)
  new-instance type@34f7
  method@9d39(empty/body object)
  const 199
  method@9d5e(builder, 199, empty/body object)
  const 87
  method@9d5a(builder, 87)
  ...
  method@9d5f(builder, empty/body object)
  const 176
  method@9d5a(builder, 176)
  method@9d65(builder, empty/body object, int-like value)
  method@9d56(builder)
```

Field/tag distribution:

```text
4106  appears in StopCapture-related templates: 0x47ed54, 0x47ef9c, 0x47f5a0
199   appears in StopCapture templates before method@9d5e
187   appears in StartCaptureWithParam-related path 0x4780ec and dispatcher 0x46a838
178   appears in StopCapture templates and other message builders through method@9d57
179   appears in sibling templates through method@9d57
89    appears before method@9d5a in several command templates
176   appears near finalization before method@9d65/method@9d56
```

StartCaptureWithParam-related path:

```text
seg08.dex 0x4780ec
  const 187
  if-ne branch
  new-instance type@34f7
  method@9d39(empty/body object)
  new-instance type@49c
  const 3
  method@b77(...)
  method@c99(...)
  method@9d5f(...)
  ...
  method@9d6f(...)
  method@9e5b(...)
```

Interpretation:

- `187` is not only an enum constant; it gates a dedicated StartCaptureWithParam serialization path.
- `type@34f7` is repeatedly used as an empty/body message object in command writers.
- `9d5a/9d5d/9d5e/9d57/9d5f/9d65/9d6f/9e5b/9e8e/9d56` form the command-body/envelope writer family.

Current confidence model:

```text
App command value / oneof-like selector:
  186 StartCapture              -> 0x4728a4 controller path + 0x479f80 object serializer
  187 StartCaptureWithParam     -> 0x473304 controller path + 0x4780ec / 0x46a838 serializer evidence
  199 StopCapture               -> 0x47ed54 and 0x47f5a0 writer templates

Body/envelope format:
  high confidence: Wire/protobuf-family generated message envelope
  not yet proven: exact encoded bytes for each command body
```

Next target:

- Build a small static register-value tracer for specific methods such as `0x47ed54`, `0x47f5a0`, `0x4780ec`, and `0x479f80`.
- The tracer should annotate each writer-family invoke with known constant arguments and unknown object sources.
- Use that to derive a minimal candidate protobuf envelope for `199` first, because StopCapture has the clearest empty-body path.

## 2026-07-06 Step 54 - Register tracing for StopCapture and StartCapture paths

Added helper:

```text
reverse_apk/tools/dex_reg_trace.py
```

Purpose:

- Track lightweight register sources inside one `code_item`.
- Handles constants, moves, strings, new-instance, instance-field reads, move-result, and invoke register lists.
- This is not a full decompiler; it is a focused evidence tool for writer-family calls.

StopCapture trace: `seg08.dex 0x47ed54`

```text
method@9cb8 {v0, v1, v2, v3, v4, v5}
  v0=v13
  v1=const(4106)
  v2=string@6fc
  v3=string@9be
  v4=const(0)
  v5=const(0)

method@9d57 {v6, v2, v13, v0, v1}
  v6=result(method@9cb8)
  v2=const(178)
  v13=unknown object/source
  v0=string@6fb
  v1=string@98e9

method@9d5a {v6, v13}
  v6=result(method@9cb8)
  v13=const(89)

method@9d5e {v6, v0, v13}
  v6=result(method@9cb8)
  v0=const(199)
  v13=new(type@34f7)

method@9d5a {v6, v0}
  v6=result(method@9cb8)
  v0=const(87)

method@9d5a {v6, v13}
  v6=result(method@9cb8)
  v13=const(176)

method@9d65 {v6, v13, v14}
  v6=result(method@9cb8)
  v13=result(method@684)
  v14=const(0)

method@9d56 {v6}
  v6=result(method@9cb8)
```

StopCapture trace: `seg08.dex 0x47f5a0`

- Same fixed sequence as `0x47ed54`.
- Difference: after a helper call, the builder is copied/aliased into `v11`, so the tail calls use `v11` instead of `v6`.
- Constants and object sources remain the same:

```text
4106, string@6fc, string@9be, 178, string@6fb, string@98e9, 89, 199 + new(type@34f7), 87, 176, result(method@684), 0
```

Plain StartCapture controller trace: `seg08.dex 0x4728a4`

```text
method@9de5 {v0, v3, v4, v5, v6}
  v0=field@43c5

method@9c6b {v4, v6, v5}
  v6=const(186)

method@9c71 {v4, v5}
  v5=const(0)

method@9d11 {v4, v6, v5, v3, v0}
  v6=const(186)
  v5=const(0)
  v3=result(method@9dce)
  v0=field@43c5
```

StartCaptureWithParam controller trace: `seg08.dex 0x473304`

```text
method@9ddb {v0, v5}
  v0=field@43c5

method@9c6b {v0, v4, v1}

method@9d11 {v0, v4, v1, v5, v2}
  v5=const(187)
  v2=const(3)
```

StartCapture object serializer trace: `seg08.dex 0x479f80`

```text
method@9d5d {v5, v0, v1, v2, v3}
method@9e8e {v4, v5}
```

The object serializer only reads instance fields before invoking `9d5d`, so this trace cannot resolve concrete constants without reconstructing the object constructor and field assignments. The earlier constructor evidence still shows the associated command value is `186`.

Current concrete result:

- StopCapture has the clearest APK-only body construction trace so far.
- The app constructs a command/envelope builder with `4106`, appends metadata-like fields `178/89/87/176`, and places command/body selector `199` with an empty/body object `type@34f7`.
- Plain StartCapture and StartCaptureWithParam are confirmed at the controller API level with `186` and `187`, but their full body bytes still require object-field reconstruction or runtime builder instrumentation.

Next target:

- Use `dex_reg_trace.py` on constructor/field-assignment methods for the `0x479f80` serializer cluster, especially `0x479f54`, to recover StartCapture field values.
- Try to identify whether `type@34f7` serializes to an empty message (`length 0`) or carries unknown fields through `method@9d39`.
- Once StopCapture body bytes are hypothesized, wrap them through the confirmed `seg12` Packet/UCD2 builder chain and test only if clearly marked as candidate.

## 2026-07-06 Step 55 - StartCapture field mapping and empty-body candidate

Improved `reverse_apk/tools/dex_reg_trace.py`:

- It now prints `iput*` field writes.
- It also distinguishes `iget*` reads as `field@xxxx(object-source)`.

StartCapture constructor: `seg08.dex 0x479f54`

```text
const 186
method@9e8c {v1, v0}
  v0=const(186)

iput field@46eb obj=v1 src=v2
iput field@46ea obj=v1 src=v3
iput field@46e8 obj=v1 src=v4
iput field@46e9 obj=v1 src=v5
```

Interpretation:

- This constructor initializes the base/parent command object with command value `186`.
- It then stores four constructor arguments into:

```text
field@46eb <- arg1
field@46ea <- arg2
field@46e8 <- arg3
field@46e9 <- arg4
```

StartCapture serializer: `seg08.dex 0x479f80`

```text
method@9d5d {v5, v0, v1, v2, v3}
  v5=writer/builder parameter
  v0=field@46eb(this)
  v1=field@46ea(this)
  v2=field@46e8(this)
  v3=field@46e9(this)

method@9e8e {v4, v5}
```

Interpretation:

- The body serializer forwards the same four constructor arguments, in order, to `method@9d5d`.
- The command id `186` is not passed directly to `9d5d`; it is carried by the base command initialization via `method@9e8c`.

Sibling method: `seg08.dex 0x479f20`

```text
new(type@3513)
method@9edd {v5, field@46eb, field@46ea, field@46e8, field@46e9}
method@9e92 {v5, this}
```

Interpretation:

- This is likely a string/equals/hash/size-style helper over the same four fields.
- It confirms the field cluster belongs together and should be treated as the StartCapture parameter set.

Empty/default body candidate: `type@34f7`

Observed cache/getter method: `seg08.dex 0x47a074`

```text
if field@46ed exists: return it
new-instance type@34f7
method@9d39 {new(type@34f7)}
iput field@46ed <- new(type@34f7)
return field@46ed
```

Observed constructor/setter around the same field:

```text
seg08.dex 0x47a0dc
  method@9e8c {this, const(-1)}
  iput field@46ed <- constructor arg

seg08.dex 0x47a11c
  iput field@46ed <- const(0)
```

Interpretation:

- No payload field writes were observed for the cached `type@34f7` instance.
- The strongest current reading is that `type@34f7` is an empty/default message object, or a message object whose unknown fields are empty after `method@9d39`.
- Therefore StopCapture's `method@9d5e(builder, 199, new(type@34f7))` likely encodes command selector/body `199` with an empty/default body.

Envelope-family comparison:

StopCapture-like `4106` template: `seg08.dex 0x47ef9c`

```text
method@9cb8 {v0, v1, v2, v3, v4, v5}
  v1=const(4106)
  v2=string@6fc
  v3=string@a2a
  v4=const(0)
  v5=const(0)

method@a0d1(...)
method@9d5a(builder, 176)
method@9d65(builder, result(method@a0d1), const(3))
method@9d56(builder)
```

Sibling `4104` template: `seg08.dex 0x47f4fc`

```text
method@9cb8 {v0, v1, v2, v3, v4, v5}
  v1=const(4104)
  v2=string@14ab
  v3=string@9b4
  v4=const(0)
  v5=const(0)

method@a0d1(...)
method@9d57(builder, 179, object, string@6fb, string@98e9)
method@9d5a(builder, 177)
method@9d65(builder, result(method@a0d1), const(0))
method@9d56(builder)
```

Interpretation:

- `9cb8 -> 9d65 -> 9d56` is a reusable command/envelope builder family.
- `4104` and `4106` are sibling business/message ids in that family.
- StopCapture's `4106 + 199 + empty/default type@34f7` path is now the clearest candidate for reconstructing a first control packet body.

Still missing:

- Exact byte output of `9cb8`, `9d57`, `9d5a`, `9d5e`, `9d65`, and `9d56`.
- Concrete runtime values for the two string references in damaged `seg08` (`string@6fc`, `string@9be`, etc.).
- Four StartCapture constructor argument values from its call sites.

Next target:

- Search for constructor call sites for the `0x479f54` StartCapture object indirectly via field cluster or nearby class shape.
- Try to recover string values for `string@6fc/string@9be/string@6fb/string@98e9` from the protected data/string pools.
- If string recovery stalls, proceed with a carefully marked StopCapture candidate encoder using known constants and empty body, then validate only by a controlled raw UCD2 test.

## 2026-07-06 Step 56 - Damaged string indexes around command envelope

Added helper:

```text
reverse_apk/tools/dex_string_probe.py
```

Purpose:

- Print declared `string_ids_size` / `string_ids_off`.
- For selected string indexes, print the raw `string_data_off`, ULEB length, UTF-8 payload if plausible, and nearby printable context.

Probe result: `seg08.dex`

```text
string_ids_size=0xdbf2 string_ids_off=0x70

string@6fc
  data_off=0xfb6d
  declared_utf16_len=0
  payload text is only damaged bytes: '\ufffd\t'

string@9be
  data_off=0x2d1a0022
  outside file

string@6fb
  data_off=0x10702eb7
  outside file

string@98e9
  data_off=0x08040a06
  outside file

string@a2a
  data_off=0xe0001
  decodes to unrelated-looking '/protocol/Geo$Deserializer;'

string@14ab
  data_off=0x2fc6e
  binary payload, not a normal Java/Kotlin string

string@9b4
  data_off=0x10720002
  outside file
```

Probe result: root `classes.dex`

```text
string_ids_size=0x79
string@6fc / 9be / 6fb / 98e9 are outside the declared string table
```

Probe result: `seg12.dex`

```text
selected string indexes from the Packet router point outside the file
```

Interpretation:

- The reconstructed DEX string metadata in these protected regions is damaged or intentionally misleading.
- The `string@...` operands in `seg08` command-envelope code cannot currently be resolved through ordinary DEX `string_ids`.
- Therefore the envelope constants and object flow are reliable code evidence, but the string arguments must remain unresolved until a better protected-string recovery path is found.

Updated StopCapture status:

- Reliable:
  - `9cb8(..., 4106, string@6fc, string@9be, 0, 0)` creates the envelope builder.
  - `9d5e(builder, 199, new(type@34f7))` writes the StopCapture selector/body.
  - `type@34f7` is the strongest empty/default body candidate.
  - `9d56(builder)` finalizes the envelope.
- Unresolved:
  - Exact byte value produced by unresolved string operands.
  - Exact byte output of each writer method.

Next target:

- Search protected string decoding notes/tools already present in `ASHIELD_UNPACK_NOTES.md` and `ashield_decode_strings.py` for a way to map these damaged `string@...` operands to recovered strings.
- If that does not recover them, focus on dynamic/emulated writer output from code paths rather than string-table reconstruction.

## 2026-07-06 Step 57 - ASHIELD string decode and binary-output search results

Checked existing ASHIELD materials:

- `reverse_apk/ASHIELD_UNPACK_NOTES.md`
- `reverse_apk/tools/ashield_decode_strings.py`
- `reverse_apk/vm_heap_dumps/trace_vm_regs.log`
- `reverse_apk/vm_heap_dumps/trace_parent_stack.log`

Results:

- Running `ashield_decode_strings.py` and filtering for protocol terms did **not** reveal `Packet`, `Message`, `UCD2`, `PHONE_COMMAND`, `Proto`, `Wire`, `4106`, `4104`, `camera`, or command-envelope strings.
- VM trace logs also did not contain those protocol terms or direct writer ids such as `9cb8` / `9d5e`.
- Current ASHIELD notes still indicate the native unpacking path is blocked around worker/out_ctx mode setup, so there is no newly recovered full DEX/string table from that route yet.

Added helper:

```text
reverse_apk/tools/binary_pattern_probe.py
```

Purpose:

- Search one or more binary files for hex/ascii needles.
- Print hit offsets and nearby hex/printable context.

Searched for StopCapture constants in:

- `rc4_preimage_51b97500_0_6766307.bin`
- `heap_51981980_6766307_alloc_0xe070.bin`
- `reconstructed_dex/seg08.dex`
- forced-mode mmap dumps under `vm_heap_dumps/force_mode2_normalized`

Needles included:

```text
4106 little-endian: 0a 10
4106 big-endian-ish: 10 0a
4106 varint: 8a 20
199 varint: c7 01
candidate pair: 8a 20 c7 01
candidate tag run: b2 01 59 c7 01 57 b0 01
```

Results:

- Single constants have many hits, mostly code/data noise.
- The combined candidate sequences were not found:

```text
8a 20 c7 01                         no hits
b2 01 59 c7 01 57 b0 01             no hits
```

Interpretation:

- The final writer output has not been captured as a simple static byte sequence in the current heap/preimage/dump files.
- Blind byte search is no longer likely to be productive without a more accurate candidate encoding or a runtime/emulated writer output.

Code-side cross-check:

`seg08.dex --target-const 199`:

- `0x47ed54` and `0x47f5a0` are the two concrete StopCapture writer templates carrying:

```text
4106, 178, 89, 199, 87, 176
```

- `0x47e804` and `0x47f6dc` contain `199` but are enum/table/control mappings, not the same envelope builder.
- `0x42a51c` contains unrelated constants including `199`.

`seg08.dex --target-const 4106`:

- `0x47ed54` and `0x47f5a0` contain both `4106` and `199`.
- `0x47ef9c` contains `4106` and the same `9cb8 -> 9d65 -> 9d56` envelope family, but not command selector `199`.

Updated conclusion:

- StopCapture's command-envelope path remains high confidence:

```text
0x47ed54 / 0x47f5a0:
  9cb8(..., 4106, unresolved strings, 0, 0)
  9d57(..., 178, unresolved strings)
  9d5a(..., 89)
  9d5e(..., 199, empty/default type@34f7)
  9d5a(..., 87)
  9d5a(..., 176)
  9d65(...)
  9d56(...)
```

- Exact packet bytes are still missing because the writer output and unresolved string operands are not recovered.
- The next productive path is not broad search; it is either:
  - emulate/instrument the writer methods enough to dump the output of `9d56(builder)`, or
  - implement a candidate encoder from the observed Wire/protobuf-style envelope and test it as explicitly experimental.

Next target:

- Inspect local app/Rust code for any existing raw UCD2 test harness where a candidate StopCapture body could be wrapped safely after the confirmed UCD2 handshake.
- In parallel, continue static recovery of `9cb8/9d56` semantics by looking for lower-level byte-array or writer calls reachable from the envelope-builder family.

## 2026-07-06 Step 58 - Local app harness and stale UCD2 client check

Checked the current Rust/HTML app entry points:

- `src/bin/html_app.rs`
- `src/adapters/luna_local.rs`
- `src/adapters/ucd2_client.rs`
- `src/adapters/mod.rs`
- `web/index.html`

Result:

- The HTML app uses `luna_local.rs` for the tested LunaU path.
- `luna_local.rs` already uses the recovered APK/evidence-backed UCD2 outer frame:
  - magic `UCD2`
  - version `01`
  - header length `0c`
  - two message-type bytes
  - little-endian payload length
  - recovered non-reflected checksum tail
- `html_app.rs` keeps a persistent `Ucd2RawSession`, so it matches the real-device finding that the socket must stay open.
- `ucd2_client.rs` still contains an older guessed client with a 16-byte header and guessed message IDs such as `0x0301 StopCapture`.
- `ucd2_client.rs` is **not** exported by `src/adapters/mod.rs`, so it is not part of the current HTML app control path.

Risk note:

- Do not reuse `ucd2_client.rs` for LunaU control unless it is rewritten or clearly deprecated. It conflicts with the APK/evidence-backed outer UCD2 frame now proven on device.

## 2026-07-06 Step 59 - Method table/proto table probe

Added helper:

```text
reverse_apk/tools/dex_method_probe.py
```

Purpose:

- Print method table metadata for target `method@...` indexes.
- Decode class/type/name/proto when the reconstructed DEX tables are usable.
- Report explicit out-of-range/corrupt table reads when they are not usable.

Tested on StopCapture writer methods:

```text
9cb8, 9d55, 9d57, 9d5a, 9d5e, 9d5f, 9d65, 9d56
```

Result:

- The method table/proto table around these method ids is corrupted or shifted.
- Example outputs decode class/proto/name indexes into impossible values such as:
  - `type@746e outside size=0x474e`
  - `string@26003b6e outside size=0xdbf2`
  - `proto@6f69 outside size=0x2976`
- The decoded bytes resemble accidental ASCII fragments from nearby strings rather than real DEX table entries.

Interpretation:

- Direct method signature recovery from the reconstructed `seg08.dex` method table is not reliable for the StopCapture writer family.
- The raw code stream and invoke/register shapes remain reliable, but method/proto names must not be treated as evidence.

## 2026-07-06 Step 60 - 199 field-number vs enum-value static check

Reason:

- StopCapture calls `method@9d5e {builder, const(199), empty type@34f7}`.
- `199` is also proven as `PHONE_COMMAND_STOP_CAPTURE`.
- In Wire/protobuf-style code, a method shaped like `encodeWithTag(writer, int, value)` could mean the integer is a field tag, not a command enum value.

Searched for byte patterns that would appear if `199` were encoded directly as:

```text
c7 01          varint value 199
ba 0c          protobuf field tag for field 199, length-delimited
ba 0c 00       field 199 with zero-length body
```

Also searched combined candidates:

```text
b2 05 ba 0c
ba 0c c7 01
59 00 ba 0c
b2 01 59 ba 0c
```

Files searched:

- `reverse_apk/vm_heap_dumps/rc4_preimages/rc4_preimage_51b97500_0_6766307.bin`
- `reverse_apk/reconstructed_dex/seg08.dex`

Result:

- Single values have many noisy hits in code/data regions.
- Combined candidate sequences had no hits:

```text
b2 05 ba 0c        no hits
ba 0c c7 01        no hits
59 00 ba 0c        no hits
b2 01 59 ba 0c     no hits
```

Interpretation:

- Static byte-search still does not expose the final StopCapture payload.
- The APK writer output is probably only materialized at runtime or inside protected/emulated code state not captured as a clean static byte sequence.
- The next productive path is to reconstruct or emulate the narrow writer family:
  - `9cb8` creates the command/envelope builder.
  - `9d57`, `9d5a`, `9d5e`, `9d5f`, `9d65` add fields/body/unknown-fields-like data.
  - `9d56` finalizes the builder.

Current StopCapture packet status:

- UCD2 outer framing is solved.
- The command enum and StopCapture writer template are solved.
- The exact internal Packet payload bytes are still not solved.
- Do not send generated StopCapture packets yet.

## 2026-07-06 Step 61 - Emulator forced-mode rerun

Ran:

```text
python reverse_apk/tools/ashield_vm_emulate.py
  --sync-pthreads
  --native-body-check
  --force-chunk-mode 2
  --normalize-huge-src-len
  --normalize-vm-huge-length
  --skip-invalid-worker
  --zero-invalid-u32
  --dump-dir reverse_apk/vm_heap_dumps/run_step61
  --max-insn 1200000
```

Result:

- First worker copied/mapped:

```text
src=0x7310adcb len=0x150204 -> heap_51b98000_150204_mmap.bin
```

- Second worker normalized and copied/mapped:

```text
old src_len=0xd19c306e
new src_len=0x000e0102
src=0x7325afd7 len=0xe00fe -> heap_51cea000_e00fe_mmap.bin
```

- Third worker reached:

```text
old src_len=0x32093002
candidate/new length=0x1a0f1006
```

- This caused emulated heap exhaustion when the code attempted to map/copy about `0x1a0f1002` bytes.

Searched `run_step61` dumps for:

```text
Packet
EncryptionManager
UCD2
UCD2-XOR-KEY-001
dex\n
StopCapture
PHONE_COMMAND_STOP_CAPTURE
```

Result:

- No hits in the new forced-mode dumps.

Interpretation:

- Forced mode still does not produce clean Packet/UCD2 protected material.
- Third worker descriptor/length remains wrong or needs another normalization rule.
- Simply increasing heap would only copy a huge likely-invalid region and is not a good next step.

## 2026-07-06 Step 62 - Experimental huge-worker skip

Modified:

```text
reverse_apk/tools/ashield_vm_emulate.py
```

Added experimental option:

```text
--skip-huge-worker-len <threshold>
```

Behavior:

- At worker call site `0x2A118`, read the effective source length.
- If it is above the threshold, make the inline worker return `0` before it enters the chunk parser.
- Default behavior is unchanged.

Ran:

```text
python reverse_apk/tools/ashield_vm_emulate.py
  --sync-pthreads
  --native-body-check
  --force-chunk-mode 2
  --normalize-huge-src-len
  --normalize-vm-huge-length
  --skip-invalid-worker
  --skip-huge-worker-len 0x02000000
  --zero-invalid-u32
  --dump-dir reverse_apk/vm_heap_dumps/run_step62
  --max-insn 1600000
```

Result:

- Third worker was skipped:

```text
skip huge worker len=0x1a0f1006 threshold=0x02000000 data=0x7333b0d9
```

- Parent-side execution then eventually hit an unhandled CPU exception at stack address:

```text
pc=0x701ffef0
```

Interpretation:

- Skipping the huge worker avoids the mmap/heap explosion, but it does not preserve the parent/result chain.
- This confirms the third worker's output is structurally needed.
- The next productive target is not "skip the third worker"; it is to recover why the third worker descriptor length becomes `0x1a0f1006`.

Updated next target:

- Trace parent descriptor writes for the third worker:
  - worker arg around `0x50a96480`
  - source around `0x7333b0d5..0x7333b0d9`
  - parent state PCs `0x2d5ac`, `0x2d7dc`, `0x2d824`, and nearby writes into the worker-array allocation.
- Compare third-worker descriptor bytes against the first two worker descriptors to infer the correct length/source field positions.

## 2026-07-06 Step 63 - Third worker descriptor comparison

Inspected `run_step62` worker-array dump:

```text
reverse_apk/vm_heap_dumps/run_step62/heap_50a96460_1101080_alloc_0xa4570.bin
```

The first three 16-byte descriptors are:

```text
0x50a96460: out=0x50000020 src=0x7310adc3
0x50a96470: out=0x50000020 src=0x7325afcf
0x50a96480: out=0x50000020 src=0x7333b0d5
```

Decoded source bytes from protected `classes.dex` body:

Worker 1 source:

```text
ptr=0x7310adc3 bodyrel=0x10adc3
u32: 00150208 06766307 00008705 ...
```

Worker 1 parser data (`src+8` in the successful forced-mode path):

```text
ptr=0x7310adcb bodyrel=0x10adcb
u32: 00008705 a7f72072 001f000d ...
```

Worker 2 source:

```text
ptr=0x7325afcf bodyrel=0x25afcf
u32: d19c306e 000e0102 0002000e ...
```

Worker 2 parser data after normalization:

```text
ptr=0x7325afd7 bodyrel=0x25afd7
u32: 0002000e 00000006 00000000 ...
```

Worker 3 source:

```text
ptr=0x7333b0d5 bodyrel=0x33b0d5
u32: 32093002 1a0f1006 4a013002 1a16100e ...
ascii-ish: .0.2.....0.J.....0.2.....0.J...
```

Worker 3 parser data (`src+4`/`src+8` candidates) is also code-like:

```text
ptr=0x7333b0d9 bodyrel=0x33b0d9
u32: 1a0f1006 4a013002 1a16100e 32093002 ...
```

Interpretation:

- Worker 3 is not pointing at a normal chunk descriptor like worker 1/2.
- The bytes at worker 3 source look like protected/reconstructed code stream, not a length-prefixed chunk.
- Therefore the bad `0x1a0f1006` length is probably not a value to normalize; it is a symptom that the parent descriptor source pointer is wrong.
- The likely reason is that the forced-mode handling for worker 1/2 does not reproduce their real output/result side effects. The parent then constructs worker 3 from an invalid downstream state.

Updated conclusion:

- `--force-chunk-mode 2` is useful for producing exploratory dumps, but it is not a faithful unpack path.
- The next true fix must recover why `out_ctx+0x50` / handler mode remains empty and why workers are forced into the wrong chunk path.
- The current emulator path still cannot yield the final Packet/UCD2 command bytes.

## 2026-07-06 Step 64 - Worker descriptor writes and invalid third worker source

Ran a focused parent/worker trace:

```text
python reverse_apk/tools/ashield_vm_emulate.py
  --sync-pthreads
  --native-body-check
  --trace-parent
  --trace-worker-array-writes
  --trace-threads
  --trace-handlers
  --trace-reader
  --max-insn 260000
```

Important worker descriptor writes:

```text
slot 0 off 0x0 <- 0x50000020
slot 0 off 0x8 <- 0x7310adc3

slot 1 off 0x0 <- 0x50000020
slot 1 off 0x8 <- 0x7325afcf

slot 2 off 0x0 <- 0x50000020
slot 2 off 0x8 <- 0x144c1e041
```

Key observations:

- Descriptor writes happen at native PC `0x2c95c`.
- The descriptor value comes through `sp570` / register `x11`.
- Without forced mode, the third worker source becomes unmapped pointer `0x144c1e041`.
- With earlier forced-mode exploration, the third worker source became `0x7333b0d5` and pointed into code-like bytes. The two different bad sources have the same root cause: worker 1/2 returned failure/zero and parent state became polluted.

First worker handler state:

```text
trace handler vcall object=0x50000020 vtbl=0xf57a0 fn18=0x40900
trace handler match-ok=01 mode=0x00000000
trace chunk-mode mode=0x00000000 orig=0x00150208 consumed=0x06766307 data_ptr=0x7310adcb
trace reader return value=0x00000000
```

Second worker handler state:

```text
trace chunk-mode mode=0x00000000 orig=0xd19c306e consumed=0x000e0102 data_ptr=0x7325afd7
trace reader return value=0x00000000
```

Interpretation:

- The parent descriptor construction itself is not mysterious anymore: `0x2c95c` stores two qwords per worker descriptor.
- The third descriptor is bad because the earlier worker result chain is bad.
- The next target is the handler/reader path at `0x2848c`, not the third worker descriptor.

## 2026-07-06 Step 65 - Handler key strings and `fastLevel` lookup

Decoded native string calls with `ashield_decode_strings.py`:

```text
0x28530 func=0x2848c len=10 key=125 -> fastLevel
0x28b88 func=0x2848c len=4 key=46 -> rc4
```

Disassembled and traced the handler vtable function:

```text
vtbl[0x18] = 0x40900
```

The `0x40900` function is called with:

```text
x0 = 0x50000020
x1 = decoded string object for "fastLevel"
```

Observed trace:

```text
hook strlen src=0xfdb51 -> 0x9        # "fastLevel"
pc=0x40900 x0=0x50000020 x1=0x701ff388
...
hook strlen src=0xd3ff2 -> 0x0
trace handler vcall result local=0x701ff3a0 head=00 ... 48 00 00 50 ...
trace handler compare-call x0=0x701ff3a0 b'' ... x3=0xd3ff2 b''
trace handler compare-ret w0=0x00000000
trace handler match-ok=01 mode=0x00000000
```

`out_ctx` initialization writes:

```text
0x4977c off=0x00 <- 0xf57a0
0x49998 off=0x28 <- 0x50000050
0x49a7c off=0x30 <- 0
0x49a08 off=0x38 <- 0
0x497e4 off=0x50 <- 0
0x497ec off=0x50 <- 0
```

Interpretation:

- `0x2848c` asks the output/context object for `fastLevel`.
- The current emulation returns an empty/default value, causing `mode=0`.
- `mode=0` then selects the `rc4` path.
- However, the later reader failure suggests `mode=0` is either wrong for this material, or some object/config side effect is still missing before the lookup.

## 2026-07-06 Step 66 - RC4 hook validation and skip-RC4 test

Validated the Python fast PRGA hook against native execution:

```text
python reverse_apk/tools/ashield_vm_emulate.py
  --sync-pthreads
  --native-body-check
  --trace-reader
  --trace-handlers
  --rc4-native-prefix 64
  --max-insn 115000
```

Result:

```text
rc4 native-prefix compare data=0x51b97500 prefix=0x40
state_ok=True data_ok=True ij_ok=True
native_i=0x40 native_j=0x19 expected_i=0x40 expected_j=0x19
```

This proves the Python PRGA fast hook matches native behavior for the tested prefix.

Then tested:

```text
python reverse_apk/tools/ashield_vm_emulate.py
  --sync-pthreads
  --native-body-check
  --trace-reader
  --trace-threads
  --trace-worker-array-writes
  --skip-rc4
  --max-insn 180000
```

Result:

```text
trace reader enter buf=0x51b97500 len=0x6766307 head=05 87 00 00 72 20 f7 a7 ...
trace reader return value=0x00000000
...
heap 0x51b97500 contains Packet at +0x43f02f
heap 0x51b97500 contains EncryptionManager at +0x43efb6
```

Interpretation:

- The PRGA implementation is not the bug.
- The pre-PRGA buffer is meaningful protected DEX/code material and contains the target Packet/Encryption strings.
- Skipping PRGA preserves those strings but still does not satisfy the native reader.
- Therefore the missing piece is not simply "apply rc4" or "skip rc4"; it is the correct `0x2848c` handler mode / reader semantic for this worker material.

Updated next target:

- Trace the handler branch table after `mode` is set and identify all nonzero mode paths.
- Determine whether `fastLevel` should be provided by an earlier context side effect or whether `mode=0` is expected but the reader's input pointer/length is wrong.
- Avoid further forced-mode output as protocol evidence until the handler mode is recovered.

## 2026-07-06 Step 67 - DEX writer object behind `method@9d5e`

Rechecked the StopCapture writer path and the shared writer wrappers in `seg08.dex`.

Confirmed wrapper methods:

```text
0x47058c: field@44f5.method@9d56()
0x4705ac: field@44f5.method@9d57(arg1,arg2,arg3,arg4)
0x470618: field@44f5.method@9d5a(arg1)
0x470694: field@44f5.method@9d5e(arg1,arg2)
0x4706b4: field@44f5.method@9d5f(arg1)
0x4707cc: field@44f5.method@9d65(arg1,arg2)
```

This confirms the previously inferred parameter order for the StopCapture call:

```text
method@9d5e {builder, const(199), new(type@34f7)}
```

The concrete writer object source was located at `seg08.dex 0x46d414`:

```text
0000 new-instance type@34fa
000b invoke/range method@9d72 {v0, v1, v2, v3, v4, v5, v6, v7}
0017 iput field@44f5 obj=v9(field@43ea(v8)) src=v0(new(type@34fa))
0019 iput field@43ea obj=v8 src=v0(new(type@34fa))
```

Interpretation:

- The shared writer implementation stored in `field@44f5` is `type@34fa`.
- `method@9d72` initializes `type@34fa`.
- The reconstructed DEX metadata for this region is corrupt:
  - `type@34fa` resolves to a bad string index.
  - `field@44f5` resolves to impossible class/type/name indexes.
  - Direct class_def lookup did not find a clean class definition for `type@34fa`.
- Therefore the reliable evidence is behavioral code_item evidence, not field/type table metadata.

`0x46c838` appears to be a broad writer/adapter assembly method:

```text
many calls to method@9df4(field@43fa, string@...)
many calls to method@9c71(new(type@34e8), ...)
final call method@9caa(new(type@34ea), ...)
```

It is useful for understanding initialization, but it did not directly reveal the final StopCapture bytes.

Updated StopCapture status:

- StopCapture command value and writer call sequence remain high confidence.
- The exact bytes produced by `type@34fa.method@9d5e/9d56/9d65` are still not recovered.
- Next DEX-side target is to identify `type@34fa` method bodies by code behavior rather than metadata, or find the downstream boundary after `method@9d56` where the builder's byte array is finalized.

## 2026-07-06 Step 68 - `9d56` callers and Message string duplicate check

Searched all `seg08.dex` callers of `method@9d56`.

Relevant callers:

```text
0x468924  generic builder lifecycle:
  method@9cb8(...)
  method@9d69(...)
  method@9d53(...)
  method@9d52(...)
  method@9d6e(...)
  method@9d54(...)
  method@9d55(...)
  method@9d56(...)

0x47ed54  StopCapture template:
  method@9cb8(..., 4106, ...)
  method@9d55(...)
  method@9d57(..., 178, ...)
  method@9d5a(..., 89)
  method@9d5e(..., 199, empty type@34f7)
  method@9d5a(..., 87)
  method@9d5f(...)
  method@9d5a(..., 176)
  method@9d65(...)
  method@9d56(...)

0x47f5a0  StopCapture template duplicate:
  same command/writer pattern as 0x47ed54
```

Interpretation:

- `method@9cb8` creates the builder/envelope object.
- `method@9d56` is a finalize/end-write method in that builder lifecycle.
- The code still does not expose the final byte array directly; `9d56` appears to finalize internal writer state rather than return bytes at the call site.

Checked duplicate Packet/Message string material:

```text
seg12.dex:
  0x74a4cd Lcom/arashivision/onedriver/packet/Message;
  0x75f0bd Message(messageMethod
  0x75f0c5 messageMethod=

seg13.dex:
  0x0808de Lcom/arashivision/onedriver/packet/Message;
  0x09598e Message(messageMethod
  0x095996 messageMethod=
```

However, direct string_ids reverse lookup for `seg13.dex` returned no clean string index for these raw strings. The raw strings are present, but the metadata table cannot be trusted for code reference tracing.

Updated route:

- Continue using code behavior and known code_item offsets as primary evidence.
- Do not rely on raw Packet/Message strings alone to identify class methods unless a valid code_item reference is also found.

## 2026-07-06 Step 69 - Pretty Dalvik disassembler for damaged DEX code items

Added a local helper:

```text
F:\Insta360onWin\reverse_apk\tools\dex_pretty_disasm.py
```

Reason:

- Existing `dex_callgraph_slice.py --raw-disasm` intentionally prints many opcodes generically (`op_21`, `op_8d`, `binop/lit`, `array-op`).
- The UCD2/Packet builder needs exact array writes, shifts, and byte casts.
- The new helper decodes the opcodes needed for this area, including:
  - `array-length`
  - `aput-byte`
  - `int-to-byte`
  - `and-int/lit16`
  - `shr-int/lit8`
  - `add-int/lit8`
  - `add-int/2addr`
  - invoke register operands

This is an analysis-only tool; it does not modify APK data.

## 2026-07-06 Step 70 - Exact `ee8b` UCD2 outer-frame builder

Disassembled `seg12.dex 0x5651f4`, which is the implementation reached by `method@ee8b`.

Call evidence from `seg12.dex 0x565824`:

```text
016e invoke-static {v7, v0, v1, v5}, method@ee8b
```

`0x5651f4` has `regs=15 ins=4`, so its parameters are `v11..v14`.

Recovered behavior:

```text
arg0 = v11  message-type object; byte value from arg0.eeb3()
arg1 = v12  payload byte array
arg2 = v13  optional extra header/prefix byte array; nullable
arg3 = v14  sequence/session byte
```

Algorithm:

```text
extra_len = arg2 != null ? len(arg2) : 0
header_len = 12 + extra_len
payload_len = len(arg1)
tail_offset = header_len + payload_len
out_len = tail_offset + 4

out[0..3] = 55 43 44 32  ("UCD2")
out[4] = 01
out[5] = header_len
out[6] = arg0.eeb3()
out[7] = arg3
out[8..11] = little-endian payload_len

if arg2 != null:
  copy arg2 to out[12..]
  copy arg1 to out[12 + len(arg2)..]
else:
  copy arg1 to out[12..]

crc = ee8c(tail_offset, out)
out[tail_offset..tail_offset+3] = little-endian crc
return new frame object from out
```

This confirms the earlier observed UCD2 frame layout and checksum placement. It also confirms that byte 5 is dynamic `header_len`, not always `0x0c`.

Validated against observed device-info request shape:

```text
55 43 44 32 01 0c 04 10 0f 00 00 00 ...
```

For the no-extra case:

- `header_len = 12`
- `payload_len = 15`
- checksum tail starts at offset `12 + 15 = 27`

## 2026-07-06 Step 71 - Standard internal 9-byte Packet header in `ee91`

Re-disassembled `seg12.dex 0x565824`, the standard internal Packet + UCD2 builder (`method@ee91` target).

Important branch:

```text
payload_bytes = v46
payload_len = len(v46)
internal_len = payload_len + 9
internal = new byte[internal_len]

internal[0] = low byte of v40.f4c9()
internal[1] = high byte of v40.f4c9()
internal[2] = byte(v41.ee7b())
internal[3..6] = four derived time/session bytes from long arithmetic
internal[7] = 0
internal[8] = 0
copy payload_bytes to internal[9..]
```

Known device-info request payload already tested by the user:

```text
08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b
```

This matches the recovered layout:

```text
08 00              internal[0..1], low/high request or sequence id
02                 internal[2], message method/type byte
01 00 00 80        internal[3..6], derived time/session bytes for this command
00 00              internal[7..8]
08 30 08 0f 08 0b  actual higher-level message payload
```

The standard branch then calls `ee8b`:

```text
ee8b(message_type_object, internal_payload, extra_header_or_null, sequence_byte)
```

So the proven send pipeline is now:

```text
higher-level command/message payload
  -> 9-byte internal Packet header
  -> UCD2 outer frame via ee8b
  -> checksum tail via ee8c
```

Still missing:

- Exact higher-level command envelope bytes for StopCapture/StartCapture.
- `seg08` command layer strongly proves the command IDs and writer sequence, but the generic writer behind `method@9d5e/9d5f/9d65/9d56` is still not fully recovered.

## 2026-07-06 Step 72 - StopCapture message class and command-envelope status

Raw APK/DEX string evidence:

```text
seg06 raw strings:
  Linsta360/messages/StopCapture$Builder;
  Linsta360/messages/StopCapture$Companion$ADAPTER$1;
  Linsta360/messages/StopCapture$Companion;
  Linsta360/messages/StopCapture;
  Linsta360/messages/StopCaptureResp$Builder;

seg12 raw strings:
  Linsta360/messages/StopCapture;
  Linsta360/messages/StopCaptureResp;
```

The `StopCapture{` toString template appears with no visible field names:

```text
StopCaptureResp{
StopCapture{
StopHdrResp{
StopHdr{
```

Interpretation:

- `StopCapture` protobuf/Wire message itself is likely empty.
- `StopCaptureResp` has response fields; the request message likely does not.
- The command-layer envelope is therefore the important missing byte source, not the empty `StopCapture` message body.

Reconfirmed StopCapture command template at `seg08.dex 0x47ed54`:

```text
9cb8(..., 4106, string@6fc, string@9be, 0, 0)
9d55(builder)
9d57(builder, 178, field@478b, string@6fb, string@98e9)
9d5a(builder, 89)
new type@34f7
9d39(type@34f7)
9d5e(builder, 199, type@34f7)
9d5a(builder, 87)
a03f(...)
optional 9d58(...)
9d5f(builder, type@34f7)
9d5a(builder, 176)
9d65(builder, method@684(result(a03f), 2), 0)
9d56(builder)
```

`seg08.dex 0x47f5a0` is a duplicate StopCapture template with the same core sequence.

Current confidence:

- `PHONE_COMMAND_STOP_CAPTURE = 199` is proven.
- Empty/default `type@34f7` as the StopCapture body is high confidence.
- The exact serialized command envelope bytes produced by the `9d5x` writer are not yet proven.

Next target:

- Recover the generic writer behind `field@44f5` / `type@34fa`, or find a downstream finalized byte-array boundary after `9d56`.

## 2026-07-06 Step 73 - `type@34fa` / writer-controller method table recovered

New helper changes:

- Extended `tools/dex_pretty_disasm.py` with more Dalvik opcode names.
- Added `--start-unit` so large code items can be sliced by instruction unit.

Important result:

`method@9d72`, invoked by `seg08.dex 0x46d414` after `new type@34fa`, resolves with `--scan-data` to:

```text
class_data=0x7a7ce6 method@9d72 code=0x471344 direct access=0x10001
```

`0x471344` initializes the object created as `type@34fa` and stores:

```text
new type@34e8 -> field@44fe
arg2 -> field@452f
string-derived ids -> fields 44fd/4523/4505/452a
optional array-derived ids -> fields 4524/4506
mode/state -> field@44ff
new type@34f7 -> field@4508
then invokes method@9d92(this, type@34f7)
```

Full class-data scan around `0x7a7ce6` shows the usable method table for this object:

```text
method@9d71 code=0x470ff0
method@9d72 code=0x471344
method@9d73 code=0x47144c
method@9d76 code=0x47149c
method@9d77 code=0x471694
method@9d79 code=0x471814
method@9d7c code=0x47188c
method@9d7d code=0x4718c0
method@9d7e code=0x471b0c
method@9d9d code=0x473160
method@9d74 code=0x47097c
method@9d78 code=0x470a80
method@9d7f code=0x471b8c
method@9d88 code=0x472244
method@9d90 code=0x4728a4
method@9d91 code=0x472944
method@9d92 code=0x472b2c
method@9d99 code=0x472fbc
method@9da2 code=0x473304
method@9da3 code=0x47338c
```

The same class-data group contains known command-controller methods:

- `0x4728a4` is the previously identified plain `StartCapture` path (`PHONE_COMMAND_START_CAPTURE = 186`).
- `0x473304` is the previously identified `StartCaptureWithParam` path (`PHONE_COMMAND_START_CAPTURE_WITH_PARAM = 187`).
- `0x472944`, `0x473160`, `0x47338c` are generic command/body encoding helpers, not independent camera commands.

Wrapper methods around `0x47058c..0x4707cc` remain confirmed as delegates through `field@44f5`; `--scan-data` resolves:

```text
method@9d56 code=0x47058c
method@9d5e code=0x470694
```

Both wrappers call the same method id on `field@44f5`, so the call chain is:

```text
StopCapture template -> wrapper builder object -> field@44f5 backend -> type@34fa/controller-writer methods
```

`0x472b2c` (`method@9d92` in the recovered table) is a structural tree/linking helper:

- computes state from output buffer fields `44fe/43c4/43c5`;
- attaches `type@34f7` nodes through fields `44e3/44e7/4500/450e`;
- calls `9d73` to position or patch nodes.

`0x471814` (`method@9d79`) is a finalization/reset helper for mode/state `field@44ff`.

Status:

- The backend method table is now recovered well enough to avoid treating wrapper method ids as opaque.
- The StopCapture high-level template is still proven, but final serialized bytes are not yet produced.
- Next target is the `Message(messageMethod=...)` / Packet layer or a post-`9d56` finalized byte-array boundary.

## 2026-07-06 Step 74 - `type@34f7` empty body and wrapper/backend mapping

`type@34f7` is now better understood.

Narrow `class_data` scan:

```text
class_data=0x7a7b11 method@9d38 code=0x46ff0c direct access=0x10008
class_data=0x7a7b11 method@9d39 code=0x46ff2c direct access=0x10001
class_data=0x7a7b11 method@9d44 code=0x46fd20 virtual access=0x11
```

`method@9d39` / `0x46ff2c` is only:

```text
invoke-direct {this}, method@69b
return-void
```

So `new type@34f7; invoke-direct method@9d39` does not populate any request fields by itself.

`0x46ff0c` creates a singleton/default `type@34f7` and stores it in `field@44d0`.

This reinforces the earlier interpretation:

- `StopCapture` request body is an empty/default `type@34f7`.
- The missing bytes are generated by the surrounding command/envelope writer, not by fields inside `StopCapture`.

Wrapper/backend mapping evidence:

The wrapper class around `0x47058c..0x4707cc` delegates calls through `field@44f5`.

The backend/controller method table around `class_data=0x7a7ce6` contains the corresponding real implementations. The likely mapping for the StopCapture sequence is:

```text
wrapper 9d55()             -> backend 9d75 / 0x471474   small state accessor/check
wrapper 9d57(a,b,c,d)      -> backend 9d77 / 0x471694   multi-field/string metadata writer
wrapper 9d5a(int)          -> backend 9d8d / 0x47279c   compact single-int/tag writer
wrapper 9d5e(int, object)  -> backend 9d91 / 0x472944   command + object/body writer
wrapper 9d5f(object)       -> backend 9d92 / 0x472b2c   object/node attach or flush helper
wrapper 9d65(arg, int)     -> backend 9d98 candidate    tail/unknown-field writer candidate
wrapper 9d56()             -> backend 9d76 / 0x47149c   final flush/assemble helper
```

The mapping is inferred from wrapper method arity/order, `class_data=0x7a7ce6` method order, code shape, and direct references from the StopCapture template. It is not yet a final byte-level proof.

Partial symbolic path for `StopCapture`:

```text
9cb8(..., 4106, ...)
9d55()
9d57(..., 178, ...)
9d5a(89)
9d5e(199, empty type@34f7)
9d5a(87)
9d5f(empty type@34f7)
9d5a(176)
9d65(method@684(result(a03f), 2), 0)
9d56()
```

For the `9d5e(199, empty type@34f7)` call, backend `0x472944` starts with:

```text
if command < 200:
  compact_id = command - 33
else:
  compact_id = command
```

Therefore StopCapture command `199` enters this helper as compact id `166` (`0xa6`) before the object/body path is processed.

Still missing:

- exact byte sequence emitted by `9d57`, `9d5a`, `9d5e`, `9d5f`, `9d65`, and final `9d56`;
- direct bridge from this command envelope into the already recovered `seg12` Packet/UCD2 builder.

## 2026-07-06 Step 75 - StopCapture envelope byte skeleton

After improving opcode decoding, `0x47279c` is clear enough to prove the first byte emitted by `9d5a(int)`:

```text
0x47279c:
  iget-object v0, this, field@44fe
  ...
  invoke-virtual {v0, arg_int}, method@9c6d
```

`method@9c6d` is the byte-buffer single-byte writer already seen throughout this class.

Therefore in the StopCapture template:

```text
9d5a(89)   -> writes 0x59
9d5a(87)   -> writes 0x57
9d5a(176)  -> writes 0xb0
```

For `9d5e(199, empty type@34f7)`, backend `0x472944` follows this path when `type@34f7.field@44e1` is default/zero:

```text
command = 199
command < 200, so compact_id = command - 33 = 166 (0xa6)
field@44e1 of the empty object is cleared to 0
branch goes to the direct compact writer path
invoke-virtual {buffer, compact_id}, method@9c6d
invoke-virtual {empty_object, buffer, pos, 0}, method@9d43
```

So the command/body part of StopCapture includes:

```text
0x59 ... 0xa6 ... 0x57 ... 0xb0 ...
```

The proven byte skeleton for the main StopCapture template is now:

```text
9cb8(..., 4106, ...)      ; creates command envelope/context
9d55()
9d57(..., 178, ...)       ; metadata/header writer, bytes not yet resolved
9d5a(89)                  ; writes 59
9d5e(199, empty body)     ; writes a6 and patches/links the empty body node
9d5a(87)                  ; writes 57
9d5f(empty body)          ; object/node flush, bytes not yet resolved
9d5a(176)                 ; writes b0
9d65(..., 0)              ; tail/unknown-field writer, bytes not yet resolved
9d56()                    ; final flush/assemble
```

This is not yet the final packet, but it is the first byte-level command-envelope proof for `PHONE_COMMAND_STOP_CAPTURE = 199`.

Next target:

- resolve `9d57`, `9d5f`, `9d65`, and `9d56` enough to close the gaps around `59 a6 57 b0`;
- then wrap the resulting higher-level payload into the recovered 9-byte internal Packet header and UCD2 outer frame.

## 2026-07-06 Step 76 - `9d65` is state/length finalization, not direct byte emission

Disassembly of `0x472f70`:

```text
iget field@44ff
if mode == 4: invoke-direct 9d76()
if mode == 1: invoke-direct 9d77()
if mode == 2: field@4521 = field@4520
else:
  field@4521 = arg1
  field@451f = arg2
```

This means the StopCapture call:

```text
9d65(method@684(result(a03f), 2), 0)
```

does not directly write a command byte. It updates final length/state fields and may trigger the final flush helpers depending on `field@44ff`.

The important finalization helpers are therefore:

```text
0x47149c / method@9d76
0x471694 / method@9d77
```

`0x471694` walks/updates node metadata around fields `44e1`, `44ed`, `44ea`, `44e7`, `44e8`, `44eb`, and final `4521`. It appears to compute or normalize object-tree lengths rather than emit bytes directly.

Current byte-level StopCapture envelope proof remains:

```text
9d5a(89)                  -> 59
9d5e(199, empty type@34f7) -> a6 + object-position patch
9d5a(87)                  -> 57
9d5a(176)                 -> b0
```

Remaining hard gap:

- `9d57(...,178,...)` metadata/header contribution;
- exact bytes or patches emitted by object finalization `9d5f(empty)` and final flush `9d56()`.

## Step 77 - 鏍℃ StopCapture writer 鍒嗘敮涓庡伐鍏疯В鐮?

- 缁х画閬靛畧 APK-only锛氭湰姝ユ病鏈夊弬鑰冨叕寮€鍗忚璧勬枡锛屽彧鍩轰簬 `seg08.dex` 鏈湴鍙嶆眹缂栦笌璋冪敤鍥俱€?
- 涓婃娈嬬暀鐨?androguard 浠诲姟宸查€€鍑猴紱瀹冧粛鐒跺洜閲嶅缓 DEX 鐨?proto/string 鍏冩暟鎹紓甯歌€屼笉鍙敤锛屽悗缁户缁娇鐢ㄥ亸绉荤骇宸ュ叿銆?
- 淇 `tools/dex_pretty_disasm.py`锛氳ˉ鍏?`rem-int/2addr`銆乣and-int/2addr`銆乣or-int/2addr`銆乣int-to-short` 鐨勬牸寮忓寲锛岄伩鍏嶈璇?`0x472944` 鐨?StopCapture 鍒嗘敮銆傛枃浠跺凡淇濇寔 UTF-8 CRLF銆?
- 閲嶈绾犳锛歚0x472944` 閲屽懡浠ゅ弬鏁扮殑鏄犲皠鏄細褰?command `< 200` 鏃?`v3 = command`锛涘綋 command `>= 200` 鏃?`v3 = command - 33`銆傚洜姝?`PHONE_COMMAND_STOP_CAPTURE = 199` 鍦ㄨ灞傞鍏堜繚鎸?`0xc7`锛屼笉鏄箣鍓嶄复鏃舵帹鏂殑 `0xa6`銆?
- `0x47279c` 鍐嶆纭 `9d5a(int)` 鏄悜涓诲懡浠ょ紦鍐?`field@44fe` 鍐欏崟瀛楄妭鍙傛暟锛孲topCapture 妯℃澘涓殑涓変釜 marker 浠嶄负锛歚0x59`銆乣0x57`銆乣0xb0`銆?
- `0x472944` 鐨?StopCapture/瀵硅薄鍐欏叆鍒嗘敮浼氭牴鎹?body 瀵硅薄鐘舵€佸湪 `0xc7/0xc6 + length + ...` 绫荤紪鐮佷箣闂撮€夋嫨锛屽苟璋冪敤 body 鐨?`method@9d43` 鍥炲～闀垮害鎴栧啓瀵硅薄鍐呭銆?

## Step 78 - 瀹氫綅绌?StopCapture body 鐨勭湡瀹炵被鏂规硶

- `type@34f7` 绫绘暟鎹湪 `class_data=0x7a7b11`锛屽叧閿柟娉曪細
  - `method@9d39 code=0x46ff2c`锛氭瀯閫犲嚱鏁帮紝鍙皟鐢ㄥ熀绫绘瀯閫狅紝鏈～鍏呬笟鍔″瓧娈点€?
  - `method@9d43 code=0x470194`锛氬璞?body 鍐欏叆/闀垮害鍥炲～鍏ュ彛锛岃 `0x472944` 璋冪敤銆?
- `0x470194` 鍙傛暟褰㈡€佷负 body object + buffer + offset + flag锛涘绌?`type@34f7`锛屾湭鐪嬪埌浠讳綍涓氬姟瀛楁鍐欏叆锛屽彧鐪嬪埌鍩轰簬褰撳墠浣嶇疆鍜屼紶鍏?offset 鐨勯暱搴﹀洖濉矾寰勶細`9c6f/9c71`銆?
- 杩欐剰鍛崇潃 StopCapture 鐨?body 鏄┖/default object锛涘墿浣欐湭闂悎閮ㄥ垎涓嶆槸涓氬姟瀛楁锛岃€屾槸缂栫爜鍣ㄥ浣曞湪 `0xc7/0xc6` 鍒嗘敮閲岃〃杈锯€滅┖ body + 闀垮害鈥濄€?
- `0x471b8c` 宸茬‘璁や负瀹屾暣瀵硅薄搴忓垪鍖栧櫒锛氬畠鍐欏璞″ご銆佷富缂撳啿 `field@44fe`锛屽彲閫夋墿灞曠紦鍐?`field@452d`锛屼互鍙婂叾瀹冨彲閫夋銆係topCapture 鏈€灏忓璞″緢鍙兘鍙緷璧栦富缂撳啿鍜屾渶缁堥暱搴?鐘舵€佸瓧娈碉紝浣嗕粛闇€瑕佺户缁棴鍚堟渶缁堝瓧鑺傚簭鍒椼€?

## Step 79 - 搴曞眰 writer 鏂规硶涓庣搴忕‘璁?

- `type@34e8` buffer writer 鏂规硶琛ㄥ凡瀹氫綅锛?
  - `method@9c6d code=0x467ad0`锛氬啓 1 瀛楄妭锛宍int-to-byte` 鍚庤拷鍔犲埌 `field@43c4[field@43c5]`銆?
  - `method@9c71 code=0x467c60`锛氬啓 2 瀛楄妭锛屽ぇ绔簭锛岄珮 8 浣嶅湪鍓嶃€?
  - `method@9c6f code=0x467b50`锛氬啓 4 瀛楄妭锛屽ぇ绔簭锛宍>>>24/16/8/0`銆?
  - `method@9c6e code=0x467b0c`锛氬鍒?byte array 鍒板綋鍓?buffer銆?
- 杩欎簺 writer 鐩存帴鏀拺鍚庣画鏈湴妯℃嫙鍣紱涔嬪墠瑙傚療鍒扮殑 UCD2 澶栧眰闀垮害瀛楁浠嶆槸 little-endian锛屼絾杩欎釜鍐呴儴瀵硅薄缂栫爜灞傜敤 big-endian銆?
- `0x47e804` / `0x47f6dc` 璇佹槑 `198/199/167` 鏄鍐呴儴瀵硅薄缂栫爜灞傜殑鐗规畩 type/tag 鍊欓€夛紱`199` 涓嶅簲褰撹褰撲綔鏅€氬皬鏁存暟瀛楁瑁稿啓銆?

## Step 80 - StopCapture 澶栧眰搴忓垪鍖栬矾寰勭‘璁?

- StopCapture 妯℃澘浠嶆槸 `0x47ed54` / `0x47f5a0`锛屽叧閿簭鍒楋細
  `9d55 -> 9d57(178, ...) -> 9d5a(89) -> 9d5e(199, empty type@34f7) -> 9d5a(87) -> a03f/a0d1 -> 9d5f(empty) -> 9d5a(176) -> 9d65(...,0) -> 9d56`銆?
- 澶栧眰瀵硅薄 serializer `0x46c838` 鍦?`0x020b..0x0223` 鏄庣‘澶勭悊 `field@43e3 / field@44f5` 閾撅細
  - 鍏堣皟鐢?backend 鐨?`9d7b/9d7a` 姹囨€荤姸鎬併€?
  - 鍐嶈皟鐢?backend `9d7f(output)`锛屼篃灏辨槸 `0x471b8c`锛屾妸 StopCapture builder 鍐欏叆澶栧眰杈撳嚭 buffer銆?
  - 鐒跺悗娌?`field@44f5` 缁х画閾惧紡鍐欏叆銆?
- `method@9d44 code=0x46fd20` 瀵圭┖ `type@34f7`锛氳缃?`field@44e1 |= 4`銆乣field@44e0 = current_main_buffer_len`锛涜嫢 `field@44e2 == null` 鍒欒繑鍥?0銆備篃灏辨槸璇寸┖ StopCapture body 涓嶅啓涓氬姟瀛楁锛屽彧鍙備笌闀垮害/鐘舵€佸洖濉€?
- 褰撳墠杩樻湭闂悎鏈€缁?bytes 鐨勫師鍥狅細闇€瑕佺户缁‘瀹?`9d5e(199, empty)` 鍦ㄧ湡瀹炵姸鎬佷綅涓嬭蛋 `0xc7` 杩樻槸 `0xc6 + len + marker` 鍒嗘敮锛屼互鍙?`9d56` 鏈€缁?flush 鏃跺灞傚瓧娈靛ご鐨勫浐瀹?id銆備笅涓€姝ヤ紭鍏堝仛涓€涓渶灏忕姸鎬佹ā鎷熷櫒锛屾寜宸插畾浣嶇殑 writer/branch 鐢熸垚鍊欓€夊瓧鑺傦紝鍐嶅鍏?`seg12` 鐨?UCD2 Packet builder銆?

## Step 81 - UCD2 builder 鏍￠獙涓?StopCapture 闀垮害琛ヤ竵

- 鏂板/鏇存柊 `tools/luna_packet_candidate.py`锛屽彧浣跨敤 APK 鏈湴閫嗗悜璇佹嵁鐢熸垚鍊欓€夊寘銆?
- 澶栧眰 UCD2 builder 宸茬敤宸茬煡鍙敤鐨勮澶囦俊鎭姹傛牎楠岄€氳繃锛岃剼鏈緭鍑哄畬鍏ㄥ尮閰嶏細
  `55 43 44 32 01 0c 04 10 0f 00 00 00 08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b 7c 00 8e 7c`銆?
- `0x472944 / 9d5e(199, empty type@34f7)` 鍦ㄧ┖瀵硅薄鍒濆鐘舵€佷笅璧?`c7` 鍒嗘敮锛?
  - `9d5a(89)` 鍐?`59`銆?
  - `9d5e(199, empty)` 鍐?`c7`锛岄殢鍚?`type@34f7.9d43(..., flag=true)` 棰勭暀 4 瀛楄妭 `ff ff ff ff`銆?
  - `9d3b` 璁板綍琛ヤ竵淇℃伅锛歵ag 浣嶇疆鏄?`c7`锛屽疄闄?4 瀛楄妭闀垮害棰勭暀鍖轰粠 `c7` 鍚庝竴瀛楄妭寮€濮嬶紝绫诲瀷涓?`0x20000000`銆?
  - `9d5f(empty)` 瑙﹀彂 `9d44`锛屽 `0x20000000` 琛ヤ竵鍐欏叆 big-endian 4 瀛楄妭闀垮害锛歚current_len - reserved_start`銆?
- 濡傛灉 `57` 鍚庢病鏈夊叾瀹冪洿鎺ュ啓鍖呭瓧鑺傦紝鍒?StopCapture body 鍊欓€変负锛?
  `59 c7 00 00 00 05 57 b0`銆?
- 瀵瑰簲鍐呴儴 Packet 鍊欓€夛細
  `08 00 02 01 00 00 80 00 00 59 c7 00 00 00 05 57 b0`銆?
- 瀵瑰簲 UCD2 鍊欓€夛細
  `55 43 44 32 01 0c 04 10 11 00 00 00 08 00 02 01 00 00 80 00 00 59 c7 00 00 00 05 57 b0 7b a7 20 8a`銆?

## Step 82 - a03f/a0d1 鍒嗘敮闂悎鍒扮洿鎺ュ啓鍖呭瓧鑺?

- `a03f` 瀹氫箟宸插畾浣嶏細`class_data=0x7a96d9 method@a03f code=0x47ec28 direct`銆?
- `a03f` 鐨勭洿鎺ュ啓鍖呰涓猴細
  - 鍏堣皟鐢ㄦ帴鍙?`a0d1(...)`銆?
  - 鐒跺悗鎵ц `9d5a(89)`锛屾槑纭拷鍔犱竴涓?`59`銆?
  - 鐒跺悗鎵ц `9d57(179, field, string@6fb, string@98e9)`锛岃繖涓€姝ョ洰鍓嶄粛鎸夊厓鏁版嵁/瀛楁鏄犲皠澶勭悊锛屾湭璇佹槑浼氬悜涓?byte buffer 鍐欏叆鏅€?payload 瀛楄妭銆?
  - 杩斿洖鍊兼墽琛?`method@684(result, 2)`锛岀敤浜庡悗缁?`9d65(...,0)` 鐘舵€佹眹鎬汇€?
- `a0d1` 鐪熷疄瀹炵幇宸插畾浣嶏細`code=0x480534`銆傚畠鎵ц闆嗗悎/鐘舵€佸璞¤浆鎹細`a0d0 -> iterator/filter -> a0b5 -> c98 -> a0cf`锛屾病鏈夌洿鎺ヨ皟鐢?`9d5a/9d5e/9d5f` 杩欑被涓?writer銆?
- 鍥犳鍚?`a03f` 鐩存帴杩藉姞瀛楄妭鐨?StopCapture body 鍊欓€変负锛?
  `59 c7 00 00 00 06 57 59 b0`銆?
- 瀵瑰簲鍐呴儴 Packet 鍊欓€夛細
  `08 00 02 01 00 00 80 00 00 59 c7 00 00 00 06 57 59 b0`銆?
- 瀵瑰簲 UCD2 鍊欓€夛細
  `55 43 44 32 01 0c 04 10 12 00 00 00 08 00 02 01 00 00 80 00 00 59 c7 00 00 00 06 57 59 b0 de ef 4e 25`銆?
- 褰撳墠鏈渶缁堥棴鍚堢偣锛?
  - `9d57(178/179, ...)` 鏄惁鍙奖鍝?schema/field registry锛岃繕鏄篃閫氳繃鍚庣閾惧奖鍝嶆渶缁堣緭鍑哄瓧娈靛ご銆?
  - `9d58(...)` 鍦ㄩ儴鍒?Stop 妯℃澘涓槸鍚﹁拷鍔犲彲瑙佸瓧娈垫銆?
  - `9d56/9d65` 鏈€缁?flush 鏄惁鎶婁富 buffer 鍖呰繘鏇村灞傚瓧娈电粨鏋勶紱宸茬煡 `seg12` Packet 灞傚彲鍖呬换鎰?payload锛屼絾 StopCapture 浼犲叆 Packet 灞傚墠鐨勯珮灞?payload 浠嶉渶缁х画楠岃瘉銆?

澶嶇幇鍛戒护锛?

```powershell
python reverse_apk\tools\luna_packet_candidate.py
```

## Step 83 - 鍚庣 override 鏄犲皠涓庡綋鍓嶆渶寮?StopCapture 鍊欓€?

- `type@34fa` 鍚庣 class_data锛歚0x7a7ce6`銆?
- 鍏抽敭 override 璇佹嵁锛?
  - `method@9d8d code=0x47279c`锛氫笌妯℃澘 `9d5a(int)` 瀵瑰簲锛岀洿鎺ュ啓鍗曞瓧鑺傘€?
  - `method@9d91 code=0x472944`锛氫笌妯℃澘 `9d5e(int, object)` 瀵瑰簲锛屽鐞?`199/c7` 绌哄璞￠暱搴﹁ˉ涓併€?
  - `method@9d92 code=0x472b2c`锛氫笌妯℃澘 `9d5f(object)` 瀵瑰簲锛岃Е鍙?body `9d44` 鐘舵€?琛ヤ竵澶勭悊銆?
  - `method@9d98 code=0x472f70`锛氫笌妯℃澘鏈熬 `9d65(value,0)` 鐨勮涓轰竴鑷达紱褰?`field@44ff == 0` 鏃惰缃?`field@4521=value`銆乣field@451f=0`锛屼笉鐩存帴鍐欎富 byte buffer銆?
- `9d56` 鐨勫悗绔€欓€夋棤鍙傛柟娉曚腑鏈夌函 `return-void` 鏂规硶锛歚0x47221c`銆乣0x472230`銆傜粨鍚?`field@44ff == 0` 鐨勭畝鍗曟ā寮忥紝褰撳墠娌℃湁鐪嬪埌妯℃澘鏈熬 `9d56()` 浼氱户缁拷鍔犱富 payload 瀛楄妭銆?
- `9d57(178/179, ...)` 鏄犲皠鍒扮殑鍚庣閫昏緫涓昏绠＄悊 `field@4501/4527/452d/452e` 杩欑被瀛楁绱㈠紩鎴栨墿灞曟鐘舵€侊紱鐩墠娌℃湁鍙戠幇鍏跺儚 `9d5a/9d5e` 涓€鏍风洿鎺ュ啓鏅€氫富鍛戒护 byte銆?
- 鍥犳褰撳墠鏈€寮?StopCapture 楂樺眰 body 鍊欓€夋湁涓や釜锛?
  1. 鍩虹/鏃?`a03f` 鐩存帴 marker锛?
     `59 c7 00 00 00 05 57 b0`
  2. `0x47ed54` 鐨?`a03f` 鐩存帴杩藉姞 `59` 鍒嗘敮锛?
     `59 c7 00 00 00 06 57 59 b0`
- 褰撳墠鏈€寮哄畬鏁?UCD2 鍊欓€夛細
  1. 鍩虹锛?
     `55 43 44 32 01 0c 04 10 11 00 00 00 08 00 02 01 00 00 80 00 00 59 c7 00 00 00 05 57 b0 7b a7 20 8a`
  2. `a03f`锛?
     `55 43 44 32 01 0c 04 10 12 00 00 00 08 00 02 01 00 00 80 00 00 59 c7 00 00 00 06 57 59 b0 de ef 4e 25`
- 娉ㄦ剰锛氳繖涓や釜浠嶉渶璁惧瀹炴祴鏉ュ尯鍒嗭紝鍥犱负 `0x47ed54` 涓?`0x47f5a0` 閮芥槸 StopCapture 妯℃澘锛屼絾璧扮殑 helper 鍒嗘敮涓嶅悓銆?

## Step 84 - Windows app 鎺ュ叆 StopCapture 鍊欓€夊彂閫?

- 鍦?`src/adapters/luna_local.rs` 涓柊澧烇細
  - `STOP_CAPTURE_BASE_BODY = 59 c7 00 00 00 05 57 b0`
  - `STOP_CAPTURE_A03F_BODY = 59 c7 00 00 00 06 57 59 b0`
  - `build_internal_packet(command_id=0x0008, method_id=0x02, body)`
  - `Ucd2RawSession::send_stop_capture_candidate("base" | "a03f")`
- 鍦?`src/bin/html_app.rs` 涓柊澧?IPC 鍛戒护锛?
  - `ucd2_stop_candidate`
  - payload: `{ "host": "...", "variant": "base" | "a03f" }`
- 鍦?`web/index.html` 鐨勭浉鏈烘帶鍒跺尯鏂板涓や釜鎸夐挳锛?
  - `鍋滄鍊欓€?A` -> `variant="base"`
  - `鍋滄鍊欓€?B` -> `variant="a03f"`
- 鏂板鍗曞厓娴嬭瘯 `apk_stop_capture_candidates_match_static_reverse_outputs`锛屾牎楠屼袱涓€欓€?UCD2 frame 涓庨€嗗悜宸ュ叿杈撳嚭涓€鑷淬€?
- 宸茶繍琛岋細

```powershell
cargo fmt
cargo test
```

- 娴嬭瘯缁撴灉锛氶€氳繃銆備袱涓祴璇曠洰鏍囧悇 3 涓祴璇曞潎閫氳繃锛涗粎鏈夐」鐩棦鏈?dead_code 璀﹀憡銆?
- 涓嬩竴姝ュ疄鏈洪獙璇佽矾寰勶細
  1. 杩炴帴 LunaU Wi-Fi銆?
  2. 鎵撳紑搴旂敤锛岀偣 `璇诲彇璁惧淇℃伅`锛岀‘璁?UCD2 浼氳瘽姝ｅ父銆?
  3. 鑻ユ鍦ㄦ媿鎽?褰曞儚锛屽厛鐐?`鍋滄鍊欓€?A`銆?
  4. 瑙傚療鐩告満鐘舵€佷笌杩斿洖 JSON銆?
  5. 鑻?A 鏃犳晥锛屽啀鐐?`鍋滄鍊欓€?B`锛屾妸杩斿洖 JSON 缁х画璁板綍銆?

## Step 85 - 淇 StopCapture 鍊欓€夛細9d57 纭浼氬啓鍏ュ瓧娈靛ご

- 閲嶈绾犳锛歋tep 82-84 鐨?`59 c7...` 鍊欓€夌己灏?`9d57 -> 9c6b` 鍐欏叆鐨勫瓧娈靛ご瀛楄妭锛屼笉鑳藉啀浣滀负褰撳墠鍊欓€変娇鐢ㄣ€?
- 鏂拌瘉鎹摼锛?
  - `9d57(178, field, string@6fb, string@98e9)` 鏄犲皠鍒板悗绔?`0x472244`銆?
  - `0x472244` 浼氳皟鐢?`field@44fe.9c6b(tag, id)`銆?
  - `9c6b code=0x467a10` 鏄庣‘鍚戜富 buffer 鍐?3 瀛楄妭锛歚[tag_byte, id_hi, id_lo]`銆?
  - `9cb8(4106, string@6fc, string@9be, 0, 0)` 鍦?registry 涓厛鍒嗛厤 id 1銆?锛涢殢鍚庨娆?`9d57(178, field/string triple)` 鍒嗛厤 id 3锛屾墍浠ュ啓鍏?`b2 00 03`銆?
  - `a03f` 鍐呴儴鐨?`9d57(179, same triple)` 澶嶇敤 id 3锛屾墍浠ュ啓鍏?`b3 00 03`銆?
- 淇鍚庣殑鍩虹 StopCapture body锛?
  `b2 00 03 59 c7 00 00 00 05 57 b0`
- 淇鍚庣殑鍩虹鍐呴儴 Packet锛?
  `08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 05 57 b0`
- 淇鍚庣殑鍩虹 UCD2 frame锛?
  `55 43 44 32 01 0c 04 10 14 00 00 00 08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 05 57 b0 8b 78 6a 3d`
- 淇鍚庣殑 `a03f` StopCapture body锛?
  `b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0`
- 淇鍚庣殑 `a03f` 鍐呴儴 Packet锛?
  `08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0`
- 淇鍚庣殑 `a03f` UCD2 frame锛?
  `55 43 44 32 01 0c 04 10 18 00 00 00 08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 74 ef 99 27`
- 宸插悓姝ユ洿鏂帮細
  - `reverse_apk/tools/luna_packet_candidate.py`
  - `src/adapters/luna_local.rs`
  - `web/index.html`
- 浠嶆湭瀹屽叏闂悎锛?
  - `9d58(-1,0,empty,1,type)` 鏄惁浼氬湪鏌愪釜 StopCapture 鍒嗘敮杩藉姞鍙鎵╁睍娈点€?
  - `a03b` 鏋勯€犲弬鏁版潵鑷?`0x47fa9c` 鐨勭姸鎬佸垽鏂紝闇€瑕佺户缁‘璁ら粯璁?LunaU 褰曞儚鍋滄璺緞鏇村彲鑳借蛋鍩虹鍒嗘敮銆乣a03f` 鍒嗘敮杩樻槸 `9d58` 鍒嗘敮銆?

## Step 86 - `9d58` 鍒嗘敮鏁堟灉缂╁皬鍒颁簩绾ф墿灞?buffer

- `9d58` wrapper `code=0x4705cc` 鍙槸鎶婅皟鐢ㄨ浆鍙戠粰 `field@44f5.method@9d58(...)`銆?
- 鍚庣 `method@9d89 code=0x47232c` 鏄湡瀹炲疄鐜般€?
- 瀵?StopCapture 涓嚭鐜扮殑鍥哄畾鍙傛暟褰㈡€侊細
  - `9d58(-1, 0, empty_array, 1, type_array)`
  - 杩涘叆 `v12 == -1` 璺緞銆?
- 璇ヨ矾寰勪細纭繚 `field@452d` 浜岀骇 buffer 瀛樺湪锛岀劧鍚庤鍙栧綋鍓嶄富 buffer 闀垮害 `field@44fe.field@43c5`銆?
- 瀵?`v12 == -1` 涓?`v13 == 0` 鐨勮矾寰勶紝涓?buffer 涓嶅鍔犲瓧鑺傦紱瀹冨悜 `field@452d` 鍐欏叆绫讳技 `fb + current_main_len` 鐨勪簩绾ф锛屽苟鏇存柊 `field@4528/452e/4521/451f` 鐘舵€併€?
- 鍥犳锛?
  - `9d58` 涓嶆敼鍙?`9d5e(199, empty)` 鐨?`0x20000000` 闀垮害琛ヤ竵銆?
  - `0x47f5a0` 鐨勪富 buffer 浠嶆槸 `b2 00 03 59 c7 00 00 00 05 57 b0`銆?
  - `0x47ed54` 鑻?`field@478c` 涓?true锛屼富 buffer 浠嶆槸 `b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0`銆?
- 浣?`0x471b8c` 瀹屾暣 serializer 浼氳鍙?`field@452d`锛?
  - 鍏堝啓涓?buffer `field@44fe.field@43c4`銆?
  - 鍚庣画鑻?`field@452d != null`锛屼細鎶婁簩绾ф墿灞?buffer 缂栧叆瀹屾暣瀵硅薄杈撳嚭銆?
- 褰撳墠缁撹锛?
  - 鑻?UCD2 payload 浣跨敤 Stop 妯℃澘鐨勪富 buffer锛屽垯 Step 85 涓や釜鍊欓€変繚鎸佷笉鍙樸€?
  - 鑻?UCD2 payload 浣跨敤 `0x471b8c` 瀹屾暣瀵硅薄 serializer锛屽垯杩橀渶瑕佹妸 `field@452d` 浜岀骇娈垫寜瀹屾暣 serializer 甯冨眬鎷煎叆鏈€缁堝寘銆?
- 涓嬩竴姝ワ細缁х画杩?`0x46c838 -> 0x471b8c/9d7f` 鐨勫畬鏁村璞″簭鍒楀寲璋冪敤鑰咃紝纭 StopCapture 浜や粯缁?seg12 `Packet/UCD2` 鐨勭┒绔熸槸涓?buffer 杩樻槸瀹屾暣 serializer 杈撳嚭銆?

## Step 87 - 鐢熸垚瀹屾暣 builder 鑺傜偣鍊欓€?

- `0x471344` 鏋勯€犲櫒纭 `9cb8(4106, string@6fc, string@9be, 0, 0)` 鐨勫叧閿粯璁ょ姸鎬侊細
  - `field@44fd = 4106 = 0x100a`
  - registry: `string@6fc -> id 1`
  - registry: `string@9be -> id 2`
  - `field@4523 = 1`
  - `field@4505 = 2`
  - `field@44ff = 0`
- 棣栨 `9d57(178, ...)` 鍚?registry 涓?field/string triple 涓?id 3銆?
- `0x471b8c` 瀹屾暣 builder serializer 鐨勬渶灏?StopCapture 璺緞锛?
  - 鍐?`field@44fd/4523/4505`: `10 00 00 01 00 02`
  - 鍐欏瓧娈佃鏁帮細涓?buffer + `9c5c` 鐨勪袱涓浐瀹?0 闀垮害鍏冧俊鎭 = `00 03`
  - `string@224e` 鍒嗛厤涓?id 4锛岀敤鏉ユ壙杞戒富 buffer銆?
  - 涓?buffer record 鏍煎紡锛歚id(2) + record_len(4) + 4521(2) + 451f(2) + main_len(4) + main_bytes + extension_count(2) + optional extensions`
  - `4521` 褰撳墠鎸?`max(result, 2)` 鐨勬渶灏忚矾寰勫彇 `2`锛宍451f = 0`銆?
- `0x467744 / 9c5c` 瀵?`field@44fd=0x100a` 浼氳拷鍔犱袱涓浐瀹?0 闀垮害 metadata record锛?
  - `string@8b21` -> next id锛宍u32 length = 0`
  - `string@259e` -> next id锛宍u32 length = 0`
- 宸叉洿鏂?`reverse_apk/tools/luna_packet_candidate.py`锛屾柊澧炲畬鏁磋妭鐐瑰€欓€夎緭鍑恒€?
- 鏃?`9d58` 鐨勫畬鏁?base 鑺傜偣 body锛?
  `10 00 00 01 00 02 00 03 00 04 00 00 00 15 00 02 00 00 00 00 00 0b b2 00 03 59 c7 00 00 00 05 57 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00`
- 鏃?`9d58` 鐨勫畬鏁?base UCD2锛?
  `55 43 44 32 01 0c 04 10 38 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 15 00 02 00 00 00 00 00 0b b2 00 03 59 c7 00 00 00 05 57 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 87 0a 10 6c`
- 鏃?`9d58` 鐨勫畬鏁?a03f UCD2锛?
  `55 43 44 32 01 0c 04 10 3c 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 e8 78 8f 62`
- 甯?`9d58` 浜岀骇鎵╁睍鐨勫畬鏁?base UCD2锛?
  `55 43 44 32 01 0c 04 10 42 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 1f 00 02 00 00 00 00 00 0b b2 00 03 59 c7 00 00 00 05 57 b0 00 01 00 05 00 00 00 04 00 01 fb 0a 00 06 00 00 00 00 00 07 00 00 00 00 36 2a f7 ef`
- 甯?`9d58` 浜岀骇鎵╁睍鐨勫畬鏁?a03f UCD2锛?
  `55 43 44 32 01 0c 04 10 46 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 23 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 01 00 05 00 00 00 04 00 01 fb 0e 00 06 00 00 00 00 00 07 00 00 00 00 43 b3 04 30`
- 娉ㄦ剰锛氳繖浜涘畬鏁磋妭鐐瑰€欓€夋瘮 Step 85 鏇存帴杩?`0x46c838 -> 0x471b8c` 鐨勭湡瀹炲璞¤緭鍑猴紝浣?`4521=max(result,2)` 涓殑 `result` 浠嶉渶缁х画閫氳繃 `a0d1/a03f` 杩斿洖鍊奸棴鍚堛€傚綋鍓嶆寜鏈€灏忓€?`2` 鐢熸垚銆?

## Step 88 - App 鎺ュ叆瀹屾暣 StopCapture 鍊欓€?C-F

- 鍦?`src/adapters/luna_local.rs` 鏂板瀹屾暣鑺傜偣鍊欓€夊父閲忥細
  - `STOP_CAPTURE_FULL_BASE_BODY`
  - `STOP_CAPTURE_FULL_A03F_BODY`
  - `STOP_CAPTURE_FULL_BASE_9D58_BODY`
  - `STOP_CAPTURE_FULL_A03F_9D58_BODY`
- `Ucd2RawSession::send_stop_capture_candidate` 鏂板 variant锛?
  - `full_base`
  - `full_a03f`
  - `full_base_9d58`
  - `full_a03f_9d58`
- `web/index.html` 鐨勭浉鏈烘帶鍒跺尯鏂板鎸夐挳锛?
  - `瀹屾暣鍊欓€?C` -> `full_base`
  - `瀹屾暣鍊欓€?D` -> `full_a03f`
  - `鎵╁睍鍊欓€?E` -> `full_base_9d58`
  - `鎵╁睍鍊欓€?F` -> `full_a03f_9d58`
- 淇濈暀 A/B 涓?buffer 鍊欓€夛紝浠ヤ究瀹炴満鍖哄垎 UCD2 payload 浣跨敤涓?buffer 杩樻槸瀹屾暣 builder node銆?
## Step 89 - 瀹屾暣鍊欓€夋祴璇曢攣瀹?
- 鏂板鍗曞厓娴嬭瘯 `apk_stop_capture_full_node_candidates_match_static_reverse_outputs`銆?- 娴嬭瘯閿佸畾 C-F 鍥涗釜瀹屾暣鑺傜偣鍊欓€夌殑瀹屾暣 UCD2 frame锛?  - `full_base`
  - `full_a03f`
  - `full_base_9d58`
  - `full_a03f_9d58`
- 宸茶繍琛岋細

```powershell
cargo fmt
cargo test
```

- 缁撴灉锛氶€氳繃銆?  - `html_app` 鐩爣锛? 涓祴璇曢€氳繃銆?  - `luna_mic_rust` 鐩爣锛? 涓祴璇曢€氳繃銆?  - 浠嶅彧鏈夐」鐩棦鏈?dead_code 璀﹀憡銆?- 浠嶉渶缁х画闂悎锛?  - `a0d1/a03f` 杩斿洖鍊艰繘鍏?`method@684(..., 2)` 鍚庢槸鍚︿細璁╁畬鏁磋妭鐐逛腑鐨?`4521` 澶т簬褰撳墠鏈€灏忓€欓€夊€?`2`銆?  - 瀹炴満褰撳墠 StopCapture 璺緞鏈€缁堣蛋 A/B 涓?buffer锛岃繕鏄?C-F 瀹屾暣 builder node銆?

## Step 90 - Clean note: StopCapture entry order from APK

Encoding note: Step 85-89 text above is visually corrupted, but the byte values and file references there are still useful. This Step 90 supersedes the corrupted prose with clean UTF-8/ASCII notes.

Tooling update:
- `reverse_apk/tools/dex_pretty_disasm.py` now decodes more standard DEX opcodes used in the root path: `return-void`, `check-cast`, `invoke-super`, `invoke-super/range`, and integer arithmetic opcodes used by `0x46c838`.

New APK-only StopCapture facts:
- `class_data=0x7a96d9`
  - `method@a03c code=0x47ed08 virtual`
  - `method@a03d code=0x47ed28 direct`
  - `method@a03e code=0x47ed54 direct`
  - `method@a03f code=0x47ec28 direct`
- `0x47ed08 / method@a03c` is the StopCapture public/virtual entry:
  - first calls `a03d(v1)`
  - then calls `a03e(v1, v2)`
- `0x47ed28 / method@a03d` writes a root/action marker:
  - calls `v1.9cb6(4234, string@6fb, string@98e9, 0, 0)`
  - this creates a `type@34fa` node through the root action chain.
- `0x47ed54 / method@a03e` writes the StopCapture builder body:
  - calls `v1.9cb8(4106, string@6fc, string@9be, 0, 0)`
  - then writes `9d55`, `9d57(178, field@478b, string@6fb, string@98e9)`, `9d5a(89)`, `9d5e(199, empty type@34f7)`, `9d5a(87)`, `a03f(...)`, optional `9d58(...)`, `9d5f(empty)`, `9d5a(176)`, `9d65(max(result, 2), 0)`, `9d56`.

Complete StopCapture sequence from this class:
1. root/action marker node: `4234 / string@6fb / string@98e9`
2. builder node: `4106 / string@6fc / string@9be` plus the body bytes reconstructed in Steps 85-89

Parallel branch:
- `class_data=0x7a9842`
  - `method@a066 code=0x47f4d0 virtual`
  - `method@a068 code=0x47f574 direct`
  - `method@a069 code=0x47f5a0 direct`
- `0x47f4d0` calls `a068`, then `a069`, then `a067` if `field@47a6` is true.
- `0x47f574 / method@a068` writes the same root/action marker shape with command id `4121`:
  - `v1.9cb6(4121, string@6fb, string@98e9, 0, 0)`
- `0x47f5a0 / method@a069` writes a Stop-like builder body using `4106 / string@6fc / string@9be`, field@47a4, and mandatory `9d58` extension.

Root/action chain facts:
- `0x46d414` is the concrete action-chain add path for `9cb6`:
  - creates `type@34fa`
  - calls constructor `9d72(registry, command_id, arg2, arg3, arg4, arg5, root.field43dd)`
  - links it into `field@43e3` with `field@44f5` as next pointer
- `0x471344 / method@9d72` is also the constructor used by the builder node:
  - sets `field44fd = command_id`
  - registers/stores the two object/string args as `field4523/field4522` and `field4505/field4504`
  - when `field44ff == 0`, no automatic empty object is written
- `0x46c838 / method@9cd6` serializes the root:
  - writes root magic `0xcafebabe`
  - writes root header fields
  - writes registry via `field43fa.9e13(out)`
  - writes `field43e3` count, then each node via `9d7f(out)`
- `0x467744 / method@9c5c` writes metadata records controlled by node flags:
  - if `0x1000` is not masked/suppressed, may write `string@8b21` with zero length
  - if `field452a != 0`, writes `string@898d`
  - if `0x20000` is not set, writes `string@259e` with zero length

Current conclusion:
- The earlier C-F candidates are valid builder-node candidates, but not the complete APK entry sequence.
- The next candidate family should encode both action nodes in order:
  - marker node `4234/6fb/98e9`, then builder node `4106/6fc/9be` for `a03c`
  - marker node `4121/6fb/98e9`, then builder node `4106/6fc/9be` for `a066`
- Do not call this final yet until root registry version/initial registry state from `a05e` is closed or the combined candidate is tested against LunaU.

## Step 91 - App updated with APK entry sequence candidates G-I

Implemented files:
- `reverse_apk/tools/luna_packet_candidate.py`
  - added `apk_entry_marker_node(command_id)`
  - added `apk_stop_capture_sequence_candidate(marker_command_id, builder_body, extension=b"")`
  - now prints:
    - `apk_sequence_base_ucd2`
    - `apk_sequence_a03f_ucd2`
    - `apk_sequence_4121_ucd2`
- `src/adapters/luna_local.rs`
  - added `STOP_CAPTURE_APK_SEQUENCE_BASE_BODY`
  - added `STOP_CAPTURE_APK_SEQUENCE_A03F_BODY`
  - added `STOP_CAPTURE_APK_SEQUENCE_4121_BODY`
  - added variants:
    - `seq_base`
    - `seq_a03f`
    - `seq_4121`
- `web/index.html`
  - added app buttons:
    - `APK sequence G` -> `seq_base`
    - `APK sequence H` -> `seq_a03f`
    - `APK sequence I` -> `seq_4121`

Generated candidate frames:
- G / `seq_base`:
  `55 43 44 32 01 0c 04 10 48 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 15 00 02 00 00 00 00 00 0b b2 00 03 59 c7 00 00 00 05 57 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 1f 78 5a 59`
- H / `seq_a03f`:
  `55 43 44 32 01 0c 04 10 4c 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 02 5a ad 05`
- I / `seq_4121`:
  `55 43 44 32 01 0c 04 10 56 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 19 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 23 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 01 00 05 00 00 00 04 00 01 fb 0e 00 06 00 00 00 00 00 07 00 00 00 00 7f a5 7e 85`

Verification:
- `cargo fmt`
- `cargo test`
- Result:
  - `html_app`: 5 passed
  - `luna_mic_rust`: 5 passed
  - only existing dead_code warnings

Additional APK-only reverse fact:
- full scan resolved `method@a05e code=0x47f2a8`.
- `0x47f2a8 / a05e`:
  - calls `a060(input)`
  - if result is `70`, calls `a063(69, input)`
  - creates `type@34e9` via constructor `9c76(input)`
  - calls `a063(original_result, input)`
  - returns the `type@34e9` reader/context
- This means the root constructor in `480fcc` receives a `type@34e9` context derived from the command/input object, not a completely fresh context.

Important caution:
- G-I are closer to the APK entry order than A-F, but still marked candidates.
- The remaining closure item is the exact `type@34e9` context/registry state from `a05e -> 9c76`, especially how it affects registry ids and protocol version/masking in `9d7f`.

## Step 92 - Candidate narrowing after a05e and Stop virtual entries

APK-only facts added after Step 91:
- `method@a05e code=0x47f2a8` parses a byte array context:
  - `a060(input)` reads a big-endian u16 from `input[6..8]`.
  - if the value is `70`, `a05e` temporarily writes `69` back into `input[6..8]`.
  - it constructs `type@34e9` using `9c76(input)`.
  - it restores the original version value in `input[6..8]`.
- `method@9c76 code=0x469d1c` calls `9c77(input, 0, input.length)`.
- `method@9c78 code=0x469d38` is the real `type@34e9` parser constructor:
  - with the parse flag set, it reads version/header data from the byte array.
  - it allocates index arrays `field43d2`, `field43d1`.
  - it stores parse offsets/lengths into `field43d3/field43d4`.

Wrapper path:
- `0x47c980` is a complete existing-byte-array transform path:
  - validates input with `9fda(input)`
  - parses context with `a05e(input)`
  - creates root object via `9f93(owner, context, 0)`
  - selects command/action via `a07d(...)`
  - applies action using `context.9c79(action, 8)`
  - serializes root via `root.9cd6()`
- `0x47c964 / method@9f93` stores an owner field, then calls `9cc2(context, flags)`.
- `0x47dec4 / method@9fe7` is an action wrapper that calls `9caf(0x90000, root)`.

Stop virtual-entry narrowing:
- `0x47ed08 / method@a03c` calls:
  1. `a03d(v1)` -> marker `9cb6(4234, string@6fb, string@98e9, 0, 0)`
  2. `a03e(v1, v2)` -> builder `9cb8(4106, string@6fc, string@9be, 0, 0)` and body
- `a03e` unconditionally calls `a03f`.
- Therefore among G/H, `seq_a03f` is the closer candidate for this StopCapture class.
- `seq_base` remains useful only as a diagnostic candidate.

Parallel branch narrowing:
- `0x47f4d0 / method@a066` calls:
  1. `a068(v1)` -> marker `9cb6(4121, string@6fb, string@98e9, 0, 0)`
  2. `a069(v1, v2)` -> Stop-like builder with mandatory `9d58`
  3. `a067(v1, v2)` only if `field@47a6` becomes true
- `0x47f3e4 / method@a06a` has an alternate branch that can write `9d57(179, ...)` plus `9d70(58, arg)`, or `9d67(184, field@47a4, string@6fc, string@9be, true)` plus `9d70(58, arg)`.

Current best test order:
1. H / `seq_a03f` for `a03c` StopCapture.
2. I / `seq_4121` for the parallel Stop-like branch.
3. C-F only if H/I return no meaningful response.

Remaining blocker for "final final":
- The exact starting byte array passed into `a05e(input)` is still not located in APK-only static analysis.
- Without that template, root registry ids/version masking can still be off in G-I.
- H is the best current APK-derived candidate, not yet a device-proven final packet.

## Step 93 - Shared registry correction and wrapper marker

APK-only facts resolved:
- `method@9cc2 code=0x46d4f8` initializes the root with `9cae(0x90000)`, stores flags in `field43e5`, creates a registry from the parsed context when context is non-null, then calls `9cd5(flags)`.
- For the Stop path, `480fcc` passes `flags=0`, and `method@9cd5 code=0x46d540` sets `field43dd=0`.
- `method@a0e0 code=0x480f88` wraps the root/action pair.
- `method@a0e1 code=0x480fa4` calls `a0e3(root, field47df)` and then `super.9cb5()`.
- `method@a0e4 code=0x481138` writes an extra wrapper marker before the real Stop action:
  - `9cb6(4233, external_arg, string@3865, 0, 0)`
- `method@9cb5 code=0x46c64c` then walks the next action in the chain.

Important correction:
- The previous G/H/I APK sequence candidates incorrectly treated each node as if it had a fresh registry.
- APK serializer uses one registry for the full root, so ids must be shared across marker and builder nodes.
- After marker `4234`, the builder node ids shift:
  - `6fc -> 4`
  - `9be -> 5`
  - first StopCapture triple -> `6`
  - `224e -> 7`
  - `8b21 -> 8`
  - `259e` remains `3`

Updated UCD2 frames:
- G / shared-registry base:
  `55 43 44 32 01 0c 04 10 48 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 15 00 02 00 00 00 00 00 0b b2 00 06 59 c7 00 00 00 05 57 b0 00 00 00 08 00 00 00 00 00 03 00 00 00 00 fd 21 04 26`
- H / shared-registry a03f:
  `55 43 44 32 01 0c 04 10 4c 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 06 59 c7 00 00 00 09 57 59 b3 00 06 b0 00 00 00 08 00 00 00 00 00 03 00 00 00 00 21 c5 87 ae`
- I / shared-registry 4121 branch:
  `55 43 44 32 01 0c 04 10 56 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 19 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 23 00 02 00 00 00 00 00 0f b2 00 06 59 c7 00 00 00 09 57 59 b3 00 06 b0 00 01 00 08 00 00 00 04 00 01 fb 0e 00 09 00 00 00 00 00 0a 00 00 00 00 ca 0c 32 2e`
- J / wrapper + a03c marker + a03f builder, assuming wrapper external arg occupies registry id 1:
  `55 43 44 32 01 0c 04 10 5a 00 00 00 08 00 02 01 00 00 80 00 00 00 03 10 89 00 01 00 02 00 01 00 03 00 00 00 00 10 8a 00 04 00 05 00 01 00 03 00 00 00 00 10 00 00 06 00 07 00 03 00 09 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 08 59 c7 00 00 00 09 57 59 b3 00 08 b0 00 00 00 0a 00 00 00 00 00 03 00 00 00 00 49 57 f5 6d`

Implemented:
- `reverse_apk/tools/luna_packet_candidate.py`
  - `stop_capture_inner_candidate(..., triple_id=...)`
  - parameterized `stop_capture_full_node_candidate(...)`
  - parameterized `apk_entry_marker_node(...)`
  - corrected G/H/I to use shared registry ids
  - added `apk_wrapped_stop_capture_sequence_candidate(...)`
- `src/adapters/luna_local.rs`
  - replaced G/H/I constants with shared-registry versions
  - added `STOP_CAPTURE_APK_WRAPPED_SEQUENCE_A03F_BODY`
  - added variant `seq_wrapped_a03f`
- `web/index.html`
  - added button `APK sequence J`

Verification:
- `cargo fmt`
- `cargo test`
- Result:
  - `html_app`: 5 passed
  - `luna_mic_rust`: 5 passed
  - only existing dead_code warnings

Current best test order:
1. J / `seq_wrapped_a03f`
2. H / `seq_a03f`
3. I / `seq_4121`

Caveat:
- J is the closest APK-only static reconstruction so far, but `a0e4` uses an external argument from `480fcc` as wrapper marker arg2.
- If that external argument is dynamic and not equivalent to registry id 1 in the live official flow, J still needs one more correction from runtime/device evidence.

## Step 94 - Static boundary for a0e8 external argument

Additional APK-only tracing:
- `method@a0e8 code=0x480fcc` belongs to:
  - `class_data=0x7a9c9b`
  - access `0x9` (`public static`)
- No Java bytecode caller was found by `dex_code_scan --target-method 0xa0e8`.
- Sibling methods in the same class:
  - `a0e2 code=0x4810e4`
  - `a0e3 code=0x481120`
  - `a0e4 code=0x481138`
  - `a0e5 code=0x481044`
  - `a0e6 code=0x481064`
  - `a0e7 code=0x481014`
  - `a0e8 code=0x480fcc`
- `a0e5` calls `a0e6(input, arg, string@6fa)`.
- `a0e6` creates `type@356d` and returns it after parsing an object/stream.
- `a0e7` writes:
  - `9d57(178, field47e3, field47e1, string@3865)`
  - then calls `a0f5(...)`
- `a0f5 code=0x4813e8` is a different builder path:
  - calls `a0f6(...)`
  - writes `9d5a(90)`
  - writes `9d67(182, string@b9f0, string@a892, string@a0e, false)`
  - writes `9d5a(87), 9d5a(3), 9d5a(50), 9d6f(192, string@98e9)`

Native/resource search:
- Readable searches for `a0e8`, `StopCapture`, `UCD2`, and related names did not reveal a stable Java/JNI registration name for `a0e8`.
- Native binary raw bytes contain many incidental `e8 a0` hits, but no actionable symbol/name binding for this method.

Conclusion:
- APK-only static analysis has produced the closest current StopCapture packet candidate J.
- The final uncertainty is not CRC/header/encryption anymore; it is the runtime value passed as `a0e8(byte[] input, Object externalArg)`.
- Because that `externalArg` is not present as a static APK constant or Java caller argument, a device/runtime response is required to prove whether J is final or to adjust the wrapper marker's first registry entry.

Current best packet to test:
- J / `seq_wrapped_a03f`:
  `55 43 44 32 01 0c 04 10 5a 00 00 00 08 00 02 01 00 00 80 00 00 00 03 10 89 00 01 00 02 00 01 00 03 00 00 00 00 10 8a 00 04 00 05 00 01 00 03 00 00 00 00 10 00 00 06 00 07 00 03 00 09 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 08 59 c7 00 00 00 09 57 59 b3 00 08 b0 00 00 00 0a 00 00 00 00 00 03 00 00 00 00 49 57 f5 6d`

## Step 95 - Re-opened root serializer boundary

Important correction:
- Candidate J is no longer treated as the final packet.
- J currently represents the reconstructed action-chain body for wrapper + StopCapture, but `method@a0e8 code=0x480fcc` returns `root.9cd6()`, not just the raw action list.
- The final APK-style packet therefore likely needs the full `root.9cd6()` byte array, including:
  - root magic/header (`0xcafebabe`)
  - root version/flags
  - registry/string table via `9e13`
  - field node list (`field43e2`, serialized by `9d08`)
  - action node list (`field43e3`, serialized by `9d7f`)
  - possible final transform through `9cd4(...)`

APK-only facts about the root serializer:
- `method@9cd6 code=0x46c838`:
  - computes output size first.
  - creates a `type@34e8` writer.
  - writes `0xcafebabe`.
  - writes `field43fc`.
  - writes registry data through `field43fa.9e13(out)`.
  - writes version mask and root counters.
  - serializes each `field43e2` node through `9d08`.
  - serializes each `field43e3` action node through `9d7f`.
  - if the action flag accumulator is non-zero, returns `9cd4(bytes, v5)` instead of the raw writer buffer.

New likely input-template path:
- `method@a0da code=0x480dd0` creates a fresh root/template byte array and returns `9cd6()`.
- `a0da` writes:
  - `9cd7(53, 4097, v3, 0, string@b9f0, null)`
  - `9cdb(9, string@a51d, string@3865, 0, 0)`
  - `9cda()`
- `method@a0dd code=0x480ee0` calls `a0da(...)`, so `a0da` may produce the `input` byte array that later gets parsed by `a0e8` through `a05e(input)`.

Next reverse targets:
- `9cd4` to understand the final transform used when action nodes set flags.
- `9d7f/9d7b/9d7a` to understand action-node serialization and flag accumulation.
- `9e13/9e12` to reconstruct the registry/string-table bytes instead of guessing local ids only.
- `a0da/a0dd` call chain to determine whether the StopCapture command starts from the template root produced by `a0da`.

## Step 96 - Packet/EncryptionManager line moved to seg12

Important correction:
- `Packet`, `EncryptionManager`, and `UCD2-XOR-KEY-001` were located in `reverse_apk/reconstructed_dex/seg12.dex`.
- This is a better path than continuing to guess only from seg08 action builders.

Raw locations in seg12:
- `UCD2-XOR-KEY-001` raw string data around file offset `0x56399c`.
- `Lcom/arashivision/onedriver/packet/Packet;` raw descriptor around `0x6c80d`.
- `Lcom/arashivision/onedriver/encrypt/EncryptionManager;` raw descriptor around `0x749dc0`.
- `Linsta360/messages/StopCapture;` raw descriptor around `0x708f0`.

Caveat:
- seg12 contains raw readable descriptors/strings, but the reconstructed DEX string table does not point to those raw string_data offsets.
- Standard `string_ids` parsing and `dex_inspect --class-contains` do not resolve these classes.
- The code itself is still disassemblable by known code offsets.

Packet builder at `seg12 code_off=0x565824`:
- This method constructs the internal packet and then wraps it in a UCD2 frame.
- Known APK-derived device-info packet came from this path:
  - `internal[0..1] = f4c9()` little-endian command id.
  - `internal[2] = ee7b()` method/message byte.
  - `internal[3..6] = 01 00 00 80` for the observed working device-info command.
  - `internal[7..8] = 00 00`.
  - then method payload bytes.
- The 12-byte UCD2 branch explicitly writes:
  - byte 0 `0x55`
  - byte 1 `0x43`
  - byte 2 `0x44`
  - byte 3 `0x32`
  - byte 4 `0x01`
  - byte 5 dynamic header length
  - byte 6 message type
  - byte 7 encryption/packet mode byte
  - byte 8..11 little-endian payload length
- A branch calls `field@7094.f45e(payload, header, timestamp)` and receives a result object with fields:
  - `field@7090`
  - `field@7091`
  - `field@7092`
  These are then copied into the returned packet, so this is the `EncryptionManager`/encryption result integration point.
- The long-packet branch writes:
  - little-endian total length at bytes 0..3
  - encryption/packet type at byte 4
  - padding/alignment byte at byte 5
  - command id bytes at 7..8
  - method byte at 9
  - dynamic timestamp-derived bytes at 10..13
  - zeros at 14..15
  - then copies the original payload at offset computed from header size.

Device-side check:
- `Test-NetConnection 192.168.42.1:6666` returned `True`.
- A one-off raw Python socket sending auth + device-info received no bytes in that new socket.
- Interpretation: do not rely on that quick one-off test yet; the Luna may require the existing app persistent session state or may reject concurrent/new sockets.

Current reverse target:
- Continue inside seg12 around `0x565824` and the `field@7094.f45e(...)` call chain.
- Find methods/fields for the encryption result object (`field@7090/7091/7092`) and the enum/mode fields (`field@707d`, `field@709a`, `field@7521`).
- Once the packet wrapper is fully understood, plug the seg08 StopCapture body/root output into the real seg12 Packet builder instead of sending action body J directly.

## Step 97 - Packet builder chain confirmed

Method/code mapping recovered from a narrow class_data scan:
- `class_data=0x939e36 method@edc4 code=0x55e378 virtual access=0x11`
- `class_data=0x93a5fe method@ee60 code=0x5646b4 direct access=0x9`
- `class_data=0x93a7fb method@ee8b code=0x5651f4 direct access=0x9`
- `class_data=0x93a7fb method@ee8c code=0x565d78 direct access=0x9`
- `class_data=0x93a7fb method@ee90 code=0x5656c0 direct access=0x9`
- `class_data=0x93a7fb method@ee91 code=0x565824 direct access=0x9`
- `class_data=0x93a8ef method@eea7 code=0x5662d8 direct access=0x10001`
- `class_data=0x93a919 method@eeab code=0x5663d8 direct access=0x10001`

Confirmed meanings:
- `ee8c code=0x565d78` is the UCD2 CRC/checksum:
  - init `-1`
  - for each byte: `crc ^= byte`
  - 4 rounds of `(crc << 8) ^ table[(crc >>> 24) & 0xff]`
  - returns a 32-bit checksum written little-endian by the caller.
- `ee8b code=0x5651f4` is the plain 12-byte UCD2 frame builder:
  - bytes 0..3: `55 43 44 32`
  - byte 4: `01`
  - byte 5: `12 + extra_len`
  - byte 6: message type enum byte (`eeb3()`)
  - byte 7: caller-provided sequence/mode byte
  - bytes 8..11: payload length, little-endian
  - optional extra bytes at offset 12
  - payload after header
  - final 4 bytes: `ee8c(frame_without_crc)` little-endian
- `eea7 code=0x5662d8` constructs the encryption result object (`type@2d33`):
  - `field@7090`: encrypted/body bytes
  - `field@7091`: extra/header bytes copied into the outgoing UCD2 header
  - `field@7092`: second extra bytes copied into the outgoing UCD2 header
- `eeab code=0x5663d8` constructs a Packet configuration object (`type@2d34`):
  - `field@7093`: packet type / encryption mode enum
  - `field@7094`: encryption manager object
- `ee91 code=0x565824` is the high-level Packet builder:
  - builds the internal Luna packet from command id (`f4c9()`), method byte (`ee7b()`), payload and timestamp-derived bytes.
  - if encryption config is present, calls `field@7094.f45e(payload, header, timestamp)` and expects `type@2d33`.
  - then uses `field@7091/7092/7090` to construct the final outgoing UCD2 frame.
- `edc4 code=0x55e378` is the upstream send/packetization method:
  - If the payload is short enough, calls `ee60`.
  - If the payload needs the encrypted/high-level path, calls `ee91`.
  - If the payload is too large, chunks it and calls `ee91` repeatedly.

Practical result:
- The final control packet path is no longer guessed as `action_body -> UCD2`.
- Correct path is:
  1. build APK message/root payload (seg08 StopCapture path),
  2. pass it to seg12 `edc4/ee91`,
  3. let `field@7094.f45e` produce encryption result when configured,
  4. wrap with `ee8b`-style UCD2 header and `ee8c` CRC.

Remaining target:
- Resolve the concrete implementation behind `field@7094.f45e(...)`.
- `f45e` did not appear as a normal concrete method in the immediately parsed method definitions, so it is likely an interface/virtual method implemented elsewhere or represented under a different reconstructed method id.

## Step 98 - f45e is not concrete in seg12; avoid cross-dex method-id noise

Important correction:
- Method ids are local to each dex segment. A broad scan for `method@f45e` or `method@eeab` across all `seg*.dex` produces many unrelated hits.
- Only `seg12.dex` method ids should be used for the Packet/UCD2 chain discovered in Step 97.

Confirmed seg12-only results:
- `method@f45e` is invoked only from `ee91 code=0x565824`.
- `dex_safe_method_defs seg12 --method 0xf45e` returned `hits=0`.
- Therefore `f45e` has no concrete Java/Kotlin bytecode body in this reconstructed seg12 class_data. Treat it as an interface/abstract/external implementation call.

The exact `ee91` encrypted branch around the call is:
- Builds internal payload `v3`.
- Builds a UCD2-like header seed `v11`.
- Reads `v2 = config.field@7094`.
- Converts timestamp with `method@f60f`.
- Calls `invoke-virtual {v2, v3, v11, timestampObj}, method@f45e`.
- Casts the result to `type@2d33`.
- Copies result fields:
  - `field@7091` into outgoing header position 3 for 12 bytes.
  - `field@7092` into outgoing header position 15 for 12 bytes.
  - `field@7090` as the final encrypted/body payload.

Other seg12-only findings:
- `method@eeab code=0x5663d8` constructs a `type@2d34` config-like object and stores `field@7093/field@7094`, but no direct seg12 caller was found.
- `method@edc4` has one seg12 caller at `code=0x55e34c`; it calls `edc1`, casts to `type@2cea`, loads static `field@7a4c`, then invokes `edc4`.
- `field@7a4c` appears in many command/message helpers, so it is probably a generated message/default/static command object field, not the encryption object itself.

Raw descriptor/string evidence:
- `seg12.dex` contains raw descriptors for:
  - `Lcom/arashivision/onedriver/encrypt/EncryptResult;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptionManager$Companion;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptionManager$WhenMappings;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptionManager;`
  - `Lcom/arashivision/onedriver/encrypt/PlatformCrypto;`
- `UCD2-XOR-KEY-001` exists raw at `seg12.dex` offset `0x56399c`.

Next target:
- Use raw descriptor windows and native/library scans to locate the external implementation backing `EncryptionManager.f45e(...)`.
- Do not trust normal seg12 method/string/type tables for this area; they are corrupted by packing/reconstruction.

## Step 99 - UCD2 negotiation arrays and state machine recovered

New static-init evidence in seg12:
- `code=0x563960` creates a 16-byte array and fills it with inline data:
  - ASCII: `UCD2-XOR-KEY-001`
  - hex: `55 43 44 32 2d 58 4f 52 2d 4b 45 59 2d 30 30 31`
  - stored in `field@704b`
- `code=0x563fe8` creates the handshake/static arrays:
  - `field@7059`, len 10:
    - ASCII: `syNceNdinS`
    - hex: `73 79 4e 63 65 4e 64 69 6e 53`
  - `field@705a`, len 8:
    - `08 00 00 00 b0 00 00 01`
  - `field@705b`, len 7:
    - `07 00 00 00 05 00 00`
  - `field@705c`, len 4:
    - ASCII/hex: `UCD2` / `55 43 44 32`

Negotiation/state methods:
- `code=0x563ae8` parses inbound negotiation bytes:
  - checks incoming buffer starts with `field@705c` (`UCD2`).
  - if not, and if not already in a special state, checks 8-byte input against `field@705a`.
  - also scans buffered bytes for `field@7059` (`syNceNdinS`).
  - sets connection/handshake flags (`field@704f`, `field@7087`) and calls callbacks.
- `code=0x563e48` is an active negotiation/send method:
  - if the handshake is not established, it transforms `field@7059` through `method@ee92` then `method@f975`.
  - sends the result through `method@ee50`.

Interpretation:
- The Luna connection is stateful. Sending only the later command packet on a fresh/cleared socket can disconnect because the APK state machine first negotiates with the `syNceNdinS`/`UCD2` handshake arrays.
- The previously observed constant auth frame (`05 0f`) is not the only session setup material; there is an earlier/lower negotiation layer around these arrays.

Current high-value method:
- `code=0x55caec` is a large UCD2 receive/build/parser method.
- Its early branch constructs UCD2-like bytes, uses constants `55 43 44 32`, writes packet fields, copies payload, and appends checksum through `ee8c`.
- Continue splitting `0x55caec` into branches to recover the remaining long/encrypted packet behavior.

## Step 100 - Added APK-derived negotiation probe to the app

Code update:
- Added `UCD2_NEGOTIATION_SYNC = b"syNceNdinS"` to `src/adapters/luna_local.rs`.
- Added `Ucd2RawSession::send_negotiation_sync()`.
- Added app command `ucd2_negotiation_sync` in `src/bin/html_app.rs`.
- Added a Chinese UI button `发送协商前导` in `web/index.html`.
- Added the recovered arrays to `reverse_apk/tools/luna_packet_candidate.py` output:
  - `ucd2_xor_key`
  - `ucd2_negotiation_sync`
  - `ucd2_negotiation_ack_8`
  - `ucd2_negotiation_ack_7`

Verification:
- `cargo fmt` passed.
- `cargo test` passed: 5 tests in `html_app`, 5 tests in `luna_mic_rust`.
- Existing warnings are dead-code warnings from unused/probing paths.

Important boundary:
- This new button sends the APK-recovered lower negotiation sync buffer only:
  - `73 79 4e 63 65 4e 64 69 6e 53`
- It is not a normal UCD2 frame and not the final camera-control packet.
- It is intended to validate whether LunaU requires the `0x563e48` negotiation path before later `04 10`/StopCapture packets.

Next target:
- Continue splitting `0x55caec` receive/build branches and search for the external implementation behind `EncryptionManager.f45e(...)`.
- The current StopCapture candidate J remains marked as a wrapper/action-chain candidate, not final.

## Step 101 - Packet mode enum and send dispatcher evidence

New mode enum mapping:
- `code=0x55e288` initializes `field@6f92` as a Kotlin-style enum WhenMappings table.
- The mapping is:
  - `field@70a7 -> 1`
  - `field@70a3 -> 2`
  - `field@70a5 -> 3`
  - `field@70a6 -> 4`

Send dispatcher evidence:
- `code=0x55e34c` is a tiny bridge:
  - calls `method@edc1`
  - casts the result to `type@2cea`
  - loads static `field@7a4c`
  - calls `type@2cea.method@edc4(...)`
- `code=0x55e378` (`edc4`) is the important final send dispatcher.

`edc4` branch behavior:
- It reads `this.field@6f96.field@6faf` into `v9`.
- If `field@6faf == field@70a5`, it uses `method@ee60` and then `method@f975`.
  - This branch chunks payload at `0xec` / 236 bytes.
  - It sends through `this.field@6f96.field@6f9c.method@ee35(...)`.
  - This is the short/plain packet path, not the high-level UCD2 encrypted packet path.
- Otherwise it uses `method@ee91`.
  - It computes max payload with `ee8e(mode) - ee8f() - 9`.
  - For oversized payloads it chunks and repeatedly calls `ee91(...)`.
  - For normal payload it calls `ee91(...)`, then `f975(...)`.
  - It dispatches the resulting bytes according to the enum mapping:
    - mapping `1` (`70a7`) falls through to the error/default handling.
    - mapping `2` (`70a3`) sends iterated objects to `field@6f9c`.
    - mapping `3` (`70a5`) sends via `field@6f99.method@eed2(...)`.
    - mapping `4` (`70a6`) sends raw bytes via `field@6f9d.method@ee50(...)`.

Upstream command builder evidence:
- `code=0x55e708` builds `type@2d24` command metadata using `ee72`/`ee73`.
- It checks `this.field@6faf` against `field@70a3` and `field@70a5` for extra capability/logging validation.
- It constructs `type@2cea` via `method@edc0(this, commandMeta, flag, null)` and submits it via `method@fa3b`.

Interpretation:
- The old StopCapture candidate is still only the action-chain/root payload candidate.
- The real outbound bytes must pass through `type@2cea.edc4`.
- For LunaU control, the next concrete target is to recover which `field@6faf` mode the APK uses and how `ee91(...)` receives encryption/config arguments.

## Step 102 - `ee91` and packet encryption integration layout

`code=0x565824` (`method@ee91`) is now split enough to model the outer packet layer.

Inputs/roles inferred from bytecode:
- `v40`: command id enum/object; `f4c9()` writes two little-endian bytes.
- `v41`: method enum/object; `ee7b()` writes one byte.
- `v43`: payload/body byte array.
- `v44/v45`: timestamp/monotonic value pair; passed through `f60f(...)`.
- `v47`: packet mode/type enum; checked by `ee8e(mode)`.
- `v48`: optional `PacketEncryptionParams`-like config object.

Unencrypted/no-config branch:
- Builds an internal payload `v3` of `9 + payload_len` bytes:
  - bytes `0..1`: command id little-endian from `f4c9()`.
  - byte `2`: method byte from `ee7b()`.
  - bytes `3..6`: time/nonce-derived bytes.
  - bytes `7..8`: zero.
  - bytes `9..`: original payload.
- If no encryption config is supplied, it wraps that internal payload with `ee8b(...)`.
- This matches the previously working device-info frame shape:
  - UCD2 payload `08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b`
  - command id `0x0008`
  - method byte `0x02`
  - 4 time/nonce bytes `01 00 00 80`
  - 2 zero bytes
  - body `08 30 08 0f 08 0b`

Encrypted/config branch:
- Reads config fields:
  - `field@7093`: encryption scheme enum.
  - `field@7094`: encryption manager/object.
- Builds a 12-byte UCD2 header seed:
  - magic `UCD2`
  - version `01`
  - dynamic header length (`14 + 1` or `14 + 29`, depending on scheme)
  - message type from `field@709a.eeb3()`
  - sequence from `ee93()`
  - payload length little-endian
- Calls:
  - `field@7094.method@f45e(internalPayload, headerSeed, f60f(timestamp))`
- The return is cast to `type@2d33`, with fields:
  - `field@7090`: ciphertext/final payload body
  - `field@7091`: first 12-byte auth/tag/header segment
  - `field@7092`: second 12-byte auth/tag/header segment
- For one scheme it builds a 31-byte extra header:
  - starts `40 1d <scheme-byte>`
  - copies `field@7091` at offset 3, length 12
  - copies `field@7092` at offset 15, length 12
- For the other scheme it builds a shorter extra header:
  - starts `40 01 <scheme-byte>`
- Finally it calls `ee8b(messageType, field@7090, extraHeader, seq)`.

`ee8b` confirmation:
- `code=0x5651f4` is the plain UCD2 frame constructor:
  - magic `55 43 44 32`
  - version `01`
  - header length `12 + extra_len`
  - message type via `eeb3()`
  - sequence byte
  - payload length little-endian
  - optional extra header at offset 12
  - payload after header
  - CRC from `ee8c`, little-endian tail.

`ee60` short-path evidence:
- `code=0x5646b4` is the `field@70a5` short/plain path.
- It uses a similar internal packet header, then wraps it in a short `type@2d1e` packet with `ff 07 40 ...` and a 16-bit checksum from `ee5f`.
- This is not the path observed for the working LunaU UCD2 device-info frame.

Current blocker:
- `method@f45e` still has no concrete safe method body in seg12.
- `method@96f6` also resolves outside the reconstructed method table, so earlier references to it are not trustworthy as a direct DEX method body.
- The next target is either:
  - recover `f45e` from protected/native material, or
  - avoid encrypted/config mode and finish a correct no-config UCD2 StopCapture packet through `ee8b`.

## Step 103 - Root serializer and template-chain clarification

Tool update:
- `reverse_apk/tools/luna_packet_candidate.py` now has APK-evidence helpers for the outer packet layer:
  - `build_ee91_internal_packet(...)`
  - `build_ucd2_from_encrypt_result(...)`
  - `OBSERVED_DEVICE_INFO_TIME_BYTES = 01 00 00 80`
- Existing known device-info output is unchanged.
- `cargo test` passed after the update:
  - 5 tests in `html_app`
  - 5 tests in `luna_mic_rust`

Root serializer evidence:
- `code=0x46c838` (`9cd6`) writes the complete root byte array:
  - big-endian `0xcafebabe`
  - `field@43fc`
  - registry/string table via `field@43fa.9e13(out)`
  - root flags/counters with `9c71`
  - field node count and `9d08(...)` nodes
  - action node count and `9d7f(...)` nodes
  - optional metadata blocks
  - if any action node reports `9d7a()`, it calls `9cd4(bytes, flags)` before returning.
- `code=0x46c7a4` (`9cd4`) is not encryption.
  - It clears root lists/state, parses the just-written bytes through `9c78/9c7a`, and serializes again through `9cd6()`.

Template-chain evidence:
- `code=0x480fcc` (`a0e8`) is the wrapper insertion helper:
  - parses input bytes with `a05e(input)`.
  - creates a `type@34ec` root from that parsed context.
  - creates wrapper/action `type@356c`.
  - calls `a0e0(0x90000, root, externalArg)`.
  - inserts wrapper with `context.9c79(wrapper, 8)`.
  - returns `root.9cd6()`.
- `code=0x480dd0` (`a0da`) generates a template root:
  - creates `type@34ec`.
  - writes `9cd7(53, 4097, v3, 0, string@b9f0, null)`.
  - writes `9cdb(9, string@a51d, string@3865, 0, 0)`.
  - calls `9cda()`.
  - returns `9cd6()`.
- `code=0x480ee0` calls `a0da(field@47db)` and stores/inserts that template into a surrounding builder.
- `code=0x480f34` calls `a0e8(input, field@47dd)` after a condition check.
- `code=0x480e38` writes `9d57(178, field@47db, string@a51d, string@3865)`.

Important distinction:
- The `a0da/a0e8/480e38` chain proves the root-template mechanism, but it uses `string@a51d/string@3865`.
- The Luna StopCapture branch previously recovered from `a03c/a03e/a03f` uses `string@6fb/string@98e9` and marker `9cb6(4234, ...)`.
- Therefore the template-chain mechanism is useful, but `a0da` is not yet proven to be the exact StopCapture input template.

Next target:
- Continue on the actual StopCapture branch:
  - `a03c -> a03d/a03e/a03f`
  - wrapper helper `a0e8/a0e0/a0e1/a0e4`
  - registry writer `9e13/9e12`
- Goal is to produce the full `root.9cd6()` payload, then pass it through the already-recovered `ee91/ee8b` outer packet layer.

## Step 104 - Live socket probe did not validate or reject candidate J

Device reachability:
- `Test-NetConnection 192.168.42.1 -Port 6666` returned `TcpTestSucceeded=True`.

One-off socket probes from this workstation:
- Same socket sequence:
  1. APK auth frame `05 0f`
  2. known device-info frame
  3. candidate J / `seq_wrapped_a03f`
- Received bytes:
  - after auth: none
  - after device info: none
  - after candidate J: none
- Additional cases:
  - device-info only: no bytes
  - `syNceNdinS` then device-info: no bytes

Interpretation:
- This does not prove candidate J is wrong.
- Earlier successful device-info responses came from the app's persistent session path, while these quick probes opened fresh sockets.
- The device may require the app's exact lower negotiation/state, may reject parallel/new clients, or may currently be bound to another session.
- Continue static APK-only reconstruction rather than using this no-response result as protocol evidence.

## Step 105 - StopCapture outer command id correction candidate

New APK-only static evidence:
- `seg07.dex` raw message descriptors show `StopCapture{` / `StopCaptureResp{`, meaning the protobuf-level `StopCapture` message itself can be empty.
- `seg08.dex code=0x47ed54` and `0x47f5a0` still write `method@9d5e(199, empty type@34f7)` in the StopCapture writer path.
- `seg08.dex code=0x47e804` treats `199` beside `198` in a capture command/state switch.
- `seg08.dex code=0x47f6dc` returns `199` unchanged for one branch, further supporting that `199` is an APK command selector/value, not just arbitrary body data.

Lower packet correction:
- `seg12.dex code=0x55e708` creates `type@2d24` command metadata from `field@6fc4.f4c9()`.
- `seg12.dex code=0x55e378` passes that metadata into `ee91(...)`.
- `ee91` stores the command id returned by `f4c9()` in the first two internal payload bytes.
- Therefore the older StopCapture candidates A-J are now suspect because they reused the device-info command id `0x0008`.

Added candidates:
- `K / command199_empty`:
  - internal command id `0x00c7` (`199`)
  - method byte `0x02`
  - body empty, matching `StopCapture{}` descriptor.
- `L / command199_selector`:
  - internal command id `0x00c7`
  - method byte `0x02`
  - body keeps the recovered selector buffer from `a03e/a03f`.

Files updated:
- `reverse_apk/tools/luna_packet_candidate.py`
- `src/adapters/luna_local.rs`
- `web/index.html`

Current interpretation:
- K is the cleanest new minimal packet candidate if LunaU expects command id `199` directly.
- L is a bridge candidate if `199` is both the outer command id and the action/body selector.
- A-J remain available for comparison, but the outer command id mismatch makes them lower confidence.

## Step 106 - K/L generated and locked by tests

Generated packets:
- K / `command199_empty`:
  - internal: `c7 00 02 01 00 00 80 00 00`
  - UCD2: `55 43 44 32 01 0c 04 10 09 00 00 00 c7 00 02 01 00 00 80 00 00 16 85 b7 5a`
- L / `command199_selector`:
  - internal: `c7 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0`
  - UCD2: `55 43 44 32 01 0c 04 10 18 00 00 00 c7 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 76 88 5c 4b`

App/UI:
- Added buttons:
  - `command199 empty K` -> `variant="command199_empty"`
  - `command199 selector L` -> `variant="command199_selector"`
- `src/adapters/luna_local.rs` now sends A-J with command id `0x0008`, and K/L with command id `0x00c7`.

Verification:
- `python .\reverse_apk\tools\luna_packet_candidate.py | Select-String -Pattern "command199|known_device_info"` prints the expected known device-info frame plus K/L.
- `cargo test` passed:
  - `html_app`: 6 tests passed.
  - `luna_mic_rust`: 6 tests passed.
  - Existing dead-code warnings only.

Next target:
- K is currently the strongest final-packet candidate from APK-only evidence, but it still needs response validation on the existing persistent LunaU session.
- If K returns only an ACK/empty heartbeat and no error, then test L.
- If both fail, continue static reverse on the exact command enum object feeding `field@6fc4.f4c9()` and recover the command id table rather than assuming `199`.

## Step 107 - Enum-table follow-up after K/L

Checked seg12 command/status tables:
- `seg12.dex code=0x562ee0` initializes enum-like objects in fields `701a/701c/7012/701b/7019/7014/7017/7016/701e/7013/7015/7018/701d`.
- Constructor arguments cover small values `1..5` and paired values `19..26`.
- These fields are used around coroutine/send result code paths (`field@6fac`, `field@7017`) and do not include `199`.
- This table therefore does not look like the StopCapture command-id table.

Checked seg12 for literal `199`:
- `dex_code_scan seg12 --target-const 199` found no code-item hit.
- This supports that `199` is produced in the upper command/action layer (`seg08`) before the packet layer.

Checked seg08 for relevant constants:
- `0x108a` appears in `0x47ed28` (`a03d`) as the marker node `9cb6(4234, string@6fb, string@98e9, 0, 0)`.
- `0x1089` appears in `0x481138` (`a0e4`) as wrapper marker `9cb6(4233, externalArg, string@3865, 0, 0)`.
- `0x00c7` appears in:
  - `0x42a51c` range/static objects with `199/192`.
  - `0x47e804` capture-state switch.
  - `0x47ed54` / `0x47f5a0` StopCapture writer paths.
  - `0x47f6dc` mapping returning `199`.

Current conclusion:
- APK-only static evidence now says:
  - `199` is definitely a StopCapture-family upper selector/value.
  - It is not present as a lower seg12 built-in packet enum constant.
- Therefore there are two plausible packet shapes:
  1. Generic lower command id `0x0008` plus reconstructed action/root payload (old A-J family).
  2. Direct lower command id `0x00c7` with empty StopCapture message (new K family).
- K is simpler and matches `StopCapture{}` better, but A-J cannot be eliminated until persistent-session device response is observed.

## Step 108 - Re-read 0x472944 and add no-length StopCapture variants

Re-read the seg08 writer backend:
- `0x472944` is the backend reached by wrapper `9d5e(int, object)`.
- It computes a compact/state value:
  - if `command < 200`, `v3 = command - 33`.
  - for `199`, the compact/state value is `166 / 0xa6`.
- Important correction: in the non-length empty-object branch, `v3 != original command`, so the branch writes the original command byte through `9c6d(v11)`.
  - For StopCapture `199`, that is still `0xc7`.
  - The earlier `0xa6` reading is therefore a state/index value, not proven output bytes for this StopCapture path.

Remaining uncertainty:
- `9d5e(199, empty type@34f7)` writes `0xc7`, then calls unresolved `type@34f7.9d43(...)`.
- Current A/B candidates assume that unresolved empty serializer writes or triggers the four-byte length patch:
  - base body: `b2 00 03 59 c7 00 00 00 05 57 b0`
  - a03f body: `b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0`
- Because `type@34f7.9d43` is not defined in this reconstructed segment, added no-length alternatives where empty `9d43` contributes no bytes:
  - M / `base_nolen` body: `b2 00 03 59 c7 57 b0`
  - N / `a03f_nolen` body: `b2 00 03 59 c7 57 59 b3 00 03 b0`

Generated packets:
- M / `base_nolen`:
  - internal: `08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 57 b0`
  - UCD2: `55 43 44 32 01 0c 04 10 10 00 00 00 08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 57 b0 73 42 b0 c0`
- N / `a03f_nolen`:
  - internal: `08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 57 59 b3 00 03 b0`
  - UCD2: `55 43 44 32 01 0c 04 10 14 00 00 00 08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 57 59 b3 00 03 b0 d7 ba e2 5e`

Files updated:
- `reverse_apk/tools/luna_packet_candidate.py`
  - Added `stop_capture_inner_no_length_candidate(...)`.
  - Prints M/N packet candidates.
- `src/adapters/luna_local.rs`
  - Added `STOP_CAPTURE_NO_LENGTH_BODY` and `STOP_CAPTURE_NO_LENGTH_A03F_BODY`.
  - Added app variants `base_nolen` and `a03f_nolen`.
  - Added packet assertions for M/N.
- `web/index.html`
  - Injects UI buttons after StopCapture B:
    - `无长度 M`
    - `无长度 N`

Verification:
- `python .\reverse_apk\tools\luna_packet_candidate.py` prints M/N as above.
- `cargo fmt` completed.
- `cargo test` passed:
  - `html_app`: 6 tests passed.
  - `luna_mic_rust`: 6 tests passed.
  - Existing dead-code warnings only.

Next target:
- On the existing persistent LunaU session, test M then N if K/L and A/B do not produce a meaningful control response.
- Static reverse should next chase the actual implementation of `type@34f7.9d43` across reconstructed segments or source clusters; that is the shortest path to eliminate either A/B or M/N.

## Step 109 - M/N eliminated; length patch path confirmed

Re-opened the old Step 78 evidence and disassembled the real empty-body methods:
- `seg08.dex code=0x470194` is `type@34f7.9d43`.
- `seg08.dex code=0x46fd20` is `type@34f7.9d44`.

`0x470194 / 9d43` behavior:
- Reads `field@44e1` flags.
- If `(flags & 4) != 0` and the incoming boolean flag is true:
  - records a `0x20000000` patch marker via `9d3b(offset, marker, current_pos)`;
  - writes `-1` through `9c6f`, i.e. a four-byte placeholder.
- If the boolean flag is false it records `0x10000000` and uses the short write path `9c71`.
- If `(flags & 4) == 0`, it writes `field@44e0 - offset` directly.

`0x46fd20 / 9d44` behavior:
- Sets `field@44e1 |= 4` and stores the current position into `field@44e0`.
- Later walks the patch table.
- For marker `0x20000000`, it writes the computed length as four big-endian bytes.

Conclusion:
- The no-length M/N alternatives from Step 108 are not the APK StopCapture path.
- `9d5e(199, empty type@34f7)` really produces:
  - `c7 ff ff ff ff` during the first write,
  - then `9d44` patches the four bytes to the body span length.
- Therefore the supported StopCapture body candidates remain the length-patched family:
  - A: `b2 00 03 59 c7 00 00 00 05 57 b0`
  - B: `b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0`
- Since `0x47ed54` calls `a03f(...)` unconditionally after `9d5a(87)`, B is stronger than A for that concrete StopCapture template.

Code cleanup:
- Removed `base_nolen` / `a03f_nolen` from:
  - `reverse_apk/tools/luna_packet_candidate.py`
  - `src/adapters/luna_local.rs`
  - `web/index.html`
- Kept Step 108 in the log as a negative branch, but Step 109 supersedes it.

Next target:
- Continue closing whether the packet delivered to seg12 is:
  - B as the raw main buffer,
  - D/F as the full `0x471b8c` builder node,
  - H/J as the surrounding action/wrapper sequence,
  - or K as direct command id `0x00c7` with empty protobuf.

## Step 110 - Wrapper external argument remains the static boundary

Reconfirmed StopCapture entry:
- `seg08.dex code=0x47ed08 / a03c`:
  - calls `a03d(v1)`;
  - then calls `a03e(v1, v2)`.
- `seg08.dex code=0x47ed28 / a03d`:
  - writes marker `9cb6(4234, string@6fb, string@98e9, 0, 0)`.
- `seg08.dex code=0x47ed54 / a03e`:
  - writes the builder body and unconditionally calls `a03f(...)`.
  - Therefore the `a03f` length-patched body is stronger than the base body for this concrete template.

Reconfirmed wrapper helper:
- `seg08.dex code=0x480fcc / a0e8`:
  - `a05e(input)` creates/loads a root context from runtime input bytes.
  - constructs a root wrapper object.
  - constructs `type@356c` action wrapper via `a0e0(0x90000, root, externalArg)`.
  - inserts it with `context.9c79(wrapper, 8)`.
  - returns `root.9cd6()`.
- `seg08.dex code=0x481138 / a0e4`:
  - writes wrapper marker `9cb6(4233, externalArg, string@3865, 0, 0)`.

External argument source:
- `seg08.dex code=0x480f34`:
  - checks `field@47de`.
  - if true, calls `a0e8(input, field@47dd)`.
- `seg08.dex code=0x480f68`:
  - constructor-like method stores:
    - `field@47de = arg1`
    - `field@47dd = arg2`
  - then calls parent constructor.
- Direct scans show `field@47dd` only in `0x480f34` and `0x480f68`.
- No normal Java caller of `a0e8` was found; the method table around this area is damaged enough that method-id reverse lookup by class_data is unreliable.

Current narrowing:
- Eliminated:
  - no-length M/N branch.
- Still plausible:
  - B: raw a03f main buffer.
  - D/F: full builder node with a03f.
  - H: `a03c` marker + a03f builder sequence.
  - J: wrapper marker + `a03c` marker + a03f builder sequence.
  - K: direct lower command id `0x00c7` + empty StopCapture protobuf.
- Strongest APK-only action-chain candidate remains J, but its wrapper marker first argument is runtime `field@47dd`, not a static literal.

High-signal packets for device validation:
- B / `a03f`:
  - `55 43 44 32 01 0c 04 10 18 00 00 00 08 00 02 01 00 00 80 00 00 b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 74 ef 99 27`
- D / `full_a03f`:
  - `55 43 44 32 01 0c 04 10 3c 00 00 00 08 00 02 01 00 00 80 00 00 10 00 00 01 00 02 00 03 00 04 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 03 59 c7 00 00 00 09 57 59 b3 00 03 b0 00 00 00 05 00 00 00 00 00 06 00 00 00 00 e8 78 8f 62`
- H / `seq_a03f`:
  - `55 43 44 32 01 0c 04 10 4c 00 00 00 08 00 02 01 00 00 80 00 00 00 02 10 8a 00 01 00 02 00 01 00 03 00 00 00 00 10 00 00 04 00 05 00 03 00 07 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 06 59 c7 00 00 00 09 57 59 b3 00 06 b0 00 00 00 08 00 00 00 00 00 03 00 00 00 00 21 c5 87 ae`
- J / `seq_wrapped_a03f`:
  - `55 43 44 32 01 0c 04 10 5a 00 00 00 08 00 02 01 00 00 80 00 00 00 03 10 89 00 01 00 02 00 01 00 03 00 00 00 00 10 8a 00 04 00 05 00 01 00 03 00 00 00 00 10 00 00 06 00 07 00 03 00 09 00 00 00 19 00 02 00 00 00 00 00 0f b2 00 08 59 c7 00 00 00 09 57 59 b3 00 08 b0 00 00 00 0a 00 00 00 00 00 03 00 00 00 00 49 57 f5 6d`
- K / `command199_empty`:
  - `55 43 44 32 01 0c 04 10 09 00 00 00 c7 00 02 01 00 00 80 00 00 16 85 b7 5a`

Next target:
- Without runtime capture of `field@47dd` / `externalArg`, static APK evidence cannot fully prove J's wrapper registry id assumptions.
- The next useful local-only route is to continue around the `a05e(input)` root parser and `9cd6()` output to see whether wrapper `field@47dd` can be inferred from the returned root object rather than from Java callers.

## Step 111 - Root mode fixed, wrapper externalArg still not static

Checked the root/context route around `a05e(input)` and `9cd6()`:
- `a05e` has only two visible caller shapes in seg08:
  - `0x47c980`
  - `0x480fcc / a0e8`
- `0x480fcc` passes flags `0` to the wrapper action and inserts it with `context.9c79(wrapper, 8)`.

Checked constructors/context methods:
- `0x46d4dc` is a tiny constructor forwarding to `9cc2(0, arg)`.
- `0x47c964` stores a field then forwards to `9cc2(arg2, arg3)`.
- `0x46d4f8` initializes a root/action object:
  - starts with `0x90000`;
  - stores `field@43e5 = flags`;
  - creates a registry/helper object;
  - calls `9cd5(flags)`.
- `0x46d540 / 9cd5`:
  - if `(flags & 2) != 0`, sets `field@43dd = 4`;
  - else if `(flags & 1) != 0`, sets `field@43dd = 1`;
  - else sets `field@43dd = 0`.
- For `0x480fcc`, flags are `0`, so the wrapper root mode is statically fixed to `field@43dd = 0`.

Updated interpretation:
- J's root-mode assumption is now supported: `a0e8` starts from mode `0`.
- The unsolved value is narrower:
  - not root flags,
  - not CRC,
  - not UCD2 wrapper,
  - not the `199` length patch,
  - only the runtime `externalArg` stored in `field@47dd` and then used by `a0e4` as wrapper marker arg2.
- Because `field@47dd` is set only by constructor-like `0x480f68`, and no normal Java caller was found, this value is probably provided by a runtime/native/callback registration path.

Practical consequence:
- For static APK-only generation, J remains the closest wrapper/action-chain packet, but the first wrapper registry entry is still an assumption.
- If avoiding the unresolved runtime `externalArg`, H (`seq_a03f`) is the strongest fully static action-chain packet for `a03c` itself.
- If the actual LunaU transport bypasses the action-chain wrapper and sends a direct onedriver packet, K remains the strongest lower-packet candidate.

## Step 112 - G/H/J are action-list candidates, not full `root.9cd6()`

Re-read the complete root serializer:
- `seg08.dex code=0x46c838 / 9cd6` is the full root byte-array output.
- It does not start with the action count.
- It writes:
  - `0xCAFEBABE` via `9c6f`;
  - root version/flags fields such as `field@43fc`, `field@43dc`, `field@43fb`, `field@43f9`;
  - registry/index data via `field@43fa.9e13(out)`;
  - root array/count fields;
  - then the `field@43e3` action chain:
    - ORs node flags with `9d7b/9d7a`;
    - writes each node through `9d7f(out)`;
    - finally writes the action-node count.

Re-read parser side:
- `seg08.dex code=0x47f2a8 / a05e`:
  - reads the input version with `a060(input)`;
  - if version is `70`, temporarily writes version `69` through `a063(69, input)`;
  - constructs `type@34e9` through `9c76(input)`;
  - restores the original version through `a063(original, input)`.
- `seg08.dex code=0x469d1c / 9c76` forwards to `9c77(input, 0, input.length)`.
- `seg08.dex code=0x469d38 / 9c78` parses the root/index table:
  - checks version at `input[offset + 6]`;
  - reads a count at `input[offset + 8]`;
  - builds arrays `field@43d2` and `field@43d1`;
  - stores parse offsets/lengths in `field@43d3/field@43d4`.

Important correction:
- Existing G/H/I/J packets in the app are not complete `root.9cd6()` outputs.
- They model only the action-node count and nodes:
  - H starts with `00 02 ...`
  - J starts with `00 03 ...`
- A complete root candidate must prepend the `CAFEBABE` root header and registry/string table generated by `9cd6`.
- That registry/header cannot be safely generated from the current manual action-list builder until `field@43fa` serialization (`9e13`, `9df4`, and related registry methods) is closed.

Code/UI updates:
- `reverse_apk/tools/luna_packet_candidate.py`
  - comments now identify G/H/I/J as action-node candidates, not full root output.
- `src/adapters/luna_local.rs`
  - returned notes for `seq_*` variants now say "action-list candidate" and "not full root.9cd6 output".
- `web/index.html`
  - button labels changed from `APK序列 G/H/I/J` to `动作序列 G/H/I/J`.
  - StopCapture card now labels G-I as action list and J as wrapper action.

Current best interpretation:
- The packet most faithful to the APK root path is not implemented yet; it must be a new full-root candidate after registry serialization is recovered.
- Until then:
  - H/J remain useful diagnostic action-list probes.
  - K remains useful as the simplest lower Packet command-id hypothesis.
  - None of H/J/K is a proven final packet.

Next target:
- Recover enough registry serialization to build a full `root.9cd6()` candidate:
  - `field@43fa.9e13(out)`
  - `field@43fa.9df4(string)` id assignment
  - registry count/string record layout.

## Step 113 - Registry is the remaining root serializer dependency

Read more root/registry-related methods.

`seg08.dex code=0x46d578`:
- Stores root fields:
  - `field@43fc = version`
  - `field@43dc = flags`
- Calls `field@43fa.9e14(version & 0xffff, arg)` and stores result in `field@43fb`.
- Registers an optional string through `field@43fa.9df4(...)` into `field@43f7`.
- Looks up optional objects through `field@43fa.9ddb(...)` and stores their registry ids.
- If an array is supplied, stores each object's registry id into `field@43e8`.
- If mode is `1` and version >= `51`, upgrades `field@43dd` to `2`.

`seg08.dex code=0x46d66c`:
- Lazily creates `field@43e6` as a byte buffer.
- Uses `field@43fa.9ddb(...)` to resolve object/string registry records.
- Writes registry ids into `field@43e6`.
- Maintains `field@43f3` count and writes back a per-record index into `field@465f`.

`seg08.dex code=0x467568`:
- Computes the extra bytes needed for node metadata.
- It calls `9df4` for metadata strings:
  - `string@8b21`
  - `string@898d`
  - `string@259e`
- This matches `0x467744`, which actually writes those metadata records.

`seg08.dex code=0x467744`:
- Writes node metadata records:
  - optionally `string@8b21` with zero length;
  - optionally `string@898d` with a two-byte-ish value record;
  - optionally `string@259e` with zero length.

Updated status:
- `9cd6` full root output is now clearly blocked on the registry object `field@43fa`.
- Known registry methods still needing direct recovery:
  - `9df4(string)` -> assign/find id for a string.
  - `9ddb(object/string)` -> return registry record with fields `465e/465f`.
  - `9e13(out)` -> write full registry/string table into root output.
  - `9e02/9e03/9dfd` -> sizes/counts used before root buffer allocation.
- Until these are recovered, adding a "full root" packet would be speculative and less honest than the current action-list candidates.

Practical note for continuation:
- Do not treat G/H/I/J as final packets.
- The next productive static target is to find the code bodies for the registry methods above, likely in the same damaged class-data region as the `0x466xxx` methods.

## Step 114 - Registry node layout from the `0x466xxx` method cluster

Continued static recovery from `seg08.dex`, using only APK DEX evidence.

Important method cluster:
- `0x466b28`
  - Size calculator for a linked `type@34e5` registry node chain.
  - If an input string/object is present, calls `registry.9df4(...)`.
  - Starts with a base size of `8`.
  - Walks child nodes via `field@43bb` and adds each child buffer length `field@43c5`.
- `0x466dac`
  - Writer for a linked node chain.
  - Computes node count and total size from `field@43bb`.
  - Writes:
    - marker/id through `9c71(...)`;
    - total length through `9c6f(...)`;
    - count through `9c71(...)`;
    - then copies each child buffer `field@43c4` with length `field@43c5` through `9c6e(...)`.
- `0x466bc0`
  - Size calculator for an array/list node.
  - Formula pattern: `count * 2 + 7`, then adds each child size minus `8`.
- `0x466e88`
  - Writer for an array/list node.
  - Formula pattern before write: `count * 2 + 1`, plus each child size minus `8`.
  - Writes:
    - marker/id through `9c71(...)`;
    - total length through `9c6f(...)`;
    - count through `9c6d(...)`;
    - per item: child count/id through `9c71(...)`, then child payload copy through `9c6e(...)`.
- `0x466c04`
  - Builds a `type@34e5` node for an object/name pair.
  - Registers two strings through `9df4`.
  - Writes the first string id, then a child marker `0x40` (`'@'`) plus second string id, then `0`.
- `0x466c78`
  - Builds an empty array/list-like `type@34e5` node.
  - Registers a string through `9df4`.
  - Writes the string id, then marker `0x5b` (`'['`) plus `0`.
- `0x466cd8`
  - Builds a `type@34e5` node using a fresh `type@34e8` byte writer.
  - Calls static writers `9e58(...)` and `9e44(...)`, then registers a string id with `9df4`, writes that id, writes `0`, and constructs the node.
- `0x466d24`
  - Similar to `0x466cd8`, but only writes a registered string id and `0`.
- `0x467324`
  - Builds/writes enum-like metadata:
    - registers first string id;
    - writes child marker `0x65` (`'e'`) with second string id;
    - registers third string id and writes it.
- `0x466f30`
  - Large type-dispatch node builder.
  - Observed marker constants:
    - `0x73` (`'s'`)
    - `0x42` (`'B'`)
    - `0x5a` (`'Z'`)
    - `0x43` (`'C'`)
    - `0x53` (`'S'`)
    - `0x63` (`'c'`)
    - `0x5b` (`'['`)
    - `0x49` (`'I'`)
    - `0x4a` (`'J'`)
    - `0x46` (`'F'`)
    - `0x44` (`'D'`)
  - This matches primitive/string/array node construction used by the root registry.

Parser cross-check:
- `0x4680e8` is a parser for the same registry node stream.
- It checks marker constants `0x65`, `0x5b`, `0x40`, then branches on primitive markers including `F/S/c/s/I/J/Z`.
- This independently supports the writer interpretation above.

Current impact:
- The full `root.9cd6()` packet is closer, but still not final.
- We now know registry nodes are structured records, not a simple string table.
- Remaining unknowns for a provable full-root packet:
  - exact `9df4` id assignment/order for all strings used by `StopCapture`;
  - exact `9e13(out)` registry table header and record order;
  - exact `9e02/9e03/9dfd` size/count semantics used by `9cd6` before allocation.

Next static target:
- Recover the class-data mapping or enough surrounding code for `field@43fa` methods:
  - `9df4`
  - `9ddb`
  - `9e13`
  - `9e02`
  - `9e03`
  - `9dfd`
- Then generate a new full-root candidate instead of extending the current action-list candidates.

## Step 115 - Registry writer body recovered

Continued APK-only static recovery from `seg08.dex`.

Strong registry class candidate:
- Methods around `0x474xxx` / `0x475xxx` use fields:
  - `field@466d`: accumulated `type@34e8` byte buffer.
  - `field@466e`: registry entry count.
  - `field@466f`: record/index array, initialized to length `256`.
  - `field@4675`: optional parsed/attached data.
  - `field@4662/4663/4664/4665`: per-record matching/cache fields.
- This is very likely the implementation backing root field `field@43fa`.

Constructor/init:
- `0x475288`
  - stores owner/root reference into `field@466c`;
  - clears `field@4675`;
  - allocates `field@466f` with `256` slots;
  - initializes `field@466e = 1`;
  - initializes `field@466d = new type@34e8()`.

Recovered registry writer:
- `0x4758e0` matches `field@43fa.9e13(out)` by signature and root call site:
  - takes `this + out`;
  - reads `field@466e`;
  - writes it with `out.9c71(...)`;
  - reads `field@466d`;
  - copies `field@466d.field@43c4` with length `field@466d.field@43c5` through `out.9c6e(...)`.
- Therefore `9e13(out)` layout is:
  - `u16 registry_count`
  - raw accumulated registry bytes

Recovered registry size/count roles from `root.9cd6()`:
- In `0x46c838`:
  - `field@43fa.9e03()` is added to root allocation size before the output buffer is created.
  - `field@43fa.9e02()` is checked against `0xffff`.
  - `field@43fa.9e13(out)` is then called immediately after:
    - `out.9c6f(0xCAFEBABE)`
    - `out.9c6f(field@43fc)`
- Based on the recovered writer, current interpretation:
  - `9e03()` = serialized registry byte size (`2 + field@466d.field@43c5`).
  - `9e02()` = registry count (`field@466e`) or max-id count used for the `0xffff` guard.
  - `9e13(out)` = write count plus raw bytes.

Additional supporting methods:
- `0x474538` returns `field@466e`; likely one of the count getters.
- `0x47474c` returns `field@4675`.
- `0x474f64`
  - calls true `9df4(string)` to get a string id;
  - writes a marker plus that id into `field@466d` via `9c6b(...)`;
  - creates a `type@3501` record and stores it through `9e11(...)`.
- `0x47410c`
  - calls true `9df4` twice for two strings;
  - writes a compound record with marker `12` through `9c6c(...)`;
  - increments `field@466e`;
  - creates/stores a `type@3501` record.

Current packet status:
- The root registry envelope is now mostly known.
- The final StopCapture root packet is still not safe to emit because the exact accumulated `field@466d` bytes require replaying the true APK registry-building sequence:
  - exact `9df4(string)` id assignment and string record encoding;
  - exact `9ddb(...)` object record reuse/creation;
  - exact marker-specific methods in the `0x474xxx` cluster (`9dda`, `9ddb`, `9def`, `9df2`, etc.).

Next target:
- Recover true `9df4(string)` by finding the method in this same `0x474xxx/0x475xxx` cluster that:
  - takes `this + string`;
  - returns an `int`;
  - searches existing `type@3501` records via `field@4663/4664/4665`;
  - if missing, writes a string record into `field@466d`, increments `field@466e`, and returns the new id.
- Once `9df4` is closed, replay the StopCapture builder call order to generate the real `field@466d` bytes and then the full `root.9cd6()` payload.

## Step 116 - More registry cluster details; true `9df4` still unresolved

Continued APK-only static recovery from the `0x474xxx/0x475xxx` registry cluster.

Additional record constructors:
- `0x474f64`
  - Takes a marker/value pair plus a string-like object.
  - Computes a record hash/key through `9e0b(...)`.
  - Searches an existing record through `9dff(...)`.
  - If missing:
    - calls true `9df4(string)` to get the string id;
    - writes marker + string id to `field@466d` via `9c6b(marker, id)`;
    - creates a `type@3501` record;
    - increments `field@466e`;
    - inserts the record through `9e11(...)`.
- `0x47410c`
  - Two-string compound record constructor.
  - Calls true `9df4(...)` twice.
  - Writes marker `12` plus both ids via `9c6c(...)`.
- `0x474c84`
  - Numeric record constructor.
  - Writes marker through `9c6d(...)`, then a 4-byte-ish value through `9c6f(...)`.
  - Creates/stores a `type@3501` record and increments `field@466e`.
- `0x474d4c`
  - Similar numeric/long-ish record constructor.
  - Writes marker through `9c6d(...)`, then value through `9c70(...)`.
  - Increments `field@466e` by `2`.
- `0x474b7c`
  - Compound record constructor.
  - Uses `9df0(...)` and writes through `9c6c(...)`.

Raw/string table helpers:
- `0x4744e4`
  - If `field@466a` exists:
    - calls true `9df4(string@1d75)`;
    - returns `field@466a.field@43c5 + 8`.
  - Else returns `0`.
- `0x47587c`
  - If `field@466a` exists:
    - writes `id(string@1d75)`;
    - writes `field@466a.field@43c5 + 2`;
    - writes `field@4669`;
    - copies raw `field@466a` bytes.
- `0x474790`
  - Uses marker `0x40`.
  - Compares an incoming raw byte span with existing `field@466a` contents.
  - If missing, creates a `type@3501` record with marker `0x40` and inserts it through `9e11(...)`.
- `0x475750`
  - Parses a raw/string section from an existing input:
    - checks `string@1d75`;
    - sets `field@4669`;
    - rebuilds `field@466a` as a `type@34e8` byte buffer;
    - creates `0x40` records for raw chunks.

Important current interpretation:
- `field@466d` is the main registry byte stream written by `9e13`.
- `field@466a/4669` is a secondary raw/string section, likely written separately by `0x47587c` when present.
- `0x4744e4` / `0x47587c` explain why `root.9cd6()` had a separate `9dfd()` size branch before full registry size:
  - `9dfd()` is likely the size of this optional raw/string section.

Still unresolved:
- The true body of `method@9df4(string)` has not been mapped to a code offset.
- It is clearly used by many record constructors, but damaged class-data prevents direct method-id-to-code-offset lookup.
- Without `9df4`, we still cannot safely compute the exact string-id order and `field@466d` bytes for StopCapture.

Next static target:
- Recover `9df4` indirectly by following the `field@466d` record constructors and the record table insertion method `9e11(...)`.
- Useful candidates:
  - `0x4750d8`: hash-table insertion / resize for `field@466f`.
  - `0x475574`: lightweight insert/update for `field@4670/466f/4664/4665`.
  - `0x474790`: marker `0x40` raw record constructor.
  - Methods calling `9dff(...)`: record lookup by precomputed key/hash.

## Step 117 - Closed true `9df4(string)` by semantics

APK-only static recovery continued in `seg08.dex`.

High-confidence true `9df4(string)` body:
- Code offset: `0x4741b8`.
- Shape:
  - `regs=7`, `ins=2`, `outs=5`.
  - Takes `this + string`.
  - Returns `int` through `return`, not `return-object`.
- Existing-record path:
  - Loads marker/type `1`.
  - Calls static `9e0b(1, string)` to compute a hash/key.
  - Calls direct `9dff(this, key)` to get the first bucket record.
  - Walks records and checks:
    - `record.field@4662 == 1`;
    - `record.field@4664 == key`;
    - `record.field@4663.equals(string)`.
  - If matched, returns `record.field@465e`.
- New string path:
  - Uses current `field@466e` as the new registry id.
  - Writes a string record into `field@466d`:
    - `field@466d.9c6d(1)`;
    - `field@466d.9c72(string)`.
  - Creates `type@3501` with constructor `9dd0(id, 1, string, key)`.
  - Increments `field@466e`.
  - Inserts the record via `9e11(this, record)`.
  - Returns `record.field@465e`.

Why this closes the unresolved method:
- Every known `method@9df4` caller passes `{registry, string}` and consumes `move-result` as an integer id.
- The body at `0x4741b8` exactly performs string-id interning into the registry stream.
- A scan for `method@9c72` found only this caller in the current APK scan, which makes `9c72(string)` the string-record writer used by `9df4`.
- Damaged class-data still prevents a clean method-id-to-code-offset map, but the semantic match is strong enough to continue packet reconstruction.

Related hash table helpers:
- `0x4750b8`
  - Lookup helper.
  - Returns `field@466f[key % field@466f.length]`.
- `0x4750d8`
  - Full hash insert / resize for `field@466f`.
  - If `field@4670 > field@466f.length * 3 / 4`, allocates a new table of `old_len * 2 + 1` and rehashes by `record.field@4664 % new_len`.
  - Increments `field@4670`.
  - Chains the new record into `field@466f[record.field@4664 % len]` via `record.field@4665`.
- `0x475574`
  - Lightweight insert/update.
  - Increments `field@4670`.
  - Chains record into the same `field@466f` bucket using `field@4664` and `field@4665`.
- `0x475288`
  - Registry constructor.
  - Sets:
    - `field@466c = owner/root`;
    - `field@4675 = null`;
    - `field@466f = new type@46af[256]`;
    - `field@466e = 1`;
    - `field@466d = new type@34e8()`.

Envelope writer confirmed:
- `0x4758e0` matches `field@43fa.9e13(out)`.
- It writes:
  - `out.9c71(field@466e)`;
  - raw bytes from `field@466d.field@43c4` with length `field@466d.field@43c5` through `out.9c6e(...)`.
- Therefore the registry envelope inside `root.9cd6()` is:
  - `u16-ish registry_count`;
  - raw registry byte stream.

Next static target:
- Recover `type@34e8` writer helper byte formats from APK code, especially:
  - `9c6d(marker)`;
  - `9c71(value)`;
  - `9c6f(value)`;
  - `9c70(value)`;
  - `9c6b(marker, id)`;
  - `9c6c(marker, id_a, id_b)`;
  - `9c72(string)`.
- Once those helper formats are proven, replay StopCapture builder order to compute exact `field@466d` bytes and then full `root.9cd6()` payload.

## Step 118 - Recovered `type@34e8` writer byte formats

APK-only static recovery continued in the `type@34e8` buffer cluster around `0x4678xx-0x467dxx`.

Confirmed byte buffer fields:
- `field@43c4`: backing byte array.
- `field@43c5`: current length/write position.
- `0x467dc4`: ensure-capacity helper, called as `method@9c68`.

Recovered writer formats:
- `method@9c6d(value)`:
  - Code shape: one-byte append, likely `0x467ad0`.
  - Writes `value & 0xff`.
- `method@9c71(value)`:
  - Code shape: two-byte append, likely `0x467c60`.
  - Writes big-endian unsigned 16-bit:
    - `(value >>> 8) & 0xff`;
    - `value & 0xff`.
- `method@9c6f(value)`:
  - Code shape: four-byte append, `0x467b50`.
  - Writes big-endian unsigned 32-bit:
    - `value >>> 24`, `>>> 16`, `>>> 8`, `value`.
- `method@9c70(value)`:
  - Code shape: eight-byte append, `0x467bb8`.
  - Writes big-endian unsigned 64-bit:
    - high 32-bit word first, then low 32-bit word.
- `method@9c6b(marker, id)`:
  - Code shape: three-byte append, `0x467a10`.
  - Writes:
    - one marker byte;
    - one big-endian u16 id.
- `method@9c6c(marker, id_a, id_b)`:
  - Code shape: five-byte append, `0x467a64`.
  - Writes:
    - one marker byte;
    - `id_a` as big-endian u16;
    - `id_b` as big-endian u16.
- `method@9c6e(bytes, offset, length)`:
  - Code shape: raw byte copy, `0x467b0c`.
  - Ensures capacity, then copies `length` bytes from input array at `offset`.
- `method@9c72(string)`:
  - Public string append path is the ASCII-fast method at `0x467cac`.
  - It writes a big-endian u16 byte length, then encoded bytes.
  - If it hits a non-ASCII or special character, it falls through to helper `method@9c67`, code offset `0x467818`.
  - `0x467818` computes Java-style modified UTF-8 length:
    - chars in `1..0x7f` use one byte;
    - chars up to `0x7ff` use two bytes;
    - others use three bytes;
    - NUL is not emitted as raw `00`, matching modified UTF behavior.

Important correction:
- Earlier assumptions that `9c72` directly pointed at `0x467818` were incomplete.
- More precise view:
  - `9c72` is the public string writer at `0x467cac`;
  - `9c67` is the slow/non-ASCII helper at `0x467818`.
- For StopCapture's observed ASCII strings, both paths produce:
  - `u16_be(len(ascii_bytes)) + ascii_bytes`.

Registry implication:
- True `9df4(string)` string record bytes are now proven:
  - `01`;
  - `u16_be(modified_utf8_length)`;
  - modified UTF-8 bytes.
- Marker/string-pair records from `0x474f64` are:
  - `marker`;
  - `u16_be(string_id)`.
- Two-string compound records from `0x47410c` are:
  - `0c`;
  - `u16_be(first_string_id)`;
  - `u16_be(second_string_id)`.
- Numeric records from `0x474c84` are:
  - `marker`;
  - `u32_be(value)`.
- Long-ish numeric records from `0x474d4c` are:
  - `marker`;
  - `u64_be(value)`;
  - and increment the registry id counter by `2`.

Next static target:
- Replay the actual StopCapture root/action builder using these proven writer formats.
- Need recover the exact root field initialization path that sets:
  - root header fields around `field@43fc`, `field@43fb`, `field@43f9`, `field@43e7`, `field@43e8`, `field@43e1`;
  - action list `field@43e3`;
  - action-node buffers under `field@43e6`, `field@43de`, `field@43f2`, `field@43f6`, etc.

## Step 119 - Rechecked wrapper/template route after writer recovery

After recovering `9df4` and the `type@34e8` writer methods, rechecked the actual StopCapture root/wrapper path instead of generating another action-list-only candidate.

Confirmed existing local evidence:
- `0x480fcc` is the wrapper insertion helper:
  - calls `a05e(input)` to parse an existing byte-array context;
  - constructs `type@34ec` root with `9cc2(context, 0)`;
  - constructs `type@356c` through `a0e0(0x00090000, root, externalArg)`;
  - inserts that wrapper through `context.9c79(wrapper, 8)`;
  - returns `root.9cd6()`.
- `0x480f68` is the small constructor that stores:
  - `field@47de = first_arg`;
  - `field@47dd = second_arg`.
- `0x480f34` is the only observed caller of `a0e8(input, field@47dd)`:
  - it checks `field@47de` against an incoming value;
  - if it matches, it calls `a0e8(...)`.
- `field@47dd` and `field@47de` are only written by that constructor path, so they are runtime inputs, not APK-static constants.
- `0x480ee0` generates/attaches a template through `a0da(field@47db)`, but this is an upper object assembly path:
  - it does not directly call `a0e8`;
  - it stores/inserts the generated template under string `a51d`.

Wrapper object facts:
- `0x481138 / a0e4` writes the extra wrapper marker:
  - `9cb6(4233, external_arg, string@3865, 0, 0)`.
- `0x481014` writes:
  - `9d57(178, field@47e3, field@47e1, string@3865)`;
  - then delegates to `a0f5(...)`.
- `0x47c980` is a generic transform path:
  - validates/parses an existing byte array;
  - creates a root from the parsed context;
  - creates an action from parsed state;
  - inserts the action through `context.9c79(action, 8)`;
  - returns `root.9cd6()`.
  - It still depends on the incoming byte array.

Static resource/template search:
- Searched local APK extraction for:
  - `CA FE BA BE`;
  - wrapper descriptor prefix `10 18 1a 02 30 01 32 0a 10 0d`;
  - known action-list prefix from the wrapper candidate.
- Hits:
  - wrapper descriptor prefix exists in `classes.dex` and RC4 preimage heap as expected string/data material.
  - no useful APK-static `CAFEBABE` root template was found outside DEX/native noise.
- This supports the current interpretation that the `a05e(input)` template is produced or supplied at runtime, not stored as an obvious asset blob.

Current blocker, narrowed:
- The final full `root.9cd6()` StopCapture payload is no longer blocked by:
  - UCD2 checksum;
  - internal Packet header;
  - registry string writer;
  - string-id interning;
  - action-list shared registry id order.
- It is blocked by the exact byte array passed into `a05e(input)` for the actual StopCapture wrapper path.
- Without that input template or a device/app runtime capture of it, static APK-only reconstruction can still be off in:
  - root version/header fields;
  - preexisting registry ids;
  - wrapper `externalArg` id;
  - parsed context action insertion order.

Best next routes:
- Static route:
  - continue upward from the constructor/caller that creates the object containing `field@47dd/47de`;
  - identify the call site that supplies the concrete `input` bytes to `0x480f34`.
- Runtime route:
  - instrument or log the argument to `a05e(input)` / `a0e8(input, externalArg)` inside the official app process.
  - This would provide the missing template and allow deterministic `root.9cd6()` replay.

## Step 120 - Corrected seg08 coordinates and closed the a0de constructor caller

Coordinate correction:
- The wrapper/template code offsets `0x480f34`, `0x480f68`, and `0x480fcc` belong to `seg08.dex`.
- A check against `seg12.dex` at the same offsets produced unrelated code. Packet/UCD2 remains in `seg12`, while this serializer/wrapper path remains in `seg08`.
- Future notes must always include the DEX segment together with a code offset.

Disassembler correction:
- `dex_pretty_disasm.py` had the Dalvik instance/static field opcodes shifted after `iget-boolean`.
- In particular, opcode `0x5b` is `iput-object`, not `iput-byte`.
- Therefore `seg08 0x480f68` still stores two object references in `field@47de` and `field@47dd`; the earlier object-field interpretation was correct.

New caller evidence:
- `seg08 0x480f68` is the constructor reached as `method@a0de`.
- Its only visible construction site is `seg08 0x481064`: `new-instance type@356b`, `invoke-direct method@a0de`, then registration through interface `method@78e`.
- `seg08 0x480f34` is a six-input callback method on that object. It compares incoming `v2` with `field@47de`, then calls `a0e8(v5, field@47dd)` on a match.
- Therefore the byte/template input to `a05e` is the callback's direct `v5` argument, not a field stored by the constructor.

Interpretation:
- `seg08 0x481064` also normalizes a name-like object with character values `47` and `46`, performs class/helper lookup, and creates `type@356d` through `method@a0e2`.
- This strongly resembles a serializer/class-resolution extension path. It is infrastructure around object serialization, not yet proof of the concrete Luna StopCapture command template.
- The static boundary is now more precise: recover the caller/provider that invokes the `type@356b` callback and supplies its sixth register argument `v5`.

Next target:
- Identify the interface implemented by `type@356b` from `method@78e/795` registration behavior, or map the virtual callback at `0x480f34` to its method id.
- Then trace the callback invocation site that supplies `v5`; only after that can the exact `a05e(input)` root template be replayed without guessing.
## Step 121 - Resolved callback method id and StopCapture builder source

Class-data recovery from the RC4 preimage:
- The ULEB sequence around the class that owns `seg08 0x480f68` maps:
  - direct method `method@a0de` -> code `0x480f68`;
  - virtual method `method@a0df` -> code `0x480f34`.
- Therefore the six-input callback at `seg08 0x480f34` is `method@a0df`.

Interface dispatch evidence:
- Global scan of six-register `invoke-interface` calls found `method@a0d1` at StopCapture-adjacent code offsets including `seg08 0x47f5a0`.
- Register tracing at `seg08 0x47f5a0` shows the call shape:
  - `invoke-interface {v6, v7, v8, v9, v10, v11}, method@a0d1`;
  - `v11` is the current builder/root object from `result(method@9cb8)` and is later passed to `method@9d56`.
- The implementation `method@a0df` maps that sixth argument to callback register `v5` and calls `a0e8(v5, field@47dd)` after the key check.

Important correction:
- The byte array parsed by `a05e(input)` is not an unknown APK asset/template field.
- For the StopCapture path it is the live builder/root object produced in the same command method (`result(method@9cb8)`) and mutated before callback dispatch.
- This removes the previous runtime-template blocker and moves the remaining work to reconstructing the exact full `root.9cd6()` stream from the builder/root serializer and registry writers.

Wrapper constructor evidence:
- `seg08 0x481044 / a0e5` calls `a0e6(input, arg, string@6fa)`.
- `seg08 0x481064 / a0e6` constructs `type@356b` with `method@a0de`, registers it, and returns `type@356d`.
- Therefore the wrapper external argument registered by this convenience path is `string@6fa`, followed by the fixed wrapper descriptor `string@3865` when `a0e4` writes `9cb6(4233, externalArg, string@3865, 0, 0)`.

Root constructor evidence:
- `seg08 0x46d4f8` invokes `method@9cae` with default flags `0x00090000`, stores `field@43e5`, creates the registry/context object, and calls `method@9cd5`.
- This confirms the full packet must be generated as a complete `root.9cd6()` output, not just the previously generated action-list candidate.

Next target:
- Recover the exact registry byte stream for the involved string and compound records, especially the runtime value behind `string@6fa` and any damaged string-id entries.
- Then assemble and verify the full `CAFEBABE + version + registry + root/action arrays` payload before wrapping it in the internal Packet header and UCD2 frame.
## Step 122 - Full-root serializer boundaries and damaged string-id table confirmed

StopCapture builder path:
- `seg08 0x47f5a0` constructs the live command/root builder with `method@9cb8`, mutates it, then dispatches the six-register interface call `method@a0d1`.
- The sixth argument of that interface call is the same live builder/root object, later finalized through `method@9d56`.
- The callback implementation `method@a0df` receives that sixth argument as its `v5`, compares the key against `field@47de`, and calls `a0e8(v5, field@47dd)`.
- Therefore the input to `a05e` is the live StopCapture builder/root stream, not an APK asset blob.

Full-root transform:
- `seg08 0x480fcc / a0e8` calls `a05e(input)`, creates a `type@34ec` root from the parsed context, constructs wrapper `type@356c` through `a0e0(0x00090000, root, externalArg)`, inserts it into the parsed context with id `8`, then returns `root.9cd6()`.
- This confirms the final payload must be the complete `root.9cd6()` byte stream, not the earlier action-list-only candidate.

Version/parser facts:
- `seg08 0x47f2a8 / a05e` reads the serialized root version through `a060(input)`.
- If the version is `70`, it temporarily rewrites bytes `[6..7]` to version `69`, parses, then restores the original version through `a063`.
- `seg08 0x47f254 / a060` reads `((input[6] & 0xff) << 8) | (input[7] & 0xff)`.
- `seg08 0x47f3bc / a063` writes the same high/low version bytes back to `[6]` and `[7]`.
- Since `root.9cd6()` writes `u32_be 0xCAFEBABE` followed by `u32_be field@43fc`, the serialized root starts like `CA FE BA BE 00 00 vv vv` for these observed versions.

Root serializer boundary:
- `seg08 0x46c838 / 9cd6` writes:
  - `u32_be 0xCAFEBABE`;
  - `u32_be field@43fc`;
  - registry table through `field@43fa.9e13(out)`;
  - root counters/flags through multiple `u16_be` writes;
  - field/action node lists through `9d7f` and optional sections.
- `seg08 0x46d578` sets the root version/flags and registry-dependent fields:
  - `field@43fc = version`;
  - `field@43dc = flags`;
  - `field@43fb = registry.9e14(version & 0xffff, arg)`;
  - optional strings/objects/arrays are interned into the registry and stored in root fields.

Template mechanism proof:
- `seg08 0x480dd0 / a0da` is a separate template generator using version `53`, flags `4097`, and strings `b9f0/a51d/3865`.
- It proves the same root serialization mechanism, but it is not itself the Luna StopCapture root because the actual StopCapture path builds the root at `seg08 0x47f5a0` and transforms it through `a0e8`.

String-id damage status:
- `seg08` and the RC4 preimage share a stable base mapping for some intact regions:
  - `string@3865` descriptor at `seg08 0x30001`, preimage `0x334fabd`;
  - class-data sequence around `a0de/a0df` at `seg08 0x7a9c64`, preimage `0x3ac9720`.
- However the critical StopCapture registry string ids are damaged or point outside the recovered table:
  - `string@6fa`, `string@6fb`, `string@6fc`, `string@9be`, `string@224e`, `string@259e`, `string@8b21`, `string@98e9` cannot yet be trusted from `seg08` string_ids directly.
- `seg07.dex` does contain intact StopCapture strings/descriptors, including:
  - `Linsta360/messages/StopCapture$Builder;`
  - `Linsta360/messages/StopCapture$Companion$ADAPTER$1;`
  - `Linsta360/messages/StopCapture;`
  - `type.googleapis.com/insta360.messages.StopCapture`.

Next target:
- Use `seg07` intact message descriptors plus `seg08` call-shape evidence to recover the exact registry strings and descriptor bytes used by the StopCapture root.
- After those are mapped, assemble the full `CAFEBABE + registry + root/action arrays` payload, then wrap it with the internal Packet header and UCD2 frame.
## Step 123 - Recovered class_data mappings for root/action chain methods

Class-data parsing by code_off ULEB:
- The method_ids table in `seg08.dex` is damaged, but class_data entries around `0x7a7ce6` and `0x7a7bc2` are parseable enough to recover method -> code offsets.
- Reliable mapping for the action-node serializer class at class_data `0x7a7ce6`:
  - `method@9d72` -> `seg08 0x471344` constructor-like initializer.
  - `method@9d7f` -> `seg08 0x471b8c` node serializer.
  - `method@9d90` -> `seg08 0x4728a4`.
  - `method@9d91` -> `seg08 0x472944`.
  - `method@9d93` -> `seg08 0x472c70`.
  - `method@9d99` -> `seg08 0x472fbc`.
  - `method@9d9a` -> `seg08 0x473088`.
- Reliable mapping for a forwarding chain class around class_data `0x7a7bc2/0x7a7bf5`:
  - `method@9d55` -> `seg08 0x47056c`.
  - `method@9d56` -> `seg08 0x47058c`.
  - `method@9d57` -> `seg08 0x4705ac`.
  - `method@9d58` -> `seg08 0x4705cc`.
  - `method@9d5a` -> `seg08 0x470618`.
  - `method@9d5e` -> `seg08 0x470694`.
  - `method@9d5f` -> `seg08 0x4706b4`.
  - `method@9d65` -> `seg08 0x4707cc`.
  - `method@9d67` -> `seg08 0x470834`.
  - `method@9d70` -> `seg08 0x47095c`.

Important correction:
- The `9d55/9d57/9d58/9d5a/9d5e/9d5f/9d65` implementations at `0x4705xx..0x4707xx` are not final writers.
- They forward to `field@44f5`, which is the next action node in the chain.
- Therefore the StopCapture call sequence mutates a linked action-node chain; the final bytes are emitted later by `method@9d7f` at `seg08 0x471b8c` when `root.9cd6()` serializes `field@43e3`.

Root delegate mapping:
- `method@9cb6` at `seg08 0x46c45c` and `method@9cb8` at `seg08 0x46c48c` are also delegate forwarders through `field@43d9`.
- `field@43d9` is set by the root/context constructor path `seg08 0x46c560`, which stores the constructor argument object into `field@43d9` after validating the root version/flags.
- Several public root APIs at `0x46c3d8..0x46c760` forward into this delegate.

Current implication:
- The final packet generator must model three layers separately:
  1. root/context delegate creation via `field@43d9`;
  2. command/action node mutation through the forwarding chain (`field@44f5`);
  3. final serialization through `root.9cd6()` and action-node `9d7f`.
- Previous candidate code that treated `9d57/9d5a/9d5e` as direct writes is still useful as byte-level evidence, but it must now be revalidated against the linked-node serialization path.

Next target:
- Identify the concrete delegate object assigned to `field@43d9` in the Luna StopCapture path.
- From that delegate, recover the actual implementation of `9cb8(4106, string@6fc, string@9be, 0, 0)` that creates the first action node returned as `v6` at `seg08 0x47f5a0`.
## Step 124 - Mapped StopCapture mutators to actual type@34fa writer implementations

Action node creation:
- `seg08 0x46d414` creates `type@34fa`, calls `method@9d72`, appends it to root `field@43e3`, stores tail in `field@43ea`, and returns the new node.
- Parameters passed into `9d72` are:
  - root registry `field@43fa`;
  - command id;
  - first string/object;
  - second string/object;
  - optional extra object;
  - optional array;
  - root mode `field@43dd`.
- For StopCapture at `seg08 0x47f5a0`, this corresponds to:
  - command id `4106 / 0x100a`;
  - `string@6fc`;
  - `string@9be`;
  - optional args `0, 0`.

Why the visible `9d55/9d57/...` methods looked like no-ops:
- The methods at `seg08 0x4705xx` are forwarding stubs through `field@44f5`.
- The concrete `type@34fa` virtual implementations use different method ids in the damaged/reconstructed DEX, but they match by parameter shape and byte-writing behavior.

Mapped StopCapture mutators:
- Parent call `9d57(int, object, string, string)` maps to child writer `seg08 0x472244`:
  - stores current buffer offset in `field@450f`;
  - calls registry helper `9de0(object, string, string)`;
  - writes `9c6b(marker, registry_id)` into `field@44fe`.
  - For StopCapture this is `9d57(178, field@47a4, string@6fb, string@98e9)` and produces the first payload/extension registry reference record.
- Parent call `9d5a(int)` maps to child writer `seg08 0x47279c`:
  - writes a single byte opcode through `9c6d(value)`;
  - updates size/position state.
  - StopCapture uses this for opcodes `89`, `87`, and `176`.
- Parent call `9d5e(int, type@34f7)` maps to child writer `seg08 0x472944`:
  - stores current buffer offset;
  - handles selectors around `167/168/198/199/200/201`;
  - writes selector bytes into `field@44fe`;
  - links or creates `type@34f7` subnodes through `9d92`/`9d73`.
  - StopCapture uses this with selector `199 / 0x00c7` and a fresh `type@34f7`.
- Parent call `9d5f(type@34f7)` maps to child linker `seg08 0x472b2c`:
  - merges or appends the supplied subnode into the active action-node structure.
- Parent call `9d65(int, int)` maps to child numeric writer `seg08 0x47338c`:
  - writes compact or extended numeric forms using `9c6d`, `9c69`, or `9c6b` depending on marker/value range.
  - StopCapture calls this with the transformed result of `invoke-static {result, 2}, method@684` and `0`.
- The likely child implementation for `9d58(int, int, object, int, object)` is `seg08 0x47232c` (`ins=6`), still needs detailed field-by-field confirmation because it is the mandatory extension branch after the Luna callback.

Updated StopCapture node sequence from APK evidence:
1. Create action node with command `0x100a`, registry args `string@6fc`, `string@9be`.
2. Initialize/flush chain through `9d55`.
3. Write registry reference record through `9d57(0x00b2, field@47a4, string@6fb, string@98e9)`.
4. Write opcode `0x59` through `9d5a(89)`.
5. Create fresh `type@34f7` and apply selector `0x00c7` through `9d5e(199, subnode)`.
6. Write opcode `0x57` through `9d5a(87)`.
7. Invoke Luna callback `a0d1`; callback wraps the live builder/root through `a0e8` and returns a status/result.
8. Mandatory extension branch calls `9d58(-1, 0, field@47a0, 1, field@47a1)`.
9. Link the earlier subnode through `9d5f(subnode)`.
10. Write opcode `0xb0` through `9d5a(176)`.
11. Write numeric status through `9d65(method@684(result,2), 0)`.
12. Finalize through `9d56`; final root serialization still happens through `root.9cd6()`.

Next target:
- Disassemble and model `seg08 0x47232c` for the mandatory `9d58` branch.
- Then replay the `type@34fa` state machine in Python to produce the full action node bytes that `9d7f` will serialize under `root.9cd6()`.
## Step 125 - Cropped Luna mandatory `9d58` branch and static array arguments

Static array arguments:
- `seg08 0x47f474` initializes the two static fields used by the Luna StopCapture `9d58` call:
  - `field@47a0` = empty `String[]` / `type@4435` array.
  - `field@47a1` = one-element `String[]` containing `string@98e9`.
- Therefore the actual StopCapture call at `seg08 0x47f5a0` is:
  - `9d58(-1, 0, [], 1, [string@98e9])`.

Path constraint in `seg08 0x47232c`:
- The concrete child implementation for this call is the `ins=6` method at `seg08 0x47232c`.
- With `v12 = -1`, the generic branch would throw unless `field@44ff == 3`.
- Therefore the Luna StopCapture action node must run in mode `3` for this `-1` extension branch to be legal.
- This is important for packet replay: `field@44ff` cannot be modeled as zero/default for this command.

Cropped mode-3 branch:
- When `field@44ff == 3`, the method first uses `field@4500` and its `field@44e3` helper/writer.
- If the helper is absent, it creates/attaches `type@34f0`, calls `9d22(registry, field@44fd, field@4504, 0)`, then `9d0e(this)`.
- If the helper is present and `v12 == -1`, it calls:
  - `helper.9d21(registry, 0, [], 1, [string@98e9])`;
  - then `helper.9d0e(this)`.
- After either path it falls through to the shared tail around `0x47232c + 0x178`:
  - because `field@44ff == 3`, it skips the `field@44ff == 2` size-recount branch;
  - updates `field@4521 = max(field@4521, 1)`;
  - updates `field@451f = max(field@451f, field@4502)`.

`9d7e(object)` helper used by the extension branch:
- `seg08 0x471b0c` writes typed values into `field@452d`:
  - one primitive/small type path writes the value directly with `9c6d(value)`;
  - string/object path writes marker `0x07`, interns the object through registry `9ddb`, then writes the registry id with `9c71(id)`;
  - nested `type@34f7` path writes marker `0x08` and then serializes that node into the same extension buffer.
- For Luna's `[string@98e9]`, the expected typed-object encoding is therefore `07 + u16_be(registry_id(string@98e9))`, subject to the exact helper path (`9d21`/`9d22`) still being fully modeled.

State dependency found:
- `type@34fa.9d72` creates an initial `type@34f7` and calls `9d92` during construction.
- In mode `3`, `9d92` can place that node into `field@4500` without necessarily materializing `field@4500.field@44e3` immediately.
- Later calls such as `9d5e(199, subnode)` and `9d58(-1, ...)` depend on whether `field@4500.field@44e3` already exists.
- This is the next exact state to resolve before the full node replay can be trusted.

Next target:
- Decode the `type@34f7` / `type@34f4` helper chain around:
  - `method@9d39` constructor;
  - `method@9d0d` helper wrapper;
  - `method@9d11`, `9d21`, `9d22`, `9d0e` writer/helper methods.
- This should determine whether the Luna `9d58` extension appends `07 00 <id>` into `field@452d`, or whether it first creates a helper node and writes through `field@44e3`.
## Step 126 - Helper chain for `9d58(-1, 0, [], 1, [string@98e9])`

Recovered helper methods:
- Class-data scan around `0x7a788e` recovered a helper/writer group:
  - `method@9d0d` constructor-like wrapper -> `seg08 0x46e8c8`.
  - `method@9d0e` / helper-to-action synchronization -> `seg08 0x46e8e4`.
  - `method@9d11` generic writer -> `seg08 0x46ea9c`.
  - `method@9d21` array/object mapper -> `seg08 0x46f730` in the most relevant helper class.
  - `method@9d22` related initializer/mapper -> `seg08 0x46f830` in the most relevant helper class.
- `seg08 0x46e8c8` stores the wrapped `type@34f7` into `field@44c4`.

`9d21` behavior for Luna's exact arguments:
- The mode-3 `9d58` branch calls `helper.9d21(registry, 0, [], 1, [string@98e9])` when `field@4500.field@44e3` already exists.
- `seg08 0x46f730` maps the first object array into `field@44be` and the second object array into `field@44bf`.
- For every object, it calls `method@9d12(registry, object)` to obtain the encoded int value.
- It adds a sentinel `0x00400000` after certain special objects (`field@45f8` / `field@4581`), but `string@98e9` does not yet appear to be one of those special sentinel objects.
- Therefore, for the Luna arguments, the likely helper state is:
  - first array/count side empty or padded;
  - second array side contains the encoded registry value for `string@98e9`.

`9d0e` synchronization behavior:
- `seg08 0x46e8e4` reads helper arrays `field@44be` and `field@44bf`.
- It computes group counts and asks the action node for insertion space through `9d8b(base, first_count, second_count)`.
- It then inserts every encoded int through action node `9d81(index, encoded_value)`.
- Finally it calls action node `9d8a()`.
- This means the Luna `9d58` branch changes the action node's internal index/metadata arrays, not just a simple byte append to `field@44fe`.

`9d7e(object)` clarification:
- `seg08 0x471b0c` can write typed objects into `field@452d` as:
  - primitive/small object -> raw byte through `9c6d`;
  - registry object/string -> marker `0x07` plus `u16_be(registry_id)`;
  - nested `type@34f7` -> marker `0x08` plus nested node serialization.
- This helper is used by some `9d58` branches, but the mode-3 Luna path primarily routes through `9d21` + `9d0e`, so modeling only `07 + id` is insufficient.

Current replay requirement:
- The final StopCapture root generator must model at least these action-node state fields:
  - `field@44fe` main byte writer;
  - `field@452d` extension byte writer, if materialized;
  - `field@4500/450e` active node chain;
  - metadata arrays controlled through `9d8b`, `9d81`, `9d8a`;
  - mode `field@44ff = 3`.
- The earlier action-list candidate only modeled `field@44fe`, so it cannot be considered final.

Next target:
- Decode the action-node array operations:
  - `9d8b` at the child implementation used by `seg08 0x46e8e4`;
  - `9d81` insertion;
  - `9d8a` finalization.
- Once those are modeled, the `9d7f` serializer at `seg08 0x471b8c` can be replayed more faithfully.
## Step 127 - `9d13` string/object encoding and `9d8a` metadata staging

Action-node metadata array operations recovered before this step:
- `seg08 0x470d0c / 9d8b(base, first_count, second_count)` allocates or ensures `field@4501` length is `first_count + second_count + 3`, stores:
  - `field@4501[0] = base`;
  - `field@4501[1] = first_count`;
  - `field@4501[2] = second_count`;
  - returns insertion index `3`.
- `seg08 0x4721a8 / 9d81(index, encoded)` writes `field@4501[index] = encoded`.
- `seg08 0x4726a8 / 9d8a()` finalizes the staged metadata array:
  - if an older `field@4527` exists, it ensures `field@452d`, calls private `9d7d()` to flush that older array into extension bytes, then increments `field@452e`;
  - for the first metadata group, it simply moves current `field@4501` into `field@4527` and clears `field@4501`.
- Therefore the first Luna `9d58(-1, 0, [], 1, [string@98e9])` most likely stages `field@4527 = [base, 0, 1, encoded(string@98e9)]` and does not immediately append raw extension bytes, unless a previous metadata group already exists.

`9d12` / `9d13` object encoding:
- `seg08 0x46e434 / 9d12(registry, object)` dispatches by object kind:
  - primitive/small `type@33f` -> returns `value | 0x00400000`;
  - string-like `type@35c` -> converts through `9e2e(obj).9e24()`, then calls `9d13(registry, converted_string_like, 0)`;
  - nested `type@34f7` -> returns a registry/node encoded int with high flags such as `0x00c00000` or `0x01000000` depending on state.
- `seg08 0x476820 / 9e2e(string)` constructs a `type@3504` slice/range object:
  - if the first character is `[` it tags the object with kind `9`;
  - otherwise it tags it with kind `12` and range `0..len`.
- `seg08 0x476588 / 9e24()` returns the backing substring for a `type@3504` slice; kind `12` also normalizes via `method@9e3d`.
- `seg08 0x46e4c0 / 9d13(registry, string_like, start)` maps descriptor-like characters to encoded ints:
  - base primitive descriptors return constants in the `0x00400000` range;
  - object descriptors beginning with `L...` use the substring after `L` and call registry `method@9dfa`, then OR the result with `0x00800000`;
  - array descriptors account for nesting depth by shifting the depth into the upper bits and OR-ing the element encoding.
- For Luna's plain `string@98e9`, because it is passed as a Java/String-like object and not known to start with `[` or a primitive descriptor, the expected path is the kind-12 plain/object-name path through `9dfa`, yielding `0x00800000 | registry_id_or_index` (exact `9dfa` id rule still pending).

Registry lookup state:
- Direct safe class-data lookup for `method@9dfa`, `9df4`, and `9d13` returned no hits, so this class area is damaged or not parseable by normal tables.
- Invoke scanning still shows `9dfa` callers at `seg08 0x46e1a0`, `0x46e4c0`, `0x46e650`, `0x46e660`, `0x46e70c`, `0x46ea9c`, `0x46f830`, `0x4742d0`.
- `9df4` remains the main string registry/id writer used heavily by root serialization and action serialization, but its method body has not yet been directly mapped.

Next target:
- Find the implementation or equivalent behavior of `method@9dfa` / `method@9df4`, because `encoded(string@98e9)` inside the Luna `9d58` metadata group depends on the registry id/index it returns.
- Then update `reverse_apk/tools/luna_packet_candidate.py` to emit a full-root `CA FE BA BE ...` candidate instead of the older action-list-only byte sequence.
## Step 128 - Candidate generator updated for `9d58` metadata staging

Code update:
- Updated `reverse_apk/tools/luna_packet_candidate.py` under `F:\Insta360onWin` only.
- Added `encode_9d58_metadata_group(base_node_id, encoded_second_arg)`:
  - models the APK-observed staged array `[base, first_count, second_count, encoded...]` from `9d0e -> 9d8b/9d81/9d8a`;
  - for Luna's mandatory call this is `[base, 0, 1, encoded(string@98e9)]`.
- Added `encoded_plain_string_ref(registry_id)`:
  - models `seg08 0x46e4c0 / 9d13`, where plain/object string-like values use `registry.9dfa(value) | 0x00800000`.
- Added a printed candidate named `stop_capture_apk_wrapped_sequence_a03f_9d58_metadata_candidate`.

Verification:
- Ran `python -B reverse_apk/tools/luna_packet_candidate.py` successfully.
- Known device-info frame still matches the previously observed working frame:
  - `55 43 44 32 01 0c 04 10 0f 00 00 00 08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b 7c 00 8e 7c`
- New staged metadata bytes currently emitted by the candidate generator:
  - `00 00 00 00 00 00 00 00 00 00 00 01 00 80 00 05`
- New UCD2 diagnostic candidate emitted by the generator:
  - `55 43 44 32 01 0c 04 10 72 00 00 00 08 00 02 01 00 00 80 00 00 00 03 10 89 00 01 00 02 00 01 00 03 00 00 00 00 10 8a 00 04 00 05 00 01 00 03 00 00 00 00 10 00 00 06 00 07 00 03 00 09 00 00 00 31 00 02 00 00 00 00 00 0f b2 00 08 59 c7 00 00 00 09 57 59 b3 00 08 b0 00 01 00 0a 00 00 00 12 00 01 00 00 00 00 00 00 00 00 00 00 00 01 00 80 00 05 00 0b 00 00 00 00 00 0c 00 00 00 00 ad fd 66 07`

Important limitation:
- This is still a diagnostic candidate, not the final full-root packet.
- The `base_node_id` is currently set to `0` because the exact `field@44e0` value for the empty `type@34f7` node has not yet been replayed through the full action-node state machine.
- The `registry_id(string@98e9)=5` assumption follows the wrapped-sequence registry order already documented in the generator comments; this must be validated against the real full-root registry stream.

Next target:
- Close the remaining `field@44e0` / `base_node_id` dependency by replaying the empty `type@34f7` node path around `9d5e(199, subnode)` and `9d5f(subnode)`.
- Then replace the action-list diagnostic with a real `CA FE BA BE ... root.9cd6()` generator.
## Step 129 - `9d43/9d44` patch origin and `base_node_id=0` evidence

Code update:
- Updated `reverse_apk/tools/luna_packet_candidate.py` under `F:\Insta360onWin` only.
- Corrected the `9d43` / `9d44` length-backfill model for the StopCapture inner `c7` branch:
  - `seg08 0x470194 / 9d43(writer, offset, wide)` registers a patch at the placeholder position, but stores the original opcode offset passed by the caller.
  - `seg08 0x46fd20 / 9d44(main_bytes, ext_writer, current_len)` backfills `current_len - original_offset`.
  - For `9d5e(199, subnode)` at `seg08 0x472944`, the opcode `c7` is written at offset `4`, and the four-byte placeholder starts at offset `5`.
  - Therefore the a03f diagnostic inner body must contain `c7 00 00 00 0a`, not `c7 00 00 00 09`.

`base_node_id` evidence:
- Rechecked `seg08 0x46ff2c / 9d39`: the `type@34f7` constructor only calls the superclass constructor and returns. It does not initialize `field@44e0`.
- Rechecked the action-node constructor path `seg08 0x471344 / 9d72`:
  - in nonzero/mode-3 setup it creates a fresh `type@34f7`;
  - then calls `9d92(initial_node)`;
  - `9d92` is the same body as `seg08 0x472b2c`, and when `field@4500 == null`, it stores the passed node into `field@4500`.
- Rechecked `seg08 0x472944 / 9d5e(199, subnode)`:
  - it writes `c7`, calls `subnode.9d43(...)`, and then enters the mode-3 branch;
  - because `field@4500` already exists, it calls `field@4500.field@44e3.9d11(199, 0, null, null)`;
  - it does not replace `field@4500.field@44e3.field@44c4` with the new subnode at this point.
- Decoded the packed-switch table inside `seg08 0x46ea9c / 9d11`:
  - key `199` routes to unit `0x020f`;
  - that branch calls `9d1c(0x03c00000)` and returns;
  - no `field@44c4` store occurs on this route.
- Rechecked `seg08 0x46e8e4 / 9d0e`: it reads `helper.field@44c4.field@44e0` directly and passes that value as `base` to `9d8b(base, first_count, second_count)`.
- Current conclusion: at the Luna `9d58(-1, 0, [], 1, [string@98e9])` point, `helper.field@44c4` still points at the constructor-installed empty `type@34f7`, whose `field@44e0` is still default `0`. The generator comment was updated accordingly, and `base_node_id=0` is no longer just a placeholder.

Verification:
- Ran `python -B reverse_apk/tools/luna_packet_candidate.py | Select-String -Pattern 'known_device_info_frame|a03f_candidate_body|apk_wrapped_sequence_a03f_9d58_metadata|metadata_9d58'`.
- Known device-info frame still matches the observed working frame:
  - `55 43 44 32 01 0c 04 10 0f 00 00 00 08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b 7c 00 8e 7c`
- Corrected inner body:
  - `b2 00 03 59 c7 00 00 00 0a 57 59 b3 00 03 b0`
- Current staged metadata bytes:
  - `00 00 00 00 00 00 00 00 00 00 00 01 00 80 00 05`
- Current UCD2 diagnostic candidate with corrected `c7` patch:
  - `55 43 44 32 01 0c 04 10 72 00 00 00 08 00 02 01 00 00 80 00 00 00 03 10 89 00 01 00 02 00 01 00 03 00 00 00 00 10 8a 00 04 00 05 00 01 00 03 00 00 00 00 10 00 00 06 00 07 00 03 00 09 00 00 00 31 00 02 00 00 00 00 00 0f b2 00 08 59 c7 00 00 00 0a 57 59 b3 00 08 b0 00 01 00 0a 00 00 00 12 00 01 00 00 00 00 00 00 00 00 00 00 00 01 00 80 00 05 00 0b 00 00 00 00 00 0c 00 00 00 00 a8 0a 09 96`

Important limitation:
- This still is not the final StopCapture packet. It is a diagnostic action/wrapper candidate.
- Remaining finalization work is now concentrated on replaying the full `root.9cd6()` stream:
  - `CA FE BA BE 00 00 vv vv` header;
  - exact registry ordering and `9dfa/9df4` ids;
  - action-node serializer `seg08 0x471b8c / 9d7f`;
  - root parser/wrapper path `a0e8 -> a05e -> root.9cd6()`.

Next target:
- Build a full-root generator around the corrected action-node body and metadata group, then compare its registry stream against observed `a0e8` / `root.9cd6()` behavior.
## Step 130 - `root.9cd6()` write order and `a0da` template route rechecked

APK-only recovery continued from the full-root side.

`root.9cd6()` write order:
- Rechecked `seg08 0x46c838 / 9cd6`.
- After size calculation and registry finalization, it writes:
  - `u32_be 0xCAFEBABE`;
  - `u32_be field@43fc`;
  - `field@43fa.9e13(out)` registry envelope;
  - `u16_be(masked field@43dc)`;
  - `u16_be field@43fb`;
  - `u16_be field@43f9`;
  - `u16_be field@43e7`, then each `field@43e8[i]` as `u16_be`;
  - `u16_be field@43e2_count`, then each field node via `9d08(out)`;
  - `u16_be field@43e3_count`, then each action node via `9d7f(out)`;
  - `u16_be extra_section_count`, then optional root sections.
- It accumulates action flags through `9d7b()` and `9d7a()` while serializing `field@43e3`.
- If the accumulated action flag value is nonzero, it returns `9cd4(bytes, flags)` instead of the raw writer buffer.

`9d7f` action-node serializer confirmation:
- Rechecked `seg08 0x471b8c / 9d7f`.
- It begins every node with:
  - `u16_be(masked field@44fd)`;
  - `u16_be field@4523`;
  - `u16_be field@4505`;
  - `u16_be section_count`.
- For nodes with a main `field@44fe` buffer, the main section writes:
  - registry id for `string@224e`;
  - `u32_be(main_record_len)`;
  - `u16_be field@4521`;
  - `u16_be field@451f`;
  - `u32_be field@44fe.length`;
  - raw `field@44fe` bytes.
- If `field@452d` exists, the extension section writes:
  - registry id for `string@8a51` on version >= 50, otherwise `string@8a4f`;
  - `u32_be(field@452d.length + 2)`;
  - `u16_be field@452e`;
  - raw `field@452d` bytes.
- This supports the current generator shape for main/extension sections, but the first staged metadata group in `field@4527` must still be serialized by the correct `9d7d`/related path when it is flushed into `field@452d`.

Root construction facts:
- Rechecked `seg08 0x46d4f8 / 9cc2(context, flags)`:
  - calls base initializer with `0x00090000`;
  - stores flags in `field@43e5`;
  - if context is null, creates a fresh `type@3503` registry/context;
  - if context is non-null, creates a registry/context from the parsed input;
  - calls `9cd5(flags)`.
- Rechecked `seg08 0x46d540 / 9cd5(flags)`:
  - if `(flags & 2) != 0`, sets `field@43dd = 4`;
  - else if `(flags & 1) != 0`, sets `field@43dd = 1`;
  - else sets `field@43dd = 0`.
- Therefore `a0e8(input, externalArg)`, which calls `9cc2(parsedContext, 0)`, statically uses root mode `field@43dd = 0`.

Template route:
- Rechecked `seg08 0x480dd0 / a0da(arg)`:
  - creates a fresh root with `9cc1(0)`;
  - computes `arg.6ee(46, 47)`;
  - calls `9cd7(53, 4097, computed, 0, string@b9f0, null)`;
  - calls `9cdb(9, string@a51d, string@3865, 0, 0)`;
  - calls `9cda()`;
  - returns `root.9cd6()`.
- Rechecked `seg08 0x480ee0 / a0dd`:
  - calls `a0da(field@47db)`;
  - attaches the returned byte array through `a0d6(...).5d6(string@a51d).7d3(0, arg)`.
- Rechecked `seg08 0x480f34`, the only observed static caller of `a0e8`:
  - compares an incoming object against `field@47de`;
  - if it matches, calls `a0e8(v5, field@47dd)`;
  - the `input` byte array is the caller-provided `v5`, not a constant directly embedded at this call site.
- Rechecked `seg08 0x480f68`:
  - constructor stores `field@47de` and `field@47dd` from external constructor arguments.

Current conclusion:
- APK static code gives a reproducible template generator (`a0da`) and a wrapper transformer (`a0e8`), but the one direct `a0e8` call site receives its input byte array from above the visible method boundary.
- The final full `root.9cd6()` StopCapture packet cannot be honestly claimed from the diagnostic action candidate alone.
- The next best APK-only path is to implement `a0da`'s template generator in Python, then feed that template into a modeled `a0e8` wrapper insertion and compare the resulting registry/action order against the already recovered `9d7f` layout.

Next target:
- Add Python helpers for:
  - registry string records (`9df4` / `9c72`);
  - minimal `root.9cd6()` envelope;
  - `a0da` template root;
  - modeled `a0e8` wrapper insertion using the corrected StopCapture action-node serializer.
## Step 131 - Added structural `root.9cd6()` generator

Code update:
- Updated `reverse_apk/tools/luna_packet_candidate.py` under `F:\Insta360onWin` only.
- Added `ApkRegistry`, a minimal model of the APK registry string interner:
  - starts ids at `1`, matching `seg08 0x475288`;
  - `intern_string(value)` writes the proven `9df4` string record format:
    - marker `01`;
    - `9c72` modified UTF-8 length and bytes;
  - `envelope()` writes the proven `9e13(out)` format:
    - `u16_be(registry_count)`;
    - raw registry record bytes.
- Added `root9cd6_minimal(...)` and `root9cd6_with_action_blob(...)`:
  - write `u32_be 0xCAFEBABE`;
  - write `u32_be version`;
  - write the registry envelope;
  - write the root header/counter area recovered from `seg08 0x46c838`;
  - write field-node count `0`;
  - write action-node count plus raw concatenated `9d7f` action-node bytes;
  - write optional-section count `0`.
- Added `root_structural_wrapped_stop_capture_candidate(action_sequence)`:
  - consumes the existing wrapped StopCapture action sequence candidate;
  - lifts it into a `CA FE BA BE ... root.9cd6()` structural candidate.

Important naming note:
- The new output is named `root9cd6_structural_*` on purpose.
- It is a structural root candidate, not final bytes, because several damaged `seg08` string ids are still represented as placeholders:
  - `external_arg`;
  - `string@3865`;
  - `string@259e`;
  - `string@6fb`;
  - `string@98e9`;
  - `string@6fc`;
  - `string@9be`;
  - `string@224e`;
  - `string@8a51`;
  - `string@8b21`.
- These placeholders keep registry order and root layout auditable without falsely claiming the final APK string bytes are known.

Verification:
- Ran `python -B -m py_compile reverse_apk/tools/luna_packet_candidate.py` successfully.
- Ran:
  - `python -B reverse_apk/tools/luna_packet_candidate.py | Select-String -Pattern 'known_device_info_frame|a03f_candidate_body|metadata_9d58|root9cd6_structural_body|root9cd6_structural_internal|root9cd6_structural_ucd2'`
- Known device-info frame remains unchanged:
  - `55 43 44 32 01 0c 04 10 0f 00 00 00 08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b 7c 00 8e 7c`
- Corrected StopCapture inner body remains:
  - `b2 00 03 59 c7 00 00 00 0a 57 59 b3 00 03 b0`
- Current structural root body starts with the expected root header:
  - `ca fe ba be 00 00 00 35 ...`
- Current structural root UCD2 frame starts:
  - `55 43 44 32 01 0c 04 10 12 01 00 00 08 00 02 01 00 00 80 00 00 ca fe ba be 00 00 00 35 ...`

Current limitation:
- This is the first generated `CA FE BA BE` root-level candidate in the tool, but not the final Packet.
- Remaining APK-only closure items:
  - map damaged `seg08` string ids used by `9d7f` and StopCapture to exact string/byte values;
  - recover/validate `field@43fb = registry.9e14(version & 0xffff, arg)` for the actual template/context;
  - model `a0da` template output and `a0e8` insertion against that template rather than using placeholder strings;
  - decide whether `9cd4(bytes, flags)` is triggered for the final StopCapture wrapper output.

Next target:
- Recover exact string values for the `9d7f` section labels and StopCapture descriptors from `seg07`/RC4 preimage evidence, then replace placeholder registry strings in the structural root candidate with APK-derived bytes.

## Step 132 - Added a practical media-library workflow based on `diamondfsd/luna-ai-cut`

Reference boundary:
- The user explicitly requested using `diamondfsd/luna-ai-cut` as a reference for new features.
- Cloned the public MIT-licensed repository into the F-drive-only reference path:
  - `reverse_apk/references/luna-ai-cut`
- Referenced commit:
  - `79374f2985fa3c2bb77fa8e29ae4c7d999877990`
  - commit subject: `fix: keep landing changelog in sync`
- The imported reference is kept separate from the application source. No Electron/React runtime or copied source module was added to this Rust/Wry application.

Feature selection:
- Adopted the media-library interaction model that is useful with the current proven Luna HTTP listing path:
  - group camera media by date;
  - filter all/photo/video;
  - newest-first or oldest-first sorting;
  - comfortable/medium/compact card density;
  - visible-item select-all and clear selection;
  - multi-file download;
  - image/video preview overlay with file metadata.
- Did not alter the APK-only UCD2 camera-control reverse-engineering path. Capture controls remain gated by recovered Packet/EncryptionManager evidence.

Rust bridge changes:
- Updated `src/bin/html_app.rs`:
  - added the `download_batch` IPC command;
  - added `BatchDownloadPayload` and `BatchDownloadItem`;
  - added Windows-safe output component handling;
  - stores batch downloads under `downloads/<camera-date>/<filename>`;
  - returns completed and failed item arrays instead of aborting the whole batch after one failure;
  - single and batch downloads now reuse `AppState.luna_session`.
- Updated `src/adapters/luna_local.rs`:
  - split the HTTP transfer into `resume_download_authenticated`;
  - added `resume_download_with_session` for an existing persistent Luna session;
  - first download uses the authentication already performed by `LunaAuthSession::open`;
  - an existing session is refreshed once before a batch, not once per media file.

HTML UI changes:
- Updated `web/index.html` with a responsive media-library surface:
  - no fixed horizontal width is introduced; grids use `auto-fill` plus bounded `minmax` tracks;
  - all controls and visible labels are Chinese;
  - the existing album list now renders date sections and media cards;
  - selection state survives filter/sort/density changes;
  - batch-download failures remain selected for retry;
  - clicking a card opens an edge-to-edge dark preview and fills the existing single-download URL field;
  - Escape, backdrop click, and the close button dismiss preview.

Verification:
- `cargo check --bin html_app` passed.
- `cargo fmt --all -- --check` passed.
- Parsed the complete inline script from `web/index.html` with Node `new Function(...)`; JavaScript syntax passed.
- Browser-plugin visual automation could not be started because the required browser JavaScript control tool was not exposed in this session. Static responsive constraints and JavaScript syntax were verified; a real camera-populated visual pass remains useful.

Known limits:
- Preview uses the camera media URL directly. Large source images/videos may take time to open because this version does not yet generate a local thumbnail cache.
- Batch transfer reports an aggregate result when the command returns; it does not yet stream per-file progress events into the UI.
- The public reference improves media management only. It is not evidence for Luna UCD2 camera-control packets and must not be used to close the APK-only Packet reverse-engineering claims.

Next target:
- Build the updated release executable and test the populated media library against Luna Ultra.
- Continue APK-only Packet work from Step 131 after the UI handoff is verified.

## Step 133 - Built and smoke-tested the updated Windows application

Build:
- Ran `cargo build --release --bin html_app` successfully.
- Updated executable:
  - `F:/Insta360onWin/target/release/html_app.exe`
- The build only emitted the pre-existing dead-code warnings for unused OSC/probe paths.

Smoke test:
- Started the release executable from `F:/Insta360onWin` with a hidden window.
- The process remained alive after three seconds, confirming that the Rust host and embedded WebView initialized without an immediate startup crash.
- Stopped only that smoke-test process after the check.

Encoding:
- Normalized all files touched in this step to UTF-8 without BOM and CRLF line endings:
  - `src/bin/html_app.rs`
  - `src/adapters/luna_local.rs`
  - `web/index.html`
  - `reverse_apk/REVERSE_CONTINUE_LOG.md`

Next real-device check:
- Run `target/release/html_app.exe` while connected to Luna Ultra Wi-Fi.
- Open the Luna Ultra page and click `读取相册列表`.
- Confirm date groups, filters, selection, preview, and a two-file batch download.
- Batch files should appear under `F:/Insta360onWin/downloads/<camera-date>/` when launched through the project run script/current working directory.

## Step 134 - Replaced the reverse-engineering console with a daily-use product UI

User direction:
- The user rejected the exposed debug/reverse-engineering controls and requested a daily-use application.

Production surface cleanup:
- Rebuilt `web/index.html` as a formatted, responsive Chinese product UI.
- Removed every visible reverse-engineering/debug surface:
  - raw UCD2 packet input;
  - auth/heartbeat/negotiation probes;
  - StopCapture candidate buttons;
  - Packet/EncryptionManager evidence;
  - APK evidence/profile page;
  - raw GATT characteristic inspection;
  - hexadecimal BLE writes;
  - JSON debug output panels.
- Searched the final HTML for `UCD2`, `Packet`, `GATT`, `hex`, `候选`, `逆向`, `调试`, `profiles`, and `ucd2`; no matches remain.
- Disabled WebView developer tools in `src/bin/html_app.rs` with `with_devtools(false)`.

Daily-use information architecture:
- `相机媒体`:
  - one clear connect/disconnect flow;
  - automatic media loading after successful detection;
  - date grouping, photo/video filters, sort order, three density levels;
  - selection, batch download, output-folder selection, and full preview.
- `水印导出`:
  - native input-file picker;
  - native output-file picker;
  - Chinese names for every watermark style;
  - position, size, and opacity controls;
  - a compact visual placement preview.
- `Mic Pro`:
  - one scan action;
  - friendly device cards showing name and address;
  - no developer-only BLE controls.

Native file dialogs:
- Added `rfd 0.15.4` to `Cargo.toml`.
- Added IPC commands:
  - `pick_media_file`;
  - `pick_watermark_output`;
  - `pick_download_dir`.
- Added optional `output_dir` to single and batch download payloads.

Watermark behavior:
- Updated image watermark sizing so `100%` preserves the APK-derived width ratio and the UI size control scales relative to that official default.

Verification:
- Inline JavaScript syntax passed through Node parsing.
- `cargo check --bin html_app` passed with only existing dead-code warnings.
- `cargo fmt --all -- --check` passed.

## Step 135 - Built a standalone daily release without interrupting the running old app

Running-app handling:
- The previous `target/release/html_app.exe` was locked by a process launched from Windows Explorer.
- Did not terminate it because it belongs to the user's active desktop session.

Build output:
- Built the new release in the F-drive-only target directory:
  - `F:/Insta360onWin/target_daily/release/html_app.exe`
- Copied the finished product executable to:
  - `F:/Insta360onWin/LunaStudio.exe`
- Updated `run_release.bat` to start `LunaStudio.exe`.

Next real-device check:
- Close the old console window and launch `F:/Insta360onWin/LunaStudio.exe`.
- Connect Luna Ultra Wi-Fi and click `连接相机`.
- Confirm the media library populates and that the selected download folder is honored.

## Step 136 - Final daily-release smoke and encoding verification

Smoke test:
- Started `F:/Insta360onWin/LunaStudio.exe` from the project working directory with a hidden window.
- The process remained alive after four seconds, confirming that the Rust host, WebView, rewritten UI, and new native-dialog dependency initialize without an immediate startup crash.
- Stopped only the smoke-test process created by this step.

Final checks:
- Inline JavaScript syntax passed again.
- Rust formatting check passed again.
- Verified UTF-8 without BOM and CRLF-only line endings for all touched source/config/log files.
- Final standalone executable size: `8,463,872` bytes.

## Step 137 - Analyzed the PCAPdroid CSV connection summary

Input:
- Read only the user-provided file:
  - `C:/Users/H!Mooo/Downloads/PCAPdroid_22_7月_19_20_13.csv`
- No files were written to C drive.
- Capture time range includes activity around `2026-07-22 19:13` through `19:19` local time.

CSV scope:
- 398 connection-summary rows total.
- The CSV contains per-connection endpoints, timing, byte counts, and packet counts.
- It does not contain packet payload bytes, TCP stream contents, HTTP methods, HTTP paths, or response bodies.
- The `Info` field is empty for all camera HTTP/TCP rows, so individual operations cannot be decoded from this export alone.

Camera-local traffic:
- 53 connections target `192.168.42.1`:
  - 51 HTTP connections to port 80;
  - 2 TCP/UCD2 connections to port 6666.
- HTTP totals:
  - sent: `1,038,218` bytes;
  - received: `381,181,681` bytes (about 363.5 MiB).
- UCD2/control session 1:
  - `19:13:14.647` to `19:18:30.135`;
  - source port `49526`;
  - sent `611,825` bytes in `14,407` packets;
  - received `84,798,664` bytes in `26,382` packets.
- UCD2/control session 2:
  - `19:18:59.500` to `19:19:36.800`;
  - source port `50304`;
  - sent `7,390` bytes in `152` packets;
  - received `498,035` bytes in `200` packets.
- Largest HTTP transfer:
  - `19:13:47.286` to `19:14:02.608`;
  - received `335,645,840` bytes;
  - likely a large media download/stream, but this cannot be assigned to a specific user action without payload or an action timeline.

Conclusion:
- The capture successfully proves substantial official-app communication over Luna ports 80 and 6666.
- This CSV cannot recover camera command packets because it is an aggregate connection report.
- The next capture must be exported as `.pcap` or `.pcapng` with full packet payloads. CSV may be exported in addition, but not instead.
- For clean command mapping, record one operation per capture or note the exact wall-clock time of each operation.

## Step 138 - Parsed the full PCAPdroid packet capture

Input:
- Read only `C:/Users/H!Mooo/Downloads/PCAPdroid_22_7月_19_28_38.pcap`.
- No files were written to C drive.
- The 796,720,630-byte capture is classic little-endian PCAP with raw-IP link type 101.
- Capture range: `2026-07-22 19:28:39.141` to `19:32:49.356` (UTC+08:00).

Added streaming parser:
- `reverse_apk/tools/analyze_pcapdroid.py`
- It reads the large PCAP without loading the entire file into memory.
- It parses raw IPv4/TCP, reassembles each TCP direction, handles retransmission overlap, extracts complete UCD2 frames, and recovers HTTP request lines.

Generated outputs under `reverse_apk/pcap_analysis/20260722_192838/`:
- `summary.json`
- `ucd2_frames.json`
- `http_requests.json`
- `ucd2_timeline.csv`

Capture quality:
- 227,377 packets parsed.
- One UCD2 flow: `10.215.173.1:51572 <-> 192.168.42.1:6666`.
- 612 phone-to-camera UCD2 frames and 6061 camera-to-phone UCD2 frames.
- Zero TCP reassembly gaps and zero discarded prefix bytes in both directions.
- 38 HTTP requests recovered.

## Step 139 - Corrected the UCD2 header model

Correction:
- UCD2 header byte 6 is the frame type (`01`, `04`, or `05` in this capture).
- Header byte 7 is a dynamic sequence byte.
- Previous labels such as `04 09` must be read as type `04`, sequence `09`, not as a fixed two-byte command type.

Parser update:
- Added `frame_type` and numeric `sequence` to every extracted frame.
- Summary counts now group by direction and frame type.
- CSV timeline now includes separate `frame_type` and `sequence` columns.

Observed counts:
- Phone to camera: type `04` = 449, type `05` = 163.
- Camera to phone: type `01` = 5167, type `04` = 487, type `05` = 407.

## Step 140 - Recovered and paired daily capture controls

Added handoff report:
- `reverse_apk/pcap_analysis/20260722_192838/CONTROL_FINDINGS.md`

Confirmed by request ID, response, and resulting file timestamps:
- Start recording: internal command `0x0004`, method `0x02`, body `08 01`.
- Stop recording: internal command `0x0005`, method `0x02`, body `10 01`.
- Take photo: internal command `0x0003`, method `0x02`, body `30 03`.

Strong protocol proof:
- Stop-recording request ID `0x8000003a` received a matching `0x00c8` response containing `/DCIM/Camera01/VID_20260722_193010_205.mp4`.
- Start-recording was sent at `19:30:10.578`; the resulting file name contains `193010`.
- Take-photo was sent at `19:30:28.531`; the resulting JPEG name contains `193028`.

Important implementation constraint:
- Captured whole frames must not be replayed unchanged.
- UCD2 sequence, internal request ID, and checksum are dynamic.
- A daily sender must allocate those fields, retain the TCP session, and pair the `0x00c8` response by request ID.

Additional evidence:
- Device info includes model `Insta360 Luna Ultra`, serial `BTLA3ABESWPJTD`, firmware `v1.0.38`, and service name `Luna Ultra SWPJTD.OSC`.
- `0x00c9` is a strong media-refresh candidate.
- `0x000b` returns large media preview/thumbnail data and is not a delete command.
- Settings/status command families were preserved in the report but remain unnamed until correlated with the user's exact action order.

## Step 141 - Recovered the official live-preview control pair

PCAP correlation:
- Start preview: command `0x0001`, method `0x02`, body `10 01 30 28 38 2c 40 01 48 28 50 22`.
- Start request at `19:29:54.447`; matching `0x00c8` response at `19:29:54.495`.
- Type-01 stream begins at `19:29:55.117`.
- Stop preview: command `0x0002`, method `0x02`, empty body.
- Last type-01 frame at `19:31:24.417`; stop request at `19:31:24.420`; matching response at `19:31:24.458`.

Stream layout:
- UCD2 type `01`, subtype `0x20`: HEVC access unit.
- Subtype payload bytes 1..8: little-endian millisecond timestamp.
- Subtype payload bytes 9..: Annex-B HEVC.
- Other observed subtypes: `0x30`, `0x40`, and `0x85` for metadata/auxiliary traffic.

## Step 142 - Implemented the production UCD2 camera worker

Updated `src/adapters/luna_local.rs`:
- Added one persistent `CameraControlSession` worker thread.
- Added dynamic outbound UCD2 sequence allocation.
- Added dynamic internal request IDs beginning after the captured initialization request.
- Added a 1.5-second type-05 heartbeat matching official-app timing.
- Added TCP stream reassembly for fragmented/coalesced UCD2 frames.
- Added `0x00c8` response pairing by exact request ID.
- Added timeout, connection-close, non-success-status, and pending-request failure handling.
- Added stop-recording media-path extraction.
- Added type-01 HEVC stream demultiplexing and keyframe-aware backpressure recovery.
- Session startup now waits for the matching initialization response before HTTP media access begins.

Daily controls:
- `take_photo`: `0x0003`, body `30 03`.
- `start_recording`: `0x0004`, body `08 01`.
- `stop_recording`: `0x0005`, body `10 01`.
- `start_preview`: `0x0001`, captured 12-byte body.
- `stop_preview`: `0x0002`, empty body.

## Step 143 - Unified media and camera control onto one port-6666 session

Previous risk:
- The old media path opened a `LunaAuthSession` on port 6666.
- New daily controls opened another UCD2 connection.
- Two control connections could replace or disconnect each other on the camera.

Change:
- `list_media`, single download, batch download, capture controls, and live preview now share the single `CameraControlSession`.
- HTTP port 80 performs only authenticated media reads while the UCD2 session remains alive.
- Connecting and refreshing the album no longer creates a second control socket.

## Step 144 - Added the daily capture and live-preview UI

Updated `web/index.html`:
- Added a stable 16:9 live canvas as the primary camera work surface.
- Added Chinese controls for preview, photo, start recording, and stop recording.
- Added recording state and elapsed-time display.
- Added control-ready, preview-waiting, active-preview, and decoder-error states.
- Photo completion schedules an album refresh after 4.5 seconds.
- Stop-recording completion shows the returned camera media path and refreshes the album.
- Verified screenshots at `1180x780` and minimum window size `760x560`.
- No horizontal overflow was observed; the minimum layout uses normal vertical page scrolling.

## Step 145 - Added bundled low-latency HEVC decoding

Compatibility finding:
- The installed Chrome WebCodecs implementation exposes `VideoDecoder` but reports both tested HEVC codec strings as unsupported.
- Browser-only HEVC decoding was therefore not accepted as a daily-grade implementation.

Decoder implementation:
- Added the same immutable `ffmpeg-static` b6.1.1 Windows x64 runtime referenced by `reverse_apk/references/luna-ai-cut/scripts/copy-ffmpeg.mjs`.
- Runtime path: `assets/ffmpeg/ffmpeg.exe`.
- Rust pipes Annex-B HEVC to FFmpeg with low-delay settings.
- FFmpeg outputs scaled 1280-wide, 15 fps MJPEG frames.
- Rust parses concatenated JPEG SOI/EOI boundaries and sends only complete JPEG frames to WebView2.
- HTML keeps one in-flight image decode and replaces queued frames with the newest frame to bound latency.
- The decoder process is created without a visible console window and exits when the app closes.

Validation:
- Extended the PCAP analyzer with `--preview-hevc`.
- Extracted `24,800,242` bytes to `reverse_apk/pcap_analysis/20260722_192838/live-preview.h265`.
- FFmpeg decoded the captured stream to a valid real camera image.
- The production MJPEG pipe test emitted exactly three complete JPEG starts and three complete JPEG ends.

## Step 146 - Built and smoke-tested the updated daily app

Tests:
- Added byte-exact tests that regenerate all five captured daily command frames, including CRC:
  - preview start;
  - preview stop;
  - take photo;
  - start recording;
  - stop recording.
- Added fragmented-frame reassembly and HEVC keyframe detection tests.
- `cargo test --bin html_app`: 8 passed, 0 failed.
- Inline JavaScript syntax check passed.
- `cargo check --bin html_app` passed with only existing unused legacy-module warnings.
- `cargo fmt --all -- --check` passed.

Release:
- Built `target_daily/release/html_app.exe`.
- The running old `LunaStudio.exe` remained locked and was not terminated.
- Copied the new build to `F:/Insta360onWin/LunaStudio-next.exe`.
- Updated `run_release.bat` to launch `LunaStudio-next.exe`.
- Smoke test kept the app alive with title `Luna 控制台` and confirmed one child decoder at `assets/ffmpeg/ffmpeg.exe`.
- Stopping the smoke-test app left zero FFmpeg child processes.

## Step 147 - Finalized the release handoff

Final checks:
- Re-ran `cargo fmt --all -- --check`; it passed.
- Re-ran the inline JavaScript syntax check; it passed.
- Verified `LunaStudio-next.exe` and `target_daily/release/html_app.exe` are byte-identical by SHA-256.
- Verified the bundled decoder reports FFmpeg 6.1.1.
- Confirmed `run_release.bat` launches `LunaStudio-next.exe`.
- Confirmed the user's existing `LunaStudio.exe` process is still running and was not terminated.

Release SHA-256:
- `7FD968D7CF8E66093EB1473A088F696039E795030046E8A101755C2FF8B037A3`

## Step 148 - Added internal-storage and SD-card media libraries

Root cause:
- The Rust HTTP media reader was hard-coded to `/storage_internal/DCIM/Camera01/`.
- The HTML IPC payload carried only the host and had no storage selector.

Implementation:
- Added internal storage root `/storage_internal/DCIM/` and SD-card HTTP root `/DCIM/`.
- Added `all`, `storage_internal`, and `sdcard` media-list modes.
- Added discovery and traversal of `CameraNN/` directories instead of assuming only `Camera01/`.
- `all` mode tolerates one unavailable storage, so an absent SD card does not hide internal media.
- Every returned media item now includes `storage_id` and a Chinese `storage_label`.
- Added a responsive “全部存储 / 内部 / SD 卡” segmented control to the daily UI.
- Media cards and the library summary now show the source storage.
- A failed SD-only request no longer marks the camera itself as disconnected.

Validation:
- Added a unit test for Camera-directory discovery, URL generation, and storage labels.
- `cargo test --bin html_app`: 9 passed, 0 failed.
- Inline JavaScript syntax check passed.
- Responsive screenshots passed at 1180x780 and 760x560 without horizontal overflow.
- Read-only checks against the connected camera returned HTTP 401 for both `/storage_internal/DCIM/` and `/DCIM/`, proving both routes exist but require the established UCD2 session; `/sdcard/DCIM/` returned 404.
- Built `target_daily/release/html_app.exe` and copied it to `LunaStudio-latest.exe`.
- Updated `run_release.bat` to launch `LunaStudio-latest.exe` because the older executables were running and left untouched.
- Smoke test opened `LunaStudio-latest.exe` with title `Luna 控制台`, started its bundled FFmpeg/WebView2 children, and left no child process behind after closing the test instance.

Release SHA-256:
- `E67218205C38ED3B6B7081F30FDAD6CA7A585310F401FE7A93054189E1BBD92F`

## Step 149 - Added gallery previews and right-click multi-delete

Preview behavior:
- Photo cards now lazy-load the actual camera JPEG instead of showing a placeholder icon.
- Video cards locate the matching `LRV_...lrv` companion in the same storage and Camera directory.
- The LRV is used for the inline moving preview and the full preview dialog, while the original MP4 remains the selected/downloaded item.
- An LRV that has a matching original video is hidden as an implementation detail, avoiding duplicate gallery cards and counts.
- Video previews play on hover and return to the first frame on pointer leave.

Right-click behavior:
- Right-clicking an unselected card selects that card; right-clicking a selected card preserves the current multi-selection.
- The context menu provides preview, select/unselect, download, and delete commands.
- Deletion opens a custom Chinese confirmation dialog and clearly states that the action is permanent.
- Deleting an original video also includes its matching LRV companion.

Deletion protocol:
- Added UCD2 command `0x000c` with protobuf `DeleteFiles { repeated string uri = 1 }` bodies.
- Requests are deduplicated and split into batches of at most 50 paths with a 20-second response timeout.
- URLs must match the connected camera host and HTTP port.
- Only `/storage_internal/DCIM/`, `/sdcard/DCIM/`, and `/DCIM/` files are accepted.
- Encoded dot segments, slashes, backslashes, traversal components, directories, and invalid UTF-8 are rejected before any command is sent.

Validation:
- Added exact protobuf body, long-varint, Unicode path, host, root, and traversal tests.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- `cargo check --bin html_app`, Rust formatting, and inline JavaScript syntax checks passed.
- Mock gallery screenshots verified real image/video thumbnails, LRV de-duplication, multi-selection, and the delete confirmation dialog at desktop and narrow widths.
- A real destructive delete was intentionally not issued during automated validation.
- Built `target_daily/release/html_app.exe` and copied it to `LunaStudio-gallery.exe`; running older builds were not terminated.
- Updated `run_release.bat` to launch `LunaStudio-gallery.exe`.

Release SHA-256:
- `C6572A58B74B8C8F9DFE559E5AF524D51DD4483B8FFFE9038665B1DDBB4C110D`

## Step 150 - Removed obsolete builds and generated caches

Cleanup policy:
- Preserved `LunaStudio-gallery.exe`, `run_release.bat`, runtime `assets`, source code, `reverse_apk`, and all handoff documents.
- Preserved the current gallery WebView2 profile.
- Did not terminate any user-owned process.

Removed:
- Cargo targets: `target`, `target_daily`, `target_apk_device_info`, `target_heartbeat_collect`, and `target_ui_test`.
- Temporary reverse Python environment `.tools_py`.
- Obsolete `LunaStudio-next.exe` and `LunaStudio-latest.exe` builds.
- Their matching WebView2 profile/cache directories.
- Generated `androguard.db`, `androguard.db-shm`, and `androguard.db-wal` files.

Result:
- Approximately 9.6 GiB of generated or obsolete files were removed.
- PID 11400 was initially left untouched while it was running.
- A later status check confirmed that the old process had exited normally.
- The final `LunaStudio.exe` and `LunaStudio.exe.WebView2` paths were then removed.
- `LunaStudio-gallery.exe` is now the only application executable in the project root.

## Step 151 - Reworked the gallery for large media libraries

Performance changes:
- The gallery now renders only the first 48 logical media items instead of creating every card at once.
- A bottom `显示更多` button adds another 48 items per action and reports the remaining count.
- Filtering, sorting, storage changes, and a fresh media response reset the gallery to the first batch.
- Photo cards no longer point WebView2 directly at full-resolution camera files.
- An `IntersectionObserver` queues thumbnails only near the visible viewport with a maximum of two concurrent camera requests.
- Rust downloads the source once, generates a maximum 480x320 JPEG at quality 74, and stores it under `data/gallery-thumbnails` for reuse.
- Thumbnail URLs are validated against the connected camera host and approved DCIM roots before any request is sent.
- Failed background thumbnails stay quiet and show a fallback instead of producing repeated error toasts.
- Video cards use `preload=none`; their LRV URL is attached only while the card is hovered and is released on pointer leave.
- LRV/original pairing now uses a one-pass map instead of repeatedly scanning the complete media array.
- Selecting, clearing, right-clicking, and updating failed downloads now modify existing card state without rebuilding the gallery.
- Off-screen date sections use CSS `content-visibility` to reduce layout and paint work.

Validation:
- Inline JavaScript syntax check passed.
- `cargo fmt --all -- --check` passed.
- `cargo check --bin html_app` passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed and a hidden smoke test opened `LunaStudio.exe` with title `Luna 控制台`.
- WebView2 and bundled FFmpeg children started correctly, and the smoke-test process closed cleanly.
- Replaced `LunaStudio-gallery.exe` with `LunaStudio.exe` and updated `run_release.bat`.
- Removed the obsolete gallery executable and its WebView2 profile after its user process had exited.
- Removed the temporary `target_fast` Cargo build directory after validation; only the release executable and current runtime profile remain.

Release SHA-256:
- `85F388610E611267D817E3CDBB6D2572AFD455D4E85B65F17FAE4F7651800F92`

## Step 152 - Replaced hover video playback with cached video thumbnails

Behavior:
- Video cards no longer create or play HTML video elements on hover.
- Every video card now requests a static JPEG preview through the same viewport-aware thumbnail queue used by photos.
- The matching LRV file is preferred as the thumbnail source; the original video is used only when no LRV companion exists.
- The play badge remains visible over the generated frame, and clicking the card still opens the normal video player.

Backend:
- Extended `media_thumbnail` with a `media_type` discriminator.
- Video thumbnails are extracted at 0.2 seconds with the bundled FFmpeg and scaled within 480x320.
- FFmpeg output is read asynchronously, validated as JPEG, and terminated after a 25-second timeout.
- Video previews use the existing `data/gallery-thumbnails` cache, with media type included in the cache hash.
- Camera host and DCIM-path validation still runs before FFmpeg receives any URL.

Validation:
- Generated a temporary MP4 on the F drive and extracted a valid JPEG using the production FFmpeg arguments.
- Inline JavaScript syntax check passed.
- `cargo fmt --all -- --check` passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed as `LunaStudio-video.exe`.
- Hidden smoke test opened the new build with title `Luna 控制台`; WebView2 and bundled FFmpeg children started and closed normally.
- The existing `LunaStudio.exe` process was left running and was not terminated.
- Updated `run_release.bat` to launch the new video-thumbnail build after the current window is closed.
- Removed the temporary FFmpeg test media and `target_video` Cargo build directory after validation.

Release SHA-256:
- `441AFD1B0B0B5EA170FCD11EB2B19185E2AF7505BB09BF50180EB0C0E74F4A76`

## Step 153 - Added native Windows Mica and Mica-aware HTML rendering

Native window:
- Enabled transparent rendering on both the tao top-level window and the wry WebView2 layer.
- Applied `DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW` through `DwmSetWindowAttribute`.
- Added the legacy Windows 11 Mica attribute as a fallback when the system-backdrop attribute is unavailable.
- Applied rounded-window preference and synchronized the native title bar with the current Windows light/dark theme.
- Reapplies native theme attributes when tao receives `WindowEvent::ThemeChanged`.
- DWM failures remain non-fatal so unsupported systems still open the application with the HTML fallback color.

HTML rendering:
- Made the document and WebView background fully transparent so the native material remains visible.
- Replaced opaque workspace surfaces with light-transmitting Mica-aware color variables.
- Limited expensive `backdrop-filter` use to the fixed sidebar and top bar; gallery cards use translucent fills without per-card blur.
- Added a complete `prefers-color-scheme: dark` palette for text, controls, borders, selection, cards, and status colors.
- Added `prefers-reduced-transparency` opaque fallbacks for accessibility and unsupported visual configurations.
- Kept the live preview and full-screen media viewer opaque for accurate image/video viewing.

Validation:
- Inline JavaScript syntax check passed.
- `cargo fmt --all -- --check` passed.
- `cargo check --bin html_app` passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed successfully.
- Visual smoke test confirmed a readable Mica window with no white flash, black page, or uncontrolled desktop transparency.
- WebView2 and bundled FFmpeg children started and closed normally during the smoke test.
- Published the result as the sole daily entry `LunaStudio.exe` and restored `run_release.bat` to that name.
- A final smoke test of `LunaStudio.exe` passed with the expected window title and child processes.
- Removed temporary Mica/video executables, their WebView2 profiles, the visual-test screenshot, and `target_mica` after validation.

Release SHA-256:
- `F2B2CEBB70584B80BFAA2C76258A56D24B34C3D615A324C77BED7E737A24AD8E`

## Step 154 - Completed the Mica capability fallback and UI audit

Audit findings from the previous build:
- Native DWM Mica calls discarded their HRESULT values, so unsupported systems still received a transparent HTML palette.
- Windows High Contrast had no dedicated fallback.
- Video badges still used one `backdrop-filter` per visible gallery card.
- Dark-mode workspace transparency was too high and could look washed out against a light desktop backdrop.

Native fixes:
- `apply_mica` now returns whether the Windows 11 main backdrop or legacy Mica call succeeded.
- The embedded HTML root is assigned `native-mica` on success or `no-native-mica` on failure before WebView2 loads it.
- Corner preference, dark title bar, theme-change reapplication, and non-fatal DWM behavior remain intact.

Frontend fixes:
- Added opaque light and dark palettes for `html.no-native-mica`.
- Added Windows High Contrast system-color handling through `forced-colors`.
- Expanded reduced-transparency handling to every blurred frontend overlay.
- Removed per-card video-badge blur.
- Increased light and dark surface opacity so Mica remains visible in the page background and fixed chrome while controls retain stable contrast.
- Kept live preview and full media viewing surfaces opaque.

Validation:
- Inline JavaScript syntax check passed.
- `cargo fmt --all -- --check` passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed successfully; only pre-existing unused OSC/dead-code warnings remain.
- Rebuilt desktop and minimum-window screenshots passed at 1196 x 819 and 760 x 560 with no horizontal overflow.
- The first desktop screenshot was discarded because another window covered the app; the clean foreground capture and final rebuilt captures were inspected instead.
- UTF-8 without BOM and CRLF were verified for all touched source and handoff files.
- Hidden smoke test created and closed the final native window successfully.
- Removed the audit executable/profile, screenshots, scripts, and `target_mica_audit`; `LunaStudio.exe` is the only application executable in the project root.
- The detailed per-operation handoff is `reverse_apk/MICA_UI_CONTINUE.md`.

Release:
- File: `F:/Insta360onWin/LunaStudio.exe`
- Size: 8,806,400 bytes.
- SHA-256: `F058AB915780073BB0BF1F55987318A1374BF943EC2004F304031F7FB807077F`

## Step 155 - Matched the original AstroBox Mica implementation and made it visible

Reference audit:
- Used `AstralSightStudios/AstroBox-Public`, not AstroBox-NG/next-gen, at commit `616c5dbcc6653b8c124337122396d18226bfbd8c`.
- Its Tauri config combines `transparent: true` with `windowEffects.effects: ["mica"]`.
- Its lockfile resolves `window-vibrancy 0.6.0`; the exact source uses DWM attribute 38 with `DWMSBT_MAINWINDOW` on current Windows 11 and attribute 1029 on early Windows 11.
- AstroBox keeps the HTML root/provider transparent and uses a very light translucent content layer, especially in dark mode.
- Read-only source snapshots are retained under `reverse_apk/references/AstroBox-Public` and `reverse_apk/references/window-vibrancy-0.6.0`.

Root cause:
- Luna Studio's native DWM request was technically equivalent to AstroBox's dependency.
- It was applied before WebView2 creation, which could alter the host composition state afterward.
- The previous UI then covered the material with 72% to 94% dark surfaces, making a successful Mica backdrop look like a flat background.

Native changes:
- Reapply Mica immediately after WebView2 is created.
- Synchronize `native-mica`/`no-native-mica` on the HTML root with the post-WebView result.
- Reapply and resynchronize after Windows theme changes.
- Preserve rounded corners, immersive title-bar mode, Windows 11 fallback attribute, and opaque unsupported-system behavior.

Frontend changes:
- Native-Mica light mode now uses a transparent root, lightly tinted chrome/sidebar, and readable white content surfaces in the same layering style as original AstroBox.
- Native-Mica dark mode uses substantially lighter material tints than the previous build while retaining enough dark tint for this machine's unexpectedly light client-area backdrop.
- Reduced-transparency and High Contrast selectors now explicitly override both `native-mica` and `no-native-mica`, fixing selector-specificity gaps.
- Added a subtle sidebar divider; camera preview and full media views remain opaque for visual accuracy.

Measured validation:
- `DwmGetWindowAttribute(hwnd, 38)` returned HRESULT `0`, value `2` on both audit and published builds; value `2` is native MainWindow Mica.
- `DwmGetWindowAttribute(hwnd, 20)` returned HRESULT `0`, value `1` during visual validation.
- The first maximally transparent dark screenshot was rejected because the real client-area material was too light for white text; a moderate dark material tint corrected contrast without restoring the old near-opaque cover.
- Final screenshots passed at 1196 x 819 and 760 x 560 without horizontal overflow or clipped controls.
- Inline JavaScript syntax and Rust formatting passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build and hidden published smoke test passed.
- Removed the audit executable/profile, validation files, and `target_mica_astrobox`; no process was left running.

Release:
- File: `F:/Insta360onWin/LunaStudio.exe`.
- Size: 8,807,936 bytes.
- SHA-256: `934D9917B20EFD5CBC84D8751E50517C561541E48A36841C55C6DEB4B029142C`.

## Step 156 - Established real full-client Mica and raised frontend transparency

User verification:
- The user correctly reported that the prior build still did not visibly show Mica in the application client.
- After the native fix, the user confirmed that Mica appeared and requested more transparent frontend elements.

Root-cause proof:
- The native DWM attribute was active, and the HTML/body/`.app` roots were transparent, but a transparent root pixel still measured pure white RGB `255,255,255`.
- A temporary red native-client paint appeared through the WebView and translucent HTML layers, proving that WebView2 transparency worked.
- The actual blocker was the top-level Win32 client buffer: it remained white and prevented the DWM material from reaching WebView2.
- Tao 0.34.0 and 0.35.3 use the same legacy transparent-window block; Wry 0.55.1 correctly requests an alpha-zero WebView2 background.

Native implementation:
- Kept WebView2 transparent but stopped using Tao's legacy top-level transparent-window mode.
- Added a pre-WebView `prepare_mica_surface` stage.
- It extends the DWM frame over the full client, installs the stock black glass brush, and invalidates the native client before WebView2 is created.
- Mica is applied before and after WebView2 creation and after theme changes.
- `native-mica` is assigned only when both native surface preparation and the Mica backdrop call succeed.

Frontend refinement:
- Dark Mica workspace now uses a 4% white material layer instead of a 55% black cover.
- Sidebar uses a 22% dark tint; standard/strong/soft surfaces use 6.5%/10%/7.5% white, and chrome uses 2.5%.
- Light Mica layers were also reduced while preserving opaque unsupported-system, reduced-transparency, and High Contrast fallbacks.
- Video/live-preview surfaces remain dark for image accuracy.

Measured validation:
- Same active 1100 x 740 window, DWM type `2` versus type `1`: 513,055 of 814,000 pixels changed.
- Changed-pixel share: 63.03%; mean per-channel RGB delta: 18.949.
- This replaces the earlier title-bar-only 3.40% result.
- The 780 x 600 minimum capture retained readable controls, vertical scrolling, and no horizontal overflow.
- Inline JavaScript syntax and Rust formatting passed.
- Static root, WebView transparency, black-glass preparation, and 4% workspace assertions passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed with only the pre-existing unused OSC/dead-code warnings.

Published build:
- File: `F:/Insta360onWin/LunaStudio.exe`.
- Size: 8,806,400 bytes.
- SHA-256: `FD3C21CF56C83F03BD0F2CB960169A8C02DF09CDC5F89F1B5BFE58C4B413BFD2`.
- Published smoke test returned backdrop HRESULT `0`, type `2`, and dark-mode HRESULT `0`, value `1`.

Cleanup and handoff:
- Removed the audit executable/profile, validation captures, and `target_mica_root`.
- Replaced the incomplete Tao 0.34.0 partial checkout with a clean exact-tag snapshot at commit `5ac00b57`.
- `LunaStudio.exe` is the only executable in the project root, and no project process remains.
- Detailed evidence and per-operation notes are in `reverse_apk/MICA_UI_CONTINUE.md`.

## Step 157 - Added the separate capture page, enforced capture modes, and restored gimbal control

Scope:
- Split capture controls out of the media library into a dedicated daily-use page.
- Require photo mode before taking a photo and video mode before starting a recording.
- Recovered the Luna Ultra gimbal command from the local APK/PCAP evidence and exposed it as directional hold controls plus a two-axis pad.

Protocol:
- Capture-mode selection uses internal command `0x0007`, method `0x02`.
- Normal photo body is `08 28 12 00`.
- Normal video body is `08 28 12 03 c0 02 01`.
- Gimbal uses internal command `0x00E2`, method `0x02`.
- Gimbal axes are signed `-100..100` protobuf ZigZag values; release uses body `08 02`.
- Rust byte-exact tests reproduce five captured gimbal points, the neutral point, and both normal capture modes.

Implementation:
- `CameraControlSession` now owns capture-mode and recording state and rejects invalid photo/record/mode transitions in the backend.
- Added IPC handlers for capture-mode switching, gimbal movement, and gimbal release.
- Added a single-flight latest-point gimbal queue so pointer motion cannot create an unbounded request backlog.
- The dedicated capture page retains live preview, mode selection, one contextual shutter, recording timer, directional controls, and a two-axis pad.
- Capture-page connection now opens the UCD2 control session directly instead of waiting for the full media library to load.
- The Mica root and responsive minimum-window behavior remain intact.

Validation:
- Inline JavaScript syntax passed.
- `cargo fmt -- --check` passed.
- `cargo test --bin html_app`: 12 passed, 0 failed.
- Release build passed with only the pre-existing unused OSC/dead-code warnings.
- Visual checks passed at 1180 x 780 and 760 x 560 with no horizontal overflow.
- During the final live check, TCP port 6666 was reachable but the camera returned no UCD2 initialization frame to either the app or a minimal two-packet probe. The UI correctly kept all capture controls disabled. No shutter or recording command was sent.

Handoff:
- Full packet evidence, file-level changes, state-machine rules, UI behavior, and live-test caveat are in `reverse_apk/CAPTURE_CONTROL_CONTINUE.md`.

Release:
- File: `F:/Insta360onWin/LunaStudio.exe`.
- Size: 8,844,288 bytes.
- SHA-256: `1F74E00B7F8F33652898FBFFE2B22C8BF883273456D0F8DC7AA08F2AD59541E5`.
- Published smoke test created the native window successfully and returned Mica attribute 38 value `2` plus immersive-dark attribute 20 value `1`.
- Removed capture-page audit images, the JavaScript syntax-check temporary file, and the published-smoke WebView2 profile.
- `LunaStudio.exe` is the only executable in the project root, and no project process remains.

## Step 158 - Corrected gimbal orientation, removed camera recentering, and confirmed real recording state

User-reported failures:
- Pressing up moved the camera left.
- Pressing down moved the camera right.
- Pressing left moved the camera down.
- Letting the joystick visually return to center also drove the camera back.
- The application displayed a recording timer even though the camera did not actually record.

Gimbal correction:
- Converted UI coordinates to device coordinates with `device_x = -ui_vertical` and `device_y = ui_horizontal`.
- Added byte-independent direction tests for UI up/down/left/right.
- Removed the queued `(0, 0)` movement from pointer release.
- Pointer release now clears stale pending movement, visually centers the UI control, and sends only captured release body `08 02`.
- If an in-flight movement fails, the queue still attempts one release; a release error does not recursively queue another release.

Capture protocol verification:
- Re-analyzed the user's official-app PCAP with full type-04 control payloads.
- Confirmed successful start at `2026-07-22T19:30:10.578+08:00`: command `0x0004`, method `2`, body `08 01`.
- Confirmed stop command `0x0005`, body `10 01`; its response contains `/DCIM/Camera01/VID_20260722_193010_205.mp4`.
- Confirmed normal photo mode `0x0007 / 08 28 12 00` followed by context query `0x000A / 08 63 10 06`.
- Retained APK-derived normal video value `1`: `0x0007 / 08 28 12 03 c0 02 01`, followed by context query `0x000A / 08 63 10 07`.
- Rejected temporary mode hypotheses `8` and `100`: PCAP proves value `8` uses context `0x43`, while `100` came from an unrelated nested status field.

Recording-state correction:
- The old implementation treated the `0x00C8` command response as proof that recording had started.
- The official PCAP shows a subsequent `0x2010` event with body `08 01 10 00 38 00` when recording is active.
- Stop transitions through `08 01 10 00 38 02` and finishes at `08 00 10 00 38 00`.
- Added a worker-side recording-state waiter and `0x2010` parser.
- Start/stop state is now committed only after the device reports field 1 as `1`/`0`; otherwise the operation fails after three seconds instead of showing a false timer.

PCAP tooling:
- Extended `reverse_apk/tools/analyze_pcapdroid.py` so type-04 records retain `control_payload_hex`.
- An initial analysis run exposed a local variable typo (`frame_type`); corrected it immediately to use `frame[6]`.
- The rerun completed successfully and was used only for byte-level verification.

Validation:
- Inline JavaScript syntax passed.
- `cargo fmt -- --check` passed.
- `cargo test --bin html_app`: 14 passed, 0 failed.
- Release build passed with only the existing unused OSC/dead-code warnings.

Live-test boundary:
- A first port check appeared reachable, but `Get-NetTCPConnection` showed the connection originated from proxy address `198.18.0.1`.
- The physical `WLAN` adapter was actually `Disconnected`; its former `192.168.42.50/24` address was deprecated.
- The failed control connection therefore did not execute mode, record, or gimbal commands.
- Reconnect LunaU Wi-Fi before final device verification. Require the UI to show a ready control session before switching mode, and require a returned MP4 path after stop.

Publication:
- Normalized touched Rust, HTML, Python, and Markdown files to UTF-8 without BOM and CRLF.
- Rebuilt after normalization and replaced `F:/Insta360onWin/LunaStudio.exe`.
- Final size: 8,851,456 bytes.
- Final SHA-256: `6D5D62A7F7EDFC29F5D9255347B2A7C5BA12EE818D280883C57160315C13880A`.
- Published smoke test created `Luna 控制台`.
- DWM attribute 38 returned HRESULT `0`, value `2` (MainWindow Mica).
- DWM attribute 20 returned HRESULT `0`, value `1` (immersive dark mode).
- The smoke-test process closed cleanly.

Detailed continuation notes are in `reverse_apk/CAPTURE_CONTROL_CONTINUE.md`.

## Step 159 - Removed the disproven gimbal release path and analog pad

Latest user verification:
- The four cardinal directions are now physically correct.
- The camera still resets after input ends, so sending captured release body `08 02` does not satisfy “hold the current position”.
- Arbitrary points from the two-axis pad do not behave consistently enough for a daily-use control.

Decision:
- Treat physical user verification as authoritative over the earlier semantic guess.
- Keep only the four directions that have passed the user's real-device check.
- Do not expose unverified diagonal/analog coordinates as if they were reliable.

Frontend changes:
- Removed the two-axis pad, moving nub, center stop button, and all associated pointer handlers.
- Replaced the release routine with local-only `endGimbalInput`.
- On pointer-up, pointer-cancel, or window blur, the app now stops the repeat timer and clears unsent coordinates.
- No packet is queued when input ends.
- A movement error now stops locally and shows one error; it no longer attempts an automatic release packet.

Backend changes:
- Removed the `camera_gimbal_release` IPC command.
- Removed `CameraControlSession::release_gimbal`.
- A complete source search now finds no daily control path that can emit `0x00E2 / 08 02`.

Validation:
- Inline JavaScript syntax passed.
- `cargo test --bin html_app`: 14 passed, 0 failed.
- Existing direction mapping tests still pass.
- Release build passed with only the pre-existing unused OSC/dead-code warnings.
- Published `F:/Insta360onWin/LunaStudio.exe`, size 8,845,312 bytes.
- Published SHA-256: `5960DED2913F04BAA28F11E69BAAD463AE7E7DA1132D021A49C30B0C2AEAC1B6`.
- Published smoke test returned Mica attribute 38 value `2` and immersive-dark attribute 20 value `1`.

Remaining protocol boundary:
- This design deliberately leaves the camera at the last command state by sending nothing on release.
- It avoids the two device behaviors already proven undesirable: zero-coordinate recenter and `08 02` release recenter.
- Reintroduce analog movement only after a separate PCAP/action correlation establishes stable semantics for intermediate coordinates.

## Step 160 - Rebuilt gimbal control from the official gesture cadence

User correction:
- Step 159 misread “only up/down/left/right are correct now” as approval of the movement behavior.
- The user only confirmed the direction signs; fixed-amplitude 70 ms repetition and immediate local stop were still chaotic.
- Step 160 supersedes Step 159's four-button-only UI decision and its assumption that a normal release should send nothing.

PCAP timing analysis:
- The official-app capture contains 282 `0x00E2` requests.
- Three normal gesture segments contain 165 frames over 7162 ms, 37 frames over 726 ms, and 79 frames over 2863 ms.
- Their average intervals are 43.7 ms, 20.2 ms, and 36.7 ms.
- Every normal gesture starts at `(0,0)`, travels through continuous intermediate values, and returns through intermediate values to `(0,0)`.
- Successful request latency is min 3 ms, median 9 ms, p90 27 ms, p99 43 ms, max 47 ms.
- The only `08 02` body occurs about 1.78 seconds after the final zero vector and immediately after command `0x00EE`; it is controller teardown, not normal pointer release.

Implementation:
- Restored the two-axis touch pad and center stop button.
- Preserved the verified UI-to-device rotation: `device_x = -ui_vertical`, `device_y = ui_horizontal`.
- Direction buttons and the touch pad now set one shared target vector.
- Added a 25 ms control loop with a maximum Euclidean step of 18 units per tick.
- A new gesture first queues the current zero vector, then ramps toward the requested vector.
- Pointer-up, pointer-cancel, stop, and window blur ramp the vector back to zero.
- The controller sends four stable zero vectors before stopping its local timer.
- The existing single-flight queue still allows one request in flight and keeps only the newest pending coordinate.
- Errors and disconnects cancel the timer, clear current/target/pending state, and visually center the pad.
- `camera_gimbal_release`, `release_gimbal`, and all normal-operation paths for `0x00E2 / 08 02` remain removed.

Validation before publication:
- Inline JavaScript syntax passed.
- `cargo fmt -- --check` passed.
- `cargo test --bin html_app`: 14 passed, 0 failed.
- Release build passed with only the pre-existing unused OSC/dead-code warnings.

Live-test boundary:
- No autonomous gimbal movement was triggered during validation.
- Direction signs have prior physical confirmation; the rebuilt acceleration/deceleration behavior still requires the user to test it on the connected Luna Ultra.

Publication:
- Normalized `web/index.html` and both continuation Markdown files to UTF-8 without BOM and CRLF.
- Rebuilt after normalization and replaced `F:/Insta360onWin/LunaStudio.exe`.
- Final size: 8,853,504 bytes.
- Final SHA-256: `050B959AB729D55AA39160090CE6D07053D7C30F9371D3290CFCFA3762F11AFA`.
- Hidden smoke launch created a live native window titled `Luna 控制台`.
- Closed the smoke process and removed its `LunaStudio.exe.WebView2` profile.

## Step 161 - Subscribed to camera events before waiting for recording state

User report:
- Starting a recording failed with `相机没有进入预期的录像状态`.
- The start command had already received `0x00C8`; the failure came from the subsequent three-second `0x2010` state waiter.

PCAP correction:
- At `19:29:51.192`, before the successful official recording, the phone sends command `0x0011`, method `0x02`, body `08 01`.
- At `19:29:51.362`, the camera accepts it with `0x00C8` and body `0a 05 08 00 10 e8 07`.
- The successful start request is later sent at `19:30:10.578`.
- Its `0x00C8` response arrives at `19:30:11.481`, followed by the active-recording event `0x2010 / 08 01 10 00 38 00` at `19:30:11.605`.
- The prior worker parsed `0x2010` but never sent the preceding `0x0011 / 08 01` camera-event subscription.

Implementation:
- Added constants for the captured event-subscription command and body.
- `CameraControlSession::open` now sends the subscription immediately after UCD2 authentication.
- A control session is reported ready only after the subscription receives its matching successful response.
- Start/stop recording still require a real `0x2010` event; the patch does not revert to optimistic command-response state.
- Added a byte-exact test for `11 00 02 <request_id> 00 00 08 01`.

Reverse-work cleanup:
- An attempted Androguard parse of the reconstructed segmented DEX was rejected by its malformed relocated tables and created `F:/Insta360onWin/androguard.db`.
- The temporary database was removed immediately; no C-drive project file was used or modified.

Validation and publication:
- `cargo fmt -- --check` passed.
- `cargo test --bin html_app`: 15 passed, 0 failed.
- Release build passed with only the existing unused OSC/dead-code warnings.
- Normalized `src/adapters/luna_local.rs` to UTF-8 without BOM and CRLF before the final build.
- Closed the running old `LunaStudio.exe` normally before replacement.
- Final file: `F:/Insta360onWin/LunaStudio.exe`, 8,857,088 bytes.
- Final SHA-256: `C77F3BBF6285E9D0BF24F23B7AC5ECFD5CE21D50387DC5823C01B279402EC2E4`.
- Hidden smoke launch created `Luna 控制台`; the process and WebView2 smoke profile were removed.
- No recording command was triggered automatically. The event-subscription fix requires a user-initiated real-device recording test.

## Step 162 - Completed the official control-session setup and recovered hardware gimbal speed levels

Latest device feedback:
- Adding only `0x0011 / 08 01` did not make the camera enter recording.
- The user corrected the initial speed interpretation: changing the joystick speed also changes the camera's hardware joystick, so this is a persistent device parameter rather than local coordinate scaling.

Recording-session correction:
- Re-aligned the official PCAP from the first device-information response instead of looking only at the commands immediately before recording.
- The official session sends `0x000F` with an empty body immediately after the initial device query.
- It then registers a role-2 client with `0x0027`, synchronizes epoch seconds/UTC+8/`Asia/Shanghai` through `0x0007`, reads status properties `0x0B/0x55/0xB4` through `0x0008`, and later enables events with `0x0011 / 08 01`.
- The previous daily client skipped the first four steps and attempted capture commands from a minimally authenticated socket.
- `CameraControlSession::open` now executes the captured setup sequence and requires a matching `0x00C8` after every step before reporting ready.
- Start and stop recording still require real `0x2010` state events. No optimistic recording timer was restored.

Hardware joystick speed proof:
- At `19:31:17.911`, the official app sends `0x0009 / 08 55 12 05 aa 05 02 10 02 18 06`.
- The camera transitions through `0x206A / 08 00 10 03` and reports target `0x206A / 08 00 10 02`.
- At `19:31:20.984`, the app sends the same body with target `01`; the camera reports `0x206A / 08 00 10 01`.
- This proves option `0x55` has three hardware levels: `1` slow, `2` medium, and `3` fast. Level `3` was the pre-change state in the capture.
- The brief local percentage-slider implementation was removed before publication.

Implementation:
- Added `CameraControlSession::set_gimbal_speed` and a validated byte builder for levels `1..3`.
- The command uses capture context `0x06` for photo and `0x07` for video, matching the existing mode context.
- Added `camera_set_gimbal_speed` IPC.
- Replaced the percentage range control with Chinese `慢 / 中 / 快` segmented buttons.
- The software joystick again emits its original normalized `0x00E2` vectors; hardware speed is no longer simulated by scaling coordinates.

Validation and publication:
- Inline JavaScript syntax passed.
- UTF-8 without BOM and CRLF checks passed for all touched source files.
- `cargo fmt -- --check` passed.
- `cargo test --bin html_app`: 17 passed, 0 failed.
- Release build passed with only the existing unused OSC/dead-code warnings.
- Published `F:/Insta360onWin/LunaStudio.exe`, size 8,873,984 bytes.
- SHA-256: `7205837312B8DF2BA2BC12E05D9E006550A5608789FC32F87FCD0515A5504A29`.
- Hidden smoke launch created a responsive native window titled `Luna 控制台`.
- The smoke process and WebView2 profile were removed; the project root contains only `LunaStudio.exe` as an executable.
- No recording or gimbal movement was triggered automatically. Both the completed session setup and hardware speed write require the user's connected-device verification.

## Step 163 - Corrected the failed speed context and completed the capture-ready sequence

Real-device rejection:
- The user confirmed that none of the three hardware joystick speed buttons changed the device.
- The user also confirmed that the completed session initialization still did not allow video recording.
- This invalidates Step 162's unproven decision to change the speed command context with the selected capture mode, and proves that connection setup alone is not the full recording preparation.

Hardware speed correction:
- The only two proven speed writes in the PCAP both end in context `18 06`.
- The published Step 162 build sent `18 07` whenever video mode or no mode was selected, so it did not reproduce the captured hardware command.
- `build_gimbal_speed_body` now always emits the captured fixed context `0x06`.
- The UI no longer assumes level `3` on connection. Every click sends a device write, even when the clicked button matches the last UI value.
- The worker now parses `0x206A / 08 00 10 <level>` and a speed operation succeeds only after the target `1/2/3` event arrives.
- The captured `0x000A / 08 63 10 06` refresh is sent after the write.

Recording correction:
- The official successful recording session had live preview active before `0x0004`.
- Immediately before `0x0001`, the app sends `0x00BF / 58 0a` and `0x00C6 / empty`; the old preview method omitted both.
- Starting a recording now automatically starts preview when needed. The frontend waits for preview success before sending `camera_start_record`.
- `start_preview` now reproduces `0x00BF`, `0x00C6`, then the captured `0x0001` body.
- Before `0x0004`, the backend reads option `0x28`, verifies normal-video value `1`, refreshes context `0x07`, and queries options `0x14/0xB0/0xB1`.
- Mode selection also reads option `0x28` after the write and only commits the UI/backend mode when the camera reports the target value.
- Start recording still requires `0x2010 / field 1 = 1`; no false timer fallback was added.

Validation and publication:
- Added raw response bytes to the internal response object while keeping them out of serialized IPC output.
- Added option-response protobuf parsing for capture mode and `0x206A` parsing for speed.
- Inline JavaScript syntax passed.
- UTF-8 without BOM and CRLF checks passed for touched source files.
- `cargo fmt -- --check` passed.
- `cargo test --bin html_app`: 18 passed, 0 failed.
- Release build passed with only existing unused OSC/dead-code warnings.
- Published `F:/Insta360onWin/LunaStudio.exe`, size 8,884,736 bytes.
- SHA-256: `71200ED752D8F2A5F5F7C193D7A560E78256FF3507303650B7B5525F8E10E6E7`.
- Hidden smoke launch created a responsive native window titled `Luna 控制台`.
- The smoke process and WebView2 profile were removed.
- The computer was not physically connected to LunaU during validation, so no recording or hardware speed command was triggered automatically.

## Step 164 - Repaired capture-mode confirmation and decoupled daily media viewing

Latest real-device rejection:
- The Step 163 build reported that the camera did not switch to the target mode.
- Photo and video viewing also regressed.
- This proves that treating the `0x0008 / option 0x28` response as a current-mode value was incorrect.

Mode reverse evidence:
- Revisited only the reconstructed APK DEX and the user's official-app PCAP.
- `reverse_apk/reconstructed_dex/seg06.dex` code item `0x427b74`, type `@1884`, constructs the eleven `SYNC_MODE_*` enum values.
- The recovered order is `UNKNOWN=0`, `NORMAL_IMAGE=1`, `NORMAL_VIDEO=2`, `HDR_IMAGE=3`, `HDR_VIDEO=4`, `INTERVAL_IMAGE=5`, `TIMELAPSE_VIDEO=6`, `BURST_PHOTO=7`, `BULLETTIME_VIDEO=8`, `TIMESHIFT_VIDEO=9`, and `AEB_NIGHT_PHOTO=10`.
- The UCD2 payload removes the application-level `UNKNOWN` slot: the captured normal-photo payload uses wire value `0`, and the APK-derived normal-video payload uses wire value `1`.
- This supports retaining normal-video body `08 28 12 03 c0 02 01`; it does not support reading option-query capability data as the current mode.

Mode-switch correction:
- Removed the Step 163 option `0x28` response parser and every strict mode check based on it.
- The official PCAP sends mode command `0x0007` and context command `0x000A` about 3 ms apart, before either delayed `0x00C8` response.
- The worker now queues both requests back-to-back on the same authenticated socket before waiting for either response.
- A fresh waiter is registered before transmission and only commits the mode after camera event `0x2053` reports the expected wire value.
- Start recording no longer repeats the invalid option `0x28` mode query. It retains the captured video context, capture-ready query, start command, and real `0x2010` recording-state confirmation.

Media-viewing correction:
- Step 163 made `list_media`, every thumbnail request, and downloads call `camera_control_for`.
- That forced the full capture-session initialization into ordinary gallery browsing, so any capture-mode failure also broke or stalled photos and videos.
- Restored a persistent lightweight `LunaAuthSession` for media listing, original photo/video viewing, thumbnails, and downloads.
- A full `CameraControlSession` is created only for capture controls and camera-side deletion.
- Before opening full control, the lightweight session is closed. While full control is active, gallery HTTP requests reuse its authenticated device session instead of opening a second port-6666 socket.
- Device detection now loads the gallery without automatically connecting capture control.
- The gallery's right-click delete remains available; deletion upgrades the session on demand.

Validation so far:
- Inline JavaScript syntax passed.
- `cargo test --bin html_app`: 18 passed, 0 failed.
- No camera mode, shutter, recording, delete, or gimbal command was triggered automatically.
- Physical verification of mode transition, original photo viewing, and video playback is still required after publishing the corrected executable.

Publication:
- Normalized all five touched Rust, HTML, and Markdown files to UTF-8 without BOM and CRLF.
- `cargo fmt --all -- --check`, inline JavaScript syntax validation, tests, and the release build all passed.
- Published `F:/Insta360onWin/LunaStudio.exe`, size 8,889,856 bytes.
- SHA-256: `02530A0C9A555A626E45E3B7D38A417571BB965EE3702215D0956544C4A86D22`.
- Hidden smoke launch created `Luna 控制台`.
- DWM attribute 38 returned value `2` and attribute 20 returned value `1`.
- The smoke window did not close within the eight-second graceful wait, so only that smoke process was force-closed.
- Removed the smoke WebView2 profile after verifying its resolved path was inside `F:/Insta360onWin`.
- The project root contains only `LunaStudio.exe` as an executable, and no Luna Studio, Cargo, or Rust compiler process remains.

## Step 165 - Replaced broken direct media URLs with a streaming local proxy

Latest real-device UI rejection:
- The gallery populated, but clicking either a photo or video produced no usable preview.
- Step 164 still assigned the camera's `http://192.168.42.1/...` URL directly to WebView `<img>` and `<video>` elements.
- The Rust gallery path supplied the camera-specific identity-encoding request and maintained UCD2 authentication, while WebView issued an unrelated browser request with its own origin, headers, proxy behavior, and media MIME handling.

Implementation:
- Replaced `WebViewBuilder::with_html` with a loopback-only HTTP application server bound to a random `127.0.0.1` port.
- The same server supplies the embedded Chinese HTML and a restricted `/media/<UTF-8 hex URL>` endpoint, so the page and preview media are same-origin.
- The proxy validates every decoded URL with the existing camera media-path guard before opening it.
- Camera HTTP requests use the persistent lightweight/full authenticated session already owned by `AppState`.
- Every camera HTTP client now explicitly disables configured HTTP proxies and requests `Accept-Encoding: identity`.
- The proxy forwards browser `Range` requests, upstream `Content-Length`, `Content-Range`, `Accept-Ranges`, `ETag`, and `Last-Modified`, then streams response bytes instead of downloading a whole video before playback.
- Known photo/video extensions override a generic upstream MIME with browser-usable `image/jpeg`, `image/png`, `image/webp`, `video/mp4`, or `video/quicktime`.

Frontend:
- Original photos now load through the local media proxy.
- On an original-photo decode/load failure, the preview falls back to the already generated gallery thumbnail instead of remaining blank.
- Videos try the associated LRV through the proxy first and automatically retry the original file if the LRV is unavailable.
- A final load failure produces a Chinese in-dialog error state.

Validation so far:
- Added tests for UTF-8 media URL round trips and browser media MIME inference.
- `cargo test --bin html_app`: 20 passed, 0 failed.
- The loopback HTML route returned HTTP 200, contained the Luna Studio page, and contained the media-proxy frontend code.
- A physical media-byte test was not possible: `Test-NetConnection` showed `192.168.42.1` routed through `FlClash` with source `198.18.0.1`; the WLAN `192.168.42.50` address was deprecated.
- The unsuccessful proxy read did not trigger capture, recording, deletion, or gimbal control.

Publication:
- Normalized all five touched Rust, HTML, and Markdown files to UTF-8 without BOM and CRLF.
- Inline JavaScript syntax, `cargo fmt --all -- --check`, 20 tests, and the release build passed.
- Published `F:/Insta360onWin/LunaStudio.exe`, size 8,942,592 bytes.
- SHA-256: `DC3C7E3FF1A258D487633B1ED1FB4C4B0A477328A0C8BABF1EC8B6B305FBB17A`.
- Published smoke launch created `Luna 控制台`; its random loopback page returned HTTP 200 with the media proxy active.
- DWM attribute 38 returned value `2`, and attribute 20 returned value `1`.
- The smoke process closed normally.
- Removed the verified in-project WebView2 profile.
- The project root contains only `LunaStudio.exe` as an executable, and no Luna Studio, Cargo, or Rust compiler process remains.

## Step 166 - Removed the disconnecting probe and added the captured 1.5-second media heartbeat

Latest real-device report:
- The user reported that the camera connection began disconnecting unexpectedly.
- While the published app was open on a real `192.168.42.50` WLAN route, its only port-6666 connection was observed in `CloseWait`.
- A neighboring short-lived port-6666 connection was also present.
- `CloseWait` proves the camera side had already sent EOF while the application still held a session object.

Short-probe correction:
- The gallery `detect` IPC still called `check_status(host, true)`.
- That opened a bare TCP connection to port 6666 and immediately dropped it before opening the authenticated media session.
- Gallery detection now checks HTTP reachability only and never opens an unauthenticated control-port probe.
- The authenticated media session is the first and only gallery-side port-6666 connection.

Captured heartbeat evidence:
- Re-read only `reverse_apk/pcap_analysis/20260722_192838/ucd2_frames.json`.
- The official phone sends empty type-05 UCD2 frames at approximately 1.5-second intervals:
  - `19:28:46.698` sequence `0x0F`
  - `19:28:48.192` sequence `0x17`
  - `19:28:49.692` sequence `0x18`
  - `19:28:51.193` sequence `0x19`
  - `19:28:52.694` sequence `0x1C`
- The lightweight `LunaAuthSession` previously authenticated once and then neither sent these heartbeats nor continuously read the camera socket.

Implementation:
- `LunaAuthSession` now owns a background worker and command channel instead of an unattended `TcpStream`.
- After the two existing APK-derived authentication frames, the worker sends a valid empty type-05 UCD2 heartbeat every 1.5 seconds and continuously drains incoming camera frames.
- The initial lightweight-session heartbeat uses sequence `0x11`, following the captured authentication request sequence `0x10`.
- Authentication does not send an extra immediate pulse; the first post-auth heartbeat waits for the captured 1.5-second interval.
- `refresh` returns while the worker is active; if the worker/socket is dead, it closes the stale worker and reconnects once.
- `close` and `Drop` stop and join the worker, preventing detached sockets.
- Lightweight and full control sessions expose `is_active`; media/control helpers replace finished sessions instead of treating `Some(session)` as proof of connectivity.
- Added a byte-exact assertion for the previously observed `05 11` frame:
  `55 43 44 32 01 0c 05 11 00 00 00 00 76 20 c6 cb`.

Validation so far:
- `cargo test --bin html_app`: 20 passed, 0 failed.
- The existing official-session heartbeat sequence and all previous media/capture tests remain green.
- The Step 165 process was closed normally before publication.
- A live pre-publication attempt showed the device was already rejecting even the first known `05 0f` frame with EOF and zero response bytes after 215 ms.
- The final client therefore was not presented as physically verified; the camera must power-cycle/release that stale device-side session first.

Final connection/UI correction:
- Removed the frontend detect round trip entirely from the gallery connect button.
- The button now directly opens the authenticated media session and lists media.
- The UI reports connected only after `list_media` succeeds.
- A failed list now changes the global status to `相机会话连接失败` instead of leaving the earlier HTTP-only `Luna Ultra 已连接` label visible.
- Authentication waits for the first scheduled 1.5-second heartbeat instead of injecting an immediate post-auth pulse.

Publication:
- Normalized all five touched Rust, HTML, and Markdown files to UTF-8 without BOM and CRLF.
- Inline JavaScript syntax, `cargo fmt --all -- --check`, 20 tests, and the release build passed.
- Published `F:/Insta360onWin/LunaStudio.exe`, size 8,968,704 bytes.
- SHA-256: `A869366D97147B16BB03309C428855F56B352FD7479096EA61DA4142CB79CC64`.
- Smoke launch created `Luna 控制台`; the loopback page returned HTTP 200 with the media proxy present.
- DWM attribute 38 returned value `2`, and attribute 20 returned value `1`.
- The smoke process closed normally.
- Removed the verified in-project WebView2 profile.
- The project root contains only `LunaStudio.exe` as an executable, and no Luna Studio, Cargo, or Rust compiler process remains.

## Step 167 - Windows 虚拟摄像机发布

- 用户要求把 Luna Ultra 实时预览画面作为 Windows 虚拟摄像机。
- 相机协议没有使用公开资料；仍沿用 APK/PCAP 已恢复的预览通道。
- Windows 侧按微软官方 Media Foundation 虚拟摄像机架构实现。
- 新增 `src/virtual_camera.rs`：
  - COM class factory
  - `IMFActivate`
  - `IMFMediaSourceEx`
  - `IMFMediaStream2`
  - `MFCreateVirtualCamera`
  - HKLM 进程内 DLL 注册和一次性 UAC 安装
  - `1280 x 720 / 15 fps / NV12` 输出
  - `127.0.0.1:38475` 主程序到 Frame Server DLL 的本机帧桥接
- 新增 `src/lib.rs`，发布 `LunaVirtualCamera.dll`；PE 导出已确认包含
  `DllGetClassObject` 和 `DllCanUnloadNow`。
- `html_app.rs` 的实时预览统一 letterbox 到 720p/15fps，并把每帧同时送入虚拟摄像机。
- 新增内部命令：
  - `virtual_camera_status`
  - `virtual_camera_start`
  - `virtual_camera_stop`
- 拍摄控制页新增日用级别的一键开关。
- 未开启预览时会先开启预览，再串行开启虚拟摄像机。
- 关闭预览、断开 Luna 或退出应用都会停止 Session lifetime 虚拟摄像机。
- 第一次开启需要用户确认一次 UAC，之后 DLL 路径未变化时不会重复请求。
- 当前自动化环境未提权，没有冒充完成系统 Start 真机验证；必须由用户第一次点击开关确认 UAC。
- JavaScript 语法通过；Rust 测试 23 passed、0 failed、1 ignored；release EXE/DLL 构建通过。
- 发布：
  - `LunaStudio.exe`：`9,221,632 bytes`
  - SHA-256：`083DB1C92C93F343A067A99D10F7A8CCF409754DF47645BF8F1869783B75613E`
  - `LunaVirtualCamera.dll`：`225,792 bytes`
  - SHA-256：`A41FF5D4E244D6DACC0AA8D3F0C639C5E9AFC94795D3E19A6FD98EC51517684B`
- DLL 必须与 EXE 同目录，但根目录仍只有一个可启动 EXE。
- 完整实现、失败原因、注册位置、验证步骤见
  `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` 的 Step 167。

## Step 168 - 虚拟摄像机间歇黑屏与模式落地修正

- 用户真机确认虚拟摄像机不是永久黑屏，而是等待很久才出画面，之后会反复黑屏和恢复。
- 运行时端口证据显示 `127.0.0.1:38475` 同时存在主程序验证实例和系统相机实例。
- 根因是旧服务器只接受并持续服务第一个连接，后续连接只进入 backlog、不接收帧。
- 帧桥接改为每个客户端独立线程，允许多个 Windows 媒体源实例同时取帧。
- 开启 Windows 虚拟摄像机前就开始缓存实时帧；Start 失败时回滚缓存状态。
- 最新画面保留期从 3 秒增加到 30 秒，短暂解码抖动不再立刻切黑。
- UI 新增真实首帧门禁：JPEG 成功绘制后才执行 `virtual_camera_start`。
- 发布版双客户端烟雾测试中，两个连接均立即收到 `LVC1` 和完整
  `1,382,400-byte` NV12 payload。
- 重新读取本地 PCAP 的普通拍照模式切换：
  - `0x0007 / 08 28 12 00`
  - `0x000A / 08 63 10 06`
  - `0x2053 / 08 00 10 64 18 03`
  - 随后还有 context、capture-ready、detail、combined-context 四项刷新。
- 后端现在等四项刷新全部返回成功后才提交拍照/录像模式。
- Video 仍使用 APK 恢复的 wire value `1` 和 PCAP context `0x07`；
  该 PCAP 没有包含切回普通录像动作，因此没有发送猜测枚举。
- 测试：24 passed，0 failed，1 ignored。
- 发布：
  - `LunaStudio.exe`：`9,221,120 bytes`
  - SHA-256：`23253EDECFF0CC3C2E6C0B8685A5880737B84E07B420B0FE3F713E56F7149F43`
  - `LunaVirtualCamera.dll`：`225,792 bytes`
  - SHA-256：`2D62C39A2F0DBC2F06EF966140D2AE2F95BD4A14CE36662C40F32553D80A9A32`
- 没有自动触发模式、拍摄、录像、删除、云台或硬件速度操作。
- 完整细节见 `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 168。

## Step 169 - Windows CameraSwitchFailed 0x80070057 修复

- 用户截图确认 Windows 已识别 `Luna Studio Camera`，但 Camera 在启动流时返回
  `0xA00F4241 / 0x80070057`。
- 只对照项目内保存的微软 VirtualCamera 官方示例，确认旧媒体源缺少
  `IMFSampleAllocatorControl`，且 Source Started/Stopped 事件没有系统时间值。
- `src/virtual_camera.rs` 现在：
  - 实现 `IMFMediaSourceEx + IMFSampleAllocatorControl`；
  - 输出流 `0` 报告 `MFSampleAllocatorUsage_UsesCustomAllocator`；
  - 用 `MFGetSystemTime()` 的 `VT_I8 PROPVARIANT` 发送 Source Started/Stopped；
  - 严格校验流选择、流 ID、Video 主类型和 NV12 子类型；
  - 同步内部 Presentation Descriptor 的 Select/Deselect；
  - 只允许合法的流状态转换。
- 新增 Media Foundation 直接首帧测试：完整读取 `1,382,400-byte` NV12。
- 新增系统注册设备测试：
  - 创建 Session 虚拟相机；
  - 从 symbolic link 重新激活系统设备；
  - `IMFSourceReader` 选择 NV12；
  - 成功读取完整首帧。
- `cargo test --all-targets` 三个测试程序集全部通过：
  `5 passed / 2 ignored`、`25 passed / 2 ignored`、`18 passed`。
- 系统注册相机 ignored 测试单独运行：`1 passed`。
- release EXE/DLL 构建通过。
- Windows Camera 占用旧 DLL 时先关闭 Camera，并通过一次 UAC 停止
  `FrameServer` 后完成替换。
- HKLM `InprocServer32` 指向
  `F:/Insta360onWin/LunaVirtualCamera.dll`，`ThreadingModel=Both`。
- 发布：
  - `LunaStudio.exe`：`9,221,120 bytes`
  - SHA-256：`6F5315B20BD42AC8CF66D58DF4259429EF8C69E50E41DC2DB778BF224F0F89EF`
  - `LunaVirtualCamera.dll`：`228,352 bytes`
  - SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`
- 根目录仍只有一个 EXE。
- 没有修改任何 Luna 相机协议，也没有新增调试 UI。
- 系统首帧验证使用本地占位帧；真实 Luna 画面仍需用户在应用内开启预览和虚拟摄像机后，
  重新打开 Windows Camera 做最终确认。
- 完整实现与验证步骤见 `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 169。
## Step 170 - 拍照与录像回归修复

- 重新核对本地 APK、`20260722_192838` PCAP 和当前代码。
- 确认日用快门包没有发错：
  - 拍照 `0x0003 / 30 03`
  - 开始录像 `0x0004 / 08 01`
  - 停止录像 `0x0005 / 10 01`
- 纠正分析中的临时误判：wire value `8` 不是普通录像；普通录像继续使用 APK 的 wire value `1`，
  context 使用成功录像 PCAP 中的 `0x07`。
- 修复 Step 168 回归：相机已经确认模式后，辅助状态查询超时不再推翻模式切换；
  socket 真断开时仍返回失败。
- 开始录像仍要求 Video context 成功，但只读 capture-ready 查询不再阻止发送正确录像包。
- 前端不再在预览命令刚返回时开始录像；必须等真实首帧绘制完成。
- 新增快门命令字节级回归测试。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`26 passed / 2 ignored`、`19 passed`。
- JavaScript 语法、Rust 格式、release 构建和 EXE 启动检查均通过。
- 发布：
  - `LunaStudio.exe`：`9,222,656 bytes`
  - SHA-256：`64C60D71C6716DBCA69A2C577ADFDA95D5705A8F2D8AC914AC7EDA6BEFA3F2B2`
- `LunaVirtualCamera.dll` 未修改。
- 根目录仍只有一个 EXE。
- 真实 Luna 拍照/录像仍需按 `CAPTURE_CONTROL_CONTINUE.md` Step 170 的顺序做最终确认。

## Step 171 - 用 UCD2 文件列表替换返回 403 的 HTTP 目录索引

- 用户现场显示内部存储 `/storage_internal/DCIM/` 与 SD 卡 `/DCIM/` 目录请求均返回
  `403 Forbidden`。
- 重新核对本地 PCAP：
  - 官方 HTTP 只 GET 具体媒体文件，不请求目录页。
  - 相册枚举使用 `command 0x000D / method 0x02`。
  - 内部第 1 页：`08 02 18 64 20 02`。
  - SD 第 1 页：`08 02 18 64 20 03`。
  - SD offset 100：`08 02 10 64 18 64 20 03`。
  - 类别 3 内部：`08 03 18 64 20 02`。
  - 响应 protobuf field 1 包含 `/storage_internal/DCIM/Camera01/...` 或
    `/DCIM/Camera01/...`。
- 从用户原始 PCAP 恢复了此前被 96-byte prefix 截断的完整能力请求：
  `0x0008 / request 0x80000003 / 183-byte body`。只在 F 盘生成临时解析，
  提取后删除；没有向 C 盘写入项目数据。
- `LunaAuthSession` 现在：
  - 等待真实设备信息成功响应后才报告认证完成；
  - 执行初始化、完整能力读取、登记、时间同步和状态读取；
  - 使用持续 worker 执行普通 UCD2 命令并保持 type-05 心跳；
  - 相机主动拒绝认证时立即停止，不连续重试已建立的 TCP 连接。
- 新增 `0x000D` 分页、protobuf 路径解析、内部/SD 存储识别、URL 编码和媒体元数据转换。
- `CameraControlSession` 也补上完整能力读取，并可直接复用控制连接列相册。
- `html_app` 的 `list_media` 不再请求 HTTP 目录；HTTP 仅处理具体文件预览、下载和缩略图。
- 没有新增任何调试 UI。
- 新增 UCD2 会话、文件列表请求和响应解析测试。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`28 passed / 3 ignored`、`21 passed / 1 ignored`。
- JavaScript 语法、Rust 格式和 release 构建通过。
- 两次只读真机验证都在认证/设备信息阶段被当前相机主动关闭，尚未发送文件列表命令；
  该边界已如实保留，后续需重启相机并关闭官方手机应用后验证。
- 发布：
  - `LunaStudio.exe`：`7,650,816 bytes`
  - SHA-256：`F51ADF9ADBB3D4B679A616F72D553198B8AA57D0340B35D2583592B1746FF998`
  - `LunaVirtualCamera.dll`：`228,352 bytes`，未修改
  - DLL SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`
- 启动检查显示 `Luna 控制台` 窗口响应正常并能正常关闭。
- 临时 PCAP 解析目录、旧 WebView2 运行缓存均已删除，根目录只有 `LunaStudio.exe`
  一个 EXE。
- 详细协议与下一次真机验证步骤见 `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 171。

## Step 172 - 新 PCAP 恢复普通录像、变焦和 1080p 48fps

- 解析用户新抓包 `PCAPdroid_28_7月_15_48_50.pcap`。
- 只读取 C 盘原始文件，所有输出写入
  `F:/Insta360onWin/reverse_apk/pcap_analysis/20260728_154850/`。
- 抓包有 `6,735` 个 UCD2 帧，控制 TCP 无重传、无解析缺口。
- 确认普通模式：
  - Photo：`0x0007 / 08 28 12 00`
  - Video：`0x0007 / 08 29 12 00`
- 确认 `0x2053` 模式事件的完整字段组合：
  - Photo：`08 00 10 64 18 03`
  - Video：`08 64 10 00 18 03`
- 修复旧代码把 `field1` 误当 `0/1` 模式枚举的问题。
- 确认变焦 `0x0009` 使用 option `0x35`、little-endian `f64` 与 context `06/07`；
  日用 UI 新增 1×、2×、3×、6×。
- 确认录像规格：
  - 1080p：`08 1f 12 03 f8 01 28 18 07`
  - 1080p 48fps：`08 1f 12 04 f8 01 84 02 18 07`
- 确认快门命令仍为：
  - 拍照 `0x0003 / 30 03`
  - 开始录像 `0x0004 / 08 01`
  - 停止录像 `0x0005 / 10 01`
- 修复录像事件误判：
  - `08 01 10 00 38 00` 是准备状态；
  - `08 01 10 00 38 02` 才是稳定录像；
  - `08 00 10 00 38 00` 是停止。
- 开始录像不再发送额外前置上下文/存储请求，也不再等待预览首帧。
- 拍摄页新增精简的变焦与组合录像规格控件，没有新增调试功能。
- 新增字节级回归测试；`cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`30 passed / 3 ignored`、`23 passed / 1 ignored`。
- Rust 格式、内联 JavaScript、release 构建和隐藏启动检查通过。
- 发布：
  - `LunaStudio.exe`：`7,671,808 bytes`
  - SHA-256：`7A200847EECDBC9D134E880154B6012A931758E743D42EDFE61FFC7D2AB4FAB7`
  - `LunaVirtualCamera.dll` 未修改：`228,352 bytes`
  - DLL SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`
- 关闭并清理了旧应用/FFmpeg 进程与 WebView2 运行缓存；项目根目录只有一个 EXE。
- 没有自动触发任何真机写操作。
- 完整时间线与证据见
  `reverse_apk/pcap_analysis/20260728_154850/CONTROL_FINDINGS.md`。
- 真机步骤见 `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 172。

## Step 173 - 从连接状态响应读取真实 Photo/Video

- 用户指出连接时应该由相机报告当前模式，不能硬编码 Photo。
- 重新读取新 PCAP 的完整状态响应：
  - 请求：`0x0008 / request 0x80000003`
  - 响应时间：`15:49:05.969`
  - internal payload：`1,171 bytes`
  - 响应 body 顶层 protobuf field 2 是嵌套完整状态。
- 嵌套状态偏移 `607` 为：

```text
c0 02 00 c8 02 64
field40=0
field41=100
```

- 用户确认该连接时相机处于 Photo，因此 `(0,100)=Photo` 是直接 PCAP 证据。
- 已捕获 `0x2053` 模式事件使用相同二值对：
  - Photo `(0,100)`
  - Video `(100,0)`
- 据相同字段结构，连接状态中的 `(100,0)` 映射为 Video。
- `CameraControlSession` 不再默认 `Some(Photo)`。
- 初始化完整状态响应不再丢弃；新增 length-delimited protobuf field 解析。
- 无法识别 field 40/41 时保持 `None`，不猜测模式。
- 连接 IPC 已有 `mode` 字段，前端会自动选中真实拍照/录像状态。
- 新增 `captured_full_status_reports_current_capture_mode` 回归测试。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`31 passed / 3 ignored`、`24 passed / 1 ignored`。
- release 构建与隐藏启动检查通过。
- 发布：
  - `LunaStudio.exe`：`7,673,856 bytes`
  - SHA-256：`AE7F0A7EE90F434D620C5557D15D7E536A17490524AA69F4528A38C75A91E8E6`
  - `LunaVirtualCamera.dll` 未修改：`228,352 bytes`
  - DLL SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`
- 关闭了正在运行的旧应用与 FFmpeg 预览子进程，清理 WebView2 缓存。
- 根目录只有一个 EXE，没有遗留项目进程。
- 完整说明与真机确认步骤见 `CAPTURE_CONTROL_CONTINUE.md` Step 173。

## Step 174 - 从 APK 恢复完整普通录像规格

- 用户纠正旧 UI：协议值 `40` 是 `1920x1080@60fps`，不能只显示为“1080p”，
  Luna Ultra 的清晰度、画幅和帧率也远不止两个档位。
- 只读取本地 APK 重建文件
  `reverse_apk/reconstructed_dex/seg02.dex`。
- `VideoResolution` 静态初始化器位于 `0x4cf304`，长度 `5299 code units`，
  构造 324 个枚举项，构造器为 `method@0x7d91`。
- 从构造参数恢复宽、高、帧率和设备协议值；PCAP 交叉确认：
  - `1920x1080@60 -> 40`
  - `1920x1080@48 -> 260`
- 接入用户列出的全部普通录像能力：
  - 10 种分辨率/画幅；
  - 64 个合法分辨率、画幅、帧率组合；
  - 8K、4K、3K、2.7K、1080p；
  - 16:9、2.35:1、1:1、9:16。
- `CameraVideoProfile` 改为数据结构，
  `CAMERA_VIDEO_FORMATS` 保存 APK 协议映射，
  `resolve_camera_video_profile` 作为后端白名单。
- IPC 不接受前端提交任意协议值，只接受 `{format, fps}`；
  非法组合、非录像模式和录像中修改都会被后端拒绝。
- 前端删除两个旧规格按钮，改为清晰度、画幅、帧率三个联动选择器。
- 真实 `1196x819` 窗口完成页面和滚动检查，控件没有横向溢出。
- 夜景、慢动作、延时和照片能力已记录，但尚未证明其完整子模式写入 body，
  所以没有用普通录像协议值伪造这些功能。
- UTF-8 无 BOM、CRLF、Rust 格式和内联 JavaScript 语法检查通过。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`32 passed / 3 ignored`、`25 passed / 1 ignored`。
- 新测试覆盖全部 64 组普通录像规格、代表性 APK 值、非法组合，以及
  PCAP 的 1080p60/48 字节一致性。
- release 构建、根目录启动和窗口响应检查通过。
- Mica 保持启用：
  - `DWMWA_SYSTEMBACKDROP_TYPE=2`
  - `DWMWA_USE_IMMERSIVE_DARK_MODE=1`
- 发布：
  - `LunaStudio.exe`：`7,686,656 bytes`
  - SHA-256：`7ED54580DDF2E893EE3CC57DF8DD45CE1A9BCEC7687C5464BF57F21AF38E59FF`
  - `LunaVirtualCamera.dll` 未修改：`228,352 bytes`
  - DLL SHA-256：`19AC8543047327AA20A9061B91CD8515778134016E094474C45643196FD1CD00`
- 删除启动检查生成的 WebView2 缓存，根目录只有一个 EXE，没有遗留项目进程。
- 没有自动连接相机或发送任何真机写操作。
- 完整协议值表、边界说明与真机步骤见
  `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 174。

## Step 175 - 修正变焦伪状态并加入设备读回

- 用户报告变焦数值与设备不一致。
- 仅复核本地 APK 和
  `reverse_apk/pcap_analysis/20260728_154850/ucd2_frames.json`。
- PCAP 证明 option `0x35` 的 1.0/2.0/3.0/6.0 fixed64 写包本身正确。
- 找到真实 bug：前端模式切换成功后硬编码 `cameraZoom=1`，没有发送或读取设备，
  并可能吞掉下一次真实 1× 点击。
- APK 字符串确认变焦使用 `ZoomScaleSetting`、`ZoomScaleRangeSegment`、
  `ZoomModel`、`ZoomState`、quick tabs 和 major ticks；四个抓包点不是完整枚举证明。
- 从 PCAP 恢复 command `0x000a` 的 83 项、168 字节拍摄设置查询。
- Photo 使用 context `0x06`，Video 使用 context `0x07`。
- 相机响应顶层 field 2 内的 field 53/wire type 1 是当前变焦 fixed64；
  抓包返回 1.0。
- `CameraControlSession` 新增 `zoom: Option<f64>`。
- 连接后读取真实变焦；不再默认或猜测。
- 设置变焦后必须再查询并解析设备实际值，成功后才同步会话和 UI。
- 模式切换完成后按官方抓包顺序真正发送 1×，再读取确认。
- 写入仍只开放 PCAP 确认的 1×、2×、3×、6×，没有伪造任意小数档位。
- IPC 的 connect/mode/zoom 响应均返回实际 `zoom`。
- 前端删除硬编码 1×，新增当前值显示；四个按钮标为“快捷变焦”。
- 新测试逐字节验证 168 字节查询，并覆盖 1.0、2.5、NaN 与缺失 field 53。
- Rust 格式与内联 JavaScript 语法检查通过。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`33 passed / 3 ignored`、`26 passed / 1 ignored`。
- release 构建和根目录启动检查通过。
- 真实 `1196x819` 拍摄页无横向溢出：
  - `target/step175-ui-printwindow.png`
  - `target/step175-control.png`
- Mica 检查：
  - `DWMWA_SYSTEMBACKDROP_TYPE=2`
  - `DWMWA_USE_IMMERSIVE_DARK_MODE=1`
- 发布：
  - `LunaStudio.exe`：`7,693,824 bytes`
  - SHA-256：`CB8E2FFA91168D207DB38BB9B823B61D850A3F9030B8E047E0B1D73674D9BEDE`
  - `LunaVirtualCamera.dll` 未修改。
- 关闭发布检查进程并删除 WebView2 缓存；根目录只有一个 EXE。
- 没有连接相机或发送任何真机写操作。
- 完整证据、实现和真机步骤见
  `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 175。

## Step 176 - Luna Ultra 水印按 APK 目录重构（进行中）

- 从三份 APK 参数表确认中文/标准是两种样式，不应把 image/video 资源拆成独立选项。
- 确认照片普通水印固定底部居中，视频普通水印有五个位置。
- 确认 Luna Ultra 另有 `ZStyle` / `ZStyle-CN` 外框水印，覆盖 15 个照片比例。
- 后端目录已收紧为四项：中文、标准、外框水印（中文）、外框水印。
- 删除前端自定义大小、透明度和 GO Ultra 等无关样式逻辑。
- 修复前端仍绑定已删除控件导致的 JavaScript 中断。
- 修正普通水印纵坐标为 APK 的底部距离语义。
- 外框 Logo 宽度改为按原照片宽度计算，不再按扩展画布宽度放大。
- 旧入口移除已删除字段，等待格式化、全目标测试和发布构建。
- 外框的拍摄参数/时间戳文字层尚无足够 APK 绘制证据，未冒充已完成。

### Step 176 完成：预览改为真实渲染

- 修复配置解析：比例键包含冒号，参数值必须从最后一个冒号后读取；普通水印和外框参数测试均通过。
- 普通水印与外框水印现在有共用内存合成函数，导出和预览不再维护两套布局。
- 新增 `watermark_preview` IPC：
  - 照片直接按当前 APK 样式合成。
  - 视频使用内置 FFmpeg 抽取真实画面，再按视频位置参数合成。
  - 返回最大 `900 x 650` 的 JPEG/Base64 供 WebView 显示。
- 前端彻底删除 CSS/文字假预览，加入自动真实渲染、160 ms 防抖、加载状态和过期响应丢弃。
- 视频素材会禁用外框水印并自动回到普通样式；照片仍使用 App 固定位置。
- 新增预览编码回归测试；内联 JavaScript 语法检查通过。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`42 passed / 3 ignored`、`35 passed / 1 ignored`。
- 发布版实际 UI 验证通过：
  - `target/step176-watermark-real-empty.png`
  - `target/step176-watermark-real-preview.png`
  - `target/step176-watermark-real-frame-preview.png`
  - `target/step176-watermark-real-video-preview.png`
- 发布：
  - `LunaStudio.exe`：`7,810,048 bytes`
  - SHA-256：`C4AC4F6BA2CE28996647D98D47302E00BD0B3BFDFFA9D574AB0D8396C1108D2E`
- 验证后关闭全部项目进程；未连接相机或发送真机写操作。
- 完整实现与证据边界见 `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 176。

## Step 177 - 外框水印按 Luna Ultra 直出图完成

- 用户提供权威直出样本 `3781 x 4209`；确认右下角没有署名，应用不得生成署名。
- 样本中的照片区域为 `3520 x 2644 @ (130,695)`。
- 精确记录顶部 Logo、`Luna Moment`、拍摄参数与时间戳四个像素包围盒；详见
  `reverse_apk/CAPTURE_CONTROL_CONTINUE.md` Step 177。
- 直出样本证明 `Frame_Watermark_Config_Table.txt` 的照片底距、顶部 Logo 宽度和
  `Moment` 宽度均以最终相框画布为基准。
- 修复外框几何基准、画布取整和底色，4:3 样本现在可得到精确 `3781 x 4209` 画布。
- 新增 `assets/apk_watermark/choose_logo_photo_moment.png`，从用户直出样本无损裁取，
  与本地 DEX 的同名资源证据对应，不包含署名。
- 新增 EXIF 读取与直出格式化：光圈、快门、ISO、拍摄时间、UTC 偏移。
- 元数据使用 Bahnschrift 绘制并按样本横向校准 `1.16`；预览和导出使用同一合成器。
- 缺少 EXIF 时不伪造参数，只保留品牌与 `Luna Moment`。
- QA 图：`target/step177-synthetic-frame.png`。
- 归一化像素校验：`Moment 341/341`、参数 `278/约275`、时间 `403/约400`，
  纵向误差约 `1-2px`。
- `cargo fmt --all -- --check` 通过。
- `cargo test --all-targets` 全部通过：
  `5 passed / 2 ignored`、`44 passed / 3 ignored`、`37 passed / 1 ignored`。
- 根目录 Release 启动冒烟检查通过并已关闭；未连接相机或发送真机写操作。
- 发布：
  - `LunaStudio.exe`：`7,992,320 bytes`
  - SHA-256：`97ABDF8C525F456088F191588D3D3A31E154E8511F06778BD2B5881EB3BA977C`

## Step 178 - 外框背景色与首张参考图格式（完成）

- 用户明确以第一张参考图的外框样式为准，并要求提供 App 内的多种背景颜色。
- 修正格式目标：`F2.0`、`19-Jun-2026 12:59 UTC+08:00`；不生成右下角署名。
- 本地 APK DEX 证据表明 `PhotoFrameColor` 同时保存 `startHex/endHex`，
  并有可用颜色、自动背景和黑色前景资源，不是单一硬编码黑底。
- 已实现黑色、白色、照片深色、照片浅色和照片渐变五个日用选项；
  照片色从源图采样，不添加与 App 无关的自由取色器。
- 预览和导出共用背景合成器，并按亮度自动切换 Logo、Moment 与参数文字前景色。
- HTML 已加入色板控件，仅对照片外框水印显示。
- 真实像素 QA：
  `target/step178-frame-black.png`、
  `target/step178-frame-white.png`、
  `target/step178-frame-gradient.png`。
- 白底模式的外围品牌字样、Moment 和参数已切换为深色；Leica 红章及章内原色保留。
- `cargo fmt --all -- --check` 与内联 JavaScript `node --check` 通过。
- `cargo test --all-targets`：
  `5 passed / 2 ignored`、`48 passed / 3 ignored`、`41 passed / 1 ignored`。
- 发布：
  - `LunaStudio.exe`：`8,029,184 bytes`
  - SHA-256：`4D02641692F0EBFE2E6A95F6F762392757C88AB856C8CBED803BDD15F272FC57`
- 隐藏启动进入进程启动路径；关闭阶段出现一次 Windows 句柄访问拒绝，
  最终复查无项目进程残留。未连接相机或发送真机写操作。

## Step 179 - 原片 EXIF 快门校正与外框字体复核（完成）

- 用户明确 `IMG_20260626_164557_021.jpg` 是无水印相机直出原片，不能用作水印版式参考。
- 原片 EXIF：`ExposureTime=1211/1000000`、`ShutterSpeedValue=9689/1000 APEX`；
  数学倒数约为 `1/826`，用户确认相机档位显示应为 `1/800`。
- `src/adapters/watermark.rs` 改为把亚秒曝光吸附到标准快门分母，
  `ExposureTime` 缺失时才回退到 APEX 快门值；已补对应单元测试。
- Bahnschrift 是错误近似；`LeagueGothic-Regular.otf` 的真实渲染也明显过窄，
  反查后确认它属于浮动文字模块，不是照片外框参数字，已撤除。
- `libarvbmg.so` 的 OpenCV `putText/getTextSize` 调用全部属于 AI/调试绘制，
  `PrintWaterMark::Process` 本身不画文字，故 Hershey 假设被排除。
- 当前使用按用户第一张水印参考图逐字形拟合的 OFL 方角 SemiBold 字体，
  明确记录为图像拟合替代，不写成 APK 内置字体。真实原片 QA
  `target/step179-original-frame.png` 已输出 `F2.0  1/800  ISO161`。
- 清理错误字体和临时候选后，`cargo fmt --all -- --check` 通过；
  `cargo test --all-targets` 结果为 `5/2 ignored`、`48/4 ignored`、`41/2 ignored`。
- 发布 `LunaStudio.exe`：`8,090,112 bytes`，SHA-256
  `611018F24B892541C8C94BC8556CDAE4A0EE32BEC2235BA40FEAB65F33205333`。
- 发布后隐藏启动冒烟被 Windows 拒绝访问；复查无 `LunaStudio/html_app` 进程残留，
  不将该项记为通过。
- 未连接相机或发送真机写操作。
