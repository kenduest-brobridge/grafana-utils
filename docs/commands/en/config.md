# `grafana-util config`

## Root

Purpose: open repo-local configuration workflows for repeatable Grafana access.

When to use: when you want to manage repo-local defaults instead of repeating live connection flags in every command.

Description: the current public `config` surface is intentionally small. Today it exists to host `config profile`, which owns repo-local connection defaults, secret handling, and reusable authentication setup for local work and CI jobs.

Primary entrypoint:

- [`config profile`](./profile.md): add, validate, inspect, and initialize repo-local profiles

Examples:

```bash
# Purpose: initialize a starter config file in the current checkout.
grafana-util config profile init --overwrite
```

```bash
# Purpose: add a reusable production profile backed by prompt-based secrets.
grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
```

Related commands: `grafana-util status live`, `grafana-util status overview`, `grafana-util workspace preview`.
