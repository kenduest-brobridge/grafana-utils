# `grafana-util change`

## Root

用途：以先審核、再套用為主的同步工作流程，支援可選的 live Grafana 擷取與套用路徑。

適用時機：當你需要整理 desired resources、比對 live state、檢視 plan、套用已審核的 plan、稽核 drift，或產生 bundle 與 promotion 的 preflight 文件時。

主要旗標：root 指令本身只是命名空間；主要操作旗標都在子指令上。常見的工作流程輸入包含 `--desired-file`、`--plan-file`、`--live-file`、`--fetch-live`、`--approve`、`--execute-live`、`--source-bundle`、`--target-inventory`、`--availability-file`、`--mapping-file` 和 `--output`。

範例：

```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change plan --desired-file ./desired.json --fetch-live --profile prod
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --basic-user admin --basic-password admin
grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"
```

相關指令：`grafana-util overview`、`grafana-util status`、`grafana-util snapshot`。

## `summary`

用途：彙總本機 desired 同步資源。

適用時機：當你想在規劃或套用前，先快速確認規模大小時。

主要旗標：`--desired-file`、`--output`。

範例：

```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change summary --desired-file ./desired.json --output json
```

相關指令：`change plan`、`change preflight`。

## `plan`

用途：根據 desired 與 live state 建立分階段的同步 plan。

適用時機：當你需要一份可供審核的 plan，確認後再標記完成或直接套用時。

主要旗標：`--desired-file`、`--live-file`、`--fetch-live`、`--org-id`、`--page-size`、`--allow-prune`、`--trace-id`、`--output`。

範例：

```bash
grafana-util change plan --desired-file ./desired.json --live-file ./live.json
grafana-util change plan --desired-file ./desired.json --fetch-live --profile prod --output json
grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --basic-user admin --basic-password admin --allow-prune --output json
grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --allow-prune --output json
```

相關指令：`change review`、`change apply`、`change summary`。

## `review`

用途：將分階段的同步 plan 標記為已審核。

適用時機：當 plan 已經檢視完成，且在 apply 之前需要明確的審核 token 時。

主要旗標：`--plan-file`、`--review-token`、`--reviewed-by`、`--reviewed-at`、`--review-note`、`--interactive`、`--output`。

範例：

```bash
grafana-util change review --plan-file ./sync-plan.json
grafana-util change review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output json
```

相關指令：`change plan`、`change apply`。

## `apply`

用途：根據已審核的同步 plan 產生受控的 apply intent，並可選擇直接執行到 live。

適用時機：當 plan 已經審核完成，而你準備輸出或執行 apply 步驟時。

主要旗標：`--plan-file`、`--preflight-file`、`--bundle-preflight-file`、`--approve`、`--execute-live`、`--allow-folder-delete`、`--allow-policy-reset`、`--org-id`、`--output`、`--applied-by`、`--applied-at`、`--approval-reason`、`--apply-note`。

範例：

```bash
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --basic-user admin --basic-password admin
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --allow-folder-delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"
```

相關指令：`change review`、`change preflight`、`change bundle-preflight`。

## `audit`

用途：比對受管 Grafana 資源的 checksum lock 與目前 live state，進行稽核。

適用時機：當你需要做 drift 檢查，或想刷新 lock snapshot 時。

主要旗標：`--managed-file`、`--lock-file`、`--live-file`、`--fetch-live`、`--org-id`、`--page-size`、`--write-lock`、`--fail-on-drift`、`--interactive`、`--output`。

範例：

```bash
grafana-util change audit --managed-file ./desired.json --live-file ./live.json --write-lock ./sync-lock.json
grafana-util change audit --lock-file ./sync-lock.json --fetch-live --profile prod --output json
grafana-util change audit --lock-file ./sync-lock.json --fetch-live --url http://localhost:3000 --basic-user admin --basic-password admin --fail-on-drift --output json
grafana-util change audit --lock-file ./sync-lock.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --fail-on-drift --output json
```

相關指令：`change preflight`、`change plan`、`status live`。

## `preflight`

用途：根據 desired resources 與可選的 availability 提示，建立分階段的同步 preflight 文件。

適用時機：當你需要在規劃或套用前先做結構性門檻檢查時。

主要旗標：`--desired-file`、`--availability-file`、`--fetch-live`、`--org-id`、`--output`。

範例：

```bash
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
grafana-util change preflight --desired-file ./desired.json --fetch-live --profile prod --output json
grafana-util change preflight --desired-file ./desired.json --fetch-live --url http://localhost:3000 --basic-user admin --basic-password admin --output json
grafana-util change preflight --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```

相關指令：`change summary`、`change plan`、`status staged`。

## `assess-alerts`

用途：評估 alert 同步規格的 candidate、plan-only 與 blocked 狀態。

適用時機：當你想在 bundling 或 apply 前，先看 alert 資源會如何分類時。

主要旗標：`--alerts-file`、`--output`。

範例：

```bash
grafana-util change assess-alerts --alerts-file ./alerts.json
grafana-util change assess-alerts --alerts-file ./alerts.json --output json
```

相關指令：`change bundle`、`change bundle-preflight`、`overview`。

## `bundle`

用途：將匯出的 dashboards、alerting 資源、datasource inventory 與 metadata 打包成單一的本機 source bundle。

適用時機：當你想要一個統一的 bundle artifact，供後續同步、審核或 preflight 使用時。

主要旗標：`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--alert-export-dir`、`--datasource-export-file`、`--datasource-provisioning-file`、`--metadata-file`、`--output-file`、`--output`。

範例：

```bash
grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
grafana-util change bundle --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
```

相關指令：`change bundle-preflight`、`change promotion-preflight`、`snapshot export`。

## `bundle-preflight`

用途：根據 source bundle 與 target inventory 建立分階段的 bundle-level sync preflight 文件。

適用時機：當你需要在 apply 前比較 source bundle 與 target inventory 時。

主要旗標：`--source-bundle`、`--target-inventory`、`--availability-file`、`--fetch-live`、`--org-id`、`--output`。

範例：

```bash
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output json
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --output json
```

相關指令：`change bundle`、`change promotion-preflight`、`status staged`。

## `promotion-preflight`

用途：根據 source bundle 與 target inventory 建立分階段的 promotion review handoff。

適用時機：當你準備進行 promotion review，並且需要明確的 mapping 與 availability 視圖時。

主要旗標：`--source-bundle`、`--target-inventory`、`--mapping-file`、`--availability-file`、`--fetch-live`、`--org-id`、`--output`。

範例：

```bash
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --output json
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --availability-file ./availability.json --output json
```

相關指令：`change bundle-preflight`、`change apply`、`status live`。
