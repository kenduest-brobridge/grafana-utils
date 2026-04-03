//! Browser-driven dashboard screenshot helpers.
//!
//! Purpose:
//! - Build Grafana dashboard URLs for browser capture.
//! - Validate screenshot CLI arguments before browser launch.
//! - Reuse dashboard auth headers for a headless Chromium session.
//! - Capture PNG, JPEG, or PDF output through a browser-rendered page.
#![cfg_attr(not(feature = "browser"), allow(dead_code))]

#[cfg(feature = "browser")]
#[path = "screenshot_full_page.rs"]
mod screenshot_full_page;
#[path = "screenshot_header.rs"]
mod screenshot_header;
#[path = "screenshot_runtime.rs"]
mod screenshot_runtime;

use reqwest::Url;
use std::borrow::Cow;
use std::path::Path;

use crate::common::{invalid_url, message, Result};

use super::{ScreenshotArgs, ScreenshotFullPageOutput, ScreenshotOutputFormat, ScreenshotTheme};
#[allow(unused_imports)]
pub(crate) use screenshot_header::resolve_manifest_title;
use screenshot_runtime::{parse_dashboard_url_state, parse_query_fragment, parse_var_assignment};
pub(crate) use screenshot_runtime::{parse_vars_query, DashboardUrlState};

#[cfg(feature = "browser")]
use headless_chrome::protocol::cdp::Page;
#[cfg(feature = "browser")]
use headless_chrome::types::PrintToPdfOptions;
#[cfg(feature = "browser")]
use image::ImageFormat;
#[cfg(feature = "browser")]
use std::fs;

#[cfg(feature = "browser")]
use super::build_auth_context;
#[cfg(feature = "browser")]
use screenshot_full_page::{
    build_screenshot_clip, capture_full_page_segments, warm_full_page_render,
    write_full_page_output,
};
#[cfg(feature = "browser")]
use screenshot_header::apply_header_if_requested;
#[cfg(feature = "browser")]
use screenshot_header::{build_header_spec, resolve_dashboard_metadata};
#[cfg(feature = "browser")]
use screenshot_runtime::{
    build_browser, build_browser_headers, collapse_sidebar_if_present, configure_capture_viewport,
    prepare_dashboard_capture_dom, read_numeric_expression, wait_for_dashboard_ready,
};

/// validate screenshot args.
pub fn validate_screenshot_args(args: &ScreenshotArgs) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: dashboard_rust_tests.rs:validate_screenshot_args_rejects_invalid_device_scale_factor, dashboard_rust_tests.rs:validate_screenshot_args_rejects_invalid_var_assignment, dashboard_rust_tests.rs:validate_screenshot_args_rejects_pdf_split_output, dashboard_rust_tests.rs:validate_screenshot_args_rejects_split_output_without_full_page, dashboard_screenshot.rs:capture_dashboard_screenshot
    // Downstream callees: common.rs:message, dashboard_screenshot.rs:infer_screenshot_output_format, dashboard_screenshot.rs:parse_query_fragment, dashboard_screenshot.rs:parse_var_assignment

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

/// infer screenshot output format.
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_dashboard_capture_url(args: &ScreenshotArgs) -> Result<String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: dashboard_rust_tests.rs:build_dashboard_capture_url_includes_panel_time_theme_and_vars, dashboard_rust_tests.rs:build_dashboard_capture_url_merges_vars_query_between_url_and_explicit_vars, dashboard_rust_tests.rs:build_dashboard_capture_url_preserves_non_var_query_from_vars_query, dashboard_rust_tests.rs:build_dashboard_capture_url_reuses_full_dashboard_url_state, dashboard_rust_tests.rs:build_dashboard_capture_url_supports_datasource_style_template_variables, dashboard_screenshot.rs:capture_dashboard_screenshot
    // Downstream callees: common.rs:message, dashboard_screenshot.rs:parse_dashboard_url_state, dashboard_screenshot.rs:parse_query_fragment, dashboard_screenshot.rs:parse_var_assignment

    let mut url = match args.dashboard_url.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => {
            Url::parse(value).map_err(|error| invalid_url("--dashboard-url", error))?
        }
        _ => Url::parse(args.common.url.trim_end_matches('/'))
            .map_err(|error| invalid_url("Grafana base URL", error))?,
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

/// capture dashboard screenshot.
pub fn capture_dashboard_screenshot(args: &ScreenshotArgs) -> Result<()> {
    #[cfg(feature = "browser")]
    {
        // Call graph (hierarchy): this function is used in related modules.
        // Upstream callers: dashboard.rs:run_dashboard_cli, dashboard.rs:run_dashboard_cli_with_client
        // Downstream callees: common.rs:message, dashboard_screenshot.rs:build_browser, dashboard_screenshot.rs:build_browser_headers, dashboard_screenshot.rs:build_dashboard_capture_url, dashboard_screenshot.rs:build_screenshot_clip, dashboard_screenshot.rs:capture_full_page_segments, dashboard_screenshot.rs:collapse_sidebar_if_present, dashboard_screenshot.rs:configure_capture_viewport, dashboard_screenshot.rs:infer_screenshot_output_format, dashboard_screenshot.rs:prepare_dashboard_capture_dom, screenshot_header.rs:apply_header_if_requested, screenshot_header.rs:build_header_spec, screenshot_header.rs:resolve_dashboard_metadata ...

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
                        &capture_metadata,
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
                        &capture_metadata,
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

    #[cfg(not(feature = "browser"))]
    {
        let _ = args;
        Err(message(
            "Dashboard screenshot support was not built in. Rebuild with the `browser` feature enabled.",
        ))
    }
}
