"""Shared helpers for batch item error-policy handling in Python CLIs."""

from typing import Any, Dict, List, Optional


ERROR_POLICY_ABORT = "abort"
ERROR_POLICY_CONTINUE = "continue"
ERROR_POLICY_CHOICES = (ERROR_POLICY_ABORT, ERROR_POLICY_CONTINUE)


def add_error_policy_argument(parser, subject: str) -> None:
    """Add one shared item-level batch error-policy flag to a parser."""
    parser.add_argument(
        "--error-policy",
        choices=ERROR_POLICY_CHOICES,
        default=ERROR_POLICY_ABORT,
        help=(
            "Control item-level %s batch errors. Default: abort on the first failed "
            "%s item; use 'continue' to record the failure, keep processing the "
            "remaining %s items, and still return a non-zero exit status if any "
            "items failed."
        )
        % (subject, subject, subject),
    )


def should_continue_on_item_error(args) -> bool:
    """Return whether one batch loop should continue after an item-level error."""
    return getattr(args, "error_policy", ERROR_POLICY_ABORT) == ERROR_POLICY_CONTINUE


def build_item_failure(
    item_kind: str,
    item_identity: str,
    item_source: str,
    exc: Exception,
) -> Dict[str, str]:
    """Normalize one item-level failure record."""
    return {
        "kind": str(item_kind or "item"),
        "identity": str(item_identity or "-"),
        "source": str(item_source or "-"),
        "error": str(exc),
    }


def append_item_failure(
    failures: List[Dict[str, str]],
    item_kind: str,
    item_identity: str,
    item_source: str,
    exc: Exception,
) -> Dict[str, str]:
    """Append one normalized item failure and return it."""
    failure = build_item_failure(item_kind, item_identity, item_source, exc)
    failures.append(failure)
    return failure


def summarize_item_failures(
    failures: List[Dict[str, str]],
    processed: int,
    succeeded: int,
    skipped: int = 0,
) -> Dict[str, int]:
    """Build a compact numeric summary for batch item processing."""
    return {
        "processed": int(processed),
        "succeeded": int(succeeded),
        "skipped": int(skipped),
        "failed": len(failures),
    }


def summarize_item_failures_with_extra(
    failures: List[Dict[str, str]],
    processed: int,
    succeeded: int,
    skipped: int = 0,
    extra: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Build a compact summary and merge command-specific counters."""
    summary = summarize_item_failures(
        failures=failures,
        processed=processed,
        succeeded=succeeded,
        skipped=skipped,
    )
    if extra:
        for key, value in extra.items():
            summary[str(key)] = value
    return summary
