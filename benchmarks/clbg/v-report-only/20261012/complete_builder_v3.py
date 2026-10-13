"""Advance qualified V drafts into separate report-only full studies.

Preserves existing qualification, work traces and timing records. Every native
stage owns its processes and acquires the shared timing mutex independently.
Tree allocator review remains an explicit root inspection of generated C.
"""
import fcntl
import json
import os
from pathlib import Path
import signal
import sys
import time

import full_study
import prepare_cohort

HERE = Path(__file__).resolve().parent
SOURCE_REVISION = "production-builder-v3"
WORK_REVISION = "builder-v3-work1"
COHORT = HERE / "qualification" / SOURCE_REVISION / "cohort.json"
OUT = HERE / "completion" / "builder-v3-run1"
REPORT = OUT / "status.json"


def read(path):
    return json.loads(path.read_text())


def require_preparation(module, benchmark, report):
    path = HERE / "qualification" / SOURCE_REVISION / benchmark / "probes.json"
    while True:
        cohort = read(COHORT)
        if path.exists():
            value = read(path)
            if value["status"] == "bounded-outputs-passed":
                return path
            if value["status"] == "stopped":
                raise RuntimeError("Stopped V preparation requires a new correction: " + benchmark)
        if cohort["status"] == "stopped":
            raise RuntimeError("V preparation coordinator stopped: " + str(cohort.get("error")))
        identity = cohort["supervisor"]
        if module.identity(identity["pid"]) != identity:
            raise RuntimeError("V preparation handle is missing or changed; inspect before recovery")
        report.update(status="waiting-for-live-preparation", waiting_benchmark=benchmark,
                      preparation_supervisor=identity)
        module.save(REPORT, report)
        time.sleep(15)


def stage(module, report, name, command):
    path = OUT / (name + ".log")
    print("STAGE", name, flush=True)
    report.update(status="running-stage", waiting_benchmark=None)
    with path.open("xb") as stream:
        status = prepare_cohort.owned_preparation(module, command, stream, report, REPORT,
                                                dict(stage=name, log=str(path)))
    if status != 0:
        raise RuntimeError("Owned V stage failed; retained without automatic retry: " + name)
    report.setdefault("completed_stages", []).append(name)
    module.save(REPORT, report)


def main():
    module = full_study.configure("nbody")
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, prepare_cohort.interrupted)
    OUT.mkdir(parents=True, exist_ok=True)
    with (OUT / "owner.lock").open("a+") as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not REPORT.exists(), "Never overwrite or implicitly resume a completion controller"
        report = dict(status="starting", started_at=module.now(), supervisor=module.identity(os.getpid()),
                      report_only=True, never_chart_eligible=True, active=None,
                      completed_stages=[], launched_studies=[], pending_root_reviews=[])
        module.save(REPORT, report)
        try:
            for benchmark in full_study.BENCHMARKS:
                if benchmark in ("regexredux", "revcomp"):
                    continue
                require_preparation(module, benchmark, report)
                extra = []
                if benchmark in ("pidigits", "knucleotide"):
                    stage(module, report, benchmark + "-work", [sys.executable, "-B", "-u",
                        str(HERE / "qualify_work_trace.py"), "--benchmark", benchmark,
                        "--source-revision", SOURCE_REVISION, "--revision", WORK_REVISION])
                    work = HERE / "work-qualification" / WORK_REVISION / benchmark / "report.json"
                    with module.LOCK.open("a+") as lock:
                        fcntl.flock(lock, fcntl.LOCK_EX)
                        assert read(work)["status"] == "passed"
                        extra = ["--work-report", str(work), "--work-report-sha256", module.sha(work)]
                if benchmark == "binarytrees":
                    stage(module, report, "binarytrees-work", [sys.executable, "-B", "-u",
                        str(HERE / "qualify_binarytrees.py"), "--source-revision", SOURCE_REVISION,
                        "--revision", WORK_REVISION])
                    stage(module, report, "binarytrees-lowering", [sys.executable, "-B", "-u",
                        str(HERE / "preserve_tree_lowering.py"), "--source-revision", SOURCE_REVISION,
                        "--revision", WORK_REVISION])
                    report["pending_root_reviews"].append(dict(benchmark=benchmark,
                        lowering=str(HERE / "lowering-evidence" / WORK_REVISION / "report.json"),
                        work=str(HERE / "work-qualification" / WORK_REVISION / benchmark / "report.json")))
                    module.save(REPORT, report)
                    continue
                stage(module, report, benchmark + "-seal", [sys.executable, "-B", "-u",
                    str(HERE / "seal_builder_v3_study.py"), "--benchmark", benchmark,
                    "--source-revision", SOURCE_REVISION] + extra)
                study = full_study.configure(benchmark)
                with module.LOCK.open("a+") as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    digest = study.sha(study.MANIFEST)
                    full_study.review_manifest(study, digest, benchmark)
                stage(module, report, benchmark + "-launch", [sys.executable, "-B", "-u",
                    str(HERE / "full_study.py"), "--detach", "--benchmark", benchmark,
                    "--manifest-sha256", digest])
                report["launched_studies"].append(dict(benchmark=benchmark, manifest=str(study.MANIFEST),
                                                       manifest_sha256=digest))
                module.save(REPORT, report)
            report.update(status="studies-launched-awaiting-measurements-and-root-tree-review", finished_at=module.now())
        except BaseException as error:
            report.update(status="stopped", error=str(error), error_type=type(error).__name__,
                          finished_at=module.now(), no_automatic_retry=True)
            raise
        finally:
            module.save(REPORT, report)


if __name__ == "__main__":
    main()
