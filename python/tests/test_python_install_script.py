import os
import stat
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
INSTALL_SCRIPT_PATH = REPO_ROOT / "scripts" / "install.sh"


class InstallScriptTests(unittest.TestCase):
    def test_install_script_exists(self):
        self.assertTrue(INSTALL_SCRIPT_PATH.is_file())

    def test_install_script_declares_release_download_contract(self):
        content = INSTALL_SCRIPT_PATH.read_text(encoding="utf-8")

        self.assertIn('REPO="${REPO:-kenduest-brobridge/grafana-util}"', content)
        self.assertIn("https://api.github.com/repos/${REPO}/releases/latest", content)
        self.assertIn(
            'archive_name="grafana-utils-rust-${PLATFORM}${ARTIFACT_SUFFIX}-${release_tag}.tar.gz"',
            content,
        )
        self.assertIn('RUST_ARTIFACT_FLAVOR="${RUST_ARTIFACT_FLAVOR:-standard}"', content)
        self.assertIn('browser) printf \'%s\\n\' "-browser" ;;', content)
        self.assertIn(
            'archive_url="https://github.com/${REPO}/releases/download/${release_tag}/${archive_name}"',
            content,
        )
        self.assertIn(
            'supported targets: linux-amd64, macos-arm64',
            content,
        )
        self.assertIn("BIN_DIR=/custom/bin", content)
        self.assertIn("$HOME/.local/bin", content)

    def test_install_script_help_describes_bin_dir_and_path_behavior(self):
        completed = subprocess.run(
            ["sh", str(INSTALL_SCRIPT_PATH), "--help"],
            cwd=str(REPO_ROOT),
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(completed.returncode, 0, msg=completed.stderr)
        self.assertIn("BIN_DIR=/custom/bin", completed.stdout)
        self.assertIn("$HOME/.local/bin", completed.stdout)
        self.assertIn("PATH", completed.stdout)

    def test_install_script_installs_from_local_archive_override(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "grafana-util"
            fake_binary.write_text("#!/bin/sh\necho grafana-util test build\n", encoding="utf-8")
            fake_binary.chmod(0o755)

            archive_path = temp_path / "grafana-utils-rust-linux-amd64-v9.9.9.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(fake_binary, arcname="grafana-util")

            bin_dir = temp_path / "bin"
            home_dir = temp_path / "home"
            home_dir.mkdir()
            env = os.environ.copy()
            env.update(
                {
                    "ASSET_URL": archive_path.resolve().as_uri(),
                    "BIN_DIR": str(bin_dir),
                    "HOME": str(home_dir),
                    "PATH": "/usr/bin:/bin",
                    "VERSION": "v9.9.9",
                }
            )

            completed = subprocess.run(
                ["sh", str(INSTALL_SCRIPT_PATH)],
                cwd=str(REPO_ROOT),
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 0, msg=completed.stderr)
            installed_binary = bin_dir / "grafana-util"
            self.assertTrue(installed_binary.is_file())
            self.assertTrue(os.access(installed_binary, os.X_OK))
            self.assertIn("Installed grafana-util to", completed.stdout)
            self.assertIn("The install directory is not currently on PATH.", completed.stdout)
            self.assertIn("Add " + str(bin_dir) + " to PATH if needed:", completed.stdout)

            mode = installed_binary.stat().st_mode
            self.assertTrue(mode & stat.S_IXUSR)

            run_completed = subprocess.run(
                [str(installed_binary)],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(run_completed.returncode, 0, msg=run_completed.stderr)
            self.assertEqual(run_completed.stdout.strip(), "grafana-util test build")


if __name__ == "__main__":
    unittest.main()
