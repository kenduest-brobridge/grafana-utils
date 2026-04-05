# `grafana-util alert list-rules`

## 目的

列出目前 Grafana 線上的 alert 規則。

## 使用時機

- 檢視單一 org 或所有可見 org 的規則清單。
- 以文字、表格、CSV、JSON 或 YAML 格式輸出。

## 主要旗標

- `--org-id` 會列出某個 Grafana org ID 的規則。
- `--all-orgs` 會彙整所有可見 org 的清單。
- `--output-format` 控制輸出格式，可選 `text`、`table`、`csv`、`json` 與 `yaml`。
- `--no-header` 省略表頭列。

## 說明

- 可重複執行的單一 org 清單查詢優先用 `--profile`。
- `--all-orgs` 最好搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 權限可能只看到部分資料。

## 採用前後對照

- 之前：要進 Grafana UI 才能確認 alert 規則有哪些、分布在哪些 org。
- 之後：一次列出規則清單，方便比對、審閱或交給 CI。

## 成功判準

- 你預期的 alert 規則會出現在輸出裡。
- 查詢範圍和你指定的 org / profile 一致。
- 輸出格式可以直接拿去人工檢視或自動化處理。

## 失敗時先檢查

- 如果清單看起來太少，先確認 token 權限是不是只看得到部分 org。
- `--all-orgs` 少資料時，改用管理員支援的 profile 或 Basic auth。
- 先確認 org / profile，再把空清單當成真的沒有規則。

## 範例

```bash
# 用途：列出目前 Grafana 線上的 alert 規則。
grafana-util alert list-rules --profile prod --output-format table
```

```bash
# 用途：列出目前 Grafana 線上的 alert 規則。
grafana-util alert list-rules --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

```bash
# 用途：列出目前 Grafana 線上的 alert 規則。
grafana-util alert list-rules --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

## 相關命令

- [alert](./alert.md)
- [alert list-contact-points](./alert-list-contact-points.md)
- [alert list-mute-timings](./alert-list-mute-timings.md)
