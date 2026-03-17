# Python 呼叫階層關係（核心層）

僅包含 `grafana_utils/*.py` 及其子套件，不含 `tests/*` 與 `examples/*`。

## 全域入口（無本地上游）


## 全域高流量（呼叫數前 120）

- `dispatch_access_command` @ `grafana_utils/access/workflows.py:3159` （呼叫 36 / 被呼叫 0）
- `build_parser` @ `grafana_utils/access/parser.py:388` （呼叫 20 / 被呼叫 1）
- `evaluate_dashboard_governance_policy` @ `grafana_utils/dashboard_governance_gate.py:386` （呼叫 15 / 被呼叫 1）
- `parse_args` @ `grafana_utils/dashboard_cli.py:1020` （呼叫 13 / 被呼叫 1）
- `build_external_export_document` @ `grafana_utils/dashboards/transformer.py:709` （呼叫 11 / 被呼叫 0）
- `_run_import_datasources_for_single_org` @ `grafana_utils/datasource/workflows.py:1613` （呼叫 11 / 被呼叫 2）
- `export_alerting_resources` @ `grafana_utils/alert_cli.py:982` （呼叫 10 / 被呼叫 1）
- `main` @ `grafana_utils/dashboard_cli.py:1482` （呼叫 10 / 被呼叫 0）
- `_capture_via_devtools` @ `grafana_utils/dashboards/screenshot.py:1574` （呼叫 10 / 被呼叫 1）
- `import_users_with_client` @ `grafana_utils/access/workflows.py:1566` （呼叫 9 / 被呼叫 1）
- `main` @ `grafana_utils/sync_cli.py:1378` （呼叫 9 / 被呼叫 0）
- `export_datasources` @ `grafana_utils/datasource/workflows.py:1142` （呼叫 8 / 被呼叫 1）
- `build_dependency_graph_document` @ `grafana_utils/roadmap_workbench.py:287` （呼叫 8 / 被呼叫 0）
- `diff_teams_with_client` @ `grafana_utils/access/workflows.py:567` （呼叫 7 / 被呼叫 1）
- `import_service_accounts_with_client` @ `grafana_utils/access/workflows.py:1798` （呼叫 7 / 被呼叫 1）
- `import_teams_with_client` @ `grafana_utils/access/workflows.py:1991` （呼叫 7 / 被呼叫 1）
- `diff_alerting_resources` @ `grafana_utils/alert_cli.py:1234` （呼叫 7 / 被呼叫 1）
- `import_alerting_resources` @ `grafana_utils/alert_cli.py:1187` （呼叫 7 / 被呼叫 1）
- `build_parser` @ `grafana_utils/datasource/parser.py:673` （呼叫 7 / 被呼叫 0）
- `_resolve_multi_org_import_targets` @ `grafana_utils/datasource/workflows.py:1347` （呼叫 7 / 被呼叫 1）
- `dispatch_datasource_command` @ `grafana_utils/datasource/workflows.py:1757` （呼叫 7 / 被呼叫 0）
- `_load_dashboard_bundle_sections` @ `grafana_utils/sync_cli.py:894` （呼叫 7 / 被呼叫 1）
- `run_bundle` @ `grafana_utils/sync_cli.py:982` （呼叫 7 / 被呼叫 1）
- `run_bundle_preflight` @ `grafana_utils/sync_cli.py:1094` （呼叫 7 / 被呼叫 1）
- `run_preflight` @ `grafana_utils/sync_cli.py:1047` （呼叫 7 / 被呼叫 1）
- `_build_alert_list_plan` @ `grafana_utils/alert_cli.py:1354` （呼叫 6 / 被呼叫 1）
- `list_alert_resources` @ `grafana_utils/alert_cli.py:1416` （呼叫 6 / 被呼叫 1）
- `build_governance_risk_records` @ `grafana_utils/dashboards/inspection_governance.py:438` （呼叫 6 / 被呼叫 1）
- `list_dashboards` @ `grafana_utils/dashboards/listing.py:380` （呼叫 6 / 被呼叫 0）
- `build_sync_plan` @ `grafana_utils/gitops_sync.py:340` （呼叫 6 / 被呼叫 0）
- `build_user_rows` @ `grafana_utils/access/models.py:170` （呼叫 5 / 被呼叫 0）
- `apply_team_membership_changes` @ `grafana_utils/access/workflows.py:2871` （呼叫 5 / 被呼叫 2）
- `delete_user_with_client` @ `grafana_utils/access/workflows.py:2787` （呼叫 5 / 被呼叫 1）
- `diff_service_accounts_with_client` @ `grafana_utils/access/workflows.py:734` （呼叫 5 / 被呼叫 1）
- `diff_users_with_client` @ `grafana_utils/access/workflows.py:501` （呼叫 5 / 被呼叫 1）
- `export_service_accounts_with_client` @ `grafana_utils/access/workflows.py:1757` （呼叫 5 / 被呼叫 1）
- `list_orgs_with_client` @ `grafana_utils/access/workflows.py:2539` （呼叫 5 / 被呼叫 1）
- `import_resource_document` @ `grafana_utils/alert_cli.py:1165` （呼叫 5 / 被呼叫 1）
- `main` @ `grafana_utils/alert_cli.py:1475` （呼叫 5 / 被呼叫 0）
- `_extract_features` @ `grafana_utils/dashboards/inspection_dependency_models.py:148` （呼叫 5 / 被呼叫 1）
- `modify_datasource` @ `grafana_utils/datasource/workflows.py:966` （呼叫 5 / 被呼叫 1）
- `normalize_resource_spec` @ `grafana_utils/gitops_sync.py:127` （呼叫 5 / 被呼叫 2）
- `run_apply` @ `grafana_utils/sync_cli.py:1338` （呼叫 5 / 被呼叫 1）
- `run_plan` @ `grafana_utils/sync_cli.py:641` （呼叫 5 / 被呼叫 1）
- `run_summary` @ `grafana_utils/sync_cli.py:675` （呼叫 5 / 被呼叫 1）
- `_build_alert_checks` @ `grafana_utils/sync_preflight_workbench.py:227` （呼叫 5 / 被呼叫 1）
- `build_team_rows` @ `grafana_utils/access/models.py:336` （呼叫 4 / 被呼叫 0）
- `_sync_team_members_for_import` @ `grafana_utils/access/workflows.py:1037` （呼叫 4 / 被呼叫 1）
- `export_orgs_with_client` @ `grafana_utils/access/workflows.py:1334` （呼叫 4 / 被呼叫 1）
- `export_teams_with_client` @ `grafana_utils/access/workflows.py:1953` （呼叫 4 / 被呼叫 1）
- `export_users_with_client` @ `grafana_utils/access/workflows.py:1296` （呼叫 4 / 被呼叫 1）
- `import_orgs_with_client` @ `grafana_utils/access/workflows.py:1442` （呼叫 4 / 被呼叫 1）
- `build_parser` @ `grafana_utils/alert_cli.py:398` （呼叫 4 / 被呼叫 1）
- `determine_import_action` @ `grafana_utils/alerts/provisioning.py:787` （呼叫 4 / 被呼叫 0）
- `run_dashboard_governance_gate` @ `grafana_utils/dashboard_governance_gate.py:851` （呼叫 4 / 被呼叫 1）
- `_run_import_dashboards_by_export_org` @ `grafana_utils/dashboards/import_workflow.py:259` （呼叫 4 / 被呼叫 1）
- `build_default_query_analysis` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:199` （呼叫 4 / 被呼叫 0）
- `build_dashboard_dependency_records` @ `grafana_utils/dashboards/inspection_governance.py:332` （呼叫 4 / 被呼叫 1）
- `build_datasource_coverage_records` @ `grafana_utils/dashboards/inspection_governance.py:233` （呼叫 4 / 被呼叫 1）
- `build_datasource_family_coverage_records` @ `grafana_utils/dashboards/inspection_governance.py:174` （呼叫 4 / 被呼叫 1）
- `build_export_inspection_governance_document` @ `grafana_utils/dashboards/inspection_governance.py:532` （呼叫 4 / 被呼叫 0）
- `build_query_report_record` @ `grafana_utils/dashboards/inspection_report.py:350` （呼叫 4 / 被呼叫 1）
- `_write_full_page_output` @ `grafana_utils/dashboards/screenshot.py:1520` （呼叫 4 / 被呼叫 1）
- `build_capture_request` @ `grafana_utils/dashboards/screenshot.py:325` （呼叫 4 / 被呼叫 1）
- `build_dashboard_capture_url` @ `grafana_utils/dashboards/screenshot.py:225` （呼叫 4 / 被呼叫 2）
- `prepare_templating_for_external_import` @ `grafana_utils/dashboards/transformer.py:563` （呼叫 4 / 被呼叫 1）
- `resolve_datasource_ref` @ `grafana_utils/dashboards/transformer.py:337` （呼叫 4 / 被呼叫 3）
- `resolve_object_datasource_ref` @ `grafana_utils/dashboards/transformer.py:281` （呼叫 4 / 被呼叫 1）
- `resolve_string_datasource_ref` @ `grafana_utils/dashboards/transformer.py:208` （呼叫 4 / 被呼叫 1）
- `_run_import_datasources_by_export_org` @ `grafana_utils/datasource/workflows.py:1494` （呼叫 4 / 被呼叫 1）
- `add_datasource` @ `grafana_utils/datasource/workflows.py:908` （呼叫 4 / 被呼叫 1）
- `list_datasources` @ `grafana_utils/datasource/workflows.py:1080` （呼叫 4 / 被呼叫 3）
- `_load_alerting_bundle_section` @ `grafana_utils/sync_cli.py:933` （呼叫 4 / 被呼叫 1）
- `build_parser` @ `grafana_utils/sync_cli.py:130` （呼叫 4 / 被呼叫 1）
- `execute_live_apply` @ `grafana_utils/sync_cli.py:1301` （呼叫 4 / 被呼叫 1）
- `build_sync_preflight_document` @ `grafana_utils/sync_preflight_workbench.py:301` （呼叫 4 / 被呼叫 0）
- `parse_args` @ `grafana_utils/access/parser.py:1358` （呼叫 3 / 被呼叫 0）
- `_build_team_export_for_diff_records` @ `grafana_utils/access/workflows.py:470` （呼叫 3 / 被呼叫 1）
- `_build_user_diff_map` @ `grafana_utils/access/workflows.py:392` （呼叫 3 / 被呼叫 1）
- `_lookup_team_memberships_by_identity` @ `grafana_utils/access/workflows.py:998` （呼叫 3 / 被呼叫 1）
- `modify_team_with_client` @ `grafana_utils/access/workflows.py:2840` （呼叫 3 / 被呼叫 1）
- `modify_user_with_client` @ `grafana_utils/access/workflows.py:2732` （呼叫 3 / 被呼叫 1）
- `parse_args` @ `grafana_utils/alert_cli.py:469` （呼叫 3 / 被呼叫 1）
- `assess_alert_sync_specs` @ `grafana_utils/alert_sync_workbench.py:77` （呼叫 3 / 被呼叫 0）
- `build_contact_point_import_payload` @ `grafana_utils/alerts/provisioning.py:543` （呼叫 3 / 被呼叫 0）
- `build_linked_dashboard_metadata` @ `grafana_utils/alerts/provisioning.py:84` （呼叫 3 / 被呼叫 0）
- `build_mute_timing_import_payload` @ `grafana_utils/alerts/provisioning.py:565` （呼叫 3 / 被呼叫 0）
- `build_policies_import_payload` @ `grafana_utils/alerts/provisioning.py:587` （呼叫 3 / 被呼叫 0）
- `build_rule_import_payload` @ `grafana_utils/alerts/provisioning.py:521` （呼叫 3 / 被呼叫 0）
- `build_template_import_payload` @ `grafana_utils/alerts/provisioning.py:602` （呼叫 3 / 被呼叫 0）
- `rewrite_rule_dashboard_linkage` @ `grafana_utils/alerts/provisioning.py:281` （呼叫 3 / 被呼叫 1）
- `resolve_auth_from_namespace` @ `grafana_utils/auth_staging.py:180` （呼叫 3 / 被呼叫 1）
- `build_bundle_preflight_document` @ `grafana_utils/bundle_preflight_workbench.py:181` （呼叫 3 / 被呼叫 0）
- `normalize_permission_record` @ `grafana_utils/dashboard_permission_workbench.py:111` （呼叫 3 / 被呼叫 1）
- `analyze_query` @ `grafana_utils/dashboards/inspection_analyzers/loki.py:86` （呼叫 3 / 被呼叫 0）
- `build_dependency_rows_from_query_report` @ `grafana_utils/dashboards/inspection_dependency_models.py:302` （呼叫 3 / 被呼叫 0）
- `list_data_sources` @ `grafana_utils/dashboards/listing.py:573` （呼叫 3 / 被呼叫 0）
- `render_data_source_table` @ `grafana_utils/dashboards/listing.py:502` （呼叫 3 / 被呼叫 1）
- `_capture_full_page_segments` @ `grafana_utils/dashboards/screenshot.py:1394` （呼叫 3 / 被呼叫 1）
- `_capture_stitched_screenshot` @ `grafana_utils/dashboards/screenshot.py:1326` （呼叫 3 / 被呼叫 0）
- `capture_dashboard_screenshot` @ `grafana_utils/dashboards/screenshot.py:1692` （呼叫 3 / 被呼叫 0）
- `validate_screenshot_args` @ `grafana_utils/dashboards/screenshot.py:119` （呼叫 3 / 被呼叫 2）
- `allocate_input_mapping` @ `grafana_utils/dashboards/transformer.py:470` （呼叫 3 / 被呼叫 2）
- `resolve_placeholder_object_ref` @ `grafana_utils/dashboards/transformer.py:251` （呼叫 3 / 被呼叫 1）
- `extract_dashboard_variables` @ `grafana_utils/dashboards/variable_inspection.py:77` （呼叫 3 / 被呼叫 1）
- `inspect_dashboard_variables_with_client` @ `grafana_utils/dashboards/variable_inspection.py:38` （呼叫 3 / 被呼叫 0）
- `normalize_add_spec` @ `grafana_utils/datasource/live_mutation.py:107` （呼叫 3 / 被呼叫 1）
- `normalize_add_spec` @ `grafana_utils/datasource/live_mutation_safe.py:103` （呼叫 3 / 被呼叫 1）
- `plan_add_datasource` @ `grafana_utils/datasource/live_mutation_safe.py:201` （呼叫 3 / 被呼叫 1）
- `build_add_datasource_spec` @ `grafana_utils/datasource/workflows.py:668` （呼叫 3 / 被呼叫 1）
- `build_modify_datasource_updates` @ `grafana_utils/datasource/workflows.py:736` （呼叫 3 / 被呼叫 1）
- `import_datasources` @ `grafana_utils/datasource/workflows.py:1746` （呼叫 3 / 被呼叫 1）
- `parse_args` @ `grafana_utils/datasource_cli.py:175` （呼叫 3 / 被呼叫 1）
- `compare_datasource_inventory` @ `grafana_utils/datasource_diff.py:241` （呼叫 3 / 被呼叫 1）
- `RequestsJsonHttpTransport.request_json` @ `grafana_utils/http_transport.py:168` （呼叫 3 / 被呼叫 0）
- `build_promotion_plan_document` @ `grafana_utils/roadmap_workbench.py:663` （呼叫 3 / 被呼叫 0）
- `_apply_datasource_operation` @ `grafana_utils/sync_cli.py:1226` （呼叫 3 / 被呼叫 1）
- `load_plan_document` @ `grafana_utils/sync_cli.py:479` （呼叫 3 / 被呼叫 2）
- `run_assess_alerts` @ `grafana_utils/sync_cli.py:1075` （呼叫 3 / 被呼叫 1）
- `_collect_alert_datasource_uids` @ `grafana_utils/sync_preflight_workbench.py:168` （呼叫 3 / 被呼叫 1）

## `grafana_utils/__init__.py`

- 無可辨識函式。

## `grafana_utils/__main__.py`

- 無可辨識函式。

## `grafana_utils/access/__init__.py`

- 無可辨識函式。

## `grafana_utils/access/common.py`

- `GrafanaApiError.__init__` @ `grafana_utils/access/common.py:45`（上游 0 / 下游 0）

## `grafana_utils/access/models.py`

- `normalize_org_role` @ `grafana_utils/access/models.py:18`（上游 4 / 下游 0）
  - 被呼叫: normalize_global_user, normalize_org_user, normalize_service_account, user_matches_filters
- `normalize_bool` @ `grafana_utils/access/models.py:35`（上游 7 / 下游 0）
  - 被呼叫: format_service_account_summary_line, normalize_global_user, normalize_org_user, normalize_service_account, serialize_service_account_row, serialize_user_row, user_matches_filters
- `bool_label` @ `grafana_utils/access/models.py:49`（上游 3 / 下游 0）
  - 被呼叫: format_service_account_summary_line, serialize_service_account_row, serialize_user_row
- `normalize_team` @ `grafana_utils/access/models.py:58`（上游 1 / 下游 0）
  - 被呼叫: build_team_rows
- `normalize_org_user` @ `grafana_utils/access/models.py:69`（上游 1 / 下游 2）

  - 呼叫: normalize_bool, normalize_org_role
  - 被呼叫: build_user_rows
- `normalize_global_user` @ `grafana_utils/access/models.py:83`（上游 1 / 下游 2）

  - 呼叫: normalize_bool, normalize_org_role
  - 被呼叫: build_user_rows
- `normalize_service_account` @ `grafana_utils/access/models.py:99`（上游 0 / 下游 2）

  - 呼叫: normalize_bool, normalize_org_role
- `user_matches_filters` @ `grafana_utils/access/models.py:116`（上游 1 / 下游 2）

  - 呼叫: normalize_bool, normalize_org_role
  - 被呼叫: build_user_rows
- `paginate_users` @ `grafana_utils/access/models.py:141`（上游 1 / 下游 0）
  - 被呼叫: build_user_rows
- `attach_team_memberships` @ `grafana_utils/access/models.py:152`（上游 1 / 下游 0）
  - 被呼叫: build_user_rows
- `build_user_rows` @ `grafana_utils/access/models.py:170`（上游 0 / 下游 5）

  - 呼叫: attach_team_memberships, normalize_global_user, normalize_org_user, paginate_users, user_matches_filters
- `serialize_user_row` @ `grafana_utils/access/models.py:193`（上游 3 / 下游 2）

  - 呼叫: bool_label, normalize_bool
  - 被呼叫: render_user_csv, render_user_json, render_user_table
- `render_user_json` @ `grafana_utils/access/models.py:207`（上游 0 / 下游 1）

  - 呼叫: serialize_user_row
- `render_user_csv` @ `grafana_utils/access/models.py:217`（上游 0 / 下游 1）

  - 呼叫: serialize_user_row
- `render_user_table` @ `grafana_utils/access/models.py:231`（上游 0 / 下游 2）

  - 呼叫: build_row, serialize_user_row
- `render_user_table.build_row` @ `grafana_utils/access/models.py:259`（上游 1 / 下游 0）
  - 被呼叫: render_user_table
- `service_account_matches_query` @ `grafana_utils/access/models.py:273`（上游 0 / 下游 0）
- `team_matches_filters` @ `grafana_utils/access/models.py:292`（上游 1 / 下游 0）
  - 被呼叫: build_team_rows
- `paginate_teams` @ `grafana_utils/access/models.py:307`（上游 1 / 下游 0）
  - 被呼叫: build_team_rows
- `attach_team_members` @ `grafana_utils/access/models.py:318`（上游 1 / 下游 0）
  - 被呼叫: build_team_rows
- `build_team_rows` @ `grafana_utils/access/models.py:336`（上游 0 / 下游 4）

  - 呼叫: attach_team_members, normalize_team, paginate_teams, team_matches_filters
- `serialize_team_row` @ `grafana_utils/access/models.py:357`（上游 3 / 下游 0）
  - 被呼叫: render_team_csv, render_team_json, render_team_table
- `render_team_json` @ `grafana_utils/access/models.py:369`（上游 0 / 下游 1）

  - 呼叫: serialize_team_row
- `render_team_csv` @ `grafana_utils/access/models.py:379`（上游 0 / 下游 1）

  - 呼叫: serialize_team_row
- `render_team_table` @ `grafana_utils/access/models.py:393`（上游 0 / 下游 2）

  - 呼叫: build_row, serialize_team_row
- `render_team_table.build_row` @ `grafana_utils/access/models.py:418`（上游 1 / 下游 0）
  - 被呼叫: render_team_table
- `format_team_summary_line` @ `grafana_utils/access/models.py:433`（上游 0 / 下游 0）
- `format_team_modify_summary_line` @ `grafana_utils/access/models.py:453`（上游 0 / 下游 0）
- `format_team_add_summary_line` @ `grafana_utils/access/models.py:475`（上游 0 / 下游 0）
- `serialize_service_account_row` @ `grafana_utils/access/models.py:495`（上游 3 / 下游 2）

  - 呼叫: bool_label, normalize_bool
  - 被呼叫: render_service_account_csv, render_service_account_json, render_service_account_table
- `render_service_account_json` @ `grafana_utils/access/models.py:509`（上游 0 / 下游 1）

  - 呼叫: serialize_service_account_row
- `render_service_account_csv` @ `grafana_utils/access/models.py:522`（上游 0 / 下游 1）

  - 呼叫: serialize_service_account_row
- `render_service_account_table` @ `grafana_utils/access/models.py:534`（上游 0 / 下游 2）

  - 呼叫: build_row, serialize_service_account_row
- `render_service_account_table.build_row` @ `grafana_utils/access/models.py:561`（上游 1 / 下游 0）
  - 被呼叫: render_service_account_table
- `format_service_account_summary_line` @ `grafana_utils/access/models.py:578`（上游 0 / 下游 2）

  - 呼叫: bool_label, normalize_bool
- `serialize_service_account_token_row` @ `grafana_utils/access/models.py:598`（上游 1 / 下游 0）
  - 被呼叫: render_service_account_token_json
- `render_service_account_token_json` @ `grafana_utils/access/models.py:608`（上游 0 / 下游 1）

  - 呼叫: serialize_service_account_token_row

## `grafana_utils/access/parser.py`

- `subparser_kwargs` @ `grafana_utils/access/parser.py:240`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `positive_int` @ `grafana_utils/access/parser.py:248`（上游 0 / 下游 0）
- `bool_choice` @ `grafana_utils/access/parser.py:260`（上游 0 / 下游 0）
- `parser_help_kwargs` @ `grafana_utils/access/parser.py:272`（上游 0 / 下游 0）
- `add_list_output_format_arg` @ `grafana_utils/access/parser.py:284`（上游 4 / 下游 0）
  - 被呼叫: add_org_list_cli_args, add_service_account_list_cli_args, add_team_list_cli_args, add_user_list_cli_args
- `add_access_export_cli_args` @ `grafana_utils/access/parser.py:298`（上游 1 / 下游 1）

  - 呼叫: access_export_filename
  - 被呼叫: build_parser
- `add_access_import_cli_args` @ `grafana_utils/access/parser.py:324`（上游 1 / 下游 1）

  - 呼叫: access_export_filename
  - 被呼叫: build_parser
- `add_access_diff_cli_args` @ `grafana_utils/access/parser.py:365`（上游 1 / 下游 1）

  - 呼叫: access_export_filename
  - 被呼叫: build_parser
- `build_parser` @ `grafana_utils/access/parser.py:388`（上游 1 / 下游 20）

  - 呼叫: add_access_diff_cli_args, add_access_export_cli_args, add_access_import_cli_args, add_common_cli_args, add_org_add_cli_args, add_org_delete_cli_args, add_org_export_cli_args, add_org_list_cli_args, add_org_modify_cli_args, add_service_account_add_cli_args ...
  - 被呼叫: parse_args
- `add_common_cli_args` @ `grafana_utils/access/parser.py:668`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_user_list_cli_args` @ `grafana_utils/access/parser.py:761`（上游 1 / 下游 1）

  - 呼叫: add_list_output_format_arg
  - 被呼叫: build_parser
- `add_user_add_cli_args` @ `grafana_utils/access/parser.py:835`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_user_modify_cli_args` @ `grafana_utils/access/parser.py:889`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_user_delete_cli_args` @ `grafana_utils/access/parser.py:957`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_service_account_list_cli_args` @ `grafana_utils/access/parser.py:993`（上游 1 / 下游 1）

  - 呼叫: add_list_output_format_arg
  - 被呼叫: build_parser
- `add_org_list_cli_args` @ `grafana_utils/access/parser.py:1031`（上游 1 / 下游 1）

  - 呼叫: add_list_output_format_arg
  - 被呼叫: build_parser
- `add_org_add_cli_args` @ `grafana_utils/access/parser.py:1072`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_org_modify_cli_args` @ `grafana_utils/access/parser.py:1086`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_org_delete_cli_args` @ `grafana_utils/access/parser.py:1112`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_org_export_cli_args` @ `grafana_utils/access/parser.py:1138`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_team_list_cli_args` @ `grafana_utils/access/parser.py:1157`（上游 1 / 下游 1）

  - 呼叫: add_list_output_format_arg
  - 被呼叫: build_parser
- `add_team_modify_cli_args` @ `grafana_utils/access/parser.py:1205`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_team_add_cli_args` @ `grafana_utils/access/parser.py:1253`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_service_account_add_cli_args` @ `grafana_utils/access/parser.py:1286`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_service_account_token_add_cli_args` @ `grafana_utils/access/parser.py:1314`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `access_export_filename` @ `grafana_utils/access/parser.py:1345`（上游 3 / 下游 0）
  - 被呼叫: add_access_diff_cli_args, add_access_export_cli_args, add_access_import_cli_args
- `parse_args` @ `grafana_utils/access/parser.py:1358`（上游 0 / 下游 3）

  - 呼叫: _normalize_output_format_args, _validate_tls_args, build_parser
- `_normalize_output_format_args` @ `grafana_utils/access/parser.py:1403`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `_validate_tls_args` @ `grafana_utils/access/parser.py:1419`（上游 1 / 下游 0）
  - 被呼叫: parse_args

## `grafana_utils/access/pending_cli_staging.py`

- `add_team_delete_cli_args` @ `grafana_utils/access/pending_cli_staging.py:15`（上游 0 / 下游 0）
- `add_service_account_delete_cli_args` @ `grafana_utils/access/pending_cli_staging.py:44`（上游 0 / 下游 0）
- `add_service_account_token_delete_cli_args` @ `grafana_utils/access/pending_cli_staging.py:73`（上游 0 / 下游 0）
- `normalize_group_alias_argv` @ `grafana_utils/access/pending_cli_staging.py:115`（上游 0 / 下游 0）
- `validate_destructive_confirmed` @ `grafana_utils/access/pending_cli_staging.py:128`（上游 0 / 下游 0）
- `_select_exact_match` @ `grafana_utils/access/pending_cli_staging.py:138`（上游 3 / 下游 0）
  - 被呼叫: resolve_service_account_id, resolve_service_account_token_record, resolve_team_id
- `resolve_team_id` @ `grafana_utils/access/pending_cli_staging.py:161`（上游 0 / 下游 1）

  - 呼叫: _select_exact_match
- `resolve_service_account_id` @ `grafana_utils/access/pending_cli_staging.py:185`（上游 0 / 下游 1）

  - 呼叫: _select_exact_match
- `resolve_service_account_token_record` @ `grafana_utils/access/pending_cli_staging.py:211`（上游 0 / 下游 1）

  - 呼叫: _select_exact_match
- `build_team_delete_request` @ `grafana_utils/access/pending_cli_staging.py:240`（上游 0 / 下游 0）
- `build_service_account_delete_request` @ `grafana_utils/access/pending_cli_staging.py:253`（上游 0 / 下游 0）
- `build_service_account_token_delete_request` @ `grafana_utils/access/pending_cli_staging.py:268`（上游 0 / 下游 0）

## `grafana_utils/access/workflows.py`

- `validate_user_list_auth` @ `grafana_utils/access/workflows.py:59`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_org_auth` @ `grafana_utils/access/workflows.py:76`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_user_add_auth` @ `grafana_utils/access/workflows.py:85`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_user_modify_args` @ `grafana_utils/access/workflows.py:94`（上游 1 / 下游 0）
  - 被呼叫: modify_user_with_client
- `validate_user_modify_auth` @ `grafana_utils/access/workflows.py:123`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_user_delete_args` @ `grafana_utils/access/workflows.py:132`（上游 1 / 下游 0）
  - 被呼叫: delete_user_with_client
- `validate_user_delete_auth` @ `grafana_utils/access/workflows.py:138`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_team_modify_args` @ `grafana_utils/access/workflows.py:147`（上游 1 / 下游 0）
  - 被呼叫: modify_team_with_client
- `validate_team_delete_auth` @ `grafana_utils/access/workflows.py:161`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_service_account_delete_auth` @ `grafana_utils/access/workflows.py:166`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `validate_service_account_token_delete_auth` @ `grafana_utils/access/workflows.py:171`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `service_account_role_to_api` @ `grafana_utils/access/workflows.py:176`（上游 2 / 下游 0）
  - 被呼叫: add_service_account_with_client, import_service_accounts_with_client
- `_normalize_org_user_record` @ `grafana_utils/access/workflows.py:184`（上游 4 / 下游 0）
  - 被呼叫: _apply_org_user_import, _build_org_export_records, _build_org_rows, _normalize_org_record
- `_normalize_org_record` @ `grafana_utils/access/workflows.py:202`（上游 4 / 下游 1）

  - 呼叫: _normalize_org_user_record
  - 被呼叫: _build_org_export_records, _build_org_rows, import_orgs_with_client, lookup_organization
- `normalize_created_user` @ `grafana_utils/access/workflows.py:224`（上游 1 / 下游 0）
  - 被呼叫: add_user_with_client
- `_build_access_export_metadata` @ `grafana_utils/access/workflows.py:238`（上游 4 / 下游 0）
  - 被呼叫: export_orgs_with_client, export_service_accounts_with_client, export_teams_with_client, export_users_with_client
- `_build_user_export_records` @ `grafana_utils/access/workflows.py:249`（上游 1 / 下游 0）
  - 被呼叫: export_users_with_client
- `_build_team_export_records` @ `grafana_utils/access/workflows.py:272`（上游 1 / 下游 2）

  - 呼叫: extract_member_identity, team_member_admin_state
  - 被呼叫: export_teams_with_client
- `_build_org_export_records` @ `grafana_utils/access/workflows.py:304`（上游 1 / 下游 2）

  - 呼叫: _normalize_org_record, _normalize_org_user_record
  - 被呼叫: export_orgs_with_client
- `_normalize_user_record` @ `grafana_utils/access/workflows.py:328`（上游 2 / 下游 1）

  - 呼叫: _normalize_access_identity_list
  - 被呼叫: diff_users_with_client, import_users_with_client
- `_normalize_team_record` @ `grafana_utils/access/workflows.py:341`（上游 2 / 下游 1）

  - 呼叫: _normalize_access_identity_list
  - 被呼叫: diff_teams_with_client, import_teams_with_client
- `_normalize_user_for_diff` @ `grafana_utils/access/workflows.py:353`（上游 1 / 下游 1）

  - 呼叫: _normalize_access_identity_list
  - 被呼叫: _build_user_diff_map
- `_normalize_team_for_diff` @ `grafana_utils/access/workflows.py:373`（上游 3 / 下游 1）

  - 呼叫: _normalize_access_identity_list
  - 被呼叫: _build_team_diff_map, _build_team_export_for_diff_records, diff_teams_with_client
- `_build_user_diff_map` @ `grafana_utils/access/workflows.py:392`（上游 1 / 下游 3）

  - 呼叫: _normalize_access_import_identity, _normalize_user_for_diff, _resolve_access_user_key
  - 被呼叫: diff_users_with_client
- `_build_team_diff_map` @ `grafana_utils/access/workflows.py:414`（上游 1 / 下游 2）

  - 呼叫: _normalize_access_import_identity, _normalize_team_for_diff
  - 被呼叫: diff_teams_with_client
- `_record_diff_fields` @ `grafana_utils/access/workflows.py:433`（上游 3 / 下游 0）
  - 被呼叫: diff_service_accounts_with_client, diff_teams_with_client, diff_users_with_client
- `_build_user_export_for_diff_records` @ `grafana_utils/access/workflows.py:443`（上游 1 / 下游 0）
  - 被呼叫: diff_users_with_client
- `_build_team_export_for_diff_records` @ `grafana_utils/access/workflows.py:470`（上游 1 / 下游 3）

  - 呼叫: _normalize_team_for_diff, extract_member_identity, team_member_admin_state
  - 被呼叫: diff_teams_with_client
- `diff_users_with_client` @ `grafana_utils/access/workflows.py:501`（上游 1 / 下游 5）

  - 呼叫: _build_user_diff_map, _build_user_export_for_diff_records, _load_access_import_bundle, _normalize_user_record, _record_diff_fields
  - 被呼叫: dispatch_access_command
- `diff_teams_with_client` @ `grafana_utils/access/workflows.py:567`（上游 1 / 下游 7）

  - 呼叫: _build_team_diff_map, _build_team_export_for_diff_records, _load_access_import_bundle, _normalize_access_import_identity, _normalize_team_for_diff, _normalize_team_record, _record_diff_fields
  - 被呼叫: dispatch_access_command
- `_iter_service_accounts` @ `grafana_utils/access/workflows.py:640`（上游 2 / 下游 0）
  - 被呼叫: diff_service_accounts_with_client, export_service_accounts_with_client
- `_normalize_service_account_record` @ `grafana_utils/access/workflows.py:659`（上游 3 / 下游 0）
  - 被呼叫: diff_service_accounts_with_client, export_service_accounts_with_client, import_service_accounts_with_client
- `_normalize_service_account_for_diff` @ `grafana_utils/access/workflows.py:676`（上游 1 / 下游 0）
  - 被呼叫: _build_service_account_diff_map
- `_build_service_account_diff_map` @ `grafana_utils/access/workflows.py:689`（上游 1 / 下游 2）

  - 呼叫: _normalize_access_import_identity, _normalize_service_account_for_diff
  - 被呼叫: diff_service_accounts_with_client
- `_lookup_service_account_by_name` @ `grafana_utils/access/workflows.py:711`（上游 1 / 下游 0）
  - 被呼叫: import_service_accounts_with_client
- `diff_service_accounts_with_client` @ `grafana_utils/access/workflows.py:734`（上游 1 / 下游 5）

  - 呼叫: _build_service_account_diff_map, _iter_service_accounts, _load_access_import_bundle, _normalize_service_account_record, _record_diff_fields
  - 被呼叫: dispatch_access_command
- `_load_json_document` @ `grafana_utils/access/workflows.py:800`（上游 1 / 下游 0）
  - 被呼叫: _load_access_import_bundle
- `_write_json_document` @ `grafana_utils/access/workflows.py:814`（上游 4 / 下游 0）
  - 被呼叫: export_orgs_with_client, export_service_accounts_with_client, export_teams_with_client, export_users_with_client
- `_assert_not_overwriting` @ `grafana_utils/access/workflows.py:826`（上游 4 / 下游 0）
  - 被呼叫: export_orgs_with_client, export_service_accounts_with_client, export_teams_with_client, export_users_with_client
- `_normalize_access_import_identity` @ `grafana_utils/access/workflows.py:838`（上游 12 / 下游 0）
  - 被呼叫: _build_service_account_diff_map, _build_team_diff_map, _build_user_diff_map, _lookup_org_user_record, _lookup_team_memberships_by_identity, _merge_team_membership_target, _normalize_access_identity_list, _sync_team_members_for_import, diff_teams_with_client, import_orgs_with_client ...
- `_normalize_access_identity_list` @ `grafana_utils/access/workflows.py:843`（上游 8 / 下游 1）

  - 呼叫: _normalize_access_import_identity
  - 被呼叫: _merge_team_membership_target, _normalize_team_for_diff, _normalize_team_record, _normalize_user_for_diff, _normalize_user_record, _sync_team_members_for_import, import_teams_with_client, import_users_with_client
- `_build_access_import_preview_row` @ `grafana_utils/access/workflows.py:859`（上游 0 / 下游 0）
- `_render_access_import_preview_table` @ `grafana_utils/access/workflows.py:873`（上游 0 / 下游 0）
- `_validate_access_import_preview_output` @ `grafana_utils/access/workflows.py:908`（上游 0 / 下游 0）
- `_load_access_import_bundle` @ `grafana_utils/access/workflows.py:928`（上游 7 / 下游 1）

  - 呼叫: _load_json_document
  - 被呼叫: _build_org_import_records, _build_service_account_import_records, _build_team_import_records, _build_user_import_records, diff_service_accounts_with_client, diff_teams_with_client, diff_users_with_client
- `_resolve_access_user_key` @ `grafana_utils/access/workflows.py:971`（上游 2 / 下游 0）
  - 被呼叫: _build_user_diff_map, import_users_with_client
- `_build_access_user_payload` @ `grafana_utils/access/workflows.py:982`（上游 1 / 下游 0）
  - 被呼叫: import_users_with_client
- `_lookup_team_memberships_by_identity` @ `grafana_utils/access/workflows.py:998`（上游 1 / 下游 3）

  - 呼叫: _normalize_access_import_identity, extract_member_identity, team_member_admin_state
  - 被呼叫: import_teams_with_client
- `_merge_team_membership_target` @ `grafana_utils/access/workflows.py:1022`（上游 1 / 下游 2）

  - 呼叫: _normalize_access_identity_list, _normalize_access_import_identity
  - 被呼叫: _sync_team_members_for_import
- `_sync_team_members_for_import` @ `grafana_utils/access/workflows.py:1037`（上游 1 / 下游 4）

  - 呼叫: _merge_team_membership_target, _normalize_access_identity_list, _normalize_access_import_identity, lookup_org_user_by_identity
  - 被呼叫: import_teams_with_client
- `_build_user_import_records` @ `grafana_utils/access/workflows.py:1171`（上游 1 / 下游 1）

  - 呼叫: _load_access_import_bundle
  - 被呼叫: import_users_with_client
- `_build_team_import_records` @ `grafana_utils/access/workflows.py:1180`（上游 1 / 下游 1）

  - 呼叫: _load_access_import_bundle
  - 被呼叫: import_teams_with_client
- `_build_org_import_records` @ `grafana_utils/access/workflows.py:1189`（上游 1 / 下游 1）

  - 呼叫: _load_access_import_bundle
  - 被呼叫: import_orgs_with_client
- `_build_service_account_import_records` @ `grafana_utils/access/workflows.py:1198`（上游 1 / 下游 1）

  - 呼叫: _load_access_import_bundle
  - 被呼叫: import_service_accounts_with_client
- `_build_service_account_import_row` @ `grafana_utils/access/workflows.py:1207`（上游 1 / 下游 0）
  - 被呼叫: import_service_accounts_with_client
- `_render_service_account_import_table` @ `grafana_utils/access/workflows.py:1217`（上游 1 / 下游 1）

  - 呼叫: _format
  - 被呼叫: _emit_service_account_import_dry_run_output
- `_render_service_account_import_table._format` @ `grafana_utils/access/workflows.py:1233`（上游 1 / 下游 0）
  - 被呼叫: _render_service_account_import_table
- `_emit_service_account_import_dry_run_output` @ `grafana_utils/access/workflows.py:1246`（上游 1 / 下游 1）

  - 呼叫: _render_service_account_import_table
  - 被呼叫: import_service_accounts_with_client
- `validate_service_account_import_dry_run_output` @ `grafana_utils/access/workflows.py:1280`（上游 1 / 下游 0）
  - 被呼叫: import_service_accounts_with_client
- `export_users_with_client` @ `grafana_utils/access/workflows.py:1296`（上游 1 / 下游 4）

  - 呼叫: _assert_not_overwriting, _build_access_export_metadata, _build_user_export_records, _write_json_document
  - 被呼叫: dispatch_access_command
- `export_orgs_with_client` @ `grafana_utils/access/workflows.py:1334`（上游 1 / 下游 4）

  - 呼叫: _assert_not_overwriting, _build_access_export_metadata, _build_org_export_records, _write_json_document
  - 被呼叫: dispatch_access_command
- `_lookup_org_user_record` @ `grafana_utils/access/workflows.py:1372`（上游 1 / 下游 1）

  - 呼叫: _normalize_access_import_identity
  - 被呼叫: _apply_org_user_import
- `_apply_org_user_import` @ `grafana_utils/access/workflows.py:1385`（上游 1 / 下游 2）

  - 呼叫: _lookup_org_user_record, _normalize_org_user_record
  - 被呼叫: import_orgs_with_client
- `import_orgs_with_client` @ `grafana_utils/access/workflows.py:1442`（上游 1 / 下游 4）

  - 呼叫: _apply_org_user_import, _build_org_import_records, _normalize_access_import_identity, _normalize_org_record
  - 被呼叫: dispatch_access_command
- `import_users_with_client` @ `grafana_utils/access/workflows.py:1566`（上游 1 / 下游 9）

  - 呼叫: _build_access_user_payload, _build_user_import_records, _normalize_access_identity_list, _normalize_access_import_identity, _normalize_user_record, _resolve_access_user_key, lookup_global_user_by_identity, lookup_org_user_by_identity, lookup_team_by_name
  - 被呼叫: dispatch_access_command
- `export_service_accounts_with_client` @ `grafana_utils/access/workflows.py:1757`（上游 1 / 下游 5）

  - 呼叫: _assert_not_overwriting, _build_access_export_metadata, _iter_service_accounts, _normalize_service_account_record, _write_json_document
  - 被呼叫: dispatch_access_command
- `import_service_accounts_with_client` @ `grafana_utils/access/workflows.py:1798`（上游 1 / 下游 7）

  - 呼叫: _build_service_account_import_records, _build_service_account_import_row, _emit_service_account_import_dry_run_output, _lookup_service_account_by_name, _normalize_service_account_record, service_account_role_to_api, validate_service_account_import_dry_run_output
  - 被呼叫: dispatch_access_command
- `export_teams_with_client` @ `grafana_utils/access/workflows.py:1953`（上游 1 / 下游 4）

  - 呼叫: _assert_not_overwriting, _build_access_export_metadata, _build_team_export_records, _write_json_document
  - 被呼叫: dispatch_access_command
- `import_teams_with_client` @ `grafana_utils/access/workflows.py:1991`（上游 1 / 下游 7）

  - 呼叫: _build_team_import_records, _lookup_team_memberships_by_identity, _normalize_access_identity_list, _normalize_access_import_identity, _normalize_team_record, _sync_team_members_for_import, lookup_team_by_name
  - 被呼叫: dispatch_access_command
- `lookup_service_account_id_by_name` @ `grafana_utils/access/workflows.py:2091`（上游 1 / 下游 0）
  - 被呼叫: add_service_account_token_with_client
- `lookup_team_by_name` @ `grafana_utils/access/workflows.py:2120`（上游 3 / 下游 0）
  - 被呼叫: import_teams_with_client, import_users_with_client, modify_team_with_client
- `lookup_org_user_by_identity` @ `grafana_utils/access/workflows.py:2137`（上游 4 / 下游 0）
  - 被呼叫: _sync_team_members_for_import, apply_team_membership_changes, delete_user_with_client, import_users_with_client
- `lookup_global_user_by_identity` @ `grafana_utils/access/workflows.py:2159`（上游 3 / 下游 0）
  - 被呼叫: delete_user_with_client, import_users_with_client, modify_user_with_client
- `lookup_org_user_by_user_id` @ `grafana_utils/access/workflows.py:2186`（上游 1 / 下游 0）
  - 被呼叫: delete_user_with_client
- `normalize_modified_user` @ `grafana_utils/access/workflows.py:2205`（上游 1 / 下游 0）
  - 被呼叫: modify_user_with_client
- `normalize_deleted_user` @ `grafana_utils/access/workflows.py:2225`（上游 1 / 下游 0）
  - 被呼叫: delete_user_with_client
- `normalize_identity_list` @ `grafana_utils/access/workflows.py:2246`（上游 1 / 下游 0）
  - 被呼叫: apply_team_membership_changes
- `validate_conflicting_identity_sets` @ `grafana_utils/access/workflows.py:2259`（上游 1 / 下游 0）
  - 被呼叫: apply_team_membership_changes
- `team_member_admin_state` @ `grafana_utils/access/workflows.py:2269`（上游 4 / 下游 0）
  - 被呼叫: _build_team_export_for_diff_records, _build_team_export_records, _lookup_team_memberships_by_identity, apply_team_membership_changes
- `extract_member_identity` @ `grafana_utils/access/workflows.py:2297`（上游 4 / 下游 0）
  - 被呼叫: _build_team_export_for_diff_records, _build_team_export_records, _lookup_team_memberships_by_identity, apply_team_membership_changes
- `format_user_summary_line` @ `grafana_utils/access/workflows.py:2304`（上游 1 / 下游 0）
  - 被呼叫: list_users_with_client
- `format_deleted_team_summary_line` @ `grafana_utils/access/workflows.py:2328`（上游 1 / 下游 0）
  - 被呼叫: delete_team_with_client
- `format_deleted_service_account_summary_line` @ `grafana_utils/access/workflows.py:2343`（上游 1 / 下游 0）
  - 被呼叫: delete_service_account_with_client
- `format_deleted_service_account_token_summary_line` @ `grafana_utils/access/workflows.py:2358`（上游 1 / 下游 0）
  - 被呼叫: delete_service_account_token_with_client
- `list_users_with_client` @ `grafana_utils/access/workflows.py:2371`（上游 1 / 下游 1）

  - 呼叫: format_user_summary_line
  - 被呼叫: dispatch_access_command
- `list_service_accounts_with_client` @ `grafana_utils/access/workflows.py:2394`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `list_teams_with_client` @ `grafana_utils/access/workflows.py:2424`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `_build_org_rows` @ `grafana_utils/access/workflows.py:2444`（上游 1 / 下游 2）

  - 呼叫: _normalize_org_record, _normalize_org_user_record
  - 被呼叫: list_orgs_with_client
- `_render_org_table` @ `grafana_utils/access/workflows.py:2472`（上游 1 / 下游 1）

  - 呼叫: _format
  - 被呼叫: list_orgs_with_client
- `_render_org_table._format` @ `grafana_utils/access/workflows.py:2487`（上游 1 / 下游 0）
  - 被呼叫: _render_org_table
- `_render_org_csv` @ `grafana_utils/access/workflows.py:2502`（上游 1 / 下游 1）

  - 呼叫: _csv_escape
  - 被呼叫: list_orgs_with_client
- `_csv_escape` @ `grafana_utils/access/workflows.py:2516`（上游 1 / 下游 0）
  - 被呼叫: _render_org_csv
- `_render_org_json` @ `grafana_utils/access/workflows.py:2524`（上游 1 / 下游 0）
  - 被呼叫: list_orgs_with_client
- `_format_org_summary_line` @ `grafana_utils/access/workflows.py:2529`（上游 1 / 下游 0）
  - 被呼叫: list_orgs_with_client
- `list_orgs_with_client` @ `grafana_utils/access/workflows.py:2539`（上游 1 / 下游 5）

  - 呼叫: _build_org_rows, _format_org_summary_line, _render_org_csv, _render_org_json, _render_org_table
  - 被呼叫: dispatch_access_command
- `add_service_account_with_client` @ `grafana_utils/access/workflows.py:2563`（上游 1 / 下游 1）

  - 呼叫: service_account_role_to_api
  - 被呼叫: dispatch_access_command
- `lookup_organization` @ `grafana_utils/access/workflows.py:2592`（上游 2 / 下游 1）

  - 呼叫: _normalize_org_record
  - 被呼叫: delete_org_with_client, modify_org_with_client
- `add_org_with_client` @ `grafana_utils/access/workflows.py:2619`（上游 1 / 下游 0）
  - 被呼叫: dispatch_access_command
- `modify_org_with_client` @ `grafana_utils/access/workflows.py:2636`（上游 1 / 下游 1）

  - 呼叫: lookup_organization
  - 被呼叫: dispatch_access_command
- `delete_org_with_client` @ `grafana_utils/access/workflows.py:2668`（上游 1 / 下游 1）

  - 呼叫: lookup_organization
  - 被呼叫: dispatch_access_command
- `add_user_with_client` @ `grafana_utils/access/workflows.py:2692`（上游 1 / 下游 1）

  - 呼叫: normalize_created_user
  - 被呼叫: dispatch_access_command
- `modify_user_with_client` @ `grafana_utils/access/workflows.py:2732`（上游 1 / 下游 3）

  - 呼叫: lookup_global_user_by_identity, normalize_modified_user, validate_user_modify_args
  - 被呼叫: dispatch_access_command
- `delete_user_with_client` @ `grafana_utils/access/workflows.py:2787`（上游 1 / 下游 5）

  - 呼叫: lookup_global_user_by_identity, lookup_org_user_by_identity, lookup_org_user_by_user_id, normalize_deleted_user, validate_user_delete_args
  - 被呼叫: dispatch_access_command
- `modify_team_with_client` @ `grafana_utils/access/workflows.py:2840`（上游 1 / 下游 3）

  - 呼叫: apply_team_membership_changes, lookup_team_by_name, validate_team_modify_args
  - 被呼叫: dispatch_access_command
- `apply_team_membership_changes` @ `grafana_utils/access/workflows.py:2871`（上游 2 / 下游 5）

  - 呼叫: extract_member_identity, lookup_org_user_by_identity, normalize_identity_list, team_member_admin_state, validate_conflicting_identity_sets
  - 被呼叫: add_team_with_client, modify_team_with_client
- `add_team_with_client` @ `grafana_utils/access/workflows.py:3013`（上游 1 / 下游 1）

  - 呼叫: apply_team_membership_changes
  - 被呼叫: dispatch_access_command
- `add_service_account_token_with_client` @ `grafana_utils/access/workflows.py:3045`（上游 1 / 下游 1）

  - 呼叫: lookup_service_account_id_by_name
  - 被呼叫: dispatch_access_command
- `delete_service_account_with_client` @ `grafana_utils/access/workflows.py:3068`（上游 1 / 下游 1）

  - 呼叫: format_deleted_service_account_summary_line
  - 被呼叫: dispatch_access_command
- `delete_service_account_token_with_client` @ `grafana_utils/access/workflows.py:3097`（上游 1 / 下游 1）

  - 呼叫: format_deleted_service_account_token_summary_line
  - 被呼叫: dispatch_access_command
- `delete_team_with_client` @ `grafana_utils/access/workflows.py:3138`（上游 1 / 下游 1）

  - 呼叫: format_deleted_team_summary_line
  - 被呼叫: dispatch_access_command
- `dispatch_access_command` @ `grafana_utils/access/workflows.py:3159`（上游 0 / 下游 36）

  - 呼叫: add_org_with_client, add_service_account_token_with_client, add_service_account_with_client, add_team_with_client, add_user_with_client, delete_org_with_client, delete_service_account_token_with_client, delete_service_account_with_client, delete_team_with_client, delete_user_with_client ...

## `grafana_utils/access_cli.py`

- `resolve_auth` @ `grafana_utils/access_cli.py:127`（上游 1 / 下游 0）
  - 被呼叫: build_request_headers
- `build_request_headers` @ `grafana_utils/access_cli.py:144`（上游 1 / 下游 1）

  - 呼叫: resolve_auth
  - 被呼叫: run
- `_read_secret_file` @ `grafana_utils/access_cli.py:149`（上游 1 / 下游 0）
  - 被呼叫: resolve_user_secret_inputs
- `resolve_user_secret_inputs` @ `grafana_utils/access_cli.py:166`（上游 1 / 下游 1）

  - 呼叫: _read_secret_file
  - 被呼叫: main
- `run` @ `grafana_utils/access_cli.py:191`（上游 1 / 下游 1）

  - 呼叫: build_request_headers
  - 被呼叫: main
- `main` @ `grafana_utils/access_cli.py:214`（上游 0 / 下游 2）

  - 呼叫: resolve_user_secret_inputs, run

## `grafana_utils/alert_cli.py`

- `add_common_args` @ `grafana_utils/alert_cli.py:135`（上游 2 / 下游 0）
  - 被呼叫: build_legacy_parser, build_parser
- `add_export_args` @ `grafana_utils/alert_cli.py:201`（上游 2 / 下游 0）
  - 被呼叫: build_legacy_parser, build_parser
- `add_list_args` @ `grafana_utils/alert_cli.py:227`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_import_args` @ `grafana_utils/alert_cli.py:280`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `build_legacy_parser` @ `grafana_utils/alert_cli.py:336`（上游 1 / 下游 2）

  - 呼叫: add_common_args, add_export_args
  - 被呼叫: parse_args
- `build_parser` @ `grafana_utils/alert_cli.py:398`（上游 1 / 下游 4）

  - 呼叫: add_common_args, add_export_args, add_import_args, add_list_args
  - 被呼叫: parse_args
- `parse_args` @ `grafana_utils/alert_cli.py:469`（上游 1 / 下游 3）

  - 呼叫: _normalize_output_format_args, build_legacy_parser, build_parser
  - 被呼叫: main
- `_normalize_output_format_args` @ `grafana_utils/alert_cli.py:516`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `resolve_auth` @ `grafana_utils/alert_cli.py:535`（上游 3 / 下游 0）
  - 被呼叫: _build_alert_list_plan, build_client, build_client_for_org
- `sanitize_path_component` @ `grafana_utils/alert_cli.py:549`（上游 4 / 下游 0）
  - 被呼叫: build_contact_point_output_path, build_mute_timing_output_path, build_rule_output_path, build_template_output_path
- `write_json` @ `grafana_utils/alert_cli.py:558`（上游 7 / 下游 0）
  - 被呼叫: export_alerting_resources, export_contact_point_documents, export_mute_timing_documents, export_policies_document, export_rule_documents, export_template_documents, write_resource_indexes
- `load_string_map` @ `grafana_utils/alert_cli.py:571`（上游 2 / 下游 0）
  - 被呼叫: diff_alerting_resources, import_alerting_resources
- `load_panel_id_map` @ `grafana_utils/alert_cli.py:576`（上游 2 / 下游 0）
  - 被呼叫: diff_alerting_resources, import_alerting_resources
- `render_compare_json` @ `grafana_utils/alert_cli.py:581`（上游 1 / 下游 0）
  - 被呼叫: print_unified_diff
- `print_unified_diff` @ `grafana_utils/alert_cli.py:591`（上游 1 / 下游 1）

  - 呼叫: render_compare_json
  - 被呼叫: diff_alerting_resources
- `load_json_file` @ `grafana_utils/alert_cli.py:615`（上游 2 / 下游 0）
  - 被呼叫: diff_alerting_resources, import_alerting_resources
- `build_resource_dirs` @ `grafana_utils/alert_cli.py:625`（上游 1 / 下游 0）
  - 被呼叫: export_alerting_resources
- `build_rule_output_path` @ `grafana_utils/alert_cli.py:632`（上游 1 / 下游 1）

  - 呼叫: sanitize_path_component
  - 被呼叫: export_rule_documents
- `build_contact_point_output_path` @ `grafana_utils/alert_cli.py:644`（上游 1 / 下游 1）

  - 呼叫: sanitize_path_component
  - 被呼叫: export_contact_point_documents
- `build_mute_timing_output_path` @ `grafana_utils/alert_cli.py:656`（上游 1 / 下游 1）

  - 呼叫: sanitize_path_component
  - 被呼叫: export_mute_timing_documents
- `build_policies_output_path` @ `grafana_utils/alert_cli.py:667`（上游 1 / 下游 0）
  - 被呼叫: export_policies_document
- `build_template_output_path` @ `grafana_utils/alert_cli.py:672`（上游 1 / 下游 1）

  - 呼叫: sanitize_path_component
  - 被呼叫: export_template_documents
- `discover_alert_resource_files` @ `grafana_utils/alert_cli.py:683`（上游 2 / 下游 0）
  - 被呼叫: diff_alerting_resources, import_alerting_resources
- `build_alert_list_table` @ `grafana_utils/alert_cli.py:711`（上游 1 / 下游 1）

  - 呼叫: build_row
  - 被呼叫: list_alert_resources
- `build_alert_list_table.build_row` @ `grafana_utils/alert_cli.py:724`（上游 1 / 下游 0）
  - 被呼叫: build_alert_list_table
- `render_alert_list_csv` @ `grafana_utils/alert_cli.py:742`（上游 1 / 下游 0）
  - 被呼叫: list_alert_resources
- `render_alert_list_json` @ `grafana_utils/alert_cli.py:750`（上游 1 / 下游 0）
  - 被呼叫: list_alert_resources
- `serialize_rule_list_rows` @ `grafana_utils/alert_cli.py:755`（上游 0 / 下游 0）
- `serialize_contact_point_list_rows` @ `grafana_utils/alert_cli.py:774`（上游 0 / 下游 0）
- `serialize_mute_timing_list_rows` @ `grafana_utils/alert_cli.py:794`（上游 0 / 下游 0）
- `serialize_template_list_rows` @ `grafana_utils/alert_cli.py:814`（上游 0 / 下游 0）
- `export_rule_documents` @ `grafana_utils/alert_cli.py:825`（上游 1 / 下游 2）

  - 呼叫: build_rule_output_path, write_json
  - 被呼叫: export_alerting_resources
- `export_contact_point_documents` @ `grafana_utils/alert_cli.py:855`（上游 1 / 下游 2）

  - 呼叫: build_contact_point_output_path, write_json
  - 被呼叫: export_alerting_resources
- `export_mute_timing_documents` @ `grafana_utils/alert_cli.py:883`（上游 1 / 下游 2）

  - 呼叫: build_mute_timing_output_path, write_json
  - 被呼叫: export_alerting_resources
- `export_policies_document` @ `grafana_utils/alert_cli.py:907`（上游 1 / 下游 2）

  - 呼叫: build_policies_output_path, write_json
  - 被呼叫: export_alerting_resources
- `export_template_documents` @ `grafana_utils/alert_cli.py:929`（上游 1 / 下游 2）

  - 呼叫: build_template_output_path, write_json
  - 被呼叫: export_alerting_resources
- `write_resource_indexes` @ `grafana_utils/alert_cli.py:953`（上游 1 / 下游 1）

  - 呼叫: write_json
  - 被呼叫: export_alerting_resources
- `format_export_summary` @ `grafana_utils/alert_cli.py:966`（上游 1 / 下游 0）
  - 被呼叫: export_alerting_resources
- `export_alerting_resources` @ `grafana_utils/alert_cli.py:982`（上游 1 / 下游 10）

  - 呼叫: build_client, build_resource_dirs, export_contact_point_documents, export_mute_timing_documents, export_policies_document, export_rule_documents, export_template_documents, format_export_summary, write_json, write_resource_indexes
  - 被呼叫: main
- `count_policy_documents` @ `grafana_utils/alert_cli.py:1048`（上游 2 / 下游 0）
  - 被呼叫: diff_alerting_resources, import_alerting_resources
- `import_rule_document` @ `grafana_utils/alert_cli.py:1062`（上游 1 / 下游 0）
  - 被呼叫: import_resource_document
- `import_contact_point_document` @ `grafana_utils/alert_cli.py:1083`（上游 1 / 下游 0）
  - 被呼叫: import_resource_document
- `import_mute_timing_document` @ `grafana_utils/alert_cli.py:1100`（上游 1 / 下游 0）
  - 被呼叫: import_resource_document
- `build_template_update_payload` @ `grafana_utils/alert_cli.py:1117`（上游 1 / 下游 0）
  - 被呼叫: import_template_document
- `import_template_document` @ `grafana_utils/alert_cli.py:1140`（上游 1 / 下游 1）

  - 呼叫: build_template_update_payload
  - 被呼叫: import_resource_document
- `import_policies_document` @ `grafana_utils/alert_cli.py:1156`（上游 1 / 下游 0）
  - 被呼叫: import_resource_document
- `import_resource_document` @ `grafana_utils/alert_cli.py:1165`（上游 1 / 下游 5）

  - 呼叫: import_contact_point_document, import_mute_timing_document, import_policies_document, import_rule_document, import_template_document
  - 被呼叫: import_alerting_resources
- `import_alerting_resources` @ `grafana_utils/alert_cli.py:1187`（上游 1 / 下游 7）

  - 呼叫: build_client, count_policy_documents, discover_alert_resource_files, import_resource_document, load_json_file, load_panel_id_map, load_string_map
  - 被呼叫: main
- `diff_alerting_resources` @ `grafana_utils/alert_cli.py:1234`（上游 1 / 下游 7）

  - 呼叫: build_client, count_policy_documents, discover_alert_resource_files, load_json_file, load_panel_id_map, load_string_map, print_unified_diff
  - 被呼叫: main
- `build_client` @ `grafana_utils/alert_cli.py:1300`（上游 4 / 下游 2）

  - 呼叫: _build_alert_client_for_headers, resolve_auth
  - 被呼叫: diff_alerting_resources, export_alerting_resources, import_alerting_resources, list_visible_orgs
- `_build_alert_client_for_headers` @ `grafana_utils/alert_cli.py:1306`（上游 3 / 下游 0）
  - 被呼叫: _build_alert_list_plan, build_client, build_client_for_org
- `build_client_for_org` @ `grafana_utils/alert_cli.py:1319`（上游 1 / 下游 2）

  - 呼叫: _build_alert_client_for_headers, resolve_auth
  - 被呼叫: _build_alert_list_plan
- `list_visible_orgs` @ `grafana_utils/alert_cli.py:1329`（上游 1 / 下游 1）

  - 呼叫: build_client
  - 被呼叫: _build_alert_list_plan
- `fetch_current_org` @ `grafana_utils/alert_cli.py:1338`（上游 1 / 下游 0）
  - 被呼叫: _build_alert_list_plan
- `_normalize_alert_org_id` @ `grafana_utils/alert_cli.py:1346`（上游 1 / 下游 0）
  - 被呼叫: _build_alert_list_plan
- `_build_alert_list_plan` @ `grafana_utils/alert_cli.py:1354`（上游 1 / 下游 6）

  - 呼叫: _build_alert_client_for_headers, _normalize_alert_org_id, build_client_for_org, fetch_current_org, list_visible_orgs, resolve_auth
  - 被呼叫: list_alert_resources
- `_attach_alert_org_scope` @ `grafana_utils/alert_cli.py:1387`（上游 1 / 下游 0）
  - 被呼叫: list_alert_resources
- `_expand_alert_list_fields_with_org_scope` @ `grafana_utils/alert_cli.py:1403`（上游 1 / 下游 0）
  - 被呼叫: list_alert_resources
- `list_alert_resources` @ `grafana_utils/alert_cli.py:1416`（上游 1 / 下游 6）

  - 呼叫: _attach_alert_org_scope, _build_alert_list_plan, _expand_alert_list_fields_with_org_scope, build_alert_list_table, render_alert_list_csv, render_alert_list_json
  - 被呼叫: main
- `main` @ `grafana_utils/alert_cli.py:1475`（上游 0 / 下游 5）

  - 呼叫: diff_alerting_resources, export_alerting_resources, import_alerting_resources, list_alert_resources, parse_args

## `grafana_utils/alert_sync_workbench.py`

- `_normalize_text` @ `grafana_utils/alert_sync_workbench.py:42`（上游 3 / 下游 0）
  - 被呼叫: _normalize_managed_fields, assess_alert_sync_specs, render_alert_sync_assessment_text
- `_require_mapping` @ `grafana_utils/alert_sync_workbench.py:52`（上游 2 / 下游 0）
  - 被呼叫: assess_alert_sync_specs, render_alert_sync_assessment_text
- `_normalize_managed_fields` @ `grafana_utils/alert_sync_workbench.py:59`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: assess_alert_sync_specs
- `assess_alert_sync_specs` @ `grafana_utils/alert_sync_workbench.py:77`（上游 0 / 下游 3）

  - 呼叫: _normalize_managed_fields, _normalize_text, _require_mapping
- `render_alert_sync_assessment_text` @ `grafana_utils/alert_sync_workbench.py:167`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, _require_mapping

## `grafana_utils/alerts/__init__.py`

- 無可辨識函式。

## `grafana_utils/alerts/common.py`

- `GrafanaApiError.__init__` @ `grafana_utils/alerts/common.py:44`（上游 0 / 下游 0）

## `grafana_utils/alerts/provisioning.py`

- `strip_server_managed_fields` @ `grafana_utils/alerts/provisioning.py:32`（上游 11 / 下游 0）
  - 被呼叫: build_contact_point_export_document, build_contact_point_import_payload, build_mute_timing_export_document, build_mute_timing_import_payload, build_policies_export_document, build_policies_import_payload, build_rule_export_document, build_rule_import_payload, build_template_export_document, build_template_import_payload ...
- `get_rule_linkage` @ `grafana_utils/alerts/provisioning.py:40`（上游 2 / 下游 0）
  - 被呼叫: apply_rule_linkage_maps, build_linked_dashboard_metadata
- `find_panel_by_id` @ `grafana_utils/alerts/provisioning.py:59`（上游 1 / 下游 0）
  - 被呼叫: build_linked_dashboard_metadata
- `derive_dashboard_slug` @ `grafana_utils/alerts/provisioning.py:75`（上游 3 / 下游 0）
  - 被呼叫: build_linked_dashboard_metadata, filter_dashboard_search_matches, resolve_dashboard_uid_fallback
- `build_linked_dashboard_metadata` @ `grafana_utils/alerts/provisioning.py:84`（上游 0 / 下游 3）

  - 呼叫: derive_dashboard_slug, find_panel_by_id, get_rule_linkage
- `filter_dashboard_search_matches` @ `grafana_utils/alerts/provisioning.py:125`（上游 1 / 下游 1）

  - 呼叫: derive_dashboard_slug
  - 被呼叫: resolve_dashboard_uid_fallback
- `resolve_dashboard_uid_fallback` @ `grafana_utils/alerts/provisioning.py:156`（上游 1 / 下游 2）

  - 呼叫: derive_dashboard_slug, filter_dashboard_search_matches
  - 被呼叫: rewrite_rule_dashboard_linkage
- `load_string_map` @ `grafana_utils/alerts/provisioning.py:191`（上游 0 / 下游 0）
- `load_panel_id_map` @ `grafana_utils/alerts/provisioning.py:212`（上游 0 / 下游 0）
- `apply_rule_linkage_maps` @ `grafana_utils/alerts/provisioning.py:239`（上游 1 / 下游 1）

  - 呼叫: get_rule_linkage
  - 被呼叫: rewrite_rule_dashboard_linkage
- `extract_linked_dashboard_metadata` @ `grafana_utils/alerts/provisioning.py:265`（上游 1 / 下游 0）
  - 被呼叫: rewrite_rule_dashboard_linkage
- `rewrite_rule_dashboard_linkage` @ `grafana_utils/alerts/provisioning.py:281`（上游 1 / 下游 3）

  - 呼叫: apply_rule_linkage_maps, extract_linked_dashboard_metadata, resolve_dashboard_uid_fallback
  - 被呼叫: prepare_rule_payload_for_target
- `build_rule_metadata` @ `grafana_utils/alerts/provisioning.py:315`（上游 0 / 下游 0）
- `build_contact_point_metadata` @ `grafana_utils/alerts/provisioning.py:335`（上游 0 / 下游 0）
- `build_mute_timing_metadata` @ `grafana_utils/alerts/provisioning.py:348`（上游 0 / 下游 0）
- `build_policies_metadata` @ `grafana_utils/alerts/provisioning.py:357`（上游 0 / 下游 0）
- `build_template_metadata` @ `grafana_utils/alerts/provisioning.py:366`（上游 0 / 下游 0）
- `build_tool_document` @ `grafana_utils/alerts/provisioning.py:375`（上游 5 / 下游 0）
  - 被呼叫: build_contact_point_export_document, build_mute_timing_export_document, build_policies_export_document, build_rule_export_document, build_template_export_document
- `build_rule_export_document` @ `grafana_utils/alerts/provisioning.py:394`（上游 0 / 下游 2）

  - 呼叫: build_tool_document, strip_server_managed_fields
- `build_contact_point_export_document` @ `grafana_utils/alerts/provisioning.py:412`（上游 0 / 下游 2）

  - 呼叫: build_tool_document, strip_server_managed_fields
- `build_mute_timing_export_document` @ `grafana_utils/alerts/provisioning.py:426`（上游 0 / 下游 2）

  - 呼叫: build_tool_document, strip_server_managed_fields
- `build_policies_export_document` @ `grafana_utils/alerts/provisioning.py:440`（上游 0 / 下游 2）

  - 呼叫: build_tool_document, strip_server_managed_fields
- `build_template_export_document` @ `grafana_utils/alerts/provisioning.py:454`（上游 0 / 下游 2）

  - 呼叫: build_tool_document, strip_server_managed_fields
- `reject_provisioning_export` @ `grafana_utils/alerts/provisioning.py:468`（上游 5 / 下游 0）
  - 被呼叫: build_contact_point_import_payload, build_mute_timing_import_payload, build_policies_import_payload, build_rule_import_payload, build_template_import_payload
- `detect_document_kind` @ `grafana_utils/alerts/provisioning.py:482`（上游 1 / 下游 0）
  - 被呼叫: build_import_operation
- `extract_tool_spec` @ `grafana_utils/alerts/provisioning.py:500`（上游 5 / 下游 0）
  - 被呼叫: build_contact_point_import_payload, build_mute_timing_import_payload, build_policies_import_payload, build_rule_import_payload, build_template_import_payload
- `build_rule_import_payload` @ `grafana_utils/alerts/provisioning.py:521`（上游 0 / 下游 3）

  - 呼叫: extract_tool_spec, reject_provisioning_export, strip_server_managed_fields
- `build_contact_point_import_payload` @ `grafana_utils/alerts/provisioning.py:543`（上游 0 / 下游 3）

  - 呼叫: extract_tool_spec, reject_provisioning_export, strip_server_managed_fields
- `build_mute_timing_import_payload` @ `grafana_utils/alerts/provisioning.py:565`（上游 0 / 下游 3）

  - 呼叫: extract_tool_spec, reject_provisioning_export, strip_server_managed_fields
- `build_policies_import_payload` @ `grafana_utils/alerts/provisioning.py:587`（上游 0 / 下游 3）

  - 呼叫: extract_tool_spec, reject_provisioning_export, strip_server_managed_fields
- `build_template_import_payload` @ `grafana_utils/alerts/provisioning.py:602`（上游 0 / 下游 3）

  - 呼叫: extract_tool_spec, reject_provisioning_export, strip_server_managed_fields
- `build_import_operation` @ `grafana_utils/alerts/provisioning.py:622`（上游 0 / 下游 1）

  - 呼叫: detect_document_kind
- `prepare_rule_payload_for_target` @ `grafana_utils/alerts/provisioning.py:641`（上游 1 / 下游 1）

  - 呼叫: rewrite_rule_dashboard_linkage
  - 被呼叫: prepare_import_payload_for_target
- `prepare_import_payload_for_target` @ `grafana_utils/alerts/provisioning.py:658`（上游 0 / 下游 1）

  - 呼叫: prepare_rule_payload_for_target
- `build_compare_document` @ `grafana_utils/alerts/provisioning.py:682`（上游 1 / 下游 0）
  - 被呼叫: fetch_live_compare_document
- `serialize_compare_document` @ `grafana_utils/alerts/provisioning.py:687`（上游 0 / 下游 0）
- `build_resource_identity` @ `grafana_utils/alerts/provisioning.py:696`（上游 0 / 下游 0）
- `build_diff_label` @ `grafana_utils/alerts/provisioning.py:713`（上游 0 / 下游 0）
- `determine_rule_import_action` @ `grafana_utils/alerts/provisioning.py:722`（上游 1 / 下游 0）
  - 被呼叫: determine_import_action
- `determine_contact_point_import_action` @ `grafana_utils/alerts/provisioning.py:742`（上游 1 / 下游 0）
  - 被呼叫: determine_import_action
- `determine_mute_timing_import_action` @ `grafana_utils/alerts/provisioning.py:757`（上游 1 / 下游 0）
  - 被呼叫: determine_import_action
- `determine_template_import_action` @ `grafana_utils/alerts/provisioning.py:772`（上游 1 / 下游 0）
  - 被呼叫: determine_import_action
- `determine_import_action` @ `grafana_utils/alerts/provisioning.py:787`（上游 0 / 下游 4）

  - 呼叫: determine_contact_point_import_action, determine_mute_timing_import_action, determine_rule_import_action, determine_template_import_action
- `fetch_live_compare_document` @ `grafana_utils/alerts/provisioning.py:809`（上游 0 / 下游 2）

  - 呼叫: build_compare_document, strip_server_managed_fields
- `build_empty_root_index` @ `grafana_utils/alerts/provisioning.py:879`（上游 0 / 下游 0）

## `grafana_utils/auth_staging.py`

- `format_cli_auth_error_message` @ `grafana_utils/auth_staging.py:17`（上游 1 / 下游 0）
  - 被呼叫: resolve_cli_auth_from_namespace
- `_first_present` @ `grafana_utils/auth_staging.py:67`（上游 1 / 下游 0）
  - 被呼叫: resolve_auth_from_namespace
- `_env_value` @ `grafana_utils/auth_staging.py:76`（上游 1 / 下游 0）
  - 被呼叫: resolve_auth_headers
- `_encode_basic_auth` @ `grafana_utils/auth_staging.py:87`（上游 1 / 下游 0）
  - 被呼叫: resolve_auth_headers
- `add_org_id_header` @ `grafana_utils/auth_staging.py:95`（上游 1 / 下游 0）
  - 被呼叫: resolve_auth_from_namespace
- `resolve_auth_headers` @ `grafana_utils/auth_staging.py:107`（上游 1 / 下游 2）

  - 呼叫: _encode_basic_auth, _env_value
  - 被呼叫: resolve_auth_from_namespace
- `resolve_auth_from_namespace` @ `grafana_utils/auth_staging.py:180`（上游 1 / 下游 3）

  - 呼叫: _first_present, add_org_id_header, resolve_auth_headers
  - 被呼叫: resolve_cli_auth_from_namespace
- `resolve_cli_auth_from_namespace` @ `grafana_utils/auth_staging.py:219`（上游 0 / 下游 2）

  - 呼叫: format_cli_auth_error_message, resolve_auth_from_namespace

## `grafana_utils/bundle_preflight_workbench.py`

- `_normalize_text` @ `grafana_utils/bundle_preflight_workbench.py:31`（上游 4 / 下游 0）
  - 被呼叫: _build_provider_assessment, _build_secret_assessment, _require_string_list, render_bundle_preflight_text
- `_require_mapping` @ `grafana_utils/bundle_preflight_workbench.py:41`（上游 2 / 下游 0）
  - 被呼叫: build_bundle_preflight_document, render_bundle_preflight_text
- `_require_string_list` @ `grafana_utils/bundle_preflight_workbench.py:50`（上游 2 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: _build_provider_assessment, _build_secret_assessment
- `_build_secret_assessment` @ `grafana_utils/bundle_preflight_workbench.py:64`（上游 1 / 下游 2）

  - 呼叫: _normalize_text, _require_string_list
  - 被呼叫: build_bundle_preflight_document
- `_build_provider_assessment` @ `grafana_utils/bundle_preflight_workbench.py:123`（上游 1 / 下游 2）

  - 呼叫: _normalize_text, _require_string_list
  - 被呼叫: build_bundle_preflight_document
- `build_bundle_preflight_document` @ `grafana_utils/bundle_preflight_workbench.py:181`（上游 0 / 下游 3）

  - 呼叫: _build_provider_assessment, _build_secret_assessment, _require_mapping
- `render_bundle_preflight_text` @ `grafana_utils/bundle_preflight_workbench.py:278`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, _require_mapping

## `grafana_utils/clients/__init__.py`

- 無可辨識函式。

## `grafana_utils/clients/access_client.py`

- `GrafanaAccessClient.__init__` @ `grafana_utils/clients/access_client.py:18`（上游 0 / 下游 0）
- `GrafanaAccessClient.request_json` @ `grafana_utils/clients/access_client.py:43`（上游 36 / 下游 0）
  - 被呼叫: add_team_member, add_user_to_organization, create_organization, create_service_account, create_service_account_token, create_team, create_user, delete_global_user, delete_org_user, delete_organization ...
- `GrafanaAccessClient.list_org_users` @ `grafana_utils/clients/access_client.py:66`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.list_organizations` @ `grafana_utils/clients/access_client.py:80`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.get_organization` @ `grafana_utils/clients/access_client.py:94`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.create_organization` @ `grafana_utils/clients/access_client.py:113`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_organization` @ `grafana_utils/clients/access_client.py:131`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_organization` @ `grafana_utils/clients/access_client.py:152`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.list_organization_users` @ `grafana_utils/clients/access_client.py:172`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.add_user_to_organization` @ `grafana_utils/clients/access_client.py:191`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_organization_user_role` @ `grafana_utils/clients/access_client.py:215`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_organization_user` @ `grafana_utils/clients/access_client.py:245`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.iter_global_users` @ `grafana_utils/clients/access_client.py:269`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.list_user_teams` @ `grafana_utils/clients/access_client.py:295`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.get_user` @ `grafana_utils/clients/access_client.py:313`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.create_user` @ `grafana_utils/clients/access_client.py:331`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_user` @ `grafana_utils/clients/access_client.py:349`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_user_password` @ `grafana_utils/clients/access_client.py:369`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_user_org_role` @ `grafana_utils/clients/access_client.py:390`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_user_permissions` @ `grafana_utils/clients/access_client.py:410`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_global_user` @ `grafana_utils/clients/access_client.py:435`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_org_user` @ `grafana_utils/clients/access_client.py:455`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.list_service_accounts` @ `grafana_utils/clients/access_client.py:474`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.list_teams` @ `grafana_utils/clients/access_client.py:507`（上游 1 / 下游 1）

  - 呼叫: request_json
  - 被呼叫: iter_teams
- `GrafanaAccessClient.iter_teams` @ `grafana_utils/clients/access_client.py:532`（上游 0 / 下游 1）

  - 呼叫: list_teams
- `GrafanaAccessClient.list_team_members` @ `grafana_utils/clients/access_client.py:561`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.get_team` @ `grafana_utils/clients/access_client.py:579`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_team` @ `grafana_utils/clients/access_client.py:597`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.create_team` @ `grafana_utils/clients/access_client.py:616`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.add_team_member` @ `grafana_utils/clients/access_client.py:634`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.remove_team_member` @ `grafana_utils/clients/access_client.py:654`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_team_members` @ `grafana_utils/clients/access_client.py:677`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.create_service_account` @ `grafana_utils/clients/access_client.py:698`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.get_service_account` @ `grafana_utils/clients/access_client.py:718`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_service_account` @ `grafana_utils/clients/access_client.py:737`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.update_service_account` @ `grafana_utils/clients/access_client.py:757`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.list_service_account_tokens` @ `grafana_utils/clients/access_client.py:782`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.create_service_account_token` @ `grafana_utils/clients/access_client.py:805`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAccessClient.delete_service_account_token` @ `grafana_utils/clients/access_client.py:830`（上游 0 / 下游 1）

  - 呼叫: request_json

## `grafana_utils/clients/alert_client.py`

- `GrafanaAlertClient.__init__` @ `grafana_utils/clients/alert_client.py:18`（上游 0 / 下游 0）
- `GrafanaAlertClient.request_json` @ `grafana_utils/clients/alert_client.py:41`（上游 17 / 下游 0）
  - 被呼叫: create_alert_rule, create_contact_point, create_mute_timing, get_alert_rule, get_dashboard, get_notification_policies, get_template, list_alert_rules, list_contact_points, list_mute_timings ...
- `GrafanaAlertClient.list_alert_rules` @ `grafana_utils/clients/alert_client.py:61`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.search_dashboards` @ `grafana_utils/clients/alert_client.py:75`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.get_dashboard` @ `grafana_utils/clients/alert_client.py:92`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.get_alert_rule` @ `grafana_utils/clients/alert_client.py:106`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.create_alert_rule` @ `grafana_utils/clients/alert_client.py:122`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.update_alert_rule` @ `grafana_utils/clients/alert_client.py:140`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.list_contact_points` @ `grafana_utils/clients/alert_client.py:158`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.create_contact_point` @ `grafana_utils/clients/alert_client.py:172`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.update_contact_point` @ `grafana_utils/clients/alert_client.py:190`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.list_mute_timings` @ `grafana_utils/clients/alert_client.py:208`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.create_mute_timing` @ `grafana_utils/clients/alert_client.py:222`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.update_mute_timing` @ `grafana_utils/clients/alert_client.py:240`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.get_notification_policies` @ `grafana_utils/clients/alert_client.py:258`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.update_notification_policies` @ `grafana_utils/clients/alert_client.py:272`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.list_templates` @ `grafana_utils/clients/alert_client.py:292`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.get_template` @ `grafana_utils/clients/alert_client.py:308`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaAlertClient.update_template` @ `grafana_utils/clients/alert_client.py:324`（上游 0 / 下游 1）

  - 呼叫: request_json

## `grafana_utils/clients/dashboard_client.py`

- `GrafanaClient.__init__` @ `grafana_utils/clients/dashboard_client.py:18`（上游 0 / 下游 0）
- `GrafanaClient.request_json` @ `grafana_utils/clients/dashboard_client.py:45`（上游 9 / 下游 0）
  - 被呼叫: create_folder, create_organization, fetch_current_org, fetch_dashboard_if_exists, fetch_folder_if_exists, import_dashboard, iter_dashboard_summaries, list_datasources, list_orgs
- `GrafanaClient.iter_dashboard_summaries` @ `grafana_utils/clients/dashboard_client.py:65`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.fetch_folder_if_exists` @ `grafana_utils/clients/dashboard_client.py:98`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.create_folder` @ `grafana_utils/clients/dashboard_client.py:114`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.fetch_dashboard` @ `grafana_utils/clients/dashboard_client.py:137`（上游 0 / 下游 1）

  - 呼叫: fetch_dashboard_if_exists
- `GrafanaClient.fetch_dashboard_if_exists` @ `grafana_utils/clients/dashboard_client.py:154`（上游 1 / 下游 1）

  - 呼叫: request_json
  - 被呼叫: fetch_dashboard
- `GrafanaClient.import_dashboard` @ `grafana_utils/clients/dashboard_client.py:167`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.list_datasources` @ `grafana_utils/clients/dashboard_client.py:182`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.fetch_current_org` @ `grafana_utils/clients/dashboard_client.py:193`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.list_orgs` @ `grafana_utils/clients/dashboard_client.py:204`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.create_organization` @ `grafana_utils/clients/dashboard_client.py:215`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaClient.with_org_id` @ `grafana_utils/clients/dashboard_client.py:230`（上游 0 / 下游 0）

## `grafana_utils/clients/datasource_client.py`

- `GrafanaDatasourceClient.__init__` @ `grafana_utils/clients/datasource_client.py:28`（上游 0 / 下游 0）
- `GrafanaDatasourceClient.request_json` @ `grafana_utils/clients/datasource_client.py:55`（上游 4 / 下游 0）
  - 被呼叫: create_datasource, delete_datasource, fetch_datasource_by_uid_if_exists, list_datasources
- `GrafanaDatasourceClient.list_datasources` @ `grafana_utils/clients/datasource_client.py:78`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaDatasourceClient.fetch_datasource_by_uid_if_exists` @ `grafana_utils/clients/datasource_client.py:92`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaDatasourceClient.create_datasource` @ `grafana_utils/clients/datasource_client.py:113`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaDatasourceClient.delete_datasource` @ `grafana_utils/clients/datasource_client.py:131`（上游 0 / 下游 1）

  - 呼叫: request_json
- `GrafanaDatasourceClient.with_org_id` @ `grafana_utils/clients/datasource_client.py:151`（上游 0 / 下游 0）

## `grafana_utils/dashboard_cli.py`

- `HelpFullAction.__call__` @ `grafana_utils/dashboard_cli.py:231`（上游 0 / 下游 0）
- `add_common_cli_args` @ `grafana_utils/dashboard_cli.py:248`（上游 4 / 下游 0）
  - 被呼叫: add_inspect_live_cli_args, add_inspect_vars_cli_args, add_screenshot_cli_args, parse_args
- `add_export_cli_args` @ `grafana_utils/dashboard_cli.py:314`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `add_list_cli_args` @ `grafana_utils/dashboard_cli.py:382`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `add_list_data_sources_cli_args` @ `grafana_utils/dashboard_cli.py:448`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `add_import_cli_args` @ `grafana_utils/dashboard_cli.py:484`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `add_diff_cli_args` @ `grafana_utils/dashboard_cli.py:635`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `add_inspect_export_cli_args` @ `grafana_utils/dashboard_cli.py:661`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `add_inspect_live_cli_args` @ `grafana_utils/dashboard_cli.py:752`（上游 1 / 下游 1）

  - 呼叫: add_common_cli_args
  - 被呼叫: parse_args
- `add_inspect_vars_cli_args` @ `grafana_utils/dashboard_cli.py:842`（上游 1 / 下游 1）

  - 呼叫: add_common_cli_args
  - 被呼叫: parse_args
- `add_screenshot_cli_args` @ `grafana_utils/dashboard_cli.py:880`（上游 1 / 下游 1）

  - 呼叫: add_common_cli_args
  - 被呼叫: parse_args
- `parse_args` @ `grafana_utils/dashboard_cli.py:1020`（上游 1 / 下游 13）

  - 呼叫: _normalize_output_format_args, _parse_dashboard_import_output_columns, _validate_import_routing_args, add_common_cli_args, add_diff_cli_args, add_export_cli_args, add_import_cli_args, add_inspect_export_cli_args, add_inspect_live_cli_args, add_inspect_vars_cli_args ...
  - 被呼叫: main
- `_normalize_output_format_args` @ `grafana_utils/dashboard_cli.py:1224`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `_parse_dashboard_import_output_columns` @ `grafana_utils/dashboard_cli.py:1253`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `_validate_import_routing_args` @ `grafana_utils/dashboard_cli.py:1273`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `resolve_auth` @ `grafana_utils/dashboard_cli.py:1293`（上游 1 / 下游 0）
  - 被呼叫: build_client
- `_build_export_workflow_deps` @ `grafana_utils/dashboard_cli.py:1305`（上游 1 / 下游 0）
  - 被呼叫: export_dashboards
- `export_dashboards` @ `grafana_utils/dashboard_cli.py:1329`（上游 1 / 下游 1）

  - 呼叫: _build_export_workflow_deps
  - 被呼叫: main
- `list_dashboards` @ `grafana_utils/dashboard_cli.py:1334`（上游 1 / 下游 0）
  - 被呼叫: main
- `list_data_sources` @ `grafana_utils/dashboard_cli.py:1344`（上游 1 / 下游 0）
  - 被呼叫: main
- `_build_inspection_workflow_deps` @ `grafana_utils/dashboard_cli.py:1349`（上游 2 / 下游 0）
  - 被呼叫: inspect_export, inspect_live
- `inspect_live` @ `grafana_utils/dashboard_cli.py:1371`（上游 1 / 下游 1）

  - 呼叫: _build_inspection_workflow_deps
  - 被呼叫: main
- `inspect_export` @ `grafana_utils/dashboard_cli.py:1375`（上游 1 / 下游 1）

  - 呼叫: _build_inspection_workflow_deps
  - 被呼叫: main
- `inspect_vars` @ `grafana_utils/dashboard_cli.py:1380`（上游 1 / 下游 1）

  - 呼叫: build_client
  - 被呼叫: main
- `screenshot_dashboard` @ `grafana_utils/dashboard_cli.py:1401`（上游 1 / 下游 1）

  - 呼叫: build_client
  - 被呼叫: main
- `_build_import_workflow_deps` @ `grafana_utils/dashboard_cli.py:1410`（上游 1 / 下游 0）
  - 被呼叫: import_dashboards
- `import_dashboards` @ `grafana_utils/dashboard_cli.py:1428`（上游 1 / 下游 1）

  - 呼叫: _build_import_workflow_deps
  - 被呼叫: main
- `_build_diff_workflow_deps` @ `grafana_utils/dashboard_cli.py:1433`（上游 1 / 下游 0）
  - 被呼叫: diff_dashboards
- `diff_dashboards` @ `grafana_utils/dashboard_cli.py:1466`（上游 1 / 下游 1）

  - 呼叫: _build_diff_workflow_deps
  - 被呼叫: main
- `build_client` @ `grafana_utils/dashboard_cli.py:1471`（上游 2 / 下游 1）

  - 呼叫: resolve_auth
  - 被呼叫: inspect_vars, screenshot_dashboard
- `main` @ `grafana_utils/dashboard_cli.py:1482`（上游 0 / 下游 10）

  - 呼叫: diff_dashboards, export_dashboards, import_dashboards, inspect_export, inspect_live, inspect_vars, list_dashboards, list_data_sources, parse_args, screenshot_dashboard

## `grafana_utils/dashboard_governance_gate.py`

- `_load_json_document` @ `grafana_utils/dashboard_governance_gate.py:39`（上游 1 / 下游 0）
  - 被呼叫: run_dashboard_governance_gate
- `_normalize_string_set` @ `grafana_utils/dashboard_governance_gate.py:52`（上游 2 / 下游 0）
  - 被呼叫: _build_dashboard_context_from_governance_document, evaluate_dashboard_governance_policy
- `_normalize_bool` @ `grafana_utils/dashboard_governance_gate.py:62`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `_normalize_optional_int` @ `grafana_utils/dashboard_governance_gate.py:76`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `_dashboard_key` @ `grafana_utils/dashboard_governance_gate.py:83`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `_panel_key` @ `grafana_utils/dashboard_governance_gate.py:91`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `_build_finding` @ `grafana_utils/dashboard_governance_gate.py:101`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `_query_family` @ `grafana_utils/dashboard_governance_gate.py:140`（上游 3 / 下游 0）
  - 被呼叫: _is_loki_broad_query, _is_sql_query, evaluate_dashboard_governance_policy
- `_query_text` @ `grafana_utils/dashboard_governance_gate.py:145`（上游 4 / 下游 0）
  - 被呼叫: _is_loki_broad_query, _query_uses_time_filter, _score_query_complexity, evaluate_dashboard_governance_policy
- `_is_sql_query` @ `grafana_utils/dashboard_governance_gate.py:150`（上游 1 / 下游 1）

  - 呼叫: _query_family
  - 被呼叫: evaluate_dashboard_governance_policy
- `_query_uses_time_filter` @ `grafana_utils/dashboard_governance_gate.py:155`（上游 1 / 下游 1）

  - 呼叫: _query_text
  - 被呼叫: evaluate_dashboard_governance_policy
- `_is_loki_broad_query` @ `grafana_utils/dashboard_governance_gate.py:161`（上游 1 / 下游 2）

  - 呼叫: _query_family, _query_text
  - 被呼叫: evaluate_dashboard_governance_policy
- `_governance_risk_kinds` @ `grafana_utils/dashboard_governance_gate.py:169`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `_extract_datasource_variable_name` @ `grafana_utils/dashboard_governance_gate.py:181`（上游 1 / 下游 0）
  - 被呼叫: _build_dashboard_context
- `_score_query_complexity` @ `grafana_utils/dashboard_governance_gate.py:192`（上游 1 / 下游 1）

  - 呼叫: _query_text
  - 被呼叫: evaluate_dashboard_governance_policy
- `_build_dashboard_context` @ `grafana_utils/dashboard_governance_gate.py:215`（上游 1 / 下游 1）

  - 呼叫: _extract_datasource_variable_name
  - 被呼叫: run_dashboard_governance_gate
- `_build_dashboard_context_from_governance_document` @ `grafana_utils/dashboard_governance_gate.py:310`（上游 1 / 下游 1）

  - 呼叫: _normalize_string_set
  - 被呼叫: evaluate_dashboard_governance_policy
- `_merge_dashboard_context` @ `grafana_utils/dashboard_governance_gate.py:339`（上游 1 / 下游 0）
  - 被呼叫: evaluate_dashboard_governance_policy
- `evaluate_dashboard_governance_policy` @ `grafana_utils/dashboard_governance_gate.py:386`（上游 1 / 下游 15）

  - 呼叫: _build_dashboard_context_from_governance_document, _build_finding, _dashboard_key, _governance_risk_kinds, _is_loki_broad_query, _is_sql_query, _merge_dashboard_context, _normalize_bool, _normalize_optional_int, _normalize_string_set ...
  - 被呼叫: run_dashboard_governance_gate
- `render_dashboard_governance_check` @ `grafana_utils/dashboard_governance_gate.py:767`（上游 1 / 下游 0）
  - 被呼叫: run_dashboard_governance_gate
- `build_parser` @ `grafana_utils/dashboard_governance_gate.py:811`（上游 1 / 下游 0）
  - 被呼叫: main
- `run_dashboard_governance_gate` @ `grafana_utils/dashboard_governance_gate.py:851`（上游 1 / 下游 4）

  - 呼叫: _build_dashboard_context, _load_json_document, evaluate_dashboard_governance_policy, render_dashboard_governance_check
  - 被呼叫: main
- `main` @ `grafana_utils/dashboard_governance_gate.py:882`（上游 0 / 下游 2）

  - 呼叫: build_parser, run_dashboard_governance_gate

## `grafana_utils/dashboard_permission_workbench.py`

- `_normalize_text` @ `grafana_utils/dashboard_permission_workbench.py:37`（上游 12 / 下游 0）
  - 被呼叫: build_permission_bundle_diff_document, build_permission_bundle_document, build_permission_export_document, build_permission_preflight_document, build_permission_remap_document, normalize_permission_level, normalize_permission_record, normalize_permission_subject, render_permission_bundle_text, render_permission_export_text ...
- `normalize_permission_level` @ `grafana_utils/dashboard_permission_workbench.py:45`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: normalize_permission_record
- `normalize_permission_subject` @ `grafana_utils/dashboard_permission_workbench.py:61`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: normalize_permission_record
- `normalize_permission_record` @ `grafana_utils/dashboard_permission_workbench.py:111`（上游 1 / 下游 3）

  - 呼叫: _normalize_text, normalize_permission_level, normalize_permission_subject
  - 被呼叫: build_permission_export_document
- `build_permission_export_document` @ `grafana_utils/dashboard_permission_workbench.py:140`（上游 1 / 下游 2）

  - 呼叫: _normalize_text, normalize_permission_record
  - 被呼叫: build_permission_bundle_document
- `build_permission_diff_document` @ `grafana_utils/dashboard_permission_workbench.py:182`（上游 2 / 下游 0）
  - 被呼叫: build_permission_bundle_diff_document, build_permission_promotion_document
- `render_permission_export_text` @ `grafana_utils/dashboard_permission_workbench.py:249`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `build_permission_preflight_document` @ `grafana_utils/dashboard_permission_workbench.py:290`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `build_permission_promotion_document` @ `grafana_utils/dashboard_permission_workbench.py:350`（上游 0 / 下游 1）

  - 呼叫: build_permission_diff_document
- `render_permission_preflight_text` @ `grafana_utils/dashboard_permission_workbench.py:381`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `build_permission_bundle_document` @ `grafana_utils/dashboard_permission_workbench.py:415`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, build_permission_export_document
- `build_permission_bundle_diff_document` @ `grafana_utils/dashboard_permission_workbench.py:461`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, build_permission_diff_document
- `render_permission_bundle_text` @ `grafana_utils/dashboard_permission_workbench.py:544`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `build_permission_remap_document` @ `grafana_utils/dashboard_permission_workbench.py:580`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `render_permission_remap_text` @ `grafana_utils/dashboard_permission_workbench.py:636`（上游 0 / 下游 1）

  - 呼叫: _normalize_text

## `grafana_utils/dashboards/__init__.py`

- 無可辨識函式。

## `grafana_utils/dashboards/common.py`

- `GrafanaApiError.__init__` @ `grafana_utils/dashboards/common.py:45`（上游 0 / 下游 0）

## `grafana_utils/dashboards/diff_workflow.py`

- `run_diff_dashboards` @ `grafana_utils/dashboards/diff_workflow.py:8`（上游 0 / 下游 0）

## `grafana_utils/dashboards/export_inventory.py`

- `discover_dashboard_files` @ `grafana_utils/dashboards/export_inventory.py:10`（上游 0 / 下游 0）
- `discover_org_raw_export_dirs` @ `grafana_utils/dashboards/export_inventory.py:51`（上游 0 / 下游 0）
- `load_folder_inventory` @ `grafana_utils/dashboards/export_inventory.py:77`（上游 0 / 下游 0）
- `load_datasource_inventory` @ `grafana_utils/dashboards/export_inventory.py:118`（上游 0 / 下游 0）
- `build_folder_inventory_lookup` @ `grafana_utils/dashboards/export_inventory.py:165`（上游 0 / 下游 0）
- `build_import_dashboard_folder_path` @ `grafana_utils/dashboards/export_inventory.py:181`（上游 1 / 下游 0）
  - 被呼叫: resolve_folder_inventory_record_for_dashboard
- `resolve_folder_inventory_record_for_dashboard` @ `grafana_utils/dashboards/export_inventory.py:188`（上游 0 / 下游 2）

  - 呼叫: build_import_dashboard_folder_path, build_general_record
- `resolve_folder_inventory_record_for_dashboard.build_general_record` @ `grafana_utils/dashboards/export_inventory.py:201`（上游 1 / 下游 0）
  - 被呼叫: resolve_folder_inventory_record_for_dashboard
- `validate_export_metadata` @ `grafana_utils/dashboards/export_inventory.py:240`（上游 1 / 下游 0）
  - 被呼叫: load_export_metadata
- `load_export_metadata` @ `grafana_utils/dashboards/export_inventory.py:269`（上游 0 / 下游 1）

  - 呼叫: validate_export_metadata
- `resolve_export_org_id` @ `grafana_utils/dashboards/export_inventory.py:304`（上游 0 / 下游 1）

  - 呼叫: _resolve_export_identity_field
- `resolve_export_org_name` @ `grafana_utils/dashboards/export_inventory.py:324`（上游 0 / 下游 1）

  - 呼叫: _resolve_export_identity_field
- `_resolve_export_identity_field` @ `grafana_utils/dashboards/export_inventory.py:344`（上游 2 / 下游 0）
  - 被呼叫: resolve_export_org_id, resolve_export_org_name

## `grafana_utils/dashboards/export_runtime.py`

- `build_export_workflow_deps` @ `grafana_utils/dashboards/export_runtime.py:27`（上游 0 / 下游 0）

## `grafana_utils/dashboards/export_workflow.py`

- `run_export_dashboards` @ `grafana_utils/dashboards/export_workflow.py:6`（上游 0 / 下游 0）

## `grafana_utils/dashboards/folder_path_match.py`

- `normalize_folder_path` @ `grafana_utils/dashboards/folder_path_match.py:22`（上游 2 / 下游 0）
  - 被呼叫: build_folder_path_match_result, resolve_source_dashboard_folder_path
- `resolve_source_dashboard_folder_path` @ `grafana_utils/dashboards/folder_path_match.py:33`（上游 0 / 下游 1）

  - 呼叫: normalize_folder_path
- `resolve_existing_dashboard_folder_path` @ `grafana_utils/dashboards/folder_path_match.py:63`（上游 0 / 下游 0）
- `build_folder_path_match_result` @ `grafana_utils/dashboards/folder_path_match.py:103`（上游 0 / 下游 1）

  - 呼叫: normalize_folder_path
- `apply_folder_path_guard_to_action` @ `grafana_utils/dashboards/folder_path_match.py:168`（上游 0 / 下游 0）

## `grafana_utils/dashboards/folder_support.py`

- `build_folder_inventory_record` @ `grafana_utils/dashboards/folder_support.py:23`（上游 1 / 下游 0）
  - 被呼叫: collect_folder_inventory
- `collect_folder_inventory` @ `grafana_utils/dashboards/folder_support.py:47`（上游 0 / 下游 1）

  - 呼叫: build_folder_inventory_record
- `load_folder_inventory` @ `grafana_utils/dashboards/folder_support.py:89`（上游 1 / 下游 0）
  - 被呼叫: resolve_folder_inventory_requirements
- `load_datasource_inventory` @ `grafana_utils/dashboards/folder_support.py:102`（上游 0 / 下游 0）
- `ensure_folder_inventory` @ `grafana_utils/dashboards/folder_support.py:119`（上游 0 / 下游 0）
- `inspect_folder_inventory` @ `grafana_utils/dashboards/folder_support.py:150`（上游 0 / 下游 2）

  - 呼叫: build_live_folder_inventory_record, determine_folder_inventory_status
- `resolve_folder_inventory_requirements` @ `grafana_utils/dashboards/folder_support.py:200`（上游 0 / 下游 1）

  - 呼叫: load_folder_inventory
- `build_folder_inventory_lookup` @ `grafana_utils/dashboards/folder_support.py:230`（上游 0 / 下游 0）
- `build_import_dashboard_folder_path` @ `grafana_utils/dashboards/folder_support.py:241`（上游 0 / 下游 0）
- `resolve_folder_inventory_record_for_dashboard` @ `grafana_utils/dashboards/folder_support.py:250`（上游 1 / 下游 0）
  - 被呼叫: resolve_dashboard_import_folder_path
- `build_live_folder_inventory_record` @ `grafana_utils/dashboards/folder_support.py:267`（上游 2 / 下游 0）
  - 被呼叫: determine_folder_inventory_status, inspect_folder_inventory
- `determine_folder_inventory_status` @ `grafana_utils/dashboards/folder_support.py:315`（上游 1 / 下游 1）

  - 呼叫: build_live_folder_inventory_record
  - 被呼叫: inspect_folder_inventory
- `resolve_dashboard_import_folder_path` @ `grafana_utils/dashboards/folder_support.py:339`（上游 0 / 下游 1）

  - 呼叫: resolve_folder_inventory_record_for_dashboard

## `grafana_utils/dashboards/import_runtime.py`

- `build_import_workflow_deps` @ `grafana_utils/dashboards/import_runtime.py:38`（上游 0 / 下游 0）

## `grafana_utils/dashboards/import_support.py`

- `load_json_file` @ `grafana_utils/dashboards/import_support.py:42`（上游 0 / 下游 0）
- `extract_dashboard_object` @ `grafana_utils/dashboards/import_support.py:60`（上游 1 / 下游 0）
  - 被呼叫: build_import_payload
- `build_import_payload` @ `grafana_utils/dashboards/import_support.py:68`（上游 2 / 下游 1）

  - 呼叫: extract_dashboard_object
  - 被呼叫: build_local_compare_document, resolve_dashboard_uid_for_import
- `load_export_metadata` @ `grafana_utils/dashboards/import_support.py:101`（上游 0 / 下游 0）
- `validate_export_metadata` @ `grafana_utils/dashboards/import_support.py:122`（上游 0 / 下游 0）
- `build_compare_document` @ `grafana_utils/dashboards/import_support.py:143`（上游 2 / 下游 0）
  - 被呼叫: build_local_compare_document, build_remote_compare_document
- `build_local_compare_document` @ `grafana_utils/dashboards/import_support.py:154`（上游 0 / 下游 2）

  - 呼叫: build_compare_document, build_import_payload
- `build_remote_compare_document` @ `grafana_utils/dashboards/import_support.py:172`（上游 0 / 下游 1）

  - 呼叫: build_compare_document
- `serialize_compare_document` @ `grafana_utils/dashboards/import_support.py:185`（上游 0 / 下游 0）
- `build_compare_diff_lines` @ `grafana_utils/dashboards/import_support.py:194`（上游 0 / 下游 0）
- `resolve_dashboard_uid_for_import` @ `grafana_utils/dashboards/import_support.py:230`（上游 0 / 下游 1）

  - 呼叫: build_import_payload
- `determine_dashboard_import_action` @ `grafana_utils/dashboards/import_support.py:248`（上游 0 / 下游 0）
- `determine_import_folder_uid_override` @ `grafana_utils/dashboards/import_support.py:279`（上游 0 / 下游 0）
- `describe_dashboard_import_mode` @ `grafana_utils/dashboards/import_support.py:303`（上游 0 / 下游 0）
- `build_dashboard_import_dry_run_record` @ `grafana_utils/dashboards/import_support.py:319`（上游 0 / 下游 0）
- `parse_dashboard_import_dry_run_columns` @ `grafana_utils/dashboards/import_support.py:362`（上游 0 / 下游 0）
- `_render_table` @ `grafana_utils/dashboards/import_support.py:395`（上游 2 / 下游 1）

  - 呼叫: format_row
  - 被呼叫: render_dashboard_import_dry_run_table, render_folder_inventory_dry_run_table
- `_render_table.format_row` @ `grafana_utils/dashboards/import_support.py:402`（上游 1 / 下游 0）
  - 被呼叫: _render_table
- `render_dashboard_import_dry_run_table` @ `grafana_utils/dashboards/import_support.py:418`（上游 0 / 下游 1）

  - 呼叫: _render_table
- `render_dashboard_import_dry_run_json` @ `grafana_utils/dashboards/import_support.py:447`（上游 0 / 下游 0）
- `render_folder_inventory_dry_run_table` @ `grafana_utils/dashboards/import_support.py:510`（上游 0 / 下游 1）

  - 呼叫: _render_table

## `grafana_utils/dashboards/import_workflow.py`

- `_CachedDashboardImportClient.__init__` @ `grafana_utils/dashboards/import_workflow.py:13`（上游 0 / 下游 0）
- `_CachedDashboardImportClient.__getattr__` @ `grafana_utils/dashboards/import_workflow.py:26`（上游 0 / 下游 0）
- `_CachedDashboardImportClient.fetch_dashboard_if_exists` @ `grafana_utils/dashboards/import_workflow.py:37`（上游 1 / 下游 0）
  - 被呼叫: fetch_dashboard
- `_CachedDashboardImportClient.fetch_dashboard` @ `grafana_utils/dashboards/import_workflow.py:47`（上游 0 / 下游 1）

  - 呼叫: fetch_dashboard_if_exists
- `_CachedDashboardImportClient.fetch_folder_if_exists` @ `grafana_utils/dashboards/import_workflow.py:61`（上游 0 / 下游 0）
- `_CachedDashboardImportClient.create_folder` @ `grafana_utils/dashboards/import_workflow.py:75`（上游 0 / 下游 0）
- `_normalize_org_id` @ `grafana_utils/dashboards/import_workflow.py:92`（上游 2 / 下游 0）
  - 被呼叫: _resolve_existing_orgs_by_id, _validate_export_org_match
- `_validate_export_org_match` @ `grafana_utils/dashboards/import_workflow.py:103`（上游 1 / 下游 1）

  - 呼叫: _normalize_org_id
  - 被呼叫: _run_import_dashboards_for_single_org
- `_clone_import_args` @ `grafana_utils/dashboards/import_workflow.py:129`（上游 1 / 下游 0）
  - 被呼叫: _run_import_dashboards_by_export_org
- `_resolve_existing_orgs_by_id` @ `grafana_utils/dashboards/import_workflow.py:136`（上游 1 / 下游 1）

  - 呼叫: _normalize_org_id
  - 被呼叫: _resolve_multi_org_targets
- `_resolve_created_org_id` @ `grafana_utils/dashboards/import_workflow.py:146`（上游 1 / 下游 0）
  - 被呼叫: _resolve_multi_org_targets
- `_resolve_multi_org_targets` @ `grafana_utils/dashboards/import_workflow.py:159`（上游 1 / 下游 2）

  - 呼叫: _resolve_created_org_id, _resolve_existing_orgs_by_id
  - 被呼叫: _run_import_dashboards_by_export_org
- `_run_import_dashboards_by_export_org` @ `grafana_utils/dashboards/import_workflow.py:259`（上游 1 / 下游 4）

  - 呼叫: _clone_import_args, _resolve_multi_org_targets, format_row, _run_import_dashboards_for_single_org
  - 被呼叫: run_import_dashboards
- `_run_import_dashboards_by_export_org.format_row` @ `grafana_utils/dashboards/import_workflow.py:362`（上游 1 / 下游 0）
  - 被呼叫: _run_import_dashboards_by_export_org
- `_run_import_dashboards_for_single_org` @ `grafana_utils/dashboards/import_workflow.py:431`（上游 2 / 下游 1）

  - 呼叫: _validate_export_org_match
  - 被呼叫: _run_import_dashboards_by_export_org, run_import_dashboards
- `run_import_dashboards` @ `grafana_utils/dashboards/import_workflow.py:799`（上游 0 / 下游 2）

  - 呼叫: _run_import_dashboards_by_export_org, _run_import_dashboards_for_single_org

## `grafana_utils/dashboards/inspection_analyzers/__init__.py`

- 無可辨識函式。

## `grafana_utils/dashboards/inspection_analyzers/contract.py`

- `extract_string_values` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:17`（上游 2 / 下游 0）
  - 被呼叫: extract_buckets, extract_measurements
- `unique_strings` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:33`（上游 4 / 下游 0）
  - 被呼叫: extract_buckets, extract_measurements, extract_metric_names, normalize_query_analysis
- `normalize_query_analysis` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:46`（上游 1 / 下游 1）

  - 呼叫: unique_strings
  - 被呼叫: build_default_query_analysis
- `build_query_field_and_text` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:60`（上游 0 / 下游 0）
- `extract_metric_names` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:146`（上游 1 / 下游 1）

  - 呼叫: unique_strings
  - 被呼叫: build_default_query_analysis
- `extract_measurements` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:171`（上游 1 / 下游 2）

  - 呼叫: extract_string_values, unique_strings
  - 被呼叫: build_default_query_analysis
- `extract_buckets` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:185`（上游 1 / 下游 2）

  - 呼叫: extract_string_values, unique_strings
  - 被呼叫: build_default_query_analysis
- `build_default_query_analysis` @ `grafana_utils/dashboards/inspection_analyzers/contract.py:199`（上游 0 / 下游 4）

  - 呼叫: extract_buckets, extract_measurements, extract_metric_names, normalize_query_analysis

## `grafana_utils/dashboards/inspection_analyzers/dispatcher.py`

- `iter_datasource_ref_parts` @ `grafana_utils/dashboards/inspection_analyzers/dispatcher.py:26`（上游 1 / 下游 0）
  - 被呼叫: resolve_query_analyzer_family
- `iter_inventory_datasource_parts` @ `grafana_utils/dashboards/inspection_analyzers/dispatcher.py:39`（上游 1 / 下游 0）
  - 被呼叫: resolve_query_analyzer_family
- `resolve_query_analyzer_family` @ `grafana_utils/dashboards/inspection_analyzers/dispatcher.py:55`（上游 1 / 下游 2）

  - 呼叫: iter_datasource_ref_parts, iter_inventory_datasource_parts
  - 被呼叫: dispatch_query_analysis
- `dispatch_query_analysis` @ `grafana_utils/dashboards/inspection_analyzers/dispatcher.py:73`（上游 0 / 下游 1）

  - 呼叫: resolve_query_analyzer_family

## `grafana_utils/dashboards/inspection_analyzers/flux.py`

- `extract_flux_pipeline_functions` @ `grafana_utils/dashboards/inspection_analyzers/flux.py:16`（上游 1 / 下游 0）
  - 被呼叫: analyze_query
- `analyze_query` @ `grafana_utils/dashboards/inspection_analyzers/flux.py:26`（上游 0 / 下游 1）

  - 呼叫: extract_flux_pipeline_functions

## `grafana_utils/dashboards/inspection_analyzers/generic.py`

- `analyze_query` @ `grafana_utils/dashboards/inspection_analyzers/generic.py:10`（上游 0 / 下游 0）

## `grafana_utils/dashboards/inspection_analyzers/loki.py`

- `extract_stream_matchers` @ `grafana_utils/dashboards/inspection_analyzers/loki.py:11`（上游 1 / 下游 0）
  - 被呼叫: analyze_query
- `extract_pipeline_stage_names` @ `grafana_utils/dashboards/inspection_analyzers/loki.py:25`（上游 1 / 下游 0）
  - 被呼叫: analyze_query
- `extract_range_and_aggregation_functions` @ `grafana_utils/dashboards/inspection_analyzers/loki.py:45`（上游 1 / 下游 0）
  - 被呼叫: analyze_query
- `analyze_query` @ `grafana_utils/dashboards/inspection_analyzers/loki.py:86`（上游 0 / 下游 3）

  - 呼叫: extract_pipeline_stage_names, extract_range_and_aggregation_functions, extract_stream_matchers

## `grafana_utils/dashboards/inspection_analyzers/prometheus.py`

- `extract_prometheus_metric_names` @ `grafana_utils/dashboards/inspection_analyzers/prometheus.py:18`（上游 1 / 下游 0）
  - 被呼叫: analyze_query
- `analyze_query` @ `grafana_utils/dashboards/inspection_analyzers/prometheus.py:57`（上游 0 / 下游 1）

  - 呼叫: extract_prometheus_metric_names

## `grafana_utils/dashboards/inspection_analyzers/sql.py`

- `strip_sql_comments` @ `grafana_utils/dashboards/inspection_analyzers/sql.py:11`（上游 2 / 下游 0）
  - 被呼叫: extract_sql_query_shape_hints, extract_sql_source_references
- `normalize_sql_identifier` @ `grafana_utils/dashboards/inspection_analyzers/sql.py:19`（上游 1 / 下游 0）
  - 被呼叫: extract_sql_source_references
- `extract_sql_source_references` @ `grafana_utils/dashboards/inspection_analyzers/sql.py:35`（上游 1 / 下游 2）

  - 呼叫: normalize_sql_identifier, strip_sql_comments
  - 被呼叫: analyze_query
- `extract_sql_query_shape_hints` @ `grafana_utils/dashboards/inspection_analyzers/sql.py:64`（上游 1 / 下游 1）

  - 呼叫: strip_sql_comments
  - 被呼叫: analyze_query
- `analyze_query` @ `grafana_utils/dashboards/inspection_analyzers/sql.py:91`（上游 0 / 下游 2）

  - 呼叫: extract_sql_query_shape_hints, extract_sql_source_references

## `grafana_utils/dashboards/inspection_dependency_models.py`

- `_coerce_text` @ `grafana_utils/dashboards/inspection_dependency_models.py:27`（上游 6 / 下游 0）
  - 被呼叫: _coerce_list, _coerce_record_text, _extract_flux_features, _extract_loki_features, _extract_sql_features, _normalize_family
- `_coerce_list` @ `grafana_utils/dashboards/inspection_dependency_models.py:35`（上游 2 / 下游 1）

  - 呼叫: _coerce_text
  - 被呼叫: _extract_loki_features, _extract_prometheus_features
- `_normalize_family` @ `grafana_utils/dashboards/inspection_dependency_models.py:51`（上游 2 / 下游 1）

  - 呼叫: _coerce_text
  - 被呼叫: _extract_features, build_dependency_rows_from_query_report
- `_extract_prometheus_features` @ `grafana_utils/dashboards/inspection_dependency_models.py:67`（上游 1 / 下游 1）

  - 呼叫: _coerce_list
  - 被呼叫: _extract_features
- `_extract_loki_features` @ `grafana_utils/dashboards/inspection_dependency_models.py:86`（上游 1 / 下游 2）

  - 呼叫: _coerce_list, _coerce_text
  - 被呼叫: _extract_features
- `_extract_flux_features` @ `grafana_utils/dashboards/inspection_dependency_models.py:109`（上游 1 / 下游 1）

  - 呼叫: _coerce_text
  - 被呼叫: _extract_features
- `_extract_sql_features` @ `grafana_utils/dashboards/inspection_dependency_models.py:130`（上游 1 / 下游 1）

  - 呼叫: _coerce_text
  - 被呼叫: _extract_features
- `_extract_features` @ `grafana_utils/dashboards/inspection_dependency_models.py:148`（上游 1 / 下游 5）

  - 呼叫: _extract_flux_features, _extract_loki_features, _extract_prometheus_features, _extract_sql_features, _normalize_family
  - 被呼叫: from_query
- `_coerce_record_text` @ `grafana_utils/dashboards/inspection_dependency_models.py:172`（上游 1 / 下游 1）

  - 呼叫: _coerce_text
  - 被呼叫: build_dependency_rows_from_query_report
- `QueryFeatureSet.from_query` @ `grafana_utils/dashboards/inspection_dependency_models.py:188`（上游 1 / 下游 1）

  - 呼叫: _extract_features
  - 被呼叫: build_dependency_rows_from_query_report
- `QueryFeatureSet.as_dict` @ `grafana_utils/dashboards/inspection_dependency_models.py:192`（上游 0 / 下游 0）
- `DependencyQueryRecord.as_dict` @ `grafana_utils/dashboards/inspection_dependency_models.py:227`（上游 0 / 下游 0）
- `DatasourceUsageSummary.as_dict` @ `grafana_utils/dashboards/inspection_dependency_models.py:263`（上游 0 / 下游 0）
- `OfflineDependencyReport.to_dict` @ `grafana_utils/dashboards/inspection_dependency_models.py:288`（上游 0 / 下游 0）
- `build_dependency_rows_from_query_report` @ `grafana_utils/dashboards/inspection_dependency_models.py:302`（上游 0 / 下游 3）

  - 呼叫: from_query, _coerce_record_text, _normalize_family

## `grafana_utils/dashboards/inspection_dispatch.py`

- `resolve_inspect_output_mode` @ `grafana_utils/dashboards/inspection_dispatch.py:30`（上游 1 / 下游 0）
  - 被呼叫: resolve_inspect_dispatch_args
- `resolve_inspect_dispatch_args` @ `grafana_utils/dashboards/inspection_dispatch.py:46`（上游 0 / 下游 1）

  - 呼叫: resolve_inspect_output_mode
- `build_filtered_report_document` @ `grafana_utils/dashboards/inspection_dispatch.py:107`（上游 1 / 下游 0）
  - 被呼叫: _render_report_output
- `_render_report_output` @ `grafana_utils/dashboards/inspection_dispatch.py:116`（上游 1 / 下游 1）

  - 呼叫: build_filtered_report_document
  - 被呼叫: run_inspection_dispatch
- `_render_summary_output` @ `grafana_utils/dashboards/inspection_dispatch.py:219`（上游 1 / 下游 0）
  - 被呼叫: run_inspection_dispatch
- `run_inspection_dispatch` @ `grafana_utils/dashboards/inspection_dispatch.py:238`（上游 0 / 下游 2）

  - 呼叫: _render_report_output, _render_summary_output

## `grafana_utils/dashboards/inspection_governance.py`

- `_iter_dashboard_panels` @ `grafana_utils/dashboards/inspection_governance.py:17`（上游 1 / 下游 0）
  - 被呼叫: build_dashboard_dependency_records
- `_unique_strings` @ `grafana_utils/dashboards/inspection_governance.py:32`（上游 4 / 下游 0）
  - 被呼叫: build_dashboard_dependency_records, build_datasource_coverage_records, build_datasource_family_coverage_records, build_governance_risk_records
- `_resolve_datasource_inventory` @ `grafana_utils/dashboards/inspection_governance.py:45`（上游 3 / 下游 0）
  - 被呼叫: build_datasource_coverage_records, build_datasource_family_coverage_records, build_governance_risk_records
- `_resolve_datasource_identity` @ `grafana_utils/dashboards/inspection_governance.py:64`（上游 3 / 下游 0）
  - 被呼叫: build_datasource_coverage_records, build_datasource_family_coverage_records, build_governance_risk_records
- `_normalize_family_name` @ `grafana_utils/dashboards/inspection_governance.py:92`（上游 3 / 下游 0）
  - 被呼叫: build_datasource_coverage_records, build_datasource_family_coverage_records, build_governance_risk_records
- `_build_query_analysis_state` @ `grafana_utils/dashboards/inspection_governance.py:108`（上游 1 / 下游 0）
  - 被呼叫: build_governance_risk_records
- `_extract_datasource_variable_name` @ `grafana_utils/dashboards/inspection_governance.py:117`（上游 1 / 下游 0）
  - 被呼叫: build_dashboard_dependency_records
- `_build_governance_risk_record` @ `grafana_utils/dashboards/inspection_governance.py:128`（上游 1 / 下游 0）
  - 被呼叫: build_governance_risk_records
- `build_datasource_family_coverage_records` @ `grafana_utils/dashboards/inspection_governance.py:174`（上游 1 / 下游 4）

  - 呼叫: _normalize_family_name, _resolve_datasource_identity, _resolve_datasource_inventory, _unique_strings
  - 被呼叫: build_export_inspection_governance_document
- `build_datasource_coverage_records` @ `grafana_utils/dashboards/inspection_governance.py:233`（上游 1 / 下游 4）

  - 呼叫: _normalize_family_name, _resolve_datasource_identity, _resolve_datasource_inventory, _unique_strings
  - 被呼叫: build_export_inspection_governance_document
- `_load_dashboard_object_from_record` @ `grafana_utils/dashboards/inspection_governance.py:312`（上游 1 / 下游 0）
  - 被呼叫: build_dashboard_dependency_records
- `build_dashboard_dependency_records` @ `grafana_utils/dashboards/inspection_governance.py:332`（上游 1 / 下游 4）

  - 呼叫: _extract_datasource_variable_name, _iter_dashboard_panels, _load_dashboard_object_from_record, _unique_strings
  - 被呼叫: build_export_inspection_governance_document
- `build_governance_risk_records` @ `grafana_utils/dashboards/inspection_governance.py:438`（上游 1 / 下游 6）

  - 呼叫: _build_governance_risk_record, _build_query_analysis_state, _normalize_family_name, _resolve_datasource_identity, _resolve_datasource_inventory, _unique_strings
  - 被呼叫: build_export_inspection_governance_document
- `build_export_inspection_governance_document` @ `grafana_utils/dashboards/inspection_governance.py:532`（上游 0 / 下游 4）

  - 呼叫: build_dashboard_dependency_records, build_datasource_coverage_records, build_datasource_family_coverage_records, build_governance_risk_records

## `grafana_utils/dashboards/inspection_governance_render.py`

- `_stringify_cell` @ `grafana_utils/dashboards/inspection_governance_render.py:6`（上游 1 / 下游 0）
  - 被呼叫: _render_named_section
- `_render_table` @ `grafana_utils/dashboards/inspection_governance_render.py:15`（上游 1 / 下游 0）
  - 被呼叫: _render_named_section
- `_render_named_section` @ `grafana_utils/dashboards/inspection_governance_render.py:32`（上游 1 / 下游 2）

  - 呼叫: _render_table, _stringify_cell
  - 被呼叫: render_export_inspection_governance_tables
- `render_export_inspection_governance_tables` @ `grafana_utils/dashboards/inspection_governance_render.py:50`（上游 0 / 下游 1）

  - 呼叫: _render_named_section

## `grafana_utils/dashboards/inspection_render.py`

- `format_report_column_value` @ `grafana_utils/dashboards/inspection_render.py:21`（上游 4 / 下游 0）
  - 被呼叫: render_export_inspection_grouped_report, render_export_inspection_report_csv, render_export_inspection_report_tables, render_export_inspection_tree_tables
- `render_export_inspection_report_csv` @ `grafana_utils/dashboards/inspection_render.py:29`（上游 0 / 下游 1）

  - 呼叫: format_report_column_value
- `render_export_inspection_table_section` @ `grafana_utils/dashboards/inspection_render.py:64`（上游 3 / 下游 1）

  - 呼叫: format_row
  - 被呼叫: render_export_inspection_grouped_report, render_export_inspection_report_tables, render_export_inspection_tree_tables
- `render_export_inspection_table_section.format_row` @ `grafana_utils/dashboards/inspection_render.py:75`（上游 1 / 下游 0）
  - 被呼叫: render_export_inspection_table_section
- `render_export_inspection_report_tables` @ `grafana_utils/dashboards/inspection_render.py:92`（上游 0 / 下游 2）

  - 呼叫: format_report_column_value, render_export_inspection_table_section
- `render_export_inspection_grouped_report` @ `grafana_utils/dashboards/inspection_render.py:142`（上游 0 / 下游 2）

  - 呼叫: format_report_column_value, render_export_inspection_table_section
- `render_export_inspection_tree_tables` @ `grafana_utils/dashboards/inspection_render.py:220`（上游 0 / 下游 2）

  - 呼叫: format_report_column_value, render_export_inspection_table_section

## `grafana_utils/dashboards/inspection_report.py`

- `format_supported_report_column_values` @ `grafana_utils/dashboards/inspection_report.py:136`（上游 0 / 下游 0）
- `build_export_inspection_report_document` @ `grafana_utils/dashboards/inspection_report.py:145`（上游 0 / 下游 1）

  - 呼叫: build_query_report_record
- `describe_export_datasource_ref` @ `grafana_utils/dashboards/inspection_report.py:227`（上游 1 / 下游 0）
  - 被呼叫: describe_panel_datasource
- `describe_panel_datasource` @ `grafana_utils/dashboards/inspection_report.py:265`（上游 1 / 下游 1）

  - 呼叫: describe_export_datasource_ref
  - 被呼叫: build_query_report_record
- `describe_panel_datasource_uid` @ `grafana_utils/dashboards/inspection_report.py:288`（上游 1 / 下游 0）
  - 被呼叫: build_query_report_record
- `_normalize_datasource_family_name` @ `grafana_utils/dashboards/inspection_report.py:309`（上游 1 / 下游 0）
  - 被呼叫: build_query_report_record
- `describe_panel_datasource_type` @ `grafana_utils/dashboards/inspection_report.py:321`（上游 1 / 下游 0）
  - 被呼叫: build_query_report_record
- `build_query_report_record` @ `grafana_utils/dashboards/inspection_report.py:350`（上游 1 / 下游 4）

  - 呼叫: _normalize_datasource_family_name, describe_panel_datasource, describe_panel_datasource_type, describe_panel_datasource_uid
  - 被呼叫: build_export_inspection_report_document
- `parse_report_columns` @ `grafana_utils/dashboards/inspection_report.py:418`（上游 0 / 下游 0）
- `filter_export_inspection_report_document` @ `grafana_utils/dashboards/inspection_report.py:451`（上游 0 / 下游 0）
- `build_grouped_export_inspection_report_document` @ `grafana_utils/dashboards/inspection_report.py:496`（上游 0 / 下游 0）

## `grafana_utils/dashboards/inspection_runtime.py`

- `iter_dashboard_panels` @ `grafana_utils/dashboards/inspection_runtime.py:47`（上游 0 / 下游 0）
- `build_inspection_workflow_deps` @ `grafana_utils/dashboards/inspection_runtime.py:66`（上游 0 / 下游 0）

## `grafana_utils/dashboards/inspection_summary.py`

- `summarize_datasource_inventory_usage` @ `grafana_utils/dashboards/inspection_summary.py:18`（上游 1 / 下游 0）
  - 被呼叫: build_export_inspection_document
- `build_orphaned_datasource_record` @ `grafana_utils/dashboards/inspection_summary.py:42`（上游 1 / 下游 0）
  - 被呼叫: build_export_inspection_document
- `build_export_inspection_document` @ `grafana_utils/dashboards/inspection_summary.py:56`（上游 0 / 下游 2）

  - 呼叫: build_orphaned_datasource_record, summarize_datasource_inventory_usage
- `render_export_inspection_summary` @ `grafana_utils/dashboards/inspection_summary.py:233`（上游 0 / 下游 0）
- `render_export_inspection_tables` @ `grafana_utils/dashboards/inspection_summary.py:334`（上游 0 / 下游 0）

## `grafana_utils/dashboards/inspection_workflow.py`

- `materialize_live_inspection_export` @ `grafana_utils/dashboards/inspection_workflow.py:13`（上游 1 / 下游 0）
  - 被呼叫: run_inspect_live
- `run_inspect_live` @ `grafana_utils/dashboards/inspection_workflow.py:63`（上游 0 / 下游 2）

  - 呼叫: materialize_live_inspection_export, run_inspect_export
- `run_inspect_export` @ `grafana_utils/dashboards/inspection_workflow.py:91`（上游 1 / 下游 0）
  - 被呼叫: run_inspect_live

## `grafana_utils/dashboards/listing.py`

- `format_dashboard_summary_line` @ `grafana_utils/dashboards/listing.py:29`（上游 0 / 下游 1）

  - 呼叫: build_dashboard_summary_record
- `build_dashboard_summary_record` @ `grafana_utils/dashboards/listing.py:45`（上游 4 / 下游 0）
  - 被呼叫: format_dashboard_summary_line, render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table
- `build_folder_path` @ `grafana_utils/dashboards/listing.py:64`（上游 1 / 下游 0）
  - 被呼叫: attach_dashboard_folder_paths
- `attach_dashboard_folder_paths` @ `grafana_utils/dashboards/listing.py:82`（上游 1 / 下游 1）

  - 呼叫: build_folder_path
  - 被呼叫: list_dashboards
- `describe_datasource_ref` @ `grafana_utils/dashboards/listing.py:109`（上游 1 / 下游 0）
  - 被呼叫: resolve_dashboard_source_metadata
- `resolve_datasource_uid` @ `grafana_utils/dashboards/listing.py:164`（上游 1 / 下游 0）
  - 被呼叫: resolve_dashboard_source_metadata
- `resolve_dashboard_source_metadata` @ `grafana_utils/dashboards/listing.py:215`（上游 1 / 下游 2）

  - 呼叫: describe_datasource_ref, resolve_datasource_uid
  - 被呼叫: attach_dashboard_sources
- `attach_dashboard_sources` @ `grafana_utils/dashboards/listing.py:262`（上游 1 / 下游 1）

  - 呼叫: resolve_dashboard_source_metadata
  - 被呼叫: list_dashboards
- `attach_dashboard_org` @ `grafana_utils/dashboards/listing.py:294`（上游 1 / 下游 0）
  - 被呼叫: list_dashboards
- `render_dashboard_summary_table` @ `grafana_utils/dashboards/listing.py:311`（上游 1 / 下游 2）

  - 呼叫: build_dashboard_summary_record, format_row
  - 被呼叫: list_dashboards
- `render_dashboard_summary_table.format_row` @ `grafana_utils/dashboards/listing.py:338`（上游 1 / 下游 0）
  - 被呼叫: render_dashboard_summary_table
- `render_dashboard_summary_csv` @ `grafana_utils/dashboards/listing.py:354`（上游 1 / 下游 1）

  - 呼叫: build_dashboard_summary_record
  - 被呼叫: list_dashboards
- `render_dashboard_summary_json` @ `grafana_utils/dashboards/listing.py:367`（上游 1 / 下游 1）

  - 呼叫: build_dashboard_summary_record
  - 被呼叫: list_dashboards
- `list_dashboards` @ `grafana_utils/dashboards/listing.py:380`（上游 0 / 下游 6）

  - 呼叫: attach_dashboard_folder_paths, attach_dashboard_org, attach_dashboard_sources, render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table
- `format_data_source_line` @ `grafana_utils/dashboards/listing.py:448`（上游 0 / 下游 1）

  - 呼叫: build_data_source_record
- `build_data_source_record` @ `grafana_utils/dashboards/listing.py:460`（上游 5 / 下游 0）
  - 被呼叫: build_datasource_inventory_record, format_data_source_line, render_data_source_csv, render_data_source_json, render_data_source_table
- `data_source_rows_include_org_scope` @ `grafana_utils/dashboards/listing.py:477`（上游 2 / 下游 0）
  - 被呼叫: render_data_source_csv, render_data_source_table
- `build_datasource_inventory_record` @ `grafana_utils/dashboards/listing.py:486`（上游 0 / 下游 1）

  - 呼叫: build_data_source_record
- `render_data_source_table` @ `grafana_utils/dashboards/listing.py:502`（上游 1 / 下游 3）

  - 呼叫: build_data_source_record, data_source_rows_include_org_scope, format_row
  - 被呼叫: list_data_sources
- `render_data_source_table.format_row` @ `grafana_utils/dashboards/listing.py:532`（上游 1 / 下游 0）
  - 被呼叫: render_data_source_table
- `render_data_source_csv` @ `grafana_utils/dashboards/listing.py:548`（上游 1 / 下游 2）

  - 呼叫: build_data_source_record, data_source_rows_include_org_scope
  - 被呼叫: list_data_sources
- `render_data_source_json` @ `grafana_utils/dashboards/listing.py:564`（上游 1 / 下游 1）

  - 呼叫: build_data_source_record
  - 被呼叫: list_data_sources
- `list_data_sources` @ `grafana_utils/dashboards/listing.py:573`（上游 0 / 下游 3）

  - 呼叫: render_data_source_csv, render_data_source_json, render_data_source_table

## `grafana_utils/dashboards/output_support.py`

- `sanitize_path_component` @ `grafana_utils/dashboards/output_support.py:9`（上游 2 / 下游 0）
  - 被呼叫: build_all_orgs_output_dir, build_output_path
- `build_output_path` @ `grafana_utils/dashboards/output_support.py:18`（上游 0 / 下游 1）

  - 呼叫: sanitize_path_component
- `build_all_orgs_output_dir` @ `grafana_utils/dashboards/output_support.py:43`（上游 0 / 下游 1）

  - 呼叫: sanitize_path_component
- `build_export_variant_dirs` @ `grafana_utils/dashboards/output_support.py:58`（上游 0 / 下游 0）
- `ensure_dashboard_write_target` @ `grafana_utils/dashboards/output_support.py:71`（上游 1 / 下游 0）
  - 被呼叫: write_dashboard
- `write_dashboard` @ `grafana_utils/dashboards/output_support.py:86`（上游 0 / 下游 1）

  - 呼叫: ensure_dashboard_write_target
- `write_json_document` @ `grafana_utils/dashboards/output_support.py:104`（上游 0 / 下游 0）
- `build_dashboard_index_item` @ `grafana_utils/dashboards/output_support.py:117`（上游 0 / 下游 0）
- `build_variant_index` @ `grafana_utils/dashboards/output_support.py:137`（上游 0 / 下游 0）
- `build_root_export_index` @ `grafana_utils/dashboards/output_support.py:162`（上游 0 / 下游 0）
- `build_export_metadata` @ `grafana_utils/dashboards/output_support.py:185`（上游 0 / 下游 0）

## `grafana_utils/dashboards/progress.py`

- `print_dashboard_export_progress` @ `grafana_utils/dashboards/progress.py:7`（上游 0 / 下游 0）
- `print_dashboard_export_progress_summary` @ `grafana_utils/dashboards/progress.py:28`（上游 0 / 下游 0）
- `print_dashboard_import_progress` @ `grafana_utils/dashboards/progress.py:49`（上游 0 / 下游 0）

## `grafana_utils/dashboards/reference_models.py`

- `_coerce_text` @ `grafana_utils/dashboards/reference_models.py:14`（上游 6 / 下游 0）
  - 被呼叫: as_dict, from_mappings, from_mapping, from_mapping, from_mapping, _collect_unique
- `_collect_unique` @ `grafana_utils/dashboards/reference_models.py:22`（上游 1 / 下游 1）

  - 呼叫: _coerce_text
  - 被呼叫: dedupe_text_sequence
- `DatasourceReference.stable_identity` @ `grafana_utils/dashboards/reference_models.py:52`（上游 0 / 下游 0）
- `DatasourceReference.from_mapping` @ `grafana_utils/dashboards/reference_models.py:61`（上游 0 / 下游 1）

  - 呼叫: _coerce_text
- `DatasourceReference.as_dict` @ `grafana_utils/dashboards/reference_models.py:81`（上游 0 / 下游 0）
- `DashboardReference.from_mapping` @ `grafana_utils/dashboards/reference_models.py:112`（上游 0 / 下游 1）

  - 呼叫: _coerce_text
- `PanelReference.from_mapping` @ `grafana_utils/dashboards/reference_models.py:142`（上游 0 / 下游 1）

  - 呼叫: _coerce_text
- `DashboardQueryReference.dashboard_uid` @ `grafana_utils/dashboards/reference_models.py:170`（上游 0 / 下游 0）
- `DashboardQueryReference.from_mappings` @ `grafana_utils/dashboards/reference_models.py:179`（上游 0 / 下游 1）

  - 呼叫: _coerce_text
- `DashboardQueryReference.as_dict` @ `grafana_utils/dashboards/reference_models.py:209`（上游 0 / 下游 1）

  - 呼叫: _coerce_text
- `collect_datasource_reference_index` @ `grafana_utils/dashboards/reference_models.py:233`（上游 0 / 下游 0）
- `extract_dashboard_reference_sequence` @ `grafana_utils/dashboards/reference_models.py:250`（上游 0 / 下游 0）
- `dedupe_text_sequence` @ `grafana_utils/dashboards/reference_models.py:261`（上游 0 / 下游 1）

  - 呼叫: _collect_unique

## `grafana_utils/dashboards/screenshot.py`

- `parse_var_assignment` @ `grafana_utils/dashboards/screenshot.py:46`（上游 2 / 下游 0）
  - 被呼叫: build_dashboard_capture_url, validate_screenshot_args
- `parse_vars_query` @ `grafana_utils/dashboards/screenshot.py:67`（上游 2 / 下游 0）
  - 被呼叫: build_dashboard_capture_url, validate_screenshot_args
- `infer_screenshot_output_format` @ `grafana_utils/dashboards/screenshot.py:95`（上游 2 / 下游 0）
  - 被呼叫: build_capture_request, validate_screenshot_args
- `validate_screenshot_args` @ `grafana_utils/dashboards/screenshot.py:119`（上游 2 / 下游 3）

  - 呼叫: infer_screenshot_output_format, parse_var_assignment, parse_vars_query
  - 被呼叫: build_capture_request, build_dashboard_capture_url
- `_normalize_dashboard_target_state` @ `grafana_utils/dashboards/screenshot.py:167`（上游 1 / 下游 0）
  - 被呼叫: build_dashboard_capture_url
- `build_dashboard_capture_url` @ `grafana_utils/dashboards/screenshot.py:225`（上游 2 / 下游 4）

  - 呼叫: _normalize_dashboard_target_state, parse_var_assignment, parse_vars_query, validate_screenshot_args
  - 被呼叫: build_capture_request, build_render_url
- `build_capture_request` @ `grafana_utils/dashboards/screenshot.py:325`（上游 1 / 下游 4）

  - 呼叫: build_dashboard_capture_url, build_render_url, infer_screenshot_output_format, validate_screenshot_args
  - 被呼叫: capture_dashboard_screenshot
- `_resolve_capture_metadata` @ `grafana_utils/dashboards/screenshot.py:356`（上游 1 / 下游 1）

  - 呼叫: _find_panel_title
  - 被呼叫: capture_dashboard_screenshot
- `_find_panel_title` @ `grafana_utils/dashboards/screenshot.py:404`（上游 1 / 下游 1）

  - 呼叫: visit
  - 被呼叫: _resolve_capture_metadata
- `_find_panel_title.visit` @ `grafana_utils/dashboards/screenshot.py:412`（上游 2 / 下游 0）
  - 被呼叫: _find_panel_title, visit
- `_resolve_auto_header_title` @ `grafana_utils/dashboards/screenshot.py:435`（上游 2 / 下游 0）
  - 被呼叫: _build_full_page_manifest, _build_header_lines
- `_resolve_optional_header_field` @ `grafana_utils/dashboards/screenshot.py:449`（上游 2 / 下游 0）
  - 被呼叫: _build_full_page_manifest, _build_header_lines
- `_build_header_lines` @ `grafana_utils/dashboards/screenshot.py:459`（上游 2 / 下游 2）

  - 呼叫: _resolve_auto_header_title, _resolve_optional_header_field
  - 被呼叫: _compose_header_image, _write_full_page_output
- `_wrap_header_text` @ `grafana_utils/dashboards/screenshot.py:482`（上游 1 / 下游 0）
  - 被呼叫: _compose_header_image
- `_compose_header_image` @ `grafana_utils/dashboards/screenshot.py:500`（上游 1 / 下游 2）

  - 呼叫: _build_header_lines, _wrap_header_text
  - 被呼叫: _write_raster_output
- `_write_raster_output` @ `grafana_utils/dashboards/screenshot.py:538`（上游 2 / 下游 1）

  - 呼叫: _compose_header_image
  - 被呼叫: _capture_via_devtools, _write_full_page_output
- `find_browser_executable` @ `grafana_utils/dashboards/screenshot.py:547`（上游 1 / 下游 0）
  - 被呼叫: _capture_with_browser_cli
- `build_render_url` @ `grafana_utils/dashboards/screenshot.py:570`（上游 1 / 下游 1）

  - 呼叫: build_dashboard_capture_url
  - 被呼叫: build_capture_request
- `_capture_with_grafana_render` @ `grafana_utils/dashboards/screenshot.py:595`（上游 0 / 下游 0）
- `_strip_hop_by_hop_headers` @ `grafana_utils/dashboards/screenshot.py:633`（上游 1 / 下游 0）
  - 被呼叫: _forward
- `_rewrite_location` @ `grafana_utils/dashboards/screenshot.py:654`（上游 1 / 下游 0）
  - 被呼叫: _forward
- `run_auth_proxy` @ `grafana_utils/dashboards/screenshot.py:664`（上游 0 / 下游 0）
- `run_auth_proxy.ProxyHandler.log_message` @ `grafana_utils/dashboards/screenshot.py:678`（上游 0 / 下游 0）
- `run_auth_proxy.ProxyHandler.do_GET` @ `grafana_utils/dashboards/screenshot.py:685`（上游 0 / 下游 0）
- `run_auth_proxy.ProxyHandler.do_POST` @ `grafana_utils/dashboards/screenshot.py:692`（上游 0 / 下游 0）
- `run_auth_proxy.ProxyHandler._forward` @ `grafana_utils/dashboards/screenshot.py:699`（上游 2 / 下游 0）
  - 被呼叫: do_GET, do_POST
- `_build_proxy_capture_url` @ `grafana_utils/dashboards/screenshot.py:750`（上游 0 / 下游 0）
- `_DevtoolsClient.__init__` @ `grafana_utils/dashboards/screenshot.py:771`（上游 0 / 下游 1）

  - 呼叫: _connect
- `_DevtoolsClient.close` @ `grafana_utils/dashboards/screenshot.py:793`（上游 3 / 下游 0）
  - 被呼叫: _capture_via_devtools, _pick_local_port, _read_json_url
- `_DevtoolsClient._connect` @ `grafana_utils/dashboards/screenshot.py:801`（上游 1 / 下游 1）

  - 呼叫: _recv_http_response
  - 被呼叫: __init__
- `_DevtoolsClient._recv_http_response` @ `grafana_utils/dashboards/screenshot.py:827`（上游 1 / 下游 0）
  - 被呼叫: _connect
- `_DevtoolsClient._send_frame` @ `grafana_utils/dashboards/screenshot.py:842`（上游 1 / 下游 0）
  - 被呼叫: call
- `_DevtoolsClient._recv_frame` @ `grafana_utils/dashboards/screenshot.py:864`（上游 1 / 下游 2）

  - 呼叫: _read_exact, _send_control_frame
  - 被呼叫: call
- `_DevtoolsClient._send_control_frame` @ `grafana_utils/dashboards/screenshot.py:895`（上游 1 / 下游 0）
  - 被呼叫: _recv_frame
- `_DevtoolsClient._read_exact` @ `grafana_utils/dashboards/screenshot.py:908`（上游 1 / 下游 0）
  - 被呼叫: _recv_frame
- `_DevtoolsClient.call` @ `grafana_utils/dashboards/screenshot.py:923`（上游 5 / 下游 2）

  - 呼叫: _recv_frame, _send_frame
  - 被呼叫: _capture_full_page_segments, _capture_stitched_screenshot, _capture_via_devtools, _evaluate_expression, _wait_for_ready_state
- `_read_json_url` @ `grafana_utils/dashboards/screenshot.py:950`（上游 1 / 下游 1）

  - 呼叫: close
  - 被呼叫: _launch_devtools_browser
- `_pick_local_port` @ `grafana_utils/dashboards/screenshot.py:959`（上游 1 / 下游 1）

  - 呼叫: close
  - 被呼叫: _launch_devtools_browser
- `_launch_devtools_browser` @ `grafana_utils/dashboards/screenshot.py:970`（上游 1 / 下游 2）

  - 呼叫: _pick_local_port, _read_json_url
  - 被呼叫: _capture_via_devtools
- `_wait_for_ready_state` @ `grafana_utils/dashboards/screenshot.py:1018`（上游 1 / 下游 1）

  - 呼叫: call
  - 被呼叫: _capture_via_devtools
- `_evaluate_expression` @ `grafana_utils/dashboards/screenshot.py:1074`（上游 6 / 下游 1）

  - 呼叫: call
  - 被呼叫: _capture_full_page_segments, _capture_stitched_screenshot, _collapse_sidebar_if_present, _prepare_dashboard_capture_dom, _read_numeric_expression, _warm_full_page_render
- `_collapse_sidebar_if_present` @ `grafana_utils/dashboards/screenshot.py:1084`（上游 1 / 下游 1）

  - 呼叫: _evaluate_expression
  - 被呼叫: _capture_via_devtools
- `_prepare_dashboard_capture_dom` @ `grafana_utils/dashboards/screenshot.py:1115`（上游 1 / 下游 1）

  - 呼叫: _evaluate_expression
  - 被呼叫: _capture_via_devtools
- `_read_numeric_expression` @ `grafana_utils/dashboards/screenshot.py:1258`（上游 3 / 下游 1）

  - 呼叫: _evaluate_expression
  - 被呼叫: _capture_full_page_segments, _capture_stitched_screenshot, _warm_full_page_render
- `_warm_full_page_render` @ `grafana_utils/dashboards/screenshot.py:1268`（上游 1 / 下游 2）

  - 呼叫: _evaluate_expression, _read_numeric_expression
  - 被呼叫: _capture_via_devtools
- `_capture_stitched_screenshot` @ `grafana_utils/dashboards/screenshot.py:1326`（上游 0 / 下游 3）

  - 呼叫: call, _evaluate_expression, _read_numeric_expression
- `_capture_full_page_segments` @ `grafana_utils/dashboards/screenshot.py:1394`（上游 1 / 下游 3）

  - 呼叫: call, _evaluate_expression, _read_numeric_expression
  - 被呼叫: _capture_via_devtools
- `_build_segment_output_dir` @ `grafana_utils/dashboards/screenshot.py:1473`（上游 1 / 下游 0）
  - 被呼叫: _write_full_page_output
- `_build_full_page_manifest` @ `grafana_utils/dashboards/screenshot.py:1485`（上游 1 / 下游 2）

  - 呼叫: _resolve_auto_header_title, _resolve_optional_header_field
  - 被呼叫: _write_full_page_output
- `_write_full_page_output` @ `grafana_utils/dashboards/screenshot.py:1520`（上游 1 / 下游 4）

  - 呼叫: _build_full_page_manifest, _build_header_lines, _build_segment_output_dir, _write_raster_output
  - 被呼叫: _capture_via_devtools
- `_capture_via_devtools` @ `grafana_utils/dashboards/screenshot.py:1574`（上游 1 / 下游 10）

  - 呼叫: call, close, _capture_full_page_segments, _collapse_sidebar_if_present, _launch_devtools_browser, _prepare_dashboard_capture_dom, _wait_for_ready_state, _warm_full_page_render, _write_full_page_output, _write_raster_output
  - 被呼叫: _run_browser_capture
- `_run_browser_capture` @ `grafana_utils/dashboards/screenshot.py:1649`（上游 1 / 下游 1）

  - 呼叫: _capture_via_devtools
  - 被呼叫: _capture_with_browser_cli
- `_capture_with_browser_cli` @ `grafana_utils/dashboards/screenshot.py:1674`（上游 1 / 下游 2）

  - 呼叫: _run_browser_capture, find_browser_executable
  - 被呼叫: capture_dashboard_screenshot
- `capture_dashboard_screenshot` @ `grafana_utils/dashboards/screenshot.py:1692`（上游 0 / 下游 3）

  - 呼叫: _capture_with_browser_cli, _resolve_capture_metadata, build_capture_request
- `make_screenshot_args` @ `grafana_utils/dashboards/screenshot.py:1719`（上游 0 / 下游 0）

## `grafana_utils/dashboards/transformer.py`

- `build_datasource_catalog` @ `grafana_utils/dashboards/transformer.py:34`（上游 0 / 下游 0）
- `is_placeholder_string` @ `grafana_utils/dashboards/transformer.py:54`（上游 3 / 下游 0）
  - 被呼叫: resolve_datasource_ref, resolve_object_datasource_ref, resolve_placeholder_object_ref
- `extract_placeholder_name` @ `grafana_utils/dashboards/transformer.py:59`（上游 3 / 下游 0）
  - 被呼叫: is_generated_input_placeholder, resolve_placeholder_object_ref, rewrite_template_variable_datasource
- `is_generated_input_placeholder` @ `grafana_utils/dashboards/transformer.py:68`（上游 1 / 下游 1）

  - 呼叫: extract_placeholder_name
  - 被呼叫: is_builtin_datasource_ref
- `is_builtin_datasource_ref` @ `grafana_utils/dashboards/transformer.py:73`（上游 1 / 下游 1）

  - 呼叫: is_generated_input_placeholder
  - 被呼叫: resolve_datasource_ref
- `collect_datasource_refs` @ `grafana_utils/dashboards/transformer.py:92`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `make_input_name` @ `grafana_utils/dashboards/transformer.py:105`（上游 2 / 下游 0）
  - 被呼叫: allocate_input_mapping, make_type_input_base
- `make_type_input_base` @ `grafana_utils/dashboards/transformer.py:112`（上游 0 / 下游 1）

  - 呼叫: make_input_name
- `format_plugin_name` @ `grafana_utils/dashboards/transformer.py:122`（上游 4 / 下游 0）
  - 被呼叫: allocate_input_mapping, make_input_label, resolve_placeholder_object_ref, resolve_string_datasource_ref
- `make_input_label` @ `grafana_utils/dashboards/transformer.py:128`（上游 1 / 下游 1）

  - 呼叫: format_plugin_name
  - 被呼叫: allocate_input_mapping
- `build_resolved_datasource` @ `grafana_utils/dashboards/transformer.py:136`（上游 0 / 下游 0）
- `datasource_plugin_version` @ `grafana_utils/dashboards/transformer.py:155`（上游 2 / 下游 0）
  - 被呼叫: resolve_object_datasource_ref, resolve_string_datasource_ref
- `lookup_datasource` @ `grafana_utils/dashboards/transformer.py:175`（上游 2 / 下游 0）
  - 被呼叫: resolve_object_datasource_ref, resolve_string_datasource_ref
- `resolve_datasource_type_alias` @ `grafana_utils/dashboards/transformer.py:191`（上游 1 / 下游 0）
  - 被呼叫: resolve_string_datasource_ref
- `resolve_string_datasource_ref` @ `grafana_utils/dashboards/transformer.py:208`（上游 1 / 下游 4）

  - 呼叫: datasource_plugin_version, format_plugin_name, lookup_datasource, resolve_datasource_type_alias
  - 被呼叫: resolve_datasource_ref
- `resolve_placeholder_object_ref` @ `grafana_utils/dashboards/transformer.py:251`（上游 1 / 下游 3）

  - 呼叫: extract_placeholder_name, format_plugin_name, is_placeholder_string
  - 被呼叫: resolve_object_datasource_ref
- `resolve_object_datasource_ref` @ `grafana_utils/dashboards/transformer.py:281`（上游 1 / 下游 4）

  - 呼叫: datasource_plugin_version, is_placeholder_string, lookup_datasource, resolve_placeholder_object_ref
  - 被呼叫: resolve_datasource_ref
- `resolve_datasource_ref` @ `grafana_utils/dashboards/transformer.py:337`（上游 3 / 下游 4）

  - 呼叫: is_builtin_datasource_ref, is_placeholder_string, resolve_object_datasource_ref, resolve_string_datasource_ref
  - 被呼叫: build_external_export_document, prepare_templating_for_external_import, replace_datasource_refs_in_dashboard
- `replace_datasource_refs_in_dashboard` @ `grafana_utils/dashboards/transformer.py:369`（上游 1 / 下游 1）

  - 呼叫: resolve_datasource_ref
  - 被呼叫: build_external_export_document
- `ensure_datasource_template_variable` @ `grafana_utils/dashboards/transformer.py:412`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `rewrite_panel_datasources_to_template_variable` @ `grafana_utils/dashboards/transformer.py:445`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `allocate_input_mapping` @ `grafana_utils/dashboards/transformer.py:470`（上游 2 / 下游 3）

  - 呼叫: format_plugin_name, make_input_label, make_input_name
  - 被呼叫: build_external_export_document, prepare_templating_for_external_import
- `rewrite_template_variable_query` @ `grafana_utils/dashboards/transformer.py:502`（上游 1 / 下游 0）
  - 被呼叫: prepare_templating_for_external_import
- `rewrite_template_variable_datasource` @ `grafana_utils/dashboards/transformer.py:524`（上游 1 / 下游 1）

  - 呼叫: extract_placeholder_name
  - 被呼叫: prepare_templating_for_external_import
- `prepare_templating_for_external_import` @ `grafana_utils/dashboards/transformer.py:563`（上游 1 / 下游 4）

  - 呼叫: allocate_input_mapping, resolve_datasource_ref, rewrite_template_variable_datasource, rewrite_template_variable_query
  - 被呼叫: build_external_export_document
- `collect_panel_types` @ `grafana_utils/dashboards/transformer.py:633`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `build_input_definitions` @ `grafana_utils/dashboards/transformer.py:647`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `build_requires_block` @ `grafana_utils/dashboards/transformer.py:664`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `build_preserved_web_import_document` @ `grafana_utils/dashboards/transformer.py:699`（上游 1 / 下游 0）
  - 被呼叫: build_external_export_document
- `build_external_export_document` @ `grafana_utils/dashboards/transformer.py:709`（上游 0 / 下游 11）

  - 呼叫: allocate_input_mapping, build_input_definitions, build_preserved_web_import_document, build_requires_block, collect_datasource_refs, collect_panel_types, ensure_datasource_template_variable, prepare_templating_for_external_import, replace_datasource_refs_in_dashboard, resolve_datasource_ref ...

## `grafana_utils/dashboards/variable_inspection.py`

- `resolve_dashboard_uid` @ `grafana_utils/dashboards/variable_inspection.py:15`（上游 1 / 下游 0）
  - 被呼叫: inspect_dashboard_variables_with_client
- `inspect_dashboard_variables_with_client` @ `grafana_utils/dashboards/variable_inspection.py:38`（上游 0 / 下游 3）

  - 呼叫: apply_vars_query_overrides, build_dashboard_variable_document, resolve_dashboard_uid
- `build_dashboard_variable_document` @ `grafana_utils/dashboards/variable_inspection.py:62`（上游 1 / 下游 1）

  - 呼叫: extract_dashboard_variables
  - 被呼叫: inspect_dashboard_variables_with_client
- `extract_dashboard_variables` @ `grafana_utils/dashboards/variable_inspection.py:77`（上游 1 / 下游 3）

  - 呼叫: _format_compact_value, _format_current_value, _normalize_options
  - 被呼叫: build_dashboard_variable_document
- `apply_vars_query_overrides` @ `grafana_utils/dashboards/variable_inspection.py:114`（上游 1 / 下游 1）

  - 呼叫: parse_vars_query
  - 被呼叫: inspect_dashboard_variables_with_client
- `parse_vars_query` @ `grafana_utils/dashboards/variable_inspection.py:129`（上游 1 / 下游 0）
  - 被呼叫: apply_vars_query_overrides
- `render_dashboard_variable_document` @ `grafana_utils/dashboards/variable_inspection.py:149`（上游 0 / 下游 2）

  - 呼叫: _render_simple_table, _summarize_options
- `_normalize_options` @ `grafana_utils/dashboards/variable_inspection.py:198`（上游 1 / 下游 1）

  - 呼叫: _format_option_value
  - 被呼叫: extract_dashboard_variables
- `_format_option_value` @ `grafana_utils/dashboards/variable_inspection.py:210`（上游 1 / 下游 1）

  - 呼叫: _format_compact_value
  - 被呼叫: _normalize_options
- `_format_current_value` @ `grafana_utils/dashboards/variable_inspection.py:220`（上游 1 / 下游 1）

  - 呼叫: _format_compact_value
  - 被呼叫: extract_dashboard_variables
- `_format_compact_value` @ `grafana_utils/dashboards/variable_inspection.py:231`（上游 3 / 下游 0）
  - 被呼叫: _format_current_value, _format_option_value, extract_dashboard_variables
- `_summarize_options` @ `grafana_utils/dashboards/variable_inspection.py:254`（上游 1 / 下游 0）
  - 被呼叫: render_dashboard_variable_document
- `_render_simple_table` @ `grafana_utils/dashboards/variable_inspection.py:267`（上游 1 / 下游 1）

  - 呼叫: format_row
  - 被呼叫: render_dashboard_variable_document
- `_render_simple_table.format_row` @ `grafana_utils/dashboards/variable_inspection.py:278`（上游 1 / 下游 0）
  - 被呼叫: _render_simple_table

## `grafana_utils/datasource/__init__.py`

- 無可辨識函式。

## `grafana_utils/datasource/live_mutation.py`

- `_normalize_string` @ `grafana_utils/datasource/live_mutation.py:32`（上游 5 / 下游 0）
  - 被呼叫: _normalize_bool, build_datasource_identity_lookups, delete_datasource, normalize_add_spec, resolve_datasource_target
- `_normalize_bool` @ `grafana_utils/datasource/live_mutation.py:39`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: normalize_add_spec
- `_copy_json_object` @ `grafana_utils/datasource/live_mutation.py:46`（上游 1 / 下游 0）
  - 被呼叫: normalize_add_spec
- `build_datasource_identity_lookups` @ `grafana_utils/datasource/live_mutation.py:55`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: resolve_datasource_target
- `resolve_datasource_target` @ `grafana_utils/datasource/live_mutation.py:69`（上游 2 / 下游 2）

  - 呼叫: _normalize_string, build_datasource_identity_lookups
  - 被呼叫: plan_add_datasource, plan_delete_datasource
- `normalize_add_spec` @ `grafana_utils/datasource/live_mutation.py:107`（上游 1 / 下游 3）

  - 呼叫: _copy_json_object, _normalize_bool, _normalize_string
  - 被呼叫: build_add_payload
- `build_add_payload` @ `grafana_utils/datasource/live_mutation.py:162`（上游 1 / 下游 1）

  - 呼叫: normalize_add_spec
  - 被呼叫: plan_add_datasource
- `plan_add_datasource` @ `grafana_utils/datasource/live_mutation.py:176`（上游 1 / 下游 2）

  - 呼叫: build_add_payload, resolve_datasource_target
  - 被呼叫: add_datasource
- `add_datasource` @ `grafana_utils/datasource/live_mutation.py:195`（上游 0 / 下游 1）

  - 呼叫: plan_add_datasource
- `plan_delete_datasource` @ `grafana_utils/datasource/live_mutation.py:222`（上游 1 / 下游 1）

  - 呼叫: resolve_datasource_target
  - 被呼叫: delete_datasource
- `delete_datasource` @ `grafana_utils/datasource/live_mutation.py:237`（上游 0 / 下游 2）

  - 呼叫: _normalize_string, plan_delete_datasource

## `grafana_utils/datasource/live_mutation_render.py`

- `_render_rows` @ `grafana_utils/datasource/live_mutation_render.py:16`（上游 1 / 下游 1）

  - 呼叫: render_row
  - 被呼叫: render_live_mutation_dry_run_table
- `_render_rows.render_row` @ `grafana_utils/datasource/live_mutation_render.py:25`（上游 1 / 下游 0）
  - 被呼叫: _render_rows
- `build_live_mutation_dry_run_record` @ `grafana_utils/datasource/live_mutation_render.py:38`（上游 0 / 下游 0）
- `render_live_mutation_dry_run_table` @ `grafana_utils/datasource/live_mutation_render.py:57`（上游 0 / 下游 1）

  - 呼叫: _render_rows
- `render_live_mutation_dry_run_json` @ `grafana_utils/datasource/live_mutation_render.py:73`（上游 0 / 下游 0）

## `grafana_utils/datasource/live_mutation_render_safe.py`

- `validate_columns` @ `grafana_utils/datasource/live_mutation_render_safe.py:18`（上游 1 / 下游 0）
  - 被呼叫: render_live_mutation_dry_run_table
- `build_live_mutation_dry_run_record` @ `grafana_utils/datasource/live_mutation_render_safe.py:30`（上游 0 / 下游 0）
- `render_live_mutation_dry_run_table` @ `grafana_utils/datasource/live_mutation_render_safe.py:49`（上游 0 / 下游 2）

  - 呼叫: render_row, validate_columns
- `render_live_mutation_dry_run_table.render_row` @ `grafana_utils/datasource/live_mutation_render_safe.py:65`（上游 1 / 下游 0）
  - 被呼叫: render_live_mutation_dry_run_table
- `render_live_mutation_dry_run_json` @ `grafana_utils/datasource/live_mutation_render_safe.py:81`（上游 0 / 下游 0）

## `grafana_utils/datasource/live_mutation_safe.py`

- `_normalize_string` @ `grafana_utils/datasource/live_mutation_safe.py:29`（上游 5 / 下游 0）
  - 被呼叫: _normalize_bool, build_datasource_identity_lookups, delete_datasource, normalize_add_spec, resolve_datasource_target
- `_normalize_bool` @ `grafana_utils/datasource/live_mutation_safe.py:36`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: normalize_add_spec
- `_copy_optional_json_object` @ `grafana_utils/datasource/live_mutation_safe.py:43`（上游 1 / 下游 0）
  - 被呼叫: normalize_add_spec
- `build_datasource_identity_lookups` @ `grafana_utils/datasource/live_mutation_safe.py:52`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: resolve_datasource_target
- `resolve_datasource_target` @ `grafana_utils/datasource/live_mutation_safe.py:66`（上游 2 / 下游 2）

  - 呼叫: _normalize_string, build_datasource_identity_lookups
  - 被呼叫: plan_add_datasource, plan_delete_datasource
- `normalize_add_spec` @ `grafana_utils/datasource/live_mutation_safe.py:103`（上游 1 / 下游 3）

  - 呼叫: _copy_optional_json_object, _normalize_bool, _normalize_string
  - 被呼叫: build_add_payload
- `build_add_payload` @ `grafana_utils/datasource/live_mutation_safe.py:159`（上游 1 / 下游 1）

  - 呼叫: normalize_add_spec
  - 被呼叫: plan_add_datasource
- `determine_add_action` @ `grafana_utils/datasource/live_mutation_safe.py:169`（上游 1 / 下游 0）
  - 被呼叫: plan_add_datasource
- `determine_delete_action` @ `grafana_utils/datasource/live_mutation_safe.py:186`（上游 1 / 下游 0）
  - 被呼叫: plan_delete_datasource
- `plan_add_datasource` @ `grafana_utils/datasource/live_mutation_safe.py:201`（上游 1 / 下游 3）

  - 呼叫: build_add_payload, determine_add_action, resolve_datasource_target
  - 被呼叫: add_datasource
- `add_datasource` @ `grafana_utils/datasource/live_mutation_safe.py:221`（上游 0 / 下游 1）

  - 呼叫: plan_add_datasource
- `plan_delete_datasource` @ `grafana_utils/datasource/live_mutation_safe.py:251`（上游 1 / 下游 2）

  - 呼叫: determine_delete_action, resolve_datasource_target
  - 被呼叫: delete_datasource
- `delete_datasource` @ `grafana_utils/datasource/live_mutation_safe.py:261`（上游 0 / 下游 2）

  - 呼叫: _normalize_string, plan_delete_datasource

## `grafana_utils/datasource/parser.py`

- `add_list_cli_args` @ `grafana_utils/datasource/parser.py:148`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_export_cli_args` @ `grafana_utils/datasource/parser.py:201`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_import_cli_args` @ `grafana_utils/datasource/parser.py:241`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_diff_cli_args` @ `grafana_utils/datasource/parser.py:358`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `parse_bool_choice` @ `grafana_utils/datasource/parser.py:371`（上游 0 / 下游 0）
- `add_add_cli_args` @ `grafana_utils/datasource/parser.py:385`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_modify_cli_args` @ `grafana_utils/datasource/parser.py:511`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_delete_cli_args` @ `grafana_utils/datasource/parser.py:630`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `build_parser` @ `grafana_utils/datasource/parser.py:673`（上游 0 / 下游 7）

  - 呼叫: add_add_cli_args, add_delete_cli_args, add_diff_cli_args, add_export_cli_args, add_import_cli_args, add_list_cli_args, add_modify_cli_args

## `grafana_utils/datasource/workflows.py`

- `build_client` @ `grafana_utils/datasource/workflows.py:52`（上游 8 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org, add_datasource, delete_datasource, diff_datasources, export_datasources, import_datasources, list_datasources, modify_datasource
- `build_export_index` @ `grafana_utils/datasource/workflows.py:57`（上游 1 / 下游 0）
  - 被呼叫: export_datasources
- `build_export_metadata` @ `grafana_utils/datasource/workflows.py:77`（上游 1 / 下游 0）
  - 被呼叫: export_datasources
- `build_all_orgs_export_index` @ `grafana_utils/datasource/workflows.py:91`（上游 1 / 下游 0）
  - 被呼叫: export_datasources
- `build_all_orgs_export_metadata` @ `grafana_utils/datasource/workflows.py:102`（上游 1 / 下游 0）
  - 被呼叫: export_datasources
- `build_export_records` @ `grafana_utils/datasource/workflows.py:116`（上游 1 / 下游 1）

  - 呼叫: list_datasources
  - 被呼叫: export_datasources
- `build_all_orgs_output_dir` @ `grafana_utils/datasource/workflows.py:125`（上游 1 / 下游 0）
  - 被呼叫: export_datasources
- `fetch_datasource_by_uid_if_exists` @ `grafana_utils/datasource/workflows.py:132`（上游 1 / 下游 1）

  - 呼叫: exporter_api_error_type
  - 被呼叫: plan_modify_datasource
- `exporter_api_error_type` @ `grafana_utils/datasource/workflows.py:150`（上游 1 / 下游 0）
  - 被呼叫: fetch_datasource_by_uid_if_exists
- `load_json_document` @ `grafana_utils/datasource/workflows.py:157`（上游 1 / 下游 0）
  - 被呼叫: load_import_bundle
- `load_json_object_argument` @ `grafana_utils/datasource/workflows.py:167`（上游 2 / 下游 0）
  - 被呼叫: build_add_datasource_spec, build_modify_datasource_updates
- `merge_json_object_fields` @ `grafana_utils/datasource/workflows.py:180`（上游 2 / 下游 0）
  - 被呼叫: build_add_datasource_spec, build_modify_datasource_updates
- `parse_http_header_arguments` @ `grafana_utils/datasource/workflows.py:195`（上游 2 / 下游 0）
  - 被呼叫: build_add_datasource_spec, build_modify_datasource_updates
- `load_import_bundle` @ `grafana_utils/datasource/workflows.py:217`（上游 2 / 下游 1）

  - 呼叫: load_json_document
  - 被呼叫: _resolve_multi_org_import_targets, _run_import_datasources_for_single_org
- `resolve_export_org_id` @ `grafana_utils/datasource/workflows.py:284`（上游 2 / 下游 0）
  - 被呼叫: _resolve_multi_org_import_targets, validate_export_org_match
- `resolve_export_org_name` @ `grafana_utils/datasource/workflows.py:309`（上游 1 / 下游 0）
  - 被呼叫: _resolve_multi_org_import_targets
- `_normalize_org_id` @ `grafana_utils/datasource/workflows.py:334`（上游 3 / 下游 0）
  - 被呼叫: _resolve_existing_orgs_by_id, export_datasources, list_datasources
- `_clone_import_args` @ `grafana_utils/datasource/workflows.py:345`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_by_export_org
- `_resolve_existing_orgs_by_id` @ `grafana_utils/datasource/workflows.py:352`（上游 1 / 下游 1）

  - 呼叫: _normalize_org_id
  - 被呼叫: _resolve_multi_org_import_targets
- `_resolve_created_org_id` @ `grafana_utils/datasource/workflows.py:362`（上游 1 / 下游 0）
  - 被呼叫: _resolve_multi_org_import_targets
- `create_organization` @ `grafana_utils/datasource/workflows.py:375`（上游 1 / 下游 0）
  - 被呼叫: _resolve_multi_org_import_targets
- `_discover_org_export_dirs` @ `grafana_utils/datasource/workflows.py:380`（上游 1 / 下游 0）
  - 被呼叫: _resolve_multi_org_import_targets
- `build_effective_import_client` @ `grafana_utils/datasource/workflows.py:411`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org
- `validate_export_org_match` @ `grafana_utils/datasource/workflows.py:425`（上游 1 / 下游 1）

  - 呼叫: resolve_export_org_id
  - 被呼叫: _run_import_datasources_for_single_org
- `build_existing_datasource_lookups` @ `grafana_utils/datasource/workflows.py:448`（上游 1 / 下游 1）

  - 呼叫: list_datasources
  - 被呼叫: _run_import_datasources_for_single_org
- `resolve_datasource_match` @ `grafana_utils/datasource/workflows.py:462`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org
- `determine_import_mode` @ `grafana_utils/datasource/workflows.py:481`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org
- `determine_datasource_action` @ `grafana_utils/datasource/workflows.py:490`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org
- `build_import_payload` @ `grafana_utils/datasource/workflows.py:521`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org
- `parse_import_dry_run_columns` @ `grafana_utils/datasource/workflows.py:540`（上游 0 / 下游 0）
- `render_import_dry_run_table` @ `grafana_utils/datasource/workflows.py:568`（上游 1 / 下游 1）

  - 呼叫: render_row
  - 被呼叫: _run_import_datasources_for_single_org
- `render_import_dry_run_table.render_row` @ `grafana_utils/datasource/workflows.py:588`（上游 1 / 下游 0）
  - 被呼叫: render_import_dry_run_table
- `render_import_dry_run_json` @ `grafana_utils/datasource/workflows.py:604`（上游 1 / 下游 0）
  - 被呼叫: _run_import_datasources_for_single_org
- `render_data_source_csv` @ `grafana_utils/datasource/workflows.py:647`（上游 1 / 下游 0）
  - 被呼叫: list_datasources
- `render_data_source_json` @ `grafana_utils/datasource/workflows.py:659`（上游 1 / 下游 0）
  - 被呼叫: list_datasources
- `build_add_datasource_spec` @ `grafana_utils/datasource/workflows.py:668`（上游 1 / 下游 3）

  - 呼叫: load_json_object_argument, merge_json_object_fields, parse_http_header_arguments
  - 被呼叫: add_datasource
- `build_modify_datasource_updates` @ `grafana_utils/datasource/workflows.py:736`（上游 1 / 下游 3）

  - 呼叫: load_json_object_argument, merge_json_object_fields, parse_http_header_arguments
  - 被呼叫: modify_datasource
- `split_live_add_supported_spec` @ `grafana_utils/datasource/workflows.py:798`（上游 1 / 下游 0）
  - 被呼叫: add_datasource
- `build_modify_datasource_payload` @ `grafana_utils/datasource/workflows.py:810`（上游 1 / 下游 0）
  - 被呼叫: plan_modify_datasource
- `plan_modify_datasource` @ `grafana_utils/datasource/workflows.py:850`（上游 1 / 下游 2）

  - 呼叫: build_modify_datasource_payload, fetch_datasource_by_uid_if_exists
  - 被呼叫: modify_datasource
- `render_modify_dry_run_json` @ `grafana_utils/datasource/workflows.py:869`（上游 1 / 下游 0）
  - 被呼叫: modify_datasource
- `_validate_live_mutation_dry_run_args` @ `grafana_utils/datasource/workflows.py:889`（上游 3 / 下游 0）
  - 被呼叫: add_datasource, delete_datasource, modify_datasource
- `add_datasource` @ `grafana_utils/datasource/workflows.py:908`（上游 1 / 下游 4）

  - 呼叫: _validate_live_mutation_dry_run_args, build_add_datasource_spec, build_client, split_live_add_supported_spec
  - 被呼叫: dispatch_datasource_command
- `modify_datasource` @ `grafana_utils/datasource/workflows.py:966`（上游 1 / 下游 5）

  - 呼叫: _validate_live_mutation_dry_run_args, build_client, build_modify_datasource_updates, plan_modify_datasource, render_modify_dry_run_json
  - 被呼叫: dispatch_datasource_command
- `delete_datasource` @ `grafana_utils/datasource/workflows.py:1030`（上游 1 / 下游 2）

  - 呼叫: _validate_live_mutation_dry_run_args, build_client
  - 被呼叫: dispatch_datasource_command
- `list_datasources` @ `grafana_utils/datasource/workflows.py:1080`（上游 3 / 下游 4）

  - 呼叫: _normalize_org_id, build_client, render_data_source_csv, render_data_source_json
  - 被呼叫: build_existing_datasource_lookups, build_export_records, dispatch_datasource_command
- `export_datasources` @ `grafana_utils/datasource/workflows.py:1142`（上游 1 / 下游 8）

  - 呼叫: _normalize_org_id, build_all_orgs_export_index, build_all_orgs_export_metadata, build_all_orgs_output_dir, build_client, build_export_index, build_export_metadata, build_export_records
  - 被呼叫: dispatch_datasource_command
- `_serialize_datasource_diff_record` @ `grafana_utils/datasource/workflows.py:1272`（上游 1 / 下游 0）
  - 被呼叫: _print_datasource_unified_diff
- `_print_datasource_unified_diff` @ `grafana_utils/datasource/workflows.py:1279`（上游 1 / 下游 1）

  - 呼叫: _serialize_datasource_diff_record
  - 被呼叫: diff_datasources
- `diff_datasources` @ `grafana_utils/datasource/workflows.py:1294`（上游 1 / 下游 2）

  - 呼叫: _print_datasource_unified_diff, build_client
  - 被呼叫: dispatch_datasource_command
- `_resolve_multi_org_import_targets` @ `grafana_utils/datasource/workflows.py:1347`（上游 1 / 下游 7）

  - 呼叫: _discover_org_export_dirs, _resolve_created_org_id, _resolve_existing_orgs_by_id, create_organization, load_import_bundle, resolve_export_org_id, resolve_export_org_name
  - 被呼叫: _run_import_datasources_by_export_org
- `_render_routed_datasource_import_table` @ `grafana_utils/datasource/workflows.py:1449`（上游 1 / 下游 1）

  - 呼叫: render_row
  - 被呼叫: _run_import_datasources_by_export_org
- `_render_routed_datasource_import_table.render_row` @ `grafana_utils/datasource/workflows.py:1473`（上游 1 / 下游 0）
  - 被呼叫: _render_routed_datasource_import_table
- `_run_import_datasources_by_export_org` @ `grafana_utils/datasource/workflows.py:1494`（上游 1 / 下游 4）

  - 呼叫: _clone_import_args, _render_routed_datasource_import_table, _resolve_multi_org_import_targets, _run_import_datasources_for_single_org
  - 被呼叫: import_datasources
- `_run_import_datasources_for_single_org` @ `grafana_utils/datasource/workflows.py:1613`（上游 2 / 下游 11）

  - 呼叫: build_client, build_effective_import_client, build_existing_datasource_lookups, build_import_payload, determine_datasource_action, determine_import_mode, load_import_bundle, render_import_dry_run_json, render_import_dry_run_table, resolve_datasource_match ...
  - 被呼叫: _run_import_datasources_by_export_org, import_datasources
- `import_datasources` @ `grafana_utils/datasource/workflows.py:1746`（上游 1 / 下游 3）

  - 呼叫: _run_import_datasources_by_export_org, _run_import_datasources_for_single_org, build_client
  - 被呼叫: dispatch_datasource_command
- `dispatch_datasource_command` @ `grafana_utils/datasource/workflows.py:1757`（上游 0 / 下游 7）

  - 呼叫: add_datasource, delete_datasource, diff_datasources, export_datasources, import_datasources, list_datasources, modify_datasource

## `grafana_utils/datasource_cli.py`

- `_normalize_output_format_args` @ `grafana_utils/datasource_cli.py:97`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `_parse_import_output_columns` @ `grafana_utils/datasource_cli.py:132`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `_validate_datasource_org_routing_args` @ `grafana_utils/datasource_cli.py:149`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `parse_args` @ `grafana_utils/datasource_cli.py:175`（上游 1 / 下游 3）

  - 呼叫: _normalize_output_format_args, _parse_import_output_columns, _validate_datasource_org_routing_args
  - 被呼叫: main
- `_sync_facade_overrides` @ `grafana_utils/datasource_cli.py:195`（上游 8 / 下游 0）
  - 被呼叫: add_datasource, delete_datasource, diff_datasources, dispatch_datasource_command, export_datasources, import_datasources, list_datasources, modify_datasource
- `list_datasources` @ `grafana_utils/datasource_cli.py:200`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `export_datasources` @ `grafana_utils/datasource_cli.py:210`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `import_datasources` @ `grafana_utils/datasource_cli.py:220`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `diff_datasources` @ `grafana_utils/datasource_cli.py:230`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `add_datasource` @ `grafana_utils/datasource_cli.py:240`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `delete_datasource` @ `grafana_utils/datasource_cli.py:250`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `modify_datasource` @ `grafana_utils/datasource_cli.py:260`（上游 0 / 下游 1）

  - 呼叫: _sync_facade_overrides
- `dispatch_datasource_command` @ `grafana_utils/datasource_cli.py:270`（上游 1 / 下游 1）

  - 呼叫: _sync_facade_overrides
  - 被呼叫: main
- `main` @ `grafana_utils/datasource_cli.py:280`（上游 0 / 下游 2）

  - 呼叫: dispatch_datasource_command, parse_args

## `grafana_utils/datasource_contract.py`

- `normalize_datasource_string` @ `grafana_utils/datasource_contract.py:32`（上游 2 / 下游 0）
  - 被呼叫: normalize_datasource_bool, normalize_datasource_record
- `normalize_datasource_bool` @ `grafana_utils/datasource_contract.py:41`（上游 1 / 下游 1）

  - 呼叫: normalize_datasource_string
  - 被呼叫: normalize_datasource_record
- `normalize_datasource_record` @ `grafana_utils/datasource_contract.py:47`（上游 0 / 下游 2）

  - 呼叫: normalize_datasource_bool, normalize_datasource_string
- `validate_datasource_contract_record` @ `grafana_utils/datasource_contract.py:67`（上游 0 / 下游 0）

## `grafana_utils/datasource_diff.py`

- `load_json_document` @ `grafana_utils/datasource_diff.py:31`（上游 1 / 下游 0）
  - 被呼叫: load_datasource_diff_bundle
- `load_datasource_diff_bundle` @ `grafana_utils/datasource_diff.py:41`（上游 0 / 下游 1）

  - 呼叫: load_json_document
- `build_live_datasource_diff_records` @ `grafana_utils/datasource_diff.py:117`（上游 0 / 下游 0）
- `resolve_datasource_identity` @ `grafana_utils/datasource_diff.py:132`（上游 1 / 下游 0）
  - 被呼叫: build_datasource_diff_item
- `_index_records` @ `grafana_utils/datasource_diff.py:143`（上游 1 / 下游 0）
  - 被呼叫: compare_datasource_inventory
- `_resolve_live_match` @ `grafana_utils/datasource_diff.py:159`（上游 1 / 下游 0）
  - 被呼叫: compare_datasource_inventory
- `_resolve_compare_fields` @ `grafana_utils/datasource_diff.py:193`（上游 1 / 下游 0）
  - 被呼叫: build_datasource_diff_item
- `build_datasource_diff_item` @ `grafana_utils/datasource_diff.py:209`（上游 1 / 下游 2）

  - 呼叫: _resolve_compare_fields, resolve_datasource_identity
  - 被呼叫: compare_datasource_inventory
- `compare_datasource_inventory` @ `grafana_utils/datasource_diff.py:241`（上游 1 / 下游 3）

  - 呼叫: _index_records, _resolve_live_match, build_datasource_diff_item
  - 被呼叫: compare_datasource_bundle_to_live
- `compare_datasource_bundle_to_live` @ `grafana_utils/datasource_diff.py:311`（上游 0 / 下游 1）

  - 呼叫: compare_datasource_inventory

## `grafana_utils/datasource_secret_provider_workbench.py`

- `_normalize_text` @ `grafana_utils/datasource_secret_provider_workbench.py:44`（上游 2 / 下游 0）
  - 被呼叫: build_provider_plan, parse_provider_reference
- `parse_provider_reference` @ `grafana_utils/datasource_secret_provider_workbench.py:51`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: collect_provider_references
- `collect_provider_references` @ `grafana_utils/datasource_secret_provider_workbench.py:89`（上游 1 / 下游 1）

  - 呼叫: parse_provider_reference
  - 被呼叫: build_provider_plan
- `build_provider_plan` @ `grafana_utils/datasource_secret_provider_workbench.py:101`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, collect_provider_references
- `summarize_provider_plan` @ `grafana_utils/datasource_secret_provider_workbench.py:129`（上游 0 / 下游 0）
- `iter_provider_names` @ `grafana_utils/datasource_secret_provider_workbench.py:153`（上游 0 / 下游 0）

## `grafana_utils/datasource_secret_workbench.py`

- `build_placeholder_token` @ `grafana_utils/datasource_secret_workbench.py:44`（上游 0 / 下游 0）
- `parse_secret_placeholder` @ `grafana_utils/datasource_secret_workbench.py:59`（上游 1 / 下游 0）
  - 被呼叫: collect_secret_placeholders
- `collect_secret_placeholders` @ `grafana_utils/datasource_secret_workbench.py:87`（上游 1 / 下游 1）

  - 呼叫: parse_secret_placeholder
  - 被呼叫: build_datasource_secret_plan
- `resolve_secret_placeholders` @ `grafana_utils/datasource_secret_workbench.py:101`（上游 1 / 下游 0）
  - 被呼叫: build_datasource_secret_plan
- `iter_secret_placeholder_names` @ `grafana_utils/datasource_secret_workbench.py:122`（上游 1 / 下游 0）
  - 被呼叫: summarize_secret_plan
- `build_datasource_secret_plan` @ `grafana_utils/datasource_secret_workbench.py:132`（上游 0 / 下游 2）

  - 呼叫: collect_secret_placeholders, resolve_secret_placeholders
- `summarize_secret_plan` @ `grafana_utils/datasource_secret_workbench.py:165`（上游 0 / 下游 1）

  - 呼叫: iter_secret_placeholder_names

## `grafana_utils/gitops_sync.py`

- `_normalize_string` @ `grafana_utils/gitops_sync.py:70`（上游 5 / 下游 0）
  - 被呼叫: _extract_identity, _extract_title, _normalize_string_list, mark_plan_reviewed, normalize_resource_spec
- `_copy_mapping` @ `grafana_utils/gitops_sync.py:77`（上游 4 / 下游 0）
  - 被呼叫: _body_subset_for_comparison, _normalize_body, build_sync_source_bundle_document, render_sync_source_bundle_text
- `_normalize_string_list` @ `grafana_utils/gitops_sync.py:86`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: normalize_resource_spec
- `_extract_identity` @ `grafana_utils/gitops_sync.py:101`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: normalize_resource_spec
- `_extract_title` @ `grafana_utils/gitops_sync.py:110`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: normalize_resource_spec
- `_normalize_body` @ `grafana_utils/gitops_sync.py:119`（上游 1 / 下游 1）

  - 呼叫: _copy_mapping
  - 被呼叫: normalize_resource_spec
- `normalize_resource_spec` @ `grafana_utils/gitops_sync.py:127`（上游 2 / 下游 5）

  - 呼叫: _extract_identity, _extract_title, _normalize_body, _normalize_string, _normalize_string_list
  - 被呼叫: _normalize_live_specs, build_sync_plan
- `build_resource_index` @ `grafana_utils/gitops_sync.py:167`（上游 1 / 下游 0）
  - 被呼叫: build_sync_plan
- `_body_subset_for_comparison` @ `grafana_utils/gitops_sync.py:180`（上游 1 / 下游 1）

  - 呼叫: _copy_mapping
  - 被呼叫: _compare_body
- `_compare_body` @ `grafana_utils/gitops_sync.py:192`（上游 1 / 下游 1）

  - 呼叫: _body_subset_for_comparison
  - 被呼叫: _build_operation
- `_normalize_live_specs` @ `grafana_utils/gitops_sync.py:204`（上游 1 / 下游 1）

  - 呼叫: normalize_resource_spec
  - 被呼叫: build_sync_plan
- `_build_operation` @ `grafana_utils/gitops_sync.py:212`（上游 1 / 下游 1）

  - 呼叫: _compare_body
  - 被呼叫: build_sync_plan
- `_build_prune_operation` @ `grafana_utils/gitops_sync.py:256`（上游 1 / 下游 0）
  - 被呼叫: build_sync_plan
- `summarize_alert_operations` @ `grafana_utils/gitops_sync.py:285`（上游 1 / 下游 0）
  - 被呼叫: plan_to_document
- `summarize_operations` @ `grafana_utils/gitops_sync.py:317`（上游 1 / 下游 0）
  - 被呼叫: build_sync_plan
- `build_sync_plan` @ `grafana_utils/gitops_sync.py:340`（上游 0 / 下游 6）

  - 呼叫: _build_operation, _build_prune_operation, _normalize_live_specs, build_resource_index, normalize_resource_spec, summarize_operations
- `mark_plan_reviewed` @ `grafana_utils/gitops_sync.py:378`（上游 0 / 下游 1）

  - 呼叫: _normalize_string
- `build_apply_intent` @ `grafana_utils/gitops_sync.py:397`（上游 0 / 下游 0）
- `plan_to_document` @ `grafana_utils/gitops_sync.py:426`（上游 0 / 下游 1）

  - 呼叫: summarize_alert_operations
- `build_sync_source_bundle_document` @ `grafana_utils/gitops_sync.py:468`（上游 0 / 下游 1）

  - 呼叫: _copy_mapping
- `render_sync_source_bundle_text` @ `grafana_utils/gitops_sync.py:508`（上游 0 / 下游 1）

  - 呼叫: _copy_mapping

## `grafana_utils/http_transport.py`

- `HttpTransportApiError.__init__` @ `grafana_utils/http_transport.py:23`（上游 0 / 下游 0）
- `JsonHttpTransport.request_json` @ `grafana_utils/http_transport.py:41`（上游 0 / 下游 0）
- `BaseJsonHttpTransport.__init__` @ `grafana_utils/http_transport.py:62`（上游 0 / 下游 0）
- `BaseJsonHttpTransport.verify_config` @ `grafana_utils/http_transport.py:84`（上游 2 / 下游 0）
  - 被呼叫: __init__, request_json
- `BaseJsonHttpTransport.build_url` @ `grafana_utils/http_transport.py:93`（上游 2 / 下游 0）
  - 被呼叫: request_json, request_json
- `BaseJsonHttpTransport.decode_json_response` @ `grafana_utils/http_transport.py:107`（上游 2 / 下游 0）
  - 被呼叫: request_json, request_json
- `http2_is_available` @ `grafana_utils/http_transport.py:120`（上游 2 / 下游 0）
  - 被呼叫: __init__, build_json_http_transport
- `httpx_is_available` @ `grafana_utils/http_transport.py:129`（上游 1 / 下游 0）
  - 被呼叫: build_json_http_transport
- `RequestsJsonHttpTransport.__init__` @ `grafana_utils/http_transport.py:141`（上游 0 / 下游 0）
- `RequestsJsonHttpTransport.request_json` @ `grafana_utils/http_transport.py:168`（上游 0 / 下游 3）

  - 呼叫: build_url, decode_json_response, verify_config
- `HttpxJsonHttpTransport.__init__` @ `grafana_utils/http_transport.py:207`（上游 0 / 下游 2）

  - 呼叫: verify_config, http2_is_available
- `HttpxJsonHttpTransport.request_json` @ `grafana_utils/http_transport.py:242`（上游 0 / 下游 2）

  - 呼叫: build_url, decode_json_response
- `build_json_http_transport` @ `grafana_utils/http_transport.py:276`（上游 0 / 下游 2）

  - 呼叫: http2_is_available, httpx_is_available

## `grafana_utils/roadmap_workbench.py`

- `list_workbench_sections` @ `grafana_utils/roadmap_workbench.py:167`（上游 0 / 下游 0）
- `list_workbench_tasks` @ `grafana_utils/roadmap_workbench.py:181`（上游 1 / 下游 0）
  - 被呼叫: iter_candidate_modules
- `build_workbench_index` @ `grafana_utils/roadmap_workbench.py:188`（上游 0 / 下游 0）
- `iter_candidate_modules` @ `grafana_utils/roadmap_workbench.py:200`（上游 0 / 下游 1）

  - 呼叫: list_workbench_tasks
- `_normalize_text` @ `grafana_utils/roadmap_workbench.py:215`（上游 12 / 下游 0）
  - 被呼叫: _build_dashboard_bundle_lookup, _build_datasource_bundle_lookup, _resolve_datasource_inventory, _resolve_query_datasource_record, build_dependency_graph_document, build_dependency_graph_governance_summary, build_preflight_check_document, build_promotion_plan_document, ensure_node, render_dependency_graph_governance_text ...
- `_build_dashboard_node_id` @ `grafana_utils/roadmap_workbench.py:223`（上游 1 / 下游 0）
  - 被呼叫: build_dependency_graph_document
- `_build_panel_node_id` @ `grafana_utils/roadmap_workbench.py:228`（上游 1 / 下游 0）
  - 被呼叫: build_dependency_graph_document
- `_build_datasource_node_id` @ `grafana_utils/roadmap_workbench.py:233`（上游 1 / 下游 0）
  - 被呼叫: build_dependency_graph_document
- `_resolve_datasource_inventory` @ `grafana_utils/roadmap_workbench.py:238`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: build_dependency_graph_document
- `_resolve_query_datasource_record` @ `grafana_utils/roadmap_workbench.py:257`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: build_dependency_graph_document
- `build_dependency_graph_document` @ `grafana_utils/roadmap_workbench.py:287`（上游 0 / 下游 8）

  - 呼叫: _build_dashboard_node_id, _build_datasource_node_id, _build_panel_node_id, _normalize_text, _resolve_datasource_inventory, _resolve_query_datasource_record, ensure_edge, ensure_node
- `build_dependency_graph_document.ensure_node` @ `grafana_utils/roadmap_workbench.py:302`（上游 1 / 下游 0）
  - 被呼叫: build_dependency_graph_document
- `build_dependency_graph_document.ensure_edge` @ `grafana_utils/roadmap_workbench.py:328`（上游 1 / 下游 0）
  - 被呼叫: build_dependency_graph_document
- `_escape_dot_string` @ `grafana_utils/roadmap_workbench.py:435`（上游 1 / 下游 0）
  - 被呼叫: render_dependency_graph_dot
- `render_dependency_graph_dot` @ `grafana_utils/roadmap_workbench.py:441`（上游 0 / 下游 1）

  - 呼叫: _escape_dot_string
- `build_dependency_graph_governance_summary` @ `grafana_utils/roadmap_workbench.py:491`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: render_dependency_graph_governance_text
- `render_dependency_graph_governance_text` @ `grafana_utils/roadmap_workbench.py:583`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, build_dependency_graph_governance_summary
- `_build_dashboard_bundle_lookup` @ `grafana_utils/roadmap_workbench.py:631`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: build_promotion_plan_document
- `_build_datasource_bundle_lookup` @ `grafana_utils/roadmap_workbench.py:646`（上游 1 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: build_promotion_plan_document
- `build_promotion_plan_document` @ `grafana_utils/roadmap_workbench.py:663`（上游 0 / 下游 3）

  - 呼叫: _build_dashboard_bundle_lookup, _build_datasource_bundle_lookup, _normalize_text
- `build_preflight_check_document` @ `grafana_utils/roadmap_workbench.py:764`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `render_promotion_plan_text` @ `grafana_utils/roadmap_workbench.py:867`（上游 0 / 下游 1）

  - 呼叫: _normalize_text
- `render_preflight_check_text` @ `grafana_utils/roadmap_workbench.py:907`（上游 0 / 下游 1）

  - 呼叫: _normalize_text

## `grafana_utils/sync_cli.py`

- `add_document_input_group` @ `grafana_utils/sync_cli.py:106`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_runtime_group` @ `grafana_utils/sync_cli.py:115`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_output_group` @ `grafana_utils/sync_cli.py:120`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `add_apply_control_group` @ `grafana_utils/sync_cli.py:125`（上游 1 / 下游 0）
  - 被呼叫: build_parser
- `build_parser` @ `grafana_utils/sync_cli.py:130`（上游 1 / 下游 4）

  - 呼叫: add_apply_control_group, add_document_input_group, add_output_group, add_runtime_group
  - 被呼叫: parse_args
- `load_json_document` @ `grafana_utils/sync_cli.py:370`（上游 10 / 下游 0）
  - 被呼叫: _load_alerting_bundle_section, _load_dashboard_bundle_sections, _load_optional_array_file, _load_optional_object_file, load_plan_document, run_assess_alerts, run_bundle_preflight, run_plan, run_preflight, run_summary
- `write_json_document` @ `grafana_utils/sync_cli.py:381`（上游 2 / 下游 0）
  - 被呼叫: emit_document, run_bundle
- `build_client` @ `grafana_utils/sync_cli.py:391`（上游 4 / 下游 0）
  - 被呼叫: run_apply, run_bundle_preflight, run_plan, run_preflight
- `_require_object` @ `grafana_utils/sync_cli.py:400`（上游 6 / 下游 0）
  - 被呼叫: _load_alerting_bundle_section, _load_dashboard_bundle_sections, _load_optional_object_file, load_plan_document, render_sync_summary_text, run_bundle_preflight
- `_require_resource_list` @ `grafana_utils/sync_cli.py:407`（上游 5 / 下游 0）
  - 被呼叫: _load_optional_array_file, run_assess_alerts, run_plan, run_preflight, run_summary
- `build_sync_summary_document` @ `grafana_utils/sync_cli.py:414`（上游 1 / 下游 0）
  - 被呼叫: run_summary
- `render_sync_summary_text` @ `grafana_utils/sync_cli.py:441`（上游 1 / 下游 1）

  - 呼叫: _require_object
  - 被呼叫: run_summary
- `_coerce_operation` @ `grafana_utils/sync_cli.py:461`（上游 1 / 下游 0）
  - 被呼叫: load_plan_document
- `load_plan_document` @ `grafana_utils/sync_cli.py:479`（上游 2 / 下游 3）

  - 呼叫: _coerce_operation, _require_object, load_json_document
  - 被呼叫: run_apply, run_review
- `emit_document` @ `grafana_utils/sync_cli.py:502`（上游 4 / 下游 1）

  - 呼叫: write_json_document
  - 被呼叫: run_apply, run_plan, run_review, run_summary
- `_normalize_string` @ `grafana_utils/sync_cli.py:509`（上游 11 / 下游 0）
  - 被呼叫: _apply_alert_operation, _apply_dashboard_operation, _apply_datasource_operation, _apply_folder_operation, _merge_availability, _normalize_dashboard_bundle_item, _normalize_datasource_bundle_item, _normalize_folder_bundle_item, _resolve_datasource_target, fetch_live_availability ...
- `_copy_mapping` @ `grafana_utils/sync_cli.py:519`（上游 8 / 下游 0）
  - 被呼叫: _apply_alert_operation, _apply_dashboard_operation, _apply_datasource_operation, _apply_folder_operation, _dashboard_body_from_export, _normalize_datasource_bundle_item, _normalize_folder_bundle_item, fetch_live_resource_specs
- `fetch_live_resource_specs` @ `grafana_utils/sync_cli.py:528`（上游 1 / 下游 2）

  - 呼叫: _copy_mapping, _normalize_string
  - 被呼叫: run_plan
- `run_plan` @ `grafana_utils/sync_cli.py:641`（上游 1 / 下游 5）

  - 呼叫: _require_resource_list, build_client, emit_document, fetch_live_resource_specs, load_json_document
  - 被呼叫: main
- `run_summary` @ `grafana_utils/sync_cli.py:675`（上游 1 / 下游 5）

  - 呼叫: _require_resource_list, build_sync_summary_document, emit_document, load_json_document, render_sync_summary_text
  - 被呼叫: main
- `run_review` @ `grafana_utils/sync_cli.py:693`（上游 1 / 下游 2）

  - 呼叫: emit_document, load_plan_document
  - 被呼叫: main
- `_load_optional_object_file` @ `grafana_utils/sync_cli.py:711`（上游 3 / 下游 2）

  - 呼叫: _require_object, load_json_document
  - 被呼叫: run_bundle, run_bundle_preflight, run_preflight
- `_merge_availability` @ `grafana_utils/sync_cli.py:718`（上游 2 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: run_bundle_preflight, run_preflight
- `fetch_live_availability` @ `grafana_utils/sync_cli.py:736`（上游 2 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: run_bundle_preflight, run_preflight
- `_emit_text_or_json` @ `grafana_utils/sync_cli.py:779`（上游 4 / 下游 0）
  - 被呼叫: run_assess_alerts, run_bundle, run_bundle_preflight, run_preflight
- `_load_optional_array_file` @ `grafana_utils/sync_cli.py:788`（上游 2 / 下游 2）

  - 呼叫: _require_resource_list, load_json_document
  - 被呼叫: _load_dashboard_bundle_sections, run_bundle
- `_discover_json_files` @ `grafana_utils/sync_cli.py:796`（上游 2 / 下游 0）
  - 被呼叫: _load_alerting_bundle_section, _load_dashboard_bundle_sections
- `_dashboard_body_from_export` @ `grafana_utils/sync_cli.py:806`（上游 1 / 下游 1）

  - 呼叫: _copy_mapping
  - 被呼叫: _normalize_dashboard_bundle_item
- `_normalize_dashboard_bundle_item` @ `grafana_utils/sync_cli.py:816`（上游 1 / 下游 2）

  - 呼叫: _dashboard_body_from_export, _normalize_string
  - 被呼叫: _load_dashboard_bundle_sections
- `_normalize_folder_bundle_item` @ `grafana_utils/sync_cli.py:832`（上游 1 / 下游 2）

  - 呼叫: _copy_mapping, _normalize_string
  - 被呼叫: _load_dashboard_bundle_sections
- `_normalize_datasource_bundle_item` @ `grafana_utils/sync_cli.py:853`（上游 2 / 下游 2）

  - 呼叫: _copy_mapping, _normalize_string
  - 被呼叫: _load_dashboard_bundle_sections, run_bundle
- `_classify_alert_export_path` @ `grafana_utils/sync_cli.py:878`（上游 1 / 下游 0）
  - 被呼叫: _load_alerting_bundle_section
- `_load_dashboard_bundle_sections` @ `grafana_utils/sync_cli.py:894`（上游 1 / 下游 7）

  - 呼叫: _discover_json_files, _load_optional_array_file, _normalize_dashboard_bundle_item, _normalize_datasource_bundle_item, _normalize_folder_bundle_item, _require_object, load_json_document
  - 被呼叫: run_bundle
- `_load_alerting_bundle_section` @ `grafana_utils/sync_cli.py:933`（上游 1 / 下游 4）

  - 呼叫: _classify_alert_export_path, _discover_json_files, _require_object, load_json_document
  - 被呼叫: run_bundle
- `run_bundle` @ `grafana_utils/sync_cli.py:982`（上游 1 / 下游 7）

  - 呼叫: _emit_text_or_json, _load_alerting_bundle_section, _load_dashboard_bundle_sections, _load_optional_array_file, _load_optional_object_file, _normalize_datasource_bundle_item, write_json_document
  - 被呼叫: main
- `run_preflight` @ `grafana_utils/sync_cli.py:1047`（上游 1 / 下游 7）

  - 呼叫: _emit_text_or_json, _load_optional_object_file, _merge_availability, _require_resource_list, build_client, fetch_live_availability, load_json_document
  - 被呼叫: main
- `run_assess_alerts` @ `grafana_utils/sync_cli.py:1075`（上游 1 / 下游 3）

  - 呼叫: _emit_text_or_json, _require_resource_list, load_json_document
  - 被呼叫: main
- `run_bundle_preflight` @ `grafana_utils/sync_cli.py:1094`（上游 1 / 下游 7）

  - 呼叫: _emit_text_or_json, _load_optional_object_file, _merge_availability, _require_object, build_client, fetch_live_availability, load_json_document
  - 被呼叫: main
- `_serialize_apply_intent` @ `grafana_utils/sync_cli.py:1130`（上游 1 / 下游 0）
  - 被呼叫: run_apply
- `_resolve_datasource_target` @ `grafana_utils/sync_cli.py:1153`（上游 1 / 下游 1）

  - 呼叫: _normalize_string
  - 被呼叫: _apply_datasource_operation
- `_apply_folder_operation` @ `grafana_utils/sync_cli.py:1169`（上游 1 / 下游 2）

  - 呼叫: _copy_mapping, _normalize_string
  - 被呼叫: execute_live_apply
- `_apply_dashboard_operation` @ `grafana_utils/sync_cli.py:1205`（上游 1 / 下游 2）

  - 呼叫: _copy_mapping, _normalize_string
  - 被呼叫: execute_live_apply
- `_apply_datasource_operation` @ `grafana_utils/sync_cli.py:1226`（上游 1 / 下游 3）

  - 呼叫: _copy_mapping, _normalize_string, _resolve_datasource_target
  - 被呼叫: execute_live_apply
- `_apply_alert_operation` @ `grafana_utils/sync_cli.py:1265`（上游 1 / 下游 2）

  - 呼叫: _copy_mapping, _normalize_string
  - 被呼叫: execute_live_apply
- `execute_live_apply` @ `grafana_utils/sync_cli.py:1301`（上游 1 / 下游 4）

  - 呼叫: _apply_alert_operation, _apply_dashboard_operation, _apply_datasource_operation, _apply_folder_operation
  - 被呼叫: run_apply
- `run_apply` @ `grafana_utils/sync_cli.py:1338`（上游 1 / 下游 5）

  - 呼叫: _serialize_apply_intent, build_client, emit_document, execute_live_apply, load_plan_document
  - 被呼叫: main
- `parse_args` @ `grafana_utils/sync_cli.py:1373`（上游 1 / 下游 1）

  - 呼叫: build_parser
  - 被呼叫: main
- `main` @ `grafana_utils/sync_cli.py:1378`（上游 0 / 下游 9）

  - 呼叫: parse_args, run_apply, run_assess_alerts, run_bundle, run_bundle_preflight, run_plan, run_preflight, run_review, run_summary

## `grafana_utils/sync_preflight_workbench.py`

- `_normalize_text` @ `grafana_utils/sync_preflight_workbench.py:34`（上游 6 / 下游 0）
  - 被呼叫: _build_datasource_checks, _collect_alert_contact_points, _collect_alert_datasource_names, _collect_alert_datasource_uids, _require_string_list, render_sync_preflight_text
- `_require_mapping` @ `grafana_utils/sync_preflight_workbench.py:44`（上游 4 / 下游 0）
  - 被呼叫: _build_alert_checks, _build_dashboard_checks, build_sync_preflight_document, render_sync_preflight_text
- `_require_string_list` @ `grafana_utils/sync_preflight_workbench.py:53`（上游 6 / 下游 1）

  - 呼叫: _normalize_text
  - 被呼叫: _build_alert_checks, _build_dashboard_checks, _build_datasource_checks, _collect_alert_contact_points, _collect_alert_datasource_names, _collect_alert_datasource_uids
- `_build_datasource_checks` @ `grafana_utils/sync_preflight_workbench.py:67`（上游 1 / 下游 2）

  - 呼叫: _normalize_text, _require_string_list
  - 被呼叫: build_sync_preflight_document
- `_build_dashboard_checks` @ `grafana_utils/sync_preflight_workbench.py:118`（上游 1 / 下游 2）

  - 呼叫: _require_mapping, _require_string_list
  - 被呼叫: build_sync_preflight_document
- `_is_builtin_alert_datasource_ref` @ `grafana_utils/sync_preflight_workbench.py:163`（上游 1 / 下游 0）
  - 被呼叫: _collect_alert_datasource_uids
- `_collect_alert_datasource_uids` @ `grafana_utils/sync_preflight_workbench.py:168`（上游 1 / 下游 3）

  - 呼叫: _is_builtin_alert_datasource_ref, _normalize_text, _require_string_list
  - 被呼叫: _build_alert_checks
- `_collect_alert_datasource_names` @ `grafana_utils/sync_preflight_workbench.py:192`（上游 1 / 下游 2）

  - 呼叫: _normalize_text, _require_string_list
  - 被呼叫: _build_alert_checks
- `_collect_alert_contact_points` @ `grafana_utils/sync_preflight_workbench.py:211`（上游 1 / 下游 2）

  - 呼叫: _normalize_text, _require_string_list
  - 被呼叫: _build_alert_checks
- `_build_alert_checks` @ `grafana_utils/sync_preflight_workbench.py:227`（上游 1 / 下游 5）

  - 呼叫: _collect_alert_contact_points, _collect_alert_datasource_names, _collect_alert_datasource_uids, _require_mapping, _require_string_list
  - 被呼叫: build_sync_preflight_document
- `build_sync_preflight_document` @ `grafana_utils/sync_preflight_workbench.py:301`（上游 0 / 下游 4）

  - 呼叫: _build_alert_checks, _build_dashboard_checks, _build_datasource_checks, _require_mapping
- `render_sync_preflight_text` @ `grafana_utils/sync_preflight_workbench.py:350`（上游 0 / 下游 2）

  - 呼叫: _normalize_text, _require_mapping

## `grafana_utils/unified_cli.py`

- `_print_dashboard_group_help` @ `grafana_utils/unified_cli.py:72`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `build_parser` @ `grafana_utils/unified_cli.py:89`（上游 1 / 下游 0）
  - 被呼叫: parse_args
- `parse_args` @ `grafana_utils/unified_cli.py:153`（上游 1 / 下游 2）

  - 呼叫: _print_dashboard_group_help, build_parser
  - 被呼叫: main
- `main` @ `grafana_utils/unified_cli.py:236`（上游 0 / 下游 1）

  - 呼叫: parse_args

