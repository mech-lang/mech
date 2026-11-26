import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "record-polybench-acceptance.py"
SPEC = importlib.util.spec_from_file_location("polybench_acceptance", SCRIPT)
ACCEPTANCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ACCEPTANCE)


def entry(label, optimization, holdout, count=15):
    return {"candidate": {"label": label, "source_commit": label}, "aggregates": {
        p: {"count": count, ACCEPTANCE.METRIC: value}
        for p, value in [("optimization", optimization), ("holdout", holdout)]}}


class PolybenchAcceptanceTests(unittest.TestCase):
    def test_failed_and_mixed_runs_never_replace_the_accepted_comparator(self):
        rows = [entry("19", 39.05, 84.28), entry("20", 33.21, 86.62),
                entry("21", 35.08, 116.81), entry("22", 31.63, 68.48)]
        decisions = ACCEPTANCE.classify(rows)
        self.assertEqual([d["performance_gate"] for d in decisions],
                         ["accepted", "not_promoted", "not_promoted", "accepted"])
        self.assertEqual(decisions[2]["run_outcome"], "failed_both_regressed")
        self.assertEqual(decisions[3]["compared_to_accepted_index"], 0)
        # A recovery versus a failed run still loses to the accepted holdout.
        recovery = ACCEPTANCE.classify(rows[:3] + [entry("recovery", 30, 90)])[-1]
        self.assertEqual(recovery["run_outcome"], "improved_both")
        self.assertEqual(recovery["performance_gate"], "not_promoted")

    def test_ties_and_incomplete_or_nonfinite_scores_cannot_pass(self):
        baseline = entry("start", 100, 100)
        for pair in [(99, 100), (100, 99), (100, 100), (101, 99)]:
            self.assertEqual(ACCEPTANCE.classify([baseline, entry("next", *pair)])[-1]
                             ["performance_gate"], "not_promoted")
        for bad in [entry("bad", float("nan"), 90), entry("bad", 90, float("inf")),
                    entry("bad", 0, 90), entry("bad", 90, 90, count=14)]:
            with self.assertRaises(ValueError):
                ACCEPTANCE.classify([baseline, bad])

    def test_record_is_append_only_and_detects_stale_or_changed_history(self):
        with tempfile.TemporaryDirectory() as temp:
            history, output = Path(temp) / "history.json", Path(temp) / "acceptance.json"
            rows = [entry("start", 100, 100), entry("first", 90, 80)]
            def save():
                history.write_text(json.dumps({"schema": "mech.polybench.performance-history.v1", "entries": rows}))
            save()
            original_bytes = history.read_bytes()
            first = ACCEPTANCE.record(history, output)
            original_ledger_bytes = output.read_bytes()
            self.assertEqual(ACCEPTANCE.record(history, output, check=True), first)
            self.assertEqual(history.read_bytes(), original_bytes)
            self.assertEqual(output.read_bytes(), original_ledger_bytes)
            rows.append(entry("mixed", 85, 81)); save()
            with self.assertRaisesRegex(ValueError, "stale"):
                ACCEPTANCE.record(history, output, check=True)
            second = ACCEPTANCE.record(history, output)
            self.assertEqual(second["entries"][:-1], first["entries"])
            self.assertEqual(second["adopted_at"], first["adopted_at"])
            self.assertEqual(second["accepted_history_index"], 1)
            gate = subprocess.run([sys.executable, str(SCRIPT), "--history", str(history),
                                   "--output", str(output), "--check", "--require-latest-accepted"],
                                  capture_output=True, text=True)
            self.assertEqual(gate.returncode, 1)
            self.assertEqual(json.loads(gate.stdout)["accepted"], "first")
            rows[0]["aggregates"]["holdout"][ACCEPTANCE.METRIC] = 101; save()
            with self.assertRaisesRegex(ValueError, "changed"):
                ACCEPTANCE.record(history, output)
            self.assertEqual(output.read_text(), json.dumps(second, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
