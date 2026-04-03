//! Dashboard screenshot runtime helpers.
#![cfg_attr(not(feature = "browser"), allow(dead_code))]

#[cfg(feature = "browser")]
use headless_chrome::protocol::cdp::Emulation;
#[cfg(feature = "browser")]
use headless_chrome::{Browser, LaunchOptionsBuilder};
use reqwest::Url;
#[cfg(feature = "browser")]
use std::collections::HashMap;
#[cfg(feature = "browser")]
use std::net::TcpListener;
#[cfg(feature = "browser")]
use std::thread;
#[cfg(feature = "browser")]
use std::time::Duration;

use crate::common::{message, Result};

#[cfg(feature = "browser")]
use super::ScreenshotArgs;

#[cfg(feature = "browser")]
#[derive(Debug, Clone, Copy)]
pub(crate) struct CaptureOffsets {
    pub(crate) hidden_top_height: f64,
    pub(crate) hidden_left_width: f64,
}

#[cfg(feature = "browser")]
#[derive(Debug, Clone)]
pub(crate) struct CapturedSegment {
    pub(crate) image: image::RgbaImage,
    pub(crate) index: usize,
    pub(crate) scroll_y: u32,
    pub(crate) source_top: u32,
}

#[cfg(feature = "browser")]
#[derive(Debug, Clone)]
pub(crate) struct FullPageCapture {
    pub(crate) total_height: u32,
    pub(crate) target_width: u32,
    pub(crate) viewport_width: u32,
    pub(crate) viewport_height: u32,
    pub(crate) device_scale_factor: f64,
    pub(crate) crop_top: u32,
    pub(crate) crop_left: u32,
    pub(crate) step: u32,
    pub(crate) segments: Vec<CapturedSegment>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DashboardUrlState {
    pub(crate) dashboard_uid: Option<String>,
    pub(crate) slug: Option<String>,
    pub(crate) panel_id: Option<i64>,
    pub(crate) org_id: Option<i64>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) vars: Vec<(String, String)>,
    pub(crate) passthrough_pairs: Vec<(String, String)>,
}

#[cfg(feature = "browser")]
pub(crate) fn wait_for_dashboard_ready(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    wait_ms: u64,
) -> Result<()> {
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

#[cfg(feature = "browser")]
pub(crate) fn collapse_sidebar_if_present(
    tab: &std::sync::Arc<headless_chrome::Tab>,
) -> Result<()> {
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

#[cfg(feature = "browser")]
pub(crate) fn prepare_dashboard_capture_dom(
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

#[cfg(feature = "browser")]
pub(crate) fn read_numeric_expression(
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

pub(crate) fn parse_var_assignment(assignment: &str) -> Result<(&str, &str)> {
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

pub(crate) fn parse_query_fragment(query: &str) -> Result<DashboardUrlState> {
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

pub(crate) fn parse_dashboard_url_state(url: &Url) -> DashboardUrlState {
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

#[cfg(feature = "browser")]
pub(crate) fn build_browser_headers(headers: &[(String, String)]) -> HashMap<&str, &str> {
    let mut result = HashMap::new();
    for (name, value) in headers {
        result.insert(name.as_str(), value.as_str());
    }
    result
}

#[cfg(feature = "browser")]
pub(crate) fn build_browser(args: &ScreenshotArgs) -> Result<Browser> {
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

#[cfg(feature = "browser")]
pub(crate) fn configure_capture_viewport(
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

#[cfg(feature = "browser")]
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
