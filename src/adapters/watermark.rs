use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont, point};
use anyhow::{Context, anyhow, bail};
use exif::{In, Tag, Value};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage, codecs::jpeg::JpegEncoder, imageops};
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const WATERMARK_CONFIG: &str =
    include_str!("../../assets/apk_watermark/Watermark_Config_Table.txt");
const IMAGE_WATERMARK_CONFIG: &str =
    include_str!("../../assets/apk_watermark/Image_Watermark_Config_Table.txt");
const FRAME_WATERMARK_CONFIG: &str =
    include_str!("../../assets/apk_watermark/Frame_Watermark_Config_Table.txt");
const FRAME_TEXT_FONT: &[u8] = include_bytes!("../../assets/apk_watermark/BeihaibeiSC-Regular.ttf");
const SHENSHEN_MOMENT_PRESET: &[u8] =
    include_bytes!("../../assets/moment_presets/shenshen-concert.jpg");
const FRAME_TEXT_SCALE: f32 = 1.16;
const FRAME_TEXT_TRACKING_RATIO: f32 = 0.045;

#[derive(Clone)]
pub struct WatermarkOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub position: String,
    pub style: String,
    pub frame_background: String,
    pub moment_preset: String,
    pub moment_image: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct WatermarkStyle {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub model: &'static str,
    pub profile: &'static str,
    pub image_file: Option<&'static str>,
    pub video_file: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct FrameBackgroundStyle {
    pub id: &'static str,
    pub label: &'static str,
    pub start_hex: &'static str,
    pub end_hex: &'static str,
}

#[derive(Debug, Clone, Copy)]
enum MediaKind {
    Image,
    Video,
}

#[derive(Debug, Clone, Copy)]
struct AppPlacement {
    width_ratio: f32,
    x_ratio: f32,
    bottom_ratio: f32,
}

#[derive(Debug, Clone, Copy)]
struct FramePlacement {
    width_scale: f32,
    height_scale: f32,
    photo_bottom_ratio: f32,
    watermark_width_ratio: f32,
    capture_font_ratio: f32,
    capture_spacing_ratio: f32,
    timestamp_font_ratio: f32,
    line_spacing_ratio: f32,
    moment_width_ratio: f32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct FrameMetadata {
    aperture: Option<String>,
    exposure: Option<String>,
    iso: Option<String>,
    timestamp: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct FrameBackground {
    start: [u8; 3],
    end: [u8; 3],
    foreground: [u8; 3],
}

const WATERMARK_STYLES: [WatermarkStyle; 4] = [
    WatermarkStyle {
        id: "luna-ultra-cn",
        label: "中文",
        kind: "mark",
        model: "Luna Ultra",
        profile: "Leica-CN",
        image_file: Some("ic_watermark_luna_ultra_image_cn.png"),
        video_file: Some("ic_watermark_luna_ultra_cn.png"),
    },
    WatermarkStyle {
        id: "luna-ultra",
        label: "标准",
        kind: "mark",
        model: "Luna Ultra",
        profile: "Leica",
        image_file: Some("ic_watermark_luna_ultra_image.png"),
        video_file: Some("ic_watermark_luna_ultra.png"),
    },
    WatermarkStyle {
        id: "luna-ultra-zstyle-cn",
        label: "外框水印（中文）",
        kind: "frame",
        model: "Luna Ultra",
        profile: "ZStyle-CN",
        image_file: Some("ic_watermark_luna_ultra_cn.png"),
        video_file: None,
    },
    WatermarkStyle {
        id: "luna-ultra-zstyle",
        label: "外框水印",
        kind: "frame",
        model: "Luna Ultra",
        profile: "ZStyle",
        image_file: Some("ic_watermark_luna_ultra.png"),
        video_file: None,
    },
];

const FRAME_BACKGROUNDS: [FrameBackgroundStyle; 5] = [
    FrameBackgroundStyle {
        id: "black",
        label: "黑色",
        start_hex: "#000000",
        end_hex: "#000000",
    },
    FrameBackgroundStyle {
        id: "white",
        label: "白色",
        start_hex: "#f4f4f2",
        end_hex: "#f4f4f2",
    },
    FrameBackgroundStyle {
        id: "photo-dark",
        label: "照片深色",
        start_hex: "#28353d",
        end_hex: "#28353d",
    },
    FrameBackgroundStyle {
        id: "photo-light",
        label: "照片浅色",
        start_hex: "#dce4e7",
        end_hex: "#dce4e7",
    },
    FrameBackgroundStyle {
        id: "photo-gradient",
        label: "照片渐变",
        start_hex: "#263d53",
        end_hex: "#593848",
    },
];

pub fn styles() -> Vec<WatermarkStyle> {
    WATERMARK_STYLES.to_vec()
}

pub fn frame_backgrounds() -> Vec<FrameBackgroundStyle> {
    FRAME_BACKGROUNDS.to_vec()
}

pub fn apply(options: &WatermarkOptions) -> anyhow::Result<()> {
    let style = style_for(&options.style)?;
    let ext = media_extension(&options.input);
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => {
            if style.kind == "frame" {
                apply_zstyle_frame(options, style)
            } else {
                apply_image_mark(options, style)
            }
        }
        "mp4" | "mov" | "mkv" | "avi" | "m4v" => {
            if style.kind == "frame" {
                bail!("外框水印是 Insta360 App 的照片水印，不能用于视频");
            }
            apply_video_mark(options, style)
        }
        _ => Err(anyhow!("不支持的媒体类型：{ext}")),
    }
}

pub fn preview(
    input: &Path,
    style_id: &str,
    position: &str,
    frame_background: &str,
    moment_preset: &str,
    moment_image: Option<&Path>,
    max_width: u32,
    max_height: u32,
) -> anyhow::Result<Vec<u8>> {
    let style = style_for(style_id)?;
    let ext = media_extension(input);
    let (source, kind) = match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => (
            image::open(input)
                .with_context(|| format!("无法读取图片：{}", input.display()))?
                .thumbnail(max_width.saturating_mul(2), max_height.saturating_mul(2))
                .to_rgba8(),
            MediaKind::Image,
        ),
        "mp4" | "mov" | "mkv" | "avi" | "m4v" => {
            if style.kind == "frame" {
                bail!("外框水印是 Insta360 App 的照片水印，不能用于视频");
            }
            (extract_video_preview_frame(input)?, MediaKind::Video)
        }
        _ => bail!("不支持的媒体类型：{ext}"),
    };

    let rendered = if style.kind == "frame" {
        render_zstyle_frame(
            source,
            style,
            &read_frame_metadata(input),
            frame_background,
            moment_preset,
            moment_image,
        )?
    } else {
        render_mark(source, style, kind, position)?
    };
    let preview = DynamicImage::ImageRgba8(rendered)
        .thumbnail(max_width.max(1), max_height.max(1))
        .to_rgb8();
    let mut encoded = Vec::new();
    JpegEncoder::new_with_quality(&mut encoded, 88)
        .encode_image(&DynamicImage::ImageRgb8(preview))
        .context("无法编码水印预览")?;
    Ok(encoded)
}

fn apply_image_mark(options: &WatermarkOptions, style: &WatermarkStyle) -> anyhow::Result<()> {
    let base = image::open(&options.input)
        .with_context(|| format!("无法读取图片：{}", options.input.display()))?
        .to_rgba8();
    let rendered = render_mark(base, style, MediaKind::Image, &options.position)?;
    save_image(DynamicImage::ImageRgba8(rendered), &options.output)
}

fn render_mark(
    mut base: RgbaImage,
    style: &WatermarkStyle,
    kind: MediaKind,
    position: &str,
) -> anyhow::Result<RgbaImage> {
    let placement = app_placement(style, base.width(), base.height(), kind, position)
        .ok_or_else(|| anyhow!("该媒体比例或位置不在 Insta360 App 的水印参数表中"))?;
    let mut watermark = image::load_from_memory(&load_style_image(style, kind)?)?.to_rgba8();
    let target_width = (base.width() as f32 * placement.width_ratio)
        .round()
        .max(1.0) as u32;
    let target_height = scaled_height(&watermark, target_width);
    watermark = imageops::resize(
        &watermark,
        target_width,
        target_height,
        imageops::FilterType::Lanczos3,
    );
    let (x, y) = app_coordinates(
        base.width(),
        base.height(),
        watermark.width(),
        watermark.height(),
        placement,
    );
    imageops::overlay(&mut base, &watermark, x.into(), y.into());
    Ok(base)
}

fn apply_video_mark(options: &WatermarkOptions, style: &WatermarkStyle) -> anyhow::Result<()> {
    let (width, height) = video_dimensions(&options.input)?;
    let placement = app_placement(style, width, height, MediaKind::Video, &options.position)
        .ok_or_else(|| anyhow!("该视频比例或位置不在 Insta360 App 的水印参数表中"))?;
    let watermark_path = temporary_watermark_path(options, style);
    std::fs::write(&watermark_path, load_style_image(style, MediaKind::Video)?)
        .context("无法准备视频水印资源")?;
    if let Some(parent) = options.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let filter = format!(
        "[1:v][0:v]scale2ref=w=main_w*{:.6}:h=-1[wm][base];[base][wm]overlay=x=main_w*{:.6}:y=main_h*(1-{:.6})-overlay_h:format=auto[out]",
        placement.width_ratio, placement.x_ratio, placement.bottom_ratio
    );
    let command_result = Command::new(ffmpeg_binary())
        .args(["-y", "-i"])
        .arg(&options.input)
        .args(["-loop", "1", "-i"])
        .arg(&watermark_path)
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[out]",
            "-map",
            "0:a?",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "copy",
            "-shortest",
            "-movflags",
            "+faststart",
        ])
        .arg(&options.output)
        .output()
        .context("无法启动视频导出组件")?;
    let _ = std::fs::remove_file(&watermark_path);
    if !command_result.status.success() {
        bail!(
            "视频水印导出失败：{}",
            String::from_utf8_lossy(&command_result.stderr).trim()
        );
    }
    Ok(())
}

fn apply_zstyle_frame(options: &WatermarkOptions, style: &WatermarkStyle) -> anyhow::Result<()> {
    let source = image::open(&options.input)
        .with_context(|| format!("无法读取图片：{}", options.input.display()))?
        .to_rgba8();
    let rendered = render_zstyle_frame(
        source,
        style,
        &read_frame_metadata(&options.input),
        &options.frame_background,
        &options.moment_preset,
        options.moment_image.as_deref(),
    )?;
    save_image(DynamicImage::ImageRgba8(rendered), &options.output)
}

fn render_zstyle_frame(
    source: RgbaImage,
    style: &WatermarkStyle,
    metadata: &FrameMetadata,
    background_id: &str,
    moment_preset: &str,
    moment_image: Option<&Path>,
) -> anyhow::Result<RgbaImage> {
    let config = frame_placement(style, source.width(), source.height())
        .ok_or_else(|| anyhow!("该照片比例不在 Insta360 App 的外框水印参数表中"))?;
    let canvas_width = (source.width() as f32 * config.width_scale)
        .ceil()
        .max(source.width() as f32) as u32;
    let canvas_height = (source.height() as f32 * config.height_scale)
        .ceil()
        .max(source.height() as f32) as u32;
    let photo_bottom = (canvas_height as f32 * config.photo_bottom_ratio)
        .ceil()
        .max(0.0) as u32;
    let photo_x = (canvas_width.saturating_sub(source.width())) / 2;
    let photo_y = canvas_height.saturating_sub(source.height().saturating_add(photo_bottom));

    let background = resolve_frame_background(&source, background_id);
    let mut frame = render_frame_background(canvas_width, canvas_height, background);
    imageops::overlay(&mut frame, &source, photo_x.into(), photo_y.into());

    let mut logo = image::load_from_memory(&load_style_image(style, MediaKind::Image)?)?.to_rgba8();
    tint_frame_logo(&mut logo, background.foreground);
    let target_width = (canvas_width as f32 * config.watermark_width_ratio)
        .round()
        .max(1.0) as u32;
    let target_height = scaled_height(&logo, target_width);
    logo = imageops::resize(
        &logo,
        target_width,
        target_height,
        imageops::FilterType::Lanczos3,
    );
    let logo_x = (canvas_width.saturating_sub(logo.width())) / 2;
    let logo_y = photo_y.saturating_sub(logo.height()) / 2;
    imageops::overlay(&mut frame, &logo, logo_x.into(), logo_y.into());
    render_zstyle_footer(
        &mut frame,
        photo_y.saturating_add(source.height()),
        config,
        metadata,
        background.foreground,
        moment_preset,
        moment_image,
    )?;
    Ok(frame)
}

fn resolve_frame_background(source: &RgbaImage, id: &str) -> FrameBackground {
    let (top, bottom) = sample_photo_colors(source);
    let (start, end) = match id {
        "white" => ([244, 244, 242], [244, 244, 242]),
        "photo-dark" => {
            let color = mix_rgb(mix_rgb(top, bottom, 0.5), [0, 0, 0], 0.62);
            (color, color)
        }
        "photo-light" => {
            let color = mix_rgb(mix_rgb(top, bottom, 0.5), [255, 255, 255], 0.76);
            (color, color)
        }
        "photo-gradient" => (
            mix_rgb(top, [0, 0, 0], 0.52),
            mix_rgb(bottom, [0, 0, 0], 0.52),
        ),
        _ => ([0, 0, 0], [0, 0, 0]),
    };
    let average = mix_rgb(start, end, 0.5);
    let foreground = if relative_luminance(average) > 0.56 {
        [24, 24, 24]
    } else {
        [244, 244, 244]
    };
    FrameBackground {
        start,
        end,
        foreground,
    }
}

fn sample_photo_colors(source: &RgbaImage) -> ([u8; 3], [u8; 3]) {
    let step_x = (source.width() / 64).max(1) as usize;
    let step_y = (source.height() / 64).max(1) as usize;
    let mut top = [0_u64; 3];
    let mut bottom = [0_u64; 3];
    let mut top_count = 0_u64;
    let mut bottom_count = 0_u64;
    for y in (0..source.height()).step_by(step_y) {
        for x in (0..source.width()).step_by(step_x) {
            let pixel = source.get_pixel(x, y);
            let (sum, count) = if y.saturating_mul(2) < source.height() {
                (&mut top, &mut top_count)
            } else {
                (&mut bottom, &mut bottom_count)
            };
            for channel in 0..3 {
                sum[channel] += u64::from(pixel[channel]);
            }
            *count += 1;
        }
    }
    (
        average_rgb(top, top_count),
        average_rgb(bottom, bottom_count),
    )
}

fn average_rgb(sum: [u64; 3], count: u64) -> [u8; 3] {
    let count = count.max(1);
    [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    ]
}

fn mix_rgb(left: [u8; 3], right: [u8; 3], amount: f32) -> [u8; 3] {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |channel: usize| {
        (left[channel] as f32 * (1.0 - amount) + right[channel] as f32 * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [mix(0), mix(1), mix(2)]
}

fn relative_luminance(color: [u8; 3]) -> f32 {
    let linear = |value: u8| {
        let value = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
}

fn render_frame_background(width: u32, height: u32, background: FrameBackground) -> RgbaImage {
    RgbaImage::from_fn(width, height, |_, y| {
        let amount = y as f32 / height.saturating_sub(1).max(1) as f32;
        let color = mix_rgb(background.start, background.end, amount);
        Rgba([color[0], color[1], color[2], 255])
    })
}

fn tint_frame_logo(logo: &mut RgbaImage, foreground: [u8; 3]) {
    let mut red_bounds: Option<(u32, u32, u32, u32)> = None;
    for (x, y, pixel) in logo.enumerate_pixels() {
        let leica_red = pixel[3] > 0
            && pixel[0] > 110
            && u16::from(pixel[0]) > u16::from(pixel[1]).saturating_mul(3) / 2
            && u16::from(pixel[0]) > u16::from(pixel[2]).saturating_mul(3) / 2;
        if leica_red {
            red_bounds = Some(match red_bounds {
                Some((left, top, right, bottom)) => {
                    (left.min(x), top.min(y), right.max(x), bottom.max(y))
                }
                None => (x, y, x, y),
            });
        }
    }
    let red_bounds = red_bounds.map(|(left, top, right, bottom)| {
        (
            left.saturating_sub(2),
            top.saturating_sub(2),
            right.saturating_add(2).min(logo.width().saturating_sub(1)),
            bottom
                .saturating_add(2)
                .min(logo.height().saturating_sub(1)),
        )
    });
    for (x, y, pixel) in logo.enumerate_pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        if red_bounds.is_some_and(|(left, top, right, bottom)| {
            x >= left && x <= right && y >= top && y <= bottom
        }) {
            continue;
        }
        pixel[0] = foreground[0];
        pixel[1] = foreground[1];
        pixel[2] = foreground[2];
    }
}

fn prepare_moment_asset(moment: &mut RgbaImage, foreground: [u8; 3]) {
    for pixel in moment.pixels_mut() {
        let luminance = pixel[0].max(pixel[1]).max(pixel[2]);
        let alpha = (u16::from(pixel[3]) * u16::from(luminance) / 255) as u8;
        *pixel = Rgba([foreground[0], foreground[1], foreground[2], alpha]);
    }
}

fn render_zstyle_footer(
    frame: &mut RgbaImage,
    photo_bottom: u32,
    config: FramePlacement,
    metadata: &FrameMetadata,
    foreground: [u8; 3],
    moment_preset: &str,
    moment_image: Option<&Path>,
) -> anyhow::Result<()> {
    let panel_height = frame.height().saturating_sub(photo_bottom);
    if panel_height == 0 {
        return Ok(());
    }

    let (mut moment, use_fixed_height) =
        load_moment_image(moment_preset, moment_image, foreground)?;
    let moment_max_width = (frame.width() as f32 * config.moment_width_ratio)
        .round()
        .max(1.0) as u32;
    let official_moment =
        image::load_from_memory(&load_runtime_asset("choose_logo_photo_moment.png")?)?.to_rgba8();
    let official_moment_height = scaled_height(&official_moment, moment_max_width);
    let (moment_width, moment_height) = if use_fixed_height {
        scale_to_fixed_height(
            moment.width(),
            moment.height(),
            official_moment_height,
            frame.width(),
        )
    } else {
        (moment_max_width, scaled_height(&moment, moment_max_width))
    };
    moment = imageops::resize(
        &moment,
        moment_width,
        moment_height,
        imageops::FilterType::Lanczos3,
    );
    let moment_x = (frame.width().saturating_sub(moment.width())) / 2;
    let moment_y = photo_bottom + (panel_height as f32 * 0.1460).round() as u32;
    imageops::overlay(frame, &moment, moment_x.into(), moment_y.into());

    let capture_line = metadata.capture_segments();
    let timestamp_line = metadata.timestamp.as_deref();
    if capture_line.is_none() && timestamp_line.is_none() {
        return Ok(());
    }
    let font = load_frame_font()?;
    let capture_y = photo_bottom + (panel_height as f32 * 0.6184).round() as u32;
    if let Some(segments) = capture_line {
        let font_size = frame.height() as f32 * config.capture_font_ratio * FRAME_TEXT_SCALE;
        let gap = (frame.width() as f32 * config.capture_spacing_ratio)
            .round()
            .max(0.0) as u32;
        draw_centered_segments(
            frame, &font, font_size, &segments, gap, capture_y, foreground,
        )?;
    }
    if let Some(timestamp) = timestamp_line {
        let font_size = frame.height() as f32 * config.timestamp_font_ratio * FRAME_TEXT_SCALE;
        let timestamp_y = capture_y
            + (frame.height() as f32
                * (config.capture_font_ratio * FRAME_TEXT_SCALE + config.line_spacing_ratio))
                .round()
                .max(1.0) as u32;
        draw_centered_text(frame, &font, font_size, timestamp, timestamp_y, foreground)?;
    }
    Ok(())
}

fn load_moment_image(
    preset: &str,
    custom_path: Option<&Path>,
    foreground: [u8; 3],
) -> anyhow::Result<(RgbaImage, bool)> {
    match preset {
        "shenshen-concert" => Ok((
            image::load_from_memory(SHENSHEN_MOMENT_PRESET)
                .context("无法加载深深的巡演 Moment 预设")?
                .to_rgba8(),
            true,
        )),
        "custom" => {
            let path = custom_path.ok_or_else(|| anyhow!("请先选择自定义 Luna Moment 图片"))?;
            Ok((
                image::open(path)
                    .with_context(|| {
                        format!("无法读取自定义 Luna Moment 图片：{}", path.display())
                    })?
                    .to_rgba8(),
                true,
            ))
        }
        _ => {
            let mut official =
                image::load_from_memory(&load_runtime_asset("choose_logo_photo_moment.png")?)?
                    .to_rgba8();
            prepare_moment_asset(&mut official, foreground);
            Ok((official, false))
        }
    }
}

fn scale_to_fixed_height(
    width: u32,
    height: u32,
    target_height: u32,
    canvas_width: u32,
) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let target_height = target_height.max(1);
    let scaled_width = (width as f64 * target_height as f64 / height as f64)
        .round()
        .max(1.0) as u32;
    let canvas_width = canvas_width.max(1);
    if scaled_width <= canvas_width {
        return (scaled_width, target_height);
    }

    let scaled_height = (height as f64 * canvas_width as f64 / width as f64)
        .round()
        .max(1.0) as u32;
    (canvas_width, scaled_height)
}

impl FrameMetadata {
    fn capture_segments(&self) -> Option<[String; 3]> {
        Some([
            self.aperture.clone()?,
            self.exposure.clone()?,
            self.iso.clone()?,
        ])
    }
}

fn read_frame_metadata(input: &Path) -> FrameMetadata {
    let Ok(file) = File::open(input) else {
        return FrameMetadata::default();
    };
    let Ok(exif) = exif::Reader::new().read_from_container(&mut BufReader::new(file)) else {
        return FrameMetadata::default();
    };
    FrameMetadata {
        aperture: exif_rational(&exif, Tag::FNumber).map(format_aperture),
        exposure: exif_rational(&exif, Tag::ExposureTime)
            .map(format_exposure)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                exif_signed_rational(&exif, Tag::ShutterSpeedValue).and_then(format_shutter_apex)
            }),
        iso: exif_integer(&exif, Tag::PhotographicSensitivity).map(|iso| format!("ISO{iso}")),
        timestamp: format_exif_timestamp(
            exif_ascii(&exif, Tag::DateTimeOriginal).as_deref(),
            exif_ascii(&exif, Tag::OffsetTimeOriginal).as_deref(),
        ),
    }
}

fn format_aperture(value: exif::Rational) -> String {
    format!("F/{:.1}", value.num as f64 / value.denom.max(1) as f64)
}

fn exif_rational(exif: &exif::Exif, tag: Tag) -> Option<exif::Rational> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Rational(values) => values.first().copied(),
        _ => None,
    }
}

fn exif_signed_rational(exif: &exif::Exif, tag: Tag) -> Option<exif::SRational> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::SRational(values) => values.first().copied(),
        _ => None,
    }
}

fn exif_integer(exif: &exif::Exif, tag: Tag) -> Option<u32> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    match &field.value {
        Value::Byte(values) => values.first().copied().map(u32::from),
        Value::Short(values) => values.first().copied().map(u32::from),
        Value::Long(values) => values.first().copied(),
        _ => None,
    }
}

fn exif_ascii(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let field = exif.get_field(tag, In::PRIMARY)?;
    let Value::Ascii(values) = &field.value else {
        return None;
    };
    let value = values.first()?;
    let text = String::from_utf8_lossy(value)
        .trim_matches(char::from(0))
        .trim()
        .to_string();
    (!text.is_empty()).then_some(text)
}

fn format_exposure(value: exif::Rational) -> String {
    if value.denom == 0 {
        return String::new();
    }
    format_shutter_seconds(value.num as f64 / value.denom as f64).unwrap_or_default()
}

fn format_shutter_apex(value: exif::SRational) -> Option<String> {
    if value.denom == 0 {
        return None;
    }
    let apex = value.num as f64 / value.denom as f64;
    format_shutter_seconds(2.0_f64.powf(-apex))
}

fn format_shutter_seconds(seconds: f64) -> Option<String> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    if seconds < 1.0 {
        const CAMERA_SHUTTER_DENOMINATORS: &[u32] = &[
            1, 2, 3, 4, 5, 6, 8, 10, 13, 15, 16, 20, 25, 30, 40, 50, 60, 80, 100, 120, 125, 160,
            200, 240, 250, 320, 400, 500, 640, 800, 1000, 1250, 1600, 2000, 2500, 3200, 4000, 5000,
            6400, 8000, 10000, 12800, 16000, 20000, 24000,
        ];
        let reciprocal = 1.0 / seconds;
        let denominator = CAMERA_SHUTTER_DENOMINATORS
            .iter()
            .copied()
            .min_by(|left, right| {
                (reciprocal / *left as f64)
                    .ln()
                    .abs()
                    .total_cmp(&(reciprocal / *right as f64).ln().abs())
            })?;
        return Some(format!("1/{denominator}"));
    }

    if (seconds - seconds.round()).abs() < 0.05 {
        Some(format!("{:.0}s", seconds))
    } else if seconds < 10.0 {
        Some(format!("{seconds:.1}s"))
    } else {
        Some(format!("{seconds:.0}s"))
    }
}

fn format_exif_timestamp(datetime: Option<&str>, offset: Option<&str>) -> Option<String> {
    let datetime = datetime?;
    let mut parts = datetime.split_whitespace();
    let date = parts.next()?;
    let time = parts.next()?;
    let mut date_parts = date.split(':');
    let year = date_parts.next()?;
    let month = date_parts.next()?.parse::<usize>().ok()?;
    let day = date_parts.next()?;
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month.checked_sub(1)?)?;
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?;
    let minute = time_parts.next()?;
    let mut result = format!("{day}-{month_name}-{year} {hour}:{minute}");
    let offset = offset
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("+08:00");
    result.push_str(" UTC");
    result.push_str(offset);
    Some(result)
}

fn load_frame_font() -> anyhow::Result<FontArc> {
    FontArc::try_from_slice(FRAME_TEXT_FONT).context("无法加载外框水印参数字体")
}

fn draw_centered_segments(
    target: &mut RgbaImage,
    font: &FontArc,
    size: f32,
    segments: &[String; 3],
    gap: u32,
    y: u32,
    color: [u8; 3],
) -> anyhow::Result<()> {
    let images: Vec<RgbaImage> = segments
        .iter()
        .map(|text| rasterize_text(font, size, text, color))
        .collect::<anyhow::Result<_>>()?;
    let width = images.iter().fold(gap.saturating_mul(2), |sum, image| {
        sum.saturating_add(image.width())
    });
    let height = images.iter().map(RgbaImage::height).max().unwrap_or(1);
    let mut line = RgbaImage::new(width.max(1), height.max(1));
    let mut x: u32 = 0;
    for image in images {
        imageops::overlay(&mut line, &image, x.into(), 0);
        x = x.saturating_add(image.width()).saturating_add(gap);
    }
    let x = (target.width().saturating_sub(line.width())) / 2;
    imageops::overlay(target, &line, x.into(), y.into());
    Ok(())
}

fn draw_centered_text(
    target: &mut RgbaImage,
    font: &FontArc,
    size: f32,
    text: &str,
    y: u32,
    color: [u8; 3],
) -> anyhow::Result<()> {
    let image = rasterize_text(font, size, text, color)?;
    let x = (target.width().saturating_sub(image.width())) / 2;
    imageops::overlay(target, &image, x.into(), y.into());
    Ok(())
}

fn rasterize_text(
    font: &FontArc,
    size: f32,
    text: &str,
    color: [u8; 3],
) -> anyhow::Result<RgbaImage> {
    let scale = PxScale::from(size.max(1.0));
    let scaled = font.as_scaled(scale);
    let mut caret = 0.0;
    let mut previous = None;
    let mut glyphs = Vec::<Glyph>::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        let id = scaled.glyph_id(character);
        if let Some(previous) = previous {
            caret += scaled.kern(previous, id);
        }
        glyphs.push(id.with_scale_and_position(scale, point(caret, 0.0)));
        caret += scaled.h_advance(id);
        if characters.peek().is_some() {
            caret += size * FRAME_TEXT_TRACKING_RATIO;
        }
        previous = Some(id);
    }
    let outlined: Vec<_> = glyphs
        .into_iter()
        .filter_map(|glyph| font.outline_glyph(glyph))
        .collect();
    if outlined.is_empty() {
        bail!("外框水印文字为空");
    }
    let min_x = outlined
        .iter()
        .map(|glyph| glyph.px_bounds().min.x.floor() as i32)
        .min()
        .unwrap_or(0);
    let min_y = outlined
        .iter()
        .map(|glyph| glyph.px_bounds().min.y.floor() as i32)
        .min()
        .unwrap_or(0);
    let max_x = outlined
        .iter()
        .map(|glyph| glyph.px_bounds().max.x.ceil() as i32)
        .max()
        .unwrap_or(1);
    let max_y = outlined
        .iter()
        .map(|glyph| glyph.px_bounds().max.y.ceil() as i32)
        .max()
        .unwrap_or(1);
    let mut image = RgbaImage::new((max_x - min_x).max(1) as u32, (max_y - min_y).max(1) as u32);
    for glyph in outlined {
        let bounds = glyph.px_bounds();
        let offset_x = bounds.min.x.floor() as i32 - min_x;
        let offset_y = bounds.min.y.floor() as i32 - min_y;
        glyph.draw(|x, y, coverage| {
            let x = offset_x + x as i32;
            let y = offset_y + y as i32;
            if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 {
                return;
            }
            let alpha = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
            image.put_pixel(
                x as u32,
                y as u32,
                Rgba([color[0], color[1], color[2], alpha]),
            );
        });
    }
    Ok(embolden_text(image, color))
}

fn embolden_text(mut image: RgbaImage, color: [u8; 3]) -> RgbaImage {
    let source = image.clone();
    const NEIGHBORS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for (x, y, pixel) in source.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        let alpha = ((u16::from(pixel[3]) * 112) / 255) as u8;
        for (offset_x, offset_y) in NEIGHBORS {
            let target_x = x as i32 + offset_x;
            let target_y = y as i32 + offset_y;
            if target_x < 0
                || target_y < 0
                || target_x >= image.width() as i32
                || target_y >= image.height() as i32
            {
                continue;
            }
            let target = image.get_pixel_mut(target_x as u32, target_y as u32);
            if alpha > target[3] {
                *target = Rgba([color[0], color[1], color[2], alpha]);
            }
        }
    }
    image
}

fn extract_video_preview_frame(input: &Path) -> anyhow::Result<RgbaImage> {
    let output = Command::new(ffmpeg_binary())
        .args(["-hide_banner", "-loglevel", "error", "-ss", "0.2", "-i"])
        .arg(input)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=1600:1200:force_original_aspect_ratio=decrease:force_divisible_by=2",
            "-an",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "pipe:1",
        ])
        .output()
        .context("无法启动视频预览组件")?;
    if !output.status.success() || output.stdout.is_empty() {
        bail!(
            "无法提取视频预览帧：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(image::load_from_memory(&output.stdout)
        .context("无法解码视频预览帧")?
        .to_rgba8())
}

fn save_image(image: DynamicImage, output: &Path) -> anyhow::Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let format = match media_extension(output).as_str() {
        "png" => ImageFormat::Png,
        "webp" => ImageFormat::WebP,
        _ => ImageFormat::Jpeg,
    };
    image
        .save_with_format(output, format)
        .with_context(|| format!("无法写入导出文件：{}", output.display()))
}

fn style_for(id: &str) -> anyhow::Result<&'static WatermarkStyle> {
    WATERMARK_STYLES
        .iter()
        .find(|style| style.id == id)
        .ok_or_else(|| anyhow!("未知水印样式：{id}"))
}

fn load_style_image(style: &WatermarkStyle, kind: MediaKind) -> anyhow::Result<Vec<u8>> {
    let file = match kind {
        MediaKind::Image => style.image_file,
        MediaKind::Video => style.video_file,
    }
    .ok_or_else(|| anyhow!("此水印没有对应的官方资源"))?;
    load_runtime_asset(file)
}

fn load_runtime_asset(file: &str) -> anyhow::Result<Vec<u8>> {
    for path in runtime_asset_candidates(file) {
        if path.exists() {
            return std::fs::read(&path)
                .with_context(|| format!("无法读取水印资源：{}", path.display()));
        }
    }
    bail!("缺少官方水印资源：{file}")
}

fn runtime_asset_candidates(file: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("assets").join("apk_watermark").join(file));
        }
    }
    candidates.push(PathBuf::from("assets").join("apk_watermark").join(file));
    candidates
}

fn app_placement(
    style: &WatermarkStyle,
    width: u32,
    height: u32,
    kind: MediaKind,
    position: &str,
) -> Option<AppPlacement> {
    let ratio = ratio_label(width, height);
    let app_position = app_position(position);
    let table = match kind {
        MediaKind::Image => IMAGE_WATERMARK_CONFIG,
        MediaKind::Video => WATERMARK_CONFIG,
    };
    find_placement(table, style.model, style.profile, ratio, app_position)
}

fn frame_placement(style: &WatermarkStyle, width: u32, height: u32) -> Option<FramePlacement> {
    let key = format!(
        "{}##{}##{}",
        style.model,
        style.profile,
        ratio_label(width, height)
    );
    for line in FRAME_WATERMARK_CONFIG.lines() {
        let line = line.trim();
        if !line.starts_with(&format!("\"{key}\"")) {
            continue;
        }
        let value = &line[line.rfind(':')? + 1..];
        let values: Vec<f32> = value
            .trim()
            .trim_matches(',')
            .trim_matches('"')
            .split("##")
            .filter_map(|part| part.parse::<f32>().ok())
            .collect();
        return Some(FramePlacement {
            width_scale: *values.first()?,
            height_scale: *values.get(1)?,
            photo_bottom_ratio: *values.get(2)?,
            watermark_width_ratio: *values.get(3)?,
            capture_font_ratio: *values.get(4)?,
            capture_spacing_ratio: *values.get(5)?,
            timestamp_font_ratio: *values.get(6)?,
            line_spacing_ratio: *values.get(7)?,
            moment_width_ratio: *values.get(10)?,
        });
    }
    None
}

fn app_position(position: &str) -> Option<&'static str> {
    match position {
        "top-left" => Some("TopLeft"),
        "top-right" => Some("TopRight"),
        "bottom-left" => Some("BottomLeft"),
        "bottom-center" => Some("BottomCenter"),
        "bottom-right" => Some("BottomRight"),
        _ => None,
    }
}

fn find_placement(
    table: &str,
    model: &str,
    profile: &str,
    ratio: &str,
    position: Option<&str>,
) -> Option<AppPlacement> {
    let position = position?;
    let key = format!("\"{model}##{profile}##{ratio}##{position}\"");
    for line in table.lines() {
        let line = line.trim();
        if !line.starts_with(&key) {
            continue;
        }
        let value = &line[line.rfind(':')? + 1..];
        let mut values = value
            .trim()
            .trim_matches(',')
            .trim_matches('"')
            .split("##")
            .filter_map(|part| part.parse::<f32>().ok());
        return Some(AppPlacement {
            width_ratio: values.next()?,
            x_ratio: values.next()?,
            bottom_ratio: values.next()?,
        });
    }
    None
}

fn app_coordinates(
    base_width: u32,
    base_height: u32,
    watermark_width: u32,
    watermark_height: u32,
    placement: AppPlacement,
) -> (u32, u32) {
    let x = (base_width as f32 * placement.x_ratio).round().max(0.0) as u32;
    let y = (base_height as f32 * (1.0 - placement.bottom_ratio) - watermark_height as f32)
        .round()
        .max(0.0) as u32;
    (
        x.min(base_width.saturating_sub(watermark_width)),
        y.min(base_height.saturating_sub(watermark_height)),
    )
}

fn scaled_height(watermark: &RgbaImage, target_width: u32) -> u32 {
    (watermark.height() as f32 * target_width as f32 / watermark.width().max(1) as f32)
        .round()
        .max(1.0) as u32
}

fn media_extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn ratio_label(width: u32, height: u32) -> &'static str {
    let ratio = width as f32 / height.max(1) as f32;
    let candidates = [
        ("2:1", 2.0),
        ("16:9", 16.0 / 9.0),
        ("9:16", 9.0 / 16.0),
        ("4:3", 4.0 / 3.0),
        ("3:4", 3.0 / 4.0),
        ("1:1", 1.0),
        ("3:2", 3.0 / 2.0),
        ("2:3", 2.0 / 3.0),
        ("235:100", 2.35),
        ("100:235", 1.0 / 2.35),
        ("3:1", 3.0),
        ("27:10", 2.7),
        ("10:27", 1.0 / 2.7),
        ("47:20", 2.35),
        ("20:47", 20.0 / 47.0),
    ];
    candidates
        .iter()
        .min_by(|(_, left), (_, right)| {
            (ratio - *left)
                .abs()
                .partial_cmp(&(ratio - *right).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(name, _)| *name)
        .unwrap_or("16:9")
}

fn ffmpeg_binary() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let binary_name = if cfg!(target_os = "windows") {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            };
            candidates.push(parent.join("assets").join("ffmpeg").join(binary_name));
            if cfg!(target_os = "macos") {
                if let Some(contents_dir) = parent.parent() {
                    candidates.push(
                        contents_dir
                            .join("Resources")
                            .join("ffmpeg")
                            .join(binary_name),
                    );
                }
            }
        }
    }
    candidates.push(
        PathBuf::from("assets")
            .join("ffmpeg")
            .join(if cfg!(target_os = "windows") {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            }),
    );
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

fn video_dimensions(input: &Path) -> anyhow::Result<(u32, u32)> {
    let output = Command::new(ffmpeg_binary())
        .args(["-hide_banner", "-i"])
        .arg(input)
        .output()
        .context("无法读取视频信息")?;
    let description = String::from_utf8_lossy(&output.stderr);
    let dimensions = description
        .lines()
        .find_map(|line| parse_video_dimensions(line))
        .ok_or_else(|| anyhow!("无法从视频中识别分辨率"))?;
    Ok(dimensions)
}

fn parse_video_dimensions(line: &str) -> Option<(u32, u32)> {
    let bytes = line.as_bytes();
    for index in 0..bytes.len() {
        if !bytes[index].is_ascii_digit() {
            continue;
        }
        let end_width = bytes[index..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map(|offset| index + offset)?;
        if bytes.get(end_width) != Some(&b'x') {
            continue;
        }
        let height_start = end_width + 1;
        let height_end = bytes[height_start..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map(|offset| height_start + offset)
            .unwrap_or(bytes.len());
        let width = line[index..end_width].parse().ok()?;
        let height = line[height_start..height_end].parse().ok()?;
        if width >= 64 && height >= 64 {
            return Some((width, height));
        }
    }
    None
}

fn temporary_watermark_path(options: &WatermarkOptions, style: &WatermarkStyle) -> PathBuf {
    let parent = options.output.parent().unwrap_or_else(|| Path::new("."));
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!(".luna-watermark-{}-{timestamp}.png", style.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_only_luna_app_styles() {
        let ids: Vec<_> = styles().iter().map(|style| style.id).collect();
        assert_eq!(
            ids,
            vec![
                "luna-ultra-cn",
                "luna-ultra",
                "luna-ultra-zstyle-cn",
                "luna-ultra-zstyle"
            ]
        );
    }

    #[test]
    fn frame_background_catalog_matches_the_app_color_modes() {
        let ids: Vec<_> = frame_backgrounds()
            .iter()
            .map(|background| background.id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "black",
                "white",
                "photo-dark",
                "photo-light",
                "photo-gradient"
            ]
        );
    }

    #[test]
    fn image_and_video_use_their_respective_apk_assets() {
        let style = style_for("luna-ultra-cn").unwrap();
        assert_eq!(
            style.image_file,
            Some("ic_watermark_luna_ultra_image_cn.png")
        );
        assert_eq!(style.video_file, Some("ic_watermark_luna_ultra_cn.png"));
    }

    #[test]
    fn luna_image_uses_the_apk_bottom_center_default() {
        let style = style_for("luna-ultra-cn").unwrap();
        let placement =
            app_placement(style, 3840, 2160, MediaKind::Image, "bottom-center").unwrap();
        assert_eq!(placement.width_ratio, 0.279);
        assert_eq!(placement.x_ratio, 0.361);
        assert_eq!(placement.bottom_ratio, 0.044);
        assert!(app_placement(style, 3840, 2160, MediaKind::Image, "top-left").is_none());
    }

    #[test]
    fn app_coordinates_treat_vertical_table_value_as_bottom_margin() {
        let (x, y) = app_coordinates(
            3840,
            2160,
            734,
            200,
            AppPlacement {
                width_ratio: 0.191,
                x_ratio: 0.776,
                bottom_ratio: 0.059,
            },
        );
        assert_eq!(x, 2980);
        assert_eq!(y, 1833);
    }

    #[test]
    fn luna_zstyle_frame_uses_the_apk_frame_table() {
        let style = style_for("luna-ultra-zstyle-cn").unwrap();
        let frame = frame_placement(style, 3840, 2160).unwrap();
        assert_eq!(frame.width_scale, 1.0781);
        assert_eq!(frame.height_scale, 1.8333);
        assert_eq!(frame.photo_bottom_ratio, 0.2525);
        assert_eq!(frame.watermark_width_ratio, 0.3386);
        assert_eq!(frame.capture_font_ratio, 0.0145);
        assert_eq!(frame.capture_spacing_ratio, 0.0174);
        assert_eq!(frame.timestamp_font_ratio, 0.0145);
        assert_eq!(frame.line_spacing_ratio, 0.0162);
        assert_eq!(frame.moment_width_ratio, 0.2087);
    }

    #[test]
    fn luna_direct_output_geometry_matches_the_reference_image() {
        let style = style_for("luna-ultra-zstyle-cn").unwrap();
        let frame = frame_placement(style, 3520, 2644).unwrap();
        let canvas_width = (3520.0 * frame.width_scale).ceil() as u32;
        let canvas_height = (2644.0 * frame.height_scale).ceil() as u32;
        let photo_bottom = (canvas_height as f32 * frame.photo_bottom_ratio).ceil() as u32;
        let photo_x = (canvas_width - 3520) / 2;
        let photo_y = canvas_height - 2644 - photo_bottom;
        let logo_width = (canvas_width as f32 * frame.watermark_width_ratio).round() as u32;

        assert_eq!((canvas_width, canvas_height), (3781, 4209));
        assert_eq!((photo_x, photo_y), (130, 695));
        assert_eq!(logo_width, 1217);
    }

    #[test]
    fn parses_video_stream_dimensions() {
        assert_eq!(
            parse_video_dimensions("Stream #0:0: Video: h264, yuv420p, 3840x2160 [SAR 1:1]"),
            Some((3840, 2160))
        );
    }

    #[test]
    fn renders_luna_image_mark_with_the_apk_canvas_size() {
        let (input, output) = test_image_paths("mark");
        RgbaImage::from_pixel(1600, 900, Rgba([32, 64, 96, 255]))
            .save(&input)
            .unwrap();
        apply(&WatermarkOptions {
            input: input.clone(),
            output: output.clone(),
            position: "bottom-center".to_string(),
            style: "luna-ultra-cn".to_string(),
            frame_background: "black".to_string(),
            moment_preset: "official".to_string(),
            moment_image: None,
        })
        .unwrap();
        let rendered = image::open(&output).unwrap();
        assert_eq!((rendered.width(), rendered.height()), (1600, 900));
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn renders_luna_frame_with_the_apk_canvas_and_photo_position() {
        let (input, output) = test_image_paths("frame");
        let source_color = Rgba([32, 64, 96, 255]);
        RgbaImage::from_pixel(1600, 900, source_color)
            .save(&input)
            .unwrap();
        apply(&WatermarkOptions {
            input: input.clone(),
            output: output.clone(),
            position: "bottom-center".to_string(),
            style: "luna-ultra-zstyle-cn".to_string(),
            frame_background: "black".to_string(),
            moment_preset: "official".to_string(),
            moment_image: None,
        })
        .unwrap();
        let rendered = image::open(&output).unwrap().to_rgba8();
        assert_eq!((rendered.width(), rendered.height()), (1725, 1650));
        assert_eq!(*rendered.get_pixel(63, 524), source_color);
        assert_eq!(*rendered.get_pixel(0, 0), Rgba([0, 0, 0, 255]));
        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn preview_returns_a_real_composited_jpeg_with_bounded_dimensions() {
        let (input, _) = test_image_paths("preview");
        RgbaImage::from_pixel(1600, 900, Rgba([32, 64, 96, 255]))
            .save(&input)
            .unwrap();
        let encoded = preview(
            &input,
            "luna-ultra-zstyle-cn",
            "bottom-center",
            "black",
            "official",
            None,
            900,
            650,
        )
        .unwrap();
        let rendered = image::load_from_memory(&encoded).unwrap();
        assert!(rendered.width() <= 900);
        assert!(rendered.height() <= 650);
        assert_eq!(rendered.color(), image::ColorType::Rgb8);
        let _ = std::fs::remove_file(input);
    }

    #[test]
    fn formats_luna_direct_output_metadata() {
        assert_eq!(
            format_aperture(exif::Rational { num: 20, denom: 10 }),
            "F/2.0"
        );
        let timestamp = format_exif_timestamp(Some("2026:06:18 20:08:37"), Some("+08:00"));
        assert_eq!(timestamp.as_deref(), Some("18-Jun-2026 20:08 UTC+08:00"));
        let timestamp_without_offset = format_exif_timestamp(Some("2026:06:26 16:45:57"), None);
        assert_eq!(
            timestamp_without_offset.as_deref(),
            Some("26-Jun-2026 16:45 UTC+08:00")
        );
        assert_eq!(
            format_exposure(exif::Rational {
                num: 10,
                denom: 1000
            }),
            "1/100"
        );
        assert_eq!(
            format_exposure(exif::Rational {
                num: 1211,
                denom: 1_000_000
            }),
            "1/800"
        );
        assert_eq!(
            format_exposure(exif::Rational { num: 1, denom: 16 }),
            "1/16"
        );
        assert_eq!(
            format_shutter_apex(exif::SRational {
                num: 9689,
                denom: 1000
            })
            .as_deref(),
            Some("1/800")
        );
    }

    #[test]
    #[ignore = "requires a user-provided Luna Ultra original"]
    fn renders_external_luna_original_for_qa() {
        let input = PathBuf::from(
            std::env::var_os("LUNA_WATERMARK_QA_INPUT")
                .expect("LUNA_WATERMARK_QA_INPUT is required"),
        );
        let output = std::env::var_os("LUNA_WATERMARK_QA_OUTPUT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/step179-original-frame.png"));
        let moment_preset = std::env::var("LUNA_WATERMARK_QA_MOMENT_PRESET")
            .unwrap_or_else(|_| "official".to_string());
        let moment_image = std::env::var_os("LUNA_WATERMARK_QA_MOMENT_IMAGE").map(PathBuf::from);
        let metadata = read_frame_metadata(&input);
        assert_eq!(metadata.aperture.as_deref(), Some("F/2.0"));
        assert_eq!(metadata.exposure.as_deref(), Some("1/800"));
        assert_eq!(
            metadata.timestamp.as_deref(),
            Some("26-Jun-2026 16:45 UTC+08:00")
        );
        apply(&WatermarkOptions {
            input,
            output,
            position: "bottom-center".to_string(),
            style: "luna-ultra-zstyle-cn".to_string(),
            frame_background: "black".to_string(),
            moment_preset,
            moment_image,
        })
        .unwrap();
    }

    #[test]
    fn renders_luna_footer_moment_and_metadata() {
        let style = style_for("luna-ultra-zstyle-cn").unwrap();
        let source = RgbaImage::from_pixel(1600, 1200, Rgba([32, 64, 96, 255]));
        let metadata = FrameMetadata {
            aperture: Some("F/2.0".to_string()),
            exposure: Some("1/100".to_string()),
            iso: Some("ISO416".to_string()),
            timestamp: Some("18-Jun-2026 20:08 UTC+08:00".to_string()),
        };
        let rendered =
            render_zstyle_frame(source.clone(), style, &metadata, "black", "official", None)
                .unwrap();
        assert_eq!((rendered.width(), rendered.height()), (1719, 1911));
        if std::env::var_os("LUNA_WATERMARK_QA").is_some() {
            rendered.save("target/step178-frame-black.png").unwrap();
            render_zstyle_frame(source.clone(), style, &metadata, "white", "official", None)
                .unwrap()
                .save("target/step178-frame-white.png")
                .unwrap();
            render_zstyle_frame(source, style, &metadata, "photo-gradient", "official", None)
                .unwrap()
                .save("target/step178-frame-gradient.png")
                .unwrap();
        }
        let footer_has_content = rendered
            .enumerate_pixels()
            .filter(|(_, y, _)| *y > 1600)
            .any(|(_, _, pixel)| pixel[0] > 100);
        assert!(footer_has_content);
    }

    #[test]
    fn white_frame_uses_a_light_background_and_dark_foreground() {
        let style = style_for("luna-ultra-zstyle-cn").unwrap();
        let source = RgbaImage::from_pixel(1600, 900, Rgba([32, 64, 96, 255]));
        let rendered = render_zstyle_frame(
            source,
            style,
            &FrameMetadata::default(),
            "white",
            "official",
            None,
        )
        .unwrap();
        assert_eq!(*rendered.get_pixel(0, 0), Rgba([244, 244, 242, 255]));
        assert_eq!(
            resolve_frame_background(
                &RgbaImage::from_pixel(8, 8, Rgba([32, 64, 96, 255])),
                "white"
            )
            .foreground,
            [24, 24, 24]
        );
    }

    #[test]
    fn photo_backgrounds_are_derived_from_the_source() {
        let source = RgbaImage::from_fn(16, 16, |_, y| {
            if y < 8 {
                Rgba([220, 30, 40, 255])
            } else {
                Rgba([20, 80, 210, 255])
            }
        });
        let first = resolve_frame_background(&source, "photo-gradient");
        let second = resolve_frame_background(&source, "photo-gradient");
        assert_eq!(first.start, second.start);
        assert_eq!(first.end, second.end);
        assert_ne!(first.start, first.end);
    }

    #[test]
    fn loads_builtin_and_custom_moment_images() {
        let (preset, fit_both_dimensions) =
            load_moment_image("shenshen-concert", None, [255, 255, 255]).unwrap();
        assert!(fit_both_dimensions);
        assert!(preset.pixels().any(|pixel| {
            pixel[2] > 150 && pixel[2] > pixel[0].saturating_add(40) && pixel[3] == 255
        }));

        let custom_path = PathBuf::from("target/custom-moment-test.png");
        std::fs::create_dir_all("target").unwrap();
        let mut custom = RgbaImage::from_pixel(40, 20, Rgba([0, 0, 0, 0]));
        custom.put_pixel(20, 10, Rgba([220, 40, 80, 180]));
        custom.save(&custom_path).unwrap();
        let (loaded, fit_both_dimensions) =
            load_moment_image("custom", Some(&custom_path), [255, 255, 255]).unwrap();
        assert!(fit_both_dimensions);
        assert_eq!(*loaded.get_pixel(20, 10), Rgba([220, 40, 80, 180]));
        let _ = std::fs::remove_file(custom_path);
    }

    #[test]
    fn custom_moment_images_match_the_official_height() {
        let official =
            image::load_from_memory(&load_runtime_asset("choose_logo_photo_moment.png").unwrap())
                .unwrap()
                .to_rgba8();
        assert_eq!((official.width(), official.height()), (749, 259));
        let official_height = scaled_height(&official, 359);
        assert_eq!(official_height, 124);
        assert_eq!(
            scale_to_fixed_height(1080, 478, official_height, 1719),
            (280, 124)
        );
        assert_eq!(
            scale_to_fixed_height(100, 100, official_height, 1719),
            (124, 124)
        );
        assert_eq!(
            scale_to_fixed_height(4000, 100, official_height, 1000),
            (1000, 25)
        );
    }

    fn test_image_paths(label: &str) -> (PathBuf, PathBuf) {
        let target = PathBuf::from("target");
        std::fs::create_dir_all(&target).unwrap();
        let suffix = std::process::id();
        (
            target.join(format!("watermark-{label}-{suffix}-input.png")),
            target.join(format!("watermark-{label}-{suffix}-output.png")),
        )
    }
}
