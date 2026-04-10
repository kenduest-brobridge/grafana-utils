# Command Docs

## Language

- English command reference: [current page](./index.md)
- Traditional Chinese command reference: [繁體中文逐指令說明](../zh-TW/index.md)
- English handbook: [Operator Handbook](../../user-guide/en/index.md)
- Traditional Chinese handbook: [繁體中文手冊](../../user-guide/zh-TW/index.md)

---

These pages track the current Rust CLI help for the command tree exposed by `grafana-util`.

Use these pages when you want one stable page per command or subcommand instead of a handbook chapter. The handbook explains workflow and intent; the command pages explain the concrete CLI surface.

## Start Here

The public first-run CLI is organized around a small task-first surface:

- `observe`: read-only status, overview, snapshot, and resource queries
- `export`: common backup and local-inventory capture
- `change`: review-first staged change workflows
- `config profile`: repo-local connection and secret management
- `advanced`: domain-heavy workflows once you already know the subsystem

Compatibility roots such as `dashboard`, `alert`, `datasource`, and `access` still exist, but they are no longer the primary onboarding path.
Older `status`, `overview`, and `profile` pages are kept as compatibility references only; use `observe` and `config profile` as the public entrypoints.

## Common Tasks

- [change](./change.md)
- [change inspect](./change-inspect.md)
- [change check](./change-check.md)
- [change preview](./change-preview.md)
- [change apply](./change-apply.md)
- [export](./export.md)
- `export dashboard`
- `export alert`
- `export datasource`
- `export access user|org|team|service-account`
- `observe live`
- `observe staged`
- `observe overview`
- `observe snapshot`
- `observe resource describe|kinds|list|get`
- `config profile`

## Advanced Tasks

- [advanced](./advanced.md)
- `advanced dashboard live|draft|sync|analyze|capture`
- `advanced alert live|migrate|author|scaffold|change`
- `advanced datasource types|list|browse|add|modify|delete|export|import|diff`
- `advanced access user|org|team|service-account`
- [migrate](./migrate.md)

## Domain Reference

- [dashboard](./dashboard.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [datasource](./datasource.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)

## Output Selector Conventions

Many list, review, and dry-run commands support both a long output selector and one or more direct shorthand flags.

Typical patterns:

- `--output-format table` is usually equivalent to `--table`
- `--output-format json` is usually equivalent to `--json`
- `--output-format csv` is usually equivalent to `--csv`
- `--output-format yaml` is usually equivalent to `--yaml`
- `--output-format text` is usually equivalent to `--text`

Use the long form when you want one explicit flag that is easy to templatize in scripts. Use the short form when you want a faster interactive command line.

Important exceptions:

- some commands only expose a subset of shortcuts
- `dashboard topology` is different: it supports `text`, `json`, `mermaid`, and `dot`, but it does not have shortcut flags such as `--table`
- destination-path flags such as `--output-file` or `--output` on draft/export commands are not render-format selectors

If you are unsure, treat the per-command page as authoritative for that exact command surface.

If you prefer `man` format, render [grafana-util(1)](../../man/grafana-util.1) locally with `man ./docs/man/grafana-util.1` on macOS or `man -l docs/man/grafana-util.1` on GNU/Linux.
The checked-in `docs/man/*.1` files are generated from these English command pages via `python3 scripts/generate_manpages.py`.
The checked-in `docs/html/commands/en/*.html` files are generated from the same source via `python3 scripts/generate_command_html.py`.
