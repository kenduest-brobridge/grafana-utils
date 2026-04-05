# `grafana-util change`

## Root

用途：以先審核、再套用為主的同步工作流程，支援可選的 live Grafana 擷取與套用路徑。

適用時機：當你需要整理 desired resources、比對 live state、檢視 plan、套用已審核的 plan、稽核 drift，或產生 bundle 與 promotion 的 preflight 文件時。

說明：如果你的團隊走的是先審核、再套用的變更流程，先看這一頁最合適。`change` 指令群組把 summary、preflight、plan、review、audit 與 apply 都放在同一個控制面下，方便你先看懂整條流程，再決定要執行哪個精確子命令。

採用前後對照：

- **採用前**：變更包只是一堆檔案，真正風險通常要等到 apply 才開始浮現。
- **採用後**：同一份變更包會先走 summary、preflight、plan、review，再進入 apply，每一步都有明確檢查點。

主要旗標：root 指令本身只是指令群組；主要操作旗標都在子指令上。常見的工作流程輸入包含 `--desired-file`、`--plan-file`、`--live-file`、`--fetch-live`、`--approve`、`--execute-live`、`--source-bundle`、`--target-inventory`、`--availability-file`、`--mapping-file` 和 `--output-format`。

### 給 CI / 腳本用的 JSON contract

如果你要用 `change` 系列輸出給 CI、腳本或外部系統判斷，請先用 `kind` 與 `schemaVersion` 當成 contract 識別，再讀後面的欄位。

CLI 內建快速查詢：

- `grafana-util change --help-schema`
- `grafana-util change summary --help-schema`
- `grafana-util change plan --help-schema`
- `grafana-util change apply --help-schema`

| 指令 | 輸出 kind | 主要 top-level 欄位 |
| --- | --- | --- |
| `change summary --output-format json` | `grafana-utils-sync-summary` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`resources` |
| `change plan --output-format json` | `grafana-utils-sync-plan` | `kind`、`schemaVersion`、`toolVersion`、`dryRun`、`reviewRequired`、`reviewed`、`allowPrune`、`summary`、`alertAssessment`、`operations`、`traceId`、`stage`、`stepIndex`、`parentTraceId` |
| `change review --output-format json` | `grafana-utils-sync-plan` | 和 plan 相同，但多了 `reviewedBy`、`reviewedAt`、`reviewNote`，而且 lineage 會改成 `stage=review` |
| `change apply --output-format json` | `grafana-utils-sync-apply-intent` | `kind`、`schemaVersion`、`toolVersion`、`mode`、`reviewed`、`reviewRequired`、`allowPrune`、`approved`、`summary`、`alertAssessment`、`operations`、可選 `preflightSummary`、可選 `bundlePreflightSummary`、`appliedBy`、`appliedAt`、`approvalReason`、`applyNote`、`traceId`、`stage`、`stepIndex`、`parentTraceId` |
| `change apply --execute-live --output-format json` | live apply result | `mode`、`appliedCount`、`results` |
| `change audit --output-format json` | `grafana-utils-sync-audit` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`currentLock`、`baselineLock`、`drifts` |
| `change preflight --output-format json` | `grafana-utils-sync-preflight` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`checks` |
| `change assess-alerts --output-format json` | `grafana-utils-alert-sync-plan` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`alerts` |
| `change bundle-preflight --output-format json` | `grafana-utils-sync-bundle-preflight` | `kind`、`schemaVersion`、`summary`、`syncPreflight`、`alertArtifactAssessment`、`secretPlaceholderAssessment`、`providerAssessment` |
| `change promotion-preflight --output-format json` | `grafana-utils-sync-promotion-preflight` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`bundlePreflight`、`mappingSummary`、`checkSummary`、`handoffSummary`、`continuationSummary`、`checks`、`resolvedChecks`、`blockingChecks` |

補充：

- `change apply` 有兩種 JSON shape。沒有 `--execute-live` 時，回的是 staged apply intent；有 `--execute-live` 時，回的是 live 執行結果。
- `change summary`、`change plan`、`change review`、`change apply` 都有 `summary`，但裡面的聚合欄位會隨 stage 不同而改變。
- `change bundle` 不用 `--output-format` 來挑格式；它是用 `--output-file` 把 source bundle 寫到檔案。

成功判準：

- 在 apply 之前，就能把變更規模與風險說清楚
- staged 輸入先通過 preflight，再進 plan 或 apply
- 已審核的 plan 會留下明確審核證據，而不是只靠口頭確認

失敗時先檢查：

- 如果 summary 或 preflight 看起來不合理，先停下來，不要往 plan 或 apply 走
- 如果 live fetch 讓結果和預期差很多，先回頭比對 staged 輸入與 live 目標
- 如果 JSON 要交給自動化判斷，先驗 `kind` 和 `schemaVersion`，再解析其他欄位

範例：

```bash
# 用途：在規劃前先彙總 desired 同步資源。
grafana-util change summary --desired-file ./desired.json
```

```bash
# 用途：根據 desired 與 live state 建立已審核的 plan。
grafana-util change plan --desired-file ./desired.json --fetch-live --profile prod
```

```bash
# 用途：在明確核准後，把已審核的 plan 套用到 live Grafana。
grafana-util change apply \
  --plan-file ./sync-plan-reviewed.json \
  --approve \
  --execute-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

```bash
# 用途：先把 plan 與 live 資料接起來，再看會產生什麼變更。
grafana-util change plan \
  --desired-file ./desired.json \
  --fetch-live \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN"
```

相關指令：`grafana-util overview`、`grafana-util status`、`grafana-util snapshot`。

## `summary`

用途：彙總本機 desired 同步資源。

適用時機：當你想在規劃或套用前，先快速確認規模大小時。

主要旗標：`--desired-file`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-sync-summary`
- `schemaVersion`：目前 contract 版本
- `toolVersion`：輸出這份文件的 CLI 版本
- `summary`：聚合計數
  - `resourceCount`、`dashboardCount`、`datasourceCount`、`folderCount`、`alertCount`
- `resources`：標準化後的資源列
  - `kind`、`identity`、`title`、`managedFields`、`bodyFieldCount`、`sourcePath`

範例：

```bash
# 用途：summary。
grafana-util change summary --desired-file ./desired.json
```

```bash
# 用途：summary。
grafana-util change summary --desired-file ./desired.json --output-format json
```

相關指令：`change plan`、`change preflight`。

## `plan`

用途：根據 desired 與 live state 建立分階段的同步 plan。

適用時機：當你需要一份可供審核的 plan，確認後再標記完成或直接套用時。

採用前後對照：

- **採用前**：「這次到底會改什麼」通常只能靠人讀 desired 檔或自己猜。
- **採用後**：同一份 staged plan 會先把 create、update、delete 與 alert 受阻項目列清楚，再進 review 或 apply。

主要旗標：`--desired-file`、`--live-file`、`--fetch-live`、`--org-id`、`--page-size`、`--allow-prune`、`--trace-id`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-sync-plan`
- `schemaVersion`、`toolVersion`
- staged metadata：`dryRun`、`reviewRequired`、`reviewed`、`allowPrune`
- lineage：`traceId`、`stage`、`stepIndex`、`parentTraceId`
- `summary`
  - `would_create`、`would_update`、`would_delete`、`noop`、`unmanaged`
  - `alert_candidate`、`alert_plan_only`、`alert_blocked`
- `alertAssessment`：巢狀 alert sync 摘要與 `alerts` 列表
- `operations`：每一筆 desired/live 比對結果
  - `kind`、`identity`、`title`、`action`、`reason`、`changedFields`、`managedFields`、`desired`、`live`、`sourcePath`

範例：

```bash
# 用途：plan。
grafana-util change plan --desired-file ./desired.json --live-file ./live.json
```

```bash
# 用途：plan。
grafana-util change plan \
  --desired-file ./desired.json \
  --fetch-live \
  --profile prod \
  --output-format json
```

```bash
# 用途：plan。
grafana-util change plan \
  --desired-file ./desired.json \
  --fetch-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --allow-prune \
  --output-format json
```

```bash
# 用途：plan。
grafana-util change plan \
  --desired-file ./desired.json \
  --fetch-live \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --allow-prune \
  --output-format json
```

相關指令：`change review`、`change apply`、`change summary`。

## `review`

用途：將分階段的同步 plan 標記為已審核。

適用時機：當 plan 已經檢視完成，且在 apply 之前需要明確的審核 token 時。

採用前後對照：

- **採用前**：團隊可能口頭說「這份 plan 看過了」，但檔案本身沒有任何審核證據。
- **採用後**：staged plan 會留下誰審核、何時審核，以及審核備註，apply 前不必再靠記憶或口頭交接。

主要旗標：`--plan-file`、`--review-token`、`--reviewed-by`、`--reviewed-at`、`--review-note`、`--interactive`、`--output-format`。

JSON shape：

- 基本 shape 和 `change plan` 相同
- review state 會變成：
  - `reviewed: true`
  - `stage: review`
  - `stepIndex: 2`
- review audit 欄位：
  - `reviewedBy`
  - `reviewedAt`
  - `reviewNote`

範例：

```bash
# 用途：review。
grafana-util change review --plan-file ./sync-plan.json
```

```bash
# 用途：review。
grafana-util change review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output-format json
```

成功判準：

- 已審核的 plan 會變成一份明確可交接的產物，而不是只靠口頭批准
- 後續 apply 可以看出這份 plan 已經走過 review
- reviewer 身分與 review note 會留在結果裡，方便交接與稽核

失敗時先檢查：

- 如果 review 輸出仍顯示 `reviewed: false`，先確認你讀的是新的 reviewed 檔，而不是原本的 plan
- 如果審核資訊不完整，先檢查是否有提供 `--reviewed-by`、`--reviewed-at`、`--review-note`
- 如果後續步驟拒收 reviewed plan，先看 `stage`、`stepIndex` 和 review 欄位，不要先假設 apply 壞掉

相關指令：`change plan`、`change apply`。

## `apply`

用途：根據已審核的同步 plan 產生受控的 apply intent，並可選擇直接執行到 live。

適用時機：當 plan 已經審核完成，而你準備輸出或執行 apply 步驟時。

採用前後對照：

- **採用前**：review 完到真正動手套用之間，常常還是模糊的一步，只知道「接下來要 apply」。
- **採用後**：apply 會把這一步拆成可保存的 staged intent，或明確的 live 執行結果，並留下核准證據。

主要旗標：`--plan-file`、`--preflight-file`、`--bundle-preflight-file`、`--approve`、`--execute-live`、`--allow-folder-delete`、`--allow-policy-reset`、`--org-id`、`--output-format`、`--applied-by`、`--applied-at`、`--approval-reason`、`--apply-note`。

JSON shape：

- 預設 `change apply --output-format json`
  - `kind`：`grafana-utils-sync-apply-intent`
  - `schemaVersion`、`toolVersion`
  - `mode: apply`
  - `reviewed`、`reviewRequired`、`allowPrune`、`approved`
  - `summary`、`alertAssessment`、`operations`
  - 可選 `preflightSummary`
  - 可選 `bundlePreflightSummary`
  - `appliedBy`、`appliedAt`、`approvalReason`、`applyNote`
  - `traceId`、`stage`、`stepIndex`、`parentTraceId`
- `change apply --execute-live --output-format json`
  - `mode: live-apply`
  - `appliedCount`
  - `results`
    - 每筆包含 `kind`、`identity`、`action`、`response`

範例：

```bash
# 用途：apply。
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve
```

```bash
# 用途：apply。
grafana-util change apply \
  --plan-file ./sync-plan-reviewed.json \
  --approve \
  --execute-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

```bash
# 用途：apply。
grafana-util change apply \
  --plan-file ./sync-plan-reviewed.json \
  --approve \
  --execute-live \
  --allow-folder-delete \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN"
```

成功判準：

- 已審核的 plan 能順利進入受控 apply 步驟，不會在最後一段丟失 review lineage
- staged apply intent JSON 足夠拿去跑核准流程或變更單
- live apply 輸出會明確告訴你實際執行了幾筆操作，以及每筆結果

失敗時先檢查：

- 如果 apply 不讓你繼續，先確認輸入 plan 已經 reviewed，而且有帶 `--approve`
- 如果 live 執行結果和 staged intent 差很多，先比對 plan、本次 preflight 與目標環境，再決定要不要重跑
- 如果自動化在吃 apply JSON，先分清楚這是 staged `grafana-utils-sync-apply-intent` 還是 live `mode: live-apply` 輸出，再去讀欄位

相關指令：`change review`、`change preflight`、`change bundle-preflight`。

## `audit`

用途：比對受管 Grafana 資源的 checksum lock 與目前 live state，進行稽核。

適用時機：當你需要做 drift 檢查，或想刷新 lock snapshot 時。

主要旗標：`--managed-file`、`--lock-file`、`--live-file`、`--fetch-live`、`--org-id`、`--page-size`、`--write-lock`、`--fail-on-drift`、`--interactive`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-sync-audit`
- `schemaVersion`、`toolVersion`
- `summary`
  - `managedCount`、`baselineCount`、`currentPresentCount`、`currentMissingCount`
  - `inSyncCount`、`driftCount`、`missingLockCount`、`missingLiveCount`
- `currentLock`：目前重新建出的 lock snapshot
- `baselineLock`：既有 lock 文件，或 `null`
- `drifts`
  - `kind`、`identity`、`title`、`status`
  - `baselineStatus`、`currentStatus`
  - `baselineChecksum`、`currentChecksum`
  - `driftedFields`、`sourcePath`

範例：

```bash
# 用途：audit。
grafana-util change audit --managed-file ./desired.json --live-file ./live.json --write-lock ./sync-lock.json
```

```bash
# 用途：audit。
grafana-util change audit \
  --lock-file ./sync-lock.json \
  --fetch-live \
  --profile prod \
  --output-format json
```

```bash
# 用途：audit。
grafana-util change audit \
  --lock-file ./sync-lock.json \
  --fetch-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --fail-on-drift \
  --output-format json
```

```bash
# 用途：audit。
grafana-util change audit \
  --lock-file ./sync-lock.json \
  --fetch-live \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --fail-on-drift \
  --output-format json
```

相關指令：`change preflight`、`change plan`、`status live`。

## `preflight`

用途：根據 desired resources 與可選的 availability 提示，建立分階段的同步 preflight 文件。

適用時機：當你需要在規劃或套用前先做結構性門檻檢查時。

採用前後對照：

- **採用前**：缺資料夾、缺相依物件、政策阻擋等問題，往往要等到 plan 甚至 apply 才浮現。
- **採用後**：preflight 會先把這些檢查整理成一份獨立文件，讓你在流程還很便宜時就停下來。

主要旗標：`--desired-file`、`--availability-file`、`--fetch-live`、`--org-id`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-sync-preflight`
- `schemaVersion`、`toolVersion`
- `summary`
  - `checkCount`、`okCount`、`blockingCount`
- `checks`
  - `kind`、`identity`、`status`、`detail`、`blocking`

範例：

```bash
# 用途：preflight。
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
```

```bash
# 用途：preflight。
grafana-util change preflight \
  --desired-file ./desired.json \
  --fetch-live \
  --profile prod \
  --output-format json
```

```bash
# 用途：preflight。
grafana-util change preflight \
  --desired-file ./desired.json \
  --fetch-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-format json
```

```bash
# 用途：preflight。
grafana-util change preflight \
  --desired-file ./desired.json \
  --fetch-live \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --output-format json
```

成功判準：

- preflight 文件能清楚告訴你這份變更是否適合進下一步的 plan 或 apply
- blocking check 足夠明確，讓另一位維護者或 CI 直接停止流程
- availability 提示與 live fetch 的資料和你要操作的環境一致

失敗時先檢查：

- 如果 preflight 意外被擋，先確認 `desired` 與 `availability` 是否來自同一個環境
- 如果 live-backed preflight 看起來不對，先核對認證、org 與目標 Grafana
- 如果 CI 要解析 JSON，請先看 `kind` 與 `schemaVersion`，再讀 `summary` 和 `checks`

相關指令：`change summary`、`change plan`、`status staged`。

## `assess-alerts`

用途：評估 alert 同步規格的 candidate、plan-only 與 blocked 狀態。

適用時機：當你想在 bundling 或 apply 前，先看 alert 資源會如何分類時。

主要旗標：`--alerts-file`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-alert-sync-plan`
- `schemaVersion`、`toolVersion`
- `summary`
  - `alertCount`、`candidateCount`、`planOnlyCount`、`blockedCount`
- `alerts`
  - `identity`、`title`、`managedFields`、`status`、`liveApplyAllowed`、`detail`

範例：

```bash
# 用途：assess-alerts。
grafana-util change assess-alerts --alerts-file ./alerts.json
```

```bash
# 用途：assess-alerts。
grafana-util change assess-alerts --alerts-file ./alerts.json --output-format json
```

相關指令：`change bundle`、`change bundle-preflight`、`overview`。

## `bundle`

用途：將匯出的 dashboards、alerting 資源、datasource inventory 與 metadata 打包成單一的本機 source bundle。

適用時機：當你想要一個統一的 bundle artifact，供後續同步、審核或 preflight 使用時。

主要旗標：`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--alert-export-dir`、`--datasource-export-file`、`--datasource-provisioning-file`、`--metadata-file`、`--output-file`、`--output-format`。

JSON shape：

- 這個指令有兩種輸出路徑
  - `--output-file`：把完整 source bundle contract 寫到檔案
  - `--output-format json`：把同一份 source bundle contract 印到 stdout
- source bundle 的 top-level 欄位：
  - `kind`、`schemaVersion`、`toolVersion`
  - `summary`
  - `dashboards`、`datasources`、`folders`、`alerts`
  - 可選 `alerting`
  - 可選 `metadata`

範例：

```bash
# 用途：bundle。
grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
```

```bash
# 用途：bundle。
grafana-util change bundle --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
```

相關指令：`change bundle-preflight`、`change promotion-preflight`、`snapshot export`。

## `bundle-preflight`

用途：根據 source bundle 與 target inventory 建立分階段的 bundle-level sync preflight 文件。

適用時機：當你需要在 apply 前比較 source bundle 與 target inventory 時。

主要旗標：`--source-bundle`、`--target-inventory`、`--availability-file`、`--fetch-live`、`--org-id`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-sync-bundle-preflight`
- `schemaVersion`
- `summary`
  - `resourceCount`
  - `syncBlockingCount`
  - `providerBlockingCount`
  - `secretPlaceholderBlockingCount`
  - `alertArtifactCount`
  - `alertArtifactBlockedCount`
  - `alertArtifactPlanOnlyCount`
- `syncPreflight`：巢狀 `grafana-utils-sync-preflight`
- `providerAssessment`：provider 摘要、plans 與 checks
- `secretPlaceholderAssessment`：placeholder 摘要、plans 與 checks
- `alertArtifactAssessment`：alert artifact 摘要與 checks

範例：

```bash
# 用途：bundle-preflight。
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output-format json
```

```bash
# 用途：bundle-preflight。
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --output-format json
```

相關指令：`change bundle`、`change promotion-preflight`、`status staged`。

## `promotion-preflight`

用途：根據 source bundle 與 target inventory 建立分階段的 promotion review handoff。

適用時機：當你準備進行 promotion review，並且需要明確的 mapping 與 availability 視圖時。

主要旗標：`--source-bundle`、`--target-inventory`、`--mapping-file`、`--availability-file`、`--fetch-live`、`--org-id`、`--output-format`。

JSON shape：

- `kind`：`grafana-utils-sync-promotion-preflight`
- `schemaVersion`、`toolVersion`
- `summary`
  - `resourceCount`、`directMatchCount`、`mappedCount`
  - `missingMappingCount`、`bundleBlockingCount`、`blockingCount`
- `bundlePreflight`：巢狀 bundle-preflight 結果
- `mappingSummary`
  - `mappingKind`、`mappingSchemaVersion`、`sourceEnvironment`、`targetEnvironment`
  - `folderMappingCount`、`datasourceUidMappingCount`、`datasourceNameMappingCount`
- `checkSummary`
  - `folderRemapCount`、`datasourceUidRemapCount`、`datasourceNameRemapCount`
  - `resolvedCount`、`directCount`、`mappedCount`、`missingTargetCount`
- `handoffSummary`
- `continuationSummary`
- `checks`、`resolvedChecks`、`blockingChecks`
  - 每筆包含 `kind`、`identity`、`sourceValue`、`targetValue`、`resolution`、`mappingSource`、`status`、`detail`、`blocking`

範例：

```bash
# 用途：promotion-preflight。
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --output-format json
```

```bash
# 用途：promotion-preflight。
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --availability-file ./availability.json --output-format json
```

相關指令：`change bundle-preflight`、`change apply`、`status live`。
