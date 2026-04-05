# datasource diff

## 用途
比較本地 bundle 中的 datasource inventory 與線上 Grafana，並輸出給操作人員看的 diff 報告。

## 何時使用
當您想在匯入前先取得簡潔的線上與本地差異報告時，使用這個指令。

## 重點旗標
- `--diff-dir`：要比對的本地 datasource bundle。
- `--input-format`：選擇 `inventory` 或 `provisioning`。

## 範例
```bash
# 用途：比較本地 bundle 中的 datasource inventory 與線上 Grafana，並輸出給操作人員看的 diff 報告。
grafana-util datasource diff --profile prod --diff-dir ./datasources --input-format inventory
```

```bash
# 用途：比較本地 bundle 中的 datasource inventory 與線上 Grafana，並輸出給操作人員看的 diff 報告。
grafana-util datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources/provisioning --input-format provisioning
```

```bash
# 用途：比較本地 bundle 中的 datasource inventory 與線上 Grafana，並輸出給操作人員看的 diff 報告。
grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources --input-format inventory
```

## 採用前後對照

- **採用前**：常常要手動對照本地與線上的 datasource JSON，才能看出 drift 在哪裡。
- **採用後**：一個 diff 指令就能在匯入前看出 bundle 與 Grafana 的差異。

## 成功判準

- 在匯入前就能清楚說明這次變更的內容
- inventory 與 provisioning 兩種輸入都能產出可讀的摘要
- 輸出能直接看出是 bundle 變了，還是 live 端變了

## 失敗時先檢查

- 如果 diff 意外是空的，先確認 bundle 路徑與 `--input-format`
- 如果 live 端看起來不對，先確認目標 Grafana 與 org 範圍，再相信報告
- 如果 diff 很吵，先確認你比對的是預期中的 inventory bundle，而不是舊 provisioning 樹

## 相關指令
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource inspect-export](./datasource-inspect-export.md)
