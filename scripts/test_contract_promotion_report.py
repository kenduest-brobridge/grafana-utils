from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from scripts.contract_promotion_report import (
    build_promotion_report,
    load_manifest_contracts,
    load_runtime_contracts,
    render_text_report,
)


class ContractPromotionReportTest(unittest.TestCase):
    def test_groups_manifest_and_runtime_families(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            manifests_dir = root / "schemas" / "manifests"
            change_dir = manifests_dir / "change"
            status_dir = manifests_dir / "status"
            change_dir.mkdir(parents=True)
            status_dir.mkdir(parents=True)

            change_dir.joinpath("contracts.json").write_text(
                json.dumps(
                    [
                        {
                            "contractId": "sync-plan",
                            "title": "Sync plan",
                            "goldenSample": "fixtures/machine_schema_golden_cases.json#sync-plan",
                        },
                        {
                            "contractId": "sync-preflight",
                            "title": "Sync preflight",
                            "goldenSample": "fixtures/machine_schema_golden_cases.json#sync-preflight",
                        },
                    ]
                ),
                encoding="utf-8",
            )
            status_dir.joinpath("contracts.json").write_text(
                json.dumps(
                    [
                        {
                            "contractId": "project-status",
                            "title": "Project status",
                            "goldenSample": "fixtures/machine_schema_golden_cases.json#project-status",
                        }
                    ]
                ),
                encoding="utf-8",
            )

            registry_path = root / "scripts" / "contracts"
            registry_path.mkdir(parents=True)
            registry_path.joinpath("output-contracts.json").write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "contracts": [
                            {
                                "name": "sync-plan",
                                "kind": "grafana-utils-sync-plan",
                                "schemaVersion": 1,
                                "fixture": "output-fixtures/sync-plan.json",
                                "requiredFields": ["kind"],
                            },
                            {
                                "name": "dashboard-summary-governance",
                                "kind": "grafana-utils-dashboard-summary-governance",
                                "schemaVersion": 1,
                                "fixture": "output-fixtures/dashboard-summary-governance.json",
                                "requiredFields": ["kind"],
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )

            manifest_contracts = load_manifest_contracts(manifests_dir)
            runtime_contracts = load_runtime_contracts(registry_path / "output-contracts.json")
            report = build_promotion_report(manifest_contracts, runtime_contracts)
            text = render_text_report(report, manifest_contracts, runtime_contracts)

            self.assertIn("manifest families: 2", text)
            self.assertIn("runtime families: 2", text)
            self.assertIn("change: total=2 shared=1 only=1 runtime=sync", text)
            self.assertIn("status: total=1 shared=0 only=1 runtime=-", text)
            self.assertIn("sync: total=1 shared=1 only=0 manifest=change", text)
            self.assertIn("dashboard: total=1 shared=0 only=1 manifest=-", text)
            self.assertEqual(report.shared_contract_ids, ("sync-plan",))
            self.assertEqual(report.manifest_only_contract_ids, ("project-status", "sync-preflight"))
            self.assertEqual(report.runtime_only_contract_ids, ("dashboard-summary-governance",))


if __name__ == "__main__":
    unittest.main()
