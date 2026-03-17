import argparse
import ast
import base64
import importlib
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "auth_staging.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
auth_staging = importlib.import_module("grafana_utils.auth_staging")


class AuthStagingTests(unittest.TestCase):
    def test_auth_staging_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_auth_staging_resolve_auth_headers_supports_token_auth(self):
        headers, auth_mode = auth_staging.resolve_auth_headers(token="abc123")

        self.assertEqual(auth_mode, "token")
        self.assertEqual(headers["Authorization"], "Bearer abc123")

    def test_auth_staging_resolve_auth_headers_supports_prompt_token(self):
        prompts = []

        def fake_prompt(prompt):
            prompts.append(prompt)
            return "prompted-token"

        headers, auth_mode = auth_staging.resolve_auth_headers(
            prompt_token=True,
            token_prompt_reader=fake_prompt,
        )

        self.assertEqual(auth_mode, "token")
        self.assertEqual(prompts, ["Grafana API token: "])
        self.assertEqual(headers["Authorization"], "Bearer prompted-token")

    def test_auth_staging_resolve_auth_headers_supports_basic_auth(self):
        headers, auth_mode = auth_staging.resolve_auth_headers(
            username="ops",
            password="secret",
        )

        self.assertEqual(auth_mode, "basic")
        expected = base64.b64encode(b"ops:secret").decode("ascii")
        self.assertEqual(headers["Authorization"], "Basic %s" % expected)

    def test_auth_staging_resolve_auth_headers_prefers_explicit_basic_auth_over_env_token(self):
        headers, auth_mode = auth_staging.resolve_auth_headers(
            username="ops",
            password="secret",
            env={"GRAFANA_API_TOKEN": "env-token"},
        )

        self.assertEqual(auth_mode, "basic")
        self.assertIn("Basic ", headers["Authorization"])

    def test_auth_staging_resolve_auth_headers_rejects_mixed_auth_modes(self):
        with self.assertRaises(auth_staging.AuthConfigError):
            auth_staging.resolve_auth_headers(
                token="abc123",
                username="ops",
                password="secret",
            )

    def test_auth_staging_resolve_auth_headers_rejects_explicit_and_prompt_token_together(self):
        with self.assertRaises(auth_staging.AuthConfigError):
            auth_staging.resolve_auth_headers(
                token="abc123",
                prompt_token=True,
            )

    def test_auth_staging_resolve_auth_headers_supports_prompt_password(self):
        prompts = []

        def fake_prompt(prompt):
            prompts.append(prompt)
            return "prompted"

        headers, auth_mode = auth_staging.resolve_auth_headers(
            username="ops",
            prompt_password=True,
            prompt_reader=fake_prompt,
        )

        self.assertEqual(auth_mode, "basic")
        self.assertEqual(prompts, ["Grafana Basic auth password: "])
        expected = base64.b64encode(b"ops:prompted").decode("ascii")
        self.assertEqual(headers["Authorization"], "Basic %s" % expected)

    def test_auth_staging_resolve_auth_headers_rejects_partial_env_basic_auth(self):
        with self.assertRaises(auth_staging.AuthConfigError):
            auth_staging.resolve_auth_headers(
                env={"GRAFANA_USERNAME": "ops"},
            )

    def test_auth_staging_format_cli_auth_error_message_rewrites_basic_auth_requirement(self):
        message = auth_staging.format_cli_auth_error_message(
            "Basic auth requires both username and password or --prompt-password."
        )

        self.assertEqual(
            message,
            "Basic auth requires both --basic-user and "
            "--basic-password or --prompt-password.",
        )

    def test_auth_staging_resolve_cli_auth_from_namespace_rewrites_auth_errors(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password=None,
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            auth_staging.AuthConfigError,
            "--prompt-password requires --basic-user.",
        ):
            auth_staging.resolve_cli_auth_from_namespace(args)

    def test_auth_staging_add_org_id_header_returns_copy(self):
        original = {"Authorization": "Bearer token"}

        resolved = auth_staging.add_org_id_header(original, 17)

        self.assertEqual(original, {"Authorization": "Bearer token"})
        self.assertEqual(resolved["X-Grafana-Org-Id"], "17")

    def test_auth_staging_resolve_auth_from_namespace_supports_fallback_auth_attrs(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password=None,
            auth_username="ops",
            auth_password="secret",
            prompt_password=False,
            org_id=9,
        )

        headers, auth_mode = auth_staging.resolve_auth_from_namespace(args)

        self.assertEqual(auth_mode, "basic")
        self.assertEqual(headers["X-Grafana-Org-Id"], "9")
        self.assertIn("Basic ", headers["Authorization"])
