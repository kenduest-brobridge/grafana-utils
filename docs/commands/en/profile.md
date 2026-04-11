# `grafana-util config profile`

## Root

Purpose: list, inspect, validate, add, and initialize repo-local `grafana-util` profiles through the current `config profile` entrypoint.

When to use: when you want to keep Grafana connection defaults in the current checkout and reuse them with `--profile`.

Description: open this page when you want to understand the full profile workflow before choosing one subcommand. The public entrypoint is `grafana-util config profile`; the old top-level `profile` root is no longer a runnable onboarding path. This namespace covers repo-local connection defaults, secret handling, and non-interactive command reuse across local work, SRE tasks, and CI jobs.

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
grafana-util config profile list
```

```bash
# Purpose: Inspect the resolved profile before running live commands.
grafana-util config profile show --profile prod --output-format yaml
```

```bash
# Purpose: Show the currently selected profile and resolved config path.
grafana-util config profile current --profile prod
```

```bash
# Purpose: Validate the selected profile and check Grafana reachability.
grafana-util config profile validate --profile prod --live
```

```bash
# Purpose: Create a reusable production profile with prompt-based secrets.
grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
```

```bash
# Purpose: Create a CI profile that reads the token from an environment variable.
grafana-util config profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
```

```bash
# Purpose: Print a fully annotated profile template.
grafana-util config profile example --mode full
```

```bash
# Purpose: Initialize a fresh grafana-util.yaml in the current checkout.
grafana-util config profile init --overwrite
```

Related commands: `grafana-util status live`, `grafana-util status overview`, `grafana-util workspace preview`, `grafana-util config profile current`, `grafana-util config profile validate`.

## `list`

Purpose: list profile names from the resolved `grafana-util` config file.

When to use: when you need to confirm which profiles are available in the current checkout.

Key flags: none beyond the shared root `--color`.

Examples:

```bash
# Purpose: list.
grafana-util config profile list
```

Related commands: `config profile show`, `config profile current`, `config profile add`, `config profile init`.

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
grafana-util config profile show --profile prod --output-format yaml
```

```bash
# Purpose: show.
grafana-util config profile show --profile prod --output-format json
```

```bash
# Purpose: show.
grafana-util config profile show --profile prod --show-secrets --output-format yaml
```

Notes:
- Secret values are masked by default.
- `--show-secrets` reveals plaintext values or resolves secret-store references.

Related commands: `config profile list`, `config profile add`, `config profile current`, `config profile validate`, `status live`, `status overview`.

## `current`

Purpose: show the currently selected profile, resolved config path, auth mode, and secret mode.

When to use: when you want to confirm which repo-local profile would be used before a live command runs.

Key flags:
- `--profile`
- `--output-format`

Examples:

```bash
# Purpose: current.
grafana-util config profile current
```

```bash
# Purpose: current.
grafana-util config profile current --profile prod --output-format json
```

Notes:
- The output is diagnostic only and does not reveal secrets.
- If the config file is missing, `current` reports that the config does not exist instead of failing.

Related commands: `config profile show`, `config profile validate`, `status live`, `status overview`.

## `validate`

Purpose: validate the selected profile and optionally check Grafana reachability.

When to use: when you want to confirm profile selection, auth shape, and secret resolution before running a live command.

Key flags:
- `--profile`
- `--live`
- `--output-format`

Examples:

```bash
# Purpose: validate.
grafana-util config profile validate --profile prod
```

```bash
# Purpose: validate.
grafana-util config profile validate --profile prod --live --output-format json
```

Notes:
- `--live` adds a Grafana `/api/health` check after static validation succeeds.
- Validation does not print secrets.

Related commands: `config profile current`, `config profile show`, `status live`, `status overview`.

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
grafana-util config profile add dev --url http://127.0.0.1:3000 --basic-user admin --password-env GRAFANA_DEV_PASSWORD
```

```bash
# Purpose: add.
grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os --set-default
```

```bash
# Purpose: add.
grafana-util config profile add stage --url https://grafana-stage.example.com --token-env GRAFANA_STAGE_TOKEN --store-secret encrypted-file --prompt-secret-passphrase
```

Notes:
- Default config path: `grafana-util.yaml`
- Default encrypted secret file: `.grafana-util.secrets.yaml`
- Default local key file for encrypted-file without a passphrase: `.grafana-util.secrets.key`
- `config profile add --store-secret encrypted-file` updates the config-directory `.gitignore` with those helper files when they live under the same directory tree.
- These default secret paths are resolved relative to the config file directory, not a temporary process cwd.
- `file` is the default storage mode.
- `os` and `encrypted-file` are explicit opt-in modes.
- `os` stores secrets in the macOS Keychain or Linux Secret Service, not in `grafana-util.yaml`.
- `os` is only supported on macOS and Linux. Headless Linux shells may need `password_env`, `token_env`, or `encrypted-file` instead.
- Prefer profile-backed `password_env` or `token_env` entries for repeated automation instead of pasting secrets into every live command.

Related commands: `config profile show`, `config profile current`, `config profile example`, `config profile init`.

## `example`

Purpose: print a comment-rich reference config that operators can copy and adapt.

When to use: when you want one canonical example that explains the supported profile fields and secret storage modes.

Key flags:
- `--mode basic|full`

Examples:

```bash
# Purpose: example.
grafana-util config profile example
```

```bash
# Purpose: example.
grafana-util config profile example --mode basic
```

```bash
# Purpose: example.
grafana-util config profile example --mode full
```

Notes:
- `basic` is a short starter template.
- `full` includes commented examples for `file`, `os`, and `encrypted-file`.
- The `os` examples assume macOS Keychain or Linux Secret Service is available.

Related commands: `config profile add`, `config profile init`, `config profile show`, `config profile current`, `config profile validate`.

## `init`

Purpose: initialize `grafana-util.yaml` in the current working directory.

When to use: when a checkout does not yet have a repo-local profile file and you want the built-in starter template.

Key flags:
- `--overwrite`

Examples:

```bash
# Purpose: init.
grafana-util config profile init
```

```bash
# Purpose: init.
grafana-util config profile init --overwrite
```

Notes:
- `init` seeds the built-in starter template.
- `add` is the friendlier way to create one real profile entry directly.

Related commands: `config profile add`, `config profile example`, `config profile current`, `config profile validate`, `status live`.
