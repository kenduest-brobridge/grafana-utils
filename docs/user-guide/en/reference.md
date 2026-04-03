# Technical Reference Manual

This manual provides the current command surface for `grafana-util`, including profile resolution, output flags, and live/staged status entrypoints.

Use this chapter alongside [profile](../../commands/en/profile.md), [status](../../commands/en/status.md), [overview](../../commands/en/overview.md), and [access](../../commands/en/access.md) when you want the command-by-command surface.

---

## 🔐 Profile & Authentication Management

Profiles are repo-local. `grafana-util profile` reads and writes `grafana-util.yaml` in the current working directory, and `--profile` selects one named profile from that file.

### Recommended auth order

| Method | Best fit | Strengths | Limits / cautions |
| :--- | :--- | :--- | :--- |
| `--profile` | repeatable operator work, CI, long-lived checkouts | keeps secrets out of repeated command lines, supports env-backed and secret-store-backed secrets | requires initial setup |
| direct Basic auth | bootstrap, break-glass, global admin work | simple, works well for cross-org and admin surfaces | avoid leaving plaintext passwords in shell history; prefer `--prompt-password` |
| direct token | narrow scripted reads or scoped API actions | easy to rotate and limit | scope may block `--all-orgs`, org admin, or global admin workflows |

In this repo, the intended steady-state is: store connection defaults in `grafana-util.yaml`, back secrets with `password_env`, `token_env`, or a secret store, then run routine commands with `--profile`.

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

- macOS: stores secrets in the Keychain through the `security` tool
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

### 2. Initialize, add, and list profiles
```bash
grafana-util profile init --overwrite
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
grafana-util profile list
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
grafana-util profile show --profile prod --output-format yaml
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

### 5. Daily-use examples in all common auth styles
```bash
grafana-util status live --profile prod --output yaml
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```
Use the `--profile` form by default. Keep direct Basic auth for admin-heavy workflows and token auth for scoped automation where you understand the permission envelope.

### 6. Complete secret-handling examples

```bash
# Environment-backed password for a repeatable local profile.
export GRAFANA_PROD_PASSWORD='change-me'
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --password-env GRAFANA_PROD_PASSWORD
grafana-util status live --profile prod --output yaml

# OS secret store for a desktop operator workflow on macOS or Linux.
grafana-util profile add prod-os --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os
grafana-util overview live --profile prod-os --output interactive

# Encrypted file with a prompted passphrase.
grafana-util profile add prod-encrypted --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file --prompt-secret-passphrase
grafana-util status live --profile prod-encrypted --output yaml

# Scoped token from the environment for automation.
export GRAFANA_CI_TOKEN='replace-me'
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
grafana-util overview live --profile ci --output json
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

### 1. Table or JSON selection
```bash
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
grafana-util status live -h
grafana-util overview live -h
```
**Expected Output:**
```text
Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.
...
--output <OUTPUT>
    Render project status as table, csv, text, json, yaml, or interactive output.

Render a live overview by delegating to the shared status live path.
...
--output <OUTPUT>
    Render project status as table, csv, text, json, yaml, or interactive output.
```
Both live entrypoints use `--output`; they do not accept the older `--output-format` spelling.

---

## 🗂️ Dashboard Lanes

- `raw/` is the API-safe replay/import lane.
- `prompt/` is the Grafana UI import lane.
- `dashboard export` writes the prompt lane for you.
- `dashboard raw-to-prompt` converts ordinary or raw dashboard JSON into prompt JSON when you need to repair or migrate a dashboard for Grafana UI import.
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
grafana-util status live --profile prod --output json
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
