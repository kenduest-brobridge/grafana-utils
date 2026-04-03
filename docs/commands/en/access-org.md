# `grafana-util access org`

## Purpose

List, create, modify, export, import, diff, or delete Grafana organizations.

## When to use

- Inspect organization inventory and org users.
- Create a new organization or rename an existing one.
- Export or import org bundles between environments.
- Remove an organization by id or exact name.

## Key flags

- `list`: `--org-id`, `--name`, `--query`, `--with-users`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--json`
- `modify`: `--org-id`, `--name`, `--set-name`, `--json`
- `export` and `diff`: `--org-id`, `--name`, `--export-dir` or `--diff-dir`, `--overwrite`, `--dry-run`, `--with-users`
- `import`: `--import-dir`, `--replace-existing`, `--dry-run`, `--yes`
- `delete`: `--org-id`, `--name`, `--yes`, `--json`

## Notes

- Use `--profile` for repeatable org inventory when the selected profile has the required admin privileges.
- Org administration is commonly broader than a narrow API token. Basic auth or an admin-backed profile is the safer default for create, rename, export, import, and delete flows.

## Examples

```bash
grafana-util access org list --profile prod --output-format text
grafana-util access org list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --set-name platform-core --json
grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --yes
```

## Related commands

- [access](./access.md)
- [access user](./access-user.md)
- [access team](./access-team.md)
