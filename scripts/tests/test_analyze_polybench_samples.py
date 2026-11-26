import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "analyze-polybench-samples.py"
SPEC = importlib.util.spec_from_file_location("polybench_samples", SCRIPT)
SAMPLES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SAMPLES)

CALL_GRAPH = """Call graph:
    12 Thread_main
      10 mech_gpu::native_artifact::program::runtime::execute (in test)
        7 mech_gpu::native_artifact::program::runtime::hooks::helper (in test)
          5 <mech_gpu::native_artifact::program::runtime::invocation::Invocation>::canonical_library_call (in test)
            2 malloc (in libsystem_malloc.dylib)
        3 mech_gpu::native_artifact::program::runtime::hooks::admit (in test)
      2 cache_flush (in test)
Sort by top of stack
"""


class PolybenchSampleTests(unittest.TestCase):
    def test_execution_filter_and_disjoint_categories_preserve_all_samples(self):
        result = SAMPLES.analyze(SAMPLES.parse_nodes(CALL_GRAPH))
        self.assertEqual(result["all_thread_samples"], 12)
        self.assertEqual(result["execution_samples"], 10)
        self.assertEqual(result["excluded_samples"], 2)
        actions = {row["name"]: row["samples"] for row in result["action_categories"]}
        self.assertEqual(actions, {
            "Canonical library calls": 5,
            "Source visits and admission": 3,
            "Other helper dispatch": 2,
        })
        mechanisms = {row["name"]: row["samples"] for row in result["self_time_mechanisms"]}
        self.assertEqual(mechanisms["Allocation and deallocation"], 2)
        for key in ["action_categories", "self_time_mechanisms"]:
            self.assertAlmostEqual(sum(row["percent"] for row in result[key]), 100)

    def test_malformed_parent_counts_and_missing_execution_are_rejected(self):
        with self.assertRaises(ValueError):
            SAMPLES.parse_nodes(CALL_GRAPH.replace("      10 mech_gpu", "      1 mech_gpu"))
        with self.assertRaises(ValueError):
            SAMPLES.analyze(SAMPLES.parse_nodes(CALL_GRAPH.replace("::runtime::execute", "::other")))

    def test_inclusive_functions_are_distinct_from_self_costs(self):
        result = SAMPLES.analyze(SAMPLES.parse_nodes(CALL_GRAPH))
        inclusive = {row["name"]: row["samples"] for row in result["top_inclusive_functions"]}
        own = {row["name"]: row["samples"] for row in result["top_self_functions"]}
        name = next(name for name in inclusive if name.endswith("::canonical_library_call"))
        self.assertEqual(inclusive[name], 5)
        self.assertEqual(own[name], 3)

    def test_native_update_wrappers_and_generic_bodies_have_the_same_category(self):
        for symbol in [
            "<mech_gpu::native_artifact::program::runtime::invocation::Invocation>::scalar_matrix_replace_call",
            "<mech_gpu::native_artifact::program::runtime::invocation::Invocation>::matrix_replace_call::<false>",
            "<mech_engine::resident::general::NativePreparedLibraryCall>::matrix_replace_with::<Value, false>",
        ]:
            self.assertEqual(SAMPLES.action([symbol]), "Selected matrix updates")


if __name__ == "__main__":
    unittest.main()
