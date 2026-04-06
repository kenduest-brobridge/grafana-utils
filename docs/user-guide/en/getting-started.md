# 🚀 Getting Started

This guide is for the first time you need to install `grafana-util`, prove that it can reach Grafana, and decide whether a direct command, env-backed auth, or a repo-local profile is the cleanest starting point.

## Who It Is For

- Someone installing `grafana-util` for the first time.
- An operator validating connectivity before any live mutation.
- A teammate deciding whether direct flags, environment variables, or a profile should be the cleanest default.

## Primary Goals

- Install the binary and confirm where it lands.
- Prove one safe live read against Grafana.
- Move repeated connection details into a profile only after the direct path works.

The most important design rule to understand up front is that the CLI supports several connection patterns. You can:

- pass the Grafana URL and auth flags directly on a command
- prompt for a password or token without echoing it
- let environment variables supply the auth values
- store repeatable defaults in a repo-local profile and reuse them with `--profile`

Profiles are there to simplify repeated work. They are not the only way to start, and they should not block a first connectivity check.

## Before / After

- Before: every command line had to repeat the Grafana URL and auth flags.
- After: you can prove the connection directly once, then move the repeatable parts into a profile.

## What success looks like

- The binary is installed and reachable from your shell.
- One direct live read succeeds.
- You know whether the next step should be `--profile`, env-backed auth, or a one-off bootstrap command.

## Failure checks

- If the binary is not on `PATH`, fix the install step before trying to use profiles.
- If a direct live read fails, do not move on to mutation workflows yet.
- If the profile does not resolve the fields you expect, inspect the profile file and the env-backed secret source first.

## What success looks like in the first 10 minutes

By the end of this chapter, a first successful run should look like this:

- the binary is on `PATH`
- one direct live read succeeds
- you know whether you are using Basic auth, token auth, env-backed auth, or `--profile`
- one repo-local profile works for the same target
- you know whether the next stop is dashboards, alerts, access, or automation

If you cannot reach that state yet, stop at the first failing read-only command and use [Troubleshooting](troubleshooting.md) before moving into mutation workflows.

For the exact flags behind this chapter, keep [profile](../../commands/en/profile.md), [status](../../commands/en/status.md), and [overview](../../commands/en/overview.md) open beside it.

---

## 📋 Step 1: Installation

### Download and Install
```bash
# Purpose: Download and Install.
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

If you want one fixed release or one explicit install directory, the same script also supports:

```bash
# Purpose: Install one pinned release into one explicit binary directory.
VERSION=0.8.0 BIN_DIR="$HOME/.local/bin" \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

The installer uses `BIN_DIR` when you set it. Otherwise it tries `/usr/local/bin` when that directory is writable, then falls back to `$HOME/.local/bin`.

If the chosen install directory is not already on `PATH`, the installer prints the exact shell snippet to add it for `zsh` or `bash`. You can also inspect the contract first with:

```bash
# Purpose: Show install script options, BIN_DIR behavior, and PATH setup notes.
sh ./scripts/install.sh --help
```

### Verify Version
```bash
# Purpose: Verify Version.
grafana-util --version
```
**Expected Output:**
```text
grafana-util 0.8.0
```
This confirms that the binary is on your `PATH` and matches the checked-in release.

---

## 📋 Step 2: Connection Patterns And Profile Files

Profile workflows are repo-local. `grafana-util profile` works against `grafana-util.yaml` in the current working directory by default, or against the file pointed to by `GRAFANA_UTIL_CONFIG`.

### Auth modes at a glance

`grafana-util` can read connection settings from direct flags, prompt-based input, environment variables, or a repo-local profile. Use the auth modes in this order:

| Pattern | Best for | Example |
| :--- | :--- | :--- |
| direct Basic auth | quick local checks, bootstrap, admin-only workflows | `grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml` |
| `--profile` | daily operator workflows and CI jobs once the connection is proven | `grafana-util status live --profile prod --output-format yaml` |
| direct token | narrow API automation that stays inside one org or one scoped permission set | `grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format yaml` |

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
# Purpose: 1. Pick how you want to create profiles.
grafana-util profile init --overwrite
```

```bash
# Purpose: 1. Pick how you want to create profiles.
grafana-util profile add dev \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --prompt-password
```

```bash
# Purpose: 1. Pick how you want to create profiles.
grafana-util profile add ci \
  --url https://grafana.example.com \
  --token-env GRAFANA_CI_TOKEN \
  --store-secret os
```

```bash
# Purpose: 1. Pick how you want to create profiles.
grafana-util profile example --mode full
```
`profile init` creates a minimal starter `grafana-util.yaml`. `profile add` can create a reusable Basic-auth or token-backed profile in one step, and `profile example` prints a fully commented reference template that you can copy and edit.

If you are still proving basic connectivity, you can do that before any profile work:

```bash
# Purpose: If you are still proving basic connectivity, you can do that before any profile work.
grafana-util status live \
  --url http://localhost:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format yaml
```

Then translate that same connection into a reusable profile:

```bash
# Purpose: Then translate that same connection into a reusable profile.
grafana-util profile add dev \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --prompt-password
```

```bash
# Purpose: Then translate that same connection into a reusable profile.
grafana-util status live --profile dev --output-format yaml
```

By default, the config file lives next to your current checkout. If you point `GRAFANA_UTIL_CONFIG` somewhere else, the helper files follow that config directory:

| File | Default location | Purpose |
| :--- | :--- | :--- |
| `grafana-util.yaml` | current working directory, or the path given by `GRAFANA_UTIL_CONFIG` | repo-local profile definitions |
| `.grafana-util.secrets.yaml` | same directory as `grafana-util.yaml` | encrypted secret store used by `encrypted-file` mode |
| `.grafana-util.secrets.key` | same directory as `grafana-util.yaml` | local key file used by `encrypted-file` without a passphrase |

### 2. List the profiles in the config file
```bash
# Purpose: 2. List the profiles in the config file.
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
# Purpose: 3. Inspect one resolved profile.
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
# Purpose: 1. Project Status Entry Point.
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
`status live` queries Grafana directly, and it now uses `--output-format` for format selection.

### 2. Overview Entry Point
```bash
# Purpose: 2. Overview Entry Point.
grafana-util overview live -h
```
**Expected Output:**
```text
Render a live overview by delegating to the shared status live path.

Examples:
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format yaml
```
`overview live` is a thin wrapper over shared live status. Use `--output-format yaml` for a readable summary and `--output-format interactive` for the TUI workbench.

### 3. Run the same read-only check in both common auth styles
```bash
# Purpose: 3. Run the same read-only check in both common auth styles.
grafana-util overview live --profile prod --output-format yaml
```

```bash
# Purpose: 3. Run the same read-only check in both common auth styles.
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output-format interactive
```
Use the profile form for normal repeatable work. Keep the direct Basic-auth form for bootstrap, break-glass access, or admin-only workflows when you are not ready to create a profile yet.

If your shell already exports auth variables, the same read can stay short without creating a profile first:

```bash
# Purpose: If your shell already exports auth variables, the same read can stay short without creating a profile first.
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
grafana-util overview live --url http://localhost:3000 --output-format yaml
```

### 4. Know the common token limitation

Token auth can be enough for single-org read flows, but multi-org or admin-scoped operations often need a user session or Basic auth with broader Grafana privileges.

- `--all-orgs` inventory and export flows are safest with `--profile` backed by admin credentials or with direct Basic auth.
- Org, user, team, and service account management commonly needs admin-level credentials and may not work with a narrow API token.
- When a token cannot see all target orgs, the command output is limited by that token's scope even if the flags ask for a broader view.

---

## 🖥️ Interactive Mode (TUI)

`grafana-util dashboard browse` opens the live dashboard tree in a terminal UI. `overview live --output-format interactive` opens the interactive overview mode.

---
[🏠 Home](index.md) | [➡️ Next: Architecture & Design](architecture.md)
