# `grafana-util access user`

## 目的

列出、瀏覽、建立、修改、匯出、匯入、比對或刪除 Grafana 使用者。

## 使用時機

- 檢視目前 org 或全域管理範圍內的使用者。
- 以登入名、電子郵件、角色與管理員設定建立或更新使用者。
- 匯出與匯入使用者清單套件。
- 從 org 成員關係或全域登錄中移除使用者。

## 主要旗標

- `list`: `--scope`, `--all-orgs`, `--query`, `--login`, `--email`, `--org-role`, `--grafana-admin`, `--with-teams`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `browse`: `--scope`, `--all-orgs`, `--current-org`, `--query`, `--login`, `--email`, `--org-role`, `--grafana-admin`, `--page`, `--per-page`
- `add`: `--login`, `--email`, `--name`, `--password` 或 `--password-file` 或 `--prompt-user-password`, `--org-role`, `--grafana-admin`, `--json`
- `modify`: `--user-id`, `--login`, `--email`, `--set-login`, `--set-email`, `--set-name`, `--set-password` 或 `--set-password-file` 或 `--prompt-set-password`, `--set-org-role`, `--set-grafana-admin`, `--json`
- `export` 與 `diff`: `--export-dir` 或 `--diff-dir`, `--overwrite`, `--dry-run`, `--scope`, `--with-teams`
- `import`: `--import-dir`, `--scope`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--user-id`, `--login`, `--email`, `--scope`, `--yes`, `--json`

## 範例

```bash
grafana-util access user list --profile prod --scope org --output-format text
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret
grafana-util access user list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --scope org --json
```

## 相關命令

- [access](./access.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
