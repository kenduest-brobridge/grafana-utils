# TODO

## Status

### Done

- unified primary CLI is now `grafana-utils`
- Python source-tree wrapper is now `python/grafana-utils.py`
- Python `grafana-access-utils` shim was removed
- Python and Rust both support access-management commands through `grafana-utils access ...`
- implemented access `user list`
- implemented access `user add`
- implemented access `user modify`
- implemented access `user delete`
- implemented access `team list`
- implemented access `team add`
- implemented access `team modify`
- implemented access `service-account list`
- implemented access `service-account add`
- implemented access `service-account token add`
- added unit tests and Docker-backed live validation for the implemented access workflows
- dashboard CLI also includes `list-data-sources` in both Python and Rust, but that is outside the remaining access-management scope tracked below
- Rust dashboard internals were split into:
  - `dashboard_cli_defs.rs`
  - `dashboard_list.rs`
  - `dashboard_export.rs`
  - `dashboard_prompt.rs`
- Rust dashboard export metadata and index documents now use typed internal structs without changing JSON output shape

### In Progress

- access-management CLI exists in both Python and Rust, but only part of the planned access surface is implemented
- auth preflight is implemented for current commands, but not yet for the remaining planned mutating commands
- service-account support exists, but only the initial list/add/token-add slice is implemented

### Next

- `team delete`
- `group` alias for `team`
- `service-account delete`
- `service-account token delete`
- split oversized Rust `access.rs`
- split oversized Rust `alert.rs`

## Remaining Access Work

Current implementation status:

- `user list`: done
- `user add`: done
- `user modify`: done
- `user delete`: done
- `team list`: done
- `team add`: done
- `team modify`: done
- `team delete`: not started
- `service-account list`: done
- `service-account add`: done
- `service-account token add`: done
- `service-account delete`: not started
- `service-account token delete`: not started
- `group` alias: not started

Recommended user-facing command shape:

```text
grafana-utils access user list
grafana-utils access user add
grafana-utils access user modify
grafana-utils access user delete

grafana-utils access team list
grafana-utils access team add
grafana-utils access team modify
grafana-utils access team delete

grafana-utils access group list
grafana-utils access group add
grafana-utils access group modify
grafana-utils access group delete

grafana-utils access service-account list
grafana-utils access service-account add
grafana-utils access service-account delete
grafana-utils access service-account token add
grafana-utils access service-account token delete
```

Notes:

- `group` should remain a compatibility alias for `team`
- Rust may still keep `grafana-access-utils` as a compatibility binary, but the primary command model is `grafana-utils access ...`
- Python should not reintroduce a separate `grafana-access-utils` wrapper or console script

## Shared Access Parameters

Currently implemented:

- `--url`
- `--token`
- `--basic-user`
- `--basic-password`
- `--prompt-password`
- `--org-id`
- `--json`
- `--csv`
- `--table`

Still not implemented:

- `--insecure`
- `--ca-cert`

## Authentication Rules

Current implementation status:

- `user list --scope org`: token or Basic auth
- `user list --scope global`: Basic auth only
- `user list --with-teams`: Basic auth only
- `user add`: Basic auth only
- `user modify`: Basic auth only
- `user delete --scope global`: Basic auth only
- `user delete --scope org`: token or Basic auth
- `team list`: token or Basic auth
- `team add`: token or Basic auth
- `team modify`: token or Basic auth
- `service-account list`: token or Basic auth
- `service-account add`: token or Basic auth
- `service-account token add`: token or Basic auth
- remaining planned commands still need explicit per-command auth preflight

Rules to keep:

- if `--token` is provided, treat it as the primary authentication input unless the command explicitly requires Basic auth
- only require `--basic-user` and `--basic-password` for operations that truly need Basic auth
- reject mixed auth inputs unless the command has a specific, documented reason to support them
- keep prompted password support aligned with dashboard and alert auth behavior

## Rust Refactor Backlog

### `access.rs`

Recommended first split target.

Reason:

- lower risk than `alert.rs`
- clearer responsibility boundaries
- already large enough that review and maintenance cost are rising

Recommended split:

- `access_cli_defs.rs`
- `access_user.rs`
- `access_team.rs`
- `access_service_account.rs`
- `access_render.rs`
- keep `access.rs` as orchestration and shared helpers

### `alert.rs`

Recommended second split target.

Reason:

- still oversized
- import/export/diff logic is feature-rich and harder to navigate
- resource-specific logic can be isolated further

Recommended split:

- `alert_cli_defs.rs`
- `alert_export.rs`
- `alert_import.rs`
- `alert_diff.rs`
- `alert_list.rs`
- keep `alert.rs` as orchestration and shared helpers

## Priority Order

1. `team delete`
2. `service-account delete`
3. `service-account token delete`
4. `group` alias
5. split Rust `access.rs`
6. split Rust `alert.rs`
