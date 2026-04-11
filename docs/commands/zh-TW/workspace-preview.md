# `grafana-util workspace preview`

## 用途

顯示目前 workspace 輸入會造成哪些變動。

## 適用時機

- 當你想拿到可審查的 preview artifact 時。
- 這是 task-first 路徑裡對應常見 `plan` 步驟的入口。

## 主要旗標

- 
- `--desired-file`
- `workspace package` 的輸出檔、`--target-inventory`、`--mapping-file`、`--availability-file`
- `--live-file`
- `--fetch-live`
- `--allow-prune`
- `--trace-id`
- `--output-format`、`--output-file`

## 範例

```bash
grafana-util workspace preview ./grafana-oac-repo --fetch-live --profile prod
```

## 相關指令

- [workspace](./workspace.md)
- [workspace test](./workspace-test.md)
- [workspace apply](./workspace-apply.md)
