# Insta360Linker Development Log

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

## 2026-08-07 - macOS camera route and connection fix

- Diagnosed the Mac connection failure as a false-positive route: the default camera address `192.168.42.1` was being accepted by a VPN/proxy `utun` route even though the Mac was on a different Wi-Fi subnet.
- Added macOS physical-interface discovery and same-subnet validation for the Luna Ultra private IPv4 address, explicitly excluding `utun` interfaces.
- Bound all macOS camera TCP sockets to the matching local interface address before connecting, covering detection, persistent media sessions, control/live-preview sessions and APK-derived diagnostic probes.
- Bound camera HTTP requests to the same local address for media proxying, thumbnails and resumable downloads while retaining proxy bypass.
- Added a four-second TCP connection timeout so unreachable camera attempts fail predictably instead of hanging on the system route.
- Surfaced the detailed route error in the application UI, telling the user to connect the camera hotspot and disable a VPN/proxy that takes over local-network traffic.
- Added subnet regression coverage and updated the root README with the macOS connection requirement.
- Verified the new route test and `cargo check --bin html_app` on macOS; both passed. Existing dead-code warnings are unchanged.

## 2026-08-07 - Xcode 27 Liquid Glass icon integration

- Committed `.github/workflows/compile-macos-icon.yml` and ran GitHub Actions job `31171392627` on the `xcode-27` runner.
- Compiled the checked-in Icon Composer package with Xcode 27.0 (`27A5228h`) into `macos/compiled/Assets.car` and the backward-compatible `macos/compiled/Insta360Linker.icns`.
- Preserved the generated `Assets.json` inspection report beside the compiled files so native icon contents remain auditable without reverse-engineering the binary catalog.
- Verified that the catalog contains the `Insta360Linker.iconstack`, vector and raster source renditions, dark/light/tinted appearances, layered `IconGroup` records, per-element specular gathering and `LayerHasSpecular` glass highlights.
- Added `CFBundleIconName=Insta360Linker` so modern macOS resolves the native layered catalog, plus `CFBundleIconFile=Insta360Linker` for the `.icns` fallback.
- Updated `build_macos.sh` to require and copy both compiled icon resources into `Luna Studio.app/Contents/Resources` before ad-hoc signing.
- Xcode 27 artifact SHA-256: `Assets.car` is `7d6f5c580c24e3b33722b1e0829f862e3430612edb009fddd154fd22eb2cb27b`; `Insta360Linker.icns` is `42af632f2753876943b50e6500dfb9ecc1a022d2541bd6f17e2267730f6d8949`.
- Android icon resources remain unchanged as requested.

## 2026-08-11 - Final macOS package verification

- Rebuilt `dist/Luna Studio.app` after integrating the Xcode 27 icon resources and the macOS camera-route fix.
- Verified the signed application bundle contains byte-identical `Contents/Resources/Assets.car` and `Contents/Resources/Insta360Linker.icns` files with the recorded Xcode 27 artifact hashes.
- Verified the bundled catalog exposes 87 matching native icon records across `IconGroup`, specular-layer and `Insta360Linker.iconstack` inspection queries.
- Verified `CFBundleIconName` and `CFBundleIconFile`, strict deep code signing, the arm64 application executable, the arm64 FFmpeg runtime and the 77 MB final bundle.
- Re-ran the complete macOS `html_app` test suite after the connection fix: 46 tests passed, 0 failed and 2 physical-hardware/external-input tests remained explicitly ignored.

## 2026-08-11 - GitHub Actions Android ARM64 build

- Added `.github/workflows/build-android.yml`, a manually dispatched Ubuntu workflow that installs Java 17, Android API/Build Tools 36, NDK 28.2.13676358 and the Rust `aarch64-linux-android` target.
- The workflow compiles the shared Rust JNI library for Android API 26, packages it under `lib/arm64-v8a`, runs Android Lint, builds the debug APK and verifies its signature plus embedded JNI, Web UI and official watermark resources.
- Configured the verified APK and its SHA-256 file as a downloadable `Insta360Linker-android-arm64-debug` artifact with 14-day retention.
- Initial run `31417450829` proved the Rust/NDK stage works but exposed that the checked-in Unix Gradle wrapper lacked its executable bit; corrected the repository mode before rerunning the workflow.
- Successful run `31417690489` completed in 2 minutes 55 seconds: Rust JNI compilation, Android Lint, Gradle assembly, APK signature/resource verification and artifact upload all passed.
- Downloaded the verified artifact to the repository workspace as the ignored local delivery file `Insta360Linker-android-arm64-debug.apk`.
- Final APK size is 8,348,301 bytes and SHA-256 is `503ca66a82ee975c25059b3d521308e7c4972e286b4bb99e7985935cde22a3be`.
- Independently tested the downloaded ZIP container with no errors and confirmed `lib/arm64-v8a/libluna_mic_rust.so`, `assets/web/index.html` and the packaged official watermark resource directory are present.
- Updated the root and Android README files with GitHub Actions build and artifact-download instructions.

## 2026-08-12 - macOS camera interface binding correction

- Re-investigated the reported Mac connection failure against the live machine instead of relying on the earlier TCP-port symptom.
- Confirmed the Mac was currently on `192.168.124.98/24` rather than the camera's `192.168.42.x` hotspot network, while the route to `192.168.42.1` was still owned by VPN/proxy interface `utun4`.
- Confirmed `/Applications/Luna Studio.app` already contained the earlier route-detection code, so the remaining fix required stronger socket routing rather than merely replacing an obviously old build.
- Replaced address-only camera binding with a macOS interface record containing the physical interface name, IPv4 address and non-zero interface index.
- Added native `IP_BOUND_IF` binding to every UCD2 TCP socket before the local-address bind and connect, preventing a VPN/proxy route from overriding the selected physical camera interface.
- Added Reqwest interface binding to all camera HTTP clients, covering media proxying, thumbnail generation and resumable downloads as well as their existing local-address and no-proxy settings.
- Preserved same-subnet validation and explicit `utun` exclusion, and added detailed connection errors that also point users to the macOS Local Network privacy permission.
- Updated the macOS README guidance to require the camera hotspot and Luna Studio Local Network permission.
- Ran `cargo check --bin html_app` and the complete macOS suite: 46 tests passed, 0 failed and 2 hardware/external-input tests remained explicitly ignored.
- Rebuilt and installed the corrected package at `/Applications/Luna Studio.app`; strict deep-signature and Info.plist validation passed.
- Verified the built and installed application executables are byte-identical with SHA-256 `97cd8638ec2cc6beaeeeac3b2d5fb01f88db4d2730a49d82b80fc02a021db274`.
- Physical-camera verification remains pending because the Mac was still attached to `192.168.124.0/24` rather than the camera hotspot during this diagnostic session.

## 2026-08-12 - Complete Insta360Linker rename and cross-platform glass icon

- Renamed the product throughout the active macOS, Windows, Android and Web UI surfaces to the exact display name `Insta360Linker`, while preserving `Luna Ultra`, `Luna Moment` and UCD2 names that identify camera models, media features or protocols.
- Renamed the Rust package to `insta360_linker` and the desktop Cargo binary target to `Insta360Linker`, so Windows now produces `Insta360Linker.exe` and macOS packages an `Insta360Linker` executable.
- Renamed the macOS bundle to `Insta360Linker.app`, changed its identifier to `studio.insta360.linker`, updated application/cache/user-agent strings, and retained the Xcode 27 compiled `Assets.car` native Liquid Glass catalog plus `.icns` compatibility fallback.
- Exported `assets/branding/Insta360Linker-glass.png` directly from the user-provided Icon Composer source with Apple `ictool`; generated the Windows `.ico` and Android density PNGs from that same rendered glass artwork rather than recreating the design.
- Added Windows executable metadata/icon compilation and a matching native window icon; renamed the virtual camera display name, helper DLL lookup and runtime thread labels to `Insta360Linker`.
- Changed the Android namespace/application id to `studio.insta360.linker`, renamed its Java/JNI package and Rust library to `libinsta360_linker.so`, changed the launcher label to `Insta360Linker`, and replaced the former adaptive placeholder vector with static glass PNGs for all five Android densities.
- Renamed the JavaScript host bridge to `window.Insta360LinkerBridge`, updated both desktop and Android dispatchers, updated build scripts/workflow verification paths, executable launch scripts and current documentation.
- Ran `cargo check --bin Insta360Linker` and the complete locked macOS suite: 46 tests passed, 0 failed and 2 physical-hardware/external-input tests remained explicitly ignored.
- Built and ad-hoc signed `dist/Insta360Linker.app`; strict deep-signature validation passed, `Info.plist` reports the new name/identifier, the native icon resources and all 39 official watermark files are present, and the executable SHA-256 is `2b20bf73e2c97fa4c8de7dd98d5f827eada8904c24e57b763e5caa3483d72ba8`.
- Installed and successfully launched `/Applications/Insta360Linker.app`. Moved the obsolete `/Applications/Luna Studio.app` bundle to `~/.Trash/Luna Studio.app`, where it remains recoverable.
- Static branding asset SHA-256 values: `Insta360Linker-glass.png` is `9bb94d211849760ee87473b801aad4466827e400477891d1cc948154aefa8dd4`; `Insta360Linker.ico` is `76dae6e5e0de1b4de6e3f015aa59cffb97b7630bdcdba0008fc7eb6872221385`.

## 2026-08-12 - Pure SwiftUI macOS Liquid Glass and unified cross-platform design

- Replaced the macOS WebView frontend completely with a Swift 6.4/SwiftUI application targeting macOS 26. The packaged `Contents/MacOS/Insta360Linker` executable contains the Apple-native frontend and does not host HTML or WebKit content.
- Rebuilt all four macOS feature areas as native SwiftUI: camera media grid and selection, capture/live-view controls, watermark preview/export form, and Mic Pro scanning/inspection. Navigation, forms, pickers, sliders, lists, split views, alerts, confirmation dialogs and open/save panels now use SwiftUI or AppKit system components.
- Applied native `GlassEffectContainer`, `.glassEffect`, `.buttonStyle(.glass)` and `.glassProminent` surfaces throughout the sidebar, topbar, cards, controls and notices. The native SwiftUI accessibility tree and visible Liquid Glass refraction/highlights were verified in the installed application.
- Added a process-based Swift/Rust bridge: the existing Rust camera, UCD2, media, thumbnail, Bluetooth, FFmpeg and official watermark implementation now runs as the signed bundle helper `Contents/Resources/Insta360LinkerBackend`, exchanging newline-delimited JSON and live JPEG preview events with SwiftUI.
- Preserved delayed desktop save prompts in the native frontend: media downloads choose a folder only after `下载所选`, and watermark export opens `NSSavePanel` only after `导出水印文件`.
- Updated the macOS build to compile the SwiftUI executable with the macOS 26 SDK, package and sign the Rust helper separately, retain the Xcode 27 `Assets.car`/`.icns` icon resources, and declare macOS 26 as the minimum supported system.
- Unified Windows Mica and Android static glass with the same floating navigation hierarchy, spacing, radii, selection state and card language while retaining their existing Web UI hosts.
- Added `web/app-icon.png`, a 128px render of the supplied Xcode 27 glass icon, to the Windows/Android shared brand header and Android runtime assets. Its SHA-256 is `ba90f253af831914bccbcc2f802613cbf1bd12125d2ba7b1da760fb6f1413fda`.
- Updated Android to inject the current `android-host` class, package the shared icon, use 48dp-equivalent touch targets and keep status/navigation bars synchronized with system light or dark appearance. Mobile export remains direct-to-gallery with no destination picker.
- Added static fallbacks for reduced transparency, forced colors and desktop systems without native Mica. Android uses non-blurred gradient/highlight surfaces instead of pretending to provide OS Liquid Glass.
- Verified the Swift-to-Rust bridge by issuing a real native camera connection from the installed SwiftUI interface; the Rust backend returned the expected physical-network diagnostic for the Mac's current non-camera Wi-Fi route.
- Compiled the complete native SwiftUI frontend independently with Swift 6.4 and the macOS 27 SDK, then ran the locked Rust suite: 46 tests passed, 0 failed and 2 physical-hardware/external-input tests remained explicitly ignored.
- Rebuilt, separately signed and installed the SwiftUI app plus Rust helper at `/Applications/Insta360Linker.app`; strict deep-signature and Info.plist validation passed. `otool` confirms the frontend links SwiftUI/AppKit and has no WebKit dependency.
- Final installed SHA-256 values: native SwiftUI frontend `a875454db9415a1b3f34fd43bb640227b1a566540e9a2bc3612528a678b25ff3`; Rust backend helper `e7627c98b236e9fc13d193d3792de9239b744e4af8a374db7aefd7d7786b8fed`.
- Visually and interactively verified the installed SwiftUI camera-media, capture/live-view, watermark and Mic Pro pages, including native accessibility elements and the Swift-to-Rust connection response. Android Gradle/APK compilation remains assigned to GitHub Actions because this Mac has no Java runtime or Android SDK.

## 2026-08-12 - Native macOS window background and system component correction

- Removed the transparent whole-window background that had been introduced by `.containerBackground(.clear, for: .window)` and restored `NSColor.windowBackgroundColor` as the stable native window and detail-area base.
- Replaced the hand-built sidebar, header and nested glass-panel hierarchy with Apple's `NavigationSplitView`, selectable `List`, unified `Toolbar`, `Form`, `GroupBox`, `ControlGroup`, `HSplitView`, `ContentUnavailableView` and standard control styles.
- Removed `GlassEffectContainer` and all manually applied `.glassEffect(...)` surfaces from the macOS frontend. Liquid Glass is now provided by macOS in the system navigation, toolbar, selection and button contexts where Apple intends the material to appear.
- Retained native `.buttonStyle(.glass)` and `.buttonStyle(.glassProminent)` only for appropriate custom actions such as camera connection, capture, scanning and export.
- Rebuilt and installed `/Applications/Insta360Linker.app`, then visually verified that the full window has an opaque system background and that no desktop content leaks through the detail area.
- Re-ran the standalone Swift 6.4 compile, complete macOS packaging, strict deep code-signature validation, Info.plist validation and dependency inspection; all passed and the SwiftUI frontend remains free of WebKit linkage.
- Installed SHA-256 values after this correction: native SwiftUI frontend `945384919264843c445e5900a7f9f736830399d96dae0bba8988876cf84c7409`; Rust backend helper `e7627c98b236e9fc13d193d3792de9239b744e4af8a374db7aefd7d7786b8fed`.

## 2026-08-12 - Windows and Android interface overhaul

- Fast-forwarded the Windows workspace from `35df29c` to GitHub commit `ff29aa9` before editing; no local tracked changes were overwritten.
- Audited the shared 3,210-line Web frontend, all page identifiers and event bindings, the Windows host-class injection, and the Android WebView injection before changing the UI.
- Confirmed that current macOS builds use the separate native SwiftUI frontend, so this redesign is intentionally scoped to Windows and Android.
- Added `web/app.css` as the final shared style layer, leaving existing command identifiers and camera, media, watermark, preview and Bluetooth behavior untouched.
- Rebuilt the visual language around neutral Mica-aware surfaces, teal interaction color, compact 6-8px radii, clearer typography, stable spacing and stronger control opacity.
- Replaced text-symbol navigation artwork with consistent inline stroke icons and added `aria-current` state updates for accessible page navigation.
- Reworked Windows into a spacious desktop workbench with a 252px navigation rail, contained Mica surfaces, sticky media filters, denser image-first media cards, a larger live view, a dedicated capture rail and a sticky real watermark preview.
- Reworked narrow layouts and Android into a touch-first interface with a fixed bottom tab bar, 44px controls, horizontal filter strips, two- or three-column incremental media grids, safe-area padding and full-screen media preview.
- Added storage badges and separated capture time/file size metadata on media cards while preserving lazy thumbnails and the existing incremental `显示更多` renderer.
- Preserved each storage badge when the lazy-thumbnail pipeline replaces its visual node or falls back after a thumbnail error, preventing the badge from disappearing after initial render.
- Removed the Android runtime-injected layout override; host-specific preview hiding and single-column capture layout now live in the versioned shared stylesheet.
- Updated Android system-bar colors to match the new neutral light and dark palettes.
- Added `app.css` to both the Rust desktop asset server and Android generated asset package.
- Rendered all four pages at 1440x900, 1024x768, 390x844 and 360x800 with Playwright/Edge; the first pass found no desktop overflow and isolated the mobile media toolbar as the only off-screen control group.
- Replaced that mobile horizontal toolbar with a three-row responsive grid so all storage, media type, sort, density and refresh controls remain visible without sideways scrolling at 360px.
- Extended visual validation to light and dark appearances: 24 page/viewport combinations across 1440x900, 1024x768, 390x844 and 360x800 completed with zero document-width or element-boundary overflow failures.
- Passed `cargo fmt --all --check`, `cargo check --locked --bin Insta360Linker` and `cargo test --locked --bin Insta360Linker`; 51 tests passed, 0 failed and 4 hardware/system-registration tests remained explicitly ignored.
- Built the Windows release executable with `cargo build --release --locked --bin Insta360Linker --target-dir target_daily` and built the Android ARM64 debug APK with the repository's F-drive Android toolchain; both builds completed successfully.
- Verified the APK contains `assets/web/index.html`, `assets/web/app.css`, `assets/web/app-icon.png` and `lib/arm64-v8a/libinsta360_linker.so`.
- Final Windows executable SHA-256: `8ee2921d6975f1ff68048b2ee4d9ae3685af7a30e43a6e02757691f6db4edda7`.
- Final Windows virtual-camera DLL SHA-256: `cfedd345ac8ff2b797a638735e567f72b5800bf8701e0da9f05ad5ecc5403177`.
- Final Android ARM64 debug APK SHA-256: `d19615f10b740cdcc3ddb507bebe285a8d3eba6278b51a73f964529e22b3f2ba`.
- The visual-check script, 24 screenshots, JSON report, separate Windows target directory and Android intermediate output directories were designated temporary and removed after verification; only source changes and final deliverables remain.

## 2026-08-12 - Capability-driven conditional native UI

- Added explicit image/video capability parsing for every official watermark style, using the backend catalog's `image_file`, `video_file` and `kind` fields instead of maintaining a separate visual-only assumption in SwiftUI.
- Filtered the native watermark style picker to styles supported by the selected source media. Selecting a video automatically removes photo-only frame styles and normalizes an incompatible previous selection to a supported official style.
- Changed the watermark form to progressive disclosure: style selection appears after choosing a source; position appears only for video mark styles; frame background and Luna Moment appear only for photo frame styles; the custom image row appears only for the custom Moment preset; export and preview actions appear only when their prerequisites are available.
- Normalized the payload as well as the UI so hidden settings cannot leak stale values into rendering: fixed layouts always use bottom-center, non-frame styles omit custom Moment images, and unsupported style/media combinations cannot preview or export.
- Hid media filters and management until the camera is connected, hid batch download/delete controls until media is selected, and replaced unavailable media states with native `ContentUnavailableView` content.
- Hid capture, preview, lens and gimbal controls until the control session is ready; hid recording format controls outside video mode and locked mode/format changes while recording.
- Added native scanning, empty and populated states to the Mic Pro split view so an empty device list no longer presents a blank control surface.
- Recompiled the complete SwiftUI frontend with Swift 6.4, rebuilt and signed the app bundle, and verified the initial unconnected accessibility tree contains the connection action and native unavailable state without hidden media-management controls.
- Re-ran formatting and the locked backend suite: 46 tests passed, 0 failed and 2 physical-hardware/external-input tests remained explicitly ignored; strict bundle signing, Info.plist validation and the no-WebKit frontend dependency check also passed.
- Rebuilt once more from the merged GitHub `main` containing the concurrent Windows/Android interface update, then installed the matching bundle. Final installed SHA-256 values: native SwiftUI frontend `0d12e6b4de6086975f3ca67ecf367156f477152cd2fa2906599920c5750eefa2`; Rust backend helper `b76aa10760733c866bf9fcba313fd867d451a18779488877cf0293460df931eb`.

## 2026-08-14 - Direct camera watermark export, Android playback and live-view work

- Fast-forwarded the Windows workspace from `91435c2` to GitHub `main` commit `3a2f5ab` before editing. The two incoming commits contained the latest native macOS capability-driven UI and build records; no tracked local work was overwritten.
- Preserved the pre-existing untracked legacy runtime files (`LunaStudio.exe`, `LunaStudio.exe.WebView2/` and `LunaVirtualCamera.dll`) and excluded them from this work.
- Began a three-platform audit of camera-media playback, live-preview event delivery and watermark input handling. Confirmed that Android currently hides both the preview trigger in Java and the entire live surface in shared CSS, while the virtual-camera control is a separate feature that should remain unavailable on Android.
- Set the implementation scope to: stabilize Android camera-video playback, provide a one-action camera-gallery-to-watermark workflow on Windows/Android/macOS, restore Android real-time monitoring, and keep virtual-camera integration desktop-only.
- Added a shared `prepare_watermark_media` workflow to the desktop/macOS Rust backend and Android JNI backend. It validates that the URL belongs to the connected camera, reuses the authenticated media session, downloads the original into an application-managed cache, and returns the local source path to the existing official watermark renderer.
- Added camera-gallery watermark actions to the shared Windows/Android media toolbar and context menu. A supported single selection now opens the watermark workspace with the camera original already loaded; local file picking remains available as an optional fallback.
- Added the equivalent native macOS action to the selected-media toolbar and each media card context menu, then connected it to the same Rust command and native watermark preview page.
- Corrected native macOS media classification to derive photo/video capability from the actual file extension returned by Luna Ultra instead of comparing the backend's `JPG`/`MP4` kind field to UI-only `photo`/`video` strings.
- Replaced Android's broken desktop-relative `/media/...` path with an Android-only appassets URL and a WebView native media interceptor. The interceptor proxies the connected camera stream, forwards byte-range requests, preserves `Content-Range`/length/type headers and keeps the upstream connection alive until WebView closes it, so video metadata, seeking and fallback from LRV to the original no longer depend on a nonexistent Android loopback server.
- Restored the Android live-view surface and preview action while keeping only the Windows virtual-camera panel hidden. The Android Rust bridge now starts/stops the same captured UCD2 preview commands as desktop, retains the returned HEVC chunks instead of discarding them, and exposes a zero-copy JNI polling method.
- Added an Android MediaCodec HEVC preview pump backed by ImageReader. It converts throttled decoded YUV frames to JPEG and forwards them to the existing shared live canvas, with lifecycle shutdown and visible decoder errors; no Android virtual-camera device is created.
- Added a real Android video-watermark export path with Media3 Transformer instead of calling the desktop FFmpeg executable that is unavailable on Android. Rust returns the official APK-derived Luna video asset and placement ratios, Android composites it with the system video pipeline, preserves audio, writes H.264 MP4 and publishes the result to the system gallery.
- Limited direct camera-watermark actions to formats the renderer genuinely supports (`JPG`/`JPEG`/`PNG`/`WEBP` and `MP4`/`MOV`/`M4V`), rejects photo-only frame styles for video, prevents concurrent Android video exports and cancels an active Transformer during Activity shutdown.
- Added a regression test proving the Android 16:9 video plan uses the `Luna Ultra##Leica-CN` APK table values (`0.220`, `0.390`, `0.059`) and the official PNG asset. The complete locked Windows backend suite passed with 52 tests, 0 failures and 4 physical-device/system-registration tests explicitly ignored.
- Added AndroidX Media3 Transformer/Effect/Common 1.10.1, marked the Activity's deliberate unstable Media3 API usage, and moved the API-27 light-navigation-bar style to `values-v27` so Android 8.0 remains supported. `lintDebug` and the final `assembleDebug` both passed.
- Verified shared JavaScript syntax with Node, verified Rust formatting, and normalized all modified text files to UTF-8 CRLF as required by the repository instructions. `git diff --check` reported no whitespace errors.
- Verified the final APK contains the shared HTML/CSS/icon, four DEX files including Media3 Transformer, `lib/arm64-v8a/libinsta360_linker.so`, the official Luna watermark assets and configuration tables. APK Signature Scheme v2 verification passed with the Android debug certificate.
- No Android device was attached to ADB, so physical playback, camera Wi-Fi and live HEVC monitoring could not be exercised in this Windows session. The native code, JNI bridge, Java compilation, Lint, APK assembly and package inspection all passed.
- Built the Windows release executable successfully. A hidden launch smoke attempt was denied by the Codex host process policy and left no running process; compilation and the full native test suite remain successful. Native SwiftUI macOS source was updated consistently, but macOS compilation cannot be run on Windows.
- Final Windows executable: `Insta360Linker.exe`, 10,371,072 bytes, SHA-256 `c115054cc360e7d89496c8360af4c55644c09f3a8bbe42adc13d5cd20a00523d`.
- Final Android ARM64 debug APK: `Insta360Linker-android-arm64-debug.apk`, 14,319,392 bytes, SHA-256 `b735ac126d1b6ffa9fdabd2912b5a3b0968e49446ad06fad538ebdb90d368bde`.
- The first direct recursive cleanup attempt was blocked by the Windows execution safety policy. Cleanup was then completed with Cargo and Gradle's scoped clean commands: `target_daily`, `target/aarch64-linux-android`, `android/app/build`, `android/build` and generated `android/app/src/main/jniLibs` were permanently removed, and the temporary ADB server was stopped. Final deliverables, source files and pre-existing untracked runtime files were preserved.

## 2026-08-14 - Persistent pre-development synchronization rule

- Fetched GitHub before editing and confirmed local `HEAD` and `origin/main` both pointed to `309096f62001f96d292171c0afa9e365d624df9b`; the tracked working tree was clean and the three pre-existing untracked legacy runtime items were preserved.
- Re-read the latest `DEVELOPMENT_LOG.md` entries before making this repository-instruction update.
- Added root `AGENTS.md` instructions requiring every future coding session to fetch and compare GitHub first, use only safe fast-forward pulls, preserve local work, read the development log and incoming changes, continue from the latest user request, keep UTF-8 CRLF, log every operation and keep project artifacts off the C drive.

## 2026-08-16 - Android live-view recovery and transfer task queue

- Fetched GitHub before editing and confirmed local `HEAD` and `origin/main` both pointed to `0bb787b3a09e245f488935f311279cf99b44c599`; no pull was required.
- Confirmed the tracked working tree was clean and preserved the three pre-existing untracked legacy runtime items (`LunaStudio.exe`, `LunaStudio.exe.WebView2/` and `LunaVirtualCamera.dll`).
- Re-read `AGENTS.md`, the latest development-log entries and the Android live-preview/direct-gallery implementation introduced by commit `309096f` before changing code.
- Diagnosed the Android live-view failure against the repository's captured 24,800,242-byte HEVC stream. The 2,224 UCD2 subtype-`0x20` packets are complete Annex-B access units; 46 keyframes contain VPS/SPS/PPS plus IDR and repeat approximately every two seconds.
- Identified two Android decoder reliability defects: the initial VPS/SPS/PPS were not supplied as codec-specific data, and a temporarily unavailable `MediaCodec` input buffer caused the complete access unit to be discarded, breaking inter-frame dependencies.
- Added a tested Annex-B HEVC parser that separates VPS/SPS/PPS from video samples, waits for a complete codec configuration plus IDR, configures `MediaCodec` with `csd-0`, preserves complete access units during input-buffer backpressure and rebuilds the decoder at the next keyframe after a codec failure.
- Removed the forced low-latency format key for broader hardware-codec compatibility, added a visible nonfatal resynchronization state while the decoder waits for the next keyframe, and added a six-second no-output watchdog for codecs that accept samples without producing frames.
- Replaced the camera downloader's single opaque `std::io::copy` with 256 KiB streaming progress, a two-hour whole-transfer timeout instead of the former 30-second limit, response-length validation and safe restart behavior when a camera ignores a resume Range request. The initial per-read-timeout attempt was rejected by the blocking Reqwest API during the first Android compile and was corrected before rebuilding.
- Added a Rust-to-Java task-event channel carrying throttled download progress, bytes, speed, ETA, current file and batch position for camera downloads and camera-original preparation.
- Added an Android FIFO transfer executor so camera downloads, original preparation, photo/video watermark export and MediaStore publication run one at a time without competing for camera or hardware-codec resources.
- Added Media3 Transformer progress polling for video watermarks and moved the final MediaStore copy off the main thread. Photo/video publication and batch gallery copies now report byte-level progress and flush output before marking the item complete.
- Added a compact shared task center with queued/running/completed/failed states, phase text, progress bars, bytes, speed, ETA, active-task badge and completed-history clearing; it adapts to a bottom sheet on Android-sized viewports.
- Verified the task center at 390x844 with no horizontal or element-boundary overflow, and verified the shared inline JavaScript parses successfully with Node.
- Passed `cargo fmt --all -- --check`, `cargo test --locked --bin Insta360Linker` (52 passed, 0 failed, 4 explicitly ignored physical/system tests), Android `testDebugUnitTest` (2 HEVC parser tests passed), and `lintDebug` (0 errors, 17 non-blocking existing/configuration warnings).
- Rebuilt the ARM64 debug APK and verified that it contains the current task-center HTML/CSS plus `lib/arm64-v8a/libinsta360_linker.so`; the native library exports both preview and task-event JNI poll methods. APK Signature Scheme v2 verification passed.
- No Android device was attached to ADB, so physical Luna Wi-Fi preview decoding, camera downloads and MediaStore publication remain device-verification items; this limitation was not treated as a successful hardware test.
- Final Android ARM64 debug APK: `Insta360Linker-android-arm64-debug.apk`, 12,278,378 bytes, SHA-256 `0c203fe22b4e9ff10e2aba5ba653e6c6f46f526b254739e6e4eabac2ab7015e5`.
- Permanently removed the Cargo target tree (17,770 generated files / 11.8 GiB), Gradle app/build reports, generated JNI library tree and visual-QA screenshot after verification; preserved the final APK, source files and the three pre-existing untracked legacy runtime items. Stopped the temporary ADB server.
- Committed the verified implementation as `7617e0a` (`fix: stabilize Android preview and transfer progress`) and pushed it successfully to GitHub `origin/main`; the three pre-existing untracked legacy runtime items were not staged or uploaded.

## 2026-08-16 - Android live-preview process-crash hardening

- Fetched GitHub before editing and confirmed local `HEAD` and `origin/main` both pointed to `c1fc6b574dbae451d18a2e0cfd47bb1f05d36881`; no pull was required and the tracked tree was clean.
- Re-read `AGENTS.md` and the latest Android preview/queue log after the user reported that tapping Start Preview closed the Android application before any frame appeared. Preserved the three pre-existing untracked legacy runtime items.
- Audited process-level failure paths that ordinary `Exception` handlers did not cover: uncaught `Throwable` values in the preview/image threads, codec release failures, and uncaught main-thread exceptions while injecting Base64 JPEG frames into WebView.
- Wrapped the complete preview worker and image callback in process-safe failure boundaries, made codec/ImageReader/HandlerThread cleanup individually non-throwing, and route preview status/error scripts through a guarded WebView evaluator.
- Added one-frame-in-flight backpressure between ImageReader and WebView and reduced UI delivery to approximately six frames per second so Java/Base64/WebView allocations cannot build an unbounded main-thread queue.
- Added regular-codec discovery with stable Android/Google software HEVC decoders preferred ahead of vendor hardware decoders. The selected decoder name is shown while synchronizing, and the existing next-keyframe recovery remains available if a candidate cannot start.
- Reproduced the transfer-state design flaw reported by the user: the old progress percentages were fixed stage weights, and `disconnect_luna` only closed UCD2 sessions while an independent blocking HTTP response could continue writing its cache file.
- Replaced the fixed `0..82..100` display with byte-derived `0..100` progress for the current camera-read, video-transform and MediaStore-copy phases. Batch payloads now carry camera-reported file sizes so aggregate bytes, speed and ETA come from actual transferred data.
- Added native per-task transfer controls exposed through JNI and the shared task center. Camera downloads and camera-original preparation now support Pause, Continue and Stop; paused loops stop consuming/writing chunks, and Stop is propagated to both Java and Rust instead of merely changing the visible task state.
- Made Disconnect Luna cancel every active/queued camera-transfer task on both sides of JNI before closing the sessions. This also wakes paused Java and Rust workers, so a paused transfer cannot remain stuck after disconnection or silently continue in the background.
- Made camera downloads fail closed: any HTTP/status/read/length/cancellation error removes the `.part` file, and disconnect/session-epoch changes are checked during streaming. Added a local HTTP regression test proving callback cancellation returns an error and leaves neither the final file nor its partial file.
- Made Android gallery publication cancellation-aware at every 256 KiB copy. Failed or stopped MediaStore writes delete their pending URI, legacy writes delete their destination, and source export/cache files are removed on failure or Stop.
- Moved direct-camera watermark originals into the Android application cache under `exports/watermark_sources`; successful cached originals may be reused honestly as a local-cache phase, while interrupted copies and stopped task outputs are addressable by the same cleanup boundary.
- Added paused/cancelled task states and task-card controls to the daily UI. Stopped tasks remain visible as stopped, advertise that unfinished cache was deleted, and can be cleared with the other completed history.
- Hardened the live-preview process path in the same APK: caught process-level decoder/image/WebView failures, made codec cleanup non-throwing, preferred stable software HEVC candidates, and limited WebView delivery to one approximately-6-fps JPEG in flight.
- Passed `cargo fmt --all --check`, `cargo test --locked --bin Insta360Linker` (53 passed, 0 failed, 4 explicitly ignored), Android `testDebugUnitTest`, Android `lintDebug`, the complete ARM64 debug APK build and shared inline-JavaScript parsing.
- Verified the APK contains the updated task controls, four DEX files, shared HTML/CSS and `lib/arm64-v8a/libinsta360_linker.so`; the native library exports `nativeControlTransfer`, `nativePollPreview` and `nativePollTaskEvent`. APK Signature Scheme v2 verification passed.
- No Android device was attached to ADB, so physical Luna Wi-Fi cancellation, pause/resume timing and device-specific HEVC decoder behavior remain explicit on-device verification items rather than claimed successes.
- Final Android ARM64 debug APK before generated-output cleanup: `Insta360Linker-android-arm64-debug.apk`, 18,129,648 bytes, SHA-256 `27b821424c9e52d6e091bc98e00d26e2f66232405cccfb8d6f0f6bc856245d8e`.
- Permanently removed the Cargo target tree (4,707 files / 3.3 GiB), Gradle app/build reports and generated JNI library tree after verification, then stopped the temporary ADB server. Preserved the final APK, source files and the three pre-existing untracked legacy runtime items.
- Committed the verified implementation as `d716517` (`fix: make Android transfers cancellable and crash-safe`) and pushed it successfully to GitHub `origin/main`; the three pre-existing untracked legacy runtime items were not staged or uploaded.
