# 兼容頁：`grafana-util status`

## Root

用途：舊 `status` root 的相容參考頁。

適用時機：當你在對照舊文件或舊腳本，想對應到目前的 `observe` surface 時。

說明：目前公開的 staged/live status surface 已移到 `grafana-util observe`。請改用 `observe staged`、`observe live`、`observe overview` 或 `observe snapshot`，不要再把 top-level `status` 當成主入口。

主要旗標：canonical root 是 `observe`；staged 與 live 輸入都在子指令上。常見旗標包含 `--output-format` 和共用的 live 連線 / 驗證選項。

範例：

```bash
# 用途：輸出 staged 狀態，來源是 dashboard 與 desired 產物。
grafana-util observe staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format json
```

```bash
# 用途：用可重複使用的 profile 輸出 live 狀態。
grafana-util observe live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format yaml
```

相關指令：`grafana-util observe overview`、`grafana-util change check`、`grafana-util change apply`。

Schema guide：
- `grafana-util observe --help-schema`
- `grafana-util observe staged --help-schema`
- `grafana-util observe live --help-schema`

## `staged`

用途：根據已準備好的 artifact 輸出專案狀態。

適用時機：當你需要在 apply 前，先用結構化輸出做 readiness gate 時。

主要旗標：`--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--datasource-export-dir`、`--datasource-provisioning-file`、`--access-user-export-dir`、`--access-team-export-dir`、`--access-org-export-dir`、`--access-service-account-export-dir`、`--desired-file`、`--source-bundle`、`--target-inventory`、`--alert-export-dir`、`--availability-file`、`--mapping-file`、`--output-format`。

範例：

```bash
# 用途：staged。
grafana-util observe staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format table
```

```bash
# 用途：staged。
grafana-util observe staged --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts --output-format interactive
```

相關指令：`grafana-util observe overview`、`grafana-util change inspect`、`grafana-util change check`。

Machine-readable contract：`grafana-util-project-status`

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
grafana-util observe live --profile prod --output-format yaml
```

```bash
# 用途：live。
grafana-util observe live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output-format interactive
```

```bash
# 用途：live。
grafana-util observe live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

相關指令：`grafana-util observe overview`、`grafana-util change apply`、`grafana-util config profile show`。

Machine-readable contract：`grafana-util-project-status`
