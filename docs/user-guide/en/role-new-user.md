# New User Handbook

This page is for someone opening `grafana-util` for the first time, or for a teammate who needs a safe local checklist before they touch live Grafana data.

## Who It Is For

- New operators learning the tool.
- Teammates validating a fresh checkout or a lab Grafana instance.
- Anyone who needs a read-only path before they own change workflows.

## Primary Goals

- Verify the binary, profile file, and live connectivity.
- Learn the safe auth path before you memorize the full command surface.
- Avoid pasting secrets into long command lines unless you are bootstrapping.

## Typical First-Day Tasks

- Confirm the installed binary is on `PATH`.
- Create one repo-local profile for a lab or dev Grafana.
- Run one safe live read and recognize the difference between `status live` and `overview live`.
- Learn which docs to keep open before moving on to dashboards, alerts, or access workflows.

## Recommended Auth And Secret Approach

Start with a repo-local profile and keep secrets out of the command line when you can.

1. `--profile` with `password_env`, `token_env`, or an OS-backed secret store for repeatable use.
2. Direct Basic auth with `--prompt-password` for quick local bootstrap or break-glass checks.
3. Token auth only when you already know the token is scoped tightly enough for the read you want.

## First Commands To Run

```bash
grafana-util --version
grafana-util profile init --overwrite
grafana-util profile example --mode basic
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
grafana-util status live --profile dev --output yaml
```

If you do not have a profile yet, use direct Basic auth once to confirm the instance is reachable:

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml
```

If you already have a scoped token, you can check the same live surface without a profile:

```bash
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```

## What Good Looks Like

You are ready to leave the new-user path when:

- `grafana-util --version` works from your normal shell
- `profile show --profile dev` resolves the fields you expect
- `status live --profile dev` returns readable output without prompting surprises
- you know whether your next step is dashboards, alerts, access, or CI automation

## Read Next

- [Getting Started](getting-started.md)
- [Technical Reference](reference.md)
- [Troubleshooting](troubleshooting.md)

## Keep Open

- [profile](../../commands/en/profile.md)
- [status](../../commands/en/status.md)
- [overview](../../commands/en/overview.md)
- [dashboard](../../commands/en/dashboard.md)

## Common Mistakes And Limits

- Do not start with token auth if you are still learning the profile rules; token scope can hide data and make the output look incomplete.
- Do not use `--show-secrets` on shared terminals or in screenshots.
- Do not expect `--all-orgs` inventory flows to work reliably with a narrow token.
- Do not assume interactive output is the best first check; plain YAML or JSON is easier to compare.

## When To Switch To Deeper Docs

- Switch to the handbook chapters when you need the workflow story behind dashboards, alerts, or staged change review.
- Switch to the command-reference pages when you are choosing exact flags, output modes, or auth variants.
- Switch to troubleshooting when the command works syntactically but the returned scope, auth, or output shape is not what you expected.

## Next Steps

- [Practical Scenarios](scenarios.md)
- [Best Practices & Recipes](recipes.md)
- [Command Docs](../../commands/en/index.md)
