"""Dashboard screenshot helpers with browser-driven capture by default."""

import base64
import contextlib
import hashlib
import io
import json
import os
import random
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from types import SimpleNamespace
from urllib import request as urllib_request
from urllib import parse

from .common import GrafanaError
from .variable_inspection import resolve_dashboard_uid


def _requests_module():
    # Purpose: implementation note.
    # Args: see function signature.
    # Returns: see implementation.

    try:
        import requests

        return requests
    except ImportError as exc:
        raise GrafanaError(
            "The requests library is required for dashboard screenshot runtime. "
            "Install with `python3 -m pip install requests`."
        ) from exc


_PIL_MODULES = None


def _pil_modules():
    # Purpose: implementation note.
    # Args: see function signature.
    # Returns: see implementation.

    global _PIL_MODULES
    if _PIL_MODULES is None:
        try:
            from PIL import Image, ImageDraw, ImageFont

            _PIL_MODULES = (Image, ImageDraw, ImageFont)
        except ImportError as exc:
            raise GrafanaError(
                "The pillow library is required for screenshot image composition. "
                "Install with `python3 -m pip install pillow`."
            ) from exc
    return _PIL_MODULES


SUPPORTED_OUTPUT_FORMATS = ("png", "jpeg", "pdf")
SUPPORTED_FULL_PAGE_OUTPUTS = ("single", "tiles", "manifest")
DEFAULT_SCREENSHOT_THEME = "dark"
BROWSER_CANDIDATES = (
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
    "google-chrome",
    "chromium",
    "chromium-browser",
    "chrome",
)


def parse_var_assignment(raw_value):
    """Parse one NAME=VALUE assignment used by screenshot URL builders."""
    text = str(raw_value or "").strip()
    if not text or "=" not in text:
        raise GrafanaError(
            "Invalid --var value %r. Use NAME=VALUE." % raw_value
        )
    name, value = text.split("=", 1)
    name = name.strip()
    value = value.strip()
    if not name:
        raise GrafanaError(
            "Invalid --var value %r. Use NAME=VALUE." % raw_value
        )
    if not value:
        raise GrafanaError(
            "Invalid --var value %r. VALUE cannot be empty." % raw_value
        )
    return name, value


def parse_vars_query(raw_query):
    """Parse a query fragment into ordered passthrough and var assignments."""
    text = str(raw_query or "").strip()
    if not text:
        return {"vars": [], "passthrough_pairs": []}
    query_text = text[1:] if text.startswith("?") else text
    pairs = parse.parse_qsl(query_text, keep_blank_values=True)
    vars_pairs = []
    passthrough_pairs = []
    for key, value in pairs:
        if key.startswith("var-"):
            var_name = key[4:].strip()
            if not var_name:
                raise GrafanaError(
                    "Invalid vars query fragment %r. Variable names cannot be empty."
                    % raw_query
                )
            if not value:
                raise GrafanaError(
                    "Invalid vars query fragment %r. Variable values cannot be empty."
                    % raw_query
                )
            vars_pairs.append((var_name, value))
            continue
        passthrough_pairs.append((key, value))
    return {"vars": vars_pairs, "passthrough_pairs": passthrough_pairs}


def infer_screenshot_output_format(output_path, explicit_format=None):
    """Infer screenshot output format from an explicit value or output extension."""
    if explicit_format:
        normalized = str(explicit_format).strip().lower()
        if normalized == "jpg":
            normalized = "jpeg"
        if normalized not in SUPPORTED_OUTPUT_FORMATS:
            raise GrafanaError(
                "Unsupported screenshot output format %r. Use png, jpeg, or pdf."
                % explicit_format
            )
        return normalized

    suffix = Path(output_path).suffix.lower().lstrip(".")
    if suffix == "jpg":
        suffix = "jpeg"
    if suffix in SUPPORTED_OUTPUT_FORMATS:
        return suffix
    raise GrafanaError(
        "Unable to infer screenshot output format from --output. "
        "Use a .png, .jpg, .jpeg, or .pdf filename or pass --output-format."
    )


def validate_screenshot_args(args):
    """Validate screenshot argument shape before backend execution."""
    # Call graph: see callers/callees.
    #   Upstream callers: 221, 317
    #   Downstream callees: 46, 67, 95

    dashboard_uid = str(getattr(args, "dashboard_uid", "") or "").strip()
    dashboard_url = str(getattr(args, "dashboard_url", "") or "").strip()
    if not dashboard_uid and not dashboard_url:
        raise GrafanaError(
            "Set --dashboard-uid or pass --dashboard-url so the screenshot command "
            "knows which dashboard to open."
        )
    width = int(getattr(args, "width", 0) or 0)
    height = int(getattr(args, "height", 0) or 0)
    device_scale_factor = float(getattr(args, "device_scale_factor", 1.0) or 0.0)
    if width <= 0:
        raise GrafanaError("--width must be greater than 0.")
    if height <= 0:
        raise GrafanaError("--height must be greater than 0.")
    if device_scale_factor <= 0:
        raise GrafanaError("--device-scale-factor must be greater than 0.")
    for assignment in list(getattr(args, "vars", None) or []):
        parse_var_assignment(assignment)
    vars_query = getattr(args, "vars_query", None)
    if vars_query:
        parse_vars_query(vars_query)
    infer_screenshot_output_format(
        getattr(args, "output", None),
        getattr(args, "output_format", None),
    )
    full_page_output = str(getattr(args, "full_page_output", "single") or "single").strip().lower()
    if full_page_output not in SUPPORTED_FULL_PAGE_OUTPUTS:
        raise GrafanaError(
            "Unsupported --full-page-output value %r. Use single, tiles, or manifest."
            % getattr(args, "full_page_output", None)
        )
    output_format = infer_screenshot_output_format(
        getattr(args, "output", None),
        getattr(args, "output_format", None),
    )
    if full_page_output != "single" and not bool(getattr(args, "full_page", False)):
        raise GrafanaError("--full-page-output tiles or manifest requires --full-page.")
    if full_page_output != "single" and output_format == "pdf":
        raise GrafanaError("PDF output does not support --full-page-output tiles or manifest.")
    return args


def _normalize_dashboard_target_state(args):
    """Internal helper for normalize dashboard target state."""
    raw_dashboard_url = str(getattr(args, "dashboard_url", "") or "").strip()
    url = None
    if raw_dashboard_url:
        try:
            url = parse.urlsplit(raw_dashboard_url)
        except ValueError as exc:
            raise GrafanaError("Invalid --dashboard-url: %s" % exc) from exc
        if not url.scheme or not url.netloc:
            raise GrafanaError("Invalid --dashboard-url: expected absolute URL.")
    else:
        base_url = str(getattr(args, "url", "") or "").strip().rstrip("/")
        if not base_url:
            raise GrafanaError(
                "Set --url or pass --dashboard-url so the screenshot command can build a Grafana URL."
            )
        try:
            url = parse.urlsplit(base_url)
        except ValueError as exc:
            raise GrafanaError("Invalid Grafana base URL: %s" % exc) from exc
        if not url.scheme or not url.netloc:
            raise GrafanaError("Invalid Grafana base URL: expected absolute URL.")
    segments = [segment for segment in (url.path or "").split("/") if segment]
    path_dashboard_uid = None
    path_slug = None
    if len(segments) >= 3 and segments[0] in ("d", "d-solo"):
        path_dashboard_uid = segments[1]
        path_slug = segments[2]
    query_pairs = parse.parse_qsl(url.query, keep_blank_values=True)
    state = {
        "scheme": url.scheme,
        "netloc": url.netloc,
        "dashboard_uid": path_dashboard_uid,
        "slug": path_slug,
        "panel_id": None,
        "org_id": None,
        "from_value": None,
        "to_value": None,
        "vars": [],
        "passthrough_pairs": [],
    }
    for key, value in query_pairs:
        if key == "panelId":
            state["panel_id"] = value
        elif key == "orgId":
            state["org_id"] = value
        elif key == "from":
            state["from_value"] = value
        elif key == "to":
            state["to_value"] = value
        elif key.startswith("var-"):
            state["vars"].append((key[4:], value))
        elif key not in ("viewPanel", "kiosk", "theme"):
            state["passthrough_pairs"].append((key, value))
    return state


def build_dashboard_capture_url(args):
    """Build a browser-ready Grafana dashboard or panel URL."""
    # Call graph: see callers/callees.
    #   Upstream callers: 317, 558
    #   Downstream callees: 119, 163, 46, 67

    validate_screenshot_args(args)
    path_state = _normalize_dashboard_target_state(args)
    fragment_state = parse_vars_query(getattr(args, "vars_query", None))
    dashboard_uid = str(getattr(args, "dashboard_uid", "") or "").strip()
    if not dashboard_uid:
        dashboard_uid = path_state["dashboard_uid"] or ""
    if not dashboard_uid:
        raise GrafanaError(
            "Unable to determine dashboard UID. Pass --dashboard-uid or a Grafana dashboard URL."
        )
    slug = str(getattr(args, "slug", "") or "").strip() or path_state["slug"] or dashboard_uid
    panel_id = getattr(args, "panel_id", None)
    if panel_id in (None, ""):
        panel_id = path_state["panel_id"]
    panel_id = str(panel_id).strip() if panel_id not in (None, "") else None
    org_id = getattr(args, "org_id", None)
    if org_id in (None, ""):
        for key, value in fragment_state["passthrough_pairs"]:
            if key == "orgId":
                org_id = value
                break
    if org_id in (None, ""):
        org_id = path_state["org_id"]
    org_id = str(org_id).strip() if org_id not in (None, "") else None
    from_value = str(getattr(args, "from_value", None) or getattr(args, "from_", None) or "").strip()
    if not from_value:
        for key, value in fragment_state["passthrough_pairs"]:
            if key == "from":
                from_value = value
                break
    if not from_value:
        from_value = path_state["from_value"] or ""
    to_value = str(getattr(args, "to_value", None) or "").strip()
    if not to_value:
        for key, value in fragment_state["passthrough_pairs"]:
            if key == "to":
                to_value = value
                break
    if not to_value:
        to_value = path_state["to_value"] or ""
    theme = str(getattr(args, "theme", DEFAULT_SCREENSHOT_THEME) or DEFAULT_SCREENSHOT_THEME).strip().lower()
    if theme not in ("dark", "light"):
        raise GrafanaError("Unsupported screenshot theme %r. Use dark or light." % theme)

    passthrough_pairs = list(path_state["passthrough_pairs"])
    for key, value in fragment_state["passthrough_pairs"]:
        if key in ("panelId", "orgId", "from", "to", "viewPanel", "kiosk", "theme"):
            continue
        passthrough_pairs = [
            pair for pair in passthrough_pairs if pair[0] != key
        ]
        passthrough_pairs.append((key, value))

    merged_vars = list(path_state["vars"])
    for key, value in fragment_state["vars"]:
        merged_vars = [pair for pair in merged_vars if pair[0] != key]
        merged_vars.append((key, value))
    for assignment in list(getattr(args, "vars", None) or []):
        key, value = parse_var_assignment(assignment)
        merged_vars = [pair for pair in merged_vars if pair[0] != key]
        merged_vars.append((key, value))

    path_prefix = "/d-solo/" if panel_id else "/d/"
    path = "%s%s/%s" % (
        path_prefix,
        parse.quote(dashboard_uid, safe=""),
        parse.quote(slug, safe=""),
    )
    query_pairs = list(passthrough_pairs)
    if panel_id:
        query_pairs.append(("panelId", panel_id))
        query_pairs.append(("viewPanel", panel_id))
    if org_id:
        query_pairs.append(("orgId", org_id))
    if from_value:
        query_pairs.append(("from", from_value))
    if to_value:
        query_pairs.append(("to", to_value))
    query_pairs.append(("theme", theme))
    query_pairs.append(("kiosk", "tv"))
    for key, value in merged_vars:
        query_pairs.append(("var-%s" % key, value))
    query_text = parse.urlencode(query_pairs, doseq=True)
    return parse.urlunsplit(
        (
            path_state["scheme"],
            path_state["netloc"],
            path,
            query_text,
            "",
        )
    )


def build_capture_request(args):
    """Normalize screenshot args into one backend-friendly capture request."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1644
    #   Downstream callees: 119, 221, 558, 95

    validate_screenshot_args(args)
    output_path = Path(getattr(args, "output"))
    return {
        "url": build_dashboard_capture_url(args),
        "render_url": build_render_url(args),
        "output": output_path,
        "output_format": infer_screenshot_output_format(
            output_path,
            getattr(args, "output_format", None),
        ),
        "width": int(getattr(args, "width")),
        "height": int(getattr(args, "height")),
        "device_scale_factor": float(getattr(args, "device_scale_factor", 1.0) or 1.0),
        "full_page": bool(getattr(args, "full_page", False)),
        "full_page_output": str(getattr(args, "full_page_output", "single") or "single").strip().lower(),
        "wait_ms": int(getattr(args, "wait_ms", 0) or 0),
        "theme": str(getattr(args, "theme", DEFAULT_SCREENSHOT_THEME) or DEFAULT_SCREENSHOT_THEME),
        "print_capture_url": bool(getattr(args, "print_capture_url", False)),
        "header_title": getattr(args, "header_title", None),
        "header_url": getattr(args, "header_url", None),
        "header_captured_at": bool(getattr(args, "header_captured_at", False)),
        "header_text": getattr(args, "header_text", None),
    }


def _resolve_capture_metadata(args, client):
    """Internal helper for resolve capture metadata."""
    resolved_uid = resolve_dashboard_uid(
        dashboard_uid=getattr(args, "dashboard_uid", None),
        dashboard_url=getattr(args, "dashboard_url", None),
    )
    fallback_title = None
    dashboard_url = str(getattr(args, "dashboard_url", "") or "").strip()
    if dashboard_url:
        with contextlib.suppress(Exception):
            parsed_url = parse.urlsplit(dashboard_url)
            segments = [segment for segment in (parsed_url.path or "").split("/") if segment]
            if len(segments) >= 3 and segments[0] in ("d", "d-solo"):
                slug_text = str(segments[2] or "").strip()
                if slug_text:
                    fallback_title = slug_text
    try:
        payload = client.fetch_dashboard(resolved_uid)
    except Exception:
        if str(getattr(args, "dashboard_url", "") or "").strip():
            return {
                "payload": None,
                "dashboard_uid": resolved_uid,
                "dashboard_title": fallback_title,
                "panel_title": None,
            }
        raise
    if not isinstance(payload, dict):
        return {
            "payload": None,
            "dashboard_uid": resolved_uid,
            "dashboard_title": fallback_title,
            "panel_title": None,
        }
    meta = payload.get("meta")
    if isinstance(meta, dict):
        slug = str(meta.get("slug") or "").strip()
        if slug and not str(getattr(args, "slug", "") or "").strip():
            args.slug = slug
    dashboard = payload.get("dashboard")
    return {
        "payload": payload,
        "dashboard_uid": resolved_uid,
        "dashboard_title": str((dashboard or {}).get("title") or "").strip(),
        "panel_title": _find_panel_title(dashboard or {}, getattr(args, "panel_id", None)),
    }


def _find_panel_title(dashboard, panel_id):
    """Internal helper for find panel title."""
    if not isinstance(dashboard, dict) or panel_id in (None, ""):
        return None
    panel_id_text = str(panel_id).strip()
    if not panel_id_text:
        return None

    def visit(items):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        if not isinstance(items, list):
            return None
        for item in items:
            if not isinstance(item, dict):
                continue
            item_id = item.get("id")
            if item_id is not None and str(item_id) == panel_id_text:
                title = str(item.get("title") or "").strip()
                if title:
                    return title
            nested = visit(item.get("panels"))
            if nested:
                return nested
        return None

    return visit(dashboard.get("panels"))


def _resolve_auto_header_title(request, metadata):
    """Internal helper for resolve auto header title."""
    for candidate in (
        (metadata or {}).get("panel_title"),
        (metadata or {}).get("dashboard_title"),
        (metadata or {}).get("dashboard_uid"),
        request["output"].stem,
    ):
        text = str(candidate or "").strip()
        if text:
            return text
    return None


def _resolve_optional_header_field(raw_value, auto_value):
    """Internal helper for resolve optional header field."""
    text = str(raw_value or "").strip()
    if not text:
        return None
    if text == "__auto__":
        return str(auto_value or "").strip() or None
    return text


def _build_header_lines(request, metadata):
    """Internal helper for build header lines."""
    lines = []
    title_value = _resolve_optional_header_field(
        request.get("header_title"),
        _resolve_auto_header_title(request, metadata),
    )
    if title_value:
        lines.append(("title", title_value))
    url_value = _resolve_optional_header_field(
        request.get("header_url"),
        request.get("url"),
    )
    if url_value:
        lines.append(("meta", url_value))
    if request.get("header_captured_at"):
        lines.append(("meta", "Captured at %s" % time.strftime("%Y-%m-%d %H:%M:%S")))
    text_value = str(request.get("header_text") or "").strip()
    if text_value:
        lines.append(("body", text_value))
    return lines


def _wrap_header_text(text, font, max_width, draw):
    """Internal helper for wrap header text."""
    words = str(text or "").split()
    if not words:
        return [""]
    lines = []
    current = words[0]
    for word in words[1:]:
        trial = "%s %s" % (current, word)
        if draw.textbbox((0, 0), trial, font=font)[2] <= max_width:
            current = trial
            continue
        lines.append(current)
        current = word
    lines.append(current)
    return lines


def _compose_header_image(image, request, metadata):
    """Internal helper for compose header image."""
    Image, ImageDraw, ImageFont = _pil_modules()
    header_lines = _build_header_lines(request, metadata)
    if not header_lines:
        return image
    base = image.convert("RGBA")
    width = base.width
    margin_x = 24
    margin_y = 18
    line_gap = 8
    title_font = ImageFont.load_default()
    meta_font = ImageFont.load_default()
    body_font = ImageFont.load_default()
    measure = ImageDraw.Draw(Image.new("RGBA", (width, 200), (0, 0, 0, 0)))
    rendered_lines = []
    content_width = max(width - (margin_x * 2), 120)
    for line_type, text in header_lines:
        font = title_font if line_type == "title" else meta_font if line_type == "meta" else body_font
        wrapped = _wrap_header_text(text, font, content_width, measure)
        for item in wrapped:
            box = measure.textbbox((0, 0), item, font=font)
            rendered_lines.append((line_type, item, font, box[3] - box[1]))
    header_height = (margin_y * 2) + sum(item[3] for item in rendered_lines) + (line_gap * max(len(rendered_lines) - 1, 0))
    header_image = Image.new("RGBA", (width, header_height + base.height), (17, 18, 23, 255))
    draw = ImageDraw.Draw(header_image)
    palette = {
        "title": (240, 244, 252, 255),
        "meta": (154, 169, 191, 255),
        "body": (210, 218, 230, 255),
    }
    current_y = margin_y
    for line_type, text, font, line_height in rendered_lines:
        draw.text((margin_x, current_y), text, font=font, fill=palette[line_type])
        current_y += line_height + line_gap
    header_image.alpha_composite(base, (0, header_height))
    return header_image


def _write_raster_output(image, request, metadata):
    """Internal helper for write raster output."""
    rendered = _compose_header_image(image, request, metadata)
    if request["output_format"] == "jpeg":
        rendered.convert("RGB").save(str(request["output"]), format="JPEG", quality=90)
        return
    rendered.save(str(request["output"]), format="PNG")


def find_browser_executable(browser_path=None):
    """Resolve one local Chromium-compatible browser executable."""
    explicit = str(browser_path or "").strip()
    if explicit:
        candidate = Path(explicit)
        if candidate.is_file():
            return str(candidate)
        resolved = shutil.which(explicit)
        if resolved:
            return resolved
        raise GrafanaError("Browser executable not found: %s" % explicit)
    for candidate in BROWSER_CANDIDATES:
        path = Path(candidate)
        if path.is_file():
            return str(path)
        resolved = shutil.which(candidate)
        if resolved:
            return resolved
    raise GrafanaError(
        "No Chromium-compatible browser executable was found. Set --browser-path to a local Chrome/Chromium binary."
    )


def build_render_url(args):
    """Build a Grafana server-side render URL from the browser capture URL."""
    capture_url = build_dashboard_capture_url(args)
    parsed = parse.urlsplit(capture_url)
    render_path = parsed.path
    if render_path.startswith("/d-solo/"):
        render_path = "/render%s" % render_path
    elif render_path.startswith("/d/"):
        render_path = "/render%s" % render_path
    else:
        raise GrafanaError(
            "Unable to build render URL from dashboard path %r." % parsed.path
        )
    query_pairs = parse.parse_qsl(parsed.query, keep_blank_values=True)
    query_pairs = [pair for pair in query_pairs if pair[0] not in ("width", "height")]
    query_pairs.append(("width", str(int(getattr(args, "width")))))
    query_pairs.append(("height", str(int(getattr(args, "height")))))
    if bool(getattr(args, "full_page", False)):
        query_pairs.append(("autofitpanels", "1"))
    query_text = parse.urlencode(query_pairs, doseq=True)
    return parse.urlunsplit(
        (parsed.scheme, parsed.netloc, render_path, query_text, "")
    )


def _capture_with_grafana_render(request, client, http_get=None):
    """Capture PNG output through Grafana's /render endpoint."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    requests = _requests_module()
    if http_get is None:
        http_get = requests.get
    output_format = request["output_format"]
    if output_format != "png":
        raise GrafanaError(
            "Python dashboard screenshot currently supports live capture only for PNG output through Grafana's /render endpoint. Use a .png output path."
        )
    response = http_get(
        request["render_url"],
        headers=dict(getattr(client, "headers", {}) or {}),
        timeout=getattr(client, "timeout", 30),
        verify=bool(getattr(client, "verify_ssl", False)),
    )
    try:
        response.raise_for_status()
    except requests.RequestException as exc:
        raise GrafanaError("Grafana render request failed: %s" % exc) from exc
    content_type = str(response.headers.get("Content-Type") or "").lower()
    if "image/png" not in content_type:
        raise GrafanaError(
            "Grafana render endpoint returned unexpected content type %r."
            % response.headers.get("Content-Type")
        )
    request["output"].write_bytes(response.content)
    return {
        "output": request["output"],
        "output_format": output_format,
        "render_url": request["render_url"],
        "size": len(response.content),
    }


def _strip_hop_by_hop_headers(headers):
    """Internal helper for strip hop by hop headers."""
    blocked = {
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
        "host",
        "content-length",
    }
    return {
        key: value
        for key, value in headers.items()
        if str(key).lower() not in blocked
    }


def _rewrite_location(location, remote_base_url, proxy_base_url):
    """Internal helper for rewrite location."""
    if not location:
        return location
    if location.startswith(remote_base_url):
        return proxy_base_url + location[len(remote_base_url):]
    return location


@contextlib.contextmanager
def run_auth_proxy(client):
    """Start one local proxy that forwards Grafana requests with auth headers."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    requests = _requests_module()
    remote_base_url = str(getattr(client, "base_url", "") or "").rstrip("/")
    remote_parts = parse.urlsplit(remote_base_url)
    if not remote_parts.scheme or not remote_parts.netloc:
        raise GrafanaError("Invalid Grafana base URL for screenshot proxy.")

    class ProxyHandler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, format, *args):
            # Purpose: implementation note.
            # Args: see function signature.
            # Returns: see implementation.

            return None

        def do_GET(self):
            # Purpose: implementation note.
            # Args: see function signature.
            # Returns: see implementation.

            self._forward()

        def do_POST(self):
            # Purpose: implementation note.
            # Args: see function signature.
            # Returns: see implementation.

            self._forward()

        def _forward(self):
            # Purpose: implementation note.
            # Args: see function signature.
            # Returns: see implementation.

            path = self.path or "/"
            target_url = remote_base_url + path
            content_length = int(self.headers.get("Content-Length", "0") or "0")
            body = self.rfile.read(content_length) if content_length else None
            headers = _strip_hop_by_hop_headers(dict(self.headers))
            headers.update(dict(getattr(client, "headers", {}) or {}))
            try:
                response = requests.request(
                    self.command,
                    target_url,
                    headers=headers,
                    data=body,
                    timeout=getattr(client, "timeout", 30),
                    verify=bool(getattr(client, "verify_ssl", False)),
                    allow_redirects=False,
                )
            except requests.RequestException as exc:
                self.send_error(502, "Proxy request failed: %s" % exc)
                return

            content = response.content
            self.send_response(response.status_code)
            proxy_base_url = "http://%s:%s" % self.server.server_address
            for key, value in response.headers.items():
                lower = key.lower()
                if lower in ("transfer-encoding", "content-length", "content-encoding", "connection"):
                    continue
                if lower == "location":
                    value = _rewrite_location(value, remote_base_url, proxy_base_url)
                self.send_header(key, value)
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            if content:
                self.wfile.write(content)

    server = ThreadingHTTPServer(("127.0.0.1", 0), ProxyHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield "http://127.0.0.1:%s" % server.server_port
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def _build_proxy_capture_url(proxy_base_url, capture_url):
    """Internal helper for build proxy capture url."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    parsed = parse.urlsplit(capture_url)
    return parse.urlunsplit(
        (
            parse.urlsplit(proxy_base_url).scheme,
            parse.urlsplit(proxy_base_url).netloc,
            parsed.path,
            parsed.query,
            "",
        )
    )


class _DevtoolsClient(object):
    """Tiny synchronous websocket client for Chrome DevTools Protocol."""

    def __init__(self, websocket_url, timeout):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 773

        parsed = parse.urlsplit(websocket_url)
        if parsed.scheme != "ws":
            raise GrafanaError("Unsupported DevTools websocket URL %r." % websocket_url)
        host = parsed.hostname or "127.0.0.1"
        port = int(parsed.port or 80)
        path = parsed.path or "/"
        if parsed.query:
            path = "%s?%s" % (path, parsed.query)
        self._socket = socket.create_connection((host, port), timeout=timeout)
        self._socket.settimeout(timeout)
        self._next_id = 1
        self._connect(host, port, path)

    def close(self):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        with contextlib.suppress(OSError):
            self._socket.close()

    def _connect(self, host, port, path):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request_lines = [
            "GET %s HTTP/1.1" % path,
            "Host: %s:%s" % (host, port),
            "Upgrade: websocket",
            "Connection: Upgrade",
            "Sec-WebSocket-Key: %s" % key,
            "Sec-WebSocket-Version: 13",
            "",
            "",
        ]
        self._socket.sendall("\r\n".join(request_lines).encode("ascii"))
        response = self._recv_http_response()
        if " 101 " not in response.splitlines()[0]:
            raise GrafanaError("DevTools websocket handshake failed: %s" % response.splitlines()[0])
        expected = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")).digest()
        ).decode("ascii")
        if ("sec-websocket-accept: %s" % expected).lower() not in response.lower():
            raise GrafanaError("DevTools websocket handshake returned an unexpected accept key.")

    def _recv_http_response(self):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        chunks = []
        while True:
            chunk = self._socket.recv(4096)
            if not chunk:
                break
            chunks.append(chunk)
            if b"\r\n\r\n" in b"".join(chunks):
                break
        return b"".join(chunks).decode("iso-8859-1")

    def _send_frame(self, payload):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        data = payload.encode("utf-8")
        mask_key = struct.pack("!I", random.getrandbits(32))
        masked = bytearray(data)
        for index in range(len(masked)):
            masked[index] ^= mask_key[index % 4]
        header = bytearray([0x81])
        length = len(masked)
        if length < 126:
            header.append(0x80 | length)
        elif length < (1 << 16):
            header.append(0x80 | 126)
            header.extend(struct.pack("!H", length))
        else:
            header.append(0x80 | 127)
            header.extend(struct.pack("!Q", length))
        self._socket.sendall(bytes(header) + mask_key + bytes(masked))

    def _recv_frame(self):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        header = self._read_exact(2)
        first = header[0]
        second = header[1]
        opcode = first & 0x0F
        length = second & 0x7F
        masked = bool(second & 0x80)
        if length == 126:
            length = struct.unpack("!H", self._read_exact(2))[0]
        elif length == 127:
            length = struct.unpack("!Q", self._read_exact(8))[0]
        mask_key = self._read_exact(4) if masked else None
        payload = self._read_exact(length) if length else b""
        if masked and mask_key:
            decoded = bytearray(payload)
            for index in range(len(decoded)):
                decoded[index] ^= mask_key[index % 4]
            payload = bytes(decoded)
        if opcode == 0x8:
            raise GrafanaError("DevTools websocket closed unexpectedly.")
        if opcode == 0x9:
            self._send_control_frame(0xA, payload)
            return self._recv_frame()
        if opcode != 0x1:
            return None
        return payload.decode("utf-8")

    def _send_control_frame(self, opcode, payload):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        data = bytes(payload or b"")
        mask_key = struct.pack("!I", random.getrandbits(32))
        masked = bytearray(data)
        for index in range(len(masked)):
            masked[index] ^= mask_key[index % 4]
        header = bytearray([0x80 | opcode, 0x80 | len(masked)])
        self._socket.sendall(bytes(header) + mask_key + bytes(masked))

    def _read_exact(self, size):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        chunks = []
        remaining = size
        while remaining > 0:
            chunk = self._socket.recv(remaining)
            if not chunk:
                raise GrafanaError("DevTools websocket closed unexpectedly.")
            chunks.append(chunk)
            remaining -= len(chunk)
        return b"".join(chunks)

    def call(self, method, params=None, session_id=None):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        message_id = self._next_id
        self._next_id += 1
        payload = {"id": message_id, "method": method}
        if params:
            payload["params"] = params
        if session_id:
            payload["sessionId"] = session_id
        self._send_frame(json.dumps(payload))
        while True:
            raw = self._recv_frame()
            if raw is None:
                continue
            message = json.loads(raw)
            if message.get("id") != message_id:
                continue
            if "error" in message:
                raise GrafanaError(
                    "DevTools command %s failed: %s" % (method, message["error"].get("message", message["error"]))
                )
            return message.get("result", {})


def _read_json_url(url, timeout):
    """Internal helper for read json url."""
    handle = urllib_request.urlopen(url, timeout=timeout)
    try:
        return json.loads(handle.read().decode("utf-8"))
    finally:
        handle.close()


def _pick_local_port():
    """Internal helper for pick local port."""
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]
    finally:
        probe.close()


@contextlib.contextmanager
def _launch_devtools_browser(browser_path, request):
    """Internal helper for launch devtools browser."""
    user_data_dir = tempfile.mkdtemp(prefix="grafana-utils-chrome-")
    debug_port = _pick_local_port()
    cmd = [
        browser_path,
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--no-first-run",
        "--no-default-browser-check",
        "--allow-insecure-localhost",
        "--ignore-certificate-errors",
        "--remote-debugging-address=127.0.0.1",
        "--remote-debugging-port=%s" % debug_port,
        "--user-data-dir=%s" % user_data_dir,
        "--window-size=%s,%s" % (int(request["width"]), int(request["height"])),
        "about:blank",
    ]
    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        version = None
        for _ in range(80):
            if process.poll() is not None:
                stderr = process.stderr.read() if process.stderr else ""
                raise GrafanaError("Browser failed to start: %s" % stderr.strip())
            with contextlib.suppress(Exception):
                version = _read_json_url("http://127.0.0.1:%s/json/version" % debug_port, 1)
                if version:
                    break
            threading.Event().wait(0.1)
        if not version:
            raise GrafanaError("Timed out waiting for Chrome DevTools to become ready.")
        yield process, version["webSocketDebuggerUrl"]
    finally:
        with contextlib.suppress(Exception):
            process.terminate()
        with contextlib.suppress(Exception):
            process.wait(timeout=5)
        with contextlib.suppress(Exception):
            shutil.rmtree(user_data_dir)


def _wait_for_ready_state(devtools, session_id, wait_ms):
    """Internal helper for wait for ready state."""
    ready_expression = r"""
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
    """.strip()
    deadline_seconds = max((float(wait_ms) / 1000.0) + 5.0, 10.0)
    sleeper = threading.Event()
    started = time.time()
    while (time.time() - started) < deadline_seconds:
        result = devtools.call(
            "Runtime.evaluate",
            {"expression": ready_expression, "returnByValue": True},
            session_id=session_id,
        )
        if bool(((result or {}).get("result") or {}).get("value")):
            if wait_ms > 0:
                sleeper.wait(float(wait_ms) / 1000.0)
            return
        sleeper.wait(0.25)
    raise GrafanaError("Dashboard page did not become ready before the browser wait timeout elapsed.")


def _evaluate_expression(devtools, session_id, expression):
    """Internal helper for evaluate expression."""
    result = devtools.call(
        "Runtime.evaluate",
        {"expression": expression, "returnByValue": True},
        session_id=session_id,
    )
    return (result.get("result") or {}).get("value")


def _collapse_sidebar_if_present(devtools, session_id):
    """Internal helper for collapse sidebar if present."""
    expression = r"""
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
    """.strip()
    _evaluate_expression(devtools, session_id, expression)
    threading.Event().wait(0.8)


def _prepare_dashboard_capture_dom(devtools, session_id):
    """Internal helper for prepare dashboard capture dom."""
    expression = r"""
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
  const priorStyle = document.querySelector('style[data-grafana-utils-screenshot]');
  if (priorStyle) {
    priorStyle.remove();
  }
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
  const topBarCandidates = Array.from(document.querySelectorAll('body *')).filter((element) => {
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
  let hiddenTopHeight = 0;
  for (const topBar of topBarCandidates) {
    hiddenTopHeight = Math.max(hiddenTopHeight, topBar.getBoundingClientRect().bottom);
    hideElement(topBar);
  }
  let hiddenLeftWidth = 0;
  const sidebars = Array.from(document.querySelectorAll('.sidemenu, [class*="sidemenu"]')).filter(isVisible);
  for (const sidebar of sidebars) {
    hiddenLeftWidth = Math.max(hiddenLeftWidth, sidebar.getBoundingClientRect().width);
  }
  for (const element of Array.from(document.querySelectorAll('body *'))) {
    const computed = window.getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    const marginLeft = Number.parseFloat(computed.marginLeft || '0');
    const left = Number.parseFloat(computed.left || '0');
    const paddingLeft = Number.parseFloat(computed.paddingLeft || '0');
    if (Number.isFinite(marginLeft) && marginLeft >= 180 && marginLeft <= 360) {
      element.style.marginLeft = '0px';
    }
    if (Number.isFinite(left) && left >= 180 && left <= 360) {
      element.style.left = '0px';
    }
    if (Number.isFinite(paddingLeft) && paddingLeft >= 180 && paddingLeft <= 360) {
      element.style.paddingLeft = '0px';
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
  return window.__grafanaUtilsCaptureOffsets;
})()
    """.strip()
    value = _evaluate_expression(devtools, session_id, expression) or {}
    threading.Event().wait(0.25)
    return {
        "hidden_top_height": float((value or {}).get("hiddenTopHeight") or 0.0),
        "hidden_left_width": float((value or {}).get("hiddenLeftWidth") or 0.0),
    }


def _read_numeric_expression(devtools, session_id, expression, minimum):
    """Internal helper for read numeric expression."""
    value = _evaluate_expression(devtools, session_id, expression)
    try:
        numeric = float(value)
    except (TypeError, ValueError):
        numeric = float(minimum)
    return max(numeric, float(minimum))


def _warm_full_page_render(devtools, session_id, request):
    """Internal helper for warm full page render."""
    if not request["full_page"]:
        return
    previous_height = 0.0
    stable_reads = 0
    for _ in range(8):
        height = _read_numeric_expression(
            devtools,
            session_id,
            """
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
            """.strip(),
            request["height"],
        )
        _evaluate_expression(
            devtools,
            session_id,
            """
(() => {
  const target = Math.max(0, %s - window.innerHeight);
  window.scrollTo({ top: target, left: 0, behavior: 'instant' });
  return window.scrollY;
})()
            """.strip() % height,
        )
        threading.Event().wait(1.8)
        next_height = _read_numeric_expression(
            devtools,
            session_id,
            """
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
            """.strip(),
            request["height"],
        )
        if abs(next_height - previous_height) < 1.0 and abs(next_height - height) < 1.0:
            stable_reads += 1
        else:
            stable_reads = 0
        previous_height = next_height
        if stable_reads >= 2:
            break
    _evaluate_expression(
        devtools,
        session_id,
        "window.scrollTo({ top: 0, left: 0, behavior: 'instant' })",
    )
    threading.Event().wait(0.3)


def _capture_stitched_screenshot(devtools, session_id, request, capture_offsets):
    """Internal helper for capture stitched screenshot."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 1046, 1230, 895

    Image, *_ = _pil_modules()
    total_height = _read_numeric_expression(
        devtools,
        session_id,
        """
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
        """.strip(),
        request["height"],
    )
    viewport_height = float(request["height"])
    viewport_width = int(request["width"])
    crop_top = int(max(float(capture_offsets.get("hidden_top_height") or 0.0), 0.0))
    crop_left = int(max(float(capture_offsets.get("hidden_left_width") or 0.0), 0.0))
    target_width = max(viewport_width - crop_left, 1)
    step = max(viewport_height - max(float(capture_offsets.get("hidden_top_height") or 0.0), 0.0), 200.0)
    stitched = Image.new("RGBA", (target_width, max(int(total_height), 1)))
    destination_y = 0
    current_y = 0.0
    format_name = "jpeg" if request["output_format"] == "jpeg" else "png"
    while current_y < total_height - 1.0:
        _evaluate_expression(
            devtools,
            session_id,
            "window.scrollTo({ top: %s, left: 0, behavior: 'instant' });" % int(current_y),
        )
        threading.Event().wait(0.9)
        capture_params = {
            "format": format_name,
            "captureBeyondViewport": False,
        }
        if format_name == "jpeg":
            capture_params["quality"] = 90
        captured = devtools.call(
            "Page.captureScreenshot",
            capture_params,
            session_id=session_id,
        )
        segment = Image.open(io.BytesIO(base64.b64decode(captured["data"]))).convert("RGBA")
        source_left = min(crop_left, max(segment.width - 1, 0))
        source_top = 0 if current_y <= 0.0 else min(crop_top, segment.height)
        remaining_height = max(stitched.height - destination_y, 0)
        if remaining_height == 0:
            break
        available_segment_height = max(segment.height - source_top, 0)
        available_segment_width = max(segment.width - source_left, 0)
        copy_height = min(available_segment_height, remaining_height)
        copy_width = min(available_segment_width, target_width)
        if copy_height == 0 or copy_width == 0:
            break
        cropped = segment.crop((source_left, source_top, source_left + copy_width, source_top + copy_height))
        stitched.paste(cropped, (0, destination_y))
        destination_y += copy_height
        if destination_y >= stitched.height:
            break
        current_y += step
    final_height = max(destination_y, 1)
    return stitched.crop((0, 0, target_width, final_height))


def _capture_full_page_segments(devtools, session_id, request, capture_offsets):
    """Internal helper for capture full page segments."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1534
    #   Downstream callees: 1046, 1230, 895

    Image, *_ = _pil_modules()
    total_height = _read_numeric_expression(
        devtools,
        session_id,
        """
Math.max(
  document.documentElement.scrollHeight || 0,
  document.body ? document.body.scrollHeight || 0 : 0,
  window.innerHeight || 0
)
        """.strip(),
        request["height"],
    )
    viewport_height = float(request["height"])
    viewport_width = int(request["width"])
    crop_top = int(max(float(capture_offsets.get("hidden_top_height") or 0.0), 0.0))
    crop_left = int(max(float(capture_offsets.get("hidden_left_width") or 0.0), 0.0))
    target_width = max(viewport_width - crop_left, 1)
    step = max(viewport_height - max(float(capture_offsets.get("hidden_top_height") or 0.0), 0.0), 200.0)
    format_name = "jpeg" if request["output_format"] == "jpeg" else "png"
    segments = []
    destination_y = 0
    current_y = 0.0
    while current_y < total_height - 1.0:
        _evaluate_expression(
            devtools,
            session_id,
            "window.scrollTo({ top: %s, left: 0, behavior: 'instant' });" % int(current_y),
        )
        threading.Event().wait(0.9)
        capture_params = {
            "format": format_name,
            "captureBeyondViewport": False,
        }
        if format_name == "jpeg":
            capture_params["quality"] = 90
        captured = devtools.call(
            "Page.captureScreenshot",
            capture_params,
            session_id=session_id,
        )
        segment = Image.open(io.BytesIO(base64.b64decode(captured["data"]))).convert("RGBA")
        source_left = min(crop_left, max(segment.width - 1, 0))
        source_top = 0 if current_y <= 0.0 else min(crop_top, segment.height)
        available_segment_height = max(segment.height - source_top, 0)
        available_segment_width = max(segment.width - source_left, 0)
        copy_width = min(available_segment_width, target_width)
        if available_segment_height == 0 or copy_width == 0:
            break
        cropped = segment.crop((source_left, source_top, source_left + copy_width, segment.height))
        segments.append(
            {
                "index": len(segments),
                "scroll_y": int(current_y),
                "source_top": source_top,
                "image": cropped,
                "destination_y": destination_y,
            }
        )
        destination_y += cropped.height
        current_y += step
    return {
        "total_height": int(total_height),
        "viewport_width": viewport_width,
        "viewport_height": int(viewport_height),
        "device_scale_factor": float(request.get("device_scale_factor", 1.0) or 1.0),
        "target_width": target_width,
        "crop_top": crop_top,
        "crop_left": crop_left,
        "step": step,
        "segments": segments,
    }


def _build_segment_output_dir(output_path):
    """Internal helper for build segment output dir."""
    output = Path(output_path)
    parent = output.parent
    file_stem = (output.stem or output.name or "").strip()
    if not file_stem:
        raise GrafanaError(
            "Unable to derive a segment output directory from --output. Use a normal filename such as ./dashboard.png."
        )
    return parent / file_stem


def _build_full_page_manifest(request, metadata, capture, manifest_segments):
    """Internal helper for build full page manifest."""
    metadata = metadata or request.get("metadata") or {}
    auto_title = _resolve_auto_header_title(request, metadata)
    return {
        "kind": "dashboard-screenshot-segments",
        "version": 1,
        "outputMode": request["full_page_output"],
        "outputFormat": request["output_format"],
        "fullPage": bool(request["full_page"]),
        "output": str(request["output"]),
        "viewport": {
            "width": capture["viewport_width"],
            "height": capture["viewport_height"],
            "deviceScaleFactor": capture["device_scale_factor"],
        },
        "capture": {
            "totalHeight": capture["total_height"],
            "targetWidth": capture["target_width"],
            "cropTop": capture["crop_top"],
            "cropLeft": capture["crop_left"],
            "step": capture["step"],
        },
        "title": auto_title,
        "headerTitle": _resolve_optional_header_field(
            request.get("header_title"),
            auto_title,
        ),
        "dashboardTitle": metadata.get("dashboard_title"),
        "panelTitle": metadata.get("panel_title"),
        "dashboardUid": metadata.get("dashboard_uid"),
        "segments": manifest_segments,
    }


def _write_full_page_output(request, metadata, capture):
    """Internal helper for write full page output."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1534
    #   Downstream callees: 1437, 1449, 447, 526

    output_format = request["output_format"]
    if request["full_page_output"] == "single":
        stitched = Image.new("RGBA", (capture["target_width"], max(capture["total_height"], 1)))
        destination_y = 0
        for segment in capture["segments"]:
            image = segment["image"]
            stitched.paste(image, (0, destination_y))
            destination_y += image.height
        final_image = stitched.crop((0, 0, capture["target_width"], max(destination_y, 1)))
        _write_raster_output(final_image, request, metadata)
        return

    if output_format == "pdf":
        raise GrafanaError("PDF output does not support --full-page-output tiles or manifest.")
    output_dir = _build_segment_output_dir(request["output"])
    os.makedirs(str(output_dir), exist_ok=True)
    extension = "jpg" if output_format == "jpeg" else "png"
    manifest_segments = []
    for segment in capture["segments"]:
        file_name = "part-%04d.%s" % (segment["index"] + 1, extension)
        tile_path = output_dir / file_name
        tile_request = dict(request)
        tile_request["output"] = tile_path
        if segment["index"] != 0:
            tile_request["header_title"] = None
            tile_request["header_url"] = None
            tile_request["header_captured_at"] = False
            tile_request["header_text"] = None
        _write_raster_output(segment["image"], tile_request, metadata)
        manifest_segments.append(
            {
                "file": file_name,
                "index": segment["index"],
                "scrollY": segment["scroll_y"],
                "sourceTop": segment["source_top"],
                "width": segment["image"].width,
                "height": segment["image"].height,
                "headerApplied": bool(segment["index"] == 0 and _build_header_lines(request, metadata)),
            }
        )
    if request["full_page_output"] == "manifest":
        manifest_path = output_dir / "manifest.json"
        manifest_path.write_text(
            json.dumps(_build_full_page_manifest(request, metadata, capture, manifest_segments), indent=2, sort_keys=True),
            encoding="utf-8",
        )


def _capture_via_devtools(browser_path, request, extra_headers, timeout):
    """Internal helper for capture via devtools."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1605
    #   Downstream callees: 1056, 1087, 1240, 1362, 1484, 526, 765, 895, 942, 990

    with _launch_devtools_browser(browser_path, request) as (_, websocket_url):
        Image, *_ = _pil_modules()
        devtools = _DevtoolsClient(websocket_url, timeout)
        try:
            created = devtools.call("Target.createTarget", {"url": "about:blank", "newWindow": False})
            target_id = created.get("targetId")
            attached = devtools.call(
                "Target.attachToTarget",
                {"targetId": target_id, "flatten": True},
            )
            session_id = attached.get("sessionId")
            devtools.call("Page.enable", session_id=session_id)
            devtools.call("Network.enable", session_id=session_id)
            if extra_headers:
                devtools.call(
                    "Network.setExtraHTTPHeaders",
                    {"headers": dict(extra_headers)},
                    session_id=session_id,
                )
            devtools.call(
                "Emulation.setDeviceMetricsOverride",
                {
                    "width": int(request["width"]),
                    "height": int(request["height"]),
                    "deviceScaleFactor": float(request.get("device_scale_factor", 1.0) or 1.0),
                    "mobile": False,
                },
                session_id=session_id,
            )
            devtools.call("Page.navigate", {"url": request["url"]}, session_id=session_id)
            _wait_for_ready_state(devtools, session_id, int(request["wait_ms"]))
            _collapse_sidebar_if_present(devtools, session_id)
            capture_offsets = _prepare_dashboard_capture_dom(devtools, session_id)
            _warm_full_page_render(devtools, session_id, request)

            if request["output_format"] == "pdf":
                pdf = devtools.call(
                    "Page.printToPDF",
                    {"printBackground": True, "preferCSSPageSize": True, "displayHeaderFooter": False},
                    session_id=session_id,
                )
                request["output"].write_bytes(base64.b64decode(pdf["data"]))
                return

            if request["full_page"]:
                capture = _capture_full_page_segments(devtools, session_id, request, capture_offsets)
                _write_full_page_output(request, request.get("metadata"), capture)
                return

            format_name = "jpeg" if request["output_format"] == "jpeg" else "png"
            capture_params = {
                "format": format_name,
                "captureBeyondViewport": False,
                "clip": {
                    "x": 0,
                    "y": 0,
                    "width": int(request["width"]),
                    "height": int(request["height"]),
                    "scale": float(request.get("device_scale_factor", 1.0) or 1.0),
                },
            }
            if format_name == "jpeg":
                capture_params["quality"] = 90
            captured = devtools.call("Page.captureScreenshot", capture_params, session_id=session_id)
            image = Image.open(io.BytesIO(base64.b64decode(captured["data"]))).convert("RGBA")
            _write_raster_output(image, request, request.get("metadata"))
        finally:
            devtools.close()


def _run_browser_capture(browser_path, request, headers, timeout):
    """Internal helper for run browser capture."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1626
    #   Downstream callees: 1534

    output_format = request["output_format"]
    if output_format in ("png", "pdf"):
        _capture_via_devtools(browser_path, request, headers, timeout)
        return
    temp_dir = tempfile.mkdtemp(prefix="grafana-utils-shot-")
    try:
        Image, *_ = _pil_modules()
        png_request = dict(request)
        png_request["output"] = Path(temp_dir) / "capture.png"
        png_request["output_format"] = "png"
        _capture_via_devtools(browser_path, png_request, headers, timeout)
        image = Image.open(str(png_request["output"]))
        image.convert("RGB").save(str(request["output"]), format="JPEG", quality=90)
    finally:
        with contextlib.suppress(FileNotFoundError):
            os.remove(str(Path(temp_dir) / "capture.png"))
        with contextlib.suppress(OSError):
            os.rmdir(str(temp_dir))


def _capture_with_browser_cli(request, client):
    """Capture browser-rendered output through a local Chromium DevTools session."""
    browser_path = find_browser_executable(getattr(client, "browser_path", None))
    _run_browser_capture(
        browser_path,
        request,
        dict(getattr(client, "headers", {}) or {}),
        int(getattr(client, "timeout", 30) or 30),
    )
    return {
        "browser": browser_path,
        "output": request["output"],
        "output_format": request["output_format"],
        "url": request["url"],
        "size": request["output"].stat().st_size if request["output"].exists() else 0,
    }


def capture_dashboard_screenshot(args, backend=None, client=None):
    """Run a screenshot capture through an injected or live backend."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 1626, 317, 344

    metadata = None
    if client is not None:
        metadata = _resolve_capture_metadata(args, client)
    request = build_capture_request(args)
    request["metadata"] = metadata
    output_parent = request["output"].parent
    if str(output_parent) and str(output_parent) != ".":
        os.makedirs(str(output_parent), exist_ok=True)
    if request["print_capture_url"]:
        print("Capture URL: %s" % request["url"], file=sys.stderr)
    if backend is None:
        if client is None:
            raise GrafanaError(
                "Dashboard screenshot capture requires a browser backend or a configured Grafana client."
            )
        client.browser_path = getattr(args, "browser_path", None)
        backend = lambda capture_request: _capture_with_browser_cli(capture_request, client)
    result = backend(request)
    return result


def make_screenshot_args(**kwargs):
    """Test helper for lightweight namespace construction."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    defaults = {
        "url": "http://localhost:3000",
        "dashboard_uid": None,
        "dashboard_url": None,
        "slug": None,
        "panel_id": None,
        "org_id": None,
        "from_value": None,
        "to_value": None,
        "vars": [],
        "vars_query": None,
        "theme": DEFAULT_SCREENSHOT_THEME,
        "output": "capture.png",
        "output_format": None,
        "browser_path": None,
        "width": 1440,
        "height": 1024,
        "device_scale_factor": 1.0,
        "full_page": False,
        "full_page_output": "single",
        "wait_ms": 5000,
        "print_capture_url": False,
        "header_title": None,
        "header_url": None,
        "header_captured_at": False,
        "header_text": None,
    }
    defaults.update(kwargs)
    return SimpleNamespace(**defaults)
