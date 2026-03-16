import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest import mock

from PIL import Image

from grafana_utils.dashboards.common import GrafanaError
from grafana_utils.dashboards import screenshot


class FakeClient(object):
    def __init__(self):
        self.headers = {"Authorization": "Basic abc"}
        self.base_url = "http://localhost:3000"
        self.timeout = 15
        self.verify_ssl = False

    def fetch_dashboard(self, uid):
        return {
            "meta": {"slug": "cpu-overview"},
            "dashboard": {"uid": uid, "title": "CPU Overview"},
        }


class DashboardScreenshotHelperTests(unittest.TestCase):
    def test_capture_via_devtools_uses_stitch_for_full_page_png(self):
        class FakeDevtools(object):
            def __init__(self, websocket_url, timeout):
                self.calls = []

            def call(self, method, params=None, session_id=None):
                self.calls.append((method, params, session_id))
                if method == "Target.createTarget":
                    return {"targetId": "target-1"}
                if method == "Target.attachToTarget":
                    return {"sessionId": "session-1"}
                if method == "Page.captureScreenshot":
                    self_calls = [item for item in self.calls if item[0] == "Page.captureScreenshot"]
                    return {"data": ""}
                if method == "Page.printToPDF":
                    return {"data": ""}
                return {}

            def close(self):
                return None

        with tempfile.TemporaryDirectory() as tmpdir:
            output_path = Path(tmpdir) / "full-page.png"
            request = screenshot.build_capture_request(
                screenshot.make_screenshot_args(
                    dashboard_uid="cpu-main",
                    output=str(output_path),
                    full_page=True,
                )
            )
            stitched = Image.new("RGBA", (1200, 2400), (1, 2, 3, 255))
            with mock.patch.object(
                screenshot,
                "_launch_devtools_browser",
            ) as mocked_launch, mock.patch.object(
                screenshot,
                "_DevtoolsClient",
                FakeDevtools,
            ), mock.patch.object(
                screenshot,
                "_wait_for_ready_state",
                return_value=None,
            ), mock.patch.object(
                screenshot,
                "_collapse_sidebar_if_present",
                return_value=None,
            ), mock.patch.object(
                screenshot,
                "_prepare_dashboard_capture_dom",
                return_value={"hidden_top_height": 100.0, "hidden_left_width": 0.0},
            ), mock.patch.object(
                screenshot,
                "_warm_full_page_render",
                return_value=None,
            ), mock.patch.object(
                screenshot,
                "_capture_full_page_segments",
                return_value={
                    "viewport_width": 1440,
                    "viewport_height": 1024,
                    "device_scale_factor": 1.0,
                    "total_height": 2400,
                    "target_width": 1200,
                    "crop_top": 100,
                    "crop_left": 0,
                    "step": 924.0,
                    "segments": [
                        {"index": 0, "scroll_y": 0, "source_top": 0, "image": stitched},
                    ],
                },
            ) as mocked_capture, mock.patch.object(
                screenshot,
                "_write_full_page_output",
                return_value=None,
            ) as mocked_write:
                mocked_launch.return_value.__enter__.return_value = (object(), "ws://127.0.0.1:9222/devtools/page/1")
                mocked_launch.return_value.__exit__.return_value = None
                screenshot._capture_via_devtools(
                    "/tmp/chrome",
                    request,
                    {"Authorization": "Basic abc"},
                    15,
                )
                mocked_capture.assert_called_once()
                mocked_write.assert_called_once()

    def test_infer_output_format_uses_extension(self):
        self.assertEqual(
            screenshot.infer_screenshot_output_format("capture.jpg"),
            "jpeg",
        )
        self.assertEqual(
            screenshot.infer_screenshot_output_format("capture.pdf"),
            "pdf",
        )

    def test_infer_output_format_rejects_unknown_extension(self):
        with self.assertRaisesRegex(GrafanaError, "Unable to infer screenshot output format"):
            screenshot.infer_screenshot_output_format("capture.txt")

    def test_parse_var_assignment_requires_name_and_value(self):
        self.assertEqual(
            screenshot.parse_var_assignment("env=prod"),
            ("env", "prod"),
        )
        with self.assertRaisesRegex(GrafanaError, "Use NAME=VALUE"):
            screenshot.parse_var_assignment("broken")
        with self.assertRaisesRegex(GrafanaError, "VALUE cannot be empty"):
            screenshot.parse_var_assignment("env=")

    def test_parse_vars_query_splits_vars_and_passthrough_pairs(self):
        parsed = screenshot.parse_vars_query(
            "var-env=prod&panelId=7&from=now-6h&kiosk=1"
        )
        self.assertEqual(parsed["vars"], [("env", "prod")])
        self.assertEqual(
            parsed["passthrough_pairs"],
            [("panelId", "7"), ("from", "now-6h"), ("kiosk", "1")],
        )

    def test_validate_screenshot_args_requires_target(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid=None,
            dashboard_url=None,
        )
        with self.assertRaisesRegex(GrafanaError, "Set --dashboard-uid or pass --dashboard-url"):
            screenshot.validate_screenshot_args(args)

    def test_validate_screenshot_args_rejects_invalid_dimensions(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            width=0,
        )
        with self.assertRaisesRegex(GrafanaError, "--width must be greater than 0"):
            screenshot.validate_screenshot_args(args)

    def test_validate_screenshot_args_rejects_tiles_without_full_page(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            full_page=False,
            full_page_output="tiles",
        )
        with self.assertRaisesRegex(GrafanaError, "--full-page-output tiles or manifest requires --full-page"):
            screenshot.validate_screenshot_args(args)

    def test_validate_screenshot_args_rejects_pdf_tiles_mode(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            output="capture.pdf",
            full_page=True,
            full_page_output="tiles",
        )
        with self.assertRaisesRegex(GrafanaError, "PDF output does not support --full-page-output tiles or manifest"):
            screenshot.validate_screenshot_args(args)

    def test_build_dashboard_capture_url_from_base_url(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            slug="cpu-overview",
            panel_id=7,
            org_id=2,
            from_value="now-6h",
            to_value="now",
            vars=["cluster=prod-a", "node=web-01"],
            theme="light",
        )
        url = screenshot.build_dashboard_capture_url(args)
        self.assertIn("/d-solo/cpu-main/cpu-overview", url)
        self.assertIn("panelId=7", url)
        self.assertIn("viewPanel=7", url)
        self.assertIn("orgId=2", url)
        self.assertIn("from=now-6h", url)
        self.assertIn("to=now", url)
        self.assertIn("theme=light", url)
        self.assertIn("kiosk=tv", url)
        self.assertIn("var-cluster=prod-a", url)
        self.assertIn("var-node=web-01", url)

    def test_build_dashboard_capture_url_merges_dashboard_url_and_overrides(self):
        args = screenshot.make_screenshot_args(
            dashboard_url=(
                "https://grafana.example.com/d/cpu-main/cpu-overview"
                "?orgId=1&from=now-1h&var-env=prod&keep=1"
            ),
            panel_id=9,
            vars_query="var-env=stage&var-node=web-02&from=now-24h",
            vars=["node=web-03"],
            theme="dark",
        )
        url = screenshot.build_dashboard_capture_url(args)
        self.assertIn("https://grafana.example.com/d-solo/cpu-main/cpu-overview", url)
        self.assertIn("panelId=9", url)
        self.assertIn("orgId=1", url)
        self.assertIn("from=now-24h", url)
        self.assertIn("keep=1", url)
        self.assertIn("var-env=stage", url)
        self.assertIn("var-node=web-03", url)

    def test_capture_dashboard_screenshot_requires_backend(self):
        args = screenshot.make_screenshot_args(dashboard_uid="cpu-main")
        with self.assertRaisesRegex(GrafanaError, "requires a browser backend or a configured Grafana client"):
            screenshot.capture_dashboard_screenshot(args)

    def test_capture_dashboard_screenshot_passes_normalized_request_to_backend(self):
        captured = {}

        def backend(request):
            captured.update(request)
            return {"status": "ok", "path": str(request["output"])}

        with tempfile.TemporaryDirectory() as tmpdir:
            output_path = Path(tmpdir) / "captures" / "cpu-main.png"
            args = screenshot.make_screenshot_args(
                dashboard_uid="cpu-main",
                output=str(output_path),
                full_page=True,
            )
            result = screenshot.capture_dashboard_screenshot(args, backend=backend)

        self.assertEqual(result["status"], "ok")
        self.assertEqual(captured["output"], output_path)
        self.assertEqual(captured["output_format"], "png")
        self.assertTrue(captured["full_page"])
        self.assertIn("/d/cpu-main/cpu-main", captured["url"])

    def test_build_capture_request_includes_render_url(self):
        args = screenshot.make_screenshot_args(dashboard_uid="cpu-main")
        request = screenshot.build_capture_request(args)
        self.assertIn("/render/d/cpu-main/cpu-main", request["render_url"])
        self.assertIn("width=1440", request["render_url"])
        self.assertIn("height=1024", request["render_url"])
        self.assertEqual(request["device_scale_factor"], 1.0)

    def test_build_capture_request_keeps_device_scale_factor(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            device_scale_factor=2.0,
        )
        request = screenshot.build_capture_request(args)
        self.assertEqual(request["device_scale_factor"], 2.0)
        self.assertEqual(request["full_page_output"], "single")

    def test_build_capture_request_keeps_full_page_output(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            full_page=True,
            full_page_output="manifest",
        )
        request = screenshot.build_capture_request(args)
        self.assertEqual(request["full_page_output"], "manifest")

    def test_build_capture_request_keeps_header_fields(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            print_capture_url=True,
            header_title="__auto__",
            header_url="__auto__",
            header_captured_at=True,
            header_text="Operator note",
        )
        request = screenshot.build_capture_request(args)
        self.assertTrue(request["print_capture_url"])
        self.assertEqual(request["header_title"], "__auto__")
        self.assertEqual(request["header_url"], "__auto__")
        self.assertTrue(request["header_captured_at"])
        self.assertEqual(request["header_text"], "Operator note")

    def test_compose_header_image_adds_top_block(self):
        image = Image.new("RGBA", (300, 120), (10, 20, 30, 255))
        request = {
            "output": Path("capture.png"),
            "output_format": "png",
            "url": "https://grafana.example.com/d/cpu-main/cpu-overview",
            "header_title": "__auto__",
            "header_url": "__auto__",
            "header_captured_at": True,
            "header_text": "Operator note",
        }
        metadata = {
            "dashboard_uid": "cpu-main",
            "dashboard_title": "CPU Overview",
            "panel_title": None,
        }
        rendered = screenshot._compose_header_image(image, request, metadata)
        self.assertGreater(rendered.height, image.height)
        self.assertEqual(rendered.width, image.width)

    def test_capture_dashboard_screenshot_resolves_slug_from_live_dashboard(self):
        captured = {}

        def backend(request):
            captured.update(request)
            return {"output": request["output"], "output_format": request["output_format"]}

        args = screenshot.make_screenshot_args(dashboard_uid="cpu-main")
        screenshot.capture_dashboard_screenshot(args, backend=backend, client=FakeClient())
        self.assertIn("/d/cpu-main/cpu-overview", captured["url"])

    def test_capture_dashboard_screenshot_uses_browser_backend_by_default(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_path = Path(tmpdir) / "cpu-main.png"
            args = screenshot.make_screenshot_args(
                dashboard_uid="cpu-main",
                output=str(output_path),
            )
            with mock.patch.object(
                screenshot,
                "_capture_with_browser_cli",
                return_value={"output": output_path, "output_format": "png"},
            ) as mocked:
                result = screenshot.capture_dashboard_screenshot(
                    args,
                    client=FakeClient(),
                )

        self.assertEqual(result["output"], output_path)
        mocked.assert_called_once()

    def test_capture_dashboard_screenshot_allows_pdf_with_browser_backend(self):
        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            output="capture.pdf",
        )
        with mock.patch.object(
            screenshot,
            "_capture_with_browser_cli",
            return_value={"output": Path("capture.pdf"), "output_format": "pdf"},
        ) as mocked:
            result = screenshot.capture_dashboard_screenshot(args, client=FakeClient())
        self.assertEqual(result["output_format"], "pdf")
        mocked.assert_called_once()

    def test_capture_dashboard_screenshot_prints_capture_url(self):
        stderr = io.StringIO()

        def backend(request):
            return {"output": request["output"], "output_format": request["output_format"]}

        args = screenshot.make_screenshot_args(
            dashboard_uid="cpu-main",
            print_capture_url=True,
        )
        with redirect_stderr(stderr):
            screenshot.capture_dashboard_screenshot(args, backend=backend, client=FakeClient())
        self.assertIn("Capture URL:", stderr.getvalue())

    def test_write_full_page_output_tiles_writes_parts_without_manifest(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            request = {
                "output": Path(tmpdir) / "capture.png",
                "output_format": "png",
                "full_page": True,
                "full_page_output": "tiles",
                "header_title": "__auto__",
                "header_url": None,
                "header_captured_at": False,
                "header_text": None,
                "url": "https://grafana.example.com/d/cpu-main/cpu-overview",
            }
            capture = {
                "viewport_width": 1440,
                "viewport_height": 1024,
                "device_scale_factor": 1.0,
                "total_height": 300,
                "target_width": 200,
                "crop_top": 10,
                "crop_left": 0,
                "step": 900.0,
                "segments": [
                    {"index": 0, "scroll_y": 0, "source_top": 0, "image": Image.new("RGBA", (200, 150), (1, 2, 3, 255))},
                    {"index": 1, "scroll_y": 140, "source_top": 10, "image": Image.new("RGBA", (200, 140), (4, 5, 6, 255))},
                ],
            }
            metadata = {"dashboard_uid": "cpu-main", "dashboard_title": "CPU Overview", "panel_title": None}
            screenshot._write_full_page_output(request, metadata, capture)
            output_dir = Path(tmpdir) / "capture"
            self.assertTrue((output_dir / "part-0001.png").exists())
            self.assertTrue((output_dir / "part-0002.png").exists())
            self.assertFalse((output_dir / "manifest.json").exists())

    def test_write_full_page_output_manifest_writes_manifest(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            request = {
                "output": Path(tmpdir) / "capture.png",
                "output_format": "png",
                "full_page": True,
                "full_page_output": "manifest",
                "header_title": "__auto__",
                "header_url": None,
                "header_captured_at": False,
                "header_text": None,
                "url": "https://grafana.example.com/d/cpu-main/cpu-overview",
            }
            capture = {
                "viewport_width": 1440,
                "viewport_height": 1024,
                "device_scale_factor": 1.0,
                "total_height": 150,
                "target_width": 200,
                "crop_top": 10,
                "crop_left": 0,
                "step": 900.0,
                "segments": [
                    {"index": 0, "scroll_y": 0, "source_top": 0, "image": Image.new("RGBA", (200, 150), (1, 2, 3, 255))},
                ],
            }
            metadata = {"dashboard_uid": "cpu-main", "dashboard_title": "CPU Overview", "panel_title": None}
            screenshot._write_full_page_output(request, metadata, capture)
            manifest_path = Path(tmpdir) / "capture" / "manifest.json"
            self.assertTrue(manifest_path.exists())
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["outputMode"], "manifest")
            self.assertEqual(manifest["title"], "CPU Overview")
            self.assertEqual(manifest["headerTitle"], "CPU Overview")
            self.assertEqual(manifest["dashboardTitle"], "CPU Overview")
            self.assertEqual(manifest["panelTitle"], None)
            self.assertEqual(manifest["dashboardUid"], "cpu-main")
            self.assertEqual(manifest["segments"][0]["file"], "part-0001.png")


if __name__ == "__main__":
    unittest.main()
