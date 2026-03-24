# 📊 Grafana Utilities (維運治理工具集)

[English Version](README.md) | **繁體中文版**

`grafana-utils` 是一套專為 Grafana 管理者與 SRE 打造的維運治理工具集。

`grafana-util` 適合拿來做：
- Dashboard、Datasource、Alert、Org、User、Team、Service Account 盤點
- Grafana 狀態的 export、import、diff、dry-run
- Dashboard 治理分析、查詢盤點、datasource 依賴檢查
- Dashboard 與 panel 的截圖或 PDF 擷取

### 支援矩陣 (Support Matrix)

| 模組 | 盤點 / 檢視 / 擷取 | 新增 / 修改 / 刪除 | 匯出 / 匯入 / 差異比對 | 備註 |
| --- | --- | --- | --- | --- |
| Dashboard | 是 | 否 | 是 | 以匯入驅動變更，支援資料夾感知遷移、模擬執行，以及截圖與 PDF 擷取 |
| Alerting | 是 | 否 | 是 | 以匯入驅動告警規則 (Rule) / 聯絡點 (Contact Point) 作業流程 |
| Datasource | 是 | 是 | 是 | 支援模擬執行、差異比對、全組織匯出，以及路由式多組織匯入與組織自動建立 |
| Access User | 是 | 是 | 是 | 支援 `--password-file` / `--prompt-user-password` 與 `--set-password-file` / `--prompt-set-password` |
| Access Org | 是 | 是 | 是 | 匯入時可重現組織成員關係 |
| Access Team | 是 | 是 | 是 | 成員關係可匯出 / 匯入 / 差異比對 |
| Access Service Account | 是 | 是 | 是 | 支援快照匯出/匯入/差異比對，以及 Token 新增/刪除作業流程 |


---

## 🏗️ 技術架構

目前維護中的 CLI 以 Rust `grafana-util` 二進位工具為主：
- 對外使用與 release 下載以 Rust binary 為主。
- Python 實作細節保留在 maintainer 文件中，供 parity 與驗證使用。

---

## 🛠️ 快速上手

### 安裝方式

下載入口：
- 最新版 release：`https://github.com/kenduest-brobridge/grafana-utils/releases/latest`
- 所有 releases：`https://github.com/kenduest-brobridge/grafana-utils/releases`

怎麼下載：
- 進入 release 頁面後展開 `Assets`
- 下載對應作業系統與 CPU 架構的 `grafana-util` 預編譯壓縮檔
- 如果目前沒有符合需求的 tagged release，就改成本地建置

本地建置：
```bash
cd rust && cargo build --release
```

### 常用情境範例

**批次匯出儀表板 (保留結構)：**
```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

**匯入前先執行模擬執行與比對：**
```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run --table
```

---

## 📄 文件導航

- **[繁體中文使用者指南](docs/user-guide-TW.md)**：包含全域參數、認證規則與各模組指令詳解。
- **[English User Guide](docs/user-guide.md)**: Standard operator instructions.
- **[技術細節 (Rust)](docs/overview-rust.md)**
- **[開發者手冊](docs/DEVELOPER.md)**：維護與貢獻說明。

---

## 📈 相容性與目標
- 支援 RHEL 8 / macOS 等作業系統。
- 執行型態：Rust release binary。
- Grafana 版本：支援 8.x, 9.x, 10.x+。

## 專案狀態

本專案仍在持續開發中，歡迎透過 GitHub Issues 或 Pull Requests 回報問題與使用回饋。
