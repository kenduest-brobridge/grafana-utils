# What grafana-util is for

`grafana-util` is not just a thin wrapper around the Grafana HTTP API, and it is not only a backup/export tool. Its job is to connect day-to-day operator workflows around status checks, exports, local workspace review, and safe apply into one consistent CLI.

If you regularly hit problems like these, this is the tool solving them:

- you need a fast picture of what exists across a Grafana estate, not just one UI screen at a time
- you want to work from a local Grafana workspace or package instead of doing everything by hand in the UI
- you want to know what a workspace will do before you apply it
- you want to keep exports in Git, CI/CD, or review workflows without dumping secrets into plain files
- you want repeatable operator workflows instead of rebuilding command lines from scratch every time

---

## Before / After

| Before | After with `grafana-util` |
| :--- | :--- |
| "What changed?" means opening several Grafana screens or hand-rolling API calls. | Start with `status live`, `status overview`, or `workspace scan` and get one review surface first. |
| Export/import is a fragile action with little context. | Export, inspect, dry-run, and replay in an explicit sequence. |
| Alerting or access changes are hard to explain in review. | Plans, summaries, and structured output make the intended workspace visible before apply. |
| Secrets and auth defaults get repeated across shell history and scripts. | Profiles and secret modes make repeated workflows cleaner and safer. |

This is the main design difference: the tool is trying to improve the operating path, not just shorten one command.

## What success looks like

- You can point to one operational pain point this tool removes.
- You can name the workflow lane you would use first: status, export, workspace review, or apply.
- You know whether this repository should help you more than a one-off shell script or Grafana UI click path.

## Failure checks

- If the problem is a one-off UI edit, this tool is probably not the first thing you need.
- If you cannot say which workflow lane you need, start from the role path pages before opening command docs.
- If you only want exact syntax, switch to the command reference instead of expecting this page to be a command manual.

---

## What it is

`grafana-util` is best understood as a Grafana operations workflow tool, not a single-purpose CLI.

It is a personal long-term tool, not a complete platform and not an attempt to cover every Grafana API surface. The point is to make the high-friction operator paths easier to review, repeat, and automate without losing context.

It breaks the work into a few clear surfaces:

- **Status and observation**: start with `status`
- **Asset operations**: use `dashboard`, `datasource`, `alert`, and `access`
- **Workspace review**: use `workspace` to scan, test, preview, package, and apply through one task-first lane
- **Connection and credentials**: use `config profile` to keep URLs, auth defaults, and secret sources repeatable

The goal is not to memorize every command first. The goal is to know what kind of work you are doing.

## Design orientation in context

If you already know tools such as `grafanactl` or `grizzly`, it is more accurate to think in terms of design orientation than competition:

- `grafanactl` is closer to a general resource and API-oriented Grafana CLI.
- `grizzly` is closer to declarative Grafana-as-code management.
- `grafana-util` is more focused on reviewable operations, inspection/governance flows, and safer migration or replay paths.

These tools can overlap. The useful question is which working style you need first.

---

## Feature overview table

| Area | Main command | What you use it for |
| :--- | :--- | :--- |
| Readiness and health checks | `status live` / `status staged` | Check whether live or staged state is healthy enough to move forward |
| Estate-wide overview | `status overview` | Get a fast picture of the Grafana estate and decide where to drill in next |
| Dashboard operations | `dashboard` | Browse, list, export/import, diff, review, patch, summary, dependencies, policy, screenshot, and raw-to-prompt conversion |
| Data source operations | `datasource` | Inventory, export, import, diff, mutation, and recovery for data sources |
| Alert governance | `alert` | Alert rules, notification routing, contact points, and plan/apply workflows |
| Identity and access | `access` | Manage orgs, users, teams, service accounts, and tokens |
| Workspace review | `workspace` | Scan, test, preview, package, and apply a local Grafana workspace |
| Connection and credentials | `config profile` | Keep URLs, auth defaults, and secret sources repeatable |

If all you need is “where do I start?”, use this table first, then move into the matching handbook chapter.

## What to surface where

Keep the README and handbook home pages short. Show the lanes people reach for first, then leave the deeper command tree in the docs index.

- Show on the README/home page: `status live`, `status overview`, `export dashboard|alert|datasource`, `workspace scan/test/preview/apply`, `config profile`, `dashboard browse/list/export/import/diff/review/patch/summary/dependencies/policy`, `alert`, `datasource`, and `access`.
- Keep in the docs index and per-command pages: `dashboard get/clone/serve/edit-live/delete/history/variables/impact/screenshot/convert raw-to-prompt`, `datasource browse/types/list/add/modify/delete`, `snapshot`, `resource`, and the compatibility alias pages.

---

## Where it helps most

### 1. Daily operations and checks

You want quick answers to questions like:

- what dashboards, alerts, and data sources exist right now?
- does live state look healthy?
- where does the estate already look like it is drifting?

This usually starts with `status live` or `status overview`.

### 2. Export, migration, and replay

You want to move dashboards or data sources between environments, or keep a replayable export tree. That usually means you need more than just one export command:

- export into the right lane
- inspect, diff, or dry-run first
- then decide whether to import or replay

### 3. Review before mutation

You do not want to apply changes blind. You want answers to:

- what will this actually change?
- is the staged input complete?
- are routes, secrets, dependencies, and permissions in a sane state?

That is where `workspace scan`, `workspace test`, `workspace preview`, and `alert plan` matter.

### 4. Automation and CI/CD

You want Grafana operations to fit into scripts, pipelines, or scheduled jobs instead of depending on manual UI work.

That usually means:

- using `config profile` or env-backed auth cleanly
- choosing stable output formats
- keeping gates and review steps before mutation

---

## What a first successful path looks like

If the tool is a good fit, a first successful session usually looks like this:

1. confirm the binary and one safe live read
2. export one reviewable asset tree
3. scan that workspace before test
4. preview a workspace before apply

That path proves the core value faster than reading every command page first.

---

## What it is not trying to replace

There are cases where `grafana-util` is not the first thing you need:

- you are making one tiny change in the Grafana UI
- you only need to inspect one value on one screen
- you do not need review, export, migration, replay, or automation

If the work does not need a repeatable or reviewable trail, the Grafana UI may still be the faster path.

---

## A good way to start

If this is your first time, do not start by reading every command page. A better sequence is:

1. understand the supported connection and auth patterns
2. run one safe read-only command
3. choose the path that fits you: new user, SRE/operator, or automation
4. open command docs only when you need exact syntax and flags

If you are starting now, read these next:

- [Getting Started](getting-started.md)
- [New User Path](role-new-user.md)
- [Command Docs](../../commands/en/index.md)

---
[⬅️ Back to handbook home](index.md) | [➡️ Next: Getting Started](getting-started.md)
