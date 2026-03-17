"""Module entrypoint for running the unified Grafana CLI from a repo checkout."""

import sys

from .unified_cli import main

if __name__ == "__main__":
    sys.exit(main())
