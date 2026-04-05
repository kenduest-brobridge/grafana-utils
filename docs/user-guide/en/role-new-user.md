# New User Handbook

This page is for someone opening `grafana-util` for the first time, or for a teammate who needs a safe local checklist before touching live Grafana data.

The first thing to understand is that `grafana-util` does not force one auth style. You can connect in several ways:

- pass the Grafana URL and credentials directly on each command
- prompt for a password or token instead of pasting it into the shell
- rely on environment variables that the CLI already knows how to read
- save repeatable defaults in a repo-local profile and call them back with `--profile`

Profiles matter because they remove repetition, not because direct flags are unsupported. The normal learning path is:

1. prove the binary can reach Grafana with one safe read
2. understand which auth form you are using
3. move the repeatable parts into a profile once you know the connection works

## Who It Is For

- New operators learning the tool.
- Teammates validating a fresh checkout or a lab Grafana instance.
- Anyone who needs a read-only path before they own change workflows.

## Primary Goals

- Verify the binary, live connectivity, and profile file behavior.
- Learn the supported auth inputs before memorizing the full command surface.
- Understand when direct flags are enough and when a profile becomes the cleaner choice.
- Avoid pasting secrets into long command lines unless you are bootstrapping.

## Before / After

- Before: new users had to guess whether they should start with a profile, a token, or direct flags.
- After: prove one direct read-only connection first, then move the repeatable parts into a profile.

## What success looks like

- You can run one safe live command and understand the output.
- You know which auth form you are using and why.
- You can explain when a profile is useful and when direct flags are enough.

## Failure checks

- If the first live read fails, stop and fix connection or auth before touching mutation workflows.
- If you do not know which auth source is active, inspect the command line and profile file before proceeding.
- If you are tempted to paste secrets into shell history, switch to env-backed or prompted auth first.

## Typical First-Day Tasks

- Confirm the installed binary is on `PATH`.
- Run one direct read-only command against a lab or dev Grafana.
- Create one repo-local profile once the direct connection works.
- Run one safe live read and recognize the difference between `status live` and `overview live`.
- Learn which docs to keep open before moving on to dashboards, alerts, or access workflows.

## How Connection And Auth Work

`grafana-util` accepts connection details in more than one place.

- `--url` selects the Grafana base URL.
- `--basic-user` plus `--basic-password`, or `--prompt-password`, uses Basic auth.
- `--token`, or `--prompt-token`, uses token auth.
- `GRAFANA_USERNAME`, `GRAFANA_PASSWORD`, and `GRAFANA_API_TOKEN` can supply the same credentials through environment variables.
- `--profile` loads the reusable defaults stored in `grafana-util.yaml`.

That means you can always start with a one-off command, then move to a profile later if you do not want to keep repeating the same URL and auth flags.

## Recommended Auth And Secret Approach

Use the auth modes in this order:

1. Direct Basic auth with `--prompt-password` for a first local bootstrap or a one-time reachability check.
2. `--profile` with `password_env`, `token_env`, or an OS-backed secret store for repeatable day-to-day work.
3. Direct token auth only when you already know the token is scoped tightly enough for the read you want.

The key idea is simple: direct flags prove the connection, profiles simplify repeated use.

## First Commands To Run

```bash
# Purpose: First Commands To Run.
grafana-util --version
```

```bash
# Purpose: First Commands To Run.
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

```bash
# Purpose: First Commands To Run.
grafana-util profile init --overwrite
```

```bash
# Purpose: First Commands To Run.
grafana-util profile example --mode basic
```

```bash
# Purpose: First Commands To Run.
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
```

```bash
# Purpose: First Commands To Run.
grafana-util status live --profile dev --output-format yaml
```

The sequence matters:

- first prove that the binary can talk to Grafana with one direct command
- then initialize a repo-local config
- then add a reusable profile for the same target
- then run the same safe read again through `--profile`

If you do not have a profile yet, this is the shortest safe bootstrap:

```bash
# Purpose: If you do not have a profile yet, this is the shortest safe bootstrap.
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

If you already have a scoped token, you can check the same live surface without a profile:

```bash
# Purpose: If you already have a scoped token, you can check the same live surface without a profile.
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

If your shell already exports auth variables, you can also keep the command shorter:

```bash
# Purpose: If your shell already exports auth variables, you can also keep the command shorter.
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
grafana-util status live --url http://localhost:3000 --output-format yaml
```

## What Good Looks Like

You are ready to leave the new-user path when:

- `grafana-util --version` works from your normal shell
- one direct read-only command succeeds against the Grafana you care about
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

- Do not assume profiles are mandatory before the first connectivity check; one direct command is a good starting point.
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
