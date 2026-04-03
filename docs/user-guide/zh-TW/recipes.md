# 🍳 實戰錦囊與最佳實踐

本章提供真正可落地的 Grafana 維運配方。重點不只是指令本身，而是：什麼時候該用、成功長什麼樣、失敗先查哪裡。

---

## 🚀 錦囊 1：跨環境儀表板遷移 (Dev -> Prod)

**問題描述**：直接從開發環境匯出再匯入到正式環境，常會因組織 ID、folder context 或 datasource UID 不一致而失敗。

**解決方案**：利用 **`prompt/` lane** 做乾淨遷移。

1. **從 Dev 匯出**：`grafana-util dashboard export --export-dir ./dev-assets`
2. **定位乾淨來源**：使用 `./dev-assets/prompt/` 下的檔案。這些檔案已去除來源環境專屬 metadata。
3. **匯入到 Prod**：
   ```bash
   grafana-util dashboard import --import-dir ./dev-assets/prompt --url https://prod-grafana --replace-existing
   ```

**適合什麼時候用**：來源與目標環境有相同 dashboard 意圖，但不應直接攜帶來源環境 metadata 時。

**不適合什麼時候用**：如果您的目標是 raw backup/replay 或災難恢復，請從 `raw/` 開始，不要從 `prompt/` 開始。

**成功判準**：

- 匯入不會被來源環境專屬 metadata 干擾
- 目標 dashboard 能綁定到正確的 datasource 與 folder
- 匯入後只需要最小化的人工清理

**失敗時先檢查**：

- 目標環境是否真的有對應 datasource UID 或名稱
- 這次工作流到底該用 `prompt/` 還是 `raw/`
- 您的 credential 是否看得到目標 org 或 folder

---

## 🔍 錦囊 2：匯入前的依賴性盤點

**問題描述**：dashboard 匯入成功了，但 panel 因缺少 datasource 而全部壞掉。

**解決方案**：匯入前先跑 **pre-import inspection**。

```bash
grafana-util dashboard inspect-export --import-dir ./backups/raw --output-format report-table
```

**檢查重點**：確認報告中 `Sources` 欄位列出的每個 UID，都存在於目標環境的 `datasource list`。

**適合什麼時候用**：正式匯入前、promotion bundle 送審前，或確認某批 dashboard export 是否真的可攜時。

**成功判準**：

- 所有必要 datasource UID 都在目標環境存在
- 缺少的依賴在匯入前就已被找出
- 您能說清楚哪些 dashboard 被阻擋、原因是什麼

**失敗時先檢查**：

- 目標環境是否用了不同的 datasource 命名或 UID 慣例
- 您是否匯出了正確的 lane
- 目標 credential 是否真的能列出您預期的 datasource

---

## 🛠️ 錦囊 3：批次加標籤或更名 (Surgical Patching)

**問題描述**：您需要一次幫很多 dashboard 加 tag 或做機械式更名。

**解決方案**：在迴圈中使用 `patch-file`，然後在 replay 前先 preview。

```bash
for file in ./dashboards/raw/*.json; do
  grafana-util dashboard patch-file --input "$file" --tag "ManagedBySRE" --output "$file"
done

grafana-util dashboard import --import-dir ./dashboards/raw --replace-existing --dry-run --table
```

**適合什麼時候用**：修改是本機、機械式、可審查的結構調整時。

**不適合什麼時候用**：如果 patch 邏輯太複雜、已經看不出風險，或它其實依賴 live discovery，就不該只用單純迴圈硬改。

**成功判準**：

- 改完的檔案仍能在 Git 中清楚 review
- 重複 patch 不會產生意外 drift
- 匯入前仍會先跑 `--dry-run`

**失敗時先檢查**：

- 您 patch 的是不是正確 lane 與正確檔案集合
- 這次修改是不是其實應該落在 `prompt/` 而不是 `raw/`
- 匯入前是否先跑了 `--dry-run`

---

## 🚨 錦囊 4：驗證告警路由邏輯

**問題描述**：通知策略複雜，您無法只靠眼睛判斷告警最後會送到哪個 receiver。

**解決方案**：使用 `preview-route` 模擬匹配結果。

```bash
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label service=order \
  --severity critical
```

**目標**：確認輸出中的 `receiver` 是否符合預期的 Slack channel 或 PagerDuty service。

**適合什麼時候用**：label、route 或 notification policy 有變更，而您希望在任何人假設「應該會送對地方」之前先得到明確答案時。

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
- **先確認憑證範圍**：recipe 看起來失敗時，先確認 credential 是否真的看得到目標 org、folder 或 admin surface。
- **角色分工**：workflow 選擇看 handbook，需要精確旗標時再切到逐指令頁。

---
[⬅️ 上一章：技術參考手冊](reference.md) | [🏠 回首頁](index.md) | [➡️ 下一章：疑難排解與名詞解釋](troubleshooting.md)
