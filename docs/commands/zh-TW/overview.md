# 兼容頁：`grafana-util overview`

## Root

用途：舊 top-level `overview` root 的相容參考頁。

適用時機：當你在對照舊文件或舊腳本，想對應到目前的 `observe` surface 時。

說明：目前公開的 overview surface 已移到 `grafana-util observe`。請改用 `observe overview` 或 `observe live`，不要再把 top-level `overview` 當成主入口。

主要旗標：分階段輸入，例如 `--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--datasource-export-dir`、`--datasource-provisioning-file`、`--access-user-export-dir`、`--access-team-export-dir`、`--access-org-export-dir`、`--access-service-account-export-dir`、`--desired-file`、`--source-bundle`、`--target-inventory`、`--alert-export-dir`、`--availability-file`、`--mapping-file` 和 `--output-format`。

範例：

```bash
# 用途：彙總 staged 的 dashboard、alert 與 access 產物。
grafana-util observe overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output-format table
```

```bash
# 用途：在 promotion 前先檢視 sync bundle 的輸入。
grafana-util observe overview --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --mapping-file ./mapping.json --output-format text
```

相關指令：`grafana-util observe staged`、`grafana-util change inspect`、`grafana-util snapshot review`。

## `live`

用途：透過共用的 observe live 流程，輸出 live overview。

適用時機：當你需要與 `observe live` 相同的 live readout，但想從 overview 這個指令群組來操作時。

主要旗標：共用 observe live 流程的 live 連線與驗證旗標，以及 `--sync-summary-file`、`--bundle-preflight-file`、`--promotion-summary-file`、`--mapping-file`、`--availability-file` 和 `--output-format`。

說明：
- 可重複執行的 live overview 工作優先用 `--profile`。
- 想拿到較廣 org 可見度時，直接 Basic auth 會更穩定。
- Token 驗證適合權限邊界明確的讀取流程，但最後可見結果仍受 token 權限範圍限制。

範例：

```bash
# 用途：live。
grafana-util observe overview --profile prod --output-format yaml
```

```bash
# 用途：live。
grafana-util observe overview --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

```bash
# 用途：live。
grafana-util observe overview --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

相關指令：`grafana-util observe live`、`grafana-util change apply`、`grafana-util config profile show`。
