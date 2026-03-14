Grafana Utilities User Guide
============================

This guide documents the shared command surface used by the repository. The examples use the Rust source-tree entrypoint, but the same command model also applies to the installed CLI and the Python source-tree entrypoint.

1) Before You Start
-------------------

Confirm the CLI surface first so the flags in the document match your local checkout:

```bash
cargo run --bin grafana-util -- -h
cargo run --bin grafana-util -- dashboard -h
cargo run --bin grafana-util -- alert -h
cargo run --bin grafana-util -- datasource -h
cargo run --bin grafana-util -- access -h
cargo run --bin grafana-access-utils -- -h
```

Installed entrypoints:

```text
grafana-util <domain> <command> [options]
grafana-access-utils <access-command> [options]
```

Rust-specific notes:

- `grafana-util` is the primary unified CLI.
- `grafana-access-utils` is a compatibility launcher for access workflows.
- Legacy commands such as `list-dashboard` and `export-dashboard` may still exist, but new automation should use the modern subcommand layout.

2) Global Options
-----------------

Default URLs:

- `dashboard` and `datasource` default to `http://localhost:3000`
- `alert` and `access` default to `http://127.0.0.1:3000`

| Option | Purpose | Typical use |
| --- | --- | --- |
| `--url` | Grafana base URL | Any live Grafana operation |
| `--token`, `--api-token` | API token auth | Scripts and non-interactive workflows |
| `--basic-user` | Basic auth username | Org switching, admin workflows, access management |
| `--basic-password` | Basic auth password | Used with `--basic-user` |
| `--prompt-token` | Prompt for token without echo | Safer interactive usage |
| `--prompt-password` | Prompt for password without echo | Safer interactive usage |
| `--timeout` | HTTP timeout in seconds | Slow APIs or unstable networks |
| `--verify-ssl` | Enable TLS certificate verification | Production TLS environments |

### 2.1 How To Read Example Output

- `Example command` shows a practical invocation shape.
- `Example output` shows the expected format, not a guarantee that your own UIDs, names, counts, or folders will match exactly.
- Table output is best for operators.
- JSON output is best for scripts, CI, or when you need stable machine-readable fields.
- Common `ACTION` values:
  - `create`: the target does not exist yet.
  - `update`: the target already exists and would be modified.
  - `no-change`: source and destination already match.
  - `would-*`: a dry-run prediction only.
- In diff output:
  - `-` is usually the live or current value.
  - `+` is usually the exported or expected value.

### Command Domains

- Dashboard: `dashboard export`, `dashboard list`, `dashboard list-data-sources`, `dashboard import`, `dashboard diff`, `dashboard inspect-export`, `dashboard inspect-live`
- Alert: `alert export`, `alert import`, `alert diff`, `alert list-rules`, `alert list-contact-points`, `alert list-mute-timings`, `alert list-templates`
- Datasource: `datasource list`, `datasource export`, `datasource import`, `datasource diff`
- Access: `access user list`, `access user add`, `access user modify`, `access user delete`, `access team list`, `access team add`, `access team modify`, `access team delete`, `access service-account list`, `access service-account add`, `access service-account delete`, `access service-account token add`, `access service-account token delete`

Authentication exclusivity rules:

1. `--token` / `--api-token` cannot be combined with `--basic-user` / `--basic-password`.
2. `--token` / `--api-token` cannot be combined with `--prompt-token`.
3. `--basic-password` cannot be combined with `--prompt-password`.
4. `--prompt-password` requires `--basic-user`.

3) Dashboard Commands
---------------------

### 3.1 `dashboard export` (legacy `export-dashboard`)

Purpose: export live dashboards into `raw/` and `prompt/` variants.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Export root directory | Default `dashboards`; contains `raw/` and `prompt/` |
| `--page-size` | Pagination size | Increase for large estates |
| `--org-id` | Export from one explicit org | Usually requires Basic auth |
| `--all-orgs` | Export from all visible orgs | Best for central backups |
| `--flat` | Flatten folder paths | Useful for simpler git diffs |
| `--overwrite` | Replace existing files | Typical for repeatable exports |
| `--without-dashboard-raw` | Skip `raw/` | Use only if API restore is not needed |
| `--without-dashboard-prompt` | Skip `prompt/` | Use only if UI import is not needed |
| `--dry-run` | Preview files without writing them | Validate scope and paths first |
| `--progress` | Print concise progress lines | Large exports |
| `-v`, `--verbose` | Print detailed per-item output | Troubleshooting export behavior |

Example command:
```bash
cargo run --bin grafana-util -- dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite
```

Example output:
```text
Exported raw    cpu-main -> dashboards/raw/Infra/CPU__cpu-main.json
Exported prompt cpu-main -> dashboards/prompt/Infra/CPU__cpu-main.json
Exported raw    mem-main -> dashboards/raw/Infra/MEM__mem-main.json
Exported prompt mem-main -> dashboards/prompt/Infra/MEM__mem-main.json
Dashboard export completed: 2 dashboard(s), 4 file(s) written
```

How to read it:
- `raw` is the API-friendly reversible export.
- `prompt` is the UI-import-friendly variant.
- The final summary is the fastest check for missing dashboards.

### 3.2 `dashboard list` (legacy `list-dashboard`)

Purpose: list live dashboards without writing files.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--page-size` | Results per page | Increase for large estates |
| `--org-id` | Restrict to one org | Explicit org selection |
| `--all-orgs` | Aggregate visible orgs | Cross-org inventory |
| `--with-sources` | Add datasource names in table/csv | Useful for dependency checks |
| `--table` | Table output | Best for operators |
| `--csv` | CSV output | Best for spreadsheets |
| `--json` | JSON output | Best for automation |
| `--output-format table|csv|json` | Single output selector | Replaces the legacy trio |
| `--no-header` | Hide table header row | Cleaner scripting |

Example command:
```bash
cargo run --bin grafana-util -- dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --with-sources --table
```

Example output:
```text
UID              TITLE            FOLDER   TAGS        DATASOURCES
cpu-main         CPU Overview     Infra    ops,linux   prometheus-main
mem-main         Memory Overview  Infra    ops,linux   prometheus-main
latency-main     API Latency      Apps     api,prod    loki-prod
```

How to read it:
- `UID` is the most stable identity for follow-up automation.
- `FOLDER` is the fastest way to see placement.
- `DATASOURCES` is the main reason to enable `--with-sources`.

Example command (JSON):
```bash
cargo run --bin grafana-util -- dashboard list --url http://localhost:3000 --token <TOKEN> --json
```

```json
[
  {
    "uid": "cpu-main",
    "title": "CPU Overview",
    "folder": "Infra",
    "tags": ["ops", "linux"]
  }
]
```

### 3.3 `dashboard list-data-sources`

Purpose: list the datasources currently referenced in Grafana dashboard flows.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--table` | Table output | Human inspection |
| `--csv` | CSV output | Spreadsheet workflows |
| `--json` | JSON output | Automation |
| `--output-format table|csv|json` | Unified output selector | Replaces the legacy trio |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
cargo run --bin grafana-util -- dashboard list-data-sources --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

Example output:
```text
UID                NAME               TYPE         IS_DEFAULT
prom-main          prometheus-main    prometheus   true
loki-prod          loki-prod          loki         false
tempo-prod         tempo-prod         tempo        false
```

### 3.4 `dashboard import` (legacy `import-dashboard`)

Purpose: import dashboards from a `raw/` export into live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Input `raw/` directory | Must point to `raw/`, not the top-level export root |
| `--org-id` | Target org | Org-specific import |
| `--import-folder-uid` | Force destination folder uid | Controlled placement |
| `--ensure-folders` | Create missing folders | Helpful for first-time restore |
| `--replace-existing` | Overwrite matching dashboards | Standard restore mode |
| `--update-existing-only` | Update only existing dashboards | Safe partial reconcile |
| `--require-matching-folder-path` | Refuse mismatched folder paths | Prevent wrong placement |
| `--require-matching-export-org` | Enforce exported org match | Safer cross-org restore |
| `--import-message` | Dashboard version message | Audit trail |
| `--dry-run` | Preview only | Always recommended first |
| `--table` | Dry-run table output | Best operator summary |
| `--json` | Dry-run JSON output | Best for automation |
| `--output-format text|table|json` | Dry-run output mode | Unified selector |
| `--output-columns` | Column whitelist | Tailored dry-run tables |
| `--no-header` | Hide table header | Cleaner scripting |
| `--progress` | Show import progress | Large restores |
| `-v`, `--verbose` | Detailed import logs | Troubleshooting |

Example command:
```bash
cargo run --bin grafana-util -- dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing --dry-run --table
```

Example output:
```text
UID          TITLE            ACTION   DESTINATION   FOLDER
cpu-main     CPU Overview     update   existing      Infra
mem-main     Memory Overview  create   missing       Infra

Dry-run checked 2 dashboard(s)
```

How to read it:
- `ACTION=update` means the dashboard already exists and would be changed.
- `ACTION=create` means the dashboard is not present yet.
- `DESTINATION` describes the live target state, not the local directory.

### 3.5 `dashboard diff`

Purpose: compare local exported dashboards against live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | `raw/` directory to compare | Read-only comparison |
| `--import-folder-uid` | Override folder uid assumption | Useful when folder mapping differs |
| `--context-lines` | Diff context size | Increase when JSON changes are large |

Example command:
```bash
cargo run --bin grafana-util -- dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw
```

Example output:
```text
Dashboard diff found 1 differing item(s).

--- live/cpu-main
+++ export/cpu-main
@@
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

How to read it:
- Start with the summary count.
- `-` is the current live value.
- `+` is the exported expected value.

### 3.6 `dashboard inspect-export`

Purpose: analyze exported dashboards offline without calling Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | `raw/` directory | Offline analysis only |
| `--json` | JSON output | Script-friendly |
| `--table` | Table output | Operator-friendly |
| `--report` | Shortcut report mode | Faster report selection |
| `--output-format ...` | Select report family explicitly | Most flexible reporting |
| `--report-columns` | Column whitelist | Narrow report views |
| `--report-filter-datasource` | Filter by datasource | Dependency analysis |
| `--report-filter-panel-id` | Filter by panel id | Single-panel troubleshooting |
| `--help-full` | Show richer examples | Useful for report discovery |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
cargo run --bin grafana-util -- dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table
```

Example output:
```text
UID           TITLE             PANEL_COUNT   DATASOURCES
cpu-main      CPU Overview      6             prometheus-main
mem-main      Memory Overview   4             prometheus-main
latency-main  API Latency       8             loki-prod
```

### 3.7 `dashboard inspect-live`

Purpose: run the same report logic directly against live dashboards.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--page-size` | Live pagination size | Lower it if the server is slow |
| `--org-id` | Restrict to one org | Explicit org inspection |
| `--all-orgs` | Aggregate visible orgs | Cross-org inspection |
| `--json` / `--table` / `--report` / `--output-format` | Same meaning as `inspect-export` | Same reporting, but live |
| `--help-full` | Show report details | Useful during report design |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
cargo run --bin grafana-util -- dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance-json
```

Example output:
```json
[
  {
    "uid": "cpu-main",
    "title": "CPU Overview",
    "datasource_count": 1,
    "status": "ok"
  }
]
```

4) Alert Commands
-----------------

### 4.1 `alert export` (legacy `export-alert`)

Purpose: export alerting resources into `raw/` JSON files.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--output-dir` | Export root directory | Default `alerts` |
| `--flat` | Flatten subdirectories | Easier diffing in some repos |
| `--overwrite` | Replace existing files | Standard repeatable export mode |

Example command:
```bash
cargo run --bin grafana-util -- alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
```

Example output:
```text
Exported rule          alerts/raw/rules/cpu_high.json
Exported contact point alerts/raw/contact-points/oncall_webhook.json
Exported template      alerts/raw/templates/default_message.json
Alert export completed: 3 resource(s) written
```

### 4.2 `alert import` (legacy `import-alert`)

Purpose: import alerting resources from a `raw/` directory.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Alert `raw/` directory | Must point to `raw/` |
| `--replace-existing` | Update existing resources | Standard restore mode |
| `--dry-run` | Preview only | Best first pass |
| `--dashboard-uid-map` | Dashboard UID map | Fix linked alert references |
| `--panel-id-map` | Panel id map | Fix linked panel references |

Example command:
```bash
cargo run --bin grafana-util -- alert import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./alerts/raw --replace-existing --dry-run
```

Example output:
```text
kind=contact-point name=oncall-webhook action=would-update
kind=rule-group name=linux-hosts action=would-create
kind=template name=default_message action=no-change
```

How to read it:
- `would-*` values are dry-run predictions.
- `kind` tells you which resource family would change.

### 4.3 `alert diff` (legacy `diff-alert`)

Purpose: compare local alert exports against live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--diff-dir` | Raw alert directory | Read-only comparison |
| `--dashboard-uid-map` | Dashboard mapping | Stable cross-environment compare |
| `--panel-id-map` | Panel mapping | Stable cross-environment compare |

Example command:
```bash
cargo run --bin grafana-util -- alert diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./alerts/raw
```

Example output:
```text
Diff different

resource=contact-point name=oncall-webhook
- url=http://127.0.0.1/notify
+ url=http://127.0.0.1/updated
```

### 4.4 `alert list-rules`
### 4.5 `alert list-contact-points`
### 4.6 `alert list-mute-timings`
### 4.7 `alert list-templates`

Purpose: list live alerting resources.

Common output options:

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--table` | Table output | Operators |
| `--csv` | CSV output | Spreadsheet export |
| `--json` | JSON output | Automation |
| `--output-format table|csv|json` | Unified output selector | Replaces the legacy trio |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
cargo run --bin grafana-util -- alert list-rules --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

Example output:
```text
UID                 TITLE              FOLDER        CONDITION
cpu-high            CPU High           linux-hosts   A > 80
memory-pressure     Memory Pressure    linux-hosts   B > 90
api-latency         API Latency        apps-prod     C > 500
```

`alert list-contact-points` example output:
```text
UID               NAME             TYPE      DESTINATION
oncall-webhook    Oncall Webhook   webhook   http://alert.example.com/hook
slack-primary     Slack Primary    slack     #ops-alerts
```

`alert list-mute-timings` example output:
```text
NAME                 INTERVALS
maintenance-window   mon-fri 01:00-02:00
release-freeze       sat-sun 00:00-23:59
```

`alert list-templates` example output:
```text
NAME               PREVIEW
default_message    Alert: {{ .CommonLabels.alertname }}
ops_summary        [{{ .Status }}] {{ .CommonLabels.severity }}
```

5) Datasource Commands
----------------------

### 5.1 `datasource list`

Purpose: list live datasource inventory.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--table` | Table output | Operators |
| `--csv` | CSV output | Spreadsheet export |
| `--json` | JSON output | Automation |
| `--output-format table|csv|json` | Unified output selector | Replaces the legacy trio |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
cargo run --bin grafana-util -- datasource list --url http://localhost:3000 --token <TOKEN> --table
```

Example output:
```text
UID                NAME               TYPE         URL
prom-main          prometheus-main    prometheus   http://prometheus:9090
loki-prod          loki-prod          loki         http://loki:3100
tempo-prod         tempo-prod         tempo        http://tempo:3200
```

### 5.2 `datasource export`

Purpose: export datasource inventory as normalized JSON.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Export directory | Default `datasources` |
| `--overwrite` | Replace existing export files | Repeatable export runs |
| `--dry-run` | Preview only | Validate destination first |

Example command:
```bash
cargo run --bin grafana-util -- datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./datasources --overwrite
```

Example output:
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

### 5.3 `datasource import`

Purpose: import datasource inventory into live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Export root with `datasources.json` | Not a `raw/` tree |
| `--org-id` | Target org | Explicit org restore |
| `--require-matching-export-org` | Enforce export org match | Safer multi-org restore |
| `--replace-existing` | Update existing datasources | Standard restore mode |
| `--update-existing-only` | Only touch existing datasources | Safer reconcile mode |
| `--dry-run` | Preview only | Recommended first |
| `--table` | Dry-run table output | Operator summary |
| `--json` | Dry-run JSON output | Automation |
| `--output-format text|table|json` | Dry-run output selector | Unified selector |
| `--output-columns` | Column whitelist | Tailored dry-run views |
| `--no-header` | Hide table header | Cleaner scripting |
| `--progress` | Show import progress | Large imports |
| `-v`, `--verbose` | Detailed logs | Troubleshooting |

Example command:
```bash
cargo run --bin grafana-util -- datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --replace-existing --dry-run --table
```

Example output:
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```

How to read it:
- `UID` and `NAME` both matter, but automation should prefer `UID`.
- `TYPE` helps catch name collisions with wrong datasource types.

### 5.4 `datasource diff`

Purpose: compare exported datasource inventory with live Grafana.

| Option | Purpose |
| --- | --- |
| `--diff-dir` | Datasource export root directory |

Example command:
```bash
cargo run --bin grafana-util -- datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources
```

Example output:
```text
Datasource diff found 1 differing item(s).

uid=loki-prod
- url=http://loki:3100
+ url=http://loki-prod:3100
```

6) Access Commands
------------------

`group` is an alias for `team`.

### 6.1 `access user list`

Purpose: list users in org or global scope.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--scope` | `org` or `global` | Select listing scope |
| `--query` | Fuzzy match on login/email/name | Broad discovery |
| `--login` | Exact login match | Precise lookup |
| `--email` | Exact email match | Precise lookup |
| `--org-role` | Filter by org role | Permission audit |
| `--grafana-admin` | Filter by server admin status | Admin audit |
| `--with-teams` | Include team membership | Team visibility |
| `--page`, `--per-page` | Pagination | Large user sets |
| `--table`, `--csv`, `--json` | Output mode | Human vs automation |
| `--output-format table|csv|json` | Unified output selector | Replaces the legacy trio |

Example command:
```bash
cargo run --bin grafana-util -- access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --table
```

Example output:
```text
ID   LOGIN      EMAIL                NAME             ORG_ROLE   GRAFANA_ADMIN
1    admin      admin@example.com    Grafana Admin    Admin      true
7    svc-ci     ci@example.com       CI Service       Editor     false
9    alice      alice@example.com    Alice Chen       Viewer     false
```

How to read it:
- `ORG_ROLE` is org-local, not full server-admin authority.
- `GRAFANA_ADMIN=true` should normally be rare.

### 6.2 `access user add`

Purpose: create a user.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--login` | Login name | Required |
| `--email` | Email | Required |
| `--name` | Display name | Required |
| `--password` | Initial password | Required for local user creation |
| `--org-role` | Initial org role | Default role assignment |
| `--grafana-admin` | Server admin flag | Use sparingly |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login bob --email bob@example.com --name "Bob Lin" --password '<SECRET>' --org-role Editor --json
```

Example output:
```json
{
  "id": 12,
  "login": "bob",
  "email": "bob@example.com",
  "name": "Bob Lin",
  "orgRole": "Editor",
  "grafanaAdmin": false
}
```

### 6.3 `access user modify`

Purpose: update an existing user.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--user-id` / `--login` / `--email` | User locator | Choose one |
| `--set-login` | Change login | Rename account |
| `--set-email` | Change email | Contact update |
| `--set-name` | Change display name | Identity cleanup |
| `--set-password` | Reset password | Recovery or rotation |
| `--set-org-role` | Change org role | Permission changes |
| `--set-grafana-admin` | Change server admin status | Permission changes |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --set-email alice@example.com --set-org-role Editor --json
```

Example output:
```json
{
  "id": 9,
  "login": "alice",
  "result": "updated",
  "changes": ["set-org-role", "set-email"]
}
```

### 6.4 `access user delete`

Purpose: delete a user.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--user-id` / `--login` / `--email` | User locator | Choose one |
| `--scope` | `org` or `global` | Deletion scope |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login temp-user --scope global --yes --json
```

Example output:
```json
{
  "id": 14,
  "login": "temp-user",
  "scope": "global",
  "result": "deleted"
}
```

### 6.5 `access team list`

Purpose: list teams.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--query` | Fuzzy team search | Discovery |
| `--name` | Exact team name | Precise lookup |
| `--with-members` | Include members | Team audits |
| `--page`, `--per-page` | Pagination | Large orgs |
| `--table`, `--csv`, `--json` | Output mode | Human vs automation |
| `--output-format table|csv|json` | Unified output selector | Replaces the legacy trio |

Example command:
```bash
cargo run --bin grafana-util -- access team list --url http://localhost:3000 --token <TOKEN> --with-members --table
```

Example output:
```text
ID   NAME        EMAIL              MEMBERS   ADMINS
3    sre-team    sre@example.com    5         2
7    app-team    app@example.com    8         1
```

### 6.6 `access team add`

Purpose: create a team.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--name` | Team name | Required |
| `--email` | Team email | Optional metadata |
| `--member` | Initial member | Repeatable |
| `--admin` | Initial admin | Repeatable |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access team add --url http://localhost:3000 --token <TOKEN> --name platform-team --email platform@example.com --member alice --member bob --admin alice --json
```

Example output:
```json
{
  "teamId": 15,
  "name": "platform-team",
  "membersAdded": 2,
  "adminsAdded": 1
}
```

### 6.7 `access team modify`

Purpose: adjust team members and admins.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--team-id` / `--name` | Team locator | Choose one |
| `--add-member` / `--remove-member` | Member changes | Repeatable |
| `--add-admin` / `--remove-admin` | Admin changes | Repeatable |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access team modify --url http://localhost:3000 --token <TOKEN> --name platform-team --add-member carol --remove-member bob --remove-admin alice --json
```

Example output:
```json
{
  "teamId": 15,
  "name": "platform-team",
  "membersAdded": 1,
  "membersRemoved": 1,
  "adminsRemoved": 1
}
```

### 6.8 `access team delete`

Purpose: delete a team.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--team-id` / `--name` | Team locator | Choose one |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access team delete --url http://localhost:3000 --token <TOKEN> --name platform-team --yes --json
```

Example output:
```json
{
  "teamId": 15,
  "name": "platform-team",
  "result": "deleted"
}
```

### 6.9 `access service-account list`

Purpose: list service accounts.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--query` | Fuzzy name search | Discovery |
| `--page`, `--per-page` | Pagination | Large estates |
| `--table`, `--csv`, `--json` | Output mode | Human vs automation |
| `--output-format table|csv|json` | Unified output selector | Replaces the legacy trio |

Example command:
```bash
cargo run --bin grafana-util -- access service-account list --url http://localhost:3000 --token <TOKEN> --table
```

Example output:
```text
ID   NAME          ROLE     DISABLED
2    ci-bot        Editor   false
5    backup-bot    Viewer   true
```

### 6.10 `access service-account add`

Purpose: create a service account.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--name` | Service account name | Required |
| `--role` | `Viewer|Editor|Admin|None` | Default `Viewer` |
| `--disabled` | Disabled flag | Textual boolean in Rust CLI |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access service-account add --url http://localhost:3000 --token <TOKEN> --name deploy-bot --role Editor --json
```

Example output:
```json
{
  "id": 21,
  "name": "deploy-bot",
  "role": "Editor",
  "disabled": false
}
```

### 6.11 `access service-account delete`

Purpose: delete a service account.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--service-account-id` / `--name` | Locator | Choose one |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access service-account delete --url http://localhost:3000 --token <TOKEN> --name deploy-bot --yes --json
```

Example output:
```json
{
  "id": 21,
  "name": "deploy-bot",
  "result": "deleted"
}
```

### 6.12 `access service-account token add`

Purpose: create a service-account token.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--service-account-id` / `--name` | Owner locator | Choose one |
| `--token-name` | Token name | Required |
| `--seconds-to-live` | Token TTL in seconds | Optional expiry |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access service-account token add --url http://localhost:3000 --token <TOKEN> --name deploy-bot --token-name ci-token --seconds-to-live 86400 --json
```

Example output:
```json
{
  "serviceAccountId": 21,
  "tokenId": 34,
  "tokenName": "ci-token",
  "secondsToLive": 86400,
  "key": "glsa_xxxxxxxxx"
}
```

### 6.13 `access service-account token delete`

Purpose: delete a service-account token.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--service-account-id` / `--name` | Owner locator | Choose one |
| `--token-id` / `--token-name` | Token locator | Choose one |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
cargo run --bin grafana-util -- access service-account token delete --url http://localhost:3000 --token <TOKEN> --name deploy-bot --token-name ci-token --yes --json
```

Example output:
```json
{
  "serviceAccountId": 21,
  "tokenName": "ci-token",
  "result": "deleted"
}
```

7) Shared Output Rules
----------------------

| Rule | Explanation |
| --- | --- |
| Output flags are mutually exclusive | Most commands do not allow `--table`, `--csv`, `--json`, and `--output-format` together |
| Prefer dry-run first | Especially for import-like workflows |
| Org control is explicit | `--org-id` and `--all-orgs` should be used deliberately |
| Legacy commands still exist | Prefer the modern subcommand layout for new automation |
| `access group` is an alias | It maps to `access team` |

8) Common Operator Scenarios
----------------------------

### 8.1 Cross-environment dashboard migration

1. `grafana-util dashboard export --all-orgs --overwrite --export-dir ./dashboards`
2. `grafana-util dashboard import --dry-run --replace-existing --table --import-dir ./dashboards/raw`
3. Remove `--dry-run` after reviewing the output.

### 8.2 Audit only

1. Use `dashboard diff`, `datasource diff`, or `alert diff`.
2. Use `dashboard inspect-export` or `dashboard inspect-live` for structural analysis.
3. Prefer JSON output when another system will parse the results.

### 8.3 Access cleanup

1. Start with `access user list --scope global --table`.
2. Use `access user modify` for role changes.
3. Use `access team modify` for membership changes.
4. Use `access service-account` and token commands for automation identities.

9) Minimal SOP Commands
-----------------------

```bash
cargo run --bin grafana-util -- dashboard export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--all-orgs]
cargo run --bin grafana-util -- dashboard list --url <URL> --basic-user <USER> --basic-password <PASS> [--table|--csv|--json]
cargo run --bin grafana-util -- dashboard import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run]
cargo run --bin grafana-util -- dashboard diff --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw

cargo run --bin grafana-util -- alert export --url <URL> --basic-user <USER> --basic-password <PASS> --output-dir <DIR> [--overwrite]
cargo run --bin grafana-util -- alert import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run]
cargo run --bin grafana-util -- alert diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>/raw

cargo run --bin grafana-util -- datasource list --url <URL> --token <TOKEN> [--table|--csv|--json]
cargo run --bin grafana-util -- datasource export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite]
cargo run --bin grafana-util -- datasource import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR> --replace-existing [--dry-run]
cargo run --bin grafana-util -- datasource diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>

cargo run --bin grafana-util -- access user list --url <URL> --basic-user <USER> --basic-password <PASS> --scope global --table
cargo run --bin grafana-util -- access team list --url <URL> --token <TOKEN> --table
cargo run --bin grafana-util -- access service-account list --url <URL> --token <TOKEN> --table
```

10) Output and Org Control Matrix
---------------------------------

| Command | `--output-format` values | Notes |
| --- | --- | --- |
| `dashboard list` | `table/csv/json` | Replaces legacy output flags |
| `dashboard import` | `text/table/json` | Dry-run focused |
| `alert list-*` | `table/csv/json` | Shared across list commands |
| `datasource list` | `table/csv/json` | Shared list pattern |
| `datasource import` | `text/table/json` | Dry-run focused |
| `access list` commands | `table/csv/json` | Shared list pattern |

| Command | `--org-id` | `--all-orgs` |
| --- | --- | --- |
| `dashboard list` | supported | supported |
| `dashboard export` | supported | supported |
| `dashboard import` | supported | not supported |
| `datasource import` | supported | not supported |
| `alert` commands | not supported | not supported |
| `access` commands | use `--scope` instead | not supported |
