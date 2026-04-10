# export

## 用途
`grafana-util export` 是給常見備份與本地 inventory 擷取工作的 task-first 入口。

## 何時使用
當你的工作就是單純匯出或備份，而不想一開始就進入 domain-heavy 的進階樹時，請先用這個 namespace。

## 說明
`export` 故意保持狹窄。它只是包住既有的 domain export 流程，不改變底層行為，讓第一次使用的人不用先理解整棵 `dashboard`、`alert`、`datasource`、`access` 命令樹。

## 子命令

### 備份與 artifact 擷取
- `export dashboard`：匯出 dashboards 的 raw、prompt、provisioning 三條 lane。
- `export alert`：把 alert resources 匯出成在地 artifact tree。
- `export datasource`：匯出 datasource inventory 供 review 或 restore。

### Access inventory 擷取
- `export access user`：匯出 Grafana users。
- `export access org`：匯出 Grafana org inventory。
- `export access team`：匯出 Grafana teams。
- `export access service-account`：匯出 service accounts。

## 範例
### Dashboard 備份
```bash
grafana-util export dashboard --output-dir ./dashboards --overwrite
```

### Alert 備份
```bash
grafana-util export alert --output-dir ./alerts --overwrite
```

### Datasource inventory
```bash
grafana-util export datasource --output-dir ./datasources
```

### Access inventory
```bash
grafana-util export access service-account --output-dir ./access-service-accounts
```

## 相關指令

- [advanced](./advanced.md)
- [dashboard export](./dashboard-export.md)
- [alert export](./alert-export.md)
- [datasource export](./datasource-export.md)
- [access](./access.md)
