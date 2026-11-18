"""Exercise the decision policy on small, complete evidence campaigns."""

from contextlib import redirect_stdout
import gzip
from hashlib import sha256
from io import StringIO
from itertools import permutations
import json
from math import exp, log
import os
from pathlib import Path
from statistics import fmean
import struct
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
import zlib

PACKET = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PACKET))
import qualify
from qualification_records import (
    ACTORS, CANDIDATE, CONTROL, MECH, PEERS, POLICY_PATH, Evidence, Findings, load_fixtures, load_policy, read_json,
)
from qualification_timings import CLEANUP, COUNTERS, DUPLICATE_ACTORS


def fake_macho(count):
    commands = 152
    header = struct.pack("<8I", 0xFEEDFACF, 0x0100000C, 0, 1, 1, commands, 0, 0)
    segment = struct.pack("<2I16s4Q4I", 0x19, commands, b"__TEXT", 0, count, 32 + commands, count, 0, 0, 1, 0)
    section = struct.pack("<16s16s2Q8I", b"__text", b"__TEXT", 0, count, 32 + commands, 0, 0, 0, 0, 0, 0, 0)
    return header + segment + section + b"\0" * count


class Campaign:
    """Build a frozen-size synthetic campaign using the repository's case contract."""

    def __init__(self, root, policy, fixtures):
        self.root = root
        self.policy = policy
        self.fixtures = fixtures
        self.names = list(fixtures)
        self.manifest = {
            "schema_version": 1,
            "policy_version": policy["version"],
            "objective": "encoder_text_bytes",
            "environment": self.json("environment.json", {"cpu": "test ARM64", "os": "test OS", "clock": "test monotonic"}),
            "contract": self.json("contract.json", {
                "fixture_inventory_sha256": policy["fixture_inventory_sha256"], "level": 12,
                "scope": "diagnostic", "timing_boundary": "fresh complete compression",
            }),
            "runs": [], "duplicate_controls": [],
        }
        self.plan = {
            "policy_version": policy["version"], "objective": self.manifest["objective"], "runs": [], "duplicate_control_ids": [],
            **self.binding(),
        }
        self.manifest["artifacts"] = {
            actor: self.actor(actor, size) for actor, size in zip(ACTORS, (151, 101, 90, 65), strict=True)
        }
        self.write_proofs()
        self.write_build_metrics()

    def file(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return {"path": name, "sha256": sha256(data).hexdigest()}

    def json(self, name, value):
        raw = json.dumps(value, separators=(",", ":")).encode()
        return self.file(name, gzip.compress(raw, mtime=0) if name.endswith(".gz") else raw)

    def get(self, ref):
        return read_json(self.root / ref["path"])

    def binding(self):
        return {f"{field}_sha256": self.manifest[field]["sha256"] for field in ("environment", "contract")}

    def actor(self, actor, size):
        row = {
            "worker": self.file(f"{actor}/worker", fake_macho(size + 10)),
            "object": self.file(f"{actor}/object.o", fake_macho(size)),
        }
        if actor == "roc":
            row["metadata"] = self.json(f"{actor}/metadata.json", {"kind": "roc"})
        if actor in MECH:
            origins = [None] * 7165
            for data in self.policy["checked_failures"].values():
                origins[data["origin"]] = {key: data[key] for key in ("node", "local")}
            row["metadata"] = self.json(f"{actor}/metadata.json", {"origins": origins, "stack_bytes": 64})
            for field in ("source", "compiler", "bytecode", "runtime", "adapter", "options"):
                row[field] = self.file(f"{actor}/{field}", field.encode())
            row["source_commit"] = "1" * 40
            self.write_link_receipt(actor, row)
        return row

    def write_link_receipt(self, actor, artifacts):
        artifacts["build_receipt"] = self.json(f"{actor}/build-receipt.json", {
            "source_commit": artifacts["source_commit"],
            "artifact_sha256": {
                key: value["sha256"] for key, value in artifacts.items()
                if key not in ("source_commit", "build_receipt")
            },
            "commands": {key: [key] for key in ("frontend", "emit", "link")},
        })

    def pins(self, run_id):
        actors = {
            actor: {"binary_sha256": record["worker"]["sha256"]} for actor, record in self.manifest["artifacts"].items()
        }
        for actor, record in self.manifest["artifacts"].items():
            if "metadata" in record:
                actors[actor]["metadata_sha256"] = record["metadata"]["sha256"]
        return self.json(f"{run_id}/pins.json", {
            "actors": actors, "level": 12, "fresh_libdeflate_compressor_inside_timing": True,
            "input_inventory_sha256": self.policy["fixture_inventory_sha256"], **self.binding(),
        })

    def call(self, name, actor, duration):
        fixture = self.fixtures[name]
        row = {key: fixture[key] for key in ("input_bytes", "prefix_bytes", "raw_bytes", "capacity")}
        row.update(case=name, seconds=duration, warmup=False, profile=False, independent_inflation=True,
                   input_alias_unchanged=True, borrowed_output_alias_unchanged=True,
                   output_sha256=fixture["expected_raw_sha256"])
        if actor == "libdeflate":
            row.update(level=12, version="1.26", reference_capacity=fixture["capacity"], matches_reference_bytes=True)
        else:
            row["exact_reference"] = True
        if actor in MECH:
            row["nonreturned_private_bytes_at_return"] = 0
            row.update(dict.fromkeys(COUNTERS, 33))
        return row

    def run(self, run_id, direction, *, kind="full", names=None, wins=61, overrides=None):
        names = names if names is not None else self.names
        overrides = overrides or {}
        ordered = names if direction == "forward" else list(reversed(names))
        blocks_count = 24
        observations = {}
        summary = []
        for name in ordered:
            compress_time = 0.9 if self.names.index(name) < wins else 1.1
            means = {CONTROL: compress_time, CANDIDATE: compress_time, "roc": 2.0, "libdeflate": 1.0}
            means.update(overrides.get(name, {}))
            calls = {actor: [self.call(name, actor, duration)] for actor, duration in means.items()}
            blocks = [
                {"order": order, "means": means, "observations": calls} for order in permutations(ACTORS)
            ]
            observations[name] = self.json(f"{run_id}/{name}/observations.json.gz", blocks)
            fixture = self.fixtures[name]
            summary.append({
                "case": name, "input_bytes": fixture["input_bytes"], "reference_raw_bytes": fixture["raw_bytes"],
                "reference_capacity": fixture["capacity"], "libdeflate_raw_bytes": fixture["raw_bytes"],
                "libdeflate_capacity": fixture["capacity"], "libdeflate_same_bytes": True,
                "raw_stream_sha256": fixture["expected_raw_sha256"], "median_seconds": means,
                "timed_calls": blocks_count * len(ACTORS),
                **{f"paired_x_{peer}": {actor: means[actor] / means[peer] for actor in ACTORS} for peer in PEERS},
            })
        planned = {"id": run_id, "kind": kind, "order": direction, "blocks_per_case": blocks_count}
        self.plan["runs"].append(planned)
        entry = {
            "id": run_id, "kind": kind, "order": direction, "pins": self.pins(run_id),
            "summary": self.json(f"{run_id}/summary.json", summary),
            "complete": self.json(f"{run_id}/complete.json", {
                "cases": len(names), "balanced_blocks_per_case": blocks_count,
                "timed_calls": len(names) * blocks_count * len(ACTORS),
                "reference_capacity_matches": len(names), "libdeflate_exact_bytes_cases": len(names),
                "fresh_compression_all_cases": True, "all_outputs_independently_inflated": True,
                "all_input_and_output_aliases_preserved": True,
            }),
            "cleanup": {
                actor: self.file(f"{run_id}/{actor}.stderr", (line + "\n").encode()) for actor, line in CLEANUP.items()
            },
            "observations": observations,
        }
        self.manifest["runs"].append(entry)
        return entry

    def full(self, **options):
        return [self.run("fwd", "forward", **options), self.run("rev", "reverse", **options)]

    def rewrite(self, ref, modify):
        value = self.get(ref)
        modify(value)
        ref.update(self.json(ref["path"], value))

    def proof_subject(self, actor):
        artifacts = self.manifest["artifacts"][actor]
        return {
            "object_sha256": artifacts["object"]["sha256"], "metadata_sha256": artifacts["metadata"]["sha256"],
            "probe_worker": self.file(f"{actor}/probe-worker", b"checked and stack probe"),
        }

    def write_proofs(self):
        artifacts = self.manifest["artifacts"][CANDIDATE]
        receipt = {
            "worker_sha256": artifacts["worker"]["sha256"], "metadata_sha256": artifacts["metadata"]["sha256"],
            "complete_smokes": len(self.fixtures) * 2, "verified": True,
        }
        calls = [self.call(name, CANDIDATE, 1.0) for name in self.names for _ in range(2)]
        cases = {}
        for name, expected in self.policy["checked_failures"].items():
            outcome = {"case": name, "origin": expected["origin"], "status": expected["status"]}
            claimed = {key: expected[key] for key in ("node", "local")}
            cases[name] = {
                "outcome": outcome, "per_actor_outcomes": [outcome, outcome],
                "raw": [[{**outcome, "seconds": 0.1}]] * 2, "matched_source_origins": [claimed, claimed],
            }
        self.manifest["checks"] = {
            "correctness": {"receipt": self.json("correctness.json", receipt),
                            "calls": self.json("correctness-calls.json.gz", calls)},
            "checked_failures": self.json("checked-failures.json", {
                "subjects": {actor: self.proof_subject(actor) for actor in MECH}, "cases": cases,
            }),
            "stack": {
                role: self.json(f"{actor}/stack.json", {
                    "subject": self.proof_subject(actor), "cases": {name: {"0": 64, "1": 80} for name in self.names},
                }) for role, actor in (("control", CONTROL), ("candidate", CANDIDATE))
            },
        }

    def write_build_metrics(self):
        order_indices = ((0, 1, 3, 2), (1, 2, 0, 3), (2, 3, 1, 0), (3, 0, 2, 1))
        orders = [[ACTORS[index] for index in order] for order in order_indices]
        self.plan["build_orders"] = orders
        seconds = {CONTROL: 5.0, CANDIDATE: 4.9, "roc": 2.5, "libdeflate": 1.0}
        outputs = {
            actor: {"object_sha256": row["object"]["sha256"], "executable_sha256": row["worker"]["sha256"]}
            for actor, row in self.manifest["artifacts"].items()
        }
        self.manifest["metrics"] = self.json("build-metrics.json", {
            "scope": "source-through-final-link", "prebuilt_compilers_and_adapters": True,
            "environment_sha256": self.binding()["environment_sha256"],
            "timed_trials": [{"order": order, "seconds": seconds, "peak_rss_bytes": dict.fromkeys(ACTORS, 100),
                              "outputs": outputs} for order in orders],
        })

    def duplicate_study(self, case, aa_ratio, ab_ratio):
        run_id = "repeat"
        artifacts = self.manifest["artifacts"]
        control = artifacts[CONTROL]["worker"]["sha256"]
        timings = dict(zip(DUPLICATE_ACTORS, (1, aa_ratio, ab_ratio * aa_ratio ** 0.5), strict=True))
        contrasts = {key: {bound: ratio - 1.0 for bound in ("estimate", "lower", "upper")}
                     for key, ratio in (("aa", aa_ratio), ("ab", ab_ratio))}
        self.plan["duplicate_control_ids"].append(run_id)
        self.plan["duplicate_control_cases"] = [case]
        self.manifest["duplicate_controls"].append({
            "id": run_id,
            "study": self.json("repeat/study.json", {
                "order": [case], "distinct_worker_files": 3, "acceptance_resolution": self.policy["runtime_resolution"],
                "worker_sha256": {"control": control, "duplicate-control": control,
                                  "candidate": artifacts[CANDIDATE]["worker"]["sha256"]},
                "repetitions_per_order": 1, "seed": 27, "batch_floor": 2, **self.binding(),
            }),
            "results": self.json("repeat/results.json", {case: {"batch": 2, **contrasts}}),
            "observations": {case: self.json(f"repeat/{case}.json.gz", {
                "case": case, "batch": 2, "blocks": [
                    {"order": list(order), "means": timings, "seconds": {a: [t, t] for a, t in timings.items()}}
                    for order in permutations(DUPLICATE_ACTORS)
                ],
            })},
        })

    def seal(self):
        self.plan.setdefault("worker_sha256", {
            actor: record["worker"]["sha256"] for actor, record in self.manifest["artifacts"].items()
        })
        self.plan.setdefault("artifact_sha256", {
            actor: {key: ref["sha256"] for key, ref in record.items() if key not in ("source_commit", "build_receipt")}
            for actor, record in self.manifest["artifacts"].items()
        })
        self.manifest["plan"] = self.json("predeclared-plan.json", self.plan)
        return self.manifest["plan"]["sha256"]

    def evaluate(self, pinned_plan=None):
        return qualify.evaluate(self.manifest, self.root, pinned_plan or self.seal())


class QualificationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = read_json(POLICY_PATH)
        cls.fixture_root = Path(os.environ.get("MECH_NATIVE_QUAL_FIXTURE_ROOT", PACKET))
        findings = Findings()
        cls.fixtures = load_fixtures(Evidence(cls.fixture_root, findings), cls.policy)
        if findings.unresolved:
            raise AssertionError(findings.unresolved)

    def setUp(self):
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.campaign = Campaign(Path(self.temp.name), self.policy, self.fixtures)
        fixtures = patch.object(qualify, "load_fixtures", return_value=self.fixtures)
        fixtures.start()
        self.addCleanup(fixtures.stop)

    def outcome(self, expected):
        result = self.campaign.evaluate()
        self.assertEqual(result["decision"], expected, result["issues"])
        return result

    def test_retains_verified_improvement(self):
        self.campaign.full()
        result = self.outcome("Retain")
        self.assertEqual(result["frozen_suite"], {"named_cases": 81, "distinct_contracts": 74})
        self.assertEqual(result["runtime"]["runs"][0]["peers"]["roc"]["distinct_contract_point_estimate_wins"][CANDIDATE], 74)
        self.assertEqual(result["sizes"][CANDIDATE]["encoder_text_bytes"], 101)
        self.assertEqual(result["builds"]["source_to_link_seconds"]["libdeflate"], 1.0)

    def test_matching_below_floor_is_unresolved(self):
        self.campaign.full(wins=60)
        result = self.outcome("Unresolved")
        self.assertTrue(any("floor 61 missed (control 60, candidate 60)" in line for line in result["issues"]["unresolved"]))

    def test_fixed_floors_do_not_override_per_case_regression(self):
        self.campaign.full(overrides={"empty": {CANDIDATE: 0.93}})
        result = self.outcome("Reject")
        self.assertEqual(result["runtime"]["cases"]["empty"]["decision"], "regression")
        self.assertEqual(result["runtime"]["runs"][0]["peers"]["libdeflate"]["point_estimate_wins"][CANDIDATE], 61)

    def use_peer_win_policy(self):
        self.campaign = Campaign(Path(self.temp.name), load_policy(2), self.fixtures)

    def test_peer_win_policy_retains_smaller_artifact_with_a_reported_slowdown(self):
        self.use_peer_win_policy()
        self.campaign.full(overrides={"empty": {CANDIDATE: 0.93}})
        result = self.outcome("Retain")
        self.assertEqual(result["policy_version"], 2)
        self.assertEqual(result["runtime"]["cases"]["empty"]["decision"], "regression")
        timing = result["runtime"]["candidate_control_timing_observations"]
        self.assertEqual(timing["slowdowns"], ["empty: established slowdown above the 0.5% resolution"])
        self.assertEqual(result["issues"], {"rejected": [], "unresolved": []})

    def test_peer_win_policy_still_rejects_a_lost_win_with_equal_totals(self):
        self.use_peer_win_policy()
        lost, gained = self.campaign.names[60:62]
        self.campaign.full(overrides={lost: {CANDIDATE: 1.1}, gained: {CANDIDATE: 0.9}})
        result = self.outcome("Reject")
        peer = result["runtime"]["runs"][0]["peers"]["libdeflate"]
        self.assertEqual(peer["point_estimate_wins"], {CONTROL: 61, CANDIDATE: 61})
        self.assertEqual(peer["lost_cases"], [lost])

    def test_peer_win_policy_still_requires_both_floors(self):
        self.use_peer_win_policy()
        self.campaign.full(wins=60)
        result = self.outcome("Unresolved")
        self.assertTrue(any("floor 61 missed" in line for line in result["issues"]["unresolved"]))

    def high_water_campaign(self):
        return Campaign(Path(self.temp.name), load_policy(3), self.fixtures)

    def test_current_policy_requires_62_wins_in_each_complete_order(self):
        self.campaign = self.high_water_campaign()
        case = "prefix-natural-8192"
        assert self.campaign.names.index(case) < 61
        self.campaign.run("fwd", "forward", wins=62)
        self.campaign.run("rev", "reverse", wins=61)
        result = self.outcome("Unresolved")
        self.assertEqual(result["policy_version"], 3)
        self.assertTrue(any("rev: libdeflate floor 62 missed" in line for line in result["issues"]["unresolved"]))

    def test_current_policy_preserves_the_historical_8_kib_win(self):
        self.campaign = self.high_water_campaign()
        case = "prefix-natural-8192"
        new_win = self.campaign.names[62]
        self.campaign.full(wins=62, overrides={case: {CONTROL: 1.1, CANDIDATE: 1.1},
                                              new_win: {CONTROL: 0.9, CANDIDATE: 0.9}})
        result = self.outcome("Reject")
        self.assertEqual(result["runtime"]["runs"][0]["peers"]["libdeflate"]["point_estimate_wins"][CANDIDATE], 62)
        self.assertTrue(any("missed required libdeflate wins" in line for line in result["issues"]["rejected"]))

    def test_current_policy_accepts_restoring_the_62nd_win(self):
        self.campaign = self.high_water_campaign()
        case = "prefix-natural-8192"
        self.campaign.full(wins=62, overrides={case: {CONTROL: 1.1}})
        self.outcome("Retain")

    def test_peer_win_policy_reports_duplicate_control_uncertainty(self):
        self.use_peer_win_policy()
        self.campaign.full()
        self.campaign.duplicate_study("empty", 0.99, 0.995)
        result = self.outcome("Retain")
        self.assertEqual(result["runtime"]["cases"]["empty"]["decision"], "unresolved")
        self.assertTrue(any("empty: regression bound" in line
                            for line in result["runtime"]["candidate_control_timing_observations"]["uncertain"]))

    def test_peer_win_policy_requires_a_plan_with_the_same_version(self):
        self.use_peer_win_policy()
        self.campaign.plan["policy_version"] = 1
        self.campaign.full()
        result = self.outcome("Reject")
        self.assertTrue(any("disagrees on the policy" in line for line in result["issues"]["rejected"]))

    def test_equal_counts_cannot_conceal_lost_case(self):
        lost, gained = self.campaign.names[60:62]
        self.campaign.full(overrides={lost: {CANDIDATE: 1.1}, gained: {CANDIDATE: 0.9}})
        result = self.outcome("Reject")
        peer = result["runtime"]["runs"][0]["peers"]["libdeflate"]
        self.assertEqual(peer["lost_cases"], [lost])
        self.assertEqual(peer["gained_cases"], [gained])
        self.assertEqual(peer["point_estimate_wins"], {CONTROL: 61, CANDIDATE: 61})

    def test_focus_cannot_qualify_complete_suite(self):
        self.campaign.full(kind="focus", names=self.campaign.names[:5])
        self.outcome("Unresolved")

    def test_mislabeled_focus_fails_frozen_case_contract(self):
        self.campaign.full(names=self.campaign.names[:5])
        self.outcome("Reject")

    def test_duplicate_or_substituted_cases_fail_contract(self):
        fwd, _ = self.campaign.full()
        self.campaign.rewrite(fwd["summary"], lambda rows: rows[1].update(case=rows[0]["case"]))
        self.outcome("Reject")

    def test_changed_worker_cannot_inherit_run_receipts(self):
        self.campaign.full()
        artifacts = self.campaign.manifest["artifacts"][CANDIDATE]
        artifacts["worker"] = self.campaign.file("replacement-worker", fake_macho(905))
        self.campaign.write_link_receipt(CANDIDATE, artifacts)
        result = self.outcome("Reject")
        self.assertTrue(any("binary_sha256 differs" in line for line in result["issues"]["rejected"]))

    def test_predeclared_artifacts_cannot_change_under_same_run_ids(self):
        self.campaign.full()
        anchor = self.campaign.seal()
        artifacts = self.campaign.manifest["artifacts"][CANDIDATE]
        artifacts["options"] = self.campaign.file("replacement-options", b"new effective options")
        self.campaign.write_link_receipt(CANDIDATE, artifacts)
        result = self.campaign.evaluate(anchor)
        self.assertEqual(result["decision"], "Reject")
        self.assertTrue(any("artifacts differ from the predeclared plan" in line for line in result["issues"]["rejected"]))

    def test_byte_identical_worker_can_keep_its_runtime_receipts(self):
        self.campaign.full()
        artifacts = self.campaign.manifest["artifacts"][CANDIDATE]
        old_worker_hash = artifacts["worker"]["sha256"]
        artifacts["worker"] = self.campaign.file("relinked-exact-worker", fake_macho(111))
        self.assertEqual(artifacts["worker"]["sha256"], old_worker_hash)
        artifacts["compiler"] = self.campaign.file("new-compiler", b"compiler implementation changed")
        artifacts["source_commit"] = "2" * 40
        self.campaign.manifest["objective"] = self.campaign.plan["objective"] = "build_seconds"
        self.campaign.write_link_receipt(CANDIDATE, artifacts)
        self.outcome("Retain")

    def test_different_valid_compressed_bytes_fail_exact_output(self):
        fwd, _ = self.campaign.full()
        expected = (self.fixture_root / self.fixtures["empty"]["expected"]).read_bytes()
        replacement = bytes([expected[0] ^ 0x08]) + expected[1:]
        self.assertNotEqual(replacement, expected)
        self.assertEqual(zlib.decompress(replacement, -15), zlib.decompress(expected, -15))
        def change(rows):
            rows[0]["observations"][CANDIDATE][0]["output_sha256"] = sha256(replacement).hexdigest()
        self.campaign.rewrite(fwd["observations"]["empty"], change)
        result = self.outcome("Reject")
        self.assertTrue(any("output digest differs" in line for line in result["issues"]["rejected"]))

    def test_boolean_only_correctness_cannot_promote(self):
        self.campaign.full()
        def remove_digests(rows):
            for row in rows:
                row.pop("output_sha256")
        self.campaign.rewrite(self.campaign.manifest["checks"]["correctness"]["calls"], remove_digests)
        result = self.outcome("Unresolved")
        self.assertTrue(any("lack exact raw output SHA-256" in line for line in result["issues"]["unresolved"]))

    def test_wrong_checked_failure_origin_is_rejected(self):
        self.campaign.full()
        def change(value):
            case = value["cases"]["empty"]
            case["per_actor_outcomes"][1]["origin"] = 69
            case["raw"][1][0]["origin"] = 69
        self.campaign.rewrite(self.campaign.manifest["checks"]["checked_failures"], change)
        result = self.outcome("Reject")
        self.assertTrue(any("origin disagrees" in line for line in result["issues"]["rejected"]))

    def test_summary_numbers_must_match_raw_calls(self):
        fwd, _ = self.campaign.full()
        self.campaign.rewrite(fwd["summary"], lambda rows: rows[0]["paired_x_libdeflate"].update({CANDIDATE: 0.1}))
        result = self.outcome("Reject")
        self.assertTrue(any("summary disagrees with raw blocks" in line for line in result["issues"]["rejected"]))

    def test_passing_ab_fails_with_noisy_duplicate_controls(self):
        self.campaign.full()
        self.campaign.duplicate_study("empty", 0.99, 0.995)
        result = self.outcome("Unresolved")
        row = result["runtime"]["cases"]["empty"]
        self.assertLessEqual(row["duplicate_controls"][0]["ab"]["upper"], 0.005)
        self.assertEqual(row["decision"], "unresolved")

    def test_predeclared_adverse_run_cannot_disappear(self):
        self.campaign.run("adverse", "forward", overrides={"empty": {CANDIDATE: 0.94}})
        self.campaign.full(overrides={"empty": {CANDIDATE: 0.88}})
        anchor = self.campaign.seal()
        self.campaign.manifest["runs"].pop(0)
        result = self.campaign.evaluate(anchor)
        self.assertEqual(result["decision"], "Unresolved")
        self.assertIn("predeclared timing run missing: adverse", result["issues"]["unresolved"])
        self.campaign.plan["runs"].pop(0)
        self.campaign.seal()
        result = self.campaign.evaluate(anchor)
        self.assertEqual(result["decision"], "Reject")
        self.assertTrue(any("independently pinned plan" in line for line in result["issues"]["rejected"]))

    def test_favorable_rerun_does_not_hide_an_adverse_individual_run(self):
        self.campaign.run("adverse", "forward", overrides={"empty": {CANDIDATE: 0.92}})
        self.campaign.full(overrides={"empty": {CANDIDATE: 0.86}})
        result = self.outcome("Unresolved")
        case = result["runtime"]["cases"]["empty"]
        self.assertLess(case["combined"]["upper"], self.policy["runtime_resolution"])
        self.assertEqual(case["decision"], "unresolved")
        self.assertEqual(case["adverse_runs"], ["adverse"])

    def test_exact_tie_is_not_a_win(self):
        name = self.campaign.names[60]
        self.campaign.full(overrides={name: {CONTROL: 1.0, CANDIDATE: 1.0}})
        result = self.outcome("Unresolved")
        peer = result["runtime"]["runs"][0]["peers"]["libdeflate"]
        self.assertEqual(peer["point_estimate_wins"][CANDIDATE], 60)

    def test_quantized_zero_call_is_valid_with_positive_block_mean(self):
        fwd, _ = self.campaign.full()

        def quantize(rows):
            for block in rows:
                for actor in ACTORS:
                    sample = block["observations"][actor][0]
                    time = sample["seconds"]
                    block["observations"][actor] = [
                        {**sample, "seconds": value} for value in (0.0, time, time, 2 * time)
                    ]

        self.campaign.rewrite(fwd["observations"]["empty"], quantize)
        self.campaign.rewrite(fwd["summary"], lambda rows: rows[0].update(timed_calls=24 * 4 * 4))
        self.campaign.rewrite(fwd["complete"], lambda row: row.update(timed_calls=row["timed_calls"] + 24 * 4 * 3))
        self.outcome("Retain")

    def test_promotion_exit_status(self):
        self.campaign.full(wins=60)
        anchor = self.campaign.seal()
        manifest = self.campaign.json("manifest.json", self.campaign.manifest)
        path = self.campaign.root / manifest["path"]
        output = StringIO()
        with redirect_stdout(output):
            status = qualify.main([str(path), "--promotion", "--plan-sha256", anchor])
        self.assertEqual(status, 2)
        self.assertEqual(json.loads(output.getvalue())["decision"], "Unresolved")
        output = StringIO()
        with redirect_stdout(output):
            status = qualify.main([str(path), "--promotion", "--plan-sha256", "1" * 64])
        self.assertEqual(status, 1)
        self.assertEqual(json.loads(output.getvalue())["decision"], "Reject")


if __name__ == "__main__":
    unittest.main()
