# Insta360Linker for Android

The Android app uses a native Java WebView host and the same HTML UI as the Windows build. Camera protocol handling, media sessions, capture controls and photo watermark rendering run in the shared Rust library through JNI.

## Current Android capabilities

- Luna Ultra Wi-Fi detection and persistent UCD2 session.
- Internal storage and SD card media listing.
- Photo and video thumbnails using Android decoders.
- Multi-select save and delete. Saving does not open a destination picker; completed photos and videos are published directly to the system gallery under `DCIM/Insta360Linker`.
- Photo/video mode switching, zoom, recording profile, shutter and gimbal controls.
- Photo/video watermark preview and export, including official and custom Luna Moment artwork. Final exports are also saved directly to the system gallery.
- Native Android BLE scan for Mic Pro devices.

Windows virtual-camera registration is intentionally unavailable on Android. Live HEVC preview is not enabled until the UCD2 stream is connected to an Android MediaCodec decoder.

## Build with GitHub Actions

Run the `Build Android ARM64 APK` workflow manually from the repository Actions page. The Ubuntu job installs Java 17, Android API/Build Tools 36, NDK 28.2.13676358 and the Rust Android ARM64 target, then runs Android Lint and builds a debug-signed APK.

The successful run uploads a 14-day `Insta360Linker-android-arm64-debug` artifact containing the installable APK and `SHA256SUMS.txt`. The workflow also verifies the APK signature and checks that the ARM64 JNI library, Web UI and official watermark resources are packaged.

## Build on Windows

The checked-in wrapper uses Gradle 9.4.1 and Android Gradle Plugin 9.2.0. The local build script expects JDK 17, Android API/Build Tools 36, NDK 28.2.13676358 and the Rust `aarch64-linux-android` target under `F:\AndroidToolchain` by default.

```powershell
cd F:\Insta360onWin\android
.\build_android.ps1 -Configuration Debug
```

Set `ANDROID_TOOLCHAIN_ROOT` or pass `-ToolchainRoot` to use another toolchain directory. The resulting APK is copied to the repository root as `Insta360Linker-android-arm64-debug.apk`.
