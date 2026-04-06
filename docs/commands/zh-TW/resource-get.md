# resource get

## 用途
透過 selector 取得單一目前支援的 live Grafana 資源。

## 何時使用
當你需要某一筆支援資源的 live payload，且想用比完整 export 或 inspect workflow 更簡單的逃生入口時，使用這個指令。

## 重點旗標
- 位置參數 `SELECTOR`：必填 `<kind>/<identity>`，例如 `dashboards/cpu-main` 或 `datasources/prom-main`
- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`：live Grafana 連線設定
- `--output-format`：選擇 `text`、`table`、`json` 或 `yaml`

## 補充說明
- selector 的 kind 目前必須是 `dashboards`、`folders`、`datasources`、`alert-rules`、`orgs` 之一。
- `text` 和 `table` 只會摘要顯示主要欄位；需要完整 payload 時請用 `json` 或 `yaml`。

## 範例
```bash
# 用途：依 UID 取得單一 live dashboard。
grafana-util resource get dashboards/cpu-main --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# 用途：以 YAML 取得單一 datasource payload。
grafana-util resource get datasources/prom-main --profile prod --output-format yaml
```

```bash
# 用途：依數字 ID 取得單一 org payload。
grafana-util resource get orgs/1 --profile prod --output-format json
```

## 相關指令
- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
