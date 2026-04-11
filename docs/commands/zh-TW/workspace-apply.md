# `grafana-util workspace apply`

## 用途

把已審核的 preview 轉成 staged 或 live apply 結果。

## 適用時機

- 當 preview 與 review 都已完成時。
- 只有真的準備好動 Grafana 時，才加 `--execute-live`。

## 主要旗標

- `--preview-file`
- `--plan-file`
- `--approve`
- `--execute-live`
- `--approval-reason`、`--apply-note`
- `--output-format`

## 範例

```bash
grafana-util workspace apply --preview-file ./workspace-preview.json --approve --execute-live --profile prod
```

## 相關指令

- [workspace](./workspace.md)
- [workspace preview](./workspace-preview.md)
- [workspace ci](./workspace.md#ci)

