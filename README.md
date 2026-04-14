# grafana-util
### An SRE-focused Grafana operations toolkit for inventory, review, recovery, and CI/CD workflows

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-util/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-util/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-util)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-util)](https://github.com/kenduest-brobridge/grafana-util/tags)

English | [繁體中文](./README.zh-TW.md)

**Live inventory, export/import, diff, change preview, and safe apply in one workflow.**

`grafana-util` is a Rust CLI for Grafana operators, SREs, and dashboard teams. It helps you inventory common Grafana resources, back up and restore dashboards, move dashboards across environments, compare local changes with live state, and run safer review steps before CI/CD or manual updates. It is not just a thin API wrapper; it pulls day-to-day operational work into one repeatable command-line workflow.

Developer note:

This tool came from recurring Grafana work that did not have one convenient support path:

1. Dashboard development often starts in a lab or local Grafana instance, then the same dashboard needs to be exported and reused elsewhere.
2. Dashboard changes may happen through the Grafana web UI, or through an AI agent editing local Grafana JSON files directly.
3. Customer dashboards often need to be exported, adjusted, and imported again, but the safer path is to bring them into a local developer Grafana first.
4. Operators often need to inventory which dashboards use which data sources, which users and teams exist, and how accounts, groups, and permissions are shaped.

Grafana itself does not make those workflows especially convenient for dashboard developers, SREs, or internal users. `grafana-util` exists to make those operational loops easier to review and repeat.

Common uses:

| You want to... | Start with |
| :--- | :--- |
| confirm Grafana is reachable | `grafana-util status live` |
| save a reusable connection | `grafana-util config profile add ...` |
| export or review dashboards | `grafana-util export dashboard` or `grafana-util dashboard summary` |
| review local changes before apply | `grafana-util workspace scan` then `workspace preview` |
| work on alerts or routes | `grafana-util alert plan` or `alert preview-route` |
| manage users, teams, orgs, or service accounts | `grafana-util access ...` |

The CLI is organized around these command families: `status`, `workspace`, `dashboard`, `datasource`, `alert`, `access`, and `config profile`. Use the handbook for workflow context and the command reference for exact syntax.

Supported Grafana surfaces:

| Area | What is covered | Good first command |
| :--- | :--- | :--- |
| Dashboards | Browse, list, export/import, diff, review, patch, publish, history, dependency analysis, policy checks, screenshots, and dashboard conversion for reuse across environments. | `grafana-util dashboard browse` |
| Datasources | Inventory, export/import, diff, create/modify/delete, secret-aware recovery, and type discovery. | `grafana-util datasource list` |
| Alerting | Rules, contact points, mute timings, templates, notification routes, review plans, apply flows, and route previews. | `grafana-util alert plan` |
| Access | Orgs, users, teams, service accounts, service-account tokens, export/import, diff, and delete review. | `grafana-util access user list` |
| Status and workspace | Live readiness, resource inventory, local workspace scan/test/preview/package/apply, and CI-friendly checks. | `grafana-util status live` |
| Profiles and secrets | Repo-local connection profiles with direct flags, environment-backed auth, prompted input, and multiple credential storage options. | `grafana-util config profile add` |

---

## Install

Install the latest release:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

Install the latest release and write shell completion for your current shell:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
```

Install interactively, then choose the install directory and shell completion setup when prompted:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh -s -- --interactive
```

Install a specific version:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | VERSION=0.10.2 sh
```

Install into a custom directory:

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | BIN_DIR="$HOME/.local/bin" sh
```

Local installer help:

```bash
sh ./scripts/install.sh --help
```

Install and verify a local checkout build through the same installer path:

```bash
make install-local-interactive
```

- **Releases**: [GitHub releases](https://github.com/kenduest-brobridge/grafana-util/releases)
- **Binaries**: standard `linux-amd64` and `macos-arm64`; screenshot-enabled builds use `*-browser-*`
- **Default path**: `/usr/local/bin` if writable, otherwise `$HOME/.local/bin`
- **Completion**: set `INSTALL_COMPLETION=auto`, `INSTALL_COMPLETION=bash`, or `INSTALL_COMPLETION=zsh` to install completion from the downloaded binary
- **Interactive install**: use `sh -s -- --interactive` after the pipe to answer install directory and completion prompts; Zsh installs can also update `~/.zshrc` so `~/.zfunc` loads before `compinit`
- **Local install test**: use `make install-local` or `make install-local-interactive` to install a local checkout build through `scripts/install.sh`

Shell completion:

```bash
# Bash
mkdir -p ~/.local/share/bash-completion/completions
grafana-util completion bash > ~/.local/share/bash-completion/completions/grafana-util
```

```zsh
# Zsh
mkdir -p ~/.zfunc
grafana-util completion zsh > ~/.zfunc/_grafana-util
```

For Zsh, make sure `~/.zfunc` is in `fpath` before `compinit`. Interactive installs can add that block to `~/.zshrc` for you and clear stale `.zcompdump*` completion caches.

---

## First Run

Three steps to your first working session:

```bash
# 1. Confirm the CLI is installed.
grafana-util --version
```

```bash
# 2. Run one read-only live check.
grafana-util status live \
  --url http://grafana.example:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format yaml
```

```bash
# 3. Save the same connection for repeatable commands.
grafana-util config profile add dev \
  --url http://grafana.example:3000 \
  --basic-user admin \
  --prompt-password
```

After that:

- learn the workflow: [New User Path](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/role-new-user.html)
- look up exact syntax: [Command Reference](https://kenduest-brobridge.github.io/grafana-state-kit/commands/en/index.html)

---

## Example Commands

Check that Grafana is reachable:

```bash
grafana-util status live --profile prod --output-format interactive
```

Save a reusable connection profile:

```bash
grafana-util config profile add prod \
  --url http://grafana.example:3000 \
  --basic-user admin \
  --prompt-password
```

Export dashboards:

```bash
grafana-util export dashboard --profile prod --output-dir ./backup --overwrite
```

List dashboards using a specified connection profile:

```bash
grafana-util dashboard list --profile prod
```

List datasources:

```bash
grafana-util datasource list --profile prod
```

Look up exact syntax for a command family:

```bash
grafana-util dashboard --help
grafana-util config profile --help
```

---

## Docs

Handbook for workflow context; command reference for exact CLI syntax.

- [Published docs](https://kenduest-brobridge.github.io/grafana-state-kit/)
- First-time setup: [Getting Started](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/getting-started.html) and [New User Path](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/role-new-user.html)
- Operator workflow: [Operator Handbook](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/index.html) and [SRE / Ops Path](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/role-sre-ops.html)
- Exact CLI syntax: [Command Reference](https://kenduest-brobridge.github.io/grafana-state-kit/commands/en/index.html) and `grafana-util --help`
- [Troubleshooting](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/troubleshooting.html)

For in-repo doc maintenance:

- **Local HTML docs portal**: [docs/html/index.html](./docs/html/index.html)
- **Maintainer guide**: [docs/DEVELOPER.md](./docs/DEVELOPER.md)
- **Manpage source**: [docs/man/grafana-util.1](./docs/man/grafana-util.1)

By role:

- [New user](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/role-new-user.html)
- [SRE / operator](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/role-sre-ops.html)
- [Automation / CI owner](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/en/role-automation-ci.html)
- **Maintainer / developer**: [docs/DEVELOPER.md](./docs/DEVELOPER.md)

---

## Project Status

This project is under active development. CLI paths, help output, examples, and documentation structure may change between releases. Always prefer the command reference and `--help` output over examples from older issues or prior revisions.

---

## Contributing

For implementation setup and maintainer guidance, use [docs/DEVELOPER.md](./docs/DEVELOPER.md).
