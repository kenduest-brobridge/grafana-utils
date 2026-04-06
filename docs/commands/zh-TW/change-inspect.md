# `grafana-util change inspect`

## 用途

從自動發現或明確指定的輸入檢視 staged change package。

## 適用時機

- 當你想最快知道 staged package 裡到底有什麼時，先從這裡開始。
- 在 `change check` 或 `change preview` 前，先用它確認變更包規模。
- 如果 staged inputs 可能來自 mixed workspace artifacts，而不是單一 `desired.json`，它會比低階 `summary` 更自然。

## 採用前後對照

- **採用前**：變更包還只是磁碟上的一組目錄、檔案或 staged contracts。
- **採用後**：你會得到一份 overview 風格文件，知道哪些區塊被找到、整包看起來大概多大。

## 主要旗標

- `--workspace`：從 repo、export tree 或 provisioning tree 自動發現常見 staged inputs。
- `--desired-file`：直接檢視單一 desired change file。
- `--dashboard-export-dir`、`--dashboard-provisioning-dir`：明確指定 dashboard staged inputs。
- `--alert-export-dir`、`--datasource-provisioning-file`：補充 alert 與 datasource staged inputs。
- `--source-bundle`：直接檢視既有 source bundle，不走 per-surface 目錄。
- `--output-format`：輸出成 `text` 或 `json`。
- `--output-file`、`--also-stdout`：把輸出存成 review artifact。

## 範例

```bash
# 用途：從 repo-local inputs 檢視 staged package。
grafana-util change inspect --workspace .
```

**預期輸出：**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- datasources: 1 referenced inventory
- total impact: 11 operations
```

```bash
# 用途：直接從 export tree 檢視 staged package。
grafana-util change inspect --workspace ./dashboards/raw --output-format json
```

**預期輸出：**
```json
{
  "kind": "grafana-utils-overview",
  "schemaVersion": 1,
  "sections": [
    {
      "title": "Dashboards"
    }
  ],
  "projectStatus": {}
}
```

這代表 inspect 已成功找到 staged inputs，並輸出共用的 overview 風格文件，而不是停在 discovery 失敗。

## 成功判準

- 指令能明確告訴你找到哪些 staged surfaces
- 在進 preview 或 apply 前，就能先判斷這包變更是否合理
- JSON 輸出夠穩定，能直接交給別人 review 或附在審查紀錄裡

## 失敗時先檢查

- 如果 discovery 找不到任何東西，先改用明確輸入旗標，不要立刻假設檔案壞掉
- 如果變更包看起來太小或太大，先確認 `--workspace` 指到正確的 export tree 或 repo root
- 如果有自動化在讀 JSON，先驗 `kind` 與 `schemaVersion`

## 相關指令

- [change](./change.md)
- [change check](./change-check.md)
- [change preview](./change-preview.md)
- [overview](./overview.md)
