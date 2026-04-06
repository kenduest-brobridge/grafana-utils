# resource list

## 用途
列出某一種目前支援的 live Grafana resource kind。

## 何時使用
當你需要某一種支援資源的唯讀 live inventory，但還不需要進入更高階的 domain workflow 時，使用這個指令。

## 重點旗標
- 位置參數 `KIND`：例如 `dashboards`、`folders`、`datasources`、`alert-rules`、`orgs`
- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`：live Grafana 連線設定
- `--output-format`：選擇 `text`、`table`、`json` 或 `yaml`

## 範例
```bash
# 用途：從本機 Grafana 以表格列出 dashboards。
grafana-util resource list dashboards --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# 用途：以 YAML 列出 folders。
grafana-util resource list folders --profile prod --output-format yaml
```

```bash
# 用途：以 JSON 列出 alert rules。
grafana-util resource list alert-rules --profile prod --output-format json
```

## 相關指令
- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource get](./resource-get.md)
