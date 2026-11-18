"""Link the static candidate with the pinned fixed runtime and test its contract."""

from argparse import ArgumentParser
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from hashlib import file_digest

study = Path(__file__).resolve().parent
q = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
control = q / "native-size-20261117/source-aligned"
candidate = study / "static-candidate"
control_checks = study / "check-control"


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def link_command(folder):
    original = json.loads((control / "runtime.json").read_text())
    positions = {
        str(control / "program.o"): str(folder / "program.o"),
        str(control / "mech-worker"): str(folder / "mech-worker"),
    }
    link = list(original["link"])
    for before, after in positions.items():
        assert sum(argument.count(before) for argument in link) == 1
        link = [argument.replace(before, after) for argument in link]
    return original | {"link": link}


def run_existing(script, arguments, replacements):
    """Use the as-run correctness harness with private outputs."""
    source = (q / script).read_text()
    for original, changed in replacements:
        assert source.count(original) == 1, original
        source = source.replace(original, changed)
    sys.path.insert(0, str(q))
    before = sys.argv
    try:
        sys.argv = [str(q / script), *map(str, arguments)]
        exec(compile(source, str(q / script), "exec"), {"__file__": str(q / script), "__name__": "__main__"})
    finally:
        sys.argv = before
        assert sys.path.pop(0) == str(q)


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--stack", action="store_true")
    args = parser.parse_args()
    for key in list(os.environ):
        if key.startswith("MECH_"):
            del os.environ[key]
    report = json.loads((candidate / "report.json").read_text())
    assert sha(control / "program.o") == report["reference_sha256"]
    assert sha(candidate / "program.o") == report["candidate_sha256"]
    clean = json.loads((control / "metadata.json").read_text())
    modified = json.loads((candidate / "metadata.json").read_text())
    assert clean | {"code_bytes": report["root_instruction_bytes"]} == modified
    runtime = link_command(candidate)
    runtime["diagnostic_object_postprocess"] = {
        "scope": "Exact physical transfer copies share an existing fallthrough; flags describe the baseline input.",
        "probe_sha256": sha(study / "shared_copy_tails.py"),
        "report_sha256": sha(candidate / "report.json"),
    }
    write(candidate / "runtime.json", runtime)
    subprocess.run(runtime["link"], check=True, cwd=q)
    run_existing("verify-worker.py", [candidate / "mech-worker", candidate / "metadata.json", "shared-copy-tails"], [
        ("root = Path(__file__).resolve().parent", f"root = Path({str(study)!r})"),
        ("fixture_path = root/'fixtures.json'", f"fixture_path = Path({str(q / 'fixtures.json')!r})"),
    ])
    failure_support = study / "failure-support"
    failure_support.mkdir()
    for name in ("pinned_allocator.rs", "mech-worker.rs"):
        shutil.copyfile(q / "pinned-host-link" / name, failure_support / name)
    control_checks.mkdir()
    for name in ("program.o", "metadata.json"):
        shutil.copyfile(control / name, control_checks / name)
    write(control_checks / "runtime.json", link_command(control_checks))
    os.environ["MECH_ERROR_REPORT"] = str(study / "checked-failure-report.json")
    run_existing("check-linked-error-exits.py", [control_checks, candidate], [
        ("link_root = q / 'pinned-host-link'", f"link_root = Path({str(failure_support)!r})"),
    ])
    if args.stack:
        # Preserve the existing instrumented worker's file directory for its
        # relative allocator inclusion. Writes stay in this study.
        stack_support = study / "stack-support"
        stack_support.mkdir()
        shutil.copyfile(q / "pinned-host-link/pinned_allocator.rs", stack_support / "pinned_allocator.rs")
        os.environ["MECH_STACK_REPORT"] = str(study / "stack.json")
        run_existing("check-layout-stack.py", [str(control_checks), str(candidate)], [
            ("c_object=q/'controller-stack-probe.o'", f"c_object=Path({str(stack_support / 'probe.o')!r})"),
            ("instrumented=original.with_name('controller-stack-worker.rs')",
             f"instrumented=Path({str(stack_support / 'controller-stack-worker.rs')!r})"),
            ("prefix='layout-stack-'+label.replace('/', '-')+suffix", "prefix='layout-stack-'+p.name+suffix"),
            ("subprocess.run([sys.executable,'-B',q/'verify-worker.py',worker,p/'metadata.json',prefix],check=True)",
             f"verifier = (q/'verify-worker.py').read_text().replace(\"root = Path(__file__).resolve().parent\", "
             f"\"root = Path({str(study)!r})\").replace(\"fixture_path = root/'fixtures.json'\", "
             f"\"fixture_path = Path({str(q / 'fixtures.json')!r})\")\n"
             f"    subprocess.run([sys.executable,'-B','-c',verifier,str(worker),str(p/'metadata.json'),prefix],check=True)"),
            ("q/(prefix+'-smokes.stderr')", f"Path({str(study)!r})/(prefix+'-smokes.stderr')"),
        ])
    write(study / "linked-identities.json", {
        "control": {name: sha(control / name) for name in ("program.o", "metadata.json", "mech-worker")},
        "candidate": {name: sha(candidate / name) for name in ("program.o", "metadata.json", "mech-worker")},
    })


if __name__ == "__main__":
    main()
