# `grafana-util alert list-contact-points`

## 目的

列出目前 Grafana 線上的 alert 聯絡點。

## 使用時機

- 檢視 Grafana 內已設定的通知端點。
- 在文字、表格、CSV、JSON 與 YAML 之間切換輸出格式。

## 主要旗標

- `--org-id` 會列出某個 Grafana org ID 的聯絡點。
- `--all-orgs` 會彙整所有可見 org 的清單。
- `--output-format` 控制輸出格式，可選 `text`、`table`、`csv`、`json` 與 `yaml`。
- `--no-header` 省略表頭列。

## 說明

- 可重複執行的單一 org 清單查詢優先用 `--profile`。
- `--all-orgs` 最好搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 權限可能只看到部分資料。

## 採用前後對照

- 之前：得進 Grafana UI 一個個找通知端點，還要自己猜 scope。
- 之後：一次列出聯絡點清單，就能拿去比對、審閱或交給 CI。

## 成功判準

- 你預期的聯絡點會出現在輸出裡。
- 查詢範圍和你指定的 org / profile 一致。
- 輸出格式可以直接交給人看或給腳本處理。

## 失敗時先檢查

- 如果清單看起來不完整，先確認 token 權限是不是只看得到部分 org。
- `--all-orgs` 少資料時，改用管理員支援的 profile 或 Basic auth。
- 先確認 org / profile，再把空清單當成真的沒有資料。

## 範例

```bash
# 用途：列出目前 Grafana 線上的 alert 聯絡點。
grafana-util alert list-contact-points --profile prod --output-format table
```

```bash
# 用途：列出目前 Grafana 線上的 alert 聯絡點。
grafana-util alert list-contact-points --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

```bash
# 用途：列出目前 Grafana 線上的 alert 聯絡點。
grafana-util alert list-contact-points --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

## 相關命令

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-templates](./alert-list-templates.md)
