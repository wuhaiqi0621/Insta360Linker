param(
    [string]$ToolchainRoot = $(if ($env:ANDROID_TOOLCHAIN_ROOT) { $env:ANDROID_TOOLCHAIN_ROOT } else { "F:\AndroidToolchain" }),
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$sdk = Join-Path $ToolchainRoot "sdk"
$ndk = Join-Path $sdk "ndk\28.2.13676358"
$javaHome = Join-Path $ToolchainRoot "jdk\jdk-17.0.20+8"
$cargoHome = Join-Path $ToolchainRoot "cargo-home"
$rustupHome = Join-Path $ToolchainRoot "rustup-home"
$toolchain = Join-Path $ndk "toolchains\llvm\prebuilt\windows-x86_64\bin"
$linker = Join-Path $toolchain "aarch64-linux-android26-clang.cmd"
$portableGradle = Join-Path $ToolchainRoot "gradle\gradle-9.4.1\bin\gradle.bat"

foreach ($required in @(
    (Join-Path $javaHome "bin\java.exe"),
    (Join-Path $cargoHome "bin\cargo.exe"),
    $linker
)) {
    if (-not (Test-Path -LiteralPath $required)) {
        throw "Missing Android build dependency: $required"
    }
}

$env:JAVA_HOME = $javaHome
$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_USER_HOME = Join-Path $ToolchainRoot "android-home"
$env:GRADLE_USER_HOME = Join-Path $ToolchainRoot "gradle-home"
$env:CARGO_HOME = $cargoHome
$env:RUSTUP_HOME = $rustupHome
$env:PATH = "$cargoHome\bin;$javaHome\bin;$env:PATH"
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $linker
$env:CC_aarch64_linux_android = $linker
$env:AR_aarch64_linux_android = Join-Path $toolchain "llvm-ar.exe"

Push-Location $repoRoot
try {
    cargo build --lib --release --target aarch64-linux-android --locked
    if ($LASTEXITCODE -ne 0) { throw "Rust Android build failed" }

    $jniDirectory = Join-Path $PSScriptRoot "app\src\main\jniLibs\arm64-v8a"
    New-Item -ItemType Directory -Force -Path $jniDirectory | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot "target\aarch64-linux-android\release\libinsta360_linker.so") `
        -Destination (Join-Path $jniDirectory "libinsta360_linker.so") -Force

    Push-Location $PSScriptRoot
    try {
        $gradle = if (Test-Path -LiteralPath $portableGradle) {
            $portableGradle
        } else {
            Join-Path $PSScriptRoot "gradlew.bat"
        }
        & $gradle "assemble$Configuration" --no-daemon
        if ($LASTEXITCODE -ne 0) { throw "Gradle Android build failed" }
    } finally {
        Pop-Location
    }

    $variant = $Configuration.ToLowerInvariant()
    $apk = Join-Path $PSScriptRoot "app\build\outputs\apk\$variant\app-$variant.apk"
    $destination = Join-Path $repoRoot "Insta360Linker-android-arm64-$variant.apk"
    Copy-Item -LiteralPath $apk -Destination $destination -Force
    Write-Host "Android APK: $destination"
} finally {
    Pop-Location
}
