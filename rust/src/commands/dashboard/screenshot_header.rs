//! Dashboard screenshot header and capture metadata helpers.
#![cfg_attr(not(feature = "browser"), allow(dead_code))]

use chrono::Local;
#[cfg(feature = "browser")]
use font8x8::UnicodeFonts;
use reqwest::Url;
use serde_json::{Map, Value};

use crate::common::{object_field, string_field, value_as_object, Result};

#[cfg(feature = "browser")]
use crate::common::message;

use super::super::{build_http_client, fetch_dashboard, ScreenshotArgs};
use super::parse_dashboard_url_state;

#[derive(Debug, Clone, Default)]
pub(crate) struct DashboardCaptureMetadata {
    pub(crate) dashboard_uid: Option<String>,
    pub(crate) dashboard_title: Option<String>,
    pub(crate) panel_title: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct HeaderSpec {
    lines: Vec<HeaderLine>,
}

#[derive(Debug, Clone)]
pub(crate) struct HeaderLine {
    text: String,
    scale: u32,
    color: [u8; 4],
}

pub(crate) fn resolve_dashboard_metadata(
    args: &mut ScreenshotArgs,
) -> Result<DashboardCaptureMetadata> {
    let dashboard_uid = resolve_dashboard_uid(args);
    let Some(dashboard_uid) = dashboard_uid.as_deref() else {
        return Ok(DashboardCaptureMetadata::default());
    };
    let client = build_http_client(&args.common)?;
    let payload = fetch_dashboard(&client, dashboard_uid)?;
    let object = value_as_object(&payload, "Unexpected dashboard payload from Grafana.")?;
    let meta = match object_field(object, "meta") {
        Some(value) => value,
        None => return Ok(DashboardCaptureMetadata::default()),
    };
    let slug = string_field(meta, "slug", "");
    if !slug.trim().is_empty()
        && args
            .slug
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        && args
            .dashboard_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        args.slug = Some(slug);
    }
    let dashboard = match object_field(object, "dashboard") {
        Some(value) => value,
        None => return Ok(DashboardCaptureMetadata::default()),
    };
    let panel_title = args
        .panel_id
        .and_then(|panel_id| find_panel_title(dashboard, panel_id));
    Ok(DashboardCaptureMetadata {
        dashboard_uid: Some(dashboard_uid.to_string()),
        dashboard_title: Some(string_field(dashboard, "title", "")),
        panel_title,
    })
}

pub(crate) fn build_header_spec(
    args: &ScreenshotArgs,
    resolved_url: &str,
    metadata: &DashboardCaptureMetadata,
) -> Option<HeaderSpec> {
    let mut lines = Vec::new();
    if let Some(text) = resolve_header_title(args, metadata) {
        lines.push(HeaderLine {
            text,
            scale: 2,
            color: [240, 244, 252, 255],
        });
    }
    if let Some(text) = resolve_optional_header_field(args.header_url.as_deref(), resolved_url) {
        lines.push(HeaderLine {
            text,
            scale: 1,
            color: [154, 169, 191, 255],
        });
    }
    if args.header_captured_at {
        lines.push(HeaderLine {
            text: format!("Captured at {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
            scale: 1,
            color: [154, 169, 191, 255],
        });
    }
    if let Some(text) = args
        .header_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(HeaderLine {
            text: text.to_string(),
            scale: 1,
            color: [210, 218, 230, 255],
        });
    }
    if lines.is_empty() {
        None
    } else {
        Some(HeaderSpec { lines })
    }
}

pub(crate) fn resolve_header_title(
    args: &ScreenshotArgs,
    metadata: &DashboardCaptureMetadata,
) -> Option<String> {
    match args.header_title.as_deref() {
        Some("__auto__") => resolve_auto_title(metadata, args),
        Some(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        _ => None,
    }
}

pub(crate) fn resolve_manifest_title(
    dashboard_uid: Option<&str>,
    dashboard_title: Option<&str>,
    panel_title: Option<&str>,
    args: &ScreenshotArgs,
) -> Option<String> {
    let metadata = DashboardCaptureMetadata {
        dashboard_uid: dashboard_uid.map(str::to_string),
        dashboard_title: dashboard_title.map(str::to_string),
        panel_title: panel_title.map(str::to_string),
    };
    resolve_auto_title(&metadata, args)
}

fn resolve_dashboard_uid(args: &ScreenshotArgs) -> Option<String> {
    args.dashboard_uid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            args.dashboard_url
                .as_deref()
                .and_then(|value| Url::parse(value).ok())
                .map(|url| parse_dashboard_url_state(&url))
                .and_then(|state| state.dashboard_uid)
        })
}

fn find_panel_title(dashboard: &Map<String, Value>, panel_id: i64) -> Option<String> {
    fn visit_panels(items: &[Value], panel_id: i64) -> Option<String> {
        for item in items {
            let object = item.as_object()?;
            if object.get("id").and_then(Value::as_i64) == Some(panel_id) {
                let title = string_field(object, "title", "");
                if !title.trim().is_empty() {
                    return Some(title);
                }
            }
            if let Some(nested) = object.get("panels").and_then(Value::as_array) {
                if let Some(title) = visit_panels(nested, panel_id) {
                    return Some(title);
                }
            }
        }
        None
    }

    dashboard
        .get("panels")
        .and_then(Value::as_array)
        .and_then(|items| visit_panels(items, panel_id))
}

fn resolve_auto_title(
    metadata: &DashboardCaptureMetadata,
    args: &ScreenshotArgs,
) -> Option<String> {
    metadata
        .panel_title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            metadata
                .dashboard_title
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
        })
        .or_else(|| metadata.dashboard_uid.clone())
        .or_else(|| {
            args.output
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
}

fn resolve_optional_header_field(raw: Option<&str>, auto_value: &str) -> Option<String> {
    match raw {
        Some("__auto__") => Some(auto_value.to_string()),
        Some(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        _ => None,
    }
}

#[cfg(feature = "browser")]
use image::{DynamicImage, GenericImage, ImageFormat, Rgba, RgbaImage};

#[cfg(feature = "browser")]
pub(crate) fn apply_header_if_requested(
    bytes: Vec<u8>,
    args: &ScreenshotArgs,
    header_spec: &Option<HeaderSpec>,
    format: ImageFormat,
) -> Result<Vec<u8>> {
    if matches!(format, ImageFormat::Png | ImageFormat::Jpeg) && header_spec.is_some() {
        compose_header_image(bytes, args.width, header_spec.as_ref().unwrap(), format)
    } else {
        Ok(bytes)
    }
}

#[cfg(feature = "browser")]
fn compose_header_image(
    bytes: Vec<u8>,
    target_width: u32,
    header_spec: &HeaderSpec,
    format: ImageFormat,
) -> Result<Vec<u8>> {
    let screenshot = image::load_from_memory(&bytes).map_err(|error| {
        message(format!(
            "Failed to decode screenshot for header composition: {error}"
        ))
    })?;
    let screenshot_rgba = screenshot.to_rgba8();
    let width = target_width.max(screenshot_rgba.width());
    let header_height = measure_header_height(header_spec);
    let mut output = RgbaImage::from_pixel(
        width,
        header_height + screenshot_rgba.height(),
        Rgba([12, 16, 24, 255]),
    );
    paint_header_background(&mut output, width, header_height);
    draw_header_lines(&mut output, header_spec);
    output
        .copy_from(&screenshot_rgba, 0, header_height)
        .map_err(|error| message(format!("Failed to append screenshot under header: {error}")))?;
    let mut encoded = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(output)
        .write_to(&mut encoded, format)
        .map_err(|error| {
            message(format!(
                "Failed to encode screenshot header composition: {error}"
            ))
        })?;
    Ok(encoded.into_inner())
}

#[cfg(feature = "browser")]
fn measure_header_height(spec: &HeaderSpec) -> u32 {
    const TOP_PADDING: u32 = 20;
    const BOTTOM_PADDING: u32 = 18;
    const LINE_SPACING: u32 = 10;
    let content_height = spec
        .lines
        .iter()
        .map(|line| (8 * line.scale) + LINE_SPACING)
        .sum::<u32>();
    TOP_PADDING + BOTTOM_PADDING + content_height.saturating_sub(LINE_SPACING)
}

#[cfg(feature = "browser")]
fn paint_header_background(image: &mut RgbaImage, width: u32, header_height: u32) {
    for x in 0..width {
        image.put_pixel(x, 0, Rgba([59, 130, 246, 255]));
        if header_height > 1 {
            image.put_pixel(x, 1, Rgba([30, 64, 175, 255]));
        }
    }
    for y in 0..header_height {
        let blend = if header_height <= 2 {
            0
        } else {
            ((y as f32 / header_height as f32) * 24.0) as u8
        };
        for x in 0..width {
            let pixel = image.get_pixel_mut(x, y);
            *pixel = Rgba([
                pixel[0].saturating_add(blend / 6),
                pixel[1].saturating_add(blend / 8),
                pixel[2].saturating_add(blend / 4),
                255,
            ]);
        }
    }
}

#[cfg(feature = "browser")]
fn draw_header_lines(image: &mut RgbaImage, spec: &HeaderSpec) {
    let mut y = 20_u32;
    for line in &spec.lines {
        draw_text_line(image, 24, y, &line.text, line.scale, line.color);
        y += (8 * line.scale) + 10;
    }
}

#[cfg(feature = "browser")]
fn draw_text_line(
    image: &mut RgbaImage,
    start_x: u32,
    start_y: u32,
    text: &str,
    scale: u32,
    color: [u8; 4],
) {
    let mut cursor_x = start_x;
    for character in text.chars() {
        if character == '\n' {
            break;
        }
        draw_glyph(image, cursor_x, start_y, character, scale, color);
        cursor_x += (8 * scale) + scale;
    }
}

#[cfg(feature = "browser")]
fn draw_glyph(
    image: &mut RgbaImage,
    start_x: u32,
    start_y: u32,
    character: char,
    scale: u32,
    color: [u8; 4],
) {
    let glyph = font8x8::BASIC_FONTS
        .get(character)
        .or_else(|| font8x8::BASIC_FONTS.get('?'))
        .unwrap_or([0; 8]);
    for (row_index, row_bits) in glyph.iter().enumerate() {
        for column in 0..8 {
            if ((row_bits >> column) & 1) == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let x = start_x + (column as u32 * scale) + dx;
                    let y = start_y + (row_index as u32 * scale) + dy;
                    if x < image.width() && y < image.height() {
                        image.put_pixel(x, y, Rgba(color));
                    }
                }
            }
        }
    }
}
