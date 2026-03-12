# Developer Notes

This document is for maintainers. Keep `README.md` GitHub-facing and task-oriented; put implementation detail, internal tradeoffs, and maintenance notes here.

## Repository Scope

- `grafana_utils/dashboard_cli.py`: packaged dashboard export/import utility
- `grafana_utils/alert_cli.py`: packaged alerting resource export/import utility
- `grafana_utils/access_cli.py`: packaged access-management utility, currently covering `user list`, `user add`, `user modify`, `user delete`, `team list`, `team add`, `team modify`, and initial service-account commands
- `grafana_utils/http_transport.py`: shared HTTP transport adapters and transport selection
- `cmd/grafana-utils.py`: thin source-tree wrapper for the packaged dashboard CLI
- `cmd/grafana-alert-utils.py`: thin source-tree wrapper for the packaged alerting CLI
- `cmd/grafana-access-utils.py`: thin source-tree wrapper for the packaged access-management CLI
- `pyproject.toml`: build metadata, dependencies, and console-script entrypoints
- `tests/test_python_dashboard_cli.py`: dashboard Python unit tests
- `tests/test_python_alert_cli.py`: alerting Python unit tests
- `tests/test_python_packaging.py`: Python package metadata and console-script tests
- `Makefile`: shared developer shortcuts for Python wheel builds, Rust release builds, and test runs
- `scripts/test-rust-live-grafana.sh`: Docker-backed Grafana smoke test for the Rust CLIs

## Python Baseline

- Both Python entrypoints are kept parseable on Python 3.6 syntax for RHEL 8 compatibility.
- Avoid Python 3.9+ built-in generics such as `list[str]`.
- Avoid Python 3.10 union syntax such as `str | None`.
- Prefer `typing.List`, `typing.Dict`, `typing.Optional`, `typing.Set`, and `typing.Tuple`.

## Dashboard Utility

### CLI shape

- Mode selection is explicit.
- Installed commands are `grafana-utils`, `grafana-alert-utils`, and `grafana-access-utils`.
- Use `python3 cmd/grafana-utils.py list ...` to inspect live dashboard summaries.
- Use `python3 cmd/grafana-utils.py export ...` for export.
- Use `python3 cmd/grafana-utils.py import ...` for import.
- Use `python3 cmd/grafana-utils.py diff ...` for live-vs-local comparison.
- Use `python3 cmd/grafana-access-utils.py user list ...` to inspect Grafana users.
- Use `python3 cmd/grafana-access-utils.py user add ...` to create Grafana users through the server-admin API.
- Use `python3 cmd/grafana-access-utils.py user modify ...` to update Grafana users through the global and admin user APIs.
- Use `python3 cmd/grafana-access-utils.py user delete ...` to remove Grafana users from the org or globally with explicit confirmation.
- Use `python3 cmd/grafana-access-utils.py team list ...` to inspect Grafana teams.
- Use `python3 cmd/grafana-access-utils.py team add ...` to create an org-scoped Grafana team with optional initial members and admins.
- Use `python3 cmd/grafana-access-utils.py team modify ...` to change Grafana team membership and admin assignments.
- Use `python3 cmd/grafana-access-utils.py service-account ...` for org-scoped service-account operations.
- The export subcommand intentionally uses `--export-dir` instead of `--output-dir` to avoid mixing export terminology with import behavior.
- The `list` subcommand is read-only and defaults to compact `uid=<uid> name=<title> folder=<folder> folderUid=<folderUid> path=<folderTreePath>` output.
- `list --table` renders the same fields in columns and adds a `FOLDER_PATH` column.
- `list --csv` emits header `uid,name,folder,folderUid,path` with CSV escaping.
- `list --json` emits an array of objects with keys `uid`, `name`, `folder`, `folderUid`, and `path`.
- Folder tree path is resolved from `GET /api/folders/{uid}` using the folder `parents[]` chain when `folderUid` is present.

### Packaging layout

- The installable package lives under `grafana_utils/`.
- `cmd/` keeps only thin wrappers so the repo can still be used without installation.
- `pyproject.toml` exposes `grafana-utils`, `grafana-alert-utils`, and `grafana-access-utils` as console scripts.
- Base installation depends on `requests`.
- Optional extra `.[http2]` adds `httpx[http2]` for Python 3.8+ environments.

### Export variants

Dashboard export writes two variants by default:

- `raw/`: API-safe dashboard JSON intended for later `import`
- `prompt/`: Grafana web-import JSON with datasource `__inputs`

Current export suppression flags:

- `--without-dashboard-raw`
- `--without-dashboard-prompt`

The two variants serve different consumers and should not be treated as interchangeable.

Dashboard export also writes versioned `export-metadata.json` files at:

- the combined export root
- `raw/`
- `prompt/`

Those manifests use `schemaVersion` and `variant` markers so `import` and `diff` can reject directories that are not the expected raw export layout.

### Raw export intent

- Keep dashboard JSON close to Grafana's API payload.
- Preserve `uid`.
- Clear numeric `id`.
- Keep datasource references unchanged.
- Best input for `python3 cmd/grafana-utils.py import`.

### Prompt export intent

- Transform datasource references into Grafana web-import placeholders.
- Populate `__inputs`, `__requires`, and `__elements` in the shape Grafana expects.
- Intended for Grafana UI import, not for API re-import.

### Prompt export datasource pipeline

The prompt export rewrite flow is intentionally multi-stage:

1. Fetch datasource catalog from Grafana.
2. Index datasources by both `uid` and `name`.
3. Walk the dashboard tree and collect every `datasource` reference.
4. Normalize each datasource reference into a stable key.
5. Build one generated input mapping per unique datasource reference.
6. Rewrite matching dashboard refs to `${DS_*}` placeholders.
7. If every datasource resolves to the same plugin type, add Grafana's shared `$datasource` variable and collapse panel-level refs to it.

This is why prompt export needs live datasource metadata while raw export does not.

### Dashboard import constraints

- Import expects raw dashboard JSON, not prompt JSON.
- Files containing `__inputs` should be imported through Grafana web UI.
- Import can override folder destination with `--import-folder-uid`.
- Import can set the dashboard version-history message with `--import-message`.
- Import `--dry-run` predicts `would-create`, `would-update`, or `would-fail-existing` by checking the live Grafana UID first.
- `diff` compares normalized local raw payloads against live Grafana dashboard wrappers and prints a unified diff when they differ.

## Alerting Utility

### Supported resource kinds

`cmd/grafana-alert-utils.py` currently supports:

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates

The alerting export root is `alerts/raw/`, with one subdirectory per resource kind.

Default layout:

- `alerts/raw/rules/<folderUID>/<ruleGroup>/<title>__<uid>.json`
- `alerts/raw/contact-points/<name>/<name>__<uid>.json`
- `alerts/raw/mute-timings/<name>/<name>.json`
- `alerts/raw/policies/notification-policies.json`
- `alerts/raw/templates/<name>/<name>.json`

Alerting export documents and the root `index.json` carry both:

- `apiVersion`: the older tool document version marker kept for compatibility
- `schemaVersion`: the current export schema marker used by newer import and diff flows

### Import behavior by resource kind

- rules: create by default, update by `uid` when `--replace-existing` is set
- contact points: create by default, update by `uid` when `--replace-existing` is set
- mute timings: create by default, update by `name` when `--replace-existing` is set
- notification policies: always applied as one policy tree with `PUT`
- notification templates: applied with `PUT`; when `--replace-existing` is set, fetch the current template version first and send it back with the update payload
- import `--dry-run` predicts `would-create`, `would-update`, or `would-fail-existing` without mutating Grafana
- `--diff-dir` compares normalized import payloads with live provisioning resources and prints a unified diff when they differ

Template handling notes:

- Grafana template identity is the template `name`
- template list may return JSON `null`; treat that as an empty list
- template updates should strip `name` from the request body because the API path already carries the name
- without `--replace-existing`, importing an existing template should fail fast instead of silently updating it

### Alerting import shape and rejection rules

- Import accepts the tool-owned document format emitted by `cmd/grafana-alert-utils.py`
- Import accepts both current tool documents with `schemaVersion` and older tool documents that only carry `apiVersion`
- `detect_document_kind(...)` also accepts plain resource-shaped JSON for rules/contact points/mute timings/policies/templates
- Grafana official provisioning `/export` payloads are intentionally rejected for API import
- Round-trip import is only guaranteed for the tool-owned export format emitted by `cmd/grafana-alert-utils.py`
- Reject the combined `alerts/` export root on import; require callers to point at `alerts/raw/`

### Dashboard-linked alert rules

Alert rules may contain `__dashboardUid__` and `__panelId__` in annotations.

Export behavior:

- preserve the original linkage fields
- export extra linked-dashboard metadata used for import-time repair
- when the source dashboard still exists during export, enrich metadata with:
  - `dashboardTitle`
  - `folderTitle`
  - `folderUid`
  - `dashboardSlug`
  - `panelTitle`
  - `panelType`

Import behavior:

1. try the original `__dashboardUid__`
2. if `--dashboard-uid-map` is present, apply that mapping first
3. if `--panel-id-map` is present, rewrite `__panelId__` using the mapped source dashboard UID plus source panel ID
4. if the target Grafana has the mapped or original dashboard UID, stop there
5. otherwise fall back to exported dashboard metadata
6. search target dashboards by exported title, then narrow by folder title and slug
7. rewrite `__dashboardUid__` only when that fallback search resolves to exactly one dashboard

Current limitation:

- automatic fallback only rewrites `__dashboardUid__`
- `__panelId__` is preserved unless `--panel-id-map` is supplied
- panel matching is intentionally explicit; there is no heuristic panel-title-based rewrite

### Mapping file formats

Dashboard UID map:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

Panel ID map:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

Notes:

- both mapping loaders coerce keys and values to strings
- panel maps are keyed by source dashboard UID, then source panel ID
- explicit maps take precedence over fallback dashboard metadata matching

### Live validation notes

- Primary automated coverage lives in `tests/test_python_alert_cli.py`
- Container-based validation was done against Grafana `12.4.1`
- Verified round-trip coverage includes:
  - rules
  - contact points
  - mute timings
  - notification policies
  - notification templates
  - dashboard-linked rules with repaired `__dashboardUid__`

## Grafana API Endpoints Used

This section lists the Grafana HTTP API paths used by this project. It is intended as a maintainer map of what each endpoint means to Grafana and how the Python and Rust implementations use it.

### Dashboard and shared lookup APIs

| Method | Endpoint | Grafana meaning | Project usage |
| --- | --- | --- | --- |
| `GET` | `/api/search` | Search Grafana objects. In this project it is always called with `type=dash-db` plus pagination params. | List dashboards for export and search dashboards by title when repairing linked alert-rule dashboard references. |
| `GET` | `/api/dashboards/uid/{uid}` | Fetch one dashboard plus Grafana `meta` fields by dashboard UID. | Export a dashboard by UID, and inspect dashboard metadata during alert-rule linked-dashboard repair. |
| `POST` | `/api/dashboards/db` | Create or update a dashboard from the standard dashboard import payload. Grafana expects a wrapped payload such as `{dashboard, folderUid, overwrite, message}`. | Import dashboards from the tool's raw dashboard files. |
| `GET` | `/api/datasources` | List datasource definitions known to Grafana. | Build the datasource catalog used by dashboard prompt export so datasource references can be rewritten into Grafana import placeholders. |

Notes:

- No dashboard folder-management endpoint is used. Folder destination is carried through `folderUid` inside the dashboard import payload.
- The alerting utility reuses `/api/search` and `/api/dashboards/uid/{uid}` only for linked-dashboard metadata lookup and repair, not for dashboard export/import.

### Alerting provisioning APIs

| Method | Endpoint | Grafana meaning | Project usage |
| --- | --- | --- | --- |
| `GET` | `/api/v1/provisioning/alert-rules` | List all provisioned alert rules. | Export alert rules. |
| `GET` | `/api/v1/provisioning/alert-rules/{uid}` | Fetch one alert rule by UID. | Check whether a rule already exists before update/replace flows. |
| `POST` | `/api/v1/provisioning/alert-rules` | Create a new alert rule from a provisioning-style rule payload. | Import a rule when not replacing an existing one. |
| `PUT` | `/api/v1/provisioning/alert-rules/{uid}` | Replace an existing alert rule by UID. | Import a rule when `--replace-existing` is set. |
| `GET` | `/api/v1/provisioning/contact-points` | List provisioned contact points. | Export contact points and detect existing identities before updates. |
| `POST` | `/api/v1/provisioning/contact-points` | Create a new contact point. | Import a contact point when not replacing an existing one. |
| `PUT` | `/api/v1/provisioning/contact-points/{uid}` | Replace an existing contact point by UID. | Import a contact point when `--replace-existing` is set. |
| `GET` | `/api/v1/provisioning/mute-timings` | List provisioned mute timings. | Export mute timings and detect existing identities before updates. |
| `POST` | `/api/v1/provisioning/mute-timings` | Create a new mute timing. | Import a mute timing when not replacing an existing one. |
| `PUT` | `/api/v1/provisioning/mute-timings/{name}` | Replace an existing mute timing by name. | Import a mute timing when `--replace-existing` is set. |
| `GET` | `/api/v1/provisioning/policies` | Fetch the notification policy tree. Grafana models policies as one tree, not as many independent objects. | Export the policy tree. |
| `PUT` | `/api/v1/provisioning/policies` | Replace the notification policy tree. | Import the policy tree. The tool always uses `PUT` because this resource is tree-shaped. |
| `GET` | `/api/v1/provisioning/templates` | List notification templates. Grafana may return JSON `null` when none exist. | Export templates and detect existing template names. |
| `GET` | `/api/v1/provisioning/templates/{name}` | Fetch one notification template by name. | Read the current template version before a replace/update. |
| `PUT` | `/api/v1/provisioning/templates/{name}` | Replace a notification template by name. | Import or update a template. The request body intentionally omits `name` because the API path already carries the identity. |

Alerting import format notes:

- The tool accepts its own tool-owned export documents, not Grafana's official provisioning `/export` documents.
- The create/update payload shapes for these APIs are not the same as Grafana's `/export` response shape, which is why the project normalizes resources into its own round-trip format first.

## Access Utility

### Current scope

`cmd/grafana-access-utils.py` currently supports:

- `user list`
- `user add`
- `user modify`
- `user delete`
- `team list`
- `team modify`
- `team add`
- `service-account list`
- `service-account add`
- `service-account token add`

Not implemented yet:

- `team delete`
- any `group` alias commands

Current team creation command shape:

```bash
python3 cmd/grafana-access-utils.py team add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name platform-operators \
  --email platform-operators@example.com \
  --member alice@example.com \
  --admin bob@example.com
```

### Auth constraints

- `user list --scope org` may use token auth or Basic auth
- `user list --scope global` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user add` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user modify` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user delete --scope global` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user delete --scope org` may use token auth or Basic auth
- `team list` is org-scoped and may use token auth or Basic auth
- `team modify` is org-scoped and may use token auth or Basic auth
- `team add` is org-scoped and may use token auth or Basic auth
- service-account commands are org-scoped and may use token auth or Basic auth
- do not silently fall back from a token-only global request into a weaker behavior; fail early with a clear error instead

### Expected output modes

- compact text by default
- `--table`
- `--csv`
- `--json`

## Validation

Common checks:

```bash
make help
make build-python
make build-rust
make test
make test-rust-live
python3 -m pip install --no-deps --target /tmp/grafana-utils-install .
python3 -m unittest tests.test_python_dashboard_cli
python3 -m unittest tests.test_python_alert_cli
python3 -m unittest tests.test_python_access_cli
python3 -m unittest tests.test_python_packaging
python3 -m unittest -v
```

Rust live smoke test notes:

- `make test-rust-live` runs `scripts/test-rust-live-grafana.sh`
- the script defaults to `grafana/grafana:12.4.1` and binds Grafana to a random localhost port unless `GRAFANA_PORT` is set explicitly
- the script seeds one Prometheus datasource, one dashboard, and one webhook contact point
- dashboard coverage: export, prompt export datasource rewrite, diff same, diff drifted, dry-run export, dry-run import, delete-and-import restore
- alerting coverage: export, diff same, diff changed, dry-run import, update import
- useful overrides: `GRAFANA_IMAGE`, `GRAFANA_PORT`, `GRAFANA_USER`, `GRAFANA_PASSWORD`, `CARGO_BIN`

Useful CLI help checks:

```bash
grafana-utils -h
grafana-utils list -h
grafana-utils export -h
grafana-utils import -h
grafana-alert-utils -h
grafana-access-utils -h
grafana-access-utils user list -h
grafana-access-utils user add -h
grafana-access-utils user modify -h
grafana-access-utils user delete -h
grafana-access-utils team list -h
grafana-access-utils team add -h
grafana-access-utils team modify -h
grafana-access-utils service-account list -h
grafana-access-utils service-account add -h
grafana-access-utils service-account token add -h
python3 cmd/grafana-utils.py -h
python3 cmd/grafana-utils.py list -h
python3 cmd/grafana-utils.py export -h
python3 cmd/grafana-utils.py import -h
python3 cmd/grafana-alert-utils.py -h
python3 cmd/grafana-access-utils.py -h
python3 cmd/grafana-access-utils.py user list -h
python3 cmd/grafana-access-utils.py user add -h
python3 cmd/grafana-access-utils.py user modify -h
python3 cmd/grafana-access-utils.py user delete -h
python3 cmd/grafana-access-utils.py team list -h
python3 cmd/grafana-access-utils.py team add -h
python3 cmd/grafana-access-utils.py team modify -h
python3 cmd/grafana-access-utils.py service-account list -h
python3 cmd/grafana-access-utils.py service-account add -h
python3 cmd/grafana-access-utils.py service-account token add -h
```

## Documentation split

- `README.md`: public usage and high-level behavior
- `DEVELOPER.md`: maintenance notes, internal architecture, compatibility rules, and implementation tradeoffs
- `docs/internal/ai-status.md` / `docs/internal/ai-changes.md`: internal working notes only; do not treat them as public GitHub-facing documentation

## GitHub metadata updates

When updating GitHub repository description or topics for this project, use `gh api` against the REST endpoints instead of relying on `gh repo view` GraphQL lookups alone.

Known repositories:

- public: `kenduest/grafana-utils`
- private mirror: `kenduest-brobridge/grafana-utils`

Recommended sequence:

1. Check current auth:

```bash
gh auth status
```

2. Switch to the account that owns the target repo if needed:

```bash
gh auth switch -u kenduest
gh auth switch -u kenduest-brobridge
```

3. Update description with REST:

```bash
gh api repos/<owner>/grafana-utils -X PATCH \
  -f description='Python and Rust CLI tools for exporting, backing up, migrating, and re-importing Grafana dashboards and alerting resources.'
```

4. Update topics with REST:

```bash
gh api repos/<owner>/grafana-utils/topics -X PUT \
  -H 'Accept: application/vnd.github+json' \
  -f 'names[]=grafana' \
  -f 'names[]=dashboards' \
  -f 'names[]=alerting' \
  -f 'names[]=backup' \
  -f 'names[]=migration' \
  -f 'names[]=cli' \
  -f 'names[]=python' \
  -f 'names[]=rust'
```

Things to remember:

- `gh repo view <owner>/<repo>` may fail to resolve a private repo depending on the active account and GraphQL visibility, even when `gh api repos/<owner>/<repo>` works
- in `zsh`, quote each `names[]=...` argument or the shell will treat it as a glob and fail before the API call
- if one repo update returns `404`, check the active `gh` account before assuming the repo path is wrong

Documentation policy:

- keep `README.md` suitable for GitHub readers
- keep environment-specific validation logs, migration notes, and maintainer-only tradeoffs in `DEVELOPER.md`
- avoid relying on `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` for public project documentation
- if user-facing release history is needed, prefer a curated `CHANGELOG.md`
