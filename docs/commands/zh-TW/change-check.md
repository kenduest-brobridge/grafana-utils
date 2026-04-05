# `grafana-util change check`

## 用途

檢查 staged change package 是否在結構上適合繼續往下走。

## 適用時機

- 在 `change inspect` 之後，先做一次 readiness gate，再決定是否 preview。
- 在 CI 中，如果你只需要快速判斷 staged inputs 是否可接受，也適合先跑這一步。
- 如果你想留在 task-first `change` lane，而不是切去 `status staged`，就用這個。

## 採用前後對照

- **採用前**：你知道這包東西存在，但不知道它在結構上是否足夠一致、足夠安全。
- **採用後**：你會拿到一份明確區分 blockers 與 warnings 的 readiness 結果，可以很早就停下流程。

## 主要旗標

- `--workspace`：從常見 repo-local inputs 自動發現 staged package。
- `--availability-file`：把 staged availability hints 併進來。
- `--target-inventory`、`--mapping-file`：在 bundle / promotion 場景下加入更多檢查。
- `--fetch-live`：把 live target 的檢查也併進結果。
- `--output-format`：輸出成 `text` 或 `json`。

## 範例

```bash
# 用途：檢查自動發現到的 staged package。
grafana-util change check --workspace . --output-format json
```

**預期輸出：**
```json
{
  "status": "ready",
  "blockers": [],
  "warnings": []
}
```

```bash
# 用途：把 live 與 availability context 也合併進 staged check。
grafana-util change check --workspace . --fetch-live --availability-file ./availability.json
```

**預期輸出：**
```text
PREFLIGHT CHECK:
- dashboards: valid
- datasources: valid
- result: 0 blockers
```

## 成功判準

- 結果能清楚區分 hard blockers 與較輕的 warnings
- 另一位維護者或 CI job 可以直接據此停下流程
- live-backed 檢查結果和你原本要操作的目標環境一致

## 失敗時先檢查

- 如果 blockers 出現得很意外，先確認 staged files 與 availability hints 來自同一環境
- 如果 live-backed 結果怪怪的，先回頭確認認證、org scope 與 Grafana URL
- 如果自動化在讀 JSON，先驗結果形狀，再讀 `status`、`blockers`、`warnings`

## 相關指令

- [change](./change.md)
- [change inspect](./change-inspect.md)
- [change preview](./change-preview.md)
- [status](./status.md)
