# Technical Reference

Use this chapter when you already know the workflow and need the exact command behavior, output flags, profile rules, or secret-handling details.

This manual provides the current command surface for `grafana-util`, including profile resolution, output flags, and live/staged status entrypoints.

## Who It Is For

- Operators who know which command family they need and want exact flags or output behavior.
- Script and CI authors who need stable formatting and secret-handling rules.
- Reviewers checking which `--output-format` value fits a workflow.

## Primary Goals

- Explain connection, profile, and secret rules in one place.
- Clarify which output modes exist on which command surfaces.
- Reduce time spent jumping between individual command pages for shared behavior.

Use this chapter alongside [profile](../../commands/en/profile.md), [status](../../commands/en/status.md), [overview](../../commands/en/overview.md), and [access](../../commands/en/access.md) when you want the command-by-command surface.

---

## Output selector rules

`grafana-util` now standardizes render-format selection on `--output-format`, so it helps to learn the common patterns first.

### Standard selector: `--output-format`

Many list, review, inspect, and dry-run commands use `--output-format`.

Typical equivalents:

- `--output-format table` = `--table`
- `--output-format json` = `--json`
- `--output-format csv` = `--csv`
- `--output-format yaml` = `--yaml`
- `--output-format text` = `--text`

The long form is usually better for scripts and reusable snippets. The short form is convenient for interactive shell usage.

### Exceptions worth remembering

- not every command exposes every shorthand flag
- some commands only support one or two output shapes
- `dashboard topology` is intentionally different: it supports `text`, `json`, `mermaid`, and `dot`, and does not have shortcut flags such as `--table`
- file-writing flags such as `--output-file` or `--output` on draft/export commands mean destination paths, not render formats

If you are unsure, always trust the per-command reference page over a generic rule of thumb.

### `change` JSON documents for CI

The `change` family emits several different JSON contracts. The safest routing rule is:

1. inspect `kind`
2. confirm `schemaVersion`
3. only then branch on nested fields such as `summary`, `operations`, `checks`, or `drifts`

Fast lookups from the CLI:

- `grafana-util change --help-schema`
- `grafana-util change preview --help-schema`
- `grafana-util change apply --help-schema`
- `grafana-util change advanced audit --help-schema`

Practical mapping:

- `change inspect --output-format json` -> staged change summary or overview-style inspection output, depending on selected inputs
- `change preview --output-format json` -> `grafana-utils-sync-plan`
- `change apply --output-format json` -> `grafana-utils-sync-apply-intent`
- `change apply --execute-live --output-format json` -> live apply result
- `change advanced summary --output-format json` -> `grafana-utils-sync-summary`
- `change advanced review --output-format json` -> `grafana-utils-sync-plan`
- `change advanced audit --output-format json` -> `grafana-utils-sync-audit`
- `change advanced preflight --output-format json` -> `grafana-utils-sync-preflight`
- `change advanced assess-alerts --output-format json` -> `grafana-utils-alert-sync-plan`
- `change advanced bundle-preflight --output-format json` -> `grafana-utils-sync-bundle-preflight`
- `change advanced promotion-preflight --output-format json` -> `grafana-utils-sync-promotion-preflight`

For `grafana-utils-sync-plan`, reviewers and CI should also expect ordering metadata on the reviewed preview document itself: `ordering.mode`, `operations[].orderIndex`, `operations[].orderGroup`, `operations[].kindOrder`, and `summary.blocked_reasons`.

Use the dedicated [change command reference](../../commands/en/change.md) when you need the exact top-level keys for each document.

### `dashboard history` JSON documents for CI

`dashboard history` also exposes stable JSON contracts for automation.

For local history inspection, `dashboard history list` can also read:

- a single reusable artifact created by `dashboard history export --output ./cpu-main.history.json`
- an export tree created by `dashboard export --include-history --output-dir ./dashboards`

`dashboard history restore` remains live-only.
`dashboard history diff` can compare two historical versions from live Grafana, a reusable artifact, or export roots from different dates.

The same routing rule applies:

1. inspect `kind`
2. confirm `schemaVersion`
3. only then branch on nested fields

Fast lookups from the CLI:

- `grafana-util dashboard history --help-schema`
- `grafana-util dashboard history list --help-schema`
- `grafana-util dashboard history restore --help-schema`
- `grafana-util dashboard history diff --help-schema`
- `grafana-util dashboard history export --help-schema`

Practical mapping:

- `dashboard history list --output-format json` -> `grafana-util-dashboard-history-list`
- `dashboard history list --input-dir ./dashboards --output-format json` -> `grafana-util-dashboard-history-inventory` when no `--dashboard-uid` filter is present
- `dashboard history restore --dry-run --output-format json` -> `grafana-util-dashboard-history-restore`
- `dashboard history diff --output-format json` -> `grafana-util-dashboard-history-diff`
- `dashboard history export --output ./cpu-main.history.json` -> `grafana-util-dashboard-history-export`

Use the dedicated [dashboard history reference](../../commands/en/dashboard-history.md) when you need the exact top-level keys for each document.

### Shared diff JSON contract

`dashboard diff`, `dashboard history diff`, `alert diff`, and `datasource diff` use one shared JSON envelope in `--output-format json` mode. The safest routing rule is the same as the other machine-readable contracts:

1. inspect `kind`
2. confirm `schemaVersion`
3. then read `summary` and `rows`

Use the dedicated [shared diff JSON contract](./diff-json-contract.md) when you need the exact summary fields or row fields for each command.

CLI schema lookups:

- `grafana-util dashboard diff --help-schema`
- `grafana-util alert diff --help-schema`
- `grafana-util datasource diff --help-schema`

### `status` JSON documents for CI

`status staged` and `status live` now share one explicit machine-readable contract:

1. inspect `kind`
2. confirm `schemaVersion`
3. then read `overall`, `domains`, `topBlockers`, and `nextActions`

Stable routing:

- `kind` -> `grafana-util-project-status`
- `schemaVersion` -> `1`

CLI schema lookups:

- `grafana-util status --help-schema`
- `grafana-util status staged --help-schema`
- `grafana-util status live --help-schema`

Practical mapping:

- `status staged --output-format json` -> `grafana-util-project-status`
- `status live --output-format json` -> `grafana-util-project-status`

---

## Profile, connection, and secret handling

Profiles are repo-local. `grafana-util profile` reads and writes `grafana-util.yaml` in the current working directory, and `--profile` selects one named profile from that file.

### Recommended auth order

| Method | Best fit | Strengths | Limits / cautions |
| :--- | :--- | :--- | :--- |
| `--profile` | repeatable operator work, CI, long-lived checkouts | keeps secrets out of repeated command lines, supports env-backed and secret-store-backed secrets | requires initial setup |
| direct Basic auth | bootstrap, break-glass, global admin work | simple, works well for cross-org and admin surfaces | avoid leaving plaintext passwords in shell history; prefer `--prompt-password` |
| direct token | narrow scripted reads or scoped API actions | easy to rotate and limit | scope may block `--all-orgs`, org admin, or global admin workflows |

In practice, the normal progression is: verify connectivity with direct flags first, then move repeated URLs, usernames, and secret sources into a profile so day-to-day commands only need `--profile`.

### Secret storage modes: what they are, why they exist, and when to use them

`grafana-util` supports more than one place to hold secrets because operators usually need different tradeoffs in local development, CI, and long-lived checkouts.

| Mode | What it is | Why it is useful | Limits / cautions |
| :--- | :--- | :--- | :--- |
| `file` | plaintext secret directly in `grafana-util.yaml` | easiest to understand and edit by hand | secret lives in the config file; avoid for shared repos or routine admin use |
| `password_env` / `token_env` | secret stays in an environment variable and the profile stores only the variable name | good for CI, wrappers, and shells that already manage env injection | process environment still needs to be managed carefully |
| `os` | profile stores a reference key, while the real secret lives in the macOS Keychain or Linux Secret Service | keeps secrets out of the YAML file and shell history for day-to-day operator use | only supported on macOS and Linux; Linux also needs a working OS secret-service session |
| `encrypted-file` | profile stores a reference key and the real secret is encrypted into `.grafana-util.secrets.yaml` | portable across checkouts, good when OS secret storage is unavailable or not desirable | stronger than plaintext, but still depends on passphrase or local-key handling discipline |

Recommended order for most teams:

1. use `password_env` / `token_env` for CI and scripted automation
2. use `os` for repeated local operator work on macOS or Linux desktops
3. use `encrypted-file` when you need repo-local encrypted storage without depending on the OS secret service
4. use plaintext `file` only for demos, throwaway local labs, or explicit bootstrap cases

### macOS and Linux OS-store support

The `os` provider is platform-backed:

- macOS: stores secrets in the Keychain through the system keyring integration
- Linux: stores secrets in the desktop secret service through the system keyring integration

This is useful because your profile YAML keeps only a secret reference such as:

```yaml
password_store:
  provider: os
  key: grafana-util/profile/prod/password
```

The password itself stays outside `grafana-util.yaml`.

Important limits:

- `os` storage is only supported on macOS and Linux
- Linux server, container, or CI environments may not have a working Secret Service session
- when the OS store is unavailable, prefer `password_env`, `token_env`, or `encrypted-file`

### 1. Choose the right profile workflow
| Workflow | Purpose | When to use |
| :--- | :--- | :--- |
| `profile init` | Create a minimal starter `grafana-util.yaml`. | When you want a seed file before editing by hand. |
| `profile add` | Create or update one named profile directly. | When you want a friendly one-step setup path. |
| `profile example` | Print a fully commented reference config. | When you want a copy-edit template. |

If you set `GRAFANA_UTIL_CONFIG`, the config file moves with that path. The helper files for `encrypted-file` mode follow the same directory:

| File | Default location |
| :--- | :--- |
| `grafana-util.yaml` | current working directory, or the path given by `GRAFANA_UTIL_CONFIG` |
| `.grafana-util.secrets.yaml` | same directory as `grafana-util.yaml` |
| `.grafana-util.secrets.key` | same directory as `grafana-util.yaml` |

When `encrypted-file` uses these repo-local helper paths, `profile add` also appends them to the config-directory `.gitignore` if they are not already ignored.

### 2. Initialize, add, and list profiles
```bash
# Purpose: 2. Initialize, add, and list profiles.
grafana-util profile init --overwrite
```

```bash
# Purpose: 2. Initialize, add, and list profiles.
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
```

```bash
# Purpose: 2. Initialize, add, and list profiles.
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
```

```bash
# Purpose: 2. Initialize, add, and list profiles.
grafana-util profile list
```

```bash
# Purpose: 2. Initialize, add, and list profiles.
grafana-util profile example --mode full
```
**Expected Output:**
```text
Wrote grafana-util.yaml.
dev
prod
```
`init` creates the local config file, `add` creates a usable profile in one step, and `list` prints one resolved profile name per line. `example` prints a full commented template that you can copy and edit.

### 3. Show the resolved profile
```bash
# Purpose: 3. Show the resolved profile.
grafana-util profile show --profile prod --output-format yaml
```

```bash
# Purpose: 3. Show the resolved profile.
grafana-util profile show --profile prod --show-secrets --output-format yaml
```
**Expected Output:**
```text
name: prod
source_path: grafana-util.yaml
profile:
  url: https://grafana.example.com
  username: admin
  password_env: GRAFANA_PROD_PASSWORD
  verify_ssl: true
```
Use `show` when you need the final resolved values. `--profile` overrides the default selection rules, and `yaml` is the clearest format when you are checking auth wiring by hand.

Be careful with `--show-secrets`: it is for local inspection only. It resolves secret-store references and prints plaintext values.

### 4. Commented example output
```yaml
# Default profile used when --profile is omitted.
default_profile: dev

profiles:
  # Local demo profile using Basic auth credentials from the environment.
  dev:
    url: http://127.0.0.1:3000
    username: admin
    password_env: GRAFANA_DEV_PASSWORD
    timeout: 30
    verify_ssl: false

  # Token from the environment. Useful for scoped automation jobs.
  ci_token:
    url: https://grafana.example.com
    token_env: GRAFANA_CI_TOKEN
    timeout: 30
    verify_ssl: true

  # Plaintext example. Easy to edit, but the secret lives in grafana-util.yaml.
  prod_plaintext:
    url: https://grafana.example.com
    username: admin
    password: change-me
    verify_ssl: true

  # OS secret store example. The secret is kept in macOS Keychain or Linux Secret Service.
  prod_os_store:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: os
      key: grafana-util/profile/prod_os_store/password

  # Encrypted file with passphrase. The secret file defaults next to grafana-util.yaml.
  prod_encrypted:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: encrypted-file
      key: grafana-util/profile/prod_encrypted/password
      path: .grafana-util.secrets.yaml

  # Encrypted file without passphrase. Good for casual protection, not for local compromise resistance.
  stage_encrypted_local_key:
    url: https://grafana-stage.example.com
    username: stage-bot
    password_store:
      provider: encrypted-file
      key: grafana-util/profile/stage_encrypted_local_key/password
      path: .grafana-util.secrets.yaml
```

### 5. Daily-use examples in the common auth styles
```bash
# Purpose: 5. Daily-use examples in the common auth styles.
grafana-util status live --profile prod --output-format yaml
```

```bash
# Purpose: 5. Daily-use examples in the common auth styles.
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

```bash
# Purpose: 5. Daily-use examples in the common auth styles.
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```
Use the `--profile` form by default. Keep direct Basic auth for admin-heavy workflows and token auth for scoped automation where you understand the permission envelope.

### 6. Complete secret-handling examples

```bash
# Environment-backed password for a repeatable local profile.
export GRAFANA_PROD_PASSWORD='change-me'
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --password-env GRAFANA_PROD_PASSWORD
```

```bash
# Environment-backed password for a repeatable local profile.
grafana-util status live --profile prod --output-format yaml
```

```bash
# OS secret store for a desktop operator workflow on macOS or Linux.
grafana-util profile add prod-os --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os
```

```bash
# OS secret store for a desktop operator workflow on macOS or Linux.
grafana-util overview live --profile prod-os --output-format interactive
```

```bash
# Encrypted file with a prompted passphrase.
grafana-util profile add prod-encrypted --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file --prompt-secret-passphrase
```

```bash
# Encrypted file with a prompted passphrase.
grafana-util status live --profile prod-encrypted --output-format yaml
```

```bash
# Scoped token from the environment for automation.
export GRAFANA_CI_TOKEN='replace-me'
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
```

```bash
# Scoped token from the environment for automation.
grafana-util overview live --profile ci --output-format json
```

Why these examples matter:

- they show the same runtime commands using profile-backed secret resolution
- they keep shell examples aligned with the supported `file`, `os`, `encrypted-file`, and env-backed flows
- they make it clear that day-to-day commands do not need to repeat raw passwords when a profile is already set up

### 7. Multi-org and admin-scope caveats

- `--all-orgs` works best with `--profile` backed by admin credentials or with direct Basic auth.
- A token only sees what that token is allowed to see. Multi-org inventory, org export/import, and user or team management can return partial data or fail when token scope is narrower than the requested operation.
- Access-management surfaces such as `access org`, `access user`, `access team`, and service-account lifecycle operations commonly require broader Grafana privileges than a narrow API token carries.

### 8. Secret-storage troubleshooting

- `profile add --store-secret os` fails on macOS:
  verify that the `security` tool is available and that the local login session can access the Keychain.
- `profile add --store-secret os` fails on Linux:
  the local environment may not have a working Secret Service session. This is common in headless shells, containers, or minimal servers. Use `password_env`, `token_env`, or `encrypted-file` instead.
- `profile show --show-secrets` prints an error for a stored secret reference:
  confirm that the referenced env var exists, the OS secret store entry still exists, or the encrypted secret file and passphrase/local key are still present.
- `encrypted-file` works on one machine but not another:
  make sure the target checkout has the matching `.grafana-util.secrets.yaml` and either the same passphrase or the matching local key file.
- a profile works for normal commands but fails for `--all-orgs` or access-management workflows:
  the stored credential may be valid but too narrow. Switch to admin-backed Basic auth or an admin-capable profile.

---

## 📊 Output Formats Comparison

`grafana-util` supports explicit per-format flags plus a single `--output-format` selector. For dashboards, the current list command exposes `--json`, `--table`, `--csv`, `--yaml`, and `--output-format`.

### 0. Start with this flag map

| Situation | Syntax | Common values | Notes |
| :--- | :--- | :--- | :--- |
| Direct format selectors | `--text`, `--table`, `--csv`, `--json`, `--yaml` | `text` / `table` / `csv` / `json` / `yaml` | Common on list, review, inspect, and some dry-run mutation surfaces. |
| Single selector for common formats | `--output-format <FORMAT>` | `text` / `table` / `csv` / `json` / `yaml` | Some commands also define command-specific values such as `governance`, `governance-json`, `queries-json`, `mermaid`, or `dot`. |
| Live `status` / `overview` entrypoints | `--output-format <FORMAT>` | `table` / `csv` / `text` / `json` / `yaml` / `interactive` | These live entrypoints now use the same standard selector. |
| Write the rendered result to a file | `--output-file <PATH>` or a command-specific flag | command-specific | Common on topology, governance-gate, screenshot, and similar output-producing commands. |

### 1. Table or JSON selection
```bash
# Purpose: 1. Table or JSON selection.
grafana-util dashboard list -h
```
**Expected Output:**
```text
--text
--table
--csv
--json
--yaml
--output-format <OUTPUT_FORMAT>
```
Use `--json` for automation, `--table` for quick human review, and `--output-format` when you want to switch output with a single flag. The older `--limit` example is no longer current; the command now uses `--page-size` for fetch sizing and `--output-columns` for column selection.

### 2. Live status and overview output selectors
```bash
# Purpose: 2. Live status and overview output selectors.
grafana-util status live -h
```

```bash
# Purpose: 2. Live status and overview output selectors.
grafana-util overview live -h
```
**Expected Output:**
```text
Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.
...
--output-format <OUTPUT_FORMAT>
    Render project status as table, csv, text, json, yaml, or interactive output.

Render a live overview by delegating to the shared status live path.
...
--output-format <OUTPUT_FORMAT>
    Render project status as table, csv, text, json, yaml, or interactive output.
```
Both live entrypoints now use `--output-format`.

---

## 🗂️ Dashboard Lanes

- `raw/` is the API-safe replay/import lane.
- `prompt/` is the Grafana UI import lane.
- `dashboard export` writes the prompt lane for you.
- `migrate dashboard raw-to-prompt` converts ordinary or raw dashboard JSON into prompt JSON when you need to repair or migrate a dashboard for Grafana UI import.
- `dashboard import` consumes `raw/` or `provisioning/` input; it does not consume `prompt/`.

---

## 🤖 Automation & Scripting (CI/CD)

### 1. Filtering with `jq` (Bash/Zsh)
```bash
# Get the UID of all dashboards in organization ID 5
grafana-util dashboard list --profile prod --json | jq -r '.[] | select(.orgId == 5) | .uid'
```
This is the current JSON path for scripting. If you need fewer or different fields, use `--output-columns` instead of assuming the old sample table shape.

### 2. Handling Exit Codes
```bash
# Purpose: 2. Handling Exit Codes.
grafana-util status live --profile prod --output-format json
if [ $? -eq 2 ]; then
  echo "CRITICAL: Grafana connection blocked!"
  exit 1
fi
```

| Exit Code | Meaning |
| :---: | :--- |
| **0** | **Success**: Task completed. |
| **1** | **General Error**: Check syntax or local file permissions. |
| **2** | **Connection Blocked**: Target Grafana is down or network is rejected. |
| **3** | **Validation Failed**: Project contract or dashboard JSON is invalid. |

---
[⬅️ Previous: Operator Scenarios](scenarios.md) | [🏠 Home](index.md) | [➡️ Next: Best Practices & Recipes](recipes.md)
