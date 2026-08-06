#!/usr/bin/env python3

import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location("ci_impact", SCRIPTS / "ci-impact.py")
assert SPEC and SPEC.loader
CI_IMPACT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CI_IMPACT)
OWNERS = CI_IMPACT.load_owners()


class ImpactClassifierTests(unittest.TestCase):
    def classify(self, paths, labels=()):
        return CI_IMPACT.classify(paths, labels, OWNERS)

    def test_documentation_only_changes_compile_nothing(self):
        result = self.classify(["README.md", "docs/distributions.md"])
        self.assertTrue(result["docs_only"])
        self.assertFalse(result["standard_canaries_required"])
        self.assertFalse(result["windows_canary_required"])
        self.assertEqual(result["changed_owners"], [])

    def test_local_machine_change_selects_only_that_owner(self):
        result = self.classify(["machines/math/src/add.rs"])
        self.assertEqual(result["changed_owners"], ["mech-math"])
        self.assertTrue(result["standard_canaries_required"])
        self.assertFalse(result["cross_cutting_standard_suite_required"])

    def test_cross_cutting_change_selects_all_runnable_standard_owners(self):
        result = self.classify(["src/core/src/value.rs"])
        expected = sorted(
            name
            for name, owner in OWNERS.items()
            if owner["standard"] and owner["command"]
        )
        self.assertEqual(result["changed_owners"], expected)
        self.assertTrue(result["cross_cutting_standard_suite_required"])
        self.assertLessEqual(len(result["owner_shards"]), 10)

    def test_browser_related_change_requests_browser_canary(self):
        result = self.classify(["hosts/scene/src/lib.rs"])
        self.assertTrue(result["browser_canary_required"])

    def test_full_label_is_the_only_pr_full_validation_trigger(self):
        ordinary = self.classify(["machines/math/src/add.rs"])
        requested = self.classify(["machines/math/src/add.rs"], ["ci:full"])
        self.assertFalse(ordinary["full_validation_required"])
        self.assertTrue(requested["full_validation_required"])

    def test_unknown_paths_are_handled_conservatively(self):
        result = self.classify(["new-top-level-area/file.rs"])
        self.assertTrue(result["cross_cutting_standard_suite_required"])
        self.assertEqual(result["unmatched_paths"], ["new-top-level-area/file.rs"])

    def test_shards_never_exceed_runner_budget(self):
        names = [f"owner-{index:02}" for index in range(31)]
        shards = CI_IMPACT.make_shards(names)
        self.assertEqual(len(shards), 10)
        flattened = sorted(
            owner
            for shard in shards
            for owner in shard["owners"].split(",")
        )
        self.assertEqual(flattened, names)


if __name__ == "__main__":
    unittest.main()
