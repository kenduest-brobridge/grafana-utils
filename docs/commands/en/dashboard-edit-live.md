# dashboard edit-live

## Purpose
Fetch one live dashboard into an external editor with a safe local-draft default.

## When to use
Use this when Grafana already has the closest source dashboard and you want to edit that payload directly, but you do not want the default path to write straight back to Grafana.

## Key flags
- `--dashboard-uid`: live Grafana dashboard UID to edit.
- `--output`: local draft path to write after editing. When omitted, the default is `./<uid>.edited.json`.
- `--apply-live`: write the edited payload back to Grafana instead of writing a local draft.
- `--yes`: required with `--apply-live` because it mutates live Grafana.
- `--message`: revision message used when `--apply-live` writes back to Grafana.
- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`: live Grafana connection settings.

## Notes
- The command opens `$VISUAL`, then `$EDITOR`, then falls back to `vi`.
- Without `--apply-live`, this command always writes a local draft after the edit session.
- After editing, the command prints a review summary that includes blocking validation issues and the suggested next action.
- `--apply-live` only proceeds when the edited draft keeps `dashboard.id` null, preserves the source dashboard UID, and has no blocking review issues.
- The edited payload must still contain `dashboard.uid`.

## Examples
```bash
# Purpose: Edit one live dashboard and keep the result as a local draft.
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json
```

```bash
# Purpose: Edit one live dashboard and let the default output path be ./cpu-main.edited.json.
grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main
```

```bash
# Purpose: Edit one live dashboard and explicitly write the result back to Grafana.
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --apply-live --yes --message 'Hotfix CPU dashboard'
```

## Related commands
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard publish](./dashboard-publish.md)
