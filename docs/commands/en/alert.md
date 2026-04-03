# `grafana-util alert`

## Purpose

Run the alerting command surface for exporting, importing, diffing, planning, applying, deleting, authoring, and listing Grafana alert resources.

## When to use

- Export local alert bundles from Grafana.
- Import or diff alert bundles against live Grafana state.
- Build and apply a reviewed alert management plan.
- Author staged rules, contact points, routes, and templates.
- List live alert rules, contact points, mute timings, and templates.

## Key flags

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`
- Use the nested subcommands for `export`, `import`, `diff`, `plan`, `apply`, `delete`, `add-rule`, `clone-rule`, `add-contact-point`, `set-route`, `preview-route`, `new-rule`, `new-contact-point`, `new-template`, `list-rules`, `list-contact-points`, `list-mute-timings`, and `list-templates`.

## Auth notes

- Prefer `--profile` for normal alert review and apply loops.
- Use Basic auth when you need broader org visibility or admin-backed inventory.
- Token auth works best for scoped single-org reads or automation where the token permissions are already well understood.

## Examples

```bash
grafana-util alert list-rules --profile prod --json
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --flat
```

## Related commands

- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [alert diff](./alert-diff.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
- [access](./access.md)
