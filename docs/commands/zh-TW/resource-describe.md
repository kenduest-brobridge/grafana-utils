# resource describe

## 用途
說明目前支援的 live Grafana resource kinds 與 selector 格式。

## 何時使用
當你想先看清楚通用唯讀資源查詢介面的形狀，再決定要用 `resource list` 還是 `resource get` 時，使用這個指令。

## 重點旗標
- 位置參數 `KIND`（可省略）：限制輸出為某一種支援的 resource kind，例如 `dashboards`、`folders`、`datasources`、`alert-rules` 或 `orgs`
- `--output-format`：選擇 `text`、`table`、`json` 或 `yaml`

## 補充說明
- 這個指令只做說明，不會從 Grafana 動態探測 schema。
- 當你想知道 selector 格式、list endpoint 或 get endpoint 時，直接用這個指令。

## 範例
```bash
# 用途：說明所有支援的 live resource kinds。
grafana-util resource describe
```

```bash
# 用途：以 JSON 說明單一 resource kind。
grafana-util resource describe dashboards --output-format json
```

```bash
# 用途：以表格說明單一 resource kind。
grafana-util resource describe orgs --output-format table
```

## 相關指令
- [resource](./resource.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
