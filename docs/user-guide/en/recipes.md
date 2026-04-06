# 🍳 Best Practices & Real-World Recipes

This chapter provides practical solutions for common Grafana operational headaches. The goal is not just to show a command, but to show when to use it, what success looks like, and what to check when the workflow goes sideways.

## Who It Is For

- Operators who want proven workflow patterns instead of starting from a blank shell.
- Teams standardizing migration, recovery, and review playbooks.
- People who need success criteria and failure checks beside each example.

## Primary Goals

- Turn common operational problems into reusable playbooks.
- Show what “good output” looks like before you continue.
- Call out when a lane or workflow is the wrong fit for the job.

## Before / After

- Before: operator examples looked like an unstructured command dump.
- After: each recipe shows the problem, the safe path, the expected result, and the failure points to check first.

## What success looks like

- You can copy a recipe and understand which step is safe to try first.
- You know what result would prove the workflow worked.
- You know which failure should stop the flow before live mutation.

## Failure checks

- If the recipe assumes the wrong source lane, stop and adjust before continuing.
- If the output or preview differs from the intended result, verify the staging inputs before you mutate live state.
- If the example feels too thin for your task, switch to the command reference for exact flags.

---

## 🚀 Recipe 1: Promoting Dashboards (Dev -> Prod)

**Problem**: Exporting from Dev and importing to Prod often fails due to hardcoded organization IDs, folder context, or source-environment datasource UIDs.

**Before**: Promotion means manually cleaning export files, guessing which lane to use, and discovering environment-specific metadata problems during import.

**After**: Promotion becomes a lane choice plus one replay path that is easier to review and explain.

**Solution**: Use the **`prompt/` lane** for a clean promotion handoff.

1. **Export from Dev**: `grafana-util dashboard export --output-dir ./dev-assets`
2. **Locate Clean Source**: Use files in `./dev-assets/prompt/`. These have environment-specific metadata stripped.
3. **Import to Prod**:
   ```bash
# Purpose: Import to Prod.
   grafana-util dashboard import --input-dir ./dev-assets/prompt --url https://prod-grafana --replace-existing
   ```

**Use this when**: the source and target environments share dashboard intent, but you do not want to replay every source-specific field literally.

**Do not use this when**: you are performing raw backup/replay or disaster recovery. In that case, start from `raw/`, not `prompt/`.

**Success looks like**:

- the import lands without source-environment-only metadata causing conflicts
- target dashboards bind to the intended target datasources and folders
- the resulting live dashboards need minimal cleanup after import

**If it fails, check**:

- whether the target datasource UIDs or names actually exist
- whether the chosen lane should have been `raw/` instead of `prompt/`
- whether your credential scope can see the target org or folder

---

## 🔍 Recipe 2: Auditing Dependencies Before Import

**Problem**: Importing a dashboard without its required datasource results in broken panels and misleading "successful" imports.

**Before**: Import appears to succeed, but the first real signal is a broken dashboard after replay.

**After**: Missing datasource dependencies are visible before import, while there is still time to fix mapping or target inventory.

**Solution**: Run a **pre-import inspection**.

```bash
# Generate a report of all required datasources in your export tree
grafana-util dashboard analyze --input-dir ./backups/raw --input-format raw --output-format dependency
```

**What to check**: Ensure every datasource listed in the dependency report exists in your target Grafana's `datasource list`.

**Use this when**: you are preparing an import, validating a promotion bundle, or checking whether a dashboard export is portable enough for another environment.

**Success looks like**:

- every required datasource UID is present in the target
- missing dependencies are known before import time
- you can explain which dashboards are blocked and why

**If it fails, check**:

- whether the target environment uses different datasource naming or UID conventions
- whether you exported the correct lane
- whether the target credentials can list the datasources you expect

---

## 🛠️ Recipe 3: Mass Tagging/Renaming (Surgical Patching)

**Problem**: You need to add a tag such as `ManagedBySRE` to many dashboards at once without hand-editing every file.

**Before**: A simple bulk change turns into manual JSON edits or a risky script with no preview.

**After**: The patch stays mechanical, reviewable, and previewed before live replay.

**Solution**: Use `patch-file` in a loop, then preview the result before replaying it.

```bash
# Purpose: Solution: Use patch-file in a loop, then preview the result before replaying it.
for file in ./dashboards/raw/*.json; do
  grafana-util dashboard patch-file --input "$file" --tag "ManagedBySRE" --output "$file"
done

grafana-util dashboard import --input-dir ./dashboards/raw --replace-existing --dry-run --table
```

**Use this when**: the structural change is local and mechanical, and you want to keep the update reviewable.

**Do not use this when**: the patch logic is so complex that a loop hides too much risk, or when the right answer depends on live discovery rather than local artifacts.

**Success looks like**:

- the modified files still review cleanly in Git
- repeated patching does not create unexpected drift
- the follow-on import is still previewed with `--dry-run` before live execution

**If it fails, check**:

- whether your loop is patching the right lane and file set
- whether the patch should have targeted `prompt/` rather than `raw/`
- whether the import should be previewed first with `--dry-run`

---

## 🚨 Recipe 4: Verifying Alert Routing Logic

**Problem**: Complex notification policies make it hard to know where an alert will land.

**Solution**: Use `preview-route` to simulate matches.

```bash
# Purpose: Solution: Use preview-route to simulate matches.
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label service=order \
  --severity critical
```

**Goal**: Verify that the `receiver` in the output matches your intended Slack channel or PagerDuty service.

**Use this when**: labels or notification policies are changing and you want a deterministic answer before anyone assumes the route is correct.

**Success looks like**:

- the resolved receiver matches the intended destination
- labels that should distinguish critical paths actually do
- route previews are reviewed before a plan/apply step

**If it fails, check**:

- whether the labels in the preview match the labels your rules will actually emit
- whether the desired alert files and notification policies are in sync
- whether the issue is route logic versus rule classification

---

## 💡 Expert Tips

- **UID consistency**: Always define stable `uid`s in your JSON. Do not rely on incremental `id`s.
- **Dry-run everything**: Use `--dry-run` to see the `ACTION=update` vs `ACTION=create` preview before making live changes.
- **Git integration**: Only commit the `raw/` and `desired/` directories to Git. These are your canonical sources.
- **Credential reality check**: Before blaming the recipe, verify that the chosen credential can really see the org, folder, or admin surface you are operating on.
- **Role split**: Use the handbook for workflow choice and the command reference when you need the exact flags for one step.

---
[⬅️ Previous: Technical Reference](reference.md) | [🏠 Home](index.md) | [➡️ Next: Troubleshooting & Glossary](troubleshooting.md)
