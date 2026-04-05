# `grafana-util profile`

## Root

Purpose: list, inspect, add, and initialize repo-local `grafana-util` profiles.

When to use: when you want to keep Grafana connection defaults in the current checkout and reuse them with `--profile`.

Description: open this page when you want to understand the full profile workflow before choosing one subcommand. The `profile` namespace is the entrypoint for repo-local connection defaults, secret handling, and non-interactive command reuse across local work, SRE tasks, and CI jobs.

## Before / After

- **Before**: connection settings live in scattered flags or ad hoc shell history, so the same live command is hard to repeat later.
- **After**: a named profile keeps URL, auth, and secret handling in one place so live commands stay shorter and easier to reuse.

## What success looks like

- one profile name captures the connection setup you actually want to reuse
- secret storage mode matches the environment instead of forcing every command to restate auth
- downstream live commands can stay readable because the profile hides repeated boilerplate

## Failure checks

- if a command fails after switching profiles, verify the resolved `show` output before assuming the command is broken
- if a secret is missing, check whether the profile is using `file`, `os`, or `encrypted-file` storage and whether that mode fits the current machine
- if a live command still needs too many flags, reconsider whether the profile should carry the default URL or auth values instead

Key flags: the root command is a namespace; operational flags live on subcommands. The shared root flag is `--color`.

Examples:

```bash
# Purpose: List profiles available in the current checkout.
grafana-util profile list
```

```bash
# Purpose: Inspect the resolved profile before running live commands.
grafana-util profile show --profile prod --output-format yaml
```

```bash
# Purpose: Create a reusable production profile with prompt-based secrets.
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
```

```bash
# Purpose: Create a CI profile that reads the token from an environment variable.
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
```

```bash
# Purpose: Print a fully annotated profile template.
grafana-util profile example --mode full
```

```bash
# Purpose: Initialize a fresh grafana-util.yaml in the current checkout.
grafana-util profile init --overwrite
```

Related commands: `grafana-util status live`, `grafana-util overview live`, `grafana-util change plan`.

## `list`

Purpose: list profile names from the resolved `grafana-util` config file.

When to use: when you need to confirm which profiles are available in the current checkout.

Key flags: none beyond the shared root `--color`.

Examples:

```bash
# Purpose: list.
grafana-util profile list
```

Related commands: `profile show`, `profile add`, `profile init`.

## `show`

Purpose: show the selected profile as text, table, csv, json, or yaml.

When to use: when you want to inspect the resolved connection settings before running a live command.

Key flags:
- `--profile`
- `--output-format`
- `--show-secrets`

Examples:

```bash
# Purpose: show.
grafana-util profile show --profile prod --output-format yaml
```

```bash
# Purpose: show.
grafana-util profile show --profile prod --output-format json
```

```bash
# Purpose: show.
grafana-util profile show --profile prod --show-secrets --output-format yaml
```

Notes:
- Secret values are masked by default.
- `--show-secrets` reveals plaintext values or resolves secret-store references.

Related commands: `profile list`, `profile add`, `status live`, `overview live`.

## `add`

Purpose: create or replace one named profile without hand-editing `grafana-util.yaml`.

When to use: when you want a friendlier path than editing YAML directly, especially for storing reusable auth defaults.

Key flags:
- `--url`
- auth inputs: `--token`, `--token-env`, `--prompt-token`, `--basic-user`, `--basic-password`, `--password-env`, `--prompt-password`
- storage mode: `--store-secret file|os|encrypted-file`
- encrypted-file options: `--secret-file`, `--prompt-secret-passphrase`, `--secret-passphrase-env`
- behavior: `--replace-existing`, `--set-default`

Examples:

```bash
# Purpose: add.
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --password-env GRAFANA_DEV_PASSWORD
```

```bash
# Purpose: add.
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os --set-default
```

```bash
# Purpose: add.
grafana-util profile add stage --url https://grafana-stage.example.com --token-env GRAFANA_STAGE_TOKEN --store-secret encrypted-file --prompt-secret-passphrase
```

Notes:
- Default config path: `grafana-util.yaml`
- Default encrypted secret file: `.grafana-util.secrets.yaml`
- Default local key file for encrypted-file without a passphrase: `.grafana-util.secrets.key`
- These default secret paths are resolved relative to the config file directory, not a temporary process cwd.
- `file` is the default storage mode.
- `os` and `encrypted-file` are explicit opt-in modes.
- `os` stores secrets in the macOS Keychain or Linux Secret Service, not in `grafana-util.yaml`.
- `os` is only supported on macOS and Linux. Headless Linux shells may need `password_env`, `token_env`, or `encrypted-file` instead.
- Prefer profile-backed `password_env` or `token_env` entries for repeated automation instead of pasting secrets into every live command.

Related commands: `profile show`, `profile example`, `profile init`.

## `example`

Purpose: print a comment-rich reference config that operators can copy and adapt.

When to use: when you want one canonical example that explains the supported profile fields and secret storage modes.

Key flags:
- `--mode basic|full`

Examples:

```bash
# Purpose: example.
grafana-util profile example
```

```bash
# Purpose: example.
grafana-util profile example --mode basic
```

```bash
# Purpose: example.
grafana-util profile example --mode full
```

Notes:
- `basic` is a short starter template.
- `full` includes commented examples for `file`, `os`, and `encrypted-file`.
- The `os` examples assume macOS Keychain or Linux Secret Service is available.

Related commands: `profile add`, `profile init`, `profile show`.

## `init`

Purpose: initialize `grafana-util.yaml` in the current working directory.

When to use: when a checkout does not yet have a repo-local profile file and you want the built-in starter template.

Key flags:
- `--overwrite`

Examples:

```bash
# Purpose: init.
grafana-util profile init
```

```bash
# Purpose: init.
grafana-util profile init --overwrite
```

Notes:
- `init` seeds the built-in starter template.
- `add` is the friendlier way to create one real profile entry directly.

Related commands: `profile add`, `profile example`, `status live`.
