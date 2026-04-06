# `grafana-util change preview`

## 用途

從自動發現或明確指定的 staged inputs 預覽這次會改到什麼。

## 適用時機

- 當你想取得目前 staged package 的可審查 preview artifact。
- 這是 task-first 路徑裡對應常見 `plan` 步驟的入口。
- 當你的問題是「這次會改什麼」，而不是「我該先跑哪種 builder」，就用它。

## 採用前後對照

- **採用前**：你知道這包東西存在，也知道它大致可用，但 create / update / delete 還沒被明確攤開。
- **採用後**：你會得到一份可 review、可交接、可進入 apply 的 staged preview contract。

## 主要旗標

- `--workspace`：從常見 repo-local inputs 自動發現 staged package。
- `--desired-file`：直接預覽單一 desired change file。
- `--source-bundle`、`--target-inventory`、`--mapping-file`、`--availability-file`：切進 bundle / promotion-aware 的 preview 路徑。
- `--live-file`：和一份保存好的 live-state 文件做比對。
- `--fetch-live`：直接去問 live Grafana。
- `--allow-prune`：讓 delete 類操作可以進入 preview。
- `--trace-id`：加上明確的 review lineage。
- `--output-format`、`--output-file`：輸出或保存 preview artifact。

## 範例

```bash
# 用途：把目前 staged package 對 live Grafana 的影響先預覽出來。
grafana-util change preview --workspace . --fetch-live --profile prod
```

**預期輸出：**
```text
SYNC PLAN:
- create: 1
- update: 4
- delete: 0
- blocked alerts: 0
```

```bash
# 用途：用明確 desired/live 文件產出 JSON preview。
grafana-util change preview --desired-file ./desired.json --live-file ./live.json --output-format json
```

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-plan",
  "reviewed": false,
  "ordering": {
    "mode": "dependency-aware"
  },
  "summary": {
    "would_create": 1,
    "would_update": 4,
    "would_delete": 0,
    "blocked_reasons": []
  },
  "operations": []
}
```

這是正常的 task-first preview contract。`reviewed: false` 代表 preview 已存在，但還沒進到 apply 可接受的審核狀態。
這份 public preview contract 也會帶上排序資訊：`ordering.mode`、每筆 operation 的 `orderIndex` / `orderGroup` / `kindOrder`，以及在有未接管工作時會出現的 `summary.blocked_reasons`。`change apply` 只是消費這份已審核的 preview，不會另外發明一套排序契約。

## 成功判準

- `summary` 的 create / update / delete 計數和你的預期相符
- preview 可以直接交給別人做 review
- 輸出足夠明確，後續能拿來對照 apply 行為

## 失敗時先檢查

- 如果 `--workspace` 失敗，先試一次明確 staged input flags，再判斷是不是 package 壞掉
- 如果 live-backed 結果跟預期差很大，先核對 auth、org scope 與目標 Grafana
- 如果輸出不是 sync plan，而是 bundle / promotion preflight kinds，先回頭確認你提供了哪些 staged inputs

## 相關指令

- [change](./change.md)
- [change check](./change-check.md)
- [change apply](./change-apply.md)
- [change advanced](./change.md#advanced)
