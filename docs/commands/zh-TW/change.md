# `grafana-util change`

## Root

用途：以 task-first 的 staged change 工作流程為主，支援可選的 live Grafana preview 與 apply 路徑。

適用時機：當你需要 inspect staged package、check 它是否適合往下走、preview 會改到什麼、apply 已審核的 preview，或在必要時切進較低階的 advanced contract 時。

說明：如果你要先從正常維運路徑開始，先看這一頁最合適。現在預設的 `change` 操作面是 `inspect -> check -> preview -> apply`。較低階的 `summary`、`plan`、`review`、`preflight`、`audit`，以及 bundle / promotion 工作流，仍然保留在 `change advanced` 下面，給需要明確 staged contract 的情境使用。

採用前後對照：

- **採用前**：變更包只是一堆檔案，真正風險通常要等到 apply 才開始浮現。
- **採用後**：同一份變更包會先走 inspect、check、preview、apply，每一步都有明確檢查點；advanced contracts 則收在 `change advanced`。

主要旗標：root 指令本身只是指令群組；主要操作旗標都在子指令上。常見工作流程輸入包含 `--workspace`、`--desired-file`、`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--alert-export-dir`、`--source-bundle`、`--target-inventory`、`--availability-file`、`--mapping-file`、`--fetch-live`、`--live-file`、`--preview-file`、`--approve`、`--execute-live` 與 `--output-format`。

### 第一次使用時，先走這條路

如果你是第一次操作，先照這個順序走：

1. `change inspect` 看 staged package 裡有什麼
2. `change check` 確認輸入結構是否適合繼續往下走
3. `change preview` 預覽這次會改到什麼
4. `change apply` 只在 preview 已審核且核准後才執行

### `--workspace` 會幫你找什麼

當你帶 `--workspace .` 時，`change` 會嘗試在目前 repo 或工作目錄裡找常見的 staged inputs，並拼成同一條 review lane，讓同一個 repo root 承載多個 surface 的 source provenance：

- dashboard export trees
- dashboard provisioning trees
- 包含 `dashboards/git-sync/raw` 與 `dashboards/git-sync/provisioning` 的 Git Sync workspace roots
- datasource provisioning files
- alert export trees
- staged desired change files
- source bundle、target inventory、promotion mapping files

如果自動發現沒有找到可用輸入，先停下來，改用明確旗標，例如 `--desired-file`、`--dashboard-export-dir`、`--alert-export-dir`、`--source-bundle` 或 `--target-inventory`。

成功判準：

- 在 apply 之前，就能把變更規模與風險說清楚
- staged 輸入先通過 check，再進 preview 或 apply
- 已審核的 preview / plan 會留下明確證據，而不是只靠口頭確認

失敗時先檢查：

- 如果 inspect 或 check 看起來不合理，先停下來，不要往 preview 或 apply 走
- 如果 live fetch 讓結果和預期差很多，先回頭比對 staged 輸入與 live 目標
- 如果 JSON 要交給自動化判斷，先驗 `kind` 和 `schemaVersion`，再解析其他欄位

範例：

```bash
# 用途：先從同一個 mixed repo root 看 staged package 的形狀。
grafana-util change inspect --workspace ./grafana-oac-repo
```

同一個 `--workspace` root 可以同時包含 `dashboards/git-sync/raw`、`dashboards/git-sync/provisioning`、`alerts/raw` 與 `datasources/provisioning/datasources.yaml`。

混合 workspace tree 範例：

```text
./grafana-oac-repo/
  dashboards/git-sync/raw/
  dashboards/git-sync/provisioning/
  alerts/raw/
  datasources/provisioning/datasources.yaml
```

```bash
# 用途：先檢查 staged package 是否適合往下走。
grafana-util change check --workspace ./grafana-oac-repo --fetch-live --output-format json
```

```bash
# 用途：先預覽這次會改到什麼，再決定是否套用。
grafana-util change preview --workspace ./grafana-oac-repo --fetch-live --profile prod
```

```bash
# 用途：在明確核准後，把已審核的 preview 套用到 live Grafana。
grafana-util change apply \
  --preview-file ./change-preview.json \
  --approve \
  --execute-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

相關指令：`grafana-util observe overview`、`grafana-util observe staged`、`grafana-util snapshot`。

## 主要子指令

先用 root 頁了解整體 lane，再依你現在正在做的步驟打開對應的子頁：

- [change inspect](./change-inspect.md)：確認 staged package 裡到底有什麼
- [change check](./change-check.md)：確認 staged package 在結構上是否適合往下走
- [change preview](./change-preview.md)：建立可審查的變更預覽
- [change apply](./change-apply.md)：把已審核的 preview 轉成 apply intent，或真的套用到 live

這樣拆是刻意的。`change` 本身是 namespace，下面同時有一條 primary lane 和一組 advanced contracts；真正逐步操作的 manual，應該看各個子指令頁，而不是把所有細節都塞回 root。

## `advanced`

用途：暴露較低階的 staged contracts 與特殊 sync workflows。

適用時機：當你需要明確的 `summary`、`plan`、`review`、`preflight`、`audit`、`bundle` 或 promotion handoff 文件，而不是 task-first lane。

範例：

```bash
# 用途：只有在 primary lane 不夠用時，才進入較低階 contract 或特殊 sync workflow。
grafana-util change advanced bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --output-format json
```

### 給 CI / 腳本用的區塊

如果你是用這頁做一般維運操作，看到這裡就可以先停。除非你已經知道自己需要較低階 contract，否則先走 `inspect -> check -> preview -> apply` 的 primary lane。

如果你要用 `change` 系列輸出給 CI、腳本或外部系統判斷，請先用 `kind` 與 `schemaVersion` 當成 contract 識別，再讀後面的欄位。

CLI 內建快速查詢：

- `grafana-util change --help-schema`
- `grafana-util change inspect --help`
- `grafana-util change preview --help-schema`
- `grafana-util change apply --help-schema`

| 指令 | 輸出 kind | 主要 top-level 欄位 |
| --- | --- | --- |
| `change inspect --output-format json` | staged summary 或 overview/status 風格的 inspection 輸出 | 依輸入類型輸出 staged 摘要與 discovered-input 資訊 |
| `change check --output-format json` | project-status staged status | staged readiness/status 輸出，以及 blockers 或 warnings |
| `change preview --output-format json` | `grafana-utils-sync-plan` 或 bundle/promotion preflight kinds | task-first 入口會沿用既有 staged plan/bundle-preflight/promotion-preflight contracts；sync-plan preview 還會帶 `ordering.mode`、`operations[].orderIndex`、`operations[].orderGroup`、`operations[].kindOrder` 與 `summary.blocked_reasons` |
| `change apply --output-format json` | `grafana-utils-sync-apply-intent` | `kind`、`schemaVersion`、`toolVersion`、`mode`、`reviewed`、`reviewRequired`、`allowPrune`、`approved`、`summary`、`alertAssessment`、`operations`、可選 `preflightSummary`、可選 `bundlePreflightSummary`、`appliedBy`、`appliedAt`、`approvalReason`、`applyNote`、`traceId`、`stage`、`stepIndex`、`parentTraceId`；排序資訊保留在 reviewed preview |
| `change apply --execute-live --output-format json` | live apply result | `mode`、`appliedCount`、`results` |
| `change advanced audit --output-format json` | `grafana-utils-sync-audit` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`currentLock`、`baselineLock`、`drifts` |
| `change advanced preflight --output-format json` | `grafana-utils-sync-preflight` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`checks` |
| `change advanced assess-alerts --output-format json` | `grafana-utils-alert-sync-plan` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`alerts` |
| `change advanced bundle-preflight --output-format json` | `grafana-utils-sync-bundle-preflight` | `kind`、`schemaVersion`、`summary`、`syncPreflight`、`alertArtifactAssessment`、`secretPlaceholderAssessment`、`providerAssessment` |
| `change advanced promotion-preflight --output-format json` | `grafana-utils-sync-promotion-preflight` | `kind`、`schemaVersion`、`toolVersion`、`summary`、`bundlePreflight`、`mappingSummary`、`checkSummary`、`handoffSummary`、`continuationSummary`、`checks`、`resolvedChecks`、`blockingChecks` |

補充：

- `change apply` 有兩種 JSON shape。沒有 `--execute-live` 時，回的是 staged apply intent；有 `--execute-live` 時，回的是 live 執行結果。
- `change preview` 是 task-first 入口。依你提供的 staged 輸入不同，可能輸出既有的 plan kind，或 bundle/promotion preflight kinds。
- `change apply` 現在優先使用 `--preview-file`，但仍保留 `--plan-file` 當 alias。
- `change bundle` 不用 `--output-format` 來挑格式；它是用 `--output-file` 把 source bundle 寫到檔案。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-summary",
  "summary": {
    "resourceCount": 8,
    "dashboardCount": 5,
    "datasourceCount": 1
  },
  "resources": []
}
```
這是最快的「這包變更到底多大」摘要，適合在進 preflight 或 plan 前先看一眼。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-plan",
  "reviewed": false,
  "summary": {
    "would_create": 1,
    "would_update": 4,
    "would_delete": 0
  },
  "operations": []
}
```
這通常就是團隊真正拿來審查的主文件。先看 `summary` 的計數，再看這份 plan 是否仍然是未審核狀態。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-plan",
  "reviewed": true,
  "stage": "review",
  "stepIndex": 2,
  "reviewNote": "peer-reviewed"
}
```
review 成功後，apply 不需要再靠檔名或口頭交接去猜這份 plan 是否已經審核。

成功判準：

- 已審核的 plan 會變成一份明確可交接的產物，而不是只靠口頭批准
- 後續 apply 可以看出這份 plan 已經走過 review
- reviewer 身分與 review note 會留在結果裡，方便交接與稽核

失敗時先檢查：

- 如果 review 輸出仍顯示 `reviewed: false`，先確認你讀的是新的 reviewed 檔，而不是原本的 plan
- 如果審核資訊不完整，先檢查是否有提供 `--reviewed-by`、`--reviewed-at`、`--review-note`
- 如果後續步驟拒收 reviewed plan，先看 `stage`、`stepIndex` 和 review 欄位，不要先假設 apply 壞掉

相關指令：`change plan`、`change apply`。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-audit",
  "summary": {
    "inSyncCount": 12,
    "driftCount": 1
  },
  "drifts": []
}
```
這裡最重要的頂層訊號是 `driftCount`。只要大於 0，就代表 managed state 和 live Grafana 已經開始分離。

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

相關指令：`change preflight`、`change plan`、`observe live`。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-preflight",
  "summary": {
    "checkCount": 6,
    "okCount": 6,
    "blockingCount": 0
  },
  "checks": []
}
```
這是結構性 gate。只要 `blockingCount` 不是 0，就應該先停在這裡修輸入，而不是往下做 plan 或 apply。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-alert-sync-plan",
  "summary": {
    "alertCount": 3,
    "candidateCount": 2,
    "blockedCount": 1
  },
  "alerts": []
}
```
如果你覺得整份 sync plan 對 alert 太寬，這個指令就是把 alert 那一塊單獨拉出來看。

相關指令：`change bundle`、`change bundle-preflight`、`overview`。

## `bundle`

用途：將匯出的 dashboards、alerting 資源、datasource inventory 與 metadata 打包成單一的本機 source bundle；尤其適合在你已經從同一個 mixed workspace root 發現 source provenance 之後，把同一批內容封裝成可交接的 bundle。

適用時機：當你想要一個統一的 bundle artifact，供後續同步、審核或 preflight 使用時。

主要旗標：`--workspace`、`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--alert-export-dir`、`--datasource-export-file`、`--datasource-provisioning-file`、`--metadata-file`、`--output-file`、`--output-format`。

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

```bash
grafana-util change bundle --workspace ./grafana-oac-repo --output-file ./sync-source-bundle.json
```

這條 bundle 路徑就是把前面 inspect、check、preview 已經發現的同一個 mixed workspace tree，整理成可交接的 bundle artifact，而且也會保留同樣的 top-level `discovery` 區塊作為 provenance。

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-source-bundle",
  "summary": {
    "dashboardCount": 5,
    "datasourceCount": 1,
    "alertCount": 3
  },
  "dashboards": [],
  "alerts": []
}
```
不管是寫到檔案還是印到 stdout，這都是後續 bundle / promotion 檢查會接手的 packaging artifact。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-bundle-preflight",
  "summary": {
    "resourceCount": 9,
    "syncBlockingCount": 0,
    "providerBlockingCount": 0
  },
  "syncPreflight": {}
}
```
當你想先回答「這包東西整體能不能進下一步」時，bundle-preflight 比單純 plan 更貼近 promotion / handoff 場景。

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

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-promotion-preflight",
  "summary": {
    "resourceCount": 9,
    "directMatchCount": 6,
    "mappedCount": 3,
    "blockingCount": 0
  },
  "mappingSummary": {},
  "blockingChecks": []
}
```
這份文件要回答的不是「會改什麼」，而是「在目前 mapping 下，這包 source bundle 能不能安全 promotion 到目標環境」。

相關指令：`change bundle-preflight`、`change apply`、`observe live`。
