# TODO

## Planned: User and Group Management

Add a dedicated access-management CLI for Grafana user and team operations.

Recommended command name:

- `grafana-access-utils`

Recommended model:

- `user` for Grafana users
- `team` for Grafana teams
- `group` as an alias of `team`

Reasoning:

- Grafana's official concept is `Team`, not a generic internal `Group`
- external identity-provider groups, Team Sync, and SCIM should be treated as a separate future scope

### Proposed command shape

Use subcommands instead of `user --list` style flags.

#### User commands

```text
grafana-access-utils user list
grafana-access-utils user add
grafana-access-utils user delete
grafana-access-utils user modify
```

#### Team commands

```text
grafana-access-utils team list
grafana-access-utils team add
grafana-access-utils team delete
grafana-access-utils team modify
```

#### Group alias

```text
grafana-access-utils group list
grafana-access-utils group add
grafana-access-utils group delete
grafana-access-utils group modify
```

`group` should behave as a compatibility alias for `team`.

### Shared connection and output parameters

- `--url`
- `--token`
- `--basic-user`
- `--basic-password`
- `--org-id`
- `--insecure`
- `--ca-cert`
- `--json`
- `--csv`
- `--table`

### Authentication behavior

The access-management CLI should not assume username and password are always required.

Baseline rules:

- if `--token` is provided, treat it as the primary authentication input
- when `--token` is provided, do not require `--basic-user` or `--basic-password`
- only require `--basic-user` and `--basic-password` for operations that truly need Basic auth
- fail early with a clear error if the selected command requires Basic auth and only `--token` was provided

Recommended precedence:

1. `--token`
2. `--basic-user` plus `--basic-password`

Recommended validation:

- reject `--basic-user` without `--basic-password`
- reject `--basic-password` without `--basic-user`
- allow both token and Basic auth to be passed together only if the implementation has a clear reason to do so
- otherwise prefer rejecting mixed auth input to keep operator intent explicit

Important design note:

- current dashboard and alerting tools already support token-based access patterns
- the new access-management CLI should align with that operator experience
- do not design the new commands around a username/password-only assumption

### Proposed user CLI parameters

#### `user list`

```text
grafana-access-utils user list
  [--scope org|global]
  [--query TEXT]
  [--login LOGIN]
  [--email EMAIL]
  [--org-role Viewer|Editor|Admin|None]
  [--grafana-admin true|false]
  [--with-teams]
  [--page N]
  [--per-page N]
  [--json|--csv|--table]
```

#### `user add`

```text
grafana-access-utils user add
  --login LOGIN
  --email EMAIL
  --name NAME
  [--password PASSWORD]
  [--org-id ID]
  [--org-role Viewer|Editor|Admin|None]
  [--grafana-admin true|false]
```

#### `user modify`

```text
grafana-access-utils user modify
  (--user-id ID | --login LOGIN | --email EMAIL)
  [--set-login LOGIN]
  [--set-email EMAIL]
  [--set-name NAME]
  [--set-password PASSWORD]
  [--set-org-role Viewer|Editor|Admin|None]
  [--set-grafana-admin true|false]
  [--org-id ID]
```

#### `user delete`

```text
grafana-access-utils user delete
  (--user-id ID | --login LOGIN | --email EMAIL)
  [--scope org|global]
  [--org-id ID]
  [--yes]
```

### Proposed team CLI parameters

#### `team list`

```text
grafana-access-utils team list
  [--query TEXT]
  [--name NAME]
  [--with-members]
  [--page N]
  [--per-page N]
  [--json|--csv|--table]
```

#### `team add`

```text
grafana-access-utils team add
  --name NAME
  [--email EMAIL]
  [--member LOGIN_OR_EMAIL ...]
  [--admin LOGIN_OR_EMAIL ...]
```

#### `team modify`

```text
grafana-access-utils team modify
  (--team-id ID | --name NAME)
  [--set-name NAME]
  [--set-email EMAIL]
  [--add-member LOGIN_OR_EMAIL ...]
  [--remove-member LOGIN_OR_EMAIL ...]
  [--add-admin LOGIN_OR_EMAIL ...]
  [--remove-admin LOGIN_OR_EMAIL ...]
```

#### `team delete`

```text
grafana-access-utils team delete
  (--team-id ID | --name NAME)
  [--yes]
```

### Permission model to expose clearly

Do not collapse every permission concept into one `--role` flag.

Expose these separately:

- `--org-role`: `Viewer|Editor|Admin|None`
- `--grafana-admin`: `true|false`
- `--team-role`: `member|admin`

Reasoning:

- Grafana server admin is different from organization role
- organization role is different from team membership admin
- the CLI should mirror Grafana's real permission boundaries instead of hiding them behind an ambiguous parameter

### Authentication constraints to enforce

- global user management requires Grafana server admin
- Grafana User/Admin API should be treated as Basic-auth-first workflows
- service account tokens should not be assumed to work for server-admin user management
- team and org-scoped operations may use different APIs and permissions than global user management

The CLI should perform an auth preflight and return a clear error when the selected operation cannot work with the provided authentication mode.

### Recommended v1 implementation order

1. `user list`
2. `team list`
3. `team modify` for add/remove member and add/remove admin
4. `user add`
5. `user modify`
6. `user delete`
7. `team add`
8. `team delete`

### Explicitly out of scope for v1

- SCIM provisioning support
- Team Sync support
- LDAP or OAuth group-sync orchestration
- external identity-provider group lifecycle management

These should be evaluated later as separate features.
