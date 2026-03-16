import unittest

from grafana_utils.dashboards.reference_models import (
    DatasourceReference,
    DashboardReference,
    DashboardQueryReference,
    collect_datasource_reference_index,
    dedupe_text_sequence,
)


class DashboardReferenceModelTests(unittest.TestCase):
    def test_collect_datasource_reference_index_deduplicates(self):
        references = [
            {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
            {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
            {"uid": "loki-main", "name": "Loki Main", "type": "loki"},
        ]
        index = collect_datasource_reference_index(references)
        self.assertEqual(len(index), 2)
        self.assertIn("prom-main", index)
        self.assertIn("loki-main", index)
        self.assertEqual(index["prom-main"].datasource_type, "prometheus")

    def test_dashboard_reference_from_mapping(self):
        record = DashboardReference.from_mapping(
            {
                "uid": "cpu-main",
                "title": "CPU Main",
                "folderPath": "Platform / Infra",
                "file": "dashboards/cpu.json",
            }
        )
        self.assertEqual(record.uid, "cpu-main")
        self.assertEqual(record.title, "CPU Main")
        self.assertEqual(record.folder_path, "Platform / Infra")

    def test_query_reference_uses_panel_fallback(self):
        reference = DashboardQueryReference.from_mappings(
            dashboard={"uid": "cpu-main", "title": "CPU Main"},
            panel={"id": 7, "title": "CPU Usage", "type": "timeseries"},
            datasource={"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
            query_record={
                "panelId": "7",
                "queryField": "expr",
                "query": "rate(node_cpu_seconds_total[5m])",
                "file": "General/CPU__cpu-main.json",
            },
        )
        self.assertEqual(reference.dashboard_uid, "cpu-main")
        self.assertEqual(reference.datasource.stable_identity, "prom-main")
        self.assertEqual(reference.panel.panel_id, "7")
        self.assertEqual(reference.query_field, "expr")

    def test_dedupe_text_sequence(self):
        values = ["a", "b", "a", "c", "", "b"]
        self.assertEqual(dedupe_text_sequence(values), ["a", "b", "c"])


if __name__ == "__main__":
    unittest.main()
