"""Screen smaller checked objects against the same pinned peer workers."""
from importlib import import_module
from pathlib import Path
import argparse
import json
import os
import subprocess
import sys


q = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("labels", nargs="+")
parser.add_argument("--reference", default="matcher-matrix-words")
parser.add_argument("--focus-only", action="store_true")
parser.add_argument("--full-diagnostic", action="store_true",
                    help="record both full execution orders after the correctness checks even if the screen has a loss")
parser.add_argument("--source-map", type=Path)
parser.add_argument("--allocation-change", action="store_true",
                    help="record changed allocations without requiring equal payload counters")
parser.add_argument("--reuse-focus", action="store_true")
parser.add_argument("--both-orders", action="store_true", help="record both focused execution orders")
parser.add_argument("--case-set", type=Path, help="JSON list of cases for a separate focused run")
args = parser.parse_args()
if args.focus_only and args.full_diagnostic:
    parser.error("--full-diagnostic cannot be combined with --focus-only")
if args.case_set and not (args.focus_only or args.reuse_focus):
    parser.error("--case-set requires --focus-only or --reuse-focus")
allow_allocation_change = args.allocation_change or args.source_map is not None
reference = args.reference
control = q / f"{reference}-native" / "final-candidate"
driver = q / "compare-library.py"
source = driver.read_text()
count_line = "orders = list(permutations(commands))"
assert source.count(count_line) == 1
source = source.replace(count_line, count_line + " * 4")
case_line = "selected = {case['name'] for case in cases}"
assert source.count(case_line) == 1
anchor = q / "matcher-matrix-words-focus" / "summary.json"
cases = sorted({entry["case"] for entry in json.loads(anchor.read_text())})
assert len(cases) == 22
focus_name = "focus"
if args.case_set:
    cases = json.loads(args.case_set.read_text())
    known = {row["name"] for row in json.loads((q / "fixtures.json").read_text())}
    assert cases and len(cases) == len(set(cases)) and set(cases) <= known
    focus_name = f"focus-{args.case_set.stem}"
focus = source.replace(case_line, f"selected = set({cases!r})")
compare = import_module("checked-controller-results").compare


def sample(label, suffix, code, reverse):
    packet = f"{label}-{suffix}"
    candidate = q / f"{label}-native" / "final-candidate"
    command = [str(driver), packet, str(control), str(candidate)]
    sys.argv = command + (["reverse"] if reverse else [])
    exec(compile(code, str(driver), "exec"), {"__file__": str(driver), "__name__": "__main__"})
    return compare(packet, allow_source_counter_changes=allow_allocation_change)


def keeps_wins(report):
    roc = report["roc_wins"]
    lib = report["libdeflate_wins"]
    return roc[1] >= roc[0] and lib[1] >= lib[0]


def proof(label):
    candidate = q / f"{label}-native" / "final-candidate"
    origin_map = args.source_map
    if origin_map is None:
        images = [json.loads((actor / "runtime.json").read_text())["image_sha256"]
                  for actor in (control, candidate)]
        assert images[0] == images[1], "a changed source image needs --source-map for failure attribution"
        older, newer = (json.loads((actor / "metadata.json").read_text())["origins"]
                        for actor in (control, candidate))
        assert older == newer
        origin_map = q / f"{label}-identity-origins.json"
        origin_map.write_text(json.dumps({"source_mapping": {i: {"site": i} for i in range(len(older))}}))
    proof_env = dict(os.environ, MECH_ERROR_SOURCE_MAP=str(origin_map),
                     MECH_ERROR_REPORT=f"{label}-checked-failures.json")
    subprocess.run([sys.executable, "-B", q / "check-linked-error-exits.py",
                   control.relative_to(q), candidate.relative_to(q)], env=proof_env, check=True)
    proof_env = dict(os.environ, MECH_STACK_REPORT=f"{label}-stack.json")
    subprocess.run([sys.executable, "-B", q / "check-layout-stack.py", candidate.relative_to(q)],
                   env=proof_env, check=True)
    source_option = ["--source-map", args.source_map] if args.source_map else []
    subprocess.run([sys.executable, "-B", q / "check-matcher-matrix.py",
                   control, candidate, q / f"{label}-matrix", "--extra-shapes",
                   *source_option], check=True)


for label in args.labels:
    output = q / f"{label}-native" / f"{focus_name}-timing.json"
    if args.reuse_focus:
        previous = json.loads(output.read_text())
        assert 1 <= len(previous) <= 2
        candidate = q / f"{label}-native/final-candidate"
        result = []
        for suffix in (focus_name, f"{focus_name}-reverse")[:len(previous)]:
            replay = compare(f"{label}-{suffix}", allow_source_counter_changes=allow_allocation_change)
            for actor, expected in (("mech-control", control), ("mech-experiment", candidate)):
                pin = replay["pins"]["actors"][actor]
                assert pin["command"][:2] == [str(expected / "mech-worker"), str(expected / "metadata.json")]
            result.append(replay)
    else:
        first = sample(label, focus_name, focus, True)
        result = [first]
        if args.both_orders or keeps_wins(first):
            result.append(sample(label, f"{focus_name}-reverse", focus, False))
        output.write_text(json.dumps(result, indent=2) + "\n")
    focus_passes = len(result) == 2 and all(map(keeps_wins, result))
    if not args.focus_only and (focus_passes or args.full_diagnostic):
        proof(label)
        first = sample(label, "full", source, False)
        second = sample(label, "full-reverse", source, True)
        subprocess.run([sys.executable, "-B", q / "summarize-code-runtime.py", label,
                        "--reference", reference, "--focus-suffix", focus_name], check=True)
        if all(r["roc_wins"][1] == 81 and keeps_wins(r)
               for r in (first, second)):
            break
