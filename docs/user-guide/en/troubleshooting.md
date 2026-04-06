# 🔍 Troubleshooting & Glossary

Use this chapter when a command looks syntactically correct but the result is wrong, incomplete, or inconsistent with what you expected.

## Who It Is For

- Operators who can run the command, but do not trust the outcome yet.
- Engineers trying to separate auth, scope, staged-input, and live-state problems.
- Teams building a repeatable debugging flow instead of ad hoc trial and error.

## Primary Goals

- Identify the category of failure before changing credentials or inputs.
- Point you to the right companion chapter or command reference quickly.
- Keep debugging focused on facts you can verify from the CLI.

## Before / After

- Before: people had to guess whether a bad result was a syntax problem, a scope problem, or a workflow problem.
- After: the troubleshooting chapter separates connection, scope, staged input, and output-shape problems into readable checks.

## What success looks like

- You can identify the likely failure class before reading every command page.
- You can decide whether to fix auth, scope, staged input, or output formatting.
- You can tell when the problem belongs in the workflow chapter instead of here.

## Failure checks

- If the symptom is still unclear after the first check, do not continue mutating live state.
- If a command syntax error hides the real problem, verify the selected lane and auth source before changing anything else.
- If the output shape is surprising but the command exits cleanly, check the documented contract before assuming the renderer is wrong.

It helps you diagnose real failures instead of guessing. The most useful split is not just "error code versus no error code", but:

- auth versus scope
- live versus staged
- command shape versus output shape
- local profile wiring versus remote Grafana behavior

If you are tracing auth or connection setup, keep [profile](../../commands/en/profile.md), [status](../../commands/en/status.md), [overview](../../commands/en/overview.md), and [access](../../commands/en/access.md) open beside this chapter.

---

## 🛠️ Debugging the CLI

When a command fails or behaves unexpectedly, use these techniques to look under the hood.

### 1. Enable verbose logging

`grafana-util` uses standard Rust logging. You can increase verbosity to see the exact API requests and responses.

```bash
# Purpose: grafana-util uses standard Rust logging. You can increase verbosity to see the exact API requests and responses.
RUST_LOG=debug grafana-util overview live --profile prod
grafana-util dashboard list -v
```

Use this when you need to answer:

- did the CLI call the host you expected?
- was the request rejected by auth, scope, or network?
- did the command shape differ from what the docs led you to expect?

### 2. Common errors and fast fixes

| Symptom | Probable Cause | Fix |
| :--- | :--- | :--- |
| `401 Unauthorized` | Invalid token or user/password | Check your profile or environment variables |
| `403 Forbidden` | Credential is valid but lacks required scope | Verify role/permissions or retry with broader admin credentials |
| `Connection Refused` | Wrong URL or firewall/network block | Verify `--url` and network reachability to Grafana |
| `Timeout` | Large estate or slow backend | Increase `--timeout` and retry with a narrower scope first |

### 3. Scope and auth mismatches

These are some of the most confusing failures because the command may "work" but still return incomplete or misleading results.

| Symptom | Likely Cause | What to check next |
| :--- | :--- | :--- |
| `--all-orgs` returns fewer orgs than expected | token scope is narrower than the requested read | retry with an admin-backed profile or direct Basic auth |
| read-only status works but access/admin commands fail | credential is valid, but not broad enough | compare the scope of the current credential with the command family you are calling |
| token works in one job but fails in another | the second job is using a broader surface than the token was meant for | check whether the workflow should be profile-backed Basic auth instead |

Rule of thumb:

- auth success does not automatically imply scope success
- if the output looks suspiciously partial, suspect scope before suspecting parsing

### 4. Staged vs live confusion

This is one of the most common operator mistakes.

| Symptom | What is really happening | Fix |
| :--- | :--- | :--- |
| `status staged` looks healthy but live apply still fails | staged files are structurally valid, but live state or permissions differ | run `status live`, then `change check`, `change preview`, or command-specific dry-run paths |
| `overview live` looks good so you skip change review | live readability is not the same as staged correctness | run the staged gate and preview path before apply |
| import or apply changes more than expected | the staged package was never inspected or previewed first | use `change inspect`, `change preview`, and `--dry-run` before execution |

### 5. Secret and profile problems

| Symptom | Likely Cause | Fix |
| :--- | :--- | :--- |
| `profile show --show-secrets` fails to resolve a value | missing env var, missing OS-store entry, or missing encrypted secret file/key | verify the secret source that the profile points to |
| profile works locally but not in CI | env injection or config path differs between environments | check `GRAFANA_UTIL_CONFIG`, env vars, and any required secret files |
| `--store-secret os` works on macOS but not Linux | Linux runner may not have a working Secret Service session | switch to `password_env`, `token_env`, or `encrypted-file` |

### 6. Output-mode mistakes

| Symptom | Likely Cause | Fix |
| :--- | :--- | :--- |
| parser in CI breaks unexpectedly | a human-oriented output mode was used | prefer `json`, `yaml`, or another machine-readable mode |
| a command rejects one `--output-format` value | that command only supports a narrower set of formats | check the current command help or command-reference page |
| interactive output is confusing in a first-run check | too much presentation, not enough raw signal | switch to `yaml` or `json` first |

---

## 📖 Glossary of Terms

| Term | Definition |
| :--- | :--- |
| **Surface** | A high-level interface category such as `Status`, `Overview`, or `Change` |
| **Lane** | An isolated data path for assets such as `raw/`, `prompt/`, or `provisioning/` |
| **Contract** | A machine-readable JSON document that defines a readiness or compatibility expectation |
| **Masked Recovery** | Exporting secrets in masked form, then re-injecting them during import/replay |
| **Desired State** | The goal configuration stored in Git that the CLI compares against live Grafana |
| **Drift** | The gap between live Grafana configuration and the local staged or desired artifacts |

---

## 🆘 Getting More Help

- **Check the version**: always run `grafana-util --version` when reporting issues.
- **Project repository**: report bugs or request features on the [GitHub Issues](https://github.com/kenduest-brobridge/grafana-util/issues) page.

When escalating an issue, include:

- the exact command
- whether the command was live or staged
- whether you used `--profile`, direct Basic auth, or token auth
- whether the failure looked like syntax, connectivity, scope, or staged-input shape

---
[⬅️ Previous: Best Practices & Recipes](recipes.md) | [🏠 Home](index.md)
