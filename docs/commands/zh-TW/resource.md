# resource

## 用途
透過通用、唯讀的查詢介面讀取少數幾種 live Grafana 資源。

## 何時使用
當你需要先做唯讀的 live lookup，但 `dashboard`、`alert`、`datasource`、`access`、`workspace` 還沒有對應的高階工作流時，可以先用這個命名空間。

## 說明
這個命名空間刻意比主要維運工作流更通用、更窄。它的目的不是取代既有的高階操作，而是提供一條唯讀資源查詢路徑，讓你能先檢查少數支援的 live Grafana resource kinds。

把它當成通用查詢工具，不是日常 mutation 的主入口。

## 使用方式
- 先用 `resource describe` 看各 kind 的 selector 格式與 endpoint 形狀。
- 先用 `resource kinds` 看目前支援哪些 live resource kinds。
- 需要盤點某一類資源時，用 `resource list <kind>`。
- 需要抓某一筆完整 live payload 時，用 `resource get <kind>/<identity>`。

## 目前支援的 kinds
- `dashboards`
- `folders`
- `datasources`
- `alert-rules`
- `orgs`

## 輸出
- `kinds` 支援 `text`、`table`、`json`、`yaml`
- `list` 支援 `text`、`table`、`json`、`yaml`
- `get` 支援 `text`、`table`、`json`、`yaml`

## 範例
```bash
# 用途：說明目前支援的 live resource kinds 與 selector 格式。
grafana-util resource describe
```

```bash
# 用途：顯示目前支援的 resource kinds。
grafana-util resource kinds
```

```bash
# 用途：從本機 Grafana 列出 live dashboards。
grafana-util resource list dashboards --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# 用途：以 YAML 取得單一 datasource payload。
grafana-util resource get datasources/prom-main --profile prod --output-format yaml
```

## 相關指令
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
- [dashboard](./dashboard.md)
