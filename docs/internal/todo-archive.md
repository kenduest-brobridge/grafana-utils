# TODO Archive

This file preserves completed and superseded items that previously lived in `TODO.md`.

`TODO.md` should stay focused on active backlog and current constraints.

Historical note:

- This archive intentionally preserves older wording from the time each item was active.
- Current preferred user-facing commands use `grafana-util ...`; older `grafana-utils ...`, wrapper-path, and compatibility-binary references below should be read as historical context, not current primary guidance.
- Current source-tree Python guidance is the unified module entrypoint `python3 -m grafana_utils ...`, not an older wrapper path.

## Completed Access And CLI Shape Work

- unified primary CLI is now `grafana-util`
- Python source-tree usage is now centered on the unified module entrypoint
- Python `grafana-access-utils` shim was removed
- Python and Rust both support access-management commands through `grafana-util access ...`
- implemented access `user list`
- implemented access `user add`
- implemented access `user modify`
- implemented access `user delete`
- implemented access `team list`
- implemented access `team add`
- implemented access `team modify`
- implemented access `team delete`
- implemented access `service-account list`
- implemented access `service-account add`
- implemented access `service-account delete`
- implemented access `service-account token add`
- implemented access `service-account token delete`
- implemented access `group` alias for `team`
- added unit tests and Docker-backed live validation for the implemented access workflows
- dashboard CLI also includes `list-data-sources` in both Python and Rust, but that is outside the remaining access-management scope once tracked in `TODO.md`

### Access Command Shape

```text
grafana-util access user list
grafana-util access user add
grafana-util access user modify
grafana-util access user delete

grafana-util access team list
grafana-util access team add
grafana-util access team modify
grafana-util access team delete

grafana-util access group list
grafana-util access group add
grafana-util access group modify
grafana-util access group delete

grafana-util access service-account list
grafana-util access service-account add
grafana-util access service-account delete
grafana-util access service-account token add
grafana-util access service-account token delete
```

### Access Notes

- `group` remains a compatibility alias for `team`
- Rust may still keep `grafana-access-utils` as a compatibility binary, but the primary command model is `grafana-util access ...`
- Python should not reintroduce a separate `grafana-access-utils` wrapper or console script

## Completed Internal Refactors

- Rust dashboard internals were split into:
  - `dashboard_cli_defs.rs`
  - `dashboard_list.rs`
  - `dashboard_export.rs`
  - `dashboard_prompt.rs`
- Rust dashboard export metadata and index documents now use typed internal structs without changing JSON output shape
- Rust `access.rs` internals were split into:
  - `access_cli_defs.rs`
  - `access_user.rs`
  - `access_team.rs`
  - `access_service_account.rs`
  - `access_render.rs`
- Rust alert internals were split further across:
  - `alert_cli_defs.rs`
  - `alert_client.rs`
  - `alert_list.rs`
  - `alert.rs` orchestration helpers
