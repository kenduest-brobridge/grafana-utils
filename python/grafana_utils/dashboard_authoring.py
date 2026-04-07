"""Dashboard live authoring, preview, and history helpers for the Python CLI."""

from __future__ import annotations

import copy
import json
import os
import shlex
import sys
import tempfile
import threading
import time
import subprocess
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Optional

from .clients.dashboard_client import GrafanaClient
from .dashboards.common import DEFAULT_DASHBOARD_TITLE, DEFAULT_FOLDER_UID, GrafanaError
from .dashboards.import_support import (
    build_import_payload,
    determine_dashboard_import_action,
    extract_dashboard_object,
    load_json_file,
)
from .dashboards.output_support import write_json_document
from .dashboards.transformer import build_preserved_web_import_document, collect_datasource_refs
from . import yaml_compat as yaml

HISTORY_EXPORT_KIND = "grafana-utils-dashboard-history-export"
HISTORY_LIST_KIND = "grafana-utils-dashboard-history-list"
HISTORY_RESTORE_KIND = "grafana-utils-dashboard-history-restore"
HISTORY_INVENTORY_KIND = "grafana-utils-dashboard-history-inventory"
DEFAULT_HISTORY_RESTORE_MESSAGE = "Restored by grafana-util dashboard history"


def _normalize_text(value: Any, default: str = "") -> str:
    text = str(value or "").strip()
    return text or default


def _load_dashboard_document_from_path(path: Path) -> dict[str, Any]:
    if str(path) == "-":
        try:
            raw = json.loads(sys.stdin.read())
        except json.JSONDecodeError as exc:
            raise GrafanaError(f"Invalid JSON in standard input: {exc}") from exc
        if not isinstance(raw, dict):
            raise GrafanaError("Dashboard file must contain a JSON object: -")
        return raw
    return load_json_file(path)


def load_dashboard_document(path: Path | str) -> dict[str, Any]:
    """Load one dashboard JSON document from disk or standard input."""

    return _load_dashboard_document_from_path(Path(path))


def _dashboard_object(document: dict[str, Any]) -> dict[str, Any]:
    return extract_dashboard_object(
        document,
        "Dashboard file must contain a JSON object.",
    )


def fetch_live_dashboard(client: GrafanaClient, dashboard_uid: str) -> dict[str, Any]:
    """Fetch one live dashboard and strip the Grafana wrapper."""

    payload = client.fetch_dashboard(dashboard_uid)
    return build_preserved_web_import_document(payload)


def clone_live_dashboard(
    client: GrafanaClient,
    source_uid: str,
    name: Optional[str] = None,
    uid: Optional[str] = None,
    folder_uid: Optional[str] = None,
) -> dict[str, Any]:
    """Fetch one live dashboard and apply simple local overrides."""

    document = fetch_live_dashboard(client, source_uid)
    if name is not None:
        document["title"] = name
    if uid is not None:
        document["uid"] = uid
    if folder_uid is not None:
        document["folderUid"] = folder_uid
    document["id"] = None
    return document


def build_live_dashboard_authoring_document(
    payload: dict[str, Any],
    title_override: Optional[str] = None,
    uid_override: Optional[str] = None,
    folder_uid_override: Optional[str] = None,
) -> dict[str, Any]:
    """Build one live dashboard draft that preserves Grafana wrapper metadata."""

    document = copy.deepcopy(payload)
    if isinstance(document.get("dashboard"), dict):
        dashboard = copy.deepcopy(document["dashboard"])
    else:
        dashboard = copy.deepcopy(_dashboard_object(document))
    dashboard["id"] = None
    if title_override is not None:
        dashboard["title"] = title_override
    if uid_override is not None:
        dashboard["uid"] = uid_override
    if folder_uid_override is not None:
        meta = document.get("meta")
        if not isinstance(meta, dict):
            meta = {}
        meta["folderUid"] = folder_uid_override
        document["meta"] = meta
    document["dashboard"] = dashboard
    return document


def summarize_dashboard_change_lines(
    original: dict[str, Any],
    edited: dict[str, Any],
) -> list[str]:
    """Summarize the most visible live edit changes for dashboard drafts."""

    original_object = _dashboard_object(original)
    edited_object = _dashboard_object(edited)
    original_meta = original.get("meta") if isinstance(original, dict) else None
    edited_meta = edited.get("meta") if isinstance(edited, dict) else None
    original_folder_uid = _normalize_text(
        (original_meta or {}).get("folderUid") if isinstance(original_meta, dict) else None,
        "-",
    )
    edited_folder_uid = _normalize_text(
        (edited_meta or {}).get("folderUid") if isinstance(edited_meta, dict) else None,
        "-",
    )
    original_tags = ", ".join(
        [str(tag) for tag in (original_object.get("tags") or []) if str(tag).strip()]
    ) or "-"
    edited_tags = ", ".join(
        [str(tag) for tag in (edited_object.get("tags") or []) if str(tag).strip()]
    ) or "-"
    return [
        "Dashboard edit review uid=%s title=%s -> %s"
        % (
            _normalize_text(edited_object.get("uid")),
            _normalize_text(original_object.get("title"), DEFAULT_DASHBOARD_TITLE),
            _normalize_text(edited_object.get("title"), DEFAULT_DASHBOARD_TITLE),
        ),
        "Dashboard UID: %s -> %s"
        % (
            _normalize_text(original_object.get("uid")),
            _normalize_text(edited_object.get("uid")),
        ),
        "Folder UID: %s -> %s" % (original_folder_uid, edited_folder_uid),
        "Tags: %s -> %s" % (original_tags, edited_tags),
    ]


def build_dashboard_authoring_review(
    document: dict[str, Any],
    *,
    input_file: str = "edited draft",
    source_uid: Optional[str] = None,
) -> dict[str, Any]:
    """Build a review summary for one local dashboard authoring draft."""

    document_object = _dashboard_object(document)
    document_kind = "wrapped" if "dashboard" in document else "bare"
    title = _normalize_text(document_object.get("title"), DEFAULT_DASHBOARD_TITLE)
    uid = _normalize_text(document_object.get("uid"))
    folder_uid = _normalize_text(
        (document.get("meta") or {}).get("folderUid")
        if isinstance(document.get("meta"), dict)
        else None
    )
    tags = [
        str(tag)
        for tag in (document_object.get("tags") or [])
        if str(tag).strip()
    ]
    dashboard_id_is_null = document_object.get("id") is None
    meta_message_present = bool(
        isinstance(document.get("meta"), dict)
        and "message" in document.get("meta")
    )

    blocking_issues: list[str] = []
    try:
        build_import_payload(document, folder_uid_override=None, replace_existing=False, message="")
    except GrafanaError as exc:
        blocking_issues.append(str(exc))
    if document_object.get("id") is not None:
        blocking_issues.append("dashboard.id must stay null in the edited draft.")
    if source_uid is not None and uid != source_uid:
        blocking_issues.append(
            "edited draft changed dashboard.uid to %s" % (uid or "-")
        )

    if blocking_issues:
        suggested_next_action = "fix blocking issues, then publish --dry-run"
    elif source_uid is not None:
        suggested_next_action = "apply-live --yes"
    else:
        suggested_next_action = "publish --dry-run"

    return {
        "kind": "grafana-utils-dashboard-authoring-review",
        "inputFile": input_file,
        "documentKind": document_kind,
        "title": title,
        "uid": uid,
        "folderUid": folder_uid or None,
        "tags": tags,
        "dashboardIdIsNull": dashboard_id_is_null,
        "metaMessagePresent": meta_message_present,
        "blockingIssues": blocking_issues,
        "suggestedNextAction": suggested_next_action,
    }


def render_dashboard_authoring_review_text(review: dict[str, Any]) -> list[str]:
    """Render one dashboard authoring review as plain text lines."""

    lines = [
        "Dashboard authoring review",
        f"File: {review.get('inputFile') or '-'}",
        f"Kind: {review.get('documentKind') or '-'}",
        f"Title: {review.get('title') or '-'}",
        f"UID: {review.get('uid') or '-'}",
    ]
    folder_uid = review.get("folderUid")
    if folder_uid:
        lines.append(f"Folder UID: {folder_uid}")
    tags = review.get("tags") or []
    lines.append(f"Tags: {', '.join(tags) if tags else '-'}")
    lines.append(
        "dashboard.id: %s" % ("null" if review.get("dashboardIdIsNull") else "non-null")
    )
    lines.append(
        "meta.message: %s" % ("present" if review.get("metaMessagePresent") else "absent")
    )
    blocking_issues = review.get("blockingIssues") or []
    if blocking_issues:
        lines.append("Blocking issues:")
        lines.extend(f"- {issue}" for issue in blocking_issues)
    else:
        lines.append("Blocking issues: none")
    lines.append(f"Next action: {review.get('suggestedNextAction') or '-'}")
    return lines


def render_dashboard_authoring_review_document(review: dict[str, Any]) -> dict[str, Any]:
    """Render one dashboard authoring review as a serializable document."""

    return {
        "kind": review.get("kind"),
        "summary": dict(review),
        "blockingIssues": list(review.get("blockingIssues") or []),
    }


def edit_payload_in_external_editor(
    uid: str,
    value: dict[str, Any],
) -> Optional[dict[str, Any]]:
    """Open one dashboard draft in an external editor and return edited JSON."""

    with tempfile.NamedTemporaryFile(
        prefix=f"grafana-util-dashboard-edit-{uid}-",
        suffix=".json",
        delete=False,
        mode="w",
        encoding="utf-8",
    ) as handle:
        temp_path = Path(handle.name)
        handle.write(json.dumps(value, indent=2, ensure_ascii=False) + "\n")

    try:
        editor = (
            os.environ.get("VISUAL")
            or os.environ.get("EDITOR")
            or "vi"
        ).strip()
        command = shlex.split(editor)
        if not command:
            raise GrafanaError("Could not resolve an external editor command.")
        status = subprocess.run([*command, str(temp_path)], check=False)
        if status.returncode != 0:
            raise GrafanaError(
                f"External editor exited with status {status.returncode}."
            )
        edited = json.loads(temp_path.read_text(encoding="utf-8"))
        if not isinstance(edited, dict):
            raise GrafanaError("Edited dashboard JSON must be a JSON object.")
        if edited == value:
            return None
        return edited
    finally:
        try:
            temp_path.unlink()
        except OSError:
            pass


def run_dashboard_edit_live(
    client: GrafanaClient,
    args: Any,
) -> int:
    """Edit one live dashboard through an external editor and optional live apply."""

    if bool(getattr(args, "apply_live", False)) and not bool(getattr(args, "yes", False)):
        raise GrafanaError(
            "--apply-live requires --yes because it writes the edited dashboard back to Grafana."
        )

    live_payload = client.fetch_dashboard(getattr(args, "dashboard_uid"))
    wrapped = build_live_dashboard_authoring_document(live_payload)
    edited = edit_payload_in_external_editor(getattr(args, "dashboard_uid"), wrapped)
    if edited is None:
        print(
            f"No dashboard changes detected for {getattr(args, 'dashboard_uid')}. Nothing written."
        )
        return 0

    for line in summarize_dashboard_change_lines(wrapped, edited):
        print(line)

    review = build_dashboard_authoring_review(
        edited,
        input_file=f"edited draft for {getattr(args, 'dashboard_uid')}",
        source_uid=getattr(args, "dashboard_uid"),
    )
    for line in render_dashboard_authoring_review_text(review):
        print(line)

    if bool(getattr(args, "apply_live", False)):
        if review["blockingIssues"]:
            raise GrafanaError(
                "Cannot apply live dashboard %s because review still has blocking issues: %s"
                % (
                    getattr(args, "dashboard_uid"),
                    " | ".join(review["blockingIssues"]),
                )
            )
        if not review["dashboardIdIsNull"]:
            raise GrafanaError(
                "Cannot apply live dashboard %s because dashboard.id must stay null in the edited draft."
                % getattr(args, "dashboard_uid")
            )
        if review["uid"] != getattr(args, "dashboard_uid"):
            raise GrafanaError(
                "Cannot apply live dashboard %s because the edited draft changed dashboard.uid to %s."
                % (getattr(args, "dashboard_uid"), review["uid"])
            )
        payload = build_import_payload(
            edited,
            folder_uid_override=None,
            replace_existing=True,
            message=getattr(args, "message", "") or "Imported by grafana-utils",
        )
        response = client.import_dashboard(payload)
        if not isinstance(response, dict):
            raise GrafanaError("Unexpected dashboard edit-live response from Grafana.")
        print(
            "Applied edited dashboard %s back to Grafana."
            % getattr(args, "dashboard_uid")
        )
        return 0

    output = getattr(args, "output", None)
    output_path = Path(output) if output else Path(
        f"{getattr(args, 'dashboard_uid')}.edited.json"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    write_json_document(edited, output_path)
    print(
        "Wrote edited dashboard draft for %s to %s."
        % (getattr(args, "dashboard_uid"), output_path)
    )
    return 0


def _serve_text_document(path: Path) -> dict[str, Any]:
    raw_text = path.read_text(encoding="utf-8")
    if path.suffix.lower() in {".yaml", ".yml"}:
        value = yaml.safe_load(raw_text)
    else:
        try:
            value = json.loads(raw_text)
        except json.JSONDecodeError:
            value = yaml.safe_load(raw_text)
    if not isinstance(value, dict):
        raise GrafanaError(
            f"Dashboard serve expects JSON/YAML objects: {path}"
        )
    return value


def _serve_item_from_document(document: dict[str, Any], source: str) -> dict[str, Any]:
    dashboard = _dashboard_object(document)
    return {
        "title": _normalize_text(dashboard.get("title"), DEFAULT_DASHBOARD_TITLE),
        "uid": _normalize_text(dashboard.get("uid"), source),
        "source": source,
        "documentKind": "wrapped" if "dashboard" in document else "bare",
        "dashboard": copy.deepcopy(document),
    }


def load_dashboard_serve_items(args: Any) -> list[dict[str, Any]]:
    """Load dashboard documents for a local preview server."""

    script = getattr(args, "script", None)
    if script:
        output = subprocess.run(
            ["/bin/sh", "-lc", script],
            capture_output=True,
            check=False,
            text=True,
        )
        if output.returncode != 0:
            raise GrafanaError(
                f"Dashboard serve script exited with status {output.returncode}."
            )
        try:
            payload = (
                yaml.safe_load(output.stdout)
                if getattr(args, "script_format", "json") == "yaml"
                else json.loads(output.stdout)
            )
        except json.JSONDecodeError as exc:
            raise GrafanaError(f"Failed to parse dashboard serve JSON script output: {exc}") from exc
        if isinstance(payload, list):
            items = payload
        else:
            items = [payload]
        result = []
        for index, item in enumerate(items):
            if not isinstance(item, dict):
                raise GrafanaError("Dashboard serve script output must contain JSON/YAML objects.")
            result.append(_serve_item_from_document(item, f"script:{index}"))
        return result

    input_path = getattr(args, "input", None)
    if input_path is None:
        raise GrafanaError("dashboard serve requires --input or --script.")
    root = Path(input_path)
    if not root.exists():
        raise GrafanaError(f"Input path does not exist: {root}")

    files: list[Path] = []
    if root.is_file():
        files = [root]
    else:
        for candidate in sorted(root.rglob("*")):
            if not candidate.is_file():
                continue
            if candidate.suffix.lower() not in {".json", ".yaml", ".yml"}:
                continue
            files.append(candidate)
    if not files:
        raise GrafanaError(f"No dashboard files found under {root}.")

    items = []
    for path in files:
        items.append(_serve_item_from_document(_serve_text_document(path), str(path)))
    return items


def build_dashboard_serve_document(
    items: list[dict[str, Any]],
    last_error: Optional[str] = None,
) -> dict[str, Any]:
    """Build one preview-server snapshot document."""

    return {
        "itemCount": len(items),
        "items": items,
        "lastReloadMillis": int(time.time() * 1000),
        "lastError": last_error,
    }


def _serve_fingerprint(path: Path) -> Optional[tuple[int, int]]:
    try:
        stat = path.stat()
    except OSError:
        return None
    return (int(stat.st_mtime_ns), int(stat.st_size))


def _serve_watch_paths(args: Any) -> list[Path]:
    paths: list[Path] = []
    input_path = getattr(args, "input", None)
    if input_path is not None:
        paths.append(Path(input_path))
    for watch_path in getattr(args, "watch", []) or []:
        path = Path(watch_path)
        if path not in paths:
            paths.append(path)
    return paths


def _serve_html() -> str:
    return """<!doctype html>
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
      const list = window.document.getElementById('list');
      const meta = window.document.getElementById('meta');
      const error = window.document.getElementById('error');
      const payload = window.document.getElementById('payload');
      const status = window.document.getElementById('status');
      if (selectedIndex >= payloadDocument.itemCount) selectedIndex = 0;
      list.innerHTML = '';
      payloadDocument.items.forEach((item, index) => {
        const button = window.document.createElement('button');
        button.textContent = item.title + ' [' + item.uid + ']';
        if (index === selectedIndex) button.classList.add('active');
        button.onclick = () => { selectedIndex = index; refresh(); };
        list.appendChild(button);
      });
      status.textContent = 'items=' + payloadDocument.itemCount + ' reload=' + new Date(payloadDocument.lastReloadMillis).toLocaleTimeString();
      if (payloadDocument.lastError) {
        error.hidden = false;
        error.textContent = 'Last reload error: ' + payloadDocument.lastError;
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
      meta.textContent = 'source=' + current.source + ' kind=' + current.documentKind;
      payload.textContent = JSON.stringify(current.dashboard, null, 2);
    }
    refresh();
    setInterval(refresh, 2000);
  </script>
</body>
</html>"""


def _serve_reload_items(state: dict[str, Any], items: list[dict[str, Any]], last_error: Optional[str]) -> None:
    state["itemCount"] = len(items)
    state["items"] = items
    state["lastReloadMillis"] = int(time.time() * 1000)
    state["lastError"] = last_error


def _serve_watch_reload_loop(args: Any, state: dict[str, Any], lock: threading.Lock, watch_paths: list[Path]) -> None:
    previous = {path: _serve_fingerprint(path) for path in watch_paths}
    while True:
        time.sleep(1)
        changed = False
        for path in watch_paths:
            current = _serve_fingerprint(path)
            if previous.get(path) != current:
                previous[path] = current
                changed = True
        if not changed:
            continue
        try:
            items = load_dashboard_serve_items(args)
        except Exception as exc:  # pragma: no cover - background watcher fallback.
            with lock:
                _serve_reload_items(state, state["items"], str(exc))
            continue
        with lock:
            _serve_reload_items(state, items, None)


def run_dashboard_serve(args: Any) -> int:
    """Run one local dashboard preview server."""

    items = load_dashboard_serve_items(args)
    state: dict[str, Any] = build_dashboard_serve_document(items)
    state_lock = threading.Lock()
    watch_paths = _serve_watch_paths(args)
    if not bool(getattr(args, "no_watch", False)) and watch_paths:
        thread = threading.Thread(
            target=_serve_watch_reload_loop,
            args=(args, state, state_lock, watch_paths),
            daemon=True,
        )
        thread.start()

    class _Handler(BaseHTTPRequestHandler):
        def log_message(self, format: str, *values: Any) -> None:  # pragma: no cover - noise suppression.
            return

        def do_GET(self) -> None:  # noqa: N802
            if self.path == "/" or self.path.startswith("/?"):
                body = _serve_html().encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
                return
            if self.path == "/index.json":
                with state_lock:
                    payload = json.dumps(state, indent=2, ensure_ascii=False).encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
                return
            body = b"not found"
            self.send_response(404)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    server = ThreadingHTTPServer((getattr(args, "address"), int(getattr(args, "port"))), _Handler)
    server.daemon_threads = True
    preview_url = "http://%s:%s" % server.server_address[:2]
    print(f"Dashboard preview available at {preview_url}")
    if bool(getattr(args, "open_browser", False)):
        if not webbrowser.open(preview_url):  # pragma: no cover - browser availability varies.
            print("Dashboard serve browser launch failed: could not open the default browser.")
    try:
        server.serve_forever(poll_interval=0.2)
    except KeyboardInterrupt:  # pragma: no cover - manual termination path.
        pass
    finally:
        server.shutdown()
        server.server_close()
    return 0


def patch_dashboard_document(
    document: dict[str, Any],
    name: Optional[str] = None,
    uid: Optional[str] = None,
    folder_uid: Optional[str] = None,
    message: Optional[str] = None,
    tags: Optional[list[str]] = None,
) -> dict[str, Any]:
    """Patch one dashboard JSON document in place and return it."""

    patched = copy.deepcopy(document)
    dashboard = _dashboard_object(patched)
    if name is not None:
        dashboard["title"] = name
    if uid is not None:
        dashboard["uid"] = uid
    if folder_uid is not None:
        dashboard["folderUid"] = folder_uid
    if tags is not None:
        dashboard["tags"] = list(tags)
    dashboard["id"] = None
    if message is not None:
        meta = patched.get("meta")
        if not isinstance(meta, dict):
            meta = {}
            patched["meta"] = meta
        meta["message"] = message
    if dashboard is not patched:
        patched["dashboard"] = dashboard
    return patched


def build_dashboard_review_document(document: dict[str, Any]) -> dict[str, Any]:
    """Build one lightweight review summary for a local dashboard draft."""

    dashboard = _dashboard_object(document)
    refs: list[Any] = []
    collect_datasource_refs(dashboard, refs)
    unique_refs = []
    seen = set()
    for ref in refs:
        key = json.dumps(ref, sort_keys=True, ensure_ascii=False, default=str)
        if key in seen:
            continue
        seen.add(key)
        unique_refs.append(ref)
    panels = dashboard.get("panels")
    panel_count = len(panels) if isinstance(panels, list) else 0
    folder_uid = _normalize_text(
        dashboard.get("folderUid")
        or (document.get("meta") or {}).get("folderUid")
        or DEFAULT_FOLDER_UID
    )
    return {
        "kind": "grafana-utils-dashboard-review",
        "dashboardUid": _normalize_text(dashboard.get("uid")),
        "title": _normalize_text(dashboard.get("title"), DEFAULT_DASHBOARD_TITLE),
        "folderUid": folder_uid,
        "tagCount": len(dashboard.get("tags") or []),
        "panelCount": panel_count,
        "datasourceRefCount": len(unique_refs),
        "hasInputs": "__inputs" in document or "__inputs" in dashboard,
        "hasRequires": "__requires" in document or "__requires" in dashboard,
    }


def build_dashboard_publish_payload(
    document: dict[str, Any],
    replace_existing: bool,
    message: str,
    folder_uid: Optional[str] = None,
) -> dict[str, Any]:
    """Normalize a local dashboard draft into an import payload."""

    payload = build_import_payload(
        document,
        folder_uid_override=folder_uid,
        replace_existing=replace_existing,
        message=message,
    )
    return payload


def preview_dashboard_publish(
    client: GrafanaClient,
    document: dict[str, Any],
    replace_existing: bool,
    message: str,
    folder_uid: Optional[str] = None,
) -> dict[str, Any]:
    """Predict the import action for one local dashboard draft."""

    payload = build_dashboard_publish_payload(
        document,
        replace_existing=replace_existing,
        message=message,
        folder_uid=folder_uid,
    )
    action = determine_dashboard_import_action(
        client,
        payload,
        replace_existing=replace_existing,
    )
    dashboard = payload["dashboard"]
    return {
        "kind": "grafana-utils-dashboard-publish-preview",
        "dashboardUid": _normalize_text(dashboard.get("uid")),
        "title": _normalize_text(dashboard.get("title"), DEFAULT_DASHBOARD_TITLE),
        "folderUid": _normalize_text(payload.get("folderUid") or folder_uid),
        "action": action,
        "message": message,
    }


def publish_dashboard_document(
    client: GrafanaClient,
    document: dict[str, Any],
    replace_existing: bool,
    message: str,
    folder_uid: Optional[str] = None,
) -> dict[str, Any]:
    """Send one reviewed local dashboard draft back to Grafana."""

    payload = build_dashboard_publish_payload(
        document,
        replace_existing=replace_existing,
        message=message,
        folder_uid=folder_uid,
    )
    response = client.import_dashboard(payload)
    if not isinstance(response, dict):
        raise GrafanaError("Unexpected dashboard publish response from Grafana.")
    return response


def list_dashboard_history_versions(
    client: GrafanaClient,
    dashboard_uid: str,
    limit: int,
) -> list[dict[str, Any]]:
    """Fetch the live version history list for one dashboard."""

    response = client.request_json(
        f"/api/dashboards/uid/{dashboard_uid}/versions",
        params={"limit": limit},
    )
    if isinstance(response, list):
        items = response
    elif isinstance(response, dict):
        items = response.get("versions") or []
    else:
        raise GrafanaError("Unexpected dashboard history versions payload from Grafana.")
    versions = []
    for item in items:
        if not isinstance(item, dict):
            continue
        versions.append(
            {
                "version": int(item.get("version") or 0),
                "created": _normalize_text(item.get("created"), "-"),
                "createdBy": _normalize_text(item.get("createdBy"), "-"),
                "message": _normalize_text(item.get("message")),
            }
        )
    return versions


def build_dashboard_history_list_document(
    client: GrafanaClient,
    dashboard_uid: str,
    limit: int,
) -> dict[str, Any]:
    """Build one dashboard history list document from live Grafana."""

    versions = list_dashboard_history_versions(client, dashboard_uid, limit)
    return {
        "kind": HISTORY_LIST_KIND,
        "dashboardUid": dashboard_uid,
        "versionCount": len(versions),
        "versions": versions,
    }


def build_dashboard_history_list_document_from_export(
    document: dict[str, Any],
) -> dict[str, Any]:
    """Build one dashboard history list document from a local export artifact."""

    versions = []
    for item in document.get("versions") or []:
        if not isinstance(item, dict):
            continue
        versions.append(
            {
                "version": int(item.get("version") or 0),
                "created": _normalize_text(item.get("created"), "-"),
                "createdBy": _normalize_text(item.get("createdBy"), "-"),
                "message": _normalize_text(item.get("message")),
            }
        )
    return {
        "kind": HISTORY_LIST_KIND,
        "dashboardUid": _normalize_text(document.get("dashboardUid")),
        "versionCount": len(versions),
        "versions": versions,
    }


def _fetch_dashboard_history_version_dashboard(
    client: GrafanaClient,
    dashboard_uid: str,
    version: int,
) -> dict[str, Any]:
    response = client.request_json(f"/api/dashboards/uid/{dashboard_uid}/versions/{version}")
    if not isinstance(response, dict):
        raise GrafanaError("Unexpected dashboard history version payload from Grafana.")
    data = response.get("data")
    if not isinstance(data, dict):
        raise GrafanaError("Dashboard history version payload did not include dashboard data.")
    return data


def build_dashboard_history_export_document(
    client: GrafanaClient,
    dashboard_uid: str,
    limit: int,
) -> dict[str, Any]:
    """Export one dashboard's revision history into a reusable JSON bundle."""

    current = client.fetch_dashboard(dashboard_uid)
    dashboard = _dashboard_object(current)
    versions = []
    for item in list_dashboard_history_versions(client, dashboard_uid, limit):
        version_number = int(item.get("version") or 0)
        versions.append(
            {
                "version": version_number,
                "created": item.get("created") or "-",
                "createdBy": item.get("createdBy") or "-",
                "message": item.get("message") or "",
                "dashboard": _fetch_dashboard_history_version_dashboard(
                    client, dashboard_uid, version_number
                ),
            }
        )
    return {
        "kind": HISTORY_EXPORT_KIND,
        "dashboardUid": dashboard_uid,
        "currentVersion": int(dashboard.get("version") or 0),
        "currentTitle": _normalize_text(
            dashboard.get("title"), DEFAULT_DASHBOARD_TITLE
        ),
        "versionCount": len(versions),
        "versions": versions,
    }


def load_history_export_document(path: Path | str) -> dict[str, Any]:
    """Load one local dashboard history export artifact from disk."""

    document = load_json_file(Path(path))
    if _normalize_text(document.get("kind")) != HISTORY_EXPORT_KIND:
        raise GrafanaError(
            f"Expected {HISTORY_EXPORT_KIND} at {path}, found {document.get('kind')}."
        )
    return document


def build_history_inventory_document(
    input_dir: Path,
    artifacts: list[tuple[Path, dict[str, Any]]],
) -> dict[str, Any]:
    """Build one inventory document for a local history export tree."""

    items = []
    for path, document in artifacts:
        items.append(
            {
                "dashboardUid": _normalize_text(document.get("dashboardUid")),
                "currentTitle": _normalize_text(document.get("currentTitle")),
                "currentVersion": int(document.get("currentVersion") or 0),
                "versionCount": int(document.get("versionCount") or 0),
                "scope": _derive_history_scope(input_dir, path),
                "path": str(path),
            }
        )
    return {
        "kind": HISTORY_INVENTORY_KIND,
        "artifactCount": len(items),
        "artifacts": items,
    }


def _derive_history_scope(input_dir: Path, artifact_path: Path) -> Optional[str]:
    try:
        relative = artifact_path.relative_to(input_dir)
    except ValueError:
        return None
    parts = list(relative.parts)
    if "history" in parts:
        index = parts.index("history")
        prefix = parts[:index]
        if prefix:
            return "/".join(prefix)
    if len(parts) > 1:
        return parts[0]
    return None


def load_history_artifacts(input_dir: Path) -> list[tuple[Path, dict[str, Any]]]:
    """Load every local dashboard history artifact beneath one input directory."""

    if not input_dir.exists():
        raise GrafanaError(f"Input directory does not exist: {input_dir}")
    if not input_dir.is_dir():
        raise GrafanaError(f"Input path is not a directory: {input_dir}")
    artifacts: list[tuple[Path, dict[str, Any]]] = []
    for path in sorted(input_dir.rglob("*.history.json")):
        artifacts.append((path, load_history_export_document(path)))
    return artifacts


def restore_dashboard_history_version(
    client: GrafanaClient,
    dashboard_uid: str,
    version: int,
    message: Optional[str] = None,
) -> dict[str, Any]:
    """Restore one historical version as a new latest dashboard revision."""

    current = client.fetch_dashboard(dashboard_uid)
    current_dashboard = _dashboard_object(current)
    restored_dashboard = _fetch_dashboard_history_version_dashboard(
        client, dashboard_uid, version
    )
    restored_dashboard["id"] = current_dashboard.get("id")
    restored_dashboard["uid"] = dashboard_uid
    restored_dashboard["version"] = current_dashboard.get("version", 0)
    payload: dict[str, Any] = {
        "dashboard": restored_dashboard,
        "overwrite": True,
        "message": message
        or f"{DEFAULT_HISTORY_RESTORE_MESSAGE} to version {version}",
    }
    current_meta = current.get("meta") if isinstance(current, dict) else None
    if isinstance(current_meta, dict):
        folder_uid = _normalize_text(current_meta.get("folderUid"))
        if folder_uid:
            payload["folderUid"] = folder_uid
    response = client.import_dashboard(payload)
    if not isinstance(response, dict):
        raise GrafanaError("Unexpected dashboard history restore response from Grafana.")
    return {
        "kind": HISTORY_RESTORE_KIND,
        "dashboardUid": dashboard_uid,
        "currentVersion": int(current_dashboard.get("version") or 0),
        "restoreVersion": version,
        "message": payload["message"],
        "response": response,
    }


def preview_dashboard_history_restore(
    client: GrafanaClient,
    dashboard_uid: str,
    version: int,
    message: Optional[str] = None,
) -> dict[str, Any]:
    """Preview one dashboard history restore without mutating Grafana."""

    current = client.fetch_dashboard(dashboard_uid)
    current_dashboard = _dashboard_object(current)
    restored_dashboard = _fetch_dashboard_history_version_dashboard(
        client, dashboard_uid, version
    )
    current_meta = current.get("meta") if isinstance(current, dict) else None
    target_folder_uid = None
    if isinstance(current_meta, dict):
        target_folder_uid = _normalize_text(current_meta.get("folderUid")) or None
    return {
        "kind": HISTORY_RESTORE_KIND,
        "dashboardUid": dashboard_uid,
        "currentVersion": int(current_dashboard.get("version") or 0),
        "restoreVersion": version,
        "currentTitle": _normalize_text(
            current_dashboard.get("title"), DEFAULT_DASHBOARD_TITLE
        ),
        "restoredTitle": _normalize_text(
            restored_dashboard.get("title"), DEFAULT_DASHBOARD_TITLE
        ),
        "targetFolderUid": target_folder_uid,
        "createsNewRevision": True,
        "message": message or f"{DEFAULT_HISTORY_RESTORE_MESSAGE} to version {version}",
        "mode": "dry-run",
    }


def validate_dashboard_export_tree(import_dir: Path) -> dict[str, Any]:
    """Validate one dashboard export tree without mutating Grafana."""
    if not import_dir.exists():
        raise GrafanaError(f"Input directory does not exist: {import_dir}")
    if not import_dir.is_dir():
        raise GrafanaError(f"Input path is not a directory: {import_dir}")
    excluded_names = {
        "index.json",
        "export-metadata.json",
        "folders.json",
        "datasources.json",
        "permissions.json",
    }
    files = [
        path
        for path in sorted(import_dir.rglob("*.json"))
        if path.name not in excluded_names and path.name != ".inspect-source-root"
    ]
    parsed = []
    for path in files:
        parsed.append(load_json_file(path))
        build_import_payload(parsed[-1], folder_uid_override=None, replace_existing=False, message="")
    return {
        "kind": "grafana-utils-dashboard-export-validation",
        "inputDir": str(import_dir),
        "fileCount": len(files),
        "dashboardCount": len(parsed),
    }
