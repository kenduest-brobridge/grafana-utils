# `grafana-util access team`

## 目的

列出、瀏覽、建立、修改、匯出、匯入、比對或刪除 Grafana 團隊。

## 使用時機

- 檢視團隊清單與團隊成員關係。
- 建立或更新團隊成員與管理員指派。
- 匯出或匯入團隊套件。
- 以 id 或精確名稱刪除團隊。

## 採用前後對照

- **採用前**：團隊成員關係常常散在 UI 側邊選單或零碎腳本裡。
- **採用後**：同一個命名空間就能處理 inventory、成員更新、匯出／匯入與刪除，而且認證方式一致。

## 成功判準

- 團隊成員變更都綁定到精確的 team id 或名稱
- 在新增或移除成員前，可以先看出管理員指派
- 匯出的套件可以在另一個環境重複使用，不必手動重建團隊

## 失敗時先檢查

- 如果 list、add、modify 或 delete 失敗，先確認這個 team 在選到的 org 裡存在，而且認證範圍正確
- 如果成員看起來不完整，先核對精確的 member 名稱，以及是否有加上 `--with-members`
- 如果匯入結果不如預期，先確認來源套件與目標環境，再重試

## 主要旗標

- `list`: `--query`, `--name`, `--with-members`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `browse`: `--query`, `--name`, `--with-members`, `--page`, `--per-page`
- `add`: `--name`, `--email`, `--member`, `--admin`, `--json`
- `modify`: `--team-id`, `--name`, `--add-member`, `--remove-member`, `--add-admin`, `--remove-admin`, `--json`
- `export` 與 `diff`: `--export-dir` 或 `--diff-dir`, `--overwrite`, `--dry-run`, `--with-members`
- `import`: `--import-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--team-id`, `--name`, `--yes`, `--json`

## 範例

```bash
# 用途：在新增或移除成員前，先確認 team membership。
grafana-util access team list --profile prod --output-format text
```

```bash
# 用途：建立一個有明確成員與管理員指派的 team。
grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes
```

```bash
# 用途：在切換環境前，先匯入 team 套件。
grafana-util access team add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name platform-team --email platform@example.com --member alice --admin alice --json
```

## 相關命令

- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access service-account](./access-service-account.md)
