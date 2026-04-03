# `grafana-util status`

## Root

用途：渲染共用的全專案 staged 或 live status。

適用時機：當你需要 exported artifacts 或 live Grafana state 的最終門檻視圖時。

主要旗標：root 指令本身只是命名空間；staged 與 live 輸入都在子指令上。常見旗標包含 `--output` 和共用的 live 連線/驗證選項。

範例：

```bash
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output json
grafana-util status live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output yaml
```

相關指令：`grafana-util overview`、`grafana-util change preflight`、`grafana-util change apply`。

## `staged`

用途：根據已分階段的 artifact 渲染專案狀態。

適用時機：當你需要在 apply 前，對匯出檔案做機器可讀的 readiness gate 時。

主要旗標：`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--datasource-export-dir`、`--datasource-provisioning-file`、`--access-user-export-dir`、`--access-team-export-dir`、`--access-org-export-dir`、`--access-service-account-export-dir`、`--desired-file`、`--source-bundle`、`--target-inventory`、`--alert-export-dir`、`--availability-file`、`--mapping-file`、`--output`。

範例：

```bash
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output table
grafana-util status staged --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts --output interactive
```

相關指令：`grafana-util overview`、`grafana-util change summary`、`grafana-util change preflight`。

## `live`

用途：根據 live Grafana 的讀取面來渲染專案狀態。

適用時機：當你需要目前的 Grafana 狀態，並可選擇搭配 staged context 檔案時。

主要旗標：`--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`、`--prompt-password`、`--prompt-token`、`--timeout`、`--verify-ssl`、`--insecure`、`--ca-cert`、`--all-orgs`、`--org-id`、`--sync-summary-file`、`--bundle-preflight-file`、`--promotion-summary-file`、`--mapping-file`、`--availability-file`、`--output`。

說明：
- 一般 live status 檢查優先用 `--profile`。
- `--all-orgs` 最穩妥的是搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 權限可能看不到其他 org。

範例：

```bash
grafana-util status live --profile prod --output yaml
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output interactive
grafana-util status live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```

相關指令：`grafana-util overview live`、`grafana-util change apply`、`grafana-util profile show`。
