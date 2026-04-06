# resource kinds

## 用途
列出 `resource` 命名空間目前支援的 live resource kinds。

## 何時使用
當你要先確認通用資源查詢介面是否已經支援你想查的 live Grafana resource kind 時，使用這個指令。如果你想看 selector 格式或 endpoint 形狀，請改用 `resource describe`。

## 重點旗標
- `--output-format`：選擇 `text`、`table`、`json` 或 `yaml`。

## 範例
```bash
# 用途：用表格列出支援的 resource kinds。
grafana-util resource kinds
```

```bash
# 用途：以 JSON 輸出相同的支援清單。
grafana-util resource kinds --output-format json
```

## 相關指令
- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
