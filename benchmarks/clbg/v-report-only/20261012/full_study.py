"""Report-only V comparisons using the frozen shared native-process sampler.

Each benchmark has its own reviewed manifest, three rotated rounds, complete
output checks, and fresh Mech AOT controls. No explorer data are read or written.
"""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
HELPER = SUITE / 'uiua-mandel-parallel-v4/full_study.py'
BENCHMARKS = (
    'regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
    'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux',
)


def configure(benchmark):
    assert benchmark in BENCHMARKS, 'Unknown V benchmark'
    spec = importlib.util.spec_from_file_location('v_shared_durable_sampler', HELPER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.HERE = HERE / 'studies' / benchmark
    module.N = SUITE
    module.OUT = module.HERE / 'full-study'
    module.MANIFEST = module.OUT / 'manifest.json'
    module.REPORT = module.OUT / 'report.json'
    original_attempt = module.one_attempt

    def archived_attempt(harness, row, manifest, report, round_index, lock_fd):
        index = len(report['attempts']) + 1
        raw_path = module.OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
        try:
            return original_attempt(harness, row, manifest, report, round_index, lock_fd)
        except BaseException as error:
            # The shared sampler already cleans its native group in finally.
            # A supervisor/pin exception must also become durable, unqualified
            # evidence; it must never leave a raw success as an admitted sample.
            result = json.loads(raw_path.read_text()) if raw_path.exists() else {}
            active = report.get('active') or {}
            result.update(
                round=round_index, attempt=index, qualified=False,
                retryable_environment_only=False,
                supervisor_failure=True, supervisor_error=str(error),
                supervisor_error_type=type(error).__name__,
                original_result_path=str(raw_path),
                fatal_qualification_fields=['supervisor_failure'],
            )
            if active.get('native'):
                result['supervised_native_group'] = active['native']
            result.setdefault('error', str(error) or type(error).__name__)
            result.setdefault('error_type', type(error).__name__)
            enriched = raw_path.with_suffix('.supervisor.json')
            result['result_path'] = str(enriched)
            module.save(enriched, result)
            return result

    module.one_attempt = archived_attempt
    return module


def review_manifest(module, digest, benchmark):
    assert module.sha(module.MANIFEST) == digest, 'Reviewed V manifest changed'
    manifest = json.loads(module.MANIFEST.read_text())
    assert manifest['status'] == 'prepared-awaiting-root-review'
    assert manifest['benchmark'] == benchmark
    assert manifest['report_only'] is True
    assert manifest['never_chart_eligible'] is True
    assert manifest['samples'] == 3
    profiles = manifest['profiles']
    assert len({row['id'] for row in profiles}) == len(profiles)
    assert {row['benchmark'] for row in profiles} == {benchmark}
    v_rows = [row for row in profiles if row['language'] == 'v']
    controls = [row for row in profiles if row['language'] == 'mech']
    assert len(v_rows) == 2 and len(controls) >= 1
    assert {row['variant'] for row in v_rows} == {'compact', 'optimized'}
    assert len(v_rows) + len(controls) == len(profiles)
    ids = {row['id'] for row in profiles}
    assert [ordered['round'] for ordered in manifest['orders']] == [1, 2, 3]
    for ordered in manifest['orders']:
        assert len(ordered['profiles']) == len(ids)
        assert set(ordered['profiles']) == ids
    for row in v_rows:
        assert row['algorithm']['strict'] is True
        assert row['algorithm']['native_benchmark_kernel'] is False
        assert row['maximum_workers'] <= 8
        assert row['eligible_for_chart'] is False
        assert row['never_chart_eligible'] is True
        assert row['small_validation']['reference_match'] is True
        assert row['required_work_review']['status'] == 'passed'
    harness = module.load_harness()
    assert manifest['size'] == harness.WORK[benchmark][1]
    assert manifest['maximum_compute_workers'] <= 8
    assert manifest['shared_timing_lock'] == str(module.LOCK)
    assert manifest['pins'][str(Path(__file__).resolve())] == module.sha(__file__)
    assert manifest['pins'][str(HELPER)] == module.sha(HELPER)
    harness.PinGuard(manifest['pins']).verify()
    return manifest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=BENCHMARKS, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--run', action='store_true')
    mode.add_argument('--detach', action='store_true')
    parser.add_argument('--manifest-sha256', required=True)
    args = parser.parse_args()
    module = configure(args.benchmark)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        review_manifest(module, args.manifest_sha256, args.benchmark)
        if args.detach:
            with (module.OUT / 'launch-owner.lock').open('a+') as launch_lock:
                fcntl.flock(launch_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                assert not (module.OUT / 'launch.json').exists()
                assert not module.REPORT.exists(), 'No duplicate launch or implicit resume'
                command = [sys.executable, '-u', str(Path(__file__).resolve()),
                           '--run', '--benchmark', args.benchmark,
                           '--manifest-sha256', args.manifest_sha256]
                with (module.OUT / 'supervisor.log').open('ab') as log:
                    process = subprocess.Popen(
                        command, stdin=subprocess.DEVNULL, stdout=log,
                        stderr=subprocess.STDOUT, start_new_session=True,
                    )
                receipt = dict(
                    pid=process.pid, identity=module.identity(process.pid),
                    benchmark=args.benchmark, command=command,
                    manifest_sha256=args.manifest_sha256,
                    launched_at=module.now(), report_only=True,
                    never_chart_eligible=True,
                )
                module.save(module.OUT / 'launch.json', receipt)
                print(json.dumps(receipt, indent=2), flush=True)
                return
    # The shared supervisor takes the mutex for each individual full sample.
    # Its forked sampler and native child inherit that exact descriptor.
    module.run(args.manifest_sha256)


if __name__ == '__main__':
    main()
