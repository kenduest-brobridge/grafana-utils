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

## Typical Automation Tasks

- Run readiness checks in CI before promotion or apply.
- Build machine-readable summaries from staged or live state.
- Keep one profile shape that works across multiple jobs.
- Fail fast when auth scope, connectivity, or staged inputs are wrong.

## Recommended Auth And Secret Approach

Use a profile first, with env-backed secrets for CI.

1. `--profile` with `password_env` or `token_env` for repeatable jobs and checked-in config.
2. Direct Basic auth only for bootstrap or one-off validation in a safe local shell.
3. Token auth is the normal steady state for narrow automation, as long as the token scope matches the exact resource set you need.

## First Commands To Run

```bash
grafana-util profile init --overwrite
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
grafana-util profile show --profile ci --output-format yaml
grafana-util status live --profile ci --output json
grafana-util overview live --profile ci --output yaml
```

If you need a bootstrap check before the profile is wired, use Basic auth with a prompted password:

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml
```

If the job already receives a scoped token, you can call the live surface directly:

```bash
grafana-util overview live --url https://grafana.example.com --token "$GRAFANA_CI_TOKEN" --output json
```

## What Good Looks Like

Your automation path is in good shape when:

- jobs run without prompts
- the same profile can be reused across multiple checks
- outputs are machine-readable and stable enough for parsing
- failures clearly separate bad credentials, bad scope, bad staged input, and connectivity problems

## Read Next

- [Getting Started](getting-started.md)
- [Technical Reference](reference.md)
- [Practical Scenarios](scenarios.md)
- [Troubleshooting](troubleshooting.md)

## Keep Open

- [profile](../../commands/en/profile.md)
- [status](../../commands/en/status.md)
- [overview](../../commands/en/overview.md)
- [dashboard](../../commands/en/dashboard.md)
- [alert](../../commands/en/alert.md)
- [access](../../commands/en/access.md)

## Common Mistakes And Limits

- Do not pass raw secrets on the command line if the job can read them from `GRAFANA_CI_TOKEN` or another env-backed profile field.
- Do not rely on interactive output in CI; prefer `json`, `yaml`, or `table`.
- Do not expect narrow tokens to see every org, dashboard, or access object.
- Do not forget that `--show-secrets` is a local inspection aid, not a CI logging mode.
- Do not treat a successful live read as proof that broader admin or multi-org automation will also succeed with the same token.

## Failure Triage Hints

- Auth works but output looks incomplete:
  suspect token scope before suspecting the renderer.
- The same job works locally but fails in CI:
  check env injection, profile path resolution, and whether the CI runner has the same secret source available.
- Staged checks pass but apply or admin paths fail:
  verify that the job is using a credential with the required write or cross-org permissions.

## When To Switch To Deeper Docs

- Switch to [Technical Reference](reference.md) for output formats, exit codes, and profile-backed secret guidance.
- Switch to [Change & Status](change-overview-status.md) when the pipeline needs staged gates, preflight, or promotion review.
- Switch to [Access Management](access.md) when automation starts rotating or managing service-account credentials.
- Switch to the [Command Docs](../../commands/en/index.md) when you need the exact supported flags for one namespace.

## Next Steps

- [Technical Reference](reference.md)
- [Command Docs](../../commands/en/index.md)
- [Best Practices & Recipes](recipes.md)
