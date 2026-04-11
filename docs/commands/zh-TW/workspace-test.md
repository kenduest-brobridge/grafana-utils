# `grafana-util workspace test`

## 用途

檢查本機 workspace 在結構上是否安全可以繼續。

## 適用時機

- 當你需要在 preview 或 apply 前先做 readiness gate 時。
- 當你在 CI 裡只需要快速判斷 staged inputs 是否可接受時。

## 主要旗標

- `--availability-file`
- `--target-inventory`、`--mapping-file`
- `--fetch-live`
- `--output-format`

## 範例

```bash
grafana-util workspace test ./grafana-oac-repo --fetch-live --output-format json
```

## 相關指令

- [workspace](./workspace.md)
- [workspace scan](./workspace-scan.md)
- [workspace preview](./workspace-preview.md)
- [status staged](./status.md#staged)
