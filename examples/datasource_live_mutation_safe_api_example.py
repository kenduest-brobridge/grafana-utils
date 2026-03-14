#!/usr/bin/env python3
"""Unwired safe-draft datasource live add/delete dry-run examples.

This file is intentionally not wired into the grafana-util CLI yet.
It demonstrates how future wiring can call the safer draft live mutation
helpers directly from Python.
"""

from grafana_utils.clients.datasource_client import GrafanaDatasourceClient
from grafana_utils.datasource.live_mutation_render_safe import (
    build_live_mutation_dry_run_record,
    render_live_mutation_dry_run_json,
    render_live_mutation_dry_run_table,
)
from grafana_utils.datasource.live_mutation_safe import (
    add_datasource,
    delete_datasource,
)


def build_client():
    return GrafanaDatasourceClient(
        base_url="http://localhost:3000",
        headers={"Authorization": "Bearer REPLACE_ME"},
        timeout=30,
        verify_ssl=True,
    )


def example_add_dry_run():
    client = build_client()
    spec = {
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "jsonData": {"httpMethod": "POST"},
    }
    plan = add_datasource(client, spec, dry_run=True)
    record = build_live_mutation_dry_run_record("add", plan, spec=spec)
    print("# safe add dry-run table")
    for line in render_live_mutation_dry_run_table([record]):
        print(line)
    print("")
    print("# safe add dry-run json")
    print(render_live_mutation_dry_run_json([record]))


def example_delete_dry_run():
    client = build_client()
    plan = delete_datasource(client, uid="prom-main", dry_run=True)
    record = build_live_mutation_dry_run_record("delete", plan, uid="prom-main")
    print("# safe delete dry-run table")
    for line in render_live_mutation_dry_run_table([record]):
        print(line)
    print("")
    print("# safe delete dry-run json")
    print(render_live_mutation_dry_run_json([record]))


if __name__ == "__main__":
    print("This safe draft example is not wired into grafana-util CLI yet.")
    print("Call example_add_dry_run() or example_delete_dry_run() manually.")
