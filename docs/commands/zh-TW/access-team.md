# `grafana-util access team`

## 目的

列出、瀏覽、建立、修改、匯出、匯入、比對或刪除 Grafana 團隊。

## 使用時機

- 檢視團隊清單與團隊成員關係。
- 建立或更新團隊成員與管理員指派。
- 匯出或匯入團隊套件。
- 以 id 或精確名稱刪除團隊。

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
grafana-util access team list --profile prod --output-format text
grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes
grafana-util access team add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name platform-team --email platform@example.com --member alice --admin alice --json
```

## 相關命令

- [access](./access.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access service-account](./access-service-account.md)
