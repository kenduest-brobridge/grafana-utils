# advanced

## 用途
`grafana-util advanced` 是給 expert/operator 用的 domain-specific 進階 namespace。

## 何時使用
當你已經知道自己要進哪個 subsystem，例如 dashboard import、alert authoring、datasource diff、access administration，就進這裡。

## 說明
`advanced` 保留了 `grafana-util` 原本完整的 domain 深度，但不再讓新手在第一眼就看到所有 lane。它是之後建議的 canonical expert 入口；舊的 top-level domain roots 仍然保留作為相容路徑。

## 子命令

### Dashboard 工作流
- `advanced dashboard live ...`：瀏覽 live dashboards、history、fetch。
- `advanced dashboard draft ...`：review、patch、serve、publish 本地 draft。
- `advanced dashboard sync ...`：export、import、diff、轉換 migration artifacts。
- `advanced dashboard analyze ...`：summary、topology、impact、governance。
- `advanced dashboard capture ...`：瀏覽器渲染截圖與 PDF。

### Alert 工作流
- `advanced alert live ...`：查看 live alert inventory，或刪除一個 live resource。
- `advanced alert migrate ...`：export、import、diff alert artifacts。
- `advanced alert author ...`：初始化並編寫 desired-state alert 資源。
- `advanced alert scaffold ...`：直接建立低階 alert 檔案骨架。
- `advanced alert change ...`：plan 與 apply staged alert changes。

### Datasource 與 access 工作流
- `advanced datasource ...`：list、browse、export、import、diff datasources。
- `advanced access ...`：user、team、org、service-account 管理。

## 範例
### Dashboard 匯入
```bash
grafana-util advanced dashboard sync import --input-dir ./dashboards/raw --dry-run --table
```

### Alert 路由預覽
```bash
grafana-util advanced alert author route preview --desired-dir ./alerts/desired --label team=sre --severity critical
```

### Datasource diff
```bash
grafana-util advanced datasource diff --diff-dir ./datasources --input-format inventory
```

### Access diff
```bash
grafana-util advanced access user diff --diff-dir ./access-users --scope global
```

## 相關指令

- [export](./export.md)
- [change](./change.md)
- [dashboard](./dashboard.md)
- [alert](./alert.md)
- [datasource](./datasource.md)
- [access](./access.md)
