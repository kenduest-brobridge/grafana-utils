# 共用 Diff JSON Contract

當你要解析或自動化處理 `dashboard diff`、`alert diff`、`datasource diff` 的 JSON 輸出時，請看這一頁。

## Contract 形狀

三個 diff 指令在 JSON 模式下都會輸出相同的 top-level envelope：

- `kind`
- `schemaVersion`
- `toolVersion`
- `summary`
- `rows`

建議依照這個順序處理：

1. 先確認 `kind`
2. 再確認 `schemaVersion`
3. 最後才讀 `rows`

## 版本規則

這組共用 contract 的 `schemaVersion` 是 family-wide major version。

- 目前版本是 `1`。
- 只有在會影響所有 diff consumer 的 breaking workspace 時才提升 `schemaVersion`，例如移除或重新命名 top-level key、變更必要欄位、或改變既有欄位的語意。
- 如果只是新增向下相容欄位，`schemaVersion` 維持不變，但仍要同步更新文件與 contract tests。
- `dashboard diff`、`alert diff`、`datasource diff` 的 shared envelope 有變更時，三者要一起更新。

CLI schema 快速查詢：

- `grafana-util dashboard diff --help-schema`
- `grafana-util alert diff --help-schema`
- `grafana-util datasource diff --help-schema`

## Summary 欄位

共用 summary 會包含這些計數器：

- `checked`
- `same`
- `different`
- `missingRemote`
- `extraRemote`
- `ambiguous`

這些欄位在 dashboard、alert、datasource diff 之間保持一致。

## 各指令的 Row 欄位

### `dashboard diff`

Dashboard diff 的 row 會保留檔案路徑與簡短 diff 預覽：

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `changedFields`
- `diffText`
- `contextLines`

### `alert diff`

Alert diff 的 row 保持簡潔，聚焦在資源層級的審查：

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `changedFields`

`alert diff` 仍接受 `--json` 當相容旗標，但 canonical 寫法是 `--output-format json`。

### `datasource diff`

Datasource diff 的 row 會額外提供欄位層級的變更內容：

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `changedFields`
- `changes[]`

每個 `changes[]` item 會記錄：

- `field`
- `before`
- `after`

### `dashboard history diff`

Dashboard history diff 仍然沿用相同的 shared envelope，但會加入來源標籤，方便你比對 live history、單一 export 成品，或不同日期的兩個 export roots：

- `domain`
- `resourceKind`
- `identity`
- `status`
- `path`
- `baseSource`
- `newSource`
- `baseVersion`
- `newVersion`
- `changedFields`
- `diffText`
- `contextLines`

## 實務讀法

如果要給 CI 或腳本使用，建議照這個順序：

1. 先驗 `kind`
2. 再驗 `schemaVersion`
3. 用 `summary` 做 gate
4. 最後再看 `rows` 做審查

## 相關頁面

- [dashboard diff 指令](../../commands/zh-TW/dashboard-diff.md)
- [alert diff 指令](../../commands/zh-TW/alert-diff.md)
- [datasource diff 指令](../../commands/zh-TW/datasource-diff.md)
- [dashboard history 指令](../../commands/zh-TW/dashboard-history.md)
