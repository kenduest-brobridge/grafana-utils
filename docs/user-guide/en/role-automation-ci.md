# Automation / CI Handbook

This page is for script authors, pipeline owners, and release engineers who need non-interactive `grafana-util` runs that are predictable, secret-safe, and easy to rotate.

## Who It Is For

- CI job authors.
- Platform engineers wiring repeatable checks into pipelines.
- Automation owners who need stable command output and clear secret handling.

## Primary Goals

- Make repeated runs work without prompts.
- Keep secrets in environment variables or a secret store, not in the command line.
- Keep the command shape simple enough that failures are easy to triage.

## Before / After

- Before: every job had to rebuild the same auth and output wiring from scratch.
- After: one env-backed profile can drive repeatable runs, readable output, and clear failure gates.

## What success looks like

- CI jobs run without prompting.
- Outputs are stable enough for scripts, gates, or parsers.
- Failures separate auth, scope, staged input, and connectivity problems.

## Failure checks

- If the job still prompts, the profile or env wiring is incomplete.
- If output looks valid but too little data appears, suspect scope before suspecting rendering.
- If a read-only gate passes but apply fails, verify the write or admin scope before changing the workflow.

## Typical Automation Tasks

- Run readiness checks in CI before promotion or apply.
- Build machine-readable summaries from staged or live state.
- Keep one profile shape that works across multiple jobs.
- Fail fast when auth scope, connectivity, or staged inputs are wrong.

## Recommended connection and secret handling

Use a profile first, with env-backed secrets for CI.

1. `--profile` with `password_env` or `token_env` for repeatable jobs and checked-in config.
2. Direct Basic auth only for bootstrap or one-off validation in a safe local shell.
3. Token auth is the normal steady state for narrow automation, as long as the token scope matches the exact resource set you need.

## First commands to run

```bash
# Purpose: First commands to run.
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
```

```bash
# Purpose: First commands to run.
grafana-util profile show --profile ci --output-format yaml
```

```bash
# Purpose: First commands to run.
grafana-util status staged --desired-file ./desired.json --output-format json
```

```bash
# Purpose: First commands to run.
grafana-util change check --desired-file ./desired.json --fetch-live --output-format json
```

```bash
# Purpose: First commands to run.
grafana-util overview live --profile ci --output-format yaml
```

If the job only needs to validate one live surface, you can replace the last line with an equivalent direct Basic-auth or narrow-token read, but do not ask the credential to see more than its real scope.

If you need a bootstrap check before the profile is wired, use Basic auth with a prompted password:

```bash
# Purpose: If you need a bootstrap check before the profile is wired, use Basic auth with a prompted password.
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

If the job already receives a scoped token, you can call the live surface directly:

```bash
# Purpose: If the job already receives a scoped token, you can call the live surface directly.
grafana-util overview live --url https://grafana.example.com --token "$GRAFANA_CI_TOKEN" --output-format json
```

## What a stable automation path looks like

Your automation path is in good shape when:

- jobs run without prompts
- the same profile can be reused across multiple checks
- outputs are machine-readable and stable enough for parsing
- failures clearly separate bad credentials, bad scope, bad staged input, and connectivity problems

## Read next

- [Getting Started](getting-started.md)
- [Technical Reference](reference.md)
- [Change & Status](change-overview-status.md)
- [Access Management](access.md)

## Keep open

- [profile](../../commands/en/profile.md)
- [status](../../commands/en/status.md)
- [change](../../commands/en/change.md)
- [overview](../../commands/en/overview.md)
- [access service-account](../../commands/en/access-service-account.md)
- [access service-account token](../../commands/en/access-service-account-token.md)
- [full command index](../../commands/en/index.md)

## Common mistakes and limits

- Do not pass raw secrets on the command line if the job can read them from `GRAFANA_CI_TOKEN` or another env-backed profile field.
- Do not treat `status staged` as `apply`; it is a gate, not the mutating step.
- Do not expect narrow tokens or service-account tokens to see every org or admin-only surface.
- Do not rely on interactive output in CI; prefer `json`, `yaml`, `table`, or explicit exit codes.
- Do not forget that `--show-secrets` is a local inspection aid, not a CI logging mode.
- Do not write ad hoc plaintext config files in the pipeline when env-backed or store-backed secret paths already exist.

## Failure triage hints

- Auth works but output looks incomplete:
  suspect token scope before suspecting the renderer.
- The same job works locally but fails in CI:
  check env injection, profile path resolution, and whether the CI runner has the same secret source available.
- Staged checks pass but apply or admin paths fail:
  verify that the job is using a credential with the required write or cross-org permissions.

## When to switch to deeper docs

- Switch to [Technical Reference](reference.md) for output formats, exit codes, and profile-backed secret guidance.
- Switch to [Change & Status](change-overview-status.md) when the pipeline needs staged gates, preflight, or promotion review.
- Switch to [Access Management](access.md) when automation starts rotating or managing service-account credentials.
- Switch to the [Command Docs](../../commands/en/index.md) when you need the exact supported flags for one namespace.

## Next steps

- [Home](index.md)
- [Getting Started](getting-started.md)
- [Technical Reference](reference.md)
- [Command Docs](../../commands/en/index.md)
