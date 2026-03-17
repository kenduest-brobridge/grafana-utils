# Unit Test Command/Arguments Inventory

Generated from local repository test sources.

## Python CLI parsing tests

| File | Test | Scenario | Subcommand(s) | Arg/Flag Keywords |
| --- | --- | --- | --- | --- |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_without_command_prints_top_level_help` | `without command prints top level help` | `help` | `help, without` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_user_without_subcommand_prints_user_help` | `user without subcommand prints user help` | `help` | `help, without` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_team_without_subcommand_prints_team_help` | `team without subcommand prints team help` | `help` | `help, without` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_group_without_subcommand_prints_team_help` | `group without subcommand prints team help` | `help` | `help, without` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_service_account_token_without_subcommand_prints_token_help` | `service account token without subcommand prints token help` | `help` | `help, token, without` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_list_mode` | `supports user list mode` | `list` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_access_list_output_format` | `supports access list output format` | `list` | `format, output, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_rejects_access_output_format_with_legacy_flags` | `rejects access output format with legacy flags` | `generic` | `flags, format, legacy, output, rejects, with` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_org_list_mode` | `supports org list mode` | `list` | `mode, org, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_org_add_mode` | `supports org add mode` | `add` | `mode, org, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_org_modify_mode` | `supports org modify mode` | `modify` | `mode, org, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_org_delete_mode` | `supports org delete mode` | `delete` | `mode, org, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_org_export_mode` | `supports org export mode` | `export` | `mode, org, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_org_import_mode` | `supports org import mode` | `import` | `mode, org, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_add_mode` | `supports user add mode` | `add` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_add_password_file_mode` | `supports user add password file mode` | `add` | `mode, password, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_add_prompt_password_mode` | `supports user add prompt password mode` | `add` | `mode, password, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_rejects_multiple_user_add_password_sources` | `rejects multiple user add password sources` | `add` | `password, rejects` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_modify_mode` | `supports user modify mode` | `modify` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_modify_password_file_mode` | `supports user modify password file mode` | `modify` | `mode, password, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_modify_prompt_password_mode` | `supports user modify prompt password mode` | `modify` | `mode, password, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_rejects_multiple_user_modify_password_sources` | `rejects multiple user modify password sources` | `modify` | `password, rejects` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_delete_mode` | `supports user delete mode` | `delete` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_export_mode` | `supports user export mode` | `export` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_import_mode` | `supports user import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_diff_mode` | `supports user diff mode` | `diff` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_list_mode` | `supports team list mode` | `list` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_export_mode` | `supports team export mode` | `export` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_import_mode` | `supports team import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_diff_mode` | `supports team diff mode` | `diff` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_add_mode` | `supports team add mode` | `add` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_modify_mode` | `supports team modify mode` | `modify` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_delete_mode` | `supports team delete mode` | `delete` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_export_mode` | `supports user export mode` | `export` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_user_import_mode` | `supports user import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_export_mode` | `supports team export mode` | `export` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_team_import_mode` | `supports team import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_group_alias_mode` | `supports group alias mode` | `generic` | `alias, mode, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_preferred_auth_aliases` | `supports preferred auth aliases` | `generic` | `supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_rejects_legacy_basic_auth_aliases` | `rejects legacy basic auth aliases` | `generic` | `legacy, rejects` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_prompt_password` | `supports prompt password` | `generic` | `password, supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_prompt_token` | `supports prompt token` | `generic` | `supports, token` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_insecure_and_ca_cert` | `supports insecure and ca cert` | `generic` | `supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_insecure_on_destructive_commands` | `supports insecure on destructive commands` | `generic` | `supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_rejects_conflicting_tls_flags` | `rejects conflicting tls flags` | `generic` | `flags, rejects` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_service_account_export` | `supports service account export` | `export` | `supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_service_account_import_and_diff` | `supports service account import and diff` | `diff, import` | `supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_service_account_token_add` | `supports service account token add` | `add` | `supports, token` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_service_account_delete` | `supports service account delete` | `delete` | `supports` |
| python/python/tests/test_python_access_cli.py | `test_access_parse_args_supports_service_account_token_delete` | `supports service account token delete` | `delete` | `supports, token` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_import_mode` | `supports import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_diff_mode` | `supports diff mode` | `diff` | `mode, supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_dry_run` | `supports dry run` | `run` | `dry, run, supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_export_subcommand` | `supports export subcommand` | `export` | `supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_import_subcommand` | `supports import subcommand` | `import` | `supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_diff_subcommand` | `supports diff subcommand` | `diff` | `supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_list_rules_subcommand` | `supports list rules subcommand` | `list` | `supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_alert_list_output_format` | `supports alert list output format` | `list` | `format, output, supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_rejects_alert_output_format_with_legacy_flags` | `rejects alert output format with legacy flags` | `generic` | `flags, format, legacy, output, rejects, with` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_accepts_mapping_files` | `accepts mapping files` | `generic` | `n/a` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_defaults_output_dir_to_alerts` | `defaults output dir to alerts` | `generic` | `dir, output` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_defaults_url_to_local_grafana` | `defaults url to local grafana` | `generic` | `n/a` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_disables_ssl_verification_by_default` | `disables ssl verification by default` | `generic` | `n/a` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_can_enable_ssl_verification` | `can enable ssl verification` | `generic` | `enable` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_preferred_auth_aliases` | `supports preferred auth aliases` | `generic` | `supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_rejects_legacy_basic_auth_aliases` | `rejects legacy basic auth aliases` | `generic` | `legacy, rejects` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_prompt_password` | `supports prompt password` | `generic` | `password, supports` |
| python/python/tests/test_python_alert_cli.py | `test_alert_parse_args_supports_prompt_token` | `supports prompt token` | `generic` | `supports, token` |
| python/python/tests/test_python_dashboard_capture_cli.py | `test_dashboard_capture_parse_args_supports_inspect_vars` | `supports inspect vars` | `inspect` | `supports` |
| python/python/tests/test_python_dashboard_capture_cli.py | `test_dashboard_capture_parse_args_supports_screenshot` | `supports screenshot` | `screenshot` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_requires_subcommand` | `requires subcommand` | `generic` | `requires` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_mode` | `supports import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_org_id` | `supports import org id` | `import` | `id, org, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_require_matching_export_org` | `supports require matching export org` | `export` | `org, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_by_export_org_flags` | `supports import by export org flags` | `export, import` | `flags, org, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_only_org_id_without_use_export_org` | `rejects only org id without use export org` | `export` | `id, org, rejects, without` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_create_missing_orgs_without_use_export_org` | `rejects create missing orgs without use export org` | `export` | `org, rejects, without` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_use_export_org_with_org_id` | `rejects use export org with org id` | `export` | `id, org, rejects, with` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_use_export_org_with_require_matching_export_org` | `rejects use export org with require matching export org` | `export` | `org, rejects, with` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_create_missing_orgs_with_dry_run` | `supports create missing orgs with dry run` | `run` | `dry, run, supports, with` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_use_export_org_with_json_output` | `supports use export org with json output` | `export` | `json, org, output, supports, with` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_preferred_auth_aliases` | `supports preferred auth aliases` | `generic` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_legacy_basic_auth_aliases` | `rejects legacy basic auth aliases` | `generic` | `legacy, rejects` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_prompt_password` | `supports prompt password` | `generic` | `password, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_prompt_token` | `supports prompt token` | `generic` | `supports, token` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_list_mode` | `supports list mode` | `list` | `mode, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_list_org_selection` | `supports list org selection` | `list` | `org, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_list_data_sources_mode` | `supports list data sources mode` | `list` | `mode, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_list_output_format` | `supports list output format` | `list` | `format, output, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_list_csv_and_json_modes` | `supports list csv and json modes` | `list` | `csv, json, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_export_and_import_progress` | `supports export and import progress` | `export, import` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_multiple_list_output_modes` | `rejects multiple list output modes` | `list` | `output, rejects` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_multiple_list_data_sources_output_modes` | `rejects multiple list data sources output modes` | `list` | `output, rejects` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_list_output_format_with_legacy_flags` | `rejects list output format with legacy flags` | `list` | `flags, format, legacy, output, rejects, with` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_diff_mode` | `supports diff mode` | `diff` | `mode, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_defaults_export_dir_to_dashboards` | `defaults export dir to dashboards` | `export` | `dir` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_export_org_selection` | `supports export org selection` | `export` | `org, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_defaults_url_to_local_grafana` | `defaults url to local grafana` | `generic` | `n/a` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_variant_switches` | `supports variant switches` | `generic` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_export_dry_run` | `supports export dry run` | `export, run` | `dry, run, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_dry_run` | `supports import dry run` | `import, run` | `dry, run, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_dry_run_table_flags` | `supports import dry run table flags` | `import, run` | `dry, flags, run, supports, table` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_dry_run_json` | `supports import dry run json` | `import, run` | `dry, json, run, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_dry_run_output_format` | `supports import dry run output format` | `import, run` | `dry, format, output, run, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_import_dry_run_output_columns` | `supports import dry run output columns` | `import, run` | `dry, output, run, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_import_output_format_with_legacy_flags` | `rejects import output format with legacy flags` | `import` | `flags, format, legacy, output, rejects, with` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_import_output_columns_without_table_output` | `rejects import output columns without table output` | `import` | `output, rejects, table, without` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_update_existing_only` | `supports update existing only` | `generic` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_json` | `supports inspect export json` | `export, inspect` | `json, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_table` | `supports inspect export table` | `export, inspect` | `supports, table` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_output_format` | `supports inspect export output format` | `export, inspect` | `format, output, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_output_format_dependency` | `supports inspect export output format dependency` | `export, inspect` | `format, output, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_live_report_json` | `supports inspect live report json` | `inspect` | `json, live, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_live_report_tree_table` | `supports inspect live report tree table` | `inspect` | `live, supports, table` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_live_report_dependency` | `supports inspect live report dependency` | `inspect` | `live, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_live_output_format` | `supports inspect live output format` | `inspect` | `format, live, output, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_live_output_format_dependency` | `supports inspect live output format dependency` | `inspect` | `format, live, output, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_table` | `supports inspect export report table` | `export, inspect` | `supports, table` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_json` | `supports inspect export report json` | `export, inspect` | `json, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_csv` | `supports inspect export report csv` | `export, inspect` | `csv, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_tree` | `supports inspect export report tree` | `export, inspect` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_tree_table` | `supports inspect export report tree table` | `export, inspect` | `supports, table` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_dependency` | `supports inspect export report dependency` | `export, inspect` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_dependency_json` | `supports inspect export report dependency json` | `export, inspect` | `json, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_columns_and_filter` | `supports inspect export report columns and filter` | `export, inspect` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_inspect_export_report_panel_filter` | `supports inspect export report panel filter` | `export, inspect` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_ensure_folders` | `supports ensure folders` | `generic` | `supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_supports_require_matching_folder_path` | `supports require matching folder path` | `generic` | `path, supports` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_disables_ssl_verification_by_default` | `disables ssl verification by default` | `generic` | `n/a` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_can_enable_ssl_verification` | `can enable ssl verification` | `generic` | `enable` |
| python/python/tests/test_python_dashboard_cli.py | `test_dashboard_parse_args_rejects_old_list_subcommand_name` | `rejects old list subcommand name` | `list` | `rejects` |
| python/python/tests/test_python_dashboard_inspection_cli.py | `test_dashboard_inspection_parse_args_supports_governance_report_formats` | `supports governance report formats` | `generic` | `supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_list_mode` | `supports list mode` | `list` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_list_output_format` | `supports list output format` | `list` | `format, output, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_list_org_scoping` | `supports list org scoping` | `list` | `org, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_export_mode` | `supports export mode` | `export` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_export_org_scoping` | `supports export org scoping` | `export` | `org, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_import_mode` | `supports import mode` | `import` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_add_mode` | `supports add mode` | `add` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_add_auth_and_header_flags` | `supports add auth and header flags` | `add` | `flags, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_delete_mode` | `supports delete mode` | `delete` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_modify_mode` | `supports modify mode` | `modify` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_import_output_format` | `supports import output format` | `import` | `format, output, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_import_output_columns` | `supports import output columns` | `import` | `output, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_import_org_and_export_org_guard` | `supports import org and export org guard` | `export, import` | `guard, org, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_import_org_routing_flags` | `supports import org routing flags` | `import` | `flags, org, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_supports_diff_mode` | `supports diff mode` | `diff` | `mode, supports` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_multiple_list_output_modes` | `rejects multiple list output modes` | `list` | `output, rejects` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_list_all_orgs_with_org_id` | `rejects list all orgs with org id` | `list` | `all, id, org, rejects, with` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_output_format_with_legacy_list_flags` | `rejects output format with legacy list flags` | `list` | `flags, format, legacy, output, rejects, with` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_output_format_with_legacy_import_flags` | `rejects output format with legacy import flags` | `import` | `flags, format, legacy, output, rejects, with` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_import_output_columns_without_table_output` | `rejects import output columns without table output` | `import` | `output, rejects, table, without` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_export_all_orgs_with_org_id` | `rejects export all orgs with org id` | `export` | `all, id, org, rejects, with` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_only_org_id_without_use_export_org` | `rejects only org id without use export org` | `export` | `id, org, rejects, without` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_create_missing_orgs_without_use_export_org` | `rejects create missing orgs without use export org` | `export` | `org, rejects, without` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_use_export_org_with_org_id` | `rejects use export org with org id` | `export` | `id, org, rejects, with` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_use_export_org_with_require_matching_export_org` | `rejects use export org with require matching export org` | `export` | `org, rejects, with` |
| python/python/tests/test_python_datasource_cli.py | `test_datasource_parse_args_rejects_live_mutation_output_format_with_legacy_flags` | `rejects live mutation output format with legacy flags` | `generic` | `flags, format, legacy, live, output, rejects, with` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_without_command_prints_top_level_help` | `without command prints top level help` | `help` | `help, without` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_sync_without_subcommand_prints_sync_help` | `sync without subcommand prints sync help` | `help` | `help, without` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_dashboard_without_subcommand_prints_dashboard_help` | `dashboard without subcommand prints dashboard help` | `help` | `help, without` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_alert_without_subcommand_prints_alert_help` | `alert without subcommand prints alert help` | `help` | `help, without` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_access_without_subcommand_prints_access_help` | `access without subcommand prints access help` | `help` | `help, without` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_datasource_without_subcommand_prints_datasource_help` | `datasource without subcommand prints datasource help` | `help` | `help, without` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_dashboard_namespace` | `supports dashboard namespace` | `generic` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_dashboard_inspect_live_namespace` | `supports dashboard inspect live namespace` | `inspect` | `live, namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_alert_namespace` | `supports alert namespace` | `generic` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_alert_export_namespace` | `supports alert export namespace` | `export` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_access_namespace` | `supports access namespace` | `generic` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_datasource_namespace` | `supports datasource namespace` | `generic` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_datasource_alias` | `supports datasource alias` | `generic` | `alias, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_datasource_add_namespace` | `supports datasource add namespace` | `add` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_datasource_modify_namespace` | `supports datasource modify namespace` | `modify` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_rejects_unknown_datasource_subcommand` | `rejects unknown datasource subcommand` | `generic` | `rejects` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_datasource_diff_namespace` | `supports datasource diff namespace` | `diff` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_sync_summary_namespace` | `supports sync summary namespace` | `summary` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_sync_namespace` | `supports sync namespace` | `generic` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_supports_sync_preflight_namespace` | `supports sync preflight namespace` | `preflight` | `namespace, supports` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_rejects_unknown_top_level_command` | `rejects unknown top level command` | `generic` | `rejects` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_rejects_legacy_dashboard_direct_command` | `rejects legacy dashboard direct command` | `generic` | `legacy, rejects` |
| python/python/tests/test_python_unified_cli.py | `test_unified_parse_args_rejects_legacy_alert_direct_command` | `rejects legacy alert direct command` | `generic` | `legacy, rejects` |
| python/python/tests/test_python_unified_cli_dashboard_capture.py | `test_unified_cli_dashboard_capture_parse_args_supports_dashboard_inspect_vars_namespace` | `supports dashboard inspect vars namespace` | `inspect` | `namespace, supports` |
| python/python/tests/test_python_unified_cli_dashboard_capture.py | `test_unified_cli_dashboard_capture_parse_args_supports_dashboard_screenshot_namespace` | `supports dashboard screenshot namespace` | `screenshot` | `namespace, supports` |
| python/python/tests/test_python_unified_cli_dashboard_capture.py | `test_unified_cli_dashboard_capture_parse_args_supports_dashboard_alias` | `supports dashboard alias` | `generic` | `alias, supports` |
| python/python/tests/test_python_unified_cli_dashboard_capture.py | `test_unified_cli_dashboard_capture_parse_args_supports_sync_alias` | `supports sync alias` | `generic` | `alias, supports` |

## Rust parse/arg-adjacent tests (selected)

| File | Test |
| --- | --- |
