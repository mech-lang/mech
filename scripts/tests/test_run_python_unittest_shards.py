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
            RUNNER.module_name("scripts/tests/test_check_memory_runtime.py"),
            "scripts.tests.test_check_memory_runtime",
        )

    def test_partition_covers_each_test_once_and_balances_shards(self):
        test_ids = [f"test_{index}" for index in range(11)]
        shards = RUNNER.partition(test_ids, 4)
        self.assertCountEqual((item for shard in shards for item in shard), test_ids)
        self.assertLessEqual(max(map(len, shards)) - min(map(len, shards)), 1)

    def test_partition_does_not_create_empty_shards(self):
        self.assertEqual(RUNNER.partition(["one", "two"], 4), [["one"], ["two"]])

    def test_external_shards_cover_each_test_once(self):
        test_ids = [f"test_{index}" for index in range(11)]
        shards = [RUNNER.select_shard(test_ids, index, 4) for index in range(4)]
        self.assertCountEqual((item for shard in shards for item in shard), test_ids)
        self.assertLessEqual(max(map(len, shards)) - min(map(len, shards)), 1)

    def test_external_shard_rejects_an_invalid_index(self):
        with self.assertRaises(ValueError):
            RUNNER.select_shard(["one"], 4, 4)

    def test_nested_four_and_six_job_layouts_keep_the_complete_inventory(self):
        for size in (1, 11, 347, 348):
            test_ids = [f"test_{index}" for index in range(size)]
            for outer_count in (4, 6):
                shards = [
                    worker
                    for outer_index in range(outer_count)
                    for worker in RUNNER.partition(
                        RUNNER.select_shard(test_ids, outer_index, outer_count), 2
                    )
                ]
                with self.subTest(size=size, outer_count=outer_count):
                    self.assertCountEqual(
                        (test_id for worker in shards for test_id in worker), test_ids
                    )
                    self.assertEqual(sum(map(len, shards)), len(test_ids))


if __name__ == "__main__":
    unittest.main()
