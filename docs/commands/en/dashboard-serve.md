# dashboard serve

## Purpose
Serve one or more dashboard drafts through a lightweight local preview server.

## When to use
Use this when you are iterating on one dashboard draft, one draft directory, or one generator command and want a local browser surface without publishing back to Grafana after every workspace.

## Key flags
- `--input`: local dashboard file or directory to load into the preview server.
- `--script`: external generator command whose stdout emits one dashboard JSON or YAML document, or an array of documents.
- `--script-format`: parse `--script` stdout as `json` or `yaml`.
- `--watch`: extra local files or directories to watch for reloads.
- `--no-watch`: disable background polling reloads.
- `--open-browser`: open the preview URL in your default browser after the server starts.
- `--address`, `--port`: bind address and port for the local preview server.

## Notes
- This server is a lightweight draft preview and document-inspection surface. It does not embed a full local Grafana renderer.
- `--input` and `--script` are mutually exclusive. Use `--input` for local draft files and `--script` when an external generator already produces the dashboard payload.
- Reload errors stay visible in the preview page so you can keep editing without restarting the server.

## Examples
```bash
# Purpose: Serve one local draft file.
grafana-util dashboard serve --input ./drafts/cpu-main.json --port 18080 --open-browser
```

```bash
# Purpose: Serve all dashboard drafts in one local directory.
grafana-util dashboard serve --input ./dashboards/raw
```

```bash
# Purpose: Serve one generated dashboard and watch generator inputs for reload.
grafana-util dashboard serve --script 'jsonnet dashboards/cpu.jsonnet' --watch ./dashboards --watch ./lib --port 18080
```

## Related commands
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard edit](./dashboard-edit-live.md)
