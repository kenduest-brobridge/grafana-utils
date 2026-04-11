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

- [version](./version.md): confirm the installed binary and machine-readable version details
- [status](./status.md): read-only status, overview, snapshot, and resource queries
- [config](./config.md): repo-local configuration workflows and profile management
- [export](./export.md): common backup and local-inventory capture
- [workspace](./workspace.md): scan, test, preview, package, and apply local Grafana workspaces
- [dashboard](./dashboard.md): browse, get, clone, export/import, summary, dependencies, policy, and screenshot workflows
- [alert](./alert.md): alert inventory, authoring, review, and apply workflows
- [datasource](./datasource.md): datasource inventory and lifecycle workflows
- [access](./access.md): user, team, org, and service-account workflows

If older notes mention removed roots, use the current task names instead:

- `profile ...` from older notes -> `grafana-util config profile ...`
- `resource ...` from older notes -> `grafana-util status resource ...`
- legacy overview root -> `grafana-util status overview ...`
- `snapshot ...` from older notes -> `grafana-util status snapshot ...`

## Which command should I use?

| Need | Start with |
| :--- | :--- |
| Confirm the installed binary or scriptable version | `grafana-util version` |
| Check that Grafana is reachable | `grafana-util status live` |
| See the live estate as a human | `grafana-util status overview live` |
| Save connection defaults | `grafana-util config profile` |
| Export a backup | `grafana-util export dashboard` / `export alert` / `export datasource` |
| Review a local change package | `grafana-util workspace scan` then `workspace preview` |
| Inspect dashboards deeply | `grafana-util dashboard summary` / `dashboard diff` |
| Query one resource generically | `grafana-util status resource describe|list|get` |
| Export or review a snapshot bundle | `grafana-util status snapshot export|review` |
| Manage users, teams, orgs, or service accounts | `grafana-util access ...` |

## Common Tasks

- [version](./version.md)
- [workspace](./workspace.md)
- [workspace scan](./workspace-scan.md)
- [workspace test](./workspace-test.md)
- [workspace preview](./workspace-preview.md)
- [workspace apply](./workspace-apply.md)
- [export](./export.md)
- [status](./status.md)
- [resource queries](./resource.md)
- [snapshot bundles](./snapshot.md)
- [dashboard convert raw-to-prompt](./dashboard-convert-raw-to-prompt.md)
- `version`
- `export dashboard`
- `export alert`
- `export datasource`
- `export access user|org|team|service-account`
- `status live`
- `status staged`
- `status overview`
- `status snapshot export|review`
- `status resource describe|kinds|list|get`
- `config profile`

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
- [resource queries](./resource.md)
- [snapshot bundles](./snapshot.md)

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
- `dashboard dependencies` is different: it supports `text`, `json`, `mermaid`, and `dot`, but it does not have shortcut flags such as `--table`
- destination-path flags such as `--output-file` or `--output` on draft/export commands are not render-format selectors

If you are unsure, treat the per-command page as authoritative for that exact command surface.

If you prefer `man` format, render [grafana-util(1)](../../man/grafana-util.1) locally with `man ./docs/man/grafana-util.1` on macOS or `man -l docs/man/grafana-util.1` on GNU/Linux.
The checked-in `docs/man/*.1` files are generated from these English command pages via `python3 scripts/generate_manpages.py`.
The checked-in `docs/html/commands/en/*.html` files are generated from the same source via `python3 scripts/generate_command_html.py`.
