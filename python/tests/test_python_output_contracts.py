import copy
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = REPO_ROOT / "scripts"
REGISTRY_PATH = REPO_ROOT / "scripts" / "contracts" / "output-contracts.json"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import output_contracts


class OutputContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.registry = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
        cls.contracts = cls.registry.get("contracts") or []

    def test_registry_is_well_formed(self):
        self.assertEqual(output_contracts.validate_registry(self.registry), [])
        self.assertEqual(self.registry.get("schema"), 1)
        self.assertIsInstance(self.contracts, list)
        self.assertGreaterEqual(len(self.contracts), 3)

    def test_contract_fixtures_match_root_shape(self):
        self.assertEqual(output_contracts.check_output_contracts(), [])

    def test_registry_accepts_collection_constraints(self):
        registry = copy.deepcopy(self.registry)
        registry["contracts"][0]["minimumItems"] = {"operations": 1}
        registry["contracts"][0]["arrayItemTypes"] = {"operations": "object"}
        registry["contracts"][0]["enumValues"] = {"operations[*].kind": ["folder"]}
        registry["contracts"][0]["pathTypes"] = {"operations[*].kind": "non-empty-string"}

        self.assertEqual(output_contracts.validate_registry(registry), [])

    def test_values_for_path_supports_wildcard_collection_paths(self):
        document = {
            "operations": [
                {"kind": "folder", "action": "would-create"},
                {"kind": "plugin", "action": "would-create"},
            ]
        }

        values, errors = output_contracts.values_for_path(document, "operations[*].kind")

        self.assertEqual(errors, [])
        self.assertEqual(values, ["folder", "plugin"])

    def test_validate_registry_rejects_invalid_collection_constraint_syntax(self):
        registry = copy.deepcopy(self.registry)
        registry["contracts"][0]["minimumItems"] = {"operations": -1}
        registry["contracts"][0]["arrayItemTypes"] = {"operations": "unsupported"}
        registry["contracts"][0]["enumValues"] = {"operations[*].kind": []}

        errors = output_contracts.validate_registry(registry)
        self.assertIn(
            "contracts[0].minimumItems['operations'] must be a non-negative integer",
            errors,
        )
        self.assertIn(
            "contracts[0].arrayItemTypes['operations'] has unsupported type 'unsupported'",
            errors,
        )
        self.assertIn(
            "contracts[0].enumValues['operations[*].kind'] must be a non-empty list",
            errors,
        )

    def test_check_output_contracts_reports_enum_value_failures_for_wildcard_paths(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            temp_registry_path = temp_root / "output-contracts.json"
            temp_fixture_dir = temp_root / "output-fixtures"
            temp_fixture_dir.mkdir(parents=True, exist_ok=True)

            registry = copy.deepcopy(self.registry)
            registry["contracts"][0]["fixture"] = "output-fixtures/sync-plan.json"
            registry["contracts"][0]["pathTypes"] = {"operations[*].action": "non-empty-string"}
            temp_fixture_path = temp_fixture_dir / "sync-plan.json"
            fixture = json.loads(
                (REGISTRY_PATH.parent / "output-fixtures" / "sync-plan.json").read_text(
                    encoding="utf-8"
                )
            )
            fixture["operations"][0]["kind"] = {"name": "folder"}
            temp_fixture_path.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")
            temp_registry_path.write_text(
                json.dumps(registry, indent=2) + "\n",
                encoding="utf-8",
            )

            errors = output_contracts.check_output_contracts(temp_registry_path)

        self.assertTrue(
            any(
                "operations[0].kind" in error
                and "expected one of" in error
                for error in errors
            ),
            errors,
        )

    def test_check_output_contracts_reports_collection_constraint_failures(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            temp_registry_path = temp_root / "output-contracts.json"
            temp_fixture_dir = temp_root / "output-fixtures"
            temp_fixture_dir.mkdir(parents=True, exist_ok=True)

            registry = copy.deepcopy(self.registry)
            registry["contracts"][0]["fixture"] = "output-fixtures/sync-plan.json"
            temp_fixture_path = temp_fixture_dir / "sync-plan.json"
            fixture = json.loads(
                (REGISTRY_PATH.parent / "output-fixtures" / "sync-plan.json").read_text(
                    encoding="utf-8"
                )
            )
            fixture["operations"][0]["kind"] = "dashboard"
            temp_fixture_path.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")
            temp_registry_path.write_text(
                json.dumps(registry, indent=2) + "\n",
                encoding="utf-8",
            )

            errors = output_contracts.check_output_contracts(temp_registry_path)

        self.assertTrue(
            any("operations[0].kind" in error and "expected one of" in error for error in errors),
            errors,
        )

    def test_check_output_contracts_reports_minimum_items_failures(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            temp_registry_path = temp_root / "output-contracts.json"
            temp_fixture_dir = temp_root / "output-fixtures"
            temp_fixture_dir.mkdir(parents=True, exist_ok=True)

            registry = copy.deepcopy(self.registry)
            registry["contracts"][1]["fixture"] = "output-fixtures/sync-preflight.json"
            temp_fixture_path = temp_fixture_dir / "sync-preflight.json"
            fixture = json.loads(
                (REGISTRY_PATH.parent / "output-fixtures" / "sync-preflight.json").read_text(
                    encoding="utf-8"
                )
            )
            fixture["checks"] = fixture["checks"][:1]
            temp_fixture_path.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")
            temp_registry_path.write_text(
                json.dumps(registry, indent=2) + "\n",
                encoding="utf-8",
            )

            errors = output_contracts.check_output_contracts(temp_registry_path)

        self.assertTrue(
            any("checks" in error and "expected at least 2 items" in error for error in errors),
            errors,
        )

    def test_module_entrypoint_round_trip(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            result = output_contracts.main(["--registry", str(REGISTRY_PATH)])

        self.assertEqual(result, 0)
        self.assertIn("check_output_contracts: ok", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
