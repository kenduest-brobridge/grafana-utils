# Grafana Utilities 中文說明

英文版文件： [README.md](README.md)

這個 repo 用來把 Grafana 設定匯出成 JSON，方便備份、搬移、版本控管，以及再匯入到另一套 Grafana。

主要有兩個工具：

- `grafana-utils.py`：處理 dashboard 的匯出與匯入
- `grafana-alert-utils.py`：處理 alerting 資源的匯出與匯入，例如 alert rules、contact points、mute timings、notification policies、templates

適合的使用情境：

- 從既有 Grafana 匯出 dashboard 或 alerting 設定做備份
- 將設定從一套 Grafana 搬到另一套 Grafana
- 把 Grafana JSON 納入版本控制
- 準備 dashboard JSON 給 API 匯入，或給 Grafana Web UI 匯入並在匯入時重新對應 datasource

Dashboard 相關流程由 `grafana-utils.py` 處理：

- `python3 grafana-utils.py export ...`
- `python3 grafana-utils.py import ...`

Alerting 相關流程由 `grafana-alert-utils.py` 處理。這個工具獨立存在，因為 Grafana alerting 使用不同的 API 與檔案格式。

如果你需要完整的參數、範例與行為說明，請參考英文版 [README.md](README.md)。英文版仍是主要文件。
