# Luna Studio Development Log

This file records repository-level operations so another model or developer can continue the work without reconstructing the local state.

## 2026-08-07 - GitHub source repository preparation

- Confirmed the active project root is `F:\Insta360onWin`; no project files were created or modified on the C drive.
- Confirmed the directory was not previously initialized as a Git repository and had no remote.
- Audited the project tree, large files, nested repositories, build outputs and common secret patterns.
- Found no API keys, access tokens or private keys in `src`, `web` or `data` using the repository scan patterns.
- Expanded `.gitignore` to exclude Rust build directories, packaged EXE/DLL files, WebView2 runtime data, gallery thumbnail cache and editor/OS cache files.
- Excluded the bundled `assets/ffmpeg/ffmpeg.exe` binary and added `assets/ffmpeg/README.md` documenting the expected runtime path.
- Excluded generated or extracted APK artifacts including DEX/SO files, heap dumps, reconstructed DEX files, validation outputs and nested reference repositories.
- Kept application source, the HTML interface, watermark resources, `data/profiles.json`, protocol analysis scripts, compact PCAP findings and handoff documents in the source set.
- Added `.gitattributes` to make text and binary handling deterministic across Windows and macOS development machines.
- Created the private GitHub repository under the authenticated account. The initially created `Insta360onWin` repository was renamed to `Insta360Linker` before the first push at the user's request.
- Initialized a local Git repository with `main` as the default branch and enabled CRLF working-tree normalization.
- Staged 95 source/resource/document files totaling approximately 3.68 MB; no staged file approaches GitHub's per-file size limit.
- Re-ran the staged credential scan; no GitHub token, private-key, API-key or access-token patterns were found.
- Ran `cargo check --locked --bin html_app`; it completed successfully with existing dead-code warnings only.
- Final GitHub repository: `https://github.com/wuhaiqi0621/Insta360Linker`.
- Added the GitHub repository as the local `origin`, pushed the initial source import commit `db60341` to `main`, and configured the local branch to track `origin/main`.

## 2026-08-07 - Outer-frame watermark font correction

- Imported the user-provided `G:\备份\字体\BeihaibeiSC-Regular.ttf` into `assets/apk_watermark/BeihaibeiSC-Regular.ttf` without modifying the source file.
- Verified the copied font with SHA-256 `1240EC12D92EF9A30DF6BC473BF37D8F213A6E6ABF1C4CE132E30DEFF44C0B5D`.
- Changed only the outer-frame metadata renderer to use Beihaibei SC for aperture, shutter speed, ISO and timestamp text.
- Kept the application UI font, top branding artwork and `luna moment` signature artwork unchanged.
- Ran the HTML application's watermark test suite: 15 tests passed, 0 failed and the external-image QA test remained explicitly ignored in the regular suite.
- Ran the ignored external-image QA test against `G:\Insta360 Luna Ultra\IMG_20260626_164557_021.jpg`; it passed and rendered `target/beihaibei-frame-preview.png` with shutter speed `1/800`.
- Built the release `html_app` successfully and updated the local daily executable `F:\Insta360onWin\LunaStudio.exe` (SHA-256 `A1541AD7758058E6EE1B2C3FF77D716C34263220D2ED132FAD81D0D894BAABE4`).
- Follow-up: photos without `OffsetTimeOriginal` now use the app-reference timezone suffix `UTC+08:00`; embedded EXIF offsets still take priority.
- Follow-up: added a restrained four-neighbor alpha expansion to make Beihaibei SC metadata text slightly heavier without changing its font size or layout coordinates.
- Follow-up: increased both metadata lines to 116% of the APK table's original font ratio and adjusted the second-line offset to preserve readable spacing.
- Re-ran the watermark suite after the timezone, weight and size changes: 15 tests passed and 0 failed.
- Re-ran the external Luna original QA test and generated `target/beihaibei-frame-preview-v2.png`; the rendered timestamp is `26-Jun-2026 16:45 UTC+08:00` and the shutter remains `1/800`.
- Rebuilt and replaced the local daily `LunaStudio.exe`; final SHA-256 is `90EAF634D229406B8A88DE9173C0C37E7D6BE54498EA5F1D30598D10AE399F0F`.
- Follow-up: increased outer-frame metadata character tracking by 4.5% of the rendered font size, while preserving the separate spacing between aperture, shutter and ISO groups.
- Follow-up: corrected aperture formatting from `F2.0` to the requested photographic notation `F/2.0` and added parser/render test coverage.
- Re-ran the watermark suite after tracking and aperture-format changes: 15 tests passed and 0 failed.
- Generated and visually checked `target/beihaibei-frame-preview-v4.png`; it shows `F/2.0  1/800  ISO161` and `26-Jun-2026 16:45 UTC+08:00` with the wider tracking.
- Rebuilt and replaced the local daily `LunaStudio.exe`; final SHA-256 is `34BAA3FBBCF9B8AE526896C70C4BB4B792122CEB0671E2E3D78B3F53500515D6`.

## 2026-08-07 - Luna Moment presets and custom image

- Added a daily-use Luna Moment selector to the outer-frame watermark UI with `官方 Luna Moment`, `深深的巡演` and `自定义图片` choices.
- Added a native image picker for custom PNG/JPEG/WebP files, a restore-default command, persisted selection, and immediate real-preview refresh.
- Connected `moment_preset` and optional `moment_image` through WebView IPC, preview rendering and final export.
- Preserved custom/preset image color and alpha; only the official Luna Moment asset follows the selected frame foreground color.
- Added bounded aspect-fit behavior so wide or tall custom images remain inside the official Moment slot and cannot overlap metadata.
- Copied the user-provided Xiaohongshu image into `assets/moment_presets/shenshen-concert.jpg` and embedded it into the application; SHA-256 is `DD891AD62591866FA4DB80F76779465A0318FE5DABFAC8373DAD3AA9D0A555F7`.
- Added tests for the built-in preset's blue color, custom PNG transparency and Moment slot fitting.
- Real-render QA showed the concert artwork's small caption needed more room, so color/custom Moment images may use up to 40% of the footer height while remaining above the metadata block.
- Verified the inline frontend JavaScript with `node --check`.
- Ran the complete `html_app` suite: 50 tests passed, 0 failed and 4 hardware/external-image tests remained explicitly ignored.
- Real-rendered the built-in preset against the Luna original as `target/shenshen-moment-preview-v2.png` and visually confirmed color, size, centering and metadata clearance.
- Rebuilt and replaced the local daily `LunaStudio.exe`; final SHA-256 is `D88B1EC45664941947050448B5091234E083DE5A5CFC04347DF68C774E151780`.

## 2026-08-07 - Shenshen preset artwork replacement

- Replaced `assets/moment_presets/shenshen-concert.jpg` with the user-provided title-only artwork (white `深深的` lettering and blue strokes on black), removing the previous concert-tour caption from this preset.
- Kept the existing `shenshen-concert` preset identifier and UI selection so saved user settings remain compatible.
- The replacement source is `1080x478`; embedded asset SHA-256 is `5D1CAAAD0BB85C450CD1E984182BA328E308CCDF6B80739FB2D8DB973726E4B0`.
- Ran the watermark suite after replacement: 17 tests passed, 0 failed and the external-image QA test remained explicitly ignored in the regular suite.
- Ran the external Luna original QA render with the replacement preset and generated `target/shenshen-titleonly-preview.png`; visually confirmed aspect-fit centering, preserved blue/white artwork and metadata clearance.
- Rebuilt and replaced the local daily `LunaStudio.exe`; final SHA-256 is `0CDA7645E6C9BE132368A11833D0BF7D021E80A743421AF0092B2041F95A9C5B`.

## 2026-08-07 - Moment preset fixed-height sizing

- Changed colored built-in and custom Luna Moment artwork from width-constrained fitting to a fixed height of 40% of the footer panel; width is now derived from the source aspect ratio.
- Kept the official APK Luna Moment artwork on its original width-based placement, so the stock style remains unchanged.
- Added a canvas-width safety fallback only for exceptionally wide custom images.
- Updated sizing tests and ran the watermark suite: 17 tests passed, 0 failed and 1 external-image test remained ignored in the regular suite.
- Real-rendered `target/shenshen-fixed-height-preview.png` against the Luna original and visually confirmed the wider centered title artwork does not overlap metadata.
- Rebuilt and replaced the local daily `LunaStudio.exe`; final SHA-256 is `AFB0A239979C55CF5560AD92A5D39C9203CF1F2C230377906605D7AB2C3EDBF8`.

## 2026-08-07 - Match preset height to official Luna Moment

- Replaced the temporary 40%-of-footer rule with the official APK Luna Moment's actual rendered height as the shared sizing reference.
- The renderer loads the official `749x259` Moment asset, applies the APK `moment_width_ratio`, then gives the resulting height to built-in colored presets and custom images.
- Width remains proportional to each image; an exceptionally wide custom image is limited only by the canvas width.
- Added regression coverage proving the official and Shenshen preset both render at `124px` high on the reference test canvas.
- Ran the watermark suite: 17 tests passed, 0 failed and 1 external-image test remained ignored in the regular suite.
- Real-rendered and visually checked `target/shenshen-official-height-preview.png`.
- Rebuilt and replaced the local daily `LunaStudio.exe`; final SHA-256 is `B56B9397589011A5435A8A8A3FC0CB774AF786CF6AF28926AFD2A8D7AF683120`.

## 2026-08-07 - Native Android ARM64 application

- Added `android/`, a native Java WebView application using package id `studio.luna.linker`, minimum Android 8.0 (API 26), target/compile API 36 and an adaptive launcher icon.
- Split Cargo dependencies by target so the Windows WebView2, Win32 virtual-camera, desktop BLE and file-dialog crates are not compiled into Android.
- Added `src/android_bridge.rs`, an ARM64 JNI bridge that reuses the APK/PCAP-derived Rust UCD2 implementation for Luna detection, persistent media/control sessions, internal/SD media listing, deletion, mode switching, zoom, video profiles, photo/record commands, gimbal movement/speed and batch downloads.
- Reused the Rust watermark renderer on Android for photo preview/export, including official APK assets, frame colors, the Shenshen preset and custom Luna Moment images.
- Added Android-native file import/output locations, sampled image thumbnails, MediaMetadataRetriever video thumbnails, media scanning and a five-second Mic Pro BLE scan.
- Added Android local-network, Wi-Fi and Bluetooth permissions. Cleartext traffic is intentionally enabled because Luna Ultra serves its local media over `http://192.168.42.1`.
- Android hides Windows-only virtual-camera controls and the live-preview surface. UCD2 HEVC live preview is explicitly deferred until an Android MediaCodec pipeline is connected; capture and gimbal controls remain available without preview.
- Added `android/build_android.ps1`, the Gradle wrapper and `android/README.md`. The build script compiles Rust for `aarch64-linux-android`, packages the JNI `.so`, builds the APK and copies it to the repository root.
- Installed the portable build chain only under `F:\AndroidToolchain`: Temurin JDK 17.0.20, Android command-line tools 15859902, API/Build Tools 36, NDK 28.2.13676358, Gradle 9.4.1 and Rust 1.97.1 with `aarch64-linux-android`.
- Verified `cargo build --lib --release --target aarch64-linux-android --locked`, `lintDebug`, `assembleDebug`, Rust formatting and the complete Windows `html_app` suite (50 passed, 0 failed, 4 ignored).
- Verified the APK contains `lib/arm64-v8a/libluna_mic_rust.so`, `assets/web/index.html` and the APK watermark assets. JNI exports and native dependencies were checked with NDK `llvm-nm`/`llvm-readelf`.
- No Android device was attached to ADB, so installation and hardware session checks remain pending. Static APK verification and v2 debug-signature verification passed.
- Final debug APK: `F:\Insta360onWin\Insta360Linker-android-arm64-debug.apk`, 8,327,872 bytes, SHA-256 `EE29C5BE758F823A5D370B82638A56365303CB828B2DC5AFEBC31939C676DDAB`.

## 2026-08-07 - Native macOS application

- Added native macOS desktop support using tao/wry with the system WebKit runtime while preserving the existing Windows WebView2 build.
- Added a non-Windows virtual-camera adapter. macOS intentionally reports the system virtual camera as unavailable, while keeping the UCD2 HEVC live-preview pipeline active inside the application.
- Made FFmpeg runtime discovery platform-aware for live preview, video thumbnails and watermark export, including `Contents/Resources/ffmpeg/ffmpeg` inside a macOS application bundle.
- Added `build_macos.sh` and `macos/Info.plist` to build, bundle and ad-hoc sign `dist/Luna Studio.app`. The script downloads the appropriate Apple Silicon or Intel FFmpeg runtime when it is missing locally.
- Moved the macOS thumbnail cache to `~/Library/Caches/Luna Studio/gallery-thumbnails` so Finder-launched application bundles do not attempt to write into their launch directory.
- Added macOS local-network usage text for connecting to the Luna Ultra camera hotspot.
- Built and launched the Apple Silicon release successfully. Both the application executable and bundled FFmpeg were verified as native arm64 Mach-O binaries; bundle signature and property-list validation passed.
- Ran the complete `html_app` test suite on macOS: 45 tests passed, 0 failed and 2 physical-hardware/external-input tests remained explicitly ignored.
- The freshly cloned checkout exposed an unrelated CRLF/LF-only modification in `reverse_apk/tools/ashield_unpack_cluster_disasm.txt`; it was intentionally excluded from this change.
- Maintenance policy requested by the repository owner: every future code or build update must be recorded by appending to this existing Markdown log as part of the same change.

## 2026-08-07 - Bundle official watermark resources on macOS

- Fixed the initial macOS package, which bundled FFmpeg but omitted the runtime-loaded official watermark PNG resources.
- Updated `build_macos.sh` to copy the complete `assets/apk_watermark` resource set into `Luna Studio.app/Contents/Resources/apk_watermark`.
- Updated the watermark resource loader to resolve official assets from the macOS application bundle's `Contents/Resources` directory while retaining the existing Windows and development-tree lookup paths.
- Updated the macOS build documentation to state that both FFmpeg and official watermark resources are packaged into the application.
- Rebuilt the Apple Silicon application and verified 39 official watermark resource files inside the signed bundle, including all four Luna Ultra image/video PNG files required by the published styles.
- Re-ran the complete macOS `html_app` suite: 45 tests passed, 0 failed and 2 hardware/external-input tests remained explicitly ignored.

## 2026-08-07 - Final-action export destinations and Android gallery saving

- Removed the desktop media-library download-location control and watermark output-path field so users configure the export first and choose a destination only after clicking the final download/export button.
- Changed desktop batch downloads to open the folder picker from `下载所选`, and changed watermark export to open a native save dialog only from `导出水印文件`.
- Desktop watermark save dialogs now derive the suggested filename and output format from the selected source: MP4 for video, PNG/WebP when applicable, and JPEG for other photos.
- Changed Android batch downloads and watermark exports to use unique app-cache paths internally without showing a destination picker.
- Added Android MediaStore publishing for Android 10 and newer, saving completed photos and videos directly under the system gallery's `DCIM/Insta360Linker` collection and removing the successfully published temporary files.
- Added the Android 8-9 compatibility path using the public DCIM album, media scanning and the legacy storage permission limited to API 28.
- Updated Android action labels to `保存所选` and `保存到相册`, and changed completion messages/results to report system-gallery saves instead of private app paths.
- Updated the root and Android README files to document the delayed desktop picker and direct mobile gallery behavior.
- Verified the inline frontend JavaScript syntax and parsed `MainActivity.java` successfully with a Java syntax parser.
- Ran the complete macOS `html_app` suite: 45 tests passed, 0 failed and 2 hardware/external-input tests remained explicitly ignored.
- Rebuilt `dist/Luna Studio.app`; strict deep code-signature verification and Info.plist validation passed, and all 39 official watermark resource files remain bundled.
- Android Gradle/APK compilation was not run on this Mac because it has no Java runtime or Android SDK; the existing Windows Android toolchain remains required for the final APK build.

## 2026-08-07 - Native macOS Liquid Glass icon source

- Imported the user-provided `Insta360Linker.icon` Icon Composer package into `macos/Insta360Linker.icon`, preserving its original PNG/SVG layers, default/dark/tinted specializations and Liquid Glass annotations.
- Added a manually dispatched Xcode 27 workflow that compiles the layered Icon Composer source with Apple's `actool` into the native `Assets.car` runtime catalog and a backward-compatible `Insta360Linker.icns` file.
- Kept Android launcher resources unchanged as requested; this update is scoped to macOS.
