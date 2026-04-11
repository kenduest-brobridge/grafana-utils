# `grafana-util workspace scan`

## 用途

找出本機 workspace 或 staged package 裡有哪些內容。

## 適用時機

- 當你想先知道 package 裡有什麼內容時，先從這裡開始。
- 在 `workspace test` 或 `workspace preview` 前，用它先確認規模。

## 主要旗標

- 
- `--desired-file`
- `--dashboard-export-dir`、`--dashboard-provisioning-dir`
- `--alert-export-dir`、`--datasource-provisioning-file`
- `workspace package` 的輸出檔
- `--output-format`
- `--output-file`、`--also-stdout`

## 範例

```bash
grafana-util workspace scan ./grafana-oac-repo
```

## 相關指令

- [workspace](./workspace.md)
- [workspace test](./workspace-test.md)
- [workspace preview](./workspace-preview.md)
- [status](./status.md)
