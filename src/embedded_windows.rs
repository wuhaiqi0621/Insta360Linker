#![cfg(windows)]

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::Context;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

include!(concat!(env!("OUT_DIR"), "/embedded_windows_resources.rs"));

static FFMPEG_PATH: OnceLock<Result<PathBuf, String>> = OnceLock::new();
static VIRTUAL_CAMERA_DLL_PATH: OnceLock<Result<PathBuf, String>> = OnceLock::new();

pub fn has_embedded_ffmpeg() -> bool {
    EMBEDDED_WINDOWS_FFMPEG.is_some()
}

pub fn has_embedded_virtual_camera_dll() -> bool {
    EMBEDDED_WINDOWS_VIRTUAL_CAMERA_DLL.is_some()
}

pub fn ffmpeg_path() -> anyhow::Result<PathBuf> {
    cached_resource_path(
        &FFMPEG_PATH,
        EMBEDDED_WINDOWS_FFMPEG,
        EMBEDDED_WINDOWS_FFMPEG_FILENAME,
        EMBEDDED_WINDOWS_FFMPEG_HASH,
        "FFmpeg",
    )
}

pub fn virtual_camera_dll_path() -> anyhow::Result<PathBuf> {
    cached_resource_path(
        &VIRTUAL_CAMERA_DLL_PATH,
        EMBEDDED_WINDOWS_VIRTUAL_CAMERA_DLL,
        EMBEDDED_WINDOWS_VIRTUAL_CAMERA_DLL_FILENAME,
        EMBEDDED_WINDOWS_VIRTUAL_CAMERA_DLL_HASH,
        "虚拟摄像机",
    )
}

pub fn watermark_asset(file: &str) -> Option<&'static [u8]> {
    embedded_windows_watermark(file)
}

pub fn third_party_licenses() -> Option<&'static str> {
    EMBEDDED_WINDOWS_FFMPEG_LICENSE
}

pub fn verify_single_file_resources() -> anyhow::Result<String> {
    let ffmpeg = ffmpeg_path()?;
    let virtual_camera = virtual_camera_dll_path()?;
    if EMBEDDED_WINDOWS_WATERMARK_COUNT == 0
        || watermark_asset("ic_watermark_luna_ultra_image_cn.png").is_none()
    {
        anyhow::bail!("EXE 未内置官方水印资源");
    }
    if third_party_licenses().is_none_or(str::is_empty) {
        anyhow::bail!("EXE 未内置 FFmpeg 许可证");
    }
    Ok(format!(
        "single-file resources ok: ffmpeg={} virtual_camera={} watermarks={}",
        ffmpeg.display(),
        virtual_camera.display(),
        EMBEDDED_WINDOWS_WATERMARK_COUNT
    ))
}

fn cached_resource_path(
    cache: &'static OnceLock<Result<PathBuf, String>>,
    bytes: Option<&'static [u8]>,
    filename: &'static str,
    expected_hash: u64,
    label: &str,
) -> anyhow::Result<PathBuf> {
    cache
        .get_or_init(|| {
            let bytes = bytes.ok_or_else(|| format!("当前 EXE 未内置{label}组件"))?;
            materialize(filename, bytes, expected_hash).map_err(|error| error.to_string())
        })
        .clone()
        .map_err(anyhow::Error::msg)
}

fn materialize(filename: &str, bytes: &[u8], expected_hash: u64) -> anyhow::Result<PathBuf> {
    if filename.is_empty() || bytes.is_empty() {
        anyhow::bail!("内置运行时资源为空");
    }
    let root = runtime_root();
    fs::create_dir_all(&root).with_context(|| format!("无法创建运行时目录：{}", root.display()))?;
    let target = root.join(filename);
    if file_matches(&target, bytes.len() as u64, expected_hash)? {
        return Ok(target);
    }

    let temporary = root.join(format!(".{filename}.{}.tmp", std::process::id()));
    if temporary.exists() {
        fs::remove_file(&temporary)
            .with_context(|| format!("无法清理临时运行时文件：{}", temporary.display()))?;
    }
    let write_result = (|| -> anyhow::Result<()> {
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .with_context(|| format!("无法创建运行时文件：{}", temporary.display()))?;
        output
            .write_all(bytes)
            .with_context(|| format!("无法释放运行时文件：{}", temporary.display()))?;
        output.sync_all()?;
        if target.exists() {
            if file_matches(&target, bytes.len() as u64, expected_hash)? {
                return Ok(());
            }
            fs::remove_file(&target)
                .with_context(|| format!("无法替换运行时文件：{}", target.display()))?;
        }
        if let Err(error) = fs::rename(&temporary, &target) {
            if file_matches(&target, bytes.len() as u64, expected_hash)? {
                return Ok(());
            }
            return Err(error).with_context(|| {
                format!(
                    "无法完成运行时文件释放：{} -> {}",
                    temporary.display(),
                    target.display()
                )
            });
        }
        Ok(())
    })();
    if temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }
    write_result?;
    if !file_matches(&target, bytes.len() as u64, expected_hash)? {
        anyhow::bail!("运行时文件校验失败：{}", target.display());
    }
    Ok(target)
}

fn runtime_root() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Insta360Linker")
        .join("Runtime")
        .join(env!("CARGO_PKG_VERSION"))
}

fn file_matches(path: &Path, expected_len: u64, expected_hash: u64) -> anyhow::Result<bool> {
    let Ok(metadata) = path.metadata() else {
        return Ok(false);
    };
    if !metadata.is_file() || metadata.len() != expected_len {
        return Ok(false);
    }
    Ok(fnv1a_file(path)? == expected_hash)
}

fn fnv1a_file(path: &Path) -> anyhow::Result<u64> {
    let mut reader = BufReader::new(
        File::open(path).with_context(|| format!("无法校验运行时文件：{}", path.display()))?,
    );
    let mut buffer = [0u8; 64 * 1024];
    let mut hash = FNV_OFFSET_BASIS;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    Ok(hash)
}
