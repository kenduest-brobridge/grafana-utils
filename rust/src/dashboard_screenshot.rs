//! Browser-driven dashboard screenshot helpers.
//!
//! Purpose:
//! - Build Grafana dashboard URLs for browser capture.
//! - Validate screenshot CLI arguments before browser launch.
//! - Reuse dashboard auth headers for a headless Chromium session.
//! - Capture PNG, JPEG, or PDF output through a browser-rendered page.

use chrono::Local;
use font8x8::UnicodeFonts;
use headless_chrome::protocol::cdp::{Emulation, Page};
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use image::{DynamicImage, GenericImage, ImageFormat, Rgba, RgbaImage};
use reqwest::Url;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::common::{message, object_field, string_field, value_as_object, Result};

use super::{
    build_auth_context, build_http_client, fetch_dashboard, ScreenshotArgs,
    ScreenshotFullPageOutput, ScreenshotOutputFormat, ScreenshotTheme,
};

pub fn validate_screenshot_args(args: &ScreenshotArgs) -> Result<()> {
    if args
        .dashboard_uid
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
        && args
            .dashboard_url
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Err(message(
            "Set --dashboard-uid or pass --dashboard-url so the screenshot command knows which dashboard to open.",
        ));
    }
    if args.width == 0 {
        return Err(message("--width must be greater than 0."));
    }
    if args.height == 0 {
        return Err(message("--height must be greater than 0."));
    }
    if !args.device_scale_factor.is_finite() || args.device_scale_factor <= 0.0 {
        return Err(message("--device-scale-factor must be greater than 0."));
    }
    for assignment in &args.vars {
        let (name, value) = parse_var_assignment(assignment)?;
        if name.is_empty() {
            return Err(message(format!(
                "Invalid --var value '{assignment}'. Use NAME=VALUE."
            )));
        }
        if value.is_empty() {
            return Err(message(format!(
                "Invalid --var value '{assignment}'. VALUE cannot be empty."
            )));
        }
    }
    if let Some(vars_query) = args.vars_query.as_deref() {
        let _ = parse_query_fragment(vars_query)?;
    }
    let output_format = infer_screenshot_output_format(&args.output, args.output_format)?;
    if args.full_page_output != ScreenshotFullPageOutput::Single && !args.full_page {
        return Err(message(
            "--full-page-output tiles or manifest requires --full-page.",
        ));
    }
    if args.full_page_output != ScreenshotFullPageOutput::Single
        && output_format == ScreenshotOutputFormat::Pdf
    {
        return Err(message(
            "PDF output does not support --full-page-output tiles or manifest.",
        ));
    }
    Ok(())
}

pub fn infer_screenshot_output_format(
    output: &Path,
    explicit: Option<ScreenshotOutputFormat>,
) -> Result<ScreenshotOutputFormat> {
    if let Some(format) = explicit {
        return Ok(format);
    }
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| {
            message(
                "Unable to infer screenshot output format from --output. Use a .png, .jpg, .jpeg, or .pdf filename or pass --output-format.",
            )
        })?;

    match extension.as_str() {
        "png" => Ok(ScreenshotOutputFormat::Png),
        "jpg" | "jpeg" => Ok(ScreenshotOutputFormat::Jpeg),
        "pdf" => Ok(ScreenshotOutputFormat::Pdf),
        _ => Err(message(format!(
            "Unsupported screenshot output extension '.{extension}'. Use .png, .jpg, .jpeg, or .pdf, or pass --output-format."
        ))),
    }
}

pub fn build_dashboard_capture_url(args: &ScreenshotArgs) -> Result<String> {
    let mut url = match args.dashboard_url.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => Url::parse(value)
            .map_err(|error| message(format!("Invalid --dashboard-url: {error}")))?,
        _ => Url::parse(args.common.url.trim_end_matches('/'))
            .map_err(|error| message(format!("Invalid Grafana base URL: {error}")))?,
    };
    let path_state = parse_dashboard_url_state(&url);
    let fragment_state = match args.vars_query.as_deref() {
        Some(value) => parse_query_fragment(value)?,
        None => DashboardUrlState::default(),
    };
    let dashboard_uid = args
        .dashboard_uid
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or(path_state.dashboard_uid.clone())
        .ok_or_else(|| {
            message("Unable to determine dashboard UID. Pass --dashboard-uid or a Grafana dashboard URL.")
        })?;
    let slug = args
        .slug
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or(path_state.slug.clone())
        .unwrap_or_else(|| dashboard_uid.clone());
    let panel_id = args
        .panel_id
        .or(fragment_state.panel_id)
        .or(path_state.panel_id);
    let org_id = args.org_id.or(fragment_state.org_id).or(path_state.org_id);
    let from = args
        .from
        .as_deref()
        .map(Cow::Borrowed)
        .or(fragment_state.from.as_deref().map(Cow::Borrowed))
        .or(path_state.from.as_deref().map(Cow::Borrowed));
    let to = args
        .to
        .as_deref()
        .map(Cow::Borrowed)
        .or(fragment_state.to.as_deref().map(Cow::Borrowed))
        .or(path_state.to.as_deref().map(Cow::Borrowed));

    url.set_path(&if panel_id.is_some() {
        format!("/d-solo/{dashboard_uid}/{slug}")
    } else {
        format!("/d/{dashboard_uid}/{slug}")
    });

    let mut passthrough_pairs = path_state.passthrough_pairs;
    for (key, value) in fragment_state.passthrough_pairs {
        passthrough_pairs.retain(|(existing_key, _)| existing_key != &key);
        passthrough_pairs.push((key, value));
    }
    let mut merged_vars = path_state.vars;
    for (name, value) in fragment_state.vars {
        merged_vars.retain(|(existing_name, _)| existing_name != &name);
        merged_vars.push((name, value));
    }
    for assignment in &args.vars {
        let (name, value) = parse_var_assignment(assignment)?;
        merged_vars.retain(|(existing_name, _)| existing_name != name);
        merged_vars.push((name.to_string(), value.to_string()));
    }

    {
        let mut pairs = url.query_pairs_mut();
        pairs.clear();
        for (key, value) in passthrough_pairs.drain(..) {
            pairs.append_pair(&key, &value);
        }
        if let Some(panel_id) = panel_id {
            let panel_id_string = panel_id.to_string();
            pairs.append_pair("panelId", &panel_id_string);
            pairs.append_pair("viewPanel", &panel_id_string);
        }
        if let Some(org_id) = org_id {
            let org_id_string = org_id.to_string();
            pairs.append_pair("orgId", &org_id_string);
        }
        if let Some(from) = from.as_deref() {
            pairs.append_pair("from", from);
        }
        if let Some(to) = to.as_deref() {
            pairs.append_pair("to", to);
        }
        pairs.append_pair(
            "theme",
            match args.theme {
                ScreenshotTheme::Light => "light",
                ScreenshotTheme::Dark => "dark",
            },
        );
        pairs.append_pair("kiosk", "tv");
        for (name, value) in merged_vars {
            pairs.append_pair(&format!("var-{name}"), &value);
        }
    }

    Ok(url.to_string())
}

pub fn capture_dashboard_screenshot(args: &ScreenshotArgs) -> Result<()> {
    let mut resolved_args = args.clone();
    validate_screenshot_args(&resolved_args)?;
    let output_format =
        infer_screenshot_output_format(&resolved_args.output, resolved_args.output_format)?;
    let capture_metadata = resolve_dashboard_metadata(&mut resolved_args)?;
    let url = build_dashboard_capture_url(&resolved_args)?;
    if resolved_args.print_capture_url {
        eprintln!("Capture URL: {url}");
    }
    let header_spec = build_header_spec(&resolved_args, &url, &capture_metadata);
    let auth = build_auth_context(&args.common)?;

    if let Some(parent) = resolved_args.output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let browser = build_browser(&resolved_args)?;
    let tab = browser
        .new_tab()
        .map_err(|error| message(format!("Failed to create Chromium tab: {error}")))?;
    configure_capture_viewport(&tab, &resolved_args)?;

    tab.set_extra_http_headers(build_browser_headers(&auth.headers))
        .map_err(|error| message(format!("Failed to set Chromium request headers: {error}")))?;

    tab.navigate_to(&url)
        .map_err(|error| message(format!("Failed to open dashboard URL: {error}")))?;
    wait_for_dashboard_ready(&tab, resolved_args.wait_ms)?;

    collapse_sidebar_if_present(&tab)?;
    let capture_offsets = prepare_dashboard_capture_dom(&tab)?;
    warm_full_page_render(&tab, &resolved_args)?;
    let screenshot_clip = build_screenshot_clip(&tab, &resolved_args)?;

    match output_format {
        ScreenshotOutputFormat::Png => {
            if resolved_args.full_page {
                let segments = capture_full_page_segments(
                    &tab,
                    &resolved_args,
                    &capture_offsets,
                    Page::CaptureScreenshotFormatOption::Png,
                    None,
                )?;
                write_full_page_output(
                    &resolved_args,
                    &header_spec,
                    output_format,
                    segments,
                    ImageFormat::Png,
                )?;
            } else {
                let bytes = tab
                    .capture_screenshot(
                        Page::CaptureScreenshotFormatOption::Png,
                        None,
                        screenshot_clip.clone(),
                        true,
                    )
                    .map_err(|error| {
                        message(format!("Failed to capture PNG screenshot: {error}"))
                    })?;
                let bytes = apply_header_if_requested(
                    bytes,
                    &resolved_args,
                    &header_spec,
                    ImageFormat::Png,
                )?;
                fs::write(&resolved_args.output, bytes)?;
            }
        }
        ScreenshotOutputFormat::Jpeg => {
            if resolved_args.full_page {
                let segments = capture_full_page_segments(
                    &tab,
                    &resolved_args,
                    &capture_offsets,
                    Page::CaptureScreenshotFormatOption::Jpeg,
                    Some(90),
                )?;
                write_full_page_output(
                    &resolved_args,
                    &header_spec,
                    output_format,
                    segments,
                    ImageFormat::Jpeg,
                )?;
            } else {
                let bytes = tab
                    .capture_screenshot(
                        Page::CaptureScreenshotFormatOption::Jpeg,
                        Some(90),
                        screenshot_clip,
                        true,
                    )
                    .map_err(|error| {
                        message(format!("Failed to capture JPEG screenshot: {error}"))
                    })?;
                let bytes = apply_header_if_requested(
                    bytes,
                    &resolved_args,
                    &header_spec,
                    ImageFormat::Jpeg,
                )?;
                fs::write(&resolved_args.output, bytes)?;
            }
        }
        ScreenshotOutputFormat::Pdf => {
            if resolved_args.full_page_output != ScreenshotFullPageOutput::Single {
                return Err(message(
                    "PDF output does not support --full-page-output tiles or manifest.",
                ));
            }
            let pdf = tab
                .print_to_pdf(Some(PrintToPdfOptions {
                    landscape: Some(false),
                    display_header_footer: Some(false),
                    print_background: Some(true),
                    scale: None,
                    paper_width: None,
                    paper_height: None,
                    margin_top: None,
                    margin_bottom: None,
                    margin_left: None,
                    margin_right: None,
                    page_ranges: None,
                    ignore_invalid_page_ranges: None,
                    header_template: None,
                    footer_template: None,
                    prefer_css_page_size: Some(true),
                    transfer_mode: None,
                    generate_tagged_pdf: None,
                    generate_document_outline: None,
                }))
                .map_err(|error| message(format!("Failed to render PDF output: {error}")))?;
            fs::write(&resolved_args.output, pdf)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Default)]
struct DashboardCaptureMetadata {
    dashboard_uid: Option<String>,
    dashboard_title: Option<String>,
    panel_title: Option<String>,
}

#[derive(Debug, Clone)]
struct HeaderSpec {
    lines: Vec<HeaderLine>,
}

#[derive(Debug, Clone)]
struct HeaderLine {
    text: String,
    scale: u32,
    color: Rgba<u8>,
}

#[derive(Debug, Clone)]
struct CapturedSegment {
    image: RgbaImage,
    index: usize,
    scroll_y: u32,
    source_top: u32,
}

#[derive(Debug, Clone)]
struct FullPageCapture {
    total_height: u32,
    target_width: u32,
    viewport_width: u32,
    viewport_height: u32,
    device_scale_factor: f64,
    crop_top: u32,
    crop_left: u32,
    step: u32,
    segments: Vec<CapturedSegment>,
}

fn resolve_dashboard_metadata(args: &mut ScreenshotArgs) -> Result<DashboardCaptureMetadata> {
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

fn find_panel_title(
    dashboard: &serde_json::Map<String, serde_json::Value>,
    panel_id: i64,
) -> Option<String> {
    fn visit_panels(items: &[serde_json::Value], panel_id: i64) -> Option<String> {
        for item in items {
            let object = item.as_object()?;
            if object.get("id").and_then(serde_json::Value::as_i64) == Some(panel_id) {
                let title = string_field(object, "title", "");
                if !title.trim().is_empty() {
                    return Some(title);
                }
            }
            if let Some(nested) = object.get("panels").and_then(serde_json::Value::as_array) {
                if let Some(title) = visit_panels(nested, panel_id) {
                    return Some(title);
                }
            }
        }
        None
    }

    dashboard
        .get("panels")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| visit_panels(items, panel_id))
}

fn build_header_spec(
    args: &ScreenshotArgs,
    resolved_url: &str,
    metadata: &DashboardCaptureMetadata,
) -> Option<HeaderSpec> {
    let mut lines = Vec::new();
    if let Some(text) = resolve_header_title(args, metadata) {
        lines.push(HeaderLine {
            text,
            scale: 2,
            color: Rgba([240, 244, 252, 255]),
        });
    }
    if let Some(text) = resolve_optional_header_field(args.header_url.as_deref(), resolved_url) {
        lines.push(HeaderLine {
            text,
            scale: 1,
            color: Rgba([154, 169, 191, 255]),
        });
    }
    if args.header_captured_at {
        lines.push(HeaderLine {
            text: format!("Captured at {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
            scale: 1,
            color: Rgba([154, 169, 191, 255]),
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
            color: Rgba([210, 218, 230, 255]),
        });
    }
    if lines.is_empty() {
        None
    } else {
        Some(HeaderSpec { lines })
    }
}

fn resolve_header_title(
    args: &ScreenshotArgs,
    metadata: &DashboardCaptureMetadata,
) -> Option<String> {
    match args.header_title.as_deref() {
        Some("__auto__") => resolve_auto_title(metadata, args),
        Some(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        _ => None,
    }
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

fn apply_header_if_requested(
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

fn draw_header_lines(image: &mut RgbaImage, spec: &HeaderSpec) {
    let mut y = 20_u32;
    for line in &spec.lines {
        draw_text_line(image, 24, y, &line.text, line.scale, line.color);
        y += (8 * line.scale) + 10;
    }
}

fn draw_text_line(
    image: &mut RgbaImage,
    start_x: u32,
    start_y: u32,
    text: &str,
    scale: u32,
    color: Rgba<u8>,
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

fn draw_glyph(
    image: &mut RgbaImage,
    start_x: u32,
    start_y: u32,
    character: char,
    scale: u32,
    color: Rgba<u8>,
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
                        image.put_pixel(x, y, color);
                    }
                }
            }
        }
    }
}

fn wait_for_dashboard_ready(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    wait_ms: u64,
) -> Result<()> {
    // Grafana dashboards are SPA routes and some instances never emit the
    // navigation-complete event that headless_chrome expects. Poll DOM
    // readiness instead of failing the entire capture on that event.
    let deadline = Duration::from_millis(wait_ms.max(5_000));
    let start = std::time::Instant::now();
    loop {
        let ready = tab
            .evaluate(
                r#"
(() => {
  const body = document.body;
  const visible = (element) => {
    if (!element) {
      return false;
    }
    const rect = element.getBoundingClientRect();
    const style = window.getComputedStyle(element);
    return style.display !== 'none'
      && style.visibility !== 'hidden'
      && Number.parseFloat(style.opacity || '1') !== 0
      && rect.width > 0
      && rect.height > 0;
  };
  const hasVisibleSpinner = Array.from(document.querySelectorAll('body *')).some((element) => {
    if (!visible(element)) {
      return false;
    }
    const text = ((element.getAttribute('aria-label') || '') + ' ' + (element.getAttribute('title') || '') + ' ' + (element.className || '')).toLowerCase();
    const rect = element.getBoundingClientRect();
    return rect.width >= 24
      && rect.height >= 24
      && rect.width <= 220
      && rect.height <= 220
      && (text.includes('loading') || text.includes('spinner') || text.includes('preloader') || text.includes('grafana'));
  });
  const panelCount = document.querySelectorAll('[data-panelid],[data-testid*="panel"],[class*="panel-container"],[class*="panelContent"]').length;
  const hasMainContent = Array.from(document.querySelectorAll('main, [role="main"], .page-scrollbar, [class*="dashboard-page"]')).some(visible);
  return document.readyState !== 'loading'
    && !!body
    && body.childElementCount > 0
    && hasMainContent
    && panelCount > 0
    && !hasVisibleSpinner;
})()
                "#,
                false,
            )
            .ok()
            .and_then(|remote| remote.value)
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if ready {
            break;
        }
        if start.elapsed() >= deadline {
            return Err(message(
                "Dashboard page did not become ready before the browser wait timeout elapsed.",
            ));
        }
        thread::sleep(Duration::from_millis(250));
    }

    if wait_ms > 0 {
        thread::sleep(Duration::from_millis(wait_ms));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct CaptureOffsets {
    hidden_top_height: f64,
    hidden_left_width: f64,
}

#[derive(Debug, Clone, Default)]
struct DashboardUrlState {
    dashboard_uid: Option<String>,
    slug: Option<String>,
    panel_id: Option<i64>,
    org_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    vars: Vec<(String, String)>,
    passthrough_pairs: Vec<(String, String)>,
}

fn parse_dashboard_url_state(url: &Url) -> DashboardUrlState {
    let mut state = DashboardUrlState::default();
    let segments = match url.path_segments() {
        Some(values) => values.collect::<Vec<_>>(),
        None => Vec::new(),
    };
    if segments.len() >= 3 && (segments[0] == "d" || segments[0] == "d-solo") {
        state.dashboard_uid = Some(segments[1].to_string());
        state.slug = Some(segments[2].to_string());
    }
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "panelId" => {
                state.panel_id = value.parse::<i64>().ok();
            }
            "orgId" => {
                state.org_id = value.parse::<i64>().ok();
            }
            "from" => {
                state.from = Some(value.into_owned());
            }
            "to" => {
                state.to = Some(value.into_owned());
            }
            _ if key.starts_with("var-") => {
                state.vars.push((
                    key.trim_start_matches("var-").to_string(),
                    value.into_owned(),
                ));
            }
            "theme" | "kiosk" | "viewPanel" => {}
            _ => {
                state
                    .passthrough_pairs
                    .push((key.into_owned(), value.into_owned()));
            }
        }
    }
    state
}

fn collapse_sidebar_if_present(tab: &std::sync::Arc<headless_chrome::Tab>) -> Result<()> {
    tab.evaluate(
        r#"
(() => {
  const candidates = Array.from(document.querySelectorAll('button,[role="button"]')).filter((element) => {
    const text = ((element.getAttribute('aria-label') || '') + ' ' + (element.getAttribute('title') || '') + ' ' + (element.innerText || '')).toLowerCase();
    if (!text) {
      return false;
    }
    if (!(text.includes('menu') || text.includes('sidebar') || text.includes('navigation') || text.includes('toggle'))) {
      return false;
    }
    const rect = element.getBoundingClientRect();
    return rect.left <= 120 && rect.top <= 80 && rect.width >= 20 && rect.height >= 20;
  });
  const target = candidates.sort((left, right) => {
    const a = left.getBoundingClientRect();
    const b = right.getBoundingClientRect();
    return (a.left + a.top) - (b.left + b.top);
  })[0];
  if (!target) {
    return false;
  }
  target.click();
  return true;
})()
        "#,
        false,
    )
    .map_err(|error| message(format!("Failed to collapse Grafana sidebar: {error}")))?;
    thread::sleep(Duration::from_millis(800));
    Ok(())
}

fn prepare_dashboard_capture_dom(
    tab: &std::sync::Arc<headless_chrome::Tab>,
) -> Result<CaptureOffsets> {
    tab.evaluate(
        r#"
(() => {
  const isVisible = (element) => {
    const rect = element.getBoundingClientRect();
    const computed = window.getComputedStyle(element);
    return computed.display !== 'none'
      && computed.visibility !== 'hidden'
      && Number.parseFloat(computed.opacity || '1') !== 0
      && rect.width > 0
      && rect.height > 0;
  };
  const hideElement = (element) => {
    element.style.setProperty('display', 'none', 'important');
    element.style.setProperty('visibility', 'hidden', 'important');
    element.style.setProperty('opacity', '0', 'important');
    element.setAttribute('data-grafana-utils-hidden', 'true');
  };
  const style = document.createElement('style');
  style.setAttribute('data-grafana-utils-screenshot', 'true');
  style.textContent = `
    header,
    nav[aria-label],
    aside[aria-label],
    header[aria-label],
    [class*="topnav"],
    [class*="navbar"],
    [class*="subnav"],
    [class*="dashnav"],
    [class*="pageToolbar"],
    [class*="pageHeader"],
    [class*="dashboardHeader"],
    [data-testid*="top-nav"],
    [data-testid*="page-toolbar"],
    [data-testid*="dashboard-controls"],
    [data-testid*="dashboard-toolbar"] {
      display: none !important;
      visibility: hidden !important;
    }
    .sidemenu,
    [class*="sidemenu"],
    [class*="toolbar"] button[aria-label*="Toggle"],
    .sidemenu,
    [class*="sidemenu"] {
      display: none !important;
      visibility: hidden !important;
    }
    main,
    [role="main"],
    .page-scrollbar,
    [class*="pageScroll"],
    [class*="dashboard-page"] {
      margin-left: 0 !important;
      left: 0 !important;
      width: 100% !important;
      max-width: 100% !important;
    }
    body {
      overflow: auto !important;
    }
  `;
  document.head.appendChild(style);
  const sidebarCandidates = Array.from(document.querySelectorAll('body *')).filter((element) => {
    const rect = element.getBoundingClientRect();
    const text = (element.innerText || '').trim();
    if (!text) {
      return false;
    }
    return rect.left <= 8
      && rect.top <= 8
      && rect.width >= 160
      && rect.width <= 360
      && rect.height >= window.innerHeight * 0.5
      && text.includes('Home')
      && text.includes('Dashboards');
  });
  const sidebar = sidebarCandidates.sort((left, right) => {
    return right.getBoundingClientRect().height - left.getBoundingClientRect().height;
  })[0];
  let hiddenTopHeight = 0;
  let hiddenLeftWidth = 0;
  const topBarCandidates = Array.from(document.querySelectorAll('body *')).filter((element) => {
    const rect = element.getBoundingClientRect();
    if (rect.top < -4 || rect.top > 40) {
      return false;
    }
    if (rect.height < 24 || rect.height > 140) {
      return false;
    }
    if (rect.width < window.innerWidth * 0.5) {
      return false;
    }
    const text = (element.innerText || '').trim();
    return text.includes('Search') || text.includes('Refresh') || text.includes('Share') || text.includes('Dashboards');
  });
  const topBar = topBarCandidates.sort((left, right) => {
    return right.getBoundingClientRect().width - left.getBoundingClientRect().width;
  })[0];
  const chromeBars = Array.from(document.querySelectorAll('body *')).filter((element) => {
    if (!isVisible(element)) {
      return false;
    }
    const rect = element.getBoundingClientRect();
    const computed = window.getComputedStyle(element);
    if (!(computed.position === 'fixed' || computed.position === 'sticky')) {
      return false;
    }
    if (rect.top < -8 || rect.top > 96) {
      return false;
    }
    if (rect.height < 24 || rect.height > 160) {
      return false;
    }
    if (rect.width < window.innerWidth * 0.3) {
      return false;
    }
    if (rect.left > 32) {
      return false;
    }
    const text = ((element.innerText || '') + ' ' + (element.getAttribute('aria-label') || '')).toLowerCase();
    return text.includes('refresh')
      || text.includes('search')
      || text.includes('share')
      || text.includes('time range')
      || text.includes('dashboard')
      || text.includes('star')
      || text.includes('settings')
      || text.includes('kiosk');
  });
  if (topBar) {
    hiddenTopHeight = Math.max(hiddenTopHeight, topBar.getBoundingClientRect().bottom);
    hideElement(topBar);
  }
  for (const chromeBar of chromeBars) {
    hiddenTopHeight = Math.max(hiddenTopHeight, chromeBar.getBoundingClientRect().bottom);
    hideElement(chromeBar);
  }
  if (sidebar) {
    hiddenLeftWidth = Math.max(hiddenLeftWidth, sidebar.getBoundingClientRect().width);
    hideElement(sidebar);
  }
  for (const element of Array.from(document.querySelectorAll('body *'))) {
    const computed = window.getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    const marginLeft = Number.parseFloat(computed.marginLeft || '0');
    const left = Number.parseFloat(computed.left || '0');
    const paddingLeft = Number.parseFloat(computed.paddingLeft || '0');
    const marginTop = Number.parseFloat(computed.marginTop || '0');
    const top = Number.parseFloat(computed.top || '0');
    const paddingTop = Number.parseFloat(computed.paddingTop || '0');
    if (Number.isFinite(marginLeft) && marginLeft >= 180 && marginLeft <= 360) {
      element.style.marginLeft = '0px';
    }
    if (Number.isFinite(left) && left >= 180 && left <= 360) {
      element.style.left = '0px';
    }
    if (Number.isFinite(paddingLeft) && paddingLeft >= 180 && paddingLeft <= 360) {
      element.style.paddingLeft = '0px';
    }
    if (Number.isFinite(marginTop) && marginTop >= 32 && marginTop <= 180) {
      element.style.marginTop = '0px';
    }
    if (Number.isFinite(top) && top >= 32 && top <= 180) {
      element.style.top = '0px';
    }
    if (Number.isFinite(paddingTop) && paddingTop >= 32 && paddingTop <= 180) {
      element.style.paddingTop = '0px';
    }
    if (rect.left >= 180 && rect.left <= 360 && rect.width >= window.innerWidth - rect.left - 48) {
      element.style.left = '0px';
      element.style.marginLeft = '0px';
      element.style.width = '100%';
      element.style.maxWidth = '100%';
    }
  }
  window.scrollTo(0, 0);
  window.__grafanaUtilsCaptureOffsets = {
    hiddenTopHeight,
    hiddenLeftWidth
  };
  return true;
})()
        "#,
        false,
    )
    .map_err(|error| message(format!("Failed to prepare dashboard DOM for capture: {error}")))?;
    thread::sleep(Duration::from_millis(250));
    let hidden_top_height = read_numeric_expression(
        tab,
        "window.__grafanaUtilsCaptureOffsets?.hiddenTopHeight ?? 0",
        0.0,
    )?;
    let hidden_left_width = read_numeric_expression(
        tab,
        "window.__grafanaUtilsCaptureOffsets?.hiddenLeftWidth ?? 0",
        0.0,
    )?;
    Ok(CaptureOffsets {
        hidden_top_height,
        hidden_left_width,
    })
}

fn build_screenshot_clip(
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

fn warm_full_page_render(
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

fn capture_full_page_segments(
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
            .fold(0_u32, |acc, height| acc.saturating_add(height));
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

fn write_full_page_output(
    args: &ScreenshotArgs,
    header_spec: &Option<HeaderSpec>,
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
                let manifest =
                    build_full_page_manifest(args, output_format, &capture, manifest_segments);
                fs::write(
                    manifest_path,
                    serde_json::to_vec_pretty(&manifest).map_err(|error| {
                        message(format!(
                            "Failed to encode screenshot segment manifest: {error}"
                        ))
                    })?,
                )?;
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
    output_format: ScreenshotOutputFormat,
    capture: &FullPageCapture,
    segments: Vec<Value>,
) -> Value {
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

fn read_numeric_expression(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    expression: &str,
    minimum: f64,
) -> Result<f64> {
    let remote = tab.evaluate(expression, false).map_err(|error| {
        message(format!(
            "Failed to read page dimensions for --full-page: {error}"
        ))
    })?;
    let raw = remote
        .value
        .and_then(|value| value.as_f64())
        .ok_or_else(|| message("Chromium did not return page dimensions for --full-page."))?;
    Ok(raw.max(minimum).ceil())
}

fn parse_var_assignment(assignment: &str) -> Result<(&str, &str)> {
    let (name, value) = assignment.split_once('=').ok_or_else(|| {
        message(format!(
            "Invalid --var value '{assignment}'. Use NAME=VALUE."
        ))
    })?;
    let trimmed_name = name.trim();
    let trimmed_value = value.trim();
    if trimmed_name.is_empty() || trimmed_value.is_empty() {
        return Err(message(format!(
            "Invalid --var value '{assignment}'. Use NAME=VALUE with non-empty parts."
        )));
    }
    Ok((trimmed_name, trimmed_value))
}

pub(crate) fn parse_vars_query(query: &str) -> Result<Vec<(String, String)>> {
    Ok(parse_query_fragment(query)?.vars)
}

fn parse_query_fragment(query: &str) -> Result<DashboardUrlState> {
    let trimmed = query.trim().trim_start_matches('?');
    if trimmed.is_empty() {
        return Ok(DashboardUrlState::default());
    }
    let parsed = Url::parse(&format!("http://localhost/?{trimmed}"))
        .map_err(|error| message(format!("Invalid --vars-query value: {error}")))?;
    let mut state = DashboardUrlState::default();
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "panelId" => {
                state.panel_id = value.parse::<i64>().ok();
            }
            "orgId" => {
                state.org_id = value.parse::<i64>().ok();
            }
            "from" => {
                state.from = Some(value.into_owned());
            }
            "to" => {
                state.to = Some(value.into_owned());
            }
            _ if key.starts_with("var-") => {
                let name = key.trim_start_matches("var-").trim().to_string();
                let value = value.trim().to_string();
                if name.is_empty() || value.is_empty() {
                    return Err(message(
                        "Invalid --vars-query value. Each var-* item must have a non-empty name and value.",
                    ));
                }
                state
                    .vars
                    .retain(|(existing_name, _)| existing_name != &name);
                state.vars.push((name, value));
            }
            "theme" | "kiosk" | "viewPanel" => {}
            _ => {
                state
                    .passthrough_pairs
                    .retain(|(existing_key, _)| existing_key != key.as_ref());
                state
                    .passthrough_pairs
                    .push((key.into_owned(), value.into_owned()));
            }
        }
    }
    Ok(state)
}

fn build_browser_headers(headers: &[(String, String)]) -> HashMap<&str, &str> {
    let mut result = HashMap::new();
    for (name, value) in headers {
        result.insert(name.as_str(), value.as_str());
    }
    result
}

fn build_browser(args: &ScreenshotArgs) -> Result<Browser> {
    let debug_port = reserve_debug_port()?;
    let mut builder = LaunchOptionsBuilder::default();
    builder
        .headless(true)
        .sandbox(false)
        .window_size(Some((args.width, args.height)))
        .port(Some(debug_port))
        .ignore_certificate_errors(!args.common.verify_ssl);

    if let Some(path) = args.browser_path.as_ref() {
        builder.path(Some(path.to_path_buf()));
    }

    let options = builder
        .build()
        .map_err(|error| message(format!("Failed to build Chromium launch options: {error}")))?;
    Browser::new(options).map_err(|error| {
        message(format!(
            "Failed to launch Chromium browser session: {error}"
        ))
    })
}

fn configure_capture_viewport(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    args: &ScreenshotArgs,
) -> Result<()> {
    tab.call_method(Emulation::SetDeviceMetricsOverride {
        width: args.width,
        height: args.height,
        device_scale_factor: args.device_scale_factor,
        mobile: false,
        scale: None,
        screen_width: None,
        screen_height: None,
        position_x: None,
        position_y: None,
        dont_set_visible_size: None,
        screen_orientation: None,
        viewport: None,
        display_feature: None,
        device_posture: None,
    })
    .map_err(|error| {
        message(format!(
            "Failed to configure Chromium device metrics override: {error}"
        ))
    })?;
    Ok(())
}

fn reserve_debug_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| message(format!("Failed to reserve Chromium debug port: {error}")))?;
    let port = listener
        .local_addr()
        .map_err(|error| message(format!("Failed to inspect Chromium debug port: {error}")))?
        .port();
    drop(listener);
    Ok(port)
}
