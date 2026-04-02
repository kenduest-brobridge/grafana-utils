# Scenarios

This chapter translates the command families into operator workflows. Open it when you know the problem you need to solve and want a path that feels like a handbook chapter instead of a terse command stub.

Each scenario explains why the workflow exists, where it fits in the larger process, and which commands usually belong in that path.

## 1. Confirm the environment before you change anything

Use this workflow when you are on a new machine, a new checkout, or a new Grafana environment and you need to prove the CLI is pointed at the right place.

The practical flow is simple:

1. Verify the binary and help output.
2. Load a profile or pass live flags directly.
3. Run one read-only command.
4. Move into a more specific workflow only after the connection is proven.

Commands:

```bash
grafana-util --version
grafana-util -h
grafana-util profile list
grafana-util dashboard list --profile prod --table
grafana-util status live --profile prod
```

Validated version output:

```text
$ grafana-util --version
grafana-util 0.6.1
```

## 2. Inspect the deployment as an operator

Use this workflow when you want an inventory read instead of a mutation. It is the right path for onboarding, audits, and pre-change review because it gives you a fast read of the current surface without committing to any write action.

The usual sequence is:

1. Start with `dashboard list` or `datasource list`.
2. Use `overview` when you want a broader project snapshot.
3. Use `status live` when you need the canonical readiness view.
4. Move to `change` only if you are entering staged planning or promotion work.

Commands:

```bash
grafana-util dashboard list --profile prod --with-sources --table
grafana-util datasource list --profile prod --table
grafana-util overview --profile prod
grafana-util status live --profile prod
```

Validated `status live` output from the current local Docker Grafana `12.4.1` fixture:

```text
$ grafana-util status live --url http://127.0.0.1:33000 --basic-user admin --basic-password admin
Project status
Overall: status=partial scope=live domains=6 present=6 blocked=0 blockers=0 warnings=4 freshness=current
Domains:
- dashboard status=ready mode=live-dashboard-read primary=3 blockers=0 warnings=0 freshness=current next=re-run live dashboard read after dashboard, folder, or datasource changes
- datasource status=ready mode=live-inventory primary=1 blockers=0 warnings=1 freshness=current next=review live datasource secret and provider fields before export or import
- alert status=ready mode=live-alert-surfaces primary=2 blockers=0 warnings=0 freshness=current next=re-run the live alert snapshot after provisioning changes
- access status=ready mode=live-list-surfaces primary=2 blockers=0 warnings=3 freshness=current next=review live access drift-severity signals: admin users
...
```

That output is useful because it shows the project-level contract, the readiness state for each domain, and the next follow-up step if one of the live surfaces needs attention.

## 3. Export dashboards for backup or review

Use this workflow when you need a durable file-based copy of live dashboards. It fits backups, offline review, and cross-environment restore prep.

The shape of the process is:

1. Export live dashboards into a local tree.
2. Review the exported files or inspect the diff.
3. Re-import only after you have checked placement and overwrite behavior.

Commands:

```bash
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite --progress
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./backup --overwrite
```

What to look for:

- `raw/` is the reversible API-friendly form.
- `prompt/` is the UI-import-friendly form.
- `provisioning/` is the Grafana file-provisioning form.
- `export-metadata.json` summarizes the export root and the orgs that were included.

Validated output excerpt:

```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
Exporting dashboard 3/7: query-smoke
Exporting dashboard 4/7: smoke-main
Exporting dashboard 5/7: subfolder-chain-smoke
Exporting dashboard 6/7: subfolder-main
Exporting dashboard 7/7: two-prom-query-smoke
```

## 4. Restore dashboards from an export

Use this workflow when you already have a dashboard export tree and need to replay it into live Grafana. It is a controlled write path, so it belongs after export review and before any broader promotion workflow.

The practical sequence is:

1. Point the import command at the correct export root or the `raw/` tree.
2. Decide whether you are restoring one org or routing multiple exported orgs.
3. Run a dry run first when placement matters.
4. Apply the import only when the preview matches expectation.

Commands:

```bash
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --input-format raw --dry-run --table
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./backup --input-format provisioning --use-export-org --dry-run --json
```

What to look for:

- `--replace-existing` is the standard restore mode when the import should overwrite matching dashboards.
- `--ensure-folders` helps when the destination hierarchy does not exist yet.
- `--require-matching-folder-path` and `--require-matching-export-org` make the import stricter when placement must be exact.

## 5. Review and apply alerting changes

Use this workflow when alerting is the primary object of the change and you want a review-first path instead of a direct mutation.

The normal path is:

1. Scaffold or prepare the alerting resources.
2. Run `alert plan` to see what would change.
3. Use `alert apply` only after the plan matches the desired state.
4. Keep the export/import path for migration and backup use cases.

Commands:

```bash
grafana-util alert init
grafana-util alert plan --profile prod
grafana-util alert apply --profile prod
grafana-util alert list-rules --profile prod
grafana-util alert list-contact-points --profile prod
grafana-util alert list-mute-timings --profile prod
grafana-util alert list-templates --profile prod
```

The validated alert plan/apply lane is documented in the [Alert handbook](./alert.md), where the authoring, review, apply, and prune boundaries are described in full.

## 6. Rebuild access state from a controlled source

Use this workflow when you need to inspect, export, or reconstruct org, user, team, or service-account state. It is the right chapter for identity and membership work because the same surface can cover inventory and replay.

The workflow usually goes:

1. Start with `access ... list` to inventory the live state.
2. Export the objects you need for review or backup.
3. Import or modify only after you understand the target org boundaries.
4. Use the service-account token subcommands separately when you are managing credentials.

Commands:

```bash
grafana-util access org list --profile prod
grafana-util access user list --profile prod
grafana-util access team list --profile prod
grafana-util access service-account list --profile prod
grafana-util access service-account token add --profile prod
```

What to look for:

- Use `access org` when you need organization inventory or membership replay.
- Use `access user` and `access team` for identity and membership state.
- Use `access service-account` for long-lived automation identities and their tokens.

## 7. Move from staged change to promotion

Use this workflow when you are working with the cross-domain staged flow instead of touching one resource family at a time. It is the right path when the question is not just "what changed?" but "is the staged bundle ready to move?"

The path usually looks like this:

1. Build or gather the staged inputs.
2. Summarize them with `change summary` or bundle them with `change bundle`.
3. Run the relevant preflight command.
4. Review before you apply.
5. Use `overview` and `status` for the handoff and readiness check.

Commands:

```bash
grafana-util change summary
grafana-util change bundle
grafana-util change preflight
grafana-util change review
grafana-util change apply
grafana-util overview live
grafana-util status staged
```

## 8. Extend the handbook with local Docker Grafana runs

When you extend this handbook, keep the same documentation rule:

1. Run the command against the local Docker Grafana fixture first.
2. Keep the exact command and the output together in the page that explains that workflow.
3. Prefer short real excerpts over invented sample output.

The deeper command details live in [Reference](./reference.md), and the orientation flow lives in [Getting started](./getting-started.md).
