# Command Docs

These pages track the current Rust CLI help for the command tree exposed by `grafana-util`.

Use these pages when you want one stable page per command or subcommand instead of a handbook chapter. The handbook explains workflow and intent; the command pages explain the concrete CLI surface.

If you prefer `man` format, render [grafana-util(1)](../../man/grafana-util.1) locally with `man ./docs/man/grafana-util.1` on macOS or `man -l docs/man/grafana-util.1` on GNU/Linux.
The checked-in `docs/man/*.1` files are generated from these English command pages via `python3 scripts/generate_manpages.py`.
The checked-in `docs/html/commands/en/*.html` files are generated from the same source via `python3 scripts/generate_command_html.py`.

## Dashboard
- [dashboard](./dashboard.md)
- [dashboard browse](./dashboard-browse.md)
- [dashboard get](./dashboard-get.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard list](./dashboard-list.md)
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard import](./dashboard-import.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard screenshot](./dashboard-screenshot.md)

## Datasource
- [datasource](./datasource.md)
- [datasource types](./datasource-types.md)
- [datasource list](./datasource-list.md)
- [datasource browse](./datasource-browse.md)
- [datasource inspect-export](./datasource-inspect-export.md)
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
- [overview](./overview.md)
- [status](./status.md)
- [profile](./profile.md)
- [snapshot](./snapshot.md)

The matching manpages live under `docs/man/grafana-util-*.1`.
