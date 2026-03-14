#!/usr/bin/env python3
"""Unwired datasource live add/delete API examples.

This file is intentionally not wired into the CLI yet.
It demonstrates how future wiring can call the new datasource live mutation
helpers directly from Python.
"""

from grafana_utils.clients.datasource_client import GrafanaDatasourceClient
from grafana_utils.datasource.live_mutation import (
    add_datasource,
    delete_datasource,
)
from grafana_utils.datasource.live_mutation_render import (
    build_live_mutation_dry_run_record,
    render_live_mutation_dry_run_json,
    render_live_mutation_dry_run_table,
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
        "isDefault": True,
        "jsonData": {"httpMethod": "POST"},
    }
    plan = add_datasource(client, spec, dry_run=True)
    record = build_live_mutation_dry_run_record("add", plan, spec=spec)
    print("# add dry-run table")
    for line in render_live_mutation_dry_run_table([record]):
        print(line)
    print("")
    print("# add dry-run json")
    print(render_live_mutation_dry_run_json([record]))


def example_add_live():
    client = build_client()
    spec = {
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "jsonData": {"httpMethod": "POST"},
    }
    result = add_datasource(client, spec, dry_run=False)
    print("# add live")
    print(result)


def example_delete_dry_run():
    client = build_client()
    plan = delete_datasource(client, uid="prom-main", dry_run=True)
    record = build_live_mutation_dry_run_record("delete", plan, uid="prom-main")
    print("# delete dry-run table")
    for line in render_live_mutation_dry_run_table([record]):
        print(line)
    print("")
    print("# delete dry-run json")
    print(render_live_mutation_dry_run_json([record]))


def example_delete_live():
    client = build_client()
    result = delete_datasource(client, uid="prom-main", dry_run=False)
    print("# delete live")
    print(result)


if __name__ == "__main__":
    print("This example is not wired into grafana-util CLI yet.")
    print("Call one example_* function manually after setting valid Grafana auth.")
