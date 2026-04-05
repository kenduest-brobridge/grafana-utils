# datasource add

## 用途
透過 Grafana API 建立一個線上 Grafana datasource。

## 何時使用
當您想直接建立新的 datasource，或在套用前先 dry-run 建立步驟時，使用這個指令。

## 重點旗標
- `--uid`：穩定的 datasource 識別碼。
- `--name`：datasource 名稱。
- `--type`：Grafana datasource plugin type id。
- `--datasource-url`：datasource 目標網址。
- `--access`：proxy 或 direct 存取模式。
- `--default`：標記為預設 datasource。
- `--preset-profile` 與 `--apply-supported-defaults`：產生支援的預設值。
- `--json-data`、`--secure-json-data`、`--secure-json-data-placeholders`、`--secret-values`：設定自訂欄位與秘密值。
- `--dry-run`、`--table`、`--json`、`--output-format`、`--no-header`：預覽輸出控制。

## 範例
```bash
# 用途：透過 Grafana API 建立一個線上 Grafana datasource。
grafana-util datasource add --profile prod --name tempo-main --type tempo --datasource-url http://tempo:3200 --preset-profile full --dry-run --json
```

```bash
# 用途：透過 Grafana API 建立一個線上 Grafana datasource。
grafana-util datasource add --url http://localhost:3000 --basic-user admin --basic-password admin --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --table
```

```bash
# 用途：透過 Grafana API 建立一個線上 Grafana datasource。
grafana-util datasource add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name tempo-main --type tempo --datasource-url http://tempo:3200 --preset-profile full --dry-run --json
```

## 採用前後對照

- **採用前**：datasource 建立常常散在 UI 點選或手寫 API payload。
- **採用後**：一個指令就能產生建立 payload、先預覽，再看清楚即將送出的欄位。

## 成功判準

- 在實際建立前，type id 與 datasource URL 都是明確的
- `--dry-run` 顯示的 payload 形狀符合預期
- secret placeholder 與預設值的補齊方式每次都一致

## 失敗時先檢查

- 如果建立預覽看起來不對，先確認 type id 與 datasource URL，再決定要不要送出
- 如果某個 secret 欄位沒有解開，先確認 `--secret-values` 或所選 preset profile 是否完整
- 如果實際建立失敗，先把預覽 payload 和這個 plugin type 的資料規則對照一次

## 相關指令
- [datasource types](./datasource-types.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
