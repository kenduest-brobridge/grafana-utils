import unittest
from pathlib import Path
import re


REPO_ROOT = Path(__file__).resolve().parents[2]
PUBLIC_MD_ROOTS = (
    REPO_ROOT / "README.md",
    REPO_ROOT / "README.zh-TW.md",
    REPO_ROOT / "docs" / "commands",
    REPO_ROOT / "docs" / "user-guide",
    REPO_ROOT / "docs" / "DEVELOPER.md",
    REPO_ROOT / "docs" / "internal" / "generated-docs-architecture.md",
    REPO_ROOT / "docs" / "internal" / "generated-docs-playbook.md",
    REPO_ROOT / "docs" / "internal" / "maintainer-quickstart.md",
)
COMMAND_FENCE_RE = re.compile(r"```(bash|sh|zsh|shell)\n(.*?)\n```", re.S)
COMMAND_MARKERS = (
    "grafana-util ",
    "curl ",
    "make ",
    "cargo ",
    "poetry ",
    "python3 ",
    "man ",
    "git ",
    "export ",
)
SIMPLE_SHELL_CONTROL_RE = re.compile(r"^\s*(for|do|done|if|then|fi|while|case)\b")


def iter_markdown_files():
    for root in PUBLIC_MD_ROOTS:
        if root.is_file():
            yield root
            continue
        for path in root.rglob("*.md"):
            yield path


class MarkdownCommandCommentTests(unittest.TestCase):
    def test_command_fences_start_with_purpose_comment(self):
        missing = []
        for path in sorted(iter_markdown_files()):
            text = path.read_text()
            for match in COMMAND_FENCE_RE.finditer(text):
                body = match.group(2)
                lines = [line for line in body.splitlines() if line.strip()]
                if not lines:
                    continue
                if not any(marker in body for marker in COMMAND_MARKERS):
                    continue
                if lines[0].strip().startswith("#"):
                    continue
                line_no = text[: match.start()].count("\n") + 1
                missing.append(f"{path.relative_to(REPO_ROOT)}:{line_no}")
        self.assertEqual(
            missing,
            [],
            "Command-style shell fences must start with a '# ...' purpose comment.",
        )

    def test_simple_multi_command_fences_repeat_purpose_comments(self):
        offenders = []
        for path in sorted(iter_markdown_files()):
            text = path.read_text()
            for match in COMMAND_FENCE_RE.finditer(text):
                body = match.group(2)
                lines = body.splitlines()
                if any(SIMPLE_SHELL_CONTROL_RE.match(line) for line in lines):
                    continue
                command_count = sum(
                    1 for line in lines if line.lstrip().startswith("grafana-util ")
                )
                comment_count = sum(1 for line in lines if line.lstrip().startswith("# "))
                if command_count > 1 and comment_count < command_count:
                    line_no = text[: match.start()].count("\n") + 1
                    offenders.append(
                        f"{path.relative_to(REPO_ROOT)}:{line_no} commands={command_count} comments={comment_count}"
                    )
        self.assertEqual(
            offenders,
            [],
            "Simple multi-command shell fences must give each grafana-util command its own '# ...' purpose comment.",
        )


if __name__ == "__main__":
    unittest.main()
