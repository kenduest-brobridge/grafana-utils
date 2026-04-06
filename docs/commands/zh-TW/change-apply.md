# `grafana-util change apply`

## 用途

把已審核的 preview 轉成 staged apply intent，或在你明確選擇時直接執行 live apply。

## 適用時機

- 當 preview 與 review 都已經完成時。
- 如果你需要 approval evidence 或 machine-readable apply intent，先用預設的 staged 形式。
- 只有真的準備好動手改 Grafana 時，才加上 `--execute-live`。
- 現在建議使用 `--preview-file`；`--plan-file` 仍保留為相容舊 staged workflows 的 alias。

## 採用前後對照

- **採用前**：preview 已存在，但 review 到 mutation 之間最後那一步仍然很依賴操作人判斷。
- **採用後**：apply 會把這一步拆成清楚的 staged intent 或 live execution result，並把 approval 帶進去。
  排序契約會保留在已審核的 preview 上：`ordering.mode`、`operations[].orderIndex` / `orderGroup` / `kindOrder`、以及 `summary.blocked_reasons` 都是用來描述操作順序與受阻工作量的 preview 欄位。

## 主要旗標

- `--preview-file`：要 apply 的 reviewed preview artifact。
- `--plan-file`：舊 plan-based staged workflows 的相容 alias。
- `--approve`：明確表示這一步可以繼續。
- `--execute-live`：從 staged intent 轉成真的 live 執行。
- `--approval-reason`、`--apply-note`：把人工核准背景帶進輸出。
- `--output-format`：輸出成 `text` 或 `json`。

## 範例

```bash
# 用途：先把 reviewed preview 轉成 staged apply intent。
grafana-util change apply --preview-file ./change-preview.json --approve --output-format json
```

**預期輸出：**
```json
{
  "kind": "grafana-utils-sync-apply-intent",
  "approved": true,
  "reviewed": true,
  "operations": []
}
```

這代表 apply 先建立了一份 staged intent 文件，而不是直接對 live Grafana 動手。

```bash
# 用途：把已核准的 preview 直接套用到 live Grafana。
grafana-util change apply --preview-file ./change-preview.json --approve --execute-live --profile prod
```

**預期輸出：**
```text
SYNC APPLY:
- mode: live
- applied: 5
- failed: 0
```

## 成功判準

- review lineage 能一路保留到 apply，不會在最後一哩消失
- staged apply JSON 足夠拿去跑核准流程、變更單或交接
- live apply 結果能清楚告訴你實際執行幾筆、是否有失敗

## 失敗時先檢查

- 如果 apply 不讓你繼續，先確認輸入 preview 是 reviewed artifact，且有帶 `--approve`
- 如果 staged intent 看起來對，但 live 結果不對，先比對 preview、可選的 preflight artifacts 與目標環境
- 如果自動化在讀輸出，先分清楚這是 staged `grafana-utils-sync-apply-intent` 還是 live apply 結果

## 相關指令

- [change](./change.md)
- [change preview](./change-preview.md)
- [change review](./change.md#review)
- [change preflight](./change.md#preflight)
- [status](./status.md)
