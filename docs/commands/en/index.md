# Command Docs

## Language

- English command reference: [current page](./index.md)
- Traditional Chinese command reference: [繁體中文逐指令說明](../zh-TW/index.md)
- English handbook: [Operator Handbook](../../user-guide/en/index.md)
- Traditional Chinese handbook: [繁體中文手冊](../../user-guide/zh-TW/index.md)

---

These pages track the current Rust CLI help for the command tree exposed by `grafana-util`.

Use these pages when you want one stable page per command or subcommand instead of a handbook chapter. The handbook explains workflow and intent; the command pages explain the concrete CLI surface.

## Output selector conventions

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

## Dashboard
- [dashboard](./dashboard.md)
- [dashboard browse](./dashboard-browse.md)
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard list](./dashboard-list.md)
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard import](./dashboard-import.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard serve](./dashboard-serve.md)
- [dashboard edit-live](./dashboard-edit-live.md)
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard list-vars](./dashboard-list-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard impact](./dashboard-impact.md)
- [dashboard history](./dashboard-history.md)
- [dashboard screenshot](./dashboard-screenshot.md)

### Compatibility Alias Pages

- [dashboard analyze (local alias)](./dashboard-analyze-export.md)
- [dashboard analyze (live alias)](./dashboard-analyze-live.md)

## Datasource
- [datasource](./datasource.md)
- [datasource types](./datasource-types.md)
- [datasource list](./datasource-list.md)
- [datasource browse](./datasource-browse.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)

## Alert

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [alert diff](./alert-diff.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
- [alert add-rule](./alert-add-rule.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
- [alert preview-route](./alert-preview-route.md)
- [alert new-rule](./alert-new-rule.md)
- [alert new-contact-point](./alert-new-contact-point.md)
- [alert new-template](./alert-new-template.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-contact-points](./alert-list-contact-points.md)
- [alert list-mute-timings](./alert-list-mute-timings.md)
- [alert list-templates](./alert-list-templates.md)

## Access

- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)

## Shared Surfaces

- [change](./change.md)
- [change inspect](./change-inspect.md)
- [change check](./change-check.md)
- [change preview](./change-preview.md)
- [change apply](./change-apply.md)
- [change advanced](./change.md#advanced)
- [change advanced subcommands (summary, plan, review, audit, bundle, promotion handoff)](./change.md#advanced)
- [overview](./overview.md)
- [overview live](./overview.md#live)
- [status](./status.md)
- [status staged](./status.md#staged)
- [status live](./status.md#live)
- [profile](./profile.md)
- [profile list](./profile.md#list)
- [profile show](./profile.md#show)
- [profile add](./profile.md#add)
- [profile example](./profile.md#example)
- [profile init](./profile.md#init)
- [snapshot](./snapshot.md)
- [snapshot export](./snapshot.md#export)
- [snapshot review](./snapshot.md#review)

## Generic Resource Query

- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)

The matching manpages live under `docs/man/grafana-util-*.1`.
