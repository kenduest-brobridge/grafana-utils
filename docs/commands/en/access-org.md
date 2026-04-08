# `grafana-util access org`

## Purpose

List live or local Grafana organizations, create, modify, export, import, diff, or delete Grafana organizations.

## When to use

- Inspect organization inventory and org users.
- Inspect organizations from a live Grafana server or from a local export bundle.
- Create a new organization or rename an existing one.
- Export or import org bundles between environments.
- Remove an organization by id or exact name.

## Before / After

- **Before**: org administration often starts as a one-off admin click path or a script that only works in one environment.
- **After**: one namespace covers inventory, rename, export/import, and deletion with repeatable admin-backed access.

## What success looks like

- org names and ids stay exact across inventory and change flows
- export and import bundles can be reused when moving between environments
- privileged actions stay tied to an explicit admin-backed profile instead of an ad hoc token

## Failure checks

- if a create, rename, export, import, or delete step fails, confirm the selected profile has the required admin privileges
- if a name-based lookup returns the wrong org, recheck the exact org id or exact name before retrying
- if bundle import or export looks incomplete, confirm you are targeting the expected environment

## Key flags

- `list`: `--input-dir`, `--org-id`, `--name`, `--query`, `--with-users`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--json`
- `modify`: `--org-id`, `--name`, `--set-name`, `--json`
- `export` and `diff`: `--org-id`, `--name`, `--output-dir` or `--diff-dir`, `--overwrite`, `--dry-run`, `--with-users`
- `import`: `--input-dir`, `--replace-existing`, `--dry-run`, `--yes`
- `delete`: `--org-id`, `--name`, `--prompt`, `--yes`, `--json`

## Notes

- Use `--profile` for repeatable org inventory when the selected profile has the required admin privileges.
- Org administration is commonly broader than a narrow API token. Basic auth or an admin-backed profile is the safer default for create, rename, export, import, and delete flows.

## Examples

```bash
# Purpose: Inspect org inventory before a rename or migration.
grafana-util access org list --profile prod --output-format text
```

```bash
# Purpose: Review a saved org bundle before replaying it.
grafana-util access org list --input-dir ./access-orgs --output-format table
```

```bash
# Purpose: Rename one org after confirming the current org name.
grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --set-name platform-core --json
```

```bash
# Purpose: Review the exact org name before changing it.
grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --set-name platform-core --json
```

```bash
# Purpose: Remove one org only after checking the exact name.
grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --yes
```

```bash
# Purpose: Prompt for one organization, confirm it, and then delete it.
grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --prompt
```

## Related commands

- [access](./access.md)
- [access user](./access-user.md)
- [access team](./access-team.md)
