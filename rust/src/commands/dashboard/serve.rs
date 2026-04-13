//! Lightweight local dashboard preview server for draft authoring.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;

use crate::common::{message, string_field, value_as_object, Result};

use super::{extract_dashboard_object, load_json_file, ServeArgs};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathFingerprint {
    modified_millis: u128,
    len: u64,
}

#[derive(Debug, Clone, Serialize)]
struct DashboardServeItem {
    title: String,
    uid: String,
    source: String,
    document_kind: String,
    dashboard: Value,
}

#[derive(Debug, Clone, Serialize)]
struct DashboardServeDocument {
    item_count: usize,
    items: Vec<DashboardServeItem>,
    last_reload_millis: u128,
    last_error: Option<String>,
}

fn walk_dashboard_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if root.is_file() {
        files.push(root.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dashboard_files(&path, files)?;
            continue;
        }
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if matches!(ext, "json" | "yaml" | "yml") {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_dashboard_value(value: Value, source: String) -> Result<DashboardServeItem> {
    let object = value_as_object(&value, "Dashboard serve expects JSON/YAML objects.")?;
    let dashboard = extract_dashboard_object(object)?;
    Ok(DashboardServeItem {
        title: string_field(dashboard, "title", "dashboard"),
        uid: string_field(dashboard, "uid", source.as_str()),
        source,
        document_kind: if object.contains_key("dashboard") {
            "wrapped".to_string()
        } else {
            "bare".to_string()
        },
        dashboard: value,
    })
}

fn parse_script_output(raw: &str, yaml: bool) -> Result<Vec<Value>> {
    let value = if yaml {
        serde_yaml::from_str::<Value>(raw).map_err(|error| {
            message(format!(
                "Failed to parse dashboard serve YAML script output: {error}"
            ))
        })?
    } else {
        serde_json::from_str::<Value>(raw).map_err(|error| {
            message(format!(
                "Failed to parse dashboard serve JSON script output: {error}"
            ))
        })?
    };
    match value {
        Value::Array(items) => Ok(items),
        other => Ok(vec![other]),
    }
}

fn load_input_items(args: &ServeArgs) -> Result<Vec<DashboardServeItem>> {
    if let Some(script) = args.script.as_ref() {
        let output = Command::new("/bin/sh")
            .arg("-lc")
            .arg(script)
            .output()
            .map_err(|error| message(format!("Failed to run dashboard serve script: {error}")))?;
        if !output.status.success() {
            return Err(message(format!(
                "Dashboard serve script exited with status {}.",
                output.status
            )));
        }
        return parse_script_output(
            std::str::from_utf8(&output.stdout).map_err(|error| {
                message(format!(
                    "Dashboard serve script stdout is not UTF-8: {error}"
                ))
            })?,
            matches!(args.script_format, super::DashboardServeScriptFormat::Yaml),
        )?
        .into_iter()
        .enumerate()
        .map(|(index, value)| parse_dashboard_value(value, format!("script:{index}")))
        .collect();
    }

    let input = args
        .input
        .as_ref()
        .ok_or_else(|| message("dashboard serve requires --input or --script."))?;
    let mut files = Vec::new();
    walk_dashboard_files(input, &mut files)?;
    if files.is_empty() {
        return Err(message(format!(
            "No dashboard files found under {}.",
            input.display()
        )));
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let value = match path.extension().and_then(|value| value.to_str()) {
                Some("yaml") | Some("yml") => {
                    serde_yaml::from_str::<Value>(&fs::read_to_string(&path)?).map_err(|error| {
                        message(format!(
                            "Failed to parse dashboard YAML file {}: {error}",
                            path.display()
                        ))
                    })?
                }
                _ => load_json_file(&path)?,
            };
            parse_dashboard_value(value, path.display().to_string())
        })
        .collect()
}

fn current_fingerprint(path: &Path) -> Result<Option<PathFingerprint>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            let modified = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(|error| {
                    message(format!(
                        "Dashboard serve file timestamp is before UNIX_EPOCH for {}: {error}",
                        path.display()
                    ))
                })?;
            Ok(Some(PathFingerprint {
                modified_millis: modified.as_millis(),
                len: metadata.len(),
            }))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn collect_watch_paths(args: &ServeArgs) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(input) = args.input.as_ref() {
        paths.push(input.clone());
    }
    for path in &args.watch {
        if !paths.contains(path) {
            paths.push(path.clone());
        }
    }
    paths
}

fn serve_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>grafana-util dashboard serve</title>
  <style>
    body { font-family: ui-monospace, Menlo, Consolas, monospace; margin: 0; background: #0f141a; color: #e6edf3; }
    header { padding: 16px 20px; border-bottom: 1px solid #2d333b; }
    main { display: grid; grid-template-columns: 320px 1fr; min-height: calc(100vh - 60px); }
    nav { border-right: 1px solid #2d333b; padding: 16px; }
    nav button { display: block; width: 100%; margin-bottom: 8px; padding: 10px 12px; background: #161b22; color: #e6edf3; border: 1px solid #30363d; text-align: left; cursor: pointer; }
    nav button.active { border-color: #58a6ff; background: #0d2238; }
    section { padding: 16px; }
    pre { white-space: pre-wrap; word-break: break-word; background: #161b22; padding: 12px; border: 1px solid #30363d; border-radius: 6px; overflow: auto; }
    .meta { color: #8b949e; margin-bottom: 12px; }
    .error { color: #ffa198; background: #2d1117; border: 1px solid #8b2f2f; padding: 10px 12px; border-radius: 6px; margin-bottom: 12px; }
  </style>
</head>
<body>
  <header><strong>grafana-util dashboard serve</strong> <span id="status"></span></header>
  <main>
    <nav id="list"></nav>
    <section>
      <div class="meta" id="meta"></div>
      <div id="error" class="error" hidden></div>
      <pre id="payload">Loading…</pre>
    </section>
  </main>
  <script>
    let selectedIndex = 0;
    async function refresh() {
      const response = await fetch('/index.json', { cache: 'no-store' });
      const payloadDocument = await response.json();
      window.document.title = 'grafana-util dashboard serve';
      const list = window.document.getElementById('list');
      const meta = window.document.getElementById('meta');
      const error = window.document.getElementById('error');
      const payload = window.document.getElementById('payload');
      const status = window.document.getElementById('status');
      if (selectedIndex >= payloadDocument.items.length) selectedIndex = 0;
      list.innerHTML = '';
      payloadDocument.items.forEach((item, index) => {
        const button = window.document.createElement('button');
        button.textContent = item.title + ' [' + item.uid + ']';
        if (index === selectedIndex) button.classList.add('active');
        button.onclick = () => { selectedIndex = index; refresh(); };
        list.appendChild(button);
      });
      status.textContent = 'items=' + payloadDocument.item_count + ' reload=' + new Date(payloadDocument.last_reload_millis).toLocaleTimeString();
      if (payloadDocument.last_error) {
        error.hidden = false;
        error.textContent = 'Last reload error: ' + payloadDocument.last_error;
      } else {
        error.hidden = true;
        error.textContent = '';
      }
      const current = payloadDocument.items[selectedIndex];
      if (!current) {
        meta.textContent = 'No dashboards loaded.';
        payload.textContent = '';
        return;
      }
      meta.textContent = 'source=' + current.source + ' kind=' + current.document_kind;
      payload.textContent = JSON.stringify(current.dashboard, null, 2);
    }
    refresh();
    setInterval(refresh, 2000);
  </script>
</body>
</html>"#
}

fn serve_document(state: &Arc<Mutex<DashboardServeDocument>>) -> Result<String> {
    let document = state
        .lock()
        .map_err(|_| message("Dashboard serve state lock is poisoned."))?
        .clone();
    serde_json::to_string_pretty(&document)
        .map_err(|error| message(format!("Dashboard serve JSON rendering failed: {error}")))
}

fn current_reload_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn update_reload_state(
    state: &Arc<Mutex<DashboardServeDocument>>,
    items: Vec<DashboardServeItem>,
    last_error: Option<String>,
) -> Result<()> {
    let mut guard = state
        .lock()
        .map_err(|_| message("Dashboard serve state lock is poisoned."))?;
    guard.item_count = items.len();
    guard.items = items;
    guard.last_reload_millis = current_reload_millis();
    guard.last_error = last_error;
    Ok(())
}

fn open_preview_browser(url: &str) -> Result<()> {
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).status()
    } else if cfg!(target_family = "unix") {
        Command::new("xdg-open").arg(url).status()
    } else {
        return Err(message(
            "Dashboard serve cannot open a browser automatically on this platform.",
        ));
    }
    .map_err(|error| message(format!("Dashboard serve browser launch failed: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(message(format!(
            "Dashboard serve browser launch exited with status {status}."
        )))
    }
}

fn response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

pub(crate) fn run_dashboard_serve(args: &ServeArgs) -> Result<()> {
    let items = load_input_items(args)?;
    let state = Arc::new(Mutex::new(DashboardServeDocument {
        item_count: items.len(),
        items,
        last_reload_millis: current_reload_millis(),
        last_error: None,
    }));

    let watch_paths = collect_watch_paths(args);
    if !args.no_watch && !watch_paths.is_empty() {
        let state = Arc::clone(&state);
        let args = args.clone();
        thread::spawn(move || {
            let mut previous = watch_paths
                .iter()
                .filter_map(|path| {
                    current_fingerprint(path)
                        .ok()
                        .flatten()
                        .map(|fp| (path.clone(), fp))
                })
                .collect::<Vec<_>>();
            loop {
                thread::sleep(Duration::from_secs(1));
                let mut changed = false;
                for path in &watch_paths {
                    let current = current_fingerprint(path).ok().flatten();
                    let previous_entry =
                        previous.iter().position(|(candidate, _)| candidate == path);
                    match (previous_entry, current) {
                        (Some(index), Some(fingerprint)) if previous[index].1 != fingerprint => {
                            previous[index].1 = fingerprint;
                            changed = true;
                        }
                        (None, Some(fingerprint)) => {
                            previous.push((path.clone(), fingerprint));
                            changed = true;
                        }
                        (Some(index), None) => {
                            previous.remove(index);
                            changed = true;
                        }
                        _ => {}
                    }
                }
                if !changed {
                    continue;
                }
                match load_input_items(&args) {
                    Ok(items) => {
                        if let Err(error) = update_reload_state(&state, items, None) {
                            eprintln!("Dashboard serve reload state update failed: {error}");
                            continue;
                        }
                        eprintln!("Reloaded dashboard preview from watched inputs.");
                    }
                    Err(error) => {
                        let existing_items = match state.lock() {
                            Ok(guard) => guard.items.clone(),
                            Err(_) => {
                                eprintln!("Dashboard serve state lock is poisoned.");
                                Vec::new()
                            }
                        };
                        if let Err(state_error) =
                            update_reload_state(&state, existing_items, Some(error.to_string()))
                        {
                            eprintln!("Dashboard serve reload state update failed: {state_error}");
                        }
                        eprintln!("Dashboard serve reload failed: {error}");
                    }
                }
            }
        });
    }

    let bind = format!("{}:{}", args.address, args.port);
    let listener = TcpListener::bind(&bind)
        .map_err(|error| message(format!("Dashboard serve could not bind {bind}: {error}")))?;
    listener.set_nonblocking(true).map_err(|error| {
        message(format!(
            "Dashboard serve could not enable nonblocking accept: {error}"
        ))
    })?;
    let preview_url = format!("http://{bind}");
    println!("Dashboard preview available at {preview_url}");
    if args.open_browser {
        match open_preview_browser(&preview_url) {
            Ok(()) => eprintln!("Opened dashboard preview in your default browser."),
            Err(error) => eprintln!("Dashboard serve browser launch failed: {error}"),
        }
    }

    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|error| {
                        message(format!(
                            "Dashboard serve read timeout setup failed: {error}"
                        ))
                    })?;
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let bytes_read = match stream.read(&mut buffer) {
                        Ok(bytes_read) => bytes_read,
                        Err(error)
                            if matches!(
                                error.kind(),
                                ErrorKind::WouldBlock | ErrorKind::TimedOut
                            ) =>
                        {
                            0
                        }
                        Err(error) => {
                            eprintln!("Dashboard serve request read failed: {error}");
                            0
                        }
                    };
                    if bytes_read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..bytes_read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request_text = String::from_utf8_lossy(&request);
                let target = request_text
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let body = match target {
                    "/" => response("200 OK", "text/html; charset=utf-8", serve_html()),
                    "/index.json" => match serve_document(&state) {
                        Ok(body) => response("200 OK", "application/json", &body),
                        Err(error) => response(
                            "500 Internal Server Error",
                            "text/plain; charset=utf-8",
                            &error.to_string(),
                        ),
                    },
                    _ => response("404 Not Found", "text/plain; charset=utf-8", "not found"),
                };
                if let Err(error) = stream.write_all(body.as_bytes()) {
                    eprintln!("Dashboard serve response write failed: {error}");
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                return Err(message(format!(
                    "Dashboard serve listener accept failed: {error}"
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serve_document_serializes_last_error_state() {
        let state = Arc::new(Mutex::new(DashboardServeDocument {
            item_count: 1,
            items: vec![DashboardServeItem {
                title: "CPU Main".to_string(),
                uid: "cpu-main".to_string(),
                source: "./drafts/cpu-main.json".to_string(),
                document_kind: "wrapped".to_string(),
                dashboard: json!({
                    "dashboard": {
                        "id": null,
                        "uid": "cpu-main",
                        "title": "CPU Main"
                    }
                }),
            }],
            last_reload_millis: 123,
            last_error: Some("failed to reload".to_string()),
        }));

        let document: serde_json::Value =
            serde_json::from_str(&serve_document(&state).unwrap()).expect("serve document json");
        assert_eq!(document["item_count"], 1);
        assert_eq!(document["last_reload_millis"], 123);
        assert_eq!(document["last_error"], "failed to reload");
        assert_eq!(document["items"][0]["title"], "CPU Main");
    }
}
