# `grafana-util status`

## Root

用途：輸出整個專案的 staged 或 live 狀態摘要。

適用時機：當你需要看 exported artifacts 或 live Grafana state 的最後檢查結果時。

說明：如果你要的是最後的 readiness 或健康度讀取，而不是逐條研究命令細節，先看這一頁最合適。`status` 指令群組就是維運與 CI 常拿來回答「目前 staged bundle 能不能往下走」或「現在 live Grafana 狀態如何」的 gate 視圖。

主要旗標：root 指令本身只是指令群組；staged 與 live 輸入都在子指令上。常見旗標包含 `--output-format` 和共用的 live 連線 / 驗證選項。

範例：

```bash
# 用途：輸出 staged 狀態，來源是 dashboard 與 desired 產物。
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format json
```

```bash
# 用途：用可重複使用的 profile 輸出 live 狀態。
grafana-util status live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format yaml
```

相關指令：`grafana-util overview`、`grafana-util change preflight`、`grafana-util change apply`。

## `staged`

用途：根據已準備好的 artifact 輸出專案狀態。

適用時機：當你需要在 apply 前，先用結構化輸出做 readiness gate 時。

主要旗標：`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--datasource-export-dir`、`--datasource-provisioning-file`、`--access-user-export-dir`、`--access-team-export-dir`、`--access-org-export-dir`、`--access-service-account-export-dir`、`--desired-file`、`--source-bundle`、`--target-inventory`、`--alert-export-dir`、`--availability-file`、`--mapping-file`、`--output-format`。

範例：

```bash
# 用途：staged。
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format table
```

```bash
# 用途：staged。
grafana-util status staged --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts --output-format interactive
```

相關指令：`grafana-util overview`、`grafana-util change summary`、`grafana-util change preflight`。

## `live`

用途：根據 live Grafana 的讀取結果輸出專案狀態。

適用時機：當你需要目前的 Grafana 狀態，並可選擇搭配 staged context 檔案時。

主要旗標：`--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`、`--prompt-password`、`--prompt-token`、`--timeout`、`--verify-ssl`、`--insecure`、`--ca-cert`、`--all-orgs`、`--org-id`、`--sync-summary-file`、`--bundle-preflight-file`、`--promotion-summary-file`、`--mapping-file`、`--availability-file`、`--output-format`。

說明：
- 一般 live status 檢查優先用 `--profile`。
- `--all-orgs` 最穩妥的是搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 權限可能看不到其他 org。

範例：

```bash
# 用途：live。
grafana-util status live --profile prod --output-format yaml
```

```bash
# 用途：live。
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output-format interactive
```

```bash
# 用途：live。
grafana-util status live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

相關指令：`grafana-util overview live`、`grafana-util change apply`、`grafana-util profile show`。
