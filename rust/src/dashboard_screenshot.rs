//! Browser-driven dashboard screenshot helpers.
//!
//! Purpose:
//! - Build Grafana dashboard URLs for browser capture.
//! - Validate screenshot CLI arguments before browser launch.
//! - Reuse dashboard auth headers for a headless Chromium session.
//! - Capture PNG, JPEG, or PDF output through a browser-rendered page.

use headless_chrome::protocol::cdp::Page;
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use image::{DynamicImage, GenericImage, ImageFormat, RgbaImage};
use reqwest::Url;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::common::{message, object_field, string_field, value_as_object, Result};

use super::{
    build_auth_context, build_http_client, fetch_dashboard, ScreenshotArgs, ScreenshotOutputFormat,
    ScreenshotTheme,
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
    let _ = infer_screenshot_output_format(&args.output, args.output_format)?;
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
        Some(value) if !value.is_empty() => {
            Url::parse(value).map_err(|error| message(format!("Invalid --dashboard-url: {error}")))?
        }
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
    resolve_dashboard_slug(&mut resolved_args)?;
    let url = build_dashboard_capture_url(&resolved_args)?;
    if resolved_args.print_capture_url {
        eprintln!("Capture URL: {url}");
    }
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
            let bytes = if resolved_args.full_page {
                capture_stitched_screenshot(
                    &tab,
                    &resolved_args,
                    &capture_offsets,
                    Page::CaptureScreenshotFormatOption::Png,
                    None,
                )?
            } else {
                tab.capture_screenshot(
                    Page::CaptureScreenshotFormatOption::Png,
                    None,
                    screenshot_clip.clone(),
                    true,
                )
                .map_err(|error| message(format!("Failed to capture PNG screenshot: {error}")))?
            };
            fs::write(&resolved_args.output, bytes)?;
        }
        ScreenshotOutputFormat::Jpeg => {
            let bytes = if resolved_args.full_page {
                capture_stitched_screenshot(
                    &tab,
                    &resolved_args,
                    &capture_offsets,
                    Page::CaptureScreenshotFormatOption::Jpeg,
                    Some(90),
                )?
            } else {
                tab.capture_screenshot(
                    Page::CaptureScreenshotFormatOption::Jpeg,
                    Some(90),
                    screenshot_clip,
                    true,
                )
                .map_err(|error| message(format!("Failed to capture JPEG screenshot: {error}")))?
            };
            fs::write(&resolved_args.output, bytes)?;
        }
        ScreenshotOutputFormat::Pdf => {
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

fn resolve_dashboard_slug(args: &mut ScreenshotArgs) -> Result<()> {
    if args
        .slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        || args
            .dashboard_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
    {
        return Ok(());
    }
    let dashboard_uid = match args
        .dashboard_uid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => value,
        None => return Ok(()),
    };
    let client = build_http_client(&args.common)?;
    let payload = fetch_dashboard(&client, dashboard_uid)?;
    let object = value_as_object(&payload, "Unexpected dashboard payload from Grafana.")?;
    let meta = match object_field(object, "meta") {
        Some(value) => value,
        None => return Ok(()),
    };
    let slug = string_field(meta, "slug", "");
    if !slug.trim().is_empty() {
        args.slug = Some(slug);
    }
    Ok(())
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
                state
                    .vars
                    .push((key.trim_start_matches("var-").to_string(), value.into_owned()));
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

fn prepare_dashboard_capture_dom(tab: &std::sync::Arc<headless_chrome::Tab>) -> Result<CaptureOffsets> {
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
        scale: 1.0,
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
        tab.evaluate(&scroll_script, false)
            .map_err(|error| message(format!("Failed to scroll dashboard for --full-page: {error}")))?;
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

    tab.evaluate("window.scrollTo({ top: 0, left: 0, behavior: 'instant' })", false)
        .map_err(|error| message(format!("Failed to reset dashboard scroll position: {error}")))?;
    thread::sleep(Duration::from_millis(300));
    Ok(())
}

fn capture_stitched_screenshot(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    args: &ScreenshotArgs,
    capture_offsets: &CaptureOffsets,
    format: Page::CaptureScreenshotFormatOption,
    quality: Option<u32>,
) -> Result<Vec<u8>> {
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
    let viewport_width = args.width as u32;
    let crop_top = capture_offsets.hidden_top_height.max(0.0).ceil() as u32;
    let crop_left = capture_offsets.hidden_left_width.max(0.0).ceil() as u32;
    let target_width = viewport_width.saturating_sub(crop_left).max(1);
    let step = (viewport_height - capture_offsets.hidden_top_height.max(0.0)).max(200.0);

    let mut stitched = RgbaImage::new(target_width, total_height.ceil() as u32);
    let mut destination_y = 0_u32;
    let mut current_y = 0.0_f64;

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
            .map_err(|error| message(format!("Failed to capture stitched screenshot segment: {error}")))?;
        let segment = image::load_from_memory(&bytes)
            .map_err(|error| message(format!("Failed to decode stitched screenshot segment: {error}")))?;
        let segment_rgba = segment.to_rgba8();
        let segment_height = segment_rgba.height();
        let segment_width = segment_rgba.width();
        let source_left = crop_left.min(segment_width.saturating_sub(1));
        let source_top = if current_y <= 0.0 { 0 } else { crop_top.min(segment_height) };
        let remaining_height = stitched.height().saturating_sub(destination_y);
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
        stitched
            .copy_from(&cropped, 0, destination_y)
            .map_err(|error| message(format!("Failed to stitch screenshot segment: {error}")))?;

        destination_y = destination_y.saturating_add(copy_height);
        if destination_y >= stitched.height() {
            break;
        }
        current_y += step;
    }

    let final_height = destination_y.max(1);
    let final_image = DynamicImage::ImageRgba8(
        image::imageops::crop_imm(&stitched, 0, 0, target_width, final_height).to_image(),
    );
    let mut encoded = std::io::Cursor::new(Vec::new());
    match format {
        Page::CaptureScreenshotFormatOption::Png => final_image
            .write_to(&mut encoded, ImageFormat::Png)
            .map_err(|error| message(format!("Failed to encode stitched PNG screenshot: {error}")))?,
        Page::CaptureScreenshotFormatOption::Jpeg => final_image
            .write_to(&mut encoded, ImageFormat::Jpeg)
            .map_err(|error| message(format!("Failed to encode stitched JPEG screenshot: {error}")))?,
        Page::CaptureScreenshotFormatOption::Webp => {
            return Err(message(
                "WEBP stitched screenshot encoding is not supported by this command.",
            ))
        }
    }
    Ok(encoded.into_inner())
}

fn read_numeric_expression(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    expression: &str,
    minimum: f64,
) -> Result<f64> {
    let remote = tab
        .evaluate(expression, false)
        .map_err(|error| message(format!("Failed to read page dimensions for --full-page: {error}")))?;
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
                state.vars.retain(|(existing_name, _)| existing_name != &name);
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
