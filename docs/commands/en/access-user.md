# `grafana-util access user`

## Purpose

List or browse live and local Grafana users, create, modify, export, import, diff, or delete Grafana users.

## When to use

- Inspect users in the current org or in global admin scope.
- Inspect users from a live Grafana server or from a local export bundle.
- Create or update users with login, email, role, and admin settings.
- Export and import user inventory bundles.
- Remove users from the org membership or from the global registry.

## Before / After

- **Before**: user lifecycle work is often split across org settings, admin screens, and one-off export or import scripts.
- **After**: one namespace covers inventory, create/update, export/import, and targeted removal with the same auth model.

## What success looks like

- created or modified users have the expected login, email, and role
- inventory and bundles can be diffed before deletion or migration
- membership scope stays explicit, so you do not accidentally manage the wrong org or global registry

## Failure checks

- if list, add, or delete looks empty or wrong, confirm the selected profile or auth token has the right org or admin scope
- if create or modify fails, recheck login or email uniqueness and whether the selected scope is org or global
- if an import does not behave as expected, verify the bundle source and the target scope before retrying

## Key flags

- `list`: `--input-dir`, `--scope`, `--all-orgs`, `--query`, `--login`, `--email`, `--org-role`, `--grafana-admin`, `--with-teams`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `browse`: `--input-dir`, `--scope`, `--all-orgs`, `--current-org`, `--query`, `--login`, `--email`, `--org-role`, `--grafana-admin`, `--page`, `--per-page`
- `add`: `--login`, `--email`, `--name`, `--password` or `--password-file` or `--prompt-user-password`, `--org-role`, `--grafana-admin`, `--json`
- `modify`: `--user-id`, `--login`, `--email`, `--set-login`, `--set-email`, `--set-name`, `--set-password` or `--set-password-file` or `--prompt-set-password`, `--set-org-role`, `--set-grafana-admin`, `--json`
- `export` and `diff`: `--output-dir` or `--diff-dir`, `--overwrite`, `--dry-run`, `--scope`, `--with-teams`
- `import`: `--input-dir`, `--scope`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--user-id`, `--login`, `--email`, `--scope`, `--yes`, `--json`

## Examples

```bash
# Purpose: Inspect users in one org before changing membership or roles.
grafana-util access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope org --output-format text
```

```bash
# Purpose: Review a saved user bundle without touching Grafana.
grafana-util access user list --input-dir ./access-users --output-format table
```

```bash
# Purpose: Browse one saved user bundle interactively without touching Grafana.
grafana-util access user browse --input-dir ./access-users --login alice
```

```bash
# Purpose: Create one user with explicit auth and org scope.
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret
```

```bash
# Purpose: Delete one account after checking the current org users.
grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login temp-user --scope global --yes --json
```

## Related commands

- [access](./access.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
