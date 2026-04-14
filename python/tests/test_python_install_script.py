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
        self.assertIn('INSTALL_COMPLETION="${INSTALL_COMPLETION:-}"', content)
        self.assertIn("supported values: auto, bash, zsh", content)
        self.assertIn("--interactive", content)
        self.assertIn('INSTALL_TTY="${INSTALL_TTY:-/dev/tty}"', content)

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
        self.assertIn("INSTALL_COMPLETION=auto", completed.stdout)
        self.assertIn("--interactive", completed.stdout)
        self.assertIn("~/.zshrc", completed.stdout)

    def test_install_script_can_auto_install_zsh_completion_from_installed_binary(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "grafana-util"
            fake_binary.write_text(
                "#!/bin/sh\n"
                'if [ "${1:-}" = completion ] && [ "${2:-}" = zsh ]; then\n'
                "  echo '#compdef grafana-util'\n"
                "  exit 0\n"
                "fi\n"
                "echo grafana-util test build\n",
                encoding="utf-8",
            )
            fake_binary.chmod(0o755)

            archive_path = temp_path / "grafana-utils-rust-linux-amd64-v9.9.9.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(fake_binary, arcname="grafana-util")

            bin_dir = temp_path / "bin"
            home_dir = temp_path / "home"
            completion_dir = temp_path / "completions"
            home_dir.mkdir()
            env = os.environ.copy()
            env.update(
                {
                    "ASSET_URL": archive_path.resolve().as_uri(),
                    "BIN_DIR": str(bin_dir),
                    "COMPLETION_DIR": str(completion_dir),
                    "HOME": str(home_dir),
                    "INSTALL_COMPLETION": "auto",
                    "PATH": "/usr/bin:/bin",
                    "SHELL": "/bin/zsh",
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
            completion_path = completion_dir / "_grafana-util"
            self.assertTrue(completion_path.is_file())
            self.assertEqual(completion_path.read_text(encoding="utf-8").strip(), "#compdef grafana-util")
            self.assertIn("Installed Zsh completion to", completed.stdout)

    def test_install_script_interactive_prompts_for_install_and_completion(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "grafana-util"
            fake_binary.write_text(
                "#!/bin/sh\n"
                'if [ "${1:-}" = completion ] && [ "${2:-}" = zsh ]; then\n'
                "  echo '#compdef grafana-util'\n"
                "  exit 0\n"
                "fi\n"
                "echo grafana-util test build\n",
                encoding="utf-8",
            )
            fake_binary.chmod(0o755)

            archive_path = temp_path / "grafana-utils-rust-linux-amd64-v9.9.9.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(fake_binary, arcname="grafana-util")

            bin_dir = temp_path / "interactive-bin"
            home_dir = temp_path / "home"
            completion_dir = temp_path / "interactive-completions"
            input_path = temp_path / "interactive-input.txt"
            home_dir.mkdir()
            input_path.write_text(
                f"{bin_dir}\n\n{completion_dir}\nn\n",
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update(
                {
                    "ASSET_URL": archive_path.resolve().as_uri(),
                    "HOME": str(home_dir),
                    "INSTALL_TTY": str(input_path),
                    "PATH": "/usr/bin:/bin",
                    "SHELL": "/bin/zsh",
                    "VERSION": "v9.9.9",
                }
            )

            completed = subprocess.run(
                ["sh", str(INSTALL_SCRIPT_PATH), "--interactive"],
                cwd=str(REPO_ROOT),
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 0, msg=completed.stderr)
            self.assertTrue((bin_dir / "grafana-util").is_file())
            completion_path = completion_dir / "_grafana-util"
            self.assertTrue(completion_path.is_file())
            self.assertEqual(completion_path.read_text(encoding="utf-8").strip(), "#compdef grafana-util")
            self.assertIn("Install grafana-util into", completed.stderr)
            self.assertIn("Install zsh shell completion?", completed.stderr)
            self.assertIn("Install zsh completion into", completed.stderr)
            self.assertIn("Update ~/.zshrc to load grafana-util completion?", completed.stderr)
            self.assertIn("Skipped ~/.zshrc update.", completed.stdout)

    def test_install_script_interactive_can_update_zshrc_for_completion(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "grafana-util"
            fake_binary.write_text(
                "#!/bin/sh\n"
                'if [ "${1:-}" = completion ] && [ "${2:-}" = zsh ]; then\n'
                "  echo '#compdef grafana-util'\n"
                "  exit 0\n"
                "fi\n"
                "echo grafana-util test build\n",
                encoding="utf-8",
            )
            fake_binary.chmod(0o755)

            archive_path = temp_path / "grafana-utils-rust-linux-amd64-v9.9.9.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(fake_binary, arcname="grafana-util")

            bin_dir = temp_path / "interactive-bin"
            home_dir = temp_path / "home"
            zshrc_path = home_dir / ".zshrc"
            input_path = temp_path / "interactive-input.txt"
            home_dir.mkdir()
            (home_dir / ".zcompdump").write_text("old completion cache\n", encoding="utf-8")
            (home_dir / ".zcompdump-test-host-5.9").write_text("old completion cache\n", encoding="utf-8")
            (home_dir / ".zcompdump.test-host.5.9").write_text("old completion cache\n", encoding="utf-8")
            zshrc_path.write_text(
                'export ZSH="$HOME/.oh-my-zsh"\n'
                'source "$ZSH/oh-my-zsh.sh"\n',
                encoding="utf-8",
            )
            input_path.write_text(
                f"{bin_dir}\n\n\n\n",
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update(
                {
                    "ASSET_URL": archive_path.resolve().as_uri(),
                    "HOME": str(home_dir),
                    "INSTALL_TTY": str(input_path),
                    "PATH": "/usr/bin:/bin",
                    "SHELL": "/bin/zsh",
                    "VERSION": "v9.9.9",
                }
            )

            completed = subprocess.run(
                ["sh", str(INSTALL_SCRIPT_PATH), "--interactive"],
                cwd=str(REPO_ROOT),
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 0, msg=completed.stderr)
            completion_path = home_dir / ".zfunc" / "_grafana-util"
            self.assertTrue(completion_path.is_file())
            zshrc_content = zshrc_path.read_text(encoding="utf-8")
            self.assertIn("# >>> grafana-util completion fpath >>>", zshrc_content)
            self.assertIn("# >>> grafana-util completion compdef >>>", zshrc_content)
            self.assertIn('fpath=("$HOME/.zfunc" $fpath)', zshrc_content)
            self.assertIn("compdef _grafana-util grafana-util", zshrc_content)
            self.assertLess(
                zshrc_content.index("# >>> grafana-util completion fpath >>>"),
                zshrc_content.index('source "$ZSH/oh-my-zsh.sh"'),
            )
            self.assertGreater(
                zshrc_content.index("# >>> grafana-util completion compdef >>>"),
                zshrc_content.index('source "$ZSH/oh-my-zsh.sh"'),
            )
            self.assertFalse((home_dir / ".zcompdump").exists())
            self.assertFalse((home_dir / ".zcompdump-test-host-5.9").exists())
            self.assertFalse((home_dir / ".zcompdump.test-host.5.9").exists())
            self.assertIn("Updated " + str(zshrc_path) + " to load Zsh completion.", completed.stdout)
            self.assertIn("Cleared Zsh completion cache.", completed.stdout)

    def test_install_script_updates_existing_zshrc_completion_block_once(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "grafana-util"
            fake_binary.write_text(
                "#!/bin/sh\n"
                'if [ "${1:-}" = completion ] && [ "${2:-}" = zsh ]; then\n'
                "  echo '#compdef grafana-util'\n"
                "  exit 0\n"
                "fi\n"
                "echo grafana-util test build\n",
                encoding="utf-8",
            )
            fake_binary.chmod(0o755)

            archive_path = temp_path / "grafana-utils-rust-linux-amd64-v9.9.9.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                archive.add(fake_binary, arcname="grafana-util")

            bin_dir = temp_path / "bin"
            home_dir = temp_path / "home"
            completion_dir = temp_path / "custom completions"
            input_path = temp_path / "interactive-input.txt"
            home_dir.mkdir()
            zshrc_path = home_dir / ".zshrc"
            zshrc_path.write_text(
                "# before\n"
                "# >>> grafana-util completion >>>\n"
                'fpath=("/old/path" $fpath)\n'
                "# <<< grafana-util completion <<<\n"
                "autoload -Uz compinit\n"
                "compinit\n",
                encoding="utf-8",
            )
            input_path.write_text(
                f"{bin_dir}\n\n{completion_dir}\n\n",
                encoding="utf-8",
            )
            env = os.environ.copy()
            env.update(
                {
                    "ASSET_URL": archive_path.resolve().as_uri(),
                    "HOME": str(home_dir),
                    "INSTALL_TTY": str(input_path),
                    "PATH": "/usr/bin:/bin",
                    "SHELL": "/bin/zsh",
                    "VERSION": "v9.9.9",
                }
            )

            completed = subprocess.run(
                ["sh", str(INSTALL_SCRIPT_PATH), "--interactive"],
                cwd=str(REPO_ROOT),
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 0, msg=completed.stderr)
            zshrc_content = zshrc_path.read_text(encoding="utf-8")
            self.assertNotIn("# >>> grafana-util completion >>>", zshrc_content)
            self.assertEqual(zshrc_content.count("# >>> grafana-util completion fpath >>>"), 1)
            self.assertEqual(zshrc_content.count("# >>> grafana-util completion compdef >>>"), 1)
            self.assertNotIn("/old/path", zshrc_content)
            self.assertIn('fpath=("' + str(completion_dir) + '" $fpath)', zshrc_content)
            self.assertIn("compdef _grafana-util grafana-util", zshrc_content)
            self.assertLess(
                zshrc_content.index("# >>> grafana-util completion fpath >>>"),
                zshrc_content.index("autoload -Uz compinit"),
            )
            self.assertGreater(
                zshrc_content.index("# >>> grafana-util completion compdef >>>"),
                zshrc_content.index("compinit"),
            )

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
