# `grafana-util access user`

## Purpose

List, browse, create, modify, export, import, diff, or delete Grafana users.

## When to use

- Inspect users in the current org or in global admin scope.
- Create or update users with login, email, role, and admin settings.
- Export and import user inventory bundles.
- Remove users from the org membership or from the global registry.

## Key flags

- `list`: `--scope`, `--all-orgs`, `--query`, `--login`, `--email`, `--org-role`, `--grafana-admin`, `--with-teams`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `browse`: `--scope`, `--all-orgs`, `--current-org`, `--query`, `--login`, `--email`, `--org-role`, `--grafana-admin`, `--page`, `--per-page`
- `add`: `--login`, `--email`, `--name`, `--password` or `--password-file` or `--prompt-user-password`, `--org-role`, `--grafana-admin`, `--json`
- `modify`: `--user-id`, `--login`, `--email`, `--set-login`, `--set-email`, `--set-name`, `--set-password` or `--set-password-file` or `--prompt-set-password`, `--set-org-role`, `--set-grafana-admin`, `--json`
- `export` and `diff`: `--export-dir` or `--diff-dir`, `--overwrite`, `--dry-run`, `--scope`, `--with-teams`
- `import`: `--import-dir`, `--scope`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--user-id`, `--login`, `--email`, `--scope`, `--yes`, `--json`

## Examples

```bash
grafana-util access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope org --output-format text
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret
grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login temp-user --scope global --yes --json
```

## Related commands

- [access](./access.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
