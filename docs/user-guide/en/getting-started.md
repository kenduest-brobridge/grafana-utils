# Getting Started

This chapter gets you from a fresh checkout or install to a verified `grafana-util` session. It exists for first use, for new environments, and for the moment when you want to confirm that the new layout, the binary, and your connection settings all line up.

Use it before you move into [Reference](./reference.md) or any of the [Scenarios](./scenarios.md).

## What This Chapter Is For

- Confirm that the local build matches the handbook.
- Choose a Grafana connection model that you can reuse.
- Run the first read-only commands safely.
- Learn enough about profiles to avoid repeating live flags on every command.

## First Checks

Start by checking the binary and the help surface. This confirms that the installed CLI is the one you expect and that the domain layout is available:

```bash
grafana-util --version
grafana-util version
grafana-util -h
grafana-util dashboard -h
grafana-util alert -h
grafana-util datasource -h
grafana-util access -h
grafana-util profile -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

Validated version output from the local Docker Grafana handbook fixture:

```text
$ grafana-util --version
grafana-util 0.6.2
```

That version check is the quickest way to catch a stale checkout or an unexpected install before you start comparing command output with the handbook.

## Pick A Connection Model

`grafana-util` can talk to Grafana in two ways:

- Pass live connection flags directly on the command line for one-off work.
- Load defaults from a repo-local `grafana-util.yaml` profile when you revisit the same environment often.

Direct flags are useful when you are proving a connection or testing a single command. Profiles are better when you want repeatable operator behavior across sessions, scripts, or teammates.

### Profiles

Profiles are the preferred way to keep a known Grafana target nearby without rewriting the same flags every time. A typical flow looks like this:

```bash
grafana-util profile init
grafana-util profile list
grafana-util profile show --profile prod --output-format yaml
```

Example profile file:

```yaml
default_profile: dev
profiles:
  dev:
    url: http://127.0.0.1:3000
    token_env: GRAFANA_API_TOKEN
    timeout: 30
    verify_ssl: false

  prod:
    url: https://grafana.example.com
    username: admin
    password_env: GRAFANA_PROD_PASSWORD
    verify_ssl: true
```

Profile selection follows the same rules across the CLI:

- `--profile NAME` explicitly selects one profile.
- If you do not pass `--profile`, the CLI uses `default_profile` when it exists.
- If there is exactly one profile and no default, that profile can be selected automatically.
- Command-line flags override values from the selected profile.
- If neither the flags nor the profile provide auth, the `GRAFANA_*` environment fallback still applies.

## Safe First Commands

After the binary and connection model are in place, start with a read-only command. A safe first step is usually an inventory or readiness read:

```bash
grafana-util dashboard list --profile prod --table
grafana-util datasource list --profile prod --table
grafana-util overview --profile prod
grafana-util status live --profile prod
```

These commands are low risk because they only read state. They let you confirm that the CLI can reach the target Grafana instance and that the output contract is readable before you move into export, import, or staged change workflows.

Validated `status live` output from the local Docker Grafana `12.4.1` fixture:

```text
$ grafana-util status live --url http://127.0.0.1:33000 --basic-user admin --basic-password admin
Project status
Overall: status=partial scope=live domains=6 present=6 blocked=0 blockers=0 warnings=4 freshness=current
Domains:
- dashboard status=ready mode=live-dashboard-read primary=3 blockers=0 warnings=0 freshness=current next=re-run live dashboard read after dashboard, folder, or datasource changes
- datasource status=ready mode=live-inventory primary=1 blockers=0 warnings=1 freshness=current next=review live datasource secret and provider fields before export or import
- alert status=ready mode=live-alert-surfaces primary=2 blockers=0 warnings=0 freshness=current next=re-run the live alert snapshot after provisioning changes
- access status=ready mode=live-list-surfaces primary=2 blockers=0 warnings=3 freshness=current next=review live access drift-severity signals: admin users
...
```

Read that output as a contract check. It tells you that the CLI can reach Grafana, produce structured project status, and explain what to investigate next. The overall `partial` state in this fixture comes from warnings, not from blocked reads, so it is a useful example of the difference between "reachable" and "fully clean."

## Next Step

If you need exact flags, output modes, or command-family rules, go to [Reference](./reference.md). If you want a task-oriented path, continue to [Scenarios](./scenarios.md).
