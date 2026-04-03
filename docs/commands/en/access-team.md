# `grafana-util access team`

## Purpose

List, browse, create, modify, export, import, diff, or delete Grafana teams.

## When to use

- Inspect team inventory and team memberships.
- Create or update team membership and admin assignments.
- Export or import team bundles.
- Remove a team by id or exact name.

## Key flags

- `list`: `--query`, `--name`, `--with-members`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `browse`: `--query`, `--name`, `--with-members`, `--page`, `--per-page`
- `add`: `--name`, `--email`, `--member`, `--admin`, `--json`
- `modify`: `--team-id`, `--name`, `--add-member`, `--remove-member`, `--add-admin`, `--remove-admin`, `--json`
- `export` and `diff`: `--export-dir` or `--diff-dir`, `--overwrite`, `--dry-run`, `--with-members`
- `import`: `--import-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--team-id`, `--name`, `--yes`, `--json`

## Examples

```bash
grafana-util access team list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text
grafana-util access team add --url http://localhost:3000 --basic-user admin --basic-password admin --name platform-team --email platform@example.com --member alice --admin alice --json
grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes
```

## Related commands

- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access service-account](./access-service-account.md)
