# datasource list

## 用途
列出來自線上 Grafana 或本地匯出 bundle 的 datasource inventory。

## 何時使用
當您需要一份非互動式的 datasource 清單，不論是目前 org、指定 org、所有可見 org，還是本地磁碟上的匯出 bundle，都可以使用這個指令。

## 重點旗標
- `--input-dir`：讀取本地 datasource 匯出 bundle 或 provisioning tree。
- `--input-format`：當本地路徑可能同時對應多種來源形狀時，選擇 `inventory` 或 `provisioning`。
- `--org-id`：列出指定的 Grafana org。
- `--all-orgs`：彙整所有可見 org 的 datasource inventory。需要 Basic auth。
- `--output-format`、`--text`、`--table`、`--csv`、`--json`、`--yaml`：輸出模式控制。
- `--output-columns`：選擇顯示欄位。
- `--no-header`：隱藏表格標頭。

## 範例
```bash
# 用途：列出線上 Grafana datasource inventory。
grafana-util datasource list --profile prod --output-format text
```

```bash
# 用途：列出線上 Grafana datasource inventory。
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

```bash
# 用途：列出線上 Grafana datasource inventory。
grafana-util datasource list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
```

```bash
# 用途：列出來自本地匯出 bundle 的 datasource inventory。
grafana-util datasource list --input-dir ./datasources --json
```

## 採用前後對照

- **採用前**：想看 datasource inventory 時，常常要在 Grafana UI、export bundle 或零散 API 呼叫之間來回切換。
- **採用後**：一個 inventory 指令就能產生可審查的清單，輸出可用 text、table、CSV、JSON 或 YAML，也可來自線上 Grafana 或本地匯出 bundle。

## 成功判準

- 您可以把指令指到想看的 org，或本地 bundle，拿到預期中的 inventory
- table 與 CSV 輸出能直接交給腳本或放進 PR 審查
- 只有在真的要跨 org 檢視時，才使用 `--all-orgs`

## 失敗時先檢查

- 如果 inventory 是空的，先確認 org 範圍與驗證資訊是否真的看得到目標 org
- 如果 `--all-orgs` 失敗，先改用 Basic auth，並檢查 token 是否只看得到單一 org
- 如果本地 bundle 讀取失敗，先確認 `--input-dir` 與 `--input-format`
- 如果欄位看起來不對，先確認輸出格式與指定欄位是否一致

## 相關指令
- [datasource browse](./datasource-browse.md)
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
