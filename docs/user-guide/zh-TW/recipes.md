# 🍳 實戰錦囊與最佳實踐

本章整理幾個真的會用到的 Grafana 維運配方。重點不只是指令本身，而是什麼時候該用、成功長什麼樣、失敗先查哪裡。

## 適用對象

- 想快速套用既有維運做法的人
- 需要把 dashboard、alert、access 的常見操作做成標準套路的人
- 想先看成功判準與失敗檢查點的人

## 主要目標

- 把常見情境拆成可重複的做法
- 先讓你知道哪個 lane、哪個流程比較對
- 減少靠猜來操作 Grafana 的次數

## 採用前後對照

- 以前：維運範例看起來像沒有結構的命令集合。
- 現在：每個錦囊都先說問題、再說安全路徑、再說預期結果，最後補上要先檢查的失敗點。

## 成功判準

- 你可以直接照著範例走，而且知道第一步哪裡是安全的。
- 你看得懂什麼結果代表工作流真的成功。
- 你也知道哪一種失敗應該先停下來，不要直接碰 live。

## 失敗時先檢查

- 如果錦囊假設的 source lane 不對，先停下來調整，不要繼續。
- 如果輸出或預覽跟你想要的結果不同，先檢查 staged input，再決定要不要動 live。
- 如果這個範例對你的任務還是太薄，請改看指令詳細說明取得精確旗標。

---

## 🚀 錦囊 1：跨環境儀表板遷移 (Dev -> Prod)

**問題描述**：直接從開發環境匯出再匯入到正式環境，常會因 org ID、folder context 或 data source UID 不一致而失敗。

**採用前**：遷移常常要手動清理匯出檔、猜 lane 該用哪一個，然後在匯入時才發現來源環境 metadata 根本不適合帶過去。

**採用後**：遷移會變成 lane 選擇加 replay 流程，比較容易審查，也比較容易說清楚做法。

**解決方案**：利用 **`prompt/` lane** 做乾淨遷移。

1. **從 Dev 匯出**：`grafana-util dashboard export --export-dir ./dev-assets`
2. **定位乾淨來源**：使用 `./dev-assets/prompt/` 下的檔案。這些檔案已去除來源環境專屬 metadata。
3. **匯入到 Prod**：
   ```bash
# 用途：匯入到 Prod。
   grafana-util dashboard import --import-dir ./dev-assets/prompt --url https://prod-grafana --replace-existing
   ```

**適合什麼時候用**：來源與目標環境的 dashboard 意圖相同，但不想直接帶著來源環境 metadata 時。

**不適合什麼時候用**：如果你的目標是 raw backup/replay 或災難恢復，請從 `raw/` 開始，不要從 `prompt/` 開始。

**成功判準**：

- 匯入不會被來源環境專屬 metadata 干擾
- 目標 dashboard 能綁定到正確的 data source 與 folder
- 匯入後只需要最小化的人工清理

**失敗時先檢查**：

- 目標環境是否真的有對應 data source UID 或名稱
- 這次工作流到底該用 `prompt/` 還是 `raw/`
- 你的 credential 是否看得到目標 org 或 folder

---

## 🔍 錦囊 2：匯入前的依賴性盤點

**問題描述**：dashboard 匯入成功了，但 panel 因缺少 data source 而全部壞掉。

**採用前**：你常常要等匯入完成後，才知道某些 panel 根本接不到需要的 data source。

**採用後**：在正式匯入前，就能先知道缺哪個 data source、哪批 dashboard 會被卡住。

**解決方案**：匯入前先跑 **pre-import inspection**。

```bash
# 用途：解決方案：匯入前先跑 pre-import inspection。
grafana-util dashboard inspect-export --import-dir ./backups/raw --output-format report-table
```

**檢查重點**：確認報告中 `Sources` 欄位列出的每個 UID，都存在於目標環境的 `datasource list`。

**適合什麼時候用**：正式匯入前、promotion bundle 送審前，或確認某批 dashboard export 是否真的可攜時。

**成功判準**：

- 所有必要 data source UID 都在目標環境存在
- 缺少的依賴在匯入前就已被找出
- 你能說清楚哪些 dashboard 被阻擋、原因是什麼

**失敗時先檢查**：

- 目標環境是否用了不同的 data source 命名或 UID 慣例
- 你是否匯出了正確的 lane
- 目標 credential 是否真的能列出你預期的 data source

---

## 🛠️ 錦囊 3：批次加標籤或更名 (Surgical Patching)

**問題描述**：你需要一次幫很多 dashboard 加 tag 或做機械式更名。

**採用前**：這種小改動很容易退化成手改 JSON，或一支很難 review 的臨時腳本。

**採用後**：修改仍然是機械式的，但過程可以 review，匯入前也還能先 preview。

**解決方案**：在迴圈中使用 `patch-file`，然後在 replay 前先 preview。

```bash
# 用途：解決方案：在迴圈中使用 patch-file，然後在 replay 前先 preview。
for file in ./dashboards/raw/*.json; do
  grafana-util dashboard patch-file --input "$file" --tag "ManagedBySRE" --output "$file"
done

grafana-util dashboard import --import-dir ./dashboards/raw --replace-existing --dry-run --table
```

**適合什麼時候用**：修改是本機、機械式，而且可以清楚 review 的結構調整時。

**不適合什麼時候用**：如果 patch 邏輯太複雜、已經看不出風險，或它其實依賴 live discovery，就不該只用單純迴圈硬改。

**成功判準**：

- 改完的檔案仍能在 Git 中清楚 review
- 重複 patch 不會產生意外漂移
- 匯入前仍會先跑 `--dry-run`

**失敗時先檢查**：

- 你 patch 的是不是正確 lane 與正確檔案集合
- 這次修改是不是其實應該落在 `prompt/` 而不是 `raw/`
- 匯入前是否先跑了 `--dry-run`

---

## 🚨 錦囊 4：驗證告警路由邏輯

**問題描述**：通知策略很複雜，你很難只靠眼睛判斷告警最後會送到哪個 receiver。

**解決方案**：使用 `preview-route` 模擬匹配結果。

```bash
# 用途：解決方案：使用 preview-route 模擬匹配結果。
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label service=order \
  --severity critical
```

**目標**：確認輸出中的 `receiver` 是否符合預期的 Slack channel 或 PagerDuty service。

**適合什麼時候用**：label、route 或 notification policy 有變更，而你想在大家先假設「應該會送對地方」之前先得到明確答案時。

**成功判準**：

- 輸出的 receiver 與預期一致
- 本來應該分流 critical 路徑的 label，真的有分流成功
- preview 結果會在 plan/apply 之前先被 review

**失敗時先檢查**：

- preview 使用的 label 是否真的和 rule 實際會送出的 label 一樣
- desired alert files 與 notification policies 是否同步
- 問題是 route 邏輯，還是 rule 本身分類就錯了

---

## 💡 專家建議

- **UID 一致性**：務必在 JSON 中定義穩定的 `uid`，不要依賴自動遞增的 `id`。
- **預覽優先**：任何 live 變動前，先跑 `--dry-run`。
- **Git 整合**：只把 `raw/` 與 `desired/` 視為真正值得版本管理的核心來源。
- **先確認憑證範圍**：recipe 看起來失敗時，先確認 credential 是否真的看得到目標 org、folder 或管理員操作面。
- **角色分工**：流程選擇先看 handbook，需要精確旗標時再切到指令頁。

---
[⬅️ 上一章：技術參考手冊](reference.md) | [🏠 回首頁](index.md) | [➡️ 下一章：疑難排解與名詞解釋](troubleshooting.md)
