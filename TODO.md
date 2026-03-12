# TODO

## Status

### Done

- added `grafana-access-utils`
- implemented `user list`
- implemented `user add`
- implemented `user modify`
- implemented `user delete`
- implemented `team list`
- implemented `team add`
- implemented `team modify`
- implemented `service-account list`
- implemented `service-account add`
- implemented `service-account token add`
- added Python packaging entrypoint and thin `cmd/grafana-access-utils.py` wrapper
- added unit tests and Docker-backed live validation for the implemented access workflows

### In Progress

- access-management CLI exists, but only part of the planned user/team/group scope is implemented
- auth preflight is implemented for current commands, but not yet for the remaining planned mutating commands

### Next

- `team delete`
- `group` alias for `team`

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

Current implementation status:

- `user list`: done
- `user add`: done
- `user modify`: done
- `user delete`: done
- `team list`: done
- `team add`: done
- `team modify`: done
- `service-account list`: done
- `service-account add`: done
- `service-account token add`: done
- remaining `team` commands: `team delete` only
- `group` alias: not started

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

Current note:

- `service-account` is now also a first-class resource even though it was not in the original command sketch above
- current implemented command set is:

```text
grafana-access-utils user list
grafana-access-utils user add
grafana-access-utils user modify
grafana-access-utils user delete
grafana-access-utils team list
grafana-access-utils team add
grafana-access-utils team modify
grafana-access-utils service-account list
grafana-access-utils service-account add
grafana-access-utils service-account token add
```

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

Current implementation status:

- done: `--url`, `--token`, `--basic-user`, `--basic-password`, `--org-id`, `--json`, `--csv`, `--table`
- not done: `--insecure`, `--ca-cert`

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

Current implementation status:

- `user list --scope org`: token or Basic auth
- `user list --scope global`: Basic auth only
- `user list --with-teams`: Basic auth only
- `user add`: Basic auth only
- `user modify`: Basic auth only
- `user delete --scope global`: Basic auth only
- `user delete --scope org`: token or Basic auth
- `team list`: token or Basic auth
- `team modify`: token or Basic auth
- service-account operations: token or Basic auth
- remaining planned commands still need explicit per-command auth preflight

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

Current status:

- implemented
- org-scoped
- auth: token or Basic auth

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

1. `team list`
2. `team modify` for add/remove member and add/remove admin
3. `user add`
4. `user modify`
5. `user delete`
6. `team add`
7. `team delete`
8. `group` alias for `team`

Completed ahead of the original order:

- `user list`
- `user add`
- `user modify`
- `user delete`
- `team list`
- `team modify`
- `service-account list`
- `service-account add`
- `service-account token add`

### Explicitly out of scope for v1

- SCIM provisioning support
- Team Sync support
- LDAP or OAuth group-sync orchestration
- external identity-provider group lifecycle management

These should be evaluated later as separate features.
