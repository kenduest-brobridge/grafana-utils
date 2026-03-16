#!/usr/bin/env python3
"""Thin wrapper for the dashboard governance gate."""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from grafana_utils.dashboard_governance_gate import main


if __name__ == "__main__":
    raise SystemExit(main())
