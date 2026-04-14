import io
import json
import sys
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

    def test_module_entrypoint_round_trip(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            result = output_contracts.main(["--registry", str(REGISTRY_PATH)])

        self.assertEqual(result, 0)
        self.assertIn("check_output_contracts: ok", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
