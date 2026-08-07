# Ashield unpack notes

## APK container

- `reverse_apk/classes.dex` is a tiny valid shell DEX followed by an Ashield payload.
- The appended container starts at offset `6372` with magic `adx0`.
- Header:
  - magic: `61 64 78 30`
  - payload size: `120240987`
  - metadata length: `396`
  - body offset: `6780`
- Metadata bytes are XORed with `0x52`. Decoded metadata:
  - `activityName`: `com.arashivision.insta360akiko.app.SplashActivity`
  - `appName`: `com.arashivision.insta360akiko.app.AkikoApplication`
  - `fastLevel`: `1`
  - `jiaguVersion`: `1.5.1.3`
  - `ls`: `6372`
  - `opt`: `1`
  - `pkg`: `com.arashivision.insta360akiko`
  - `stubAppName`: `com/ashield/Stub`
  - `versionCode`: `418696`
  - `versionName`: `2.28.0`
- Body begins with little-endian count `17`.

## Native registration

`JNI_OnLoad` decodes `com/ashield/Stub` and uses two native bridge/registration paths.

### Static `RegisterNatives` path

`0xa02a0` calls JNIEnv `RegisterNatives` with table `0xfce80`, count `1`.

| Method | Signature | Function |
| --- | --- | --- |
| `ashieldStub4` | `(Landroid/content/res/AssetManager;Ljava/lang/String;Ljava/lang/String;)I` | `0xa0374` |

`ashieldStub4` handles `assetsProtect` / `.ashield_assets` / `assets/ashield.png`.

### Runtime-filled five-entry table

`0xe900` calls `0x19984` first, then calls helper `0xff7c` with table `0xf9190`, count `5`.

`0xf9190` is a five-row table whose name/signature pointers point to runtime output buffers populated by decoder logic around `0x197c4`.

| Method | Signature | Function |
| --- | --- | --- |
| `ashieldStub1` | `(Landroid/app/Application;Landroid/content/Context;)Z` | `0x28424` |
| `ashieldStub2` | `(Landroid/app/Application;Landroid/content/Context;)V` | `0x19f50` |
| `ashieldStub3` | `(Landroid/app/Application;)V` | `0x19f18` |
| `ashieldStub5` | `(Landroid/content/Context;)Landroid/app/Application;` | `0x1d748` |
| `ashieldStub7` | `(Landroid/content/Context;)V` | `0x9be60` |

The first three names are directly decoded from `0x197c4`; the last two align with the stub DEX signatures and the function pointer table.

## String decoders

### Decoder at `0xe660`

Pointer adjustment:

```text
real = obfuscated + 0xffffffff9fde43f6
```

Byte transform:

```text
dst[i] = (((src[i] - 0x69) ^ 0xad) - i - key) & 0xff
```

Confirmed examples:

- `com/ashield/Stub`
- `dalvik/system/DexFile`
- `getClassNameList`
- `ashieldStub1`
- `ashieldStub2`
- `ashieldStub3`
- `ashieldStub5`

### Decoder at `0x1a204`

Pointer adjustment:

```text
real = obfuscated + 0xffffffff9b6ea500
```

Byte transform:

```text
dst[i] = (((src[i] + 0x94) ^ i) - key) & 0xff
```

Confirmed examples:

- `android/app/Application`
- `android/app/ActivityThread`
- `mBoundApplication`
- `mPackageInfo`
- `mApplication`
- `appName`
- `java/lang/Class`
- `getClassLoader`
- `java/lang/ClassLoader`
- `loadClass`
- `newInstance`
- `com/android/dex/Dex`
- `getDex`
- `customization`
- `applicationInfo`
- `getPackageCodePath`
- `sourceDir`
- `classes.dex`
- `java/nio/ByteBuffer`
- `wrap`
- `dalvik/system/InMemoryDexClassLoader`
- `dalvik/system/BaseDexClassLoader`
- `getPackageCodePath`
- `getDex`

## `ashieldStub5` flow

`ashieldStub5` at `0x1d748`:

1. Reads `appName` from decoded metadata.
2. Uses `ClassLoader.loadClass(appName)` and `newInstance`.
3. If the real application class is not already available, it falls back to the DEX recovery path.
4. The fallback path calls into code around `0x1e560`, which works with `com/android/dex/Dex.getDex()`.

The APK path recovery path uses:

- `customization`
- `applicationInfo`
- `getPackageCodePath`
- `sourceDir`

This means the shell opens the current APK/classes payload directly; no network capture is required for this part.

## DEX injection checkpoint

Decoded native strings show that the shell supports both new and old DEX loading paths:

- `0x306ec` uses `java/nio/ByteBuffer.wrap`, `dalvik/system/InMemoryDexClassLoader`, `java/lang/Class.getClassLoader`, `com/ashield/Stub`, and `dalvik/system/BaseDexClassLoader`.
- `0x32150` and `0x35a94` use the older `BaseDexClassLoader` / `DexPathList` / `dexElements` / `DexFile.loadDex` compatibility path.
- Clear rodata around `0xce988..0xcec20` contains `pathList`, `DexPathList`, `dexElements`, `DexFile`, `loadDex`, `/tmp.dex`, `/odex.dex`, and `mCookie`.

Current recovery target: track the reconstructed DEX byte array source inside `0x1e560`, then follow its arguments into `0x306ec` or the older BaseDexClassLoader compatibility path.

## File/decompression core

ELF imports used by the unpacking path:

- `open` via PLT `0xdc30`
- `read` via PLT `0xdd10`
- `lseek64` via PLT `0xdbd0`
- `mmap` via PLT `0xdf80`
- `inflateInit2_` via PLT `0xdcd0`
- `inflate` via PLT `0xdc50`
- `inflateEnd` via PLT `0xdf20`

Important native regions:

- `0x4bdf8..0x4c650`: opens APK path, gets file size, stores file descriptor and path copy.
- `0x4c9ec..0x4d08c`: reads from the opened file, checks first bytes, seeks relative to file size.
- `0x4e044..0x4eb40`: parses a per-section record; reads 30-byte header-like data from the APK/payload.
- `0x4f544..0x4f930`: zlib raw inflate path (`inflateInit2_` with window bits `-15`).

Current static conclusion: the `adx0` body is a 17-section Ashield container. It is not a plain DEX/ZIP/GZIP stream. The native unpacker locates the APK path, parses the appended body, and inflates section data after a shell-specific header/decryption stage.

## DEX map-list scan

A byte-level scan of the `adx0` body found `17` plausible DEX `map_list` structures. This matches the body count at offset `body + 0`.

The map lists are standard-looking DEX map tables, but the data around the inferred `map_off` is not a complete DEX header. In several cases the inferred start points into string data or code/debug data. Current interpretation: Ashield stores DEX sections in a transformed/reordered layout. The DEX `map_list` survives in cleartext, while the original header/id tables or section placement still require native unpacker reconstruction.

Detected map list body-relative offsets:

- `0x008bd1bf`
- `0x010778d9`
- `0x019db1c1`
- `0x020b652b`
- `0x02577e65`
- `0x02bb4a2b`
- `0x0312e1d9`
- `0x037032b9`
- `0x03c3c703`
- `0x04230d5d`
- `0x0482ff6b`
- `0x04fa1095`
- `0x056641cf`
- `0x05de356a`
- `0x06474234`
- `0x06b571f8`
- `0x072ab8ef`

The last map list at body-relative `0x072ab8ef` decodes to:

```text
0000: size=1     off=0x000000
0001: size=67757 off=0x000070
0002: size=13064 off=0x042324
0003: size=15838 off=0x04ef44
0004: size=35576 off=0x07d5ac
0005: size=65370 off=0x0c2d6c
0006: size=6731  off=0x14283c
2001: size=49503 off=0x17719c
2003: size=1134  off=0x56ddc8
1001: size=10510 off=0x5733c4
2002: size=67757 off=0x59185c
2004: size=12975 off=0x8ba925
2000: size=6388  off=0x94a815
2005: size=205   off=0x9b7460
1003: size=9670  off=0x9bf144
1002: size=389   off=0x9df748
2006: size=6026  off=0x9e23f8
1000: size=1     off=0xa2c6d8
```

## APK-only UCD2 evidence

Clear strings recovered from the protected DEX payload include:

- `UCD2-ENCRYPT`
- `UCD2-XOR-KEY-001`
- `UCD2.0 don't need sync-packet`
- `ucd2`
- `ucd2Magic`
- `ucd2Sequence`
- `isUseUcd2FromSsid`
- `INS_UCDTYPE_FIRST`
- `INS_UCDTYPE_HEARTBEAT`
- `INS_UCDTYPE_MSG`
- `INS_UCDTYPE_STREAM`
- `INS_UCDTYPE_SYNC`
- `INS_UCDTYPE_SYNC_MEDIA_TIME`
- `INS_UCDTYPE_TUNNEL`
- `INS_UCDTYPE_WIFIPROXY`

Command/message architecture evidence:

- `Segment 10` contains camera command wrappers and OneDriver execution signatures:
  - `(Lcom/arashivision/onecamera/OneDriver;J[Lcom/arashivision/camera/command/InstaCmdExe;)V`
  - `Lcom/arashivision/camera/command/CheckAuthorizationCmd;`
  - `Lcom/arashivision/camera/command/GetFileListCmd;`
  - `Lcom/arashivision/camera/command/CaptureCommand;`
  - `Lcom/arashivision/camera/command/StartRecordCmd;`
  - `Lcom/arashivision/camera/command/StopRecordCmd;`
  - `Lcom/arashivision/camera/command/StopTakePictureCommand;`
- Those wrappers reference protobuf/Wire classes:
  - `Linsta360/messages/CheckAuthorization;`
  - `Linsta360/messages/GetFileList;`
  - `Linsta360/messages/TakePicture;`
  - `Linsta360/messages/StopCapture;`
- `Segment 5` contains the actual `insta360/messages` classes. Important constructor descriptors:
  - `CheckAuthorization`: `(Ljava/lang/String;Linsta360/messages/CheckAuthorization$InitiatorType;Ljava/lang/String;Lokio/ByteString;)V`
  - `CheckAuthorizationResp`: `(Linsta360/messages/CheckAuthorizationResp$AuthorizationStatus;Linsta360/messages/CheckAuthorizationResp$FindmyPairStatus;ILjava/lang/String;Lokio/ByteString;)V`
  - `TakePicture`: `(Linsta360/messages/TakePicture$Mode;Linsta360/messages/ExtraMetadata;Ljava/util/List;Linsta360/messages/RawCaptureType;ZLinsta360/messages/SensorDevice;ILinsta360/messages/TriggerSource;Lokio/ByteString;)V`
- `Segment 11` contains the packet/encryption layer:
  - `Lcom/arashivision/onedriver/encrypt/AesGcmResult;`
  - `Lcom/arashivision/onedriver/encrypt/EcdhKeyPair;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptResult;`
  - `Lcom/arashivision/onedriver/encrypt/EncryptionManager;`
  - `Lcom/arashivision/onedriver/packet/PacketDecryptParams;`
  - `Lcom/arashivision/onedriver/packet/PacketEncryptResult;`
  - `Lcom/arashivision/onedriver/packet/PacketEncryptionParams;`

Current control-path model:

```text
BaseCamera/Controller
  -> OneDriver
  -> InstaCmdExe[]
  -> com/arashivision/camera/command/*Cmd
  -> insta360/messages/* protobuf payload
  -> com/arashivision/onedriver/packet/Packet
  -> UCD2 transport/encryption
```

z03 config also confirms:

- `device_type`: `Insta360 Luna Ultra`
- `display_name`: `Insta360 Luna Ultra`
- `product_code`: `BT`
- `device_series`: `Luna`
- `device_variety`: `camera`
- `ucd2`: `true`
- `connection_channels`: `wifi`, `usb`, `bluetooth`

## UCD2 frame checksum status

Observed UCD2 frame tails do not match standard CRC32, CRC32C, or xxHash32 over `header + payload` with common seeds.

Tested examples:

- `55 43 44 32 01 0c 05 0f 00 00 00 00 37 05 47 7c`
- `55 43 44 32 01 0c 05 01 00 00 00 00 11 28 34 b2`
- `55 43 44 32 01 0c 05 02 00 00 00 00 f6 7b 41 8a`
- `55 43 44 32 01 0c 04 10 0f 00 00 00 08 00 02 01 00 00 80 00 00 08 30 08 0f 08 0b 7c 00 8e 7c`

Current conclusion: generating new UCD2 commands needs the APK's custom checksum/encryption path, not only replayed payload bytes.

### Packet / EncryptionManager checkpoint

APK-only evidence from segment 11 confirms packet encryption support for NONE, XOR, and AES_GCM_128. Important classes: `Packet`, `Packet$Companion`, `PacketEncryptResult`, `PacketEncryptionParams`, `PacketDecryptParams`, `Message`, `MessageMuxer`, `MessageDemuxer`, `EncryptionManager`, `PlatformCrypto`, `AesGcmResult`, `EcdhKeyPair`. Important strings: `parseEncryptOption`, `fromBytes`, `decrypt[scheme=`, `nonceHex`, `tagHex`, `aadHex`, `UCD2-XOR-KEY-001`.

Segment 5 confirms protobuf negotiation messages: `EncryptCapabilityQuery/Resp`, `EncryptKeyExchangeReq/Resp`, `EncryptScheme`, `EncryptDevType`, `EncryptErrorCode`.
### Ashield native unpack checkpoint

Decoded `adx0` metadata uses XOR `0x52`. `jiaguVersion=1.5.1.3`, `ls=6372`, business body starts at `0x1a7c`, and first body u32 is `0x11` sections. `libashield.so` is ELF64 AArch64 with `JNI_OnLoad` at `0xe300`; native unpack cluster is around `0x4b000..0x4f930`. Raw deflate confirmed: `0x4f778 -> inflateInit2_` with `w1=-15`, `0x4f7f8 -> inflate`, `0x4f8c8 -> inflateEnd`. Added reusable scanner: `reverse_apk/tools/ashield_native_scan.py`.

## Index-table observation

The `adx0` body begins with count `0x11`. The first u32 after the count is `0x008bd293`, while the first confirmed clear DEX map-list is at body-relative `0x008bd1bf`. Difference is `0xd4`, which lands inside the 18-item map-list record area. This strongly suggests the body prefix is an encrypted/encoded 17-entry index table that points into DEX map/list or section metadata, not random padding.

Using the `0x1000` map item offset as a raw DEX map offset does not immediately yield valid DEX starts/string_ids in the body, so the body layout is not simple concatenated DEX files. A native transform/reassembly step remains necessary.

## Loader / VM checkpoint

`0x880c0` is a confirmed `adx0` loader entry. It decodes `apk@classes.dex`, obtains a candidate pointer, validates the `adx0` 12-byte header/body relation, decodes `ls`, and compares against the pointer-derived offset. The APK metadata value `ls=6372` equals the `adx0` offset in the protected `classes.dex`.

On successful validation, `0x880c0 -> 0x8b9cc -> 0x2a1e0`. `0x2a1e0` seeds a `0x250`-byte context from `.data.rel.ro` around `0xf5350..0xf53c0`, then calls `0x2a328(0x250, 0xcf570, context_ptr)`.

`0x2a328` is a runner for a custom VM, and `0x2a57c` is its dispatcher. The VM reads `u32` instructions from `0xcf570`, derives fields such as `(insn >> 10) & 0x3f` and `(insn >> 21) & 0x3f`, manipulates 64-bit context slots, and can call function pointers stored in the context. This VM likely constructs the internal parser/table needed for Ashield's protected DEX reassembly.

Added helper: `reverse_apk/tools/ashield_vm_trace.py`. It prints seed context slots from `0xf5350`, bytecode fields from `0xcf570`, and opcode-pair counts. First run shows seed slots are not plain ELF pointers and direct ARM64 disassembly of `0xcf570` is incoherent, confirming a custom VM rather than normal code in `.rodata`.

Direct DEX reconstruction checkpoint: all 17 exposed positions parse as valid DEX-style `map_list` records, but using `dex_base = map_bodyrel - map_off` and writing a synthetic DEX header does not produce valid DEX files. Segment 11's reconstructed `string_id` offset contains ASCII string data, not a string offset table. The protected body is therefore rearranged/stripped beyond a missing-header problem. Failed experiment artifacts are in `reverse_apk/reconstructed_dex/segXX.dex`.

## Unicorn emulation checkpoint

Tool:

- `reverse_apk/tools/ashield_vm_emulate.py`

Purpose:

- Emulate the `0x2a328` Ashield VM path with the protected `adx0` body mapped in memory.
- Hook native/libc boundaries so the unpacker can run far enough to reveal protected Packet/Encryption data.

Confirmed PLT map:

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

Important correction:

- Do not use the older note that put `memmove` at `0xe020` or `memcmp` at `0xdf60`; those are incorrect for this binary's PLT order.

Current hooks:

- Native alloc/free, libc memory/string functions, threading noops/inline thread execution, mmap failure/success simulation.
- Temporary `0x3e1f8` body-check hook returns `1` to advance the path. This is a bypass, not a recovered algorithm.

Observed target coverage:

- The VM allocates and copies a large chunk:
  - destination `0x51981980`
  - size `0x6766307`
  - source `0x7310adcb`
- Heap dump before the fast PRGA hook contains:
  - `EncryptionManager` at `+0x43efb6`
  - `Packet` at `+0x43f02f`
  - `UCD2-XOR-KEY-001` at `+0x51113d0`

This is the strongest checkpoint so far: the native VM path has reached the actual Packet/Encryption protected material needed for UCD2 command generation.

## RC4-style loop checkpoint

The slow loop at `0x4a9a0..0x4ac54` is an RC4-style PRGA over a 256-byte state table. It is flattened; practical hook point is `0x4aae8` / `0x4aaec`, not only the apparent entry around `0x4a99c`.

Stack locals at the hook point:

- `[sp+0x50]` state/S-box pointer
- `[sp+0x48]` data pointer
- `[sp+0x40]` total length
- `[sp+0x28]` current offset
- `[sp+0x3c]` PRGA `i`
- `[sp+0x38]` PRGA `j`

The current Python fast hook processes the rest of the buffer and jumps to `0x4ac54`.

Current blocker:

- After PRGA, first worker attempts `mmap` with length `0xf99e9efd`; second worker attempts length `0xd18e2f68`.
- These lengths look invalid, so either the PRGA hook is not exactly equivalent yet, or the post-PRGA parser is reading a field that still depends on another missing native side effect.
- Third inline worker then crashes reading `[0x144c1e041]`.

Next work:

- Validate the fast PRGA hook against a few native iterations.
- Trace where `[sp+0x224]` is set before `0x28e78 -> 0x7c70c`; this value becomes the mmap length.
- Keep pre-PRGA heap dumps; they still contain the confirmed Packet/Encryption string material.

Update:

- `--rc4-native-prefix 32` validated the Python PRGA hook against native execution:
  - `state_ok=True`
  - `data_ok=True`
  - `ij_ok=True`
- The reader/mmap failure is therefore not explained by the Python PRGA implementation.
- `--native-body-check` now runs the real `0x3e1f8` path far enough to preserve the scanner after the XOR loop:
  - hook `0x3db5c` returns `0`;
  - hook `0x3e2a4` bulk-applies XOR `0x52` and resumes at `0x3e2e8`.
- This produces the same worker descriptors and same bad mmap length as the older full `0x3e1f8` bypass.

## Worker descriptor / chunk parser checkpoint

`reverse_apk/tools/ashield_vm_emulate.py` now has `--trace-threads`.

First worker descriptor:

```text
arg=0x5098b6a0
out=0x50000020
src=0x7310adc3
bodyrel=0x10adc3
src_len=0x00150208
data_u32=0x06766307
```

Actual call:

```text
0x2a0bc -> 0x2848c(
  data_ptr = 0x7310adc7,
  src_len = 0x00150208,
  scratch = x29 - 0x30,
  out_ctx = 0x50000020
)
```

Inside `0x2848c`, `0x2905c` reads the first u32 at `data_ptr`, so `consumed = 0x06766307`. The parser then advances `data_ptr` by 4 to `0x7310adcb`.

Observed mode before branch:

```text
mode=0x00000000
orig=0x00150208
consumed=0x06766307
data_ptr=0x7310adcb
```

Because `mode != 2`, `0x2848c` takes the reader path and computes:

```text
remaining = orig_len - 4 - consumed
total = reader_ret + remaining
```

For the first worker:

```text
orig=0x00150208 consumed=0x06766307 reader_ret=0x00000000 remaining=0xf99e9efd total=0xf99e9efd
```

Current interpretation:

- The bad length is caused by `0x2848c` taking the reader path with `mode=0`.
- The simple path at `0x289f0` is selected only when `[sp+0x200] == 2`; it uses `orig_len - 4` instead.
- Hooks for `atoi`, `strtoul`, and `strtol` were added, but `atoi` is not hit on this path, so the missing piece is likely handler/object matching before `0x289b8`.
- Next target is the handler selection path that sets `[sp+0x200]`, especially around `0x28500`, `0x287a0 -> 0x15270`, `0x287ec..0x28980`, and `0x49ab8`.

## Latest emulator correction

The older note saying `hook 0x3db5c returns 0` is obsolete.

Current `--native-body-check` behavior:

- `0x3db5c` runs natively.
- `0x3dcd4` is fast-hooked for the rolling/checksum loop.
- `0x3e2a4` is fast-hooked for the XOR `0x52` loop.

Observed first precheck state:

```text
rolling=a3 27 87 2d 52 63 b5 b1 db d4 b0 02 f5 52 c1 6d
acc=0x0010ade9
```

Added `--trace-parent` to `reverse_apk/tools/ashield_vm_emulate.py`.

Parent-side worker args:

```text
0x50a96460 -> out=0x50000020 src=0x7310adc3 bodyrel=0x10adc3
0x50a96470 -> out=0x50000020 src=0x7325afcf bodyrel=0x25afcf
```

The later `src=0x144c1e041` crash pointer is downstream of earlier worker failures, not a clean original source record.

Handler update:

- `out_ctx+0x50` is zero-filled in the current emulator run.
- `0x40900` uses `out_ctx+0x50`, returns an empty local string, and compares it against an intentionally empty `.rodata` string at `0xd3ff2`.
- That empty-string match skips the `atoi` fallback, leaves `mode=0`, and causes the bad reader/length path.

Current root target:

- Recover why `out_ctx+0x50` is empty in the emulator.
- Relevant code areas: `0x3db5c`, `0x3e174`, `0x40900`, `0xa0fd8`, `0x467e4`, and `0x414c8`.

Out-context write trace:

- Added `--trace-outctx-writes`.
- `out_ctx+0x50` is initialized to zero by native code at `0x497e4` / `0x497ec`.
- No later native write populates `out_ctx+0x50` before worker `0x2848c`.
- `0x3db5c` only writes/initializes the rolling/checksum area around `out_ctx+0x0a` and `out_ctx+0x1c`.
- Next concrete target: reverse constructor/setup functions `0x4977c..0x49a7c`, `0x3dfdc`, and the runtime-decoded `.bss` strings at `0xfdb48`, `0xfdb51`, `0xfdd81`.

## Forced mode experiment

Experimental emulator switches were added:

- `--force-chunk-mode <n>`
- `--normalize-huge-src-len`

Fast hook added:

- `0x290d4(dst, src, len)` behaves as a flattened copy/memmove state machine and is now fast-hooked.

Results:

- First worker with forced `mode=2` mmaps/copies `0x150204` bytes from `0x7310adcb`.
- Second worker has huge apparent `src_len=0xd19c306e` but plausible next length `0x000e0102`; with normalization it mmaps/copies `0xe00fe` bytes from `0x7325afd7`.
- Dumps are under `reverse_apk/vm_heap_dumps/force_mode2_normalized`.

Caveat:

- This is an unpacking experiment, not final protocol evidence.
- Final dumped mmap fragments do not yet contain `dex\n`, `Packet`, `EncryptionManager`, or `UCD2-XOR-KEY-001`.
- Third worker still reaches bad `src=0x144c1e041`, so the parent/worker result chain remains the next blocker.
