"""Compare exact compact artifacts with refused general SSA skipping off and on."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


SWITCH = "MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA"
SKIP_MARKER = "compact-stage: skipped general numeric SSA "
PHASE_MARKER = "compact-stage: numeric phase "
SCENARIOS = {
    "recorded": {},
    "assertions-off": {"MECH_PHASE_ASSERTION_FLOORS": None},
    "compare-ir": {"MECH_COMPACT_REUSE_EQUIVALENT_VECTOR_IR": "1"},
    "share-bodies": {"MECH_COMPACT_SHARE_PHASE_BODIES": "1"},
    "keep-general": {"MECH_COMPACT_KEEP_REFUSED_PHASE_CODE": "1"},
    "unrequested-refusal": {"MECH_COMPACT_REFUSE_GENERAL_VECTORS": None},
    "unavailable-vector": {"MECH_PHASE_VECTOR_FAST": None},
    "absent-private-proof": {"MECH_PRIVATE_NESTED_PHASES": None},
    "general-ir-dump": {},
}


def digest(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def sha256_artifacts(folder):
    return {
        file: digest(folder / file)
        for file in ("program.mecb", "program.o", "metadata.json")
    }


def omitted_phases(metadata):
    return {
        phase["symbol"]: phase
        for phase in metadata["numeric_phases"]
        if phase.get("compilation") == "omitted: geometry uses checked continuation"
    }


def compile_artifact(compiler, source, case, side, native_flags, dump_symbol):
    folder = case / side
    env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
    env.update(native_flags)
    env["PATH"] = "/usr/bin:/bin"
    if side == "on":
        env[SWITCH] = "1"
    if dump_symbol:
        env["MECH_INSPECT_PHASE_CLIF"] = str(case / f"{side}-dump" / f"{dump_symbol}.clif")
        Path(env["MECH_INSPECT_PHASE_CLIF"]).parent.mkdir()
    with (case / f"{side}.log").open("w") as log:
        result = subprocess.run(
            [compiler, source, folder, "compact"], env=env, stdout=log, stderr=subprocess.STDOUT
        )
    lines = (case / f"{side}.log").read_text().splitlines()
    return folder, result.returncode, lines


def stage_order(lines):
    return [line.removeprefix(PHASE_MARKER) for line in lines if line.startswith(PHASE_MARKER)]


def skipped(lines):
    return sorted(line.removeprefix(SKIP_MARKER) for line in lines if line.startswith(SKIP_MARKER))


def run_scenario(args, receipt, scenario, dump_symbol):
    folder = args.output / scenario
    folder.mkdir()
    flags = dict(receipt["native_flags"])
    flags.pop(SWITCH, None)
    for name, value in SCENARIOS[scenario].items():
        if value is None:
            flags.pop(name, None)
        else:
            flags[name] = value
    # Machine placement profiles bind the exact recorded code.
    # The other scenarios may change the set or shape of those bodies.
    if scenario != "recorded":
        for option in ("MECH_COMPACT_PHASE_BLOCK_LAYOUT", "MECH_COMPACT_ROOT_TAIL_PROFILE"):
            flags.pop(option, None)
    if scenario != "general-ir-dump":
        dump_symbol = None

    before, before_status, off_log = compile_artifact(
        args.compiler, args.source, folder, "off", flags, dump_symbol
    )
    after, after_status, on_log = compile_artifact(
        args.compiler, args.source, folder, "on", flags, dump_symbol
    )
    assert before_status == after_status, scenario
    assert not skipped(off_log), scenario
    assert stage_order(off_log) == stage_order(on_log), scenario
    requested_skip = scenario in {"recorded", "assertions-off"}
    if not requested_skip:
        assert not skipped(on_log), scenario
    if scenario in {"unavailable-vector", "absent-private-proof"}:
        assert before_status != 0, scenario
        reason = "geometry refusal needs a selected vector phase"
        old_errors = [line for line in off_log if reason in line]
        new_errors = [line for line in on_log if reason in line]
        assert old_errors and old_errors == new_errors, scenario
        return {"status": before_status, "diagnostic": old_errors, "skipped": []}

    assert before_status == 0, scenario
    artifacts = sha256_artifacts(before)
    assert artifacts == sha256_artifacts(after), scenario
    if scenario == "recorded":
        assert artifacts["program.o"] == receipt["object_sha256"], scenario
        assert artifacts["metadata.json"] == receipt["metadata_sha256"], scenario
    metadata = json.loads((before / "metadata.json").read_text())
    entries = omitted_phases(metadata)
    if requested_skip:
        assert skipped(on_log), scenario
        for symbol in skipped(on_log):
            phase = entries[symbol]
            assert phase["phase_entry_bounds"] == phase["vector_variant"]["phase_entry_bounds"], symbol
            assert phase["vector_variant"]["selected_by_guard"] is True, symbol
            if scenario == "assertions-off":
                assert phase["phase_entry_bounds"] == [], symbol
    if dump_symbol:
        for name in (f"{dump_symbol}.clif", f"{dump_symbol}.proof.txt"):
            assert (folder / "off-dump" / name).read_bytes() == (folder / "on-dump" / name).read_bytes(), name
    return {
        "status": before_status,
        "sha256": artifacts,
        "skipped": skipped(on_log),
        "entry_bounds": {name: phase["phase_entry_bounds"] for name, phase in entries.items()},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("compiler", type=Path, help="prebuilt compact compile_profile with the patch")
    parser.add_argument("receipt", type=Path, help="pinned direct-source rebuild.json with native_flags")
    parser.add_argument("source", type=Path, help="same program.mec as the pinned receipt")
    parser.add_argument("output", type=Path, help="new directory for the differential receipts")
    parser.add_argument("--scenario", choices=sorted(SCENARIOS) + ["all"], action="append")
    args = parser.parse_args()
    args.compiler = args.compiler.resolve(strict=True)
    args.source = args.source.resolve(strict=True)
    args.output = args.output.resolve()
    receipt = json.loads(args.receipt.read_text())
    assert receipt["compiler_pipeline"] == "direct-source"
    scenarios = args.scenario or ["recorded"]
    if "all" in scenarios:
        scenarios = list(SCENARIOS)
    symbol = None
    if "general-ir-dump" in scenarios:
        metadata = json.loads((args.receipt.parent / "native" / "metadata.json").read_text())
        symbol = next(iter(omitted_phases(metadata)))
    args.output.mkdir()
    result = {case: run_scenario(args, receipt, case, symbol) for case in scenarios}
    summary = json.dumps(result, indent=2) + "\n"
    (args.output / "result.json").write_text(summary)
    print(summary, end="")


if __name__ == "__main__":
    main()
