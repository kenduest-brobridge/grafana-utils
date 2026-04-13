//! Capture full-page dashboard screenshots from a browser session.
//! This module computes the viewport clip, stitches page segments, and applies the
//! optional header overlay for screenshot exports. It handles browser-side capture
//! details only, so call sites can stay focused on CLI arguments and output format.

use headless_chrome::protocol::cdp::Page;
use image::{DynamicImage, GenericImage, ImageFormat, RgbaImage};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::common::{message, Result};

use super::screenshot_header::{
    apply_header_if_requested, resolve_header_title, resolve_manifest_title,
    DashboardCaptureMetadata, HeaderSpec,
};
use super::screenshot_runtime::{CaptureOffsets, CapturedSegment, FullPageCapture};
use super::{
    read_numeric_expression, ScreenshotArgs, ScreenshotFullPageOutput, ScreenshotOutputFormat,
};

pub(super) fn build_screenshot_clip(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    args: &ScreenshotArgs,
) -> Result<Option<Page::Viewport>> {
    if !args.full_page {
        return Ok(None);
    }

    let width = read_numeric_expression(
        tab,
        r#"
Math.max(
  document.documentElement.scrollWidth || 0,
  document.body ? document.body.scrollWidth || 0 : 0,
  window.innerWidth || 0
)
        "#,
        args.width as f64,
    )?;
    let height = read_numeric_expression(
        tab,
        r#"
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
        "#,
        args.height as f64,
    )?;

    Ok(Some(Page::Viewport {
        x: 0.0,
        y: 0.0,
        width,
        height,
        scale: args.device_scale_factor,
    }))
}

pub(super) fn warm_full_page_render(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    args: &ScreenshotArgs,
) -> Result<()> {
    if !args.full_page {
        return Ok(());
    }

    let mut previous_height = 0.0;
    let mut stable_reads = 0_u8;

    for _ in 0..8 {
        let height = read_numeric_expression(
            tab,
            r#"
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
            "#,
            args.height as f64,
        )?;

        let scroll_script = format!(
            r#"
(() => {{
  const target = Math.max(0, {} - window.innerHeight);
  window.scrollTo({{ top: target, left: 0, behavior: 'instant' }});
  return window.scrollY;
}})()
            "#,
            height
        );
        tab.evaluate(&scroll_script, false).map_err(|error| {
            message(format!(
                "Failed to scroll dashboard for --full-page: {error}"
            ))
        })?;
        thread::sleep(Duration::from_millis(1800));

        let next_height = read_numeric_expression(
            tab,
            r#"
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
            "#,
            args.height as f64,
        )?;

        if (next_height - previous_height).abs() < 1.0 && (next_height - height).abs() < 1.0 {
            stable_reads += 1;
        } else {
            stable_reads = 0;
        }
        previous_height = next_height;

        if stable_reads >= 2 {
            break;
        }
    }

    tab.evaluate(
        "window.scrollTo({ top: 0, left: 0, behavior: 'instant' })",
        false,
    )
    .map_err(|error| {
        message(format!(
            "Failed to reset dashboard scroll position: {error}"
        ))
    })?;
    thread::sleep(Duration::from_millis(300));
    Ok(())
}

pub(super) fn capture_full_page_segments(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    args: &ScreenshotArgs,
    capture_offsets: &CaptureOffsets,
    format: Page::CaptureScreenshotFormatOption,
    quality: Option<u32>,
) -> Result<FullPageCapture> {
    let total_height = read_numeric_expression(
        tab,
        r#"
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
        "#,
        args.height as f64,
    )?;
    let viewport_height = args.height as f64;
    let viewport_width = args.width;
    let device_scale_factor = args.device_scale_factor.max(0.01);
    let crop_top = (capture_offsets.hidden_top_height.max(0.0) * device_scale_factor).ceil() as u32;
    let crop_left =
        (capture_offsets.hidden_left_width.max(0.0) * device_scale_factor).ceil() as u32;
    let target_width = ((viewport_width as f64 * device_scale_factor).ceil() as u32)
        .saturating_sub(crop_left)
        .max(1);
    let step = (viewport_height - capture_offsets.hidden_top_height.max(0.0)).max(200.0);

    let final_total_height = ((total_height * device_scale_factor).ceil() as u32).max(1);
    let mut current_y = 0.0_f64;
    let mut segments = Vec::new();

    while current_y < total_height - 1.0 {
        let scroll_script = format!(
            "window.scrollTo({{ top: {}, left: 0, behavior: 'instant' }});",
            current_y.floor()
        );
        tab.evaluate(&scroll_script, false)
            .map_err(|error| message(format!("Failed to scroll for stitched capture: {error}")))?;
        thread::sleep(Duration::from_millis(900));

        let bytes = tab
            .capture_screenshot(format.clone(), quality, None, true)
            .map_err(|error| {
                message(format!(
                    "Failed to capture stitched screenshot segment: {error}"
                ))
            })?;
        let segment = image::load_from_memory(&bytes).map_err(|error| {
            message(format!(
                "Failed to decode stitched screenshot segment: {error}"
            ))
        })?;
        let segment_rgba = segment.to_rgba8();
        let segment_height = segment_rgba.height();
        let segment_width = segment_rgba.width();
        let source_left = crop_left.min(segment_width.saturating_sub(1));
        let source_top = if current_y <= 0.0 {
            0
        } else {
            crop_top.min(segment_height)
        };
        let consumed_height = segments
            .iter()
            .map(|segment: &CapturedSegment| segment.image.height())
            .fold(0_u32, |acc: u32, height: u32| acc.saturating_add(height));
        let remaining_height = final_total_height.saturating_sub(consumed_height);
        if remaining_height == 0 {
            break;
        }
        let available_segment_height = segment_height.saturating_sub(source_top);
        let available_segment_width = segment_width.saturating_sub(source_left);
        let copy_height = available_segment_height.min(remaining_height);
        let copy_width = available_segment_width.min(target_width);
        if copy_height == 0 || copy_width == 0 {
            break;
        }
        let cropped = image::imageops::crop_imm(
            &segment_rgba,
            source_left,
            source_top,
            copy_width,
            copy_height,
        )
        .to_image();
        segments.push(CapturedSegment {
            image: cropped,
            index: segments.len(),
            scroll_y: (current_y.floor().max(0.0) * device_scale_factor).ceil() as u32,
            source_top,
        });

        if consumed_height.saturating_add(copy_height) >= final_total_height {
            break;
        }
        current_y += step;
    }

    Ok(FullPageCapture {
        total_height: final_total_height.max(1),
        target_width,
        viewport_width,
        viewport_height: args.height,
        device_scale_factor,
        crop_top,
        crop_left,
        step: step.ceil() as u32,
        segments,
    })
}

fn stitch_full_page_capture(capture: &FullPageCapture) -> Result<RgbaImage> {
    let mut stitched = RgbaImage::new(capture.target_width, capture.total_height);
    let mut destination_y = 0_u32;
    for segment in &capture.segments {
        if destination_y >= capture.total_height {
            break;
        }
        stitched
            .copy_from(&segment.image, 0, destination_y)
            .map_err(|error| message(format!("Failed to stitch screenshot segment: {error}")))?;
        destination_y = destination_y.saturating_add(segment.image.height());
    }
    let final_height = destination_y.max(1);
    Ok(image::imageops::crop_imm(&stitched, 0, 0, capture.target_width, final_height).to_image())
}

fn encode_rgba_image(image: &RgbaImage, format: ImageFormat) -> Result<Vec<u8>> {
    let final_image = DynamicImage::ImageRgba8(image.clone());
    let mut encoded = std::io::Cursor::new(Vec::new());
    match format {
        ImageFormat::Png => final_image
            .write_to(&mut encoded, ImageFormat::Png)
            .map_err(|error| {
                message(format!("Failed to encode stitched PNG screenshot: {error}"))
            })?,
        ImageFormat::Jpeg => final_image
            .write_to(&mut encoded, ImageFormat::Jpeg)
            .map_err(|error| {
                message(format!(
                    "Failed to encode stitched JPEG screenshot: {error}"
                ))
            })?,
        ImageFormat::WebP => {
            return Err(message(
                "WEBP stitched screenshot encoding is not supported by this command.",
            ))
        }
        _ => {
            return Err(message(
                "Only PNG and JPEG full-page screenshot encoding is supported by this command.",
            ))
        }
    }
    Ok(encoded.into_inner())
}

pub(super) fn write_full_page_output(
    args: &ScreenshotArgs,
    header_spec: &Option<HeaderSpec>,
    metadata: &DashboardCaptureMetadata,
    output_format: ScreenshotOutputFormat,
    capture: FullPageCapture,
    image_format: ImageFormat,
) -> Result<()> {
    match args.full_page_output {
        ScreenshotFullPageOutput::Single => {
            let stitched = stitch_full_page_capture(&capture)?;
            let encoded = encode_rgba_image(&stitched, image_format)?;
            let bytes = apply_header_if_requested(encoded, args, header_spec, image_format)?;
            fs::write(&args.output, bytes)?;
            Ok(())
        }
        ScreenshotFullPageOutput::Tiles | ScreenshotFullPageOutput::Manifest => {
            let output_dir = build_segment_output_dir(&args.output)?;
            fs::create_dir_all(&output_dir)?;
            let tile_extension = match output_format {
                ScreenshotOutputFormat::Png => "png",
                ScreenshotOutputFormat::Jpeg => "jpg",
                ScreenshotOutputFormat::Pdf => {
                    return Err(message(
                        "PDF output does not support --full-page-output tiles or manifest.",
                    ))
                }
            };
            let mut manifest_segments = Vec::new();
            for segment in &capture.segments {
                let file_name = format!("part-{:04}.{}", segment.index + 1, tile_extension);
                let tile_path = output_dir.join(&file_name);
                let encoded = encode_rgba_image(&segment.image, image_format)?;
                let bytes = if segment.index == 0 {
                    apply_header_if_requested(encoded, args, header_spec, image_format)?
                } else {
                    encoded
                };
                fs::write(&tile_path, bytes)?;
                manifest_segments.push(json!({
                    "file": file_name,
                    "index": segment.index,
                    "scrollY": segment.scroll_y,
                    "sourceTop": segment.source_top,
                    "width": segment.image.width(),
                    "height": segment.image.height(),
                    "headerApplied": segment.index == 0 && header_spec.is_some(),
                }));
            }
            if args.full_page_output == ScreenshotFullPageOutput::Manifest {
                let manifest_path = output_dir.join("manifest.json");
                let manifest = build_full_page_manifest(
                    args,
                    metadata,
                    output_format,
                    &capture,
                    manifest_segments,
                );
                fs::write(manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
            }
            Ok(())
        }
    }
}

fn build_segment_output_dir(output: &Path) -> Result<PathBuf> {
    let parent = output.parent().unwrap_or_else(|| Path::new(""));
    let directory_name = output
        .file_stem()
        .or_else(|| output.file_name())
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            message(
                "Unable to derive a segment output directory from --output. Use a normal filename such as ./dashboard.png.",
            )
        })?;
    Ok(parent.join(directory_name))
}

fn build_full_page_manifest(
    args: &ScreenshotArgs,
    metadata: &DashboardCaptureMetadata,
    output_format: ScreenshotOutputFormat,
    capture: &FullPageCapture,
    segments: Vec<Value>,
) -> Value {
    let title = resolve_manifest_title(
        metadata.dashboard_uid.as_deref(),
        metadata.dashboard_title.as_deref(),
        metadata.panel_title.as_deref(),
        args,
    );
    let header_title = resolve_header_title(args, metadata);
    json!({
        "kind": "dashboard-screenshot-segments",
        "version": 1,
        "outputMode": match args.full_page_output {
            ScreenshotFullPageOutput::Single => "single",
            ScreenshotFullPageOutput::Tiles => "tiles",
            ScreenshotFullPageOutput::Manifest => "manifest",
        },
        "outputFormat": match output_format {
            ScreenshotOutputFormat::Png => "png",
            ScreenshotOutputFormat::Jpeg => "jpeg",
            ScreenshotOutputFormat::Pdf => "pdf",
        },
        "fullPage": args.full_page,
        "output": args.output.display().to_string(),
        "title": title,
        "headerTitle": header_title,
        "dashboardUid": metadata.dashboard_uid,
        "dashboardTitle": metadata.dashboard_title,
        "panelTitle": metadata.panel_title,
        "viewport": {
            "width": capture.viewport_width,
            "height": capture.viewport_height,
            "deviceScaleFactor": capture.device_scale_factor,
        },
        "capture": {
            "totalHeight": capture.total_height,
            "targetWidth": capture.target_width,
            "cropTop": capture.crop_top,
            "cropLeft": capture.crop_left,
            "step": capture.step,
        },
        "segments": segments,
    })
}
