# `grafana-util config`

## Root

Purpose: open repo-local configuration workflows for repeatable Grafana access.

When to use: when you want to manage repo-local defaults instead of repeating live connection flags in every command.

Description: the current public `config` surface is intentionally small. Today it is a connection-setup namespace that hosts `config profile`, which owns repo-local connection defaults, secret handling, and reusable authentication setup for local work and CI jobs.

## Command groups

- Connection Setup: `profile`

Open this page first when the question is not about one profile subcommand yet, but about how this repo should store and reuse Grafana connection details at all.

Primary entrypoint:

- [`config profile`](./profile.md): add, validate, inspect, and initialize repo-local profiles

## Before / After

- **Before**: live commands often repeat `--url`, token, or Basic auth flags in shell history, scripts, and onboarding notes.
- **After**: one repo-local profile can carry the shared connection defaults so daily commands stay shorter and CI stays easier to audit.

## What success looks like

- one checkout has the connection defaults it actually needs
- repeated live commands can use `--profile` instead of restating credentials
- secret storage mode matches the machine and operating model

## Failure checks

- if a saved profile does not behave like a direct command line, inspect it with `config profile show`
- if validation fails, confirm the selected secret mode is supported on the current machine
- if a repo still needs repeated auth flags, check whether the intended profile was created and selected

### Key flags

- `--color`: control JSON color output for this namespace

### Examples

```bash
# inspect the config namespace before choosing a profile subcommand.
grafana-util config --help
```

```bash
# initialize a starter config file in the current checkout.
grafana-util config profile init --overwrite
```

```bash
# add a reusable production profile backed by prompt-based secrets.
grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
```

## Related commands

- `grafana-util status live`
- `grafana-util status overview`
- `grafana-util workspace preview`
