# 🚀 Getting Started

This guide covers the current first-run setup for `grafana-util`.

The most important design rule to understand up front is that the CLI supports several connection patterns. You can:

- pass the Grafana URL and auth flags directly on a command
- prompt for a password or token without echoing it
- let environment variables supply the auth values
- store repeatable defaults in a repo-local profile and reuse them with `--profile`

Profiles are there to simplify repeated work. They are not the only way to start.

For the exact flags behind this chapter, keep [profile](../../commands/en/profile.md), [status](../../commands/en/status.md), and [overview](../../commands/en/overview.md) open beside it.

---

## 📋 Step 1: Installation

### Download and Install
```bash
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash
```

### Verify Version
```bash
grafana-util --version
```
**Expected Output:**
```text
grafana-util 0.7.4
```
This confirms that the binary is on your `PATH` and matches the checked-in release.

---

## 📋 Step 2: Connection Patterns And Profile Files

Profile workflows are repo-local. `grafana-util profile` works against `grafana-util.yaml` in the current working directory by default, or against the file pointed to by `GRAFANA_UTIL_CONFIG`.

### Auth modes at a glance

`grafana-util` can read connection settings from direct flags, prompt-based input, environment variables, or a repo-local profile. Use the auth modes in this order:

| Pattern | Best for | Example |
| :--- | :--- | :--- |
| direct Basic auth | quick local checks, bootstrap, admin-only workflows | `grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml` |
| `--profile` | daily operator workflows and CI jobs once the connection is proven | `grafana-util status live --profile prod --output yaml` |
| direct token | narrow API automation that stays inside one org or one scoped permission set | `grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output yaml` |

Environment variables can supply the same auth without repeating sensitive values on every command:

- `GRAFANA_USERNAME`
- `GRAFANA_PASSWORD`
- `GRAFANA_API_TOKEN`

For repeatable work, prefer storing those references in a profile such as `password_env: GRAFANA_PROD_PASSWORD` or `token_env: GRAFANA_DEV_TOKEN` instead of repeating raw secrets on every command line.

### Start direct, then simplify

For a first run, the cleanest mental model is:

1. run one direct read-only command with `--url` plus either Basic auth or token auth
2. once that works, move the repeatable parts into a profile
3. keep using `--profile` for normal day-to-day work

### 1. Pick how you want to create profiles
```bash
grafana-util profile init --overwrite
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
grafana-util profile example --mode full
```
`profile init` creates a minimal starter `grafana-util.yaml`. `profile add` can create a reusable Basic-auth or token-backed profile in one step, and `profile example` prints a fully commented reference template that you can copy and edit.

If you are still proving basic connectivity, you can do that before any profile work:

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml
```

Then translate that same connection into a reusable profile:

```bash
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
grafana-util status live --profile dev --output yaml
```

By default, the config file lives next to your current checkout. If you point `GRAFANA_UTIL_CONFIG` somewhere else, the helper files follow that config directory:

| File | Default location | Purpose |
| :--- | :--- | :--- |
| `grafana-util.yaml` | current working directory, or the path given by `GRAFANA_UTIL_CONFIG` | repo-local profile definitions |
| `.grafana-util.secrets.yaml` | same directory as `grafana-util.yaml` | encrypted secret store used by `encrypted-file` mode |
| `.grafana-util.secrets.key` | same directory as `grafana-util.yaml` | local key file used by `encrypted-file` without a passphrase |

### 2. List the profiles in the config file
```bash
grafana-util profile list
```
**Expected Output:**
```text
dev
prod
```
On a freshly initialized config, `profile list` prints one discovered profile name per line.

Use the [profile](../../commands/en/profile.md) command reference when you want the flag-by-flag auth rules.

### 3. Inspect one resolved profile
```bash
grafana-util profile show --profile prod --output-format yaml
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
Use `--profile` when you want to override the default-selection rules, and `yaml` when you want the resolved fields in a readable form.

---

## 📋 Step 3: First Read-Only Checks

Once a profile file exists, use read-only commands to confirm the current command shape before you touch live data.

### 1. Project Status Entry Point
```bash
grafana-util status live -h
```
**Expected Output:**
```text
Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.

Usage: grafana-util status live [OPTIONS]

Options:
      --profile <PROFILE>
          Load connection defaults from the selected repo-local profile in grafana-util.yaml.
      --url <URL>
          Grafana base URL. [default: http://localhost:3000]
```
`status live` queries Grafana directly, and its output selector is `--output`, not `--output-format`.

### 2. Overview Entry Point
```bash
grafana-util overview live -h
```
**Expected Output:**
```text
Render a live overview by delegating to the shared status live path.

Examples:
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output interactive
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output yaml
```
`overview live` is a thin wrapper over shared live status. Use `--output yaml` for a readable summary and `--output interactive` for the TUI workbench.

### 3. Run the same read-only check in both common auth styles
```bash
grafana-util overview live --profile prod --output yaml
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output interactive
```
Use the profile form for normal repeatable work. Keep the direct Basic-auth form for bootstrap, break-glass access, or admin-only workflows when you are not ready to create a profile yet.

If your shell already exports auth variables, the same read can stay short without creating a profile first:

```bash
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
grafana-util overview live --url http://localhost:3000 --output yaml
```

### 4. Know the common token limitation

Token auth can be enough for single-org read flows, but multi-org or admin-scoped operations often need a user session or Basic auth with broader Grafana privileges.

- `--all-orgs` inventory and export flows are safest with `--profile` backed by admin credentials or with direct Basic auth.
- Org, user, team, and service account management commonly needs admin-level credentials and may not work with a narrow API token.
- When a token cannot see all target orgs, the command output is limited by that token's scope even if the flags ask for a broader view.

---

## 🖥️ Interactive Mode (TUI)

`grafana-util dashboard browse` opens the live dashboard tree in a terminal UI. `overview live --output interactive` opens the interactive overview mode.

---
[🏠 Home](index.md) | [➡️ Next: Architecture & Design](architecture.md)
