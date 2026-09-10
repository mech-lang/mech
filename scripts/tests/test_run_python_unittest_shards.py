from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "run-python-unittest-shards.py"
SPEC = importlib.util.spec_from_file_location("run_python_unittest_shards", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


class PythonUnittestShardRunnerTests(unittest.TestCase):
    def test_module_name_accepts_repository_test_paths(self):
        self.assertEqual(
            RUNNER.module_name("scripts/tests/test_check_r6_memory_runtime.py"),
            "scripts.tests.test_check_r6_memory_runtime",
        )

    def test_partition_covers_each_test_once_and_balances_shards(self):
        test_ids = [f"test_{index}" for index in range(11)]
        shards = RUNNER.partition(test_ids, 4)
        self.assertCountEqual((item for shard in shards for item in shard), test_ids)
        self.assertLessEqual(max(map(len, shards)) - min(map(len, shards)), 1)

    def test_partition_does_not_create_empty_shards(self):
        self.assertEqual(RUNNER.partition(["one", "two"], 4), [["one"], ["two"]])


if __name__ == "__main__":
    unittest.main()
