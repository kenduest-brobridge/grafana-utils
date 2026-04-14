# Getting Started

This guide is for the first time you need to install `grafana-util`, prove that it can reach Grafana, and decide whether a direct command, env-backed auth, or a repo-local `config profile` is the cleanest starting point.

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
  - store repeatable defaults in a repo-local profile and reuse them with `config profile`

Profiles are there to simplify repeated work. They are not the only way to start, and they should not block a first connectivity check.

## Before / After

- Before: every command line had to repeat the Grafana URL and auth flags.
- After: you can prove the connection directly once, then move the repeatable parts into a `config profile`.

## What success looks like

- The binary is installed and reachable from your shell.
- One direct live read succeeds.
- You know whether the next step should be `config profile`, env-backed auth, or a one-off bootstrap command.

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

For the exact flags behind this chapter, keep [config](../../commands/en/config.md), [config profile](../../commands/en/profile.md), [status live](../../commands/en/status.md), and [status overview](../../commands/en/status.md) open beside it.

## Pick the first path

| If your job is... | Start here | Why |
| :--- | :--- | :--- |
| prove one connection works | `status live --output-format yaml` | it is read-only and shows auth, URL, and scope problems early |
| understand a live estate | `status overview live --output-format interactive` | it gives a browsable summary before you drill into one resource type |
| migrate dashboards or datasources | `export`, then `diff`, then dry-run `import` | it keeps review before replay |
| automate a local package | `workspace scan`, then `workspace preview`, then `workspace test` | it turns staged files into a reviewable plan before apply |

---

## Step 1: Installation

### Download and Install
```bash
# Install the latest release.
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

To install the binary and refresh shell completion in one GitHub install step, opt in explicitly:

```bash
# Install the latest release and write completion for the current shell.
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
```

If you prefer prompts, run the installer interactively. This asks for the install directory, whether to install shell completion, and where to write the completion file:

```bash
# Ask before choosing install and completion locations.
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh -s -- --interactive
```

If you want one fixed release or one explicit install directory, the same script also supports:

```bash
# Install one pinned release into one explicit binary directory.
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | VERSION=0.10.2 BIN_DIR="$HOME/.local/bin" sh
```

The installer uses `BIN_DIR` when you set it. Otherwise it tries `/usr/local/bin` when that directory is writable, then falls back to `$HOME/.local/bin`.

If the chosen install directory is not already on `PATH`, the installer prints the exact shell snippet to add it for `zsh` or `bash`. `INSTALL_COMPLETION=auto` detects `bash` or `zsh` from `SHELL`; use `INSTALL_COMPLETION=bash` or `INSTALL_COMPLETION=zsh` when you want to choose explicitly. In interactive mode, any value you already pass through `BIN_DIR`, `INSTALL_COMPLETION`, or `COMPLETION_DIR` is treated as already chosen and is not asked again. You can also inspect the contract first with:

```bash
# Show install script options, BIN_DIR behavior, completion, and PATH setup notes.
sh ./scripts/install.sh --help
```

### Verify Version
```bash
# Confirm the installed binary and version.
grafana-util --version
```
**Expected Output:**
```text
grafana-util 0.10.2
```
This confirms that the binary is on your `PATH` and that you are running the version you expect.

---

## Step 2: Connection Patterns And Profile Files

Profile workflows are repo-local. `grafana-util config profile` works against `grafana-util.yaml` in the current working directory by default, or against the file pointed to by `GRAFANA_UTIL_CONFIG`.

### Auth modes at a glance

`grafana-util` can read connection settings from direct flags, prompt-based input, environment variables, or a repo-local profile. Use the auth modes in this order:

**Direct Basic auth**

Best for quick local checks, bootstrap, and admin-only workflows.

```bash
grafana-util status live \
  --url http://localhost:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format yaml
```

**`config profile`**

Best for daily operator workflows and CI jobs once the connection is proven.

```bash
grafana-util status live \
  --profile prod \
  --output-format yaml
```

**Direct token**

Best for narrow API automation that stays inside one org or one scoped permission set.

```bash
grafana-util status overview live \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --output-format yaml
```

Environment variables can supply the same auth without repeating sensitive values on every command:

- `GRAFANA_USERNAME`
- `GRAFANA_PASSWORD`
- `GRAFANA_API_TOKEN`

For repeatable work, prefer storing those references in a profile such as `password_env: GRAFANA_PROD_PASSWORD` or `token_env: GRAFANA_DEV_TOKEN` instead of repeating raw secrets on every command line.

### Start direct, then simplify

For a first run, the cleanest mental model is:

1. run one direct read-only command with `--url` plus either Basic auth or token auth
2. once that works, move the repeatable parts into a profile
3. keep using `config profile` for normal day-to-day work

### 1. Pick how you want to create profiles
```bash
# Create the profile config file in the current checkout.
grafana-util config profile init --overwrite
```

```bash
# Create a local dev profile and enter the password interactively.
grafana-util config profile add dev \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --prompt-password
```

```bash
# Create a CI profile that reads its token from the environment.
grafana-util config profile add ci \
  --url https://grafana.example.com \
  --token-env GRAFANA_CI_TOKEN \
  --store-secret os
```

```bash
# Print the fully annotated profile template for comparison.
grafana-util config profile example --mode full
```
`config profile init` creates a minimal starter `grafana-util.yaml`. `config profile add` can create a reusable Basic-auth or token-backed profile in one step, and `config profile example` prints a fully commented reference template that you can copy and edit.

If you are still proving basic connectivity, you can do that before any profile work:

```bash
# Before creating a profile, prove that the Grafana connection works.
grafana-util status live \
  --url http://localhost:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format yaml
```

Then translate that same connection into a reusable profile:

```bash
# Once the direct check works, turn the same connection into a dev profile.
grafana-util config profile add dev \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --prompt-password
```

```bash
# Use that profile for the same read-only check next time.
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
# List the profile names available in the current config file.
grafana-util config profile list
```
**Expected Output:**
```text
dev
prod
```
On a freshly initialized config, `config profile list` prints one discovered profile name per line.

Use the [config profile](../../commands/en/profile.md) command reference when you want the flag-by-flag auth rules.

### 3. Inspect one resolved profile
```bash
# Inspect the final resolved connection settings for the prod profile.
grafana-util config profile show --profile prod --output-format yaml
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
Use `config profile` when you want to override the default-selection rules, and `yaml` when you want the resolved fields in a readable form.

---

## Step 3: First Read-Only Checks

Once a profile file exists, use read-only commands to confirm the current command shape before you touch live data.

### 1. Project Status Entry Point
```bash
# Check which read-only options status live supports.
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
          Grafana base URL. Required unless supplied by --profile or GRAFANA_URL.
```
`status live` queries Grafana directly, and it now uses `--output-format` for format selection.

### 2. Overview Entry Point
```bash
# Inspect the human-oriented status overview entrypoint.
grafana-util status overview -h
```
**Expected Output:**
```text
Render a live overview by delegating to the shared status live path.

Examples:
  grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
  grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format yaml
```
`status overview live` is a thin wrapper over shared status overview. Use `--output-format yaml` for a readable summary and `--output-format interactive` for the TUI workbench.

### 3. Run the same read-only check in both common auth styles
```bash
# Use a profile for normal repeatable overview checks.
grafana-util status overview live --profile prod --output-format yaml
```

```bash
# Use direct Basic auth for bootstrap or break-glass checks.
grafana-util status overview live --url http://localhost:3000 --basic-user admin --prompt-password --output-format interactive
```
Use the profile form for normal repeatable work. Keep the direct Basic-auth form for bootstrap, break-glass access, or admin-only workflows when you are not ready to create a profile yet.

If your shell already exports auth variables, the same read can stay short without creating a profile first:

```bash
# If the shell already has credentials, run the same read through env vars.
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
grafana-util status overview live --url http://localhost:3000 --output-format yaml
```

### 4. Know the common token limitation

Token auth can be enough for single-org read flows, but multi-org or admin-scoped operations often need a user session or Basic auth with broader Grafana privileges.

- `--all-orgs` inventory and export flows are safest with `config profile` backed by admin credentials or with direct Basic auth.
- Org, user, team, and service account management commonly needs admin-level credentials and may not work with a narrow API token.
- When a token cannot see all target orgs, the command output is limited by that token's scope even if the flags ask for a broader view.

---

## Interactive Mode (TUI)

`grafana-util dashboard browse` opens the live dashboard tree in a terminal UI. `status overview live --output-format interactive` opens the interactive overview mode.

---
[🏠 Home](index.md) | [➡️ Next: Architecture & Design](architecture.md)
