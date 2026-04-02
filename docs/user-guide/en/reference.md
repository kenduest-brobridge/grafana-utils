# Reference

This chapter is the operator reference for `grafana-util`. Use it when you already know the job and need the exact command family, option behavior, output contract, or cross-domain surface that applies.

The [Getting started](./getting-started.md) chapter helps you prove that the binary works and the connection is wired correctly. This chapter tells you which surface to use and what rules stay constant across the CLI.

## Command Families

The CLI is grouped by the work you are trying to do, not by implementation detail. That makes it easier to move from one chapter to another without relearning the command shape each time.

| Goal | Start here | Common commands |
| --- | --- | --- |
| Dashboard inventory and analysis | `dashboard` | `browse`, `list`, `export`, `import`, `diff`, `delete`, `inspect-export`, `inspect-live`, `inspect-vars`, `screenshot` |
| Alerting management and migration | `alert` | `plan`, `apply`, `delete`, `init`, `new-rule`, `new-contact-point`, `new-template`, `list-rules`, `list-contact-points`, `list-mute-timings`, `list-templates`, `export`, `import`, `diff` |
| Datasource inventory and replay | `datasource` | `browse`, `list`, `export`, `import`, `diff`, `add`, `modify`, `delete` |
| Access management for orgs | `access org` | `list`, `add`, `modify`, `delete`, `export`, `import` |
| Access management for users | `access user` | `list`, `add`, `modify`, `delete`, `export`, `import`, `diff` |
| Access management for teams | `access team` | `list`, `add`, `modify`, `delete`, `export`, `import`, `diff` |
| Access management for service accounts | `access service-account` | `list`, `add`, `delete`, `export`, `import`, `diff`, `token add`, `token delete` |
| Staged change and promotion workflows | `change` | `summary`, `bundle`, `bundle-preflight`, `preflight`, `assess-alerts`, `plan`, `review`, `apply`, `audit`, `promotion-preflight` |
| Project-wide staged or live reads | `overview`, `status` | `overview`, `overview live`, `status staged`, `status live` |

## Global Options

Base URLs vary slightly by surface:

- `dashboard` and `datasource` default to `http://localhost:3000`
- `alert` and `access` default to `http://127.0.0.1:3000`

The most common shared flags are:

| Option | Purpose | Typical use |
| --- | --- | --- |
| `--profile` | Load repo-local live connection defaults from `grafana-util.yaml` | Reuse one named Grafana environment without repeating `--url`, auth, timeout, or TLS flags |
| `--url` | Grafana base URL | Any live Grafana operation |
| `--token`, `--api-token` | API token auth | Scripts and non-interactive workflows |
| `--basic-user` | Basic auth username | Org switching, admin workflows, access management |
| `--basic-password` | Basic auth password | Use with `--basic-user` |
| `--prompt-token` | Prompt for token without echo | Safer interactive usage |
| `--prompt-password` | Prompt for password without echo | Safer interactive usage |
| `--timeout` | HTTP timeout in seconds | Slow APIs or unstable networks |
| `--verify-ssl` | Enable TLS certificate verification | Production TLS environments |

## Authentication Rules

The CLI treats token auth and basic auth as mutually exclusive modes. That keeps the connection model predictable and prevents a command from inheriting conflicting credentials.

1. `--token` or `--api-token` cannot be combined with `--basic-user` or `--basic-password`.
2. `--token` or `--api-token` cannot be combined with `--prompt-token`.
3. `--basic-password` cannot be combined with `--prompt-password`.
4. `--prompt-password` requires `--basic-user`.

## Output Surfaces

Choose the output shape that matches the job instead of trying to force one format everywhere:

| Surface | Best for | Representative commands |
| --- | --- | --- |
| Interactive TUI | Guided review, browsing, in-terminal workflows | `dashboard browse`, `dashboard inspect-export --interactive`, `dashboard inspect-live --interactive`, `datasource browse`, `overview --output interactive`, `status ... --output interactive` |
| Plain text | Human-readable summaries and default dry-run previews | `change`, `overview`, `status`, many dry-run summaries |
| JSON | CI, scripting, stable machine-readable handoff | import dry-runs, change documents, staged/live status contracts |
| Table / CSV / report outputs | Inventory listing, diff review, dashboard analysis | list commands, `dashboard inspect-*`, review tables |

That split matters because many commands can be read in more than one mode. The right mode depends on whether the output is for a human, a script, or a later handoff into another stage of the workflow.

## Shared Workflow Surfaces

The cross-domain surfaces sit above a single resource family. They are the places to go when the task spans multiple objects or when you need a project-level view instead of a resource-specific one.

| Surface | Inputs | Live reads | Output modes | Main use |
| --- | --- | --- | --- | --- |
| `change` | desired JSON, bundle files, lock files, availability and mapping metadata | Optional, command-dependent | text/json | staged review, preflight, plan, review, apply intent |
| `overview` | staged exports plus optional change and promotion inputs | `overview live` only | text/json/interactive | one operator-facing staged or live project snapshot |
| `status` | staged exports or live Grafana | Yes | text/json/interactive | canonical project-wide staged/live readiness surface |

`change` is the planning and application surface. `overview` is the operator summary surface. `status` is the readiness surface. That distinction keeps the handbook aligned with how operators usually move from intent, to summary, to validation.

## Resource Capability Summary

Use this table to decide whether a resource supports inventory, file export/import, or drift comparison before you open a deeper chapter.

| Resource | List | Export | Import | Diff | Inspect | Add | Modify | Delete | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dashboards | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Inventory, local authoring drafts, backup, and cross-environment migration |
| Datasources | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | Inventory, masked recovery export, replay, and controlled live mutation |
| Alert rules and alerting resources | Yes | Yes | Yes | Yes | No | No | No | No | management lane: `plan/apply/delete/init/new-*`; migration lane: `export/import/diff` |
| Organizations | Yes | Yes | Yes | No | No | Yes | Yes | Yes | Org inventory plus membership replay on import |
| Users | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | User inventory, migration, and drift comparison |
| Teams | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | Team inventory, migration, and drift comparison |
| Service accounts | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | Service account lifecycle, snapshot replay, and drift review |
| Service account tokens | Yes | No | No | No | No | Yes | No | Yes | `token add` and `token delete` workflows |

## Profiles

The repo-local `grafana-util.yaml` file lets you store live connection defaults beside the repository instead of repeating them in every command. That is the normal choice when you work against the same Grafana target repeatedly.

The profile file supports:

- named profiles
- a default profile
- environment-variable references for secrets
- timeout and SSL settings

Selection rules stay simple:

- `--profile NAME` wins when you set it.
- If you do not pass a profile, the CLI uses `default_profile` when one exists.
- If there is exactly one profile and no default, the CLI can select it automatically.
- Explicit flags override values from the chosen profile.

## Legacy Path

`dashboard list-data-sources` still exists for compatibility, but the handbook now treats it as a legacy path. New documentation and new operator workflows should prefer `datasource list`.

## Next Step

If you want to work through a real operational task, continue to [Scenarios](./scenarios.md).
