# `grafana-util access team`

## Purpose

List or browse live and local Grafana teams, create, modify, export, import, diff, or delete Grafana teams.

## When to use

- Inspect team inventory and team memberships.
- Inspect teams from a live Grafana server or from a local export bundle.
- Create or update team membership and admin assignments.
- Export or import team bundles.
- Remove a team by id or exact name.

## Before / After

- **Before**: team membership changes often happen through UI side menus or scattered scripts.
- **After**: one namespace handles inventory, membership updates, export/import, and deletion with the same auth model.

## What success looks like

- team membership changes stay tied to an exact team id or name
- admin assignments are visible before you add or remove members
- exported bundles can be reused in another environment without re-creating the team by hand

## Failure checks

- if list, add, modify, or delete fails, confirm the team exists in the selected org and the auth scope is correct
- if membership looks incomplete, recheck the exact member names and whether `--with-members` was set
- if an import behaves unexpectedly, verify the source bundle and the target environment before retrying

## Key flags

- `list`: `--input-dir`, `--query`, `--name`, `--with-members`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `browse`: `--input-dir`, `--query`, `--name`, `--with-members`, `--page`, `--per-page`
- `add`: `--name`, `--email`, `--member`, `--admin`, `--json`
- `modify`: `--team-id`, `--name`, `--add-member`, `--remove-member`, `--add-admin`, `--remove-admin`, `--json`
- `export` and `diff`: `--output-dir` or `--diff-dir`, `--overwrite`, `--dry-run`, `--with-members`
- `import`: `--input-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--team-id`, `--name`, `--yes`, `--json`

## Examples

```bash
# Purpose: Inspect team membership before adding or removing people.
grafana-util access team list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text
```

```bash
# Purpose: Review a saved team bundle before replaying it.
grafana-util access team list --input-dir ./access-teams --output-format table
```

```bash
# Purpose: Browse one saved team bundle interactively without touching Grafana.
grafana-util access team browse --input-dir ./access-teams --name platform-team
```

```bash
# Purpose: Create a team with explicit member and admin assignments.
grafana-util access team add --url http://localhost:3000 --basic-user admin --basic-password admin --name platform-team --email platform@example.com --member alice --admin alice --json
```

```bash
# Purpose: Import a team bundle before switching environments.
grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes
```

## Related commands

- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access service-account](./access-service-account.md)
