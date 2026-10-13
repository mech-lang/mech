"""Complete Pony compact/optimized suite with fresh unchanged Mech controls.

Each benchmark uses its own reviewed manifest and actual three-process samples;
source/hash/compilation checks are outside elapsed timing. All ownership and
complete-output qualification comes from the frozen shared sampler guard.
"""
import argparse
import copy
import fcntl
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys

HERE = Path(__file__).resolve().parent
COHORT = HERE.parents[1]
CLBG = COHORT.parents[1]
sys.path.insert(0, str(COHORT))
import guarded_prepare_v6 as preparation
LOCK = preparation.LOCK
SOURCE_AUDIT = CLBG / 'r-apl-uiua-comparison/20261011/source-audit-r.json'
MANDEL = CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'


def full_reference(harness, benchmark):
    if benchmark == 'nbody': return None
    if benchmark in {'spectralnorm', 'fannkuchredux'}:
        data = b'1.274224153\n' if benchmark == 'spectralnorm' else b'3968050\nPfannkuchen(12) = 65\n'
        return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
    if benchmark in {'pidigits', 'regexredux'}:
        reference = CLBG / benchmark / 'inputs/output-full.txt'
        return dict(bytes=reference.stat().st_size, sha256=preparation.sha(reference))
    size, digest = harness.compact.HASHES[benchmark][1]
    return dict(bytes=size, sha256=digest)


def seal(revision, qualification, work_revision, allocation_receipt, allocation_sha256):
    output = COHORT / 'full-studies' / revision
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not output.exists(), 'Never replace a reviewed study revision'
        report_path = COHORT / 'qualification' / qualification / 'report.json'
        q = json.loads(report_path.read_text())
        assert q['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
        work_path = COHORT / 'work-qualification' / work_revision / 'report.json'
        work = json.loads(work_path.read_text())
        assert work['status'] == 'native-work-v2-passed-awaiting-scored-binary-allocation-review'
        assert work['source_work_profiles'] == 20 and work['source_work_trace_profiles'] == len(work['attempts']) == 6
        assert all(row['qualified'] is True and row['complete_output_match'] is True for row in work['attempts'])
        reviewed = {row['id']: row for row in work['reviewed_sources']}
        assert set(reviewed) == {row['id'] for row in q['profiles']}
        guard = preparation.load_guard(); harness = guard.load_harness()
        import full_prerequisite
        allocation_path = Path(allocation_receipt).resolve()
        allocation = json.loads(allocation_path.read_text())
        assert allocation['receipt_path'] == str(allocation_path)
        tree_gate = full_prerequisite.require_tree_proof(q, work, allocation, allocation_sha256,
            preparation.sha, lambda path: json.loads(Path(path).read_text()))
        pins = preparation.merge(q['pins'], work['pins'], {str(report_path): preparation.sha(report_path), str(work_path): preparation.sha(work_path)})
        allocation_pins = {str(allocation_path): allocation_sha256}
        for review in allocation['profiles'].values():
            for field in ('production_build_receipt', 'arm_disassembly', 'llvm_ir'):
                artifact = review[field]
                allocation_pins[artifact['path']] = artifact['sha256']
        pins = preparation.merge(pins, allocation_pins)
        audit = json.loads(SOURCE_AUDIT.read_text()); assert audit['status'] == 'reviewed'
        mandel = json.loads(MANDEL.read_text())
        pins = preparation.merge(pins, audit['pins'], mandel['pins'],
                                 {str(SOURCE_AUDIT): preparation.sha(SOURCE_AUDIT), str(MANDEL): preparation.sha(MANDEL)},
                                 {str(path): preparation.sha(path) for path in HERE.glob('*.py')})
        rows = []
        for row in q['profiles']:
            assert reviewed[row['id']]['source'] == row['source'] and reviewed[row['id']]['source_sha256'] == row['source_sha256']
            assert 1 <= row['maximum_compute_actors'] <= 8
            assert row['command'][1:] == ['{n}', '--ponymaxthreads', '8']
            assert preparation.sha(row['source']) == row['source_sha256'] and preparation.sha(row['command'][0]) == row['binary_sha256']
            rows.append(dict(row, binary=row['command'][0], loc=None, role='unchanged qualified Pony ' + row['variant'], execution_mode='native-aot', profile=row['variant'], source_profile=row['variant'], qualification_status='complete-bounded-output-and-production-work', measurement_status='pending', samples_seconds=[], sample_validation=[], attempts=[]))
        for benchmark in harness.BENCHES:
            if benchmark == 'mandelbrot':
                control = copy.deepcopy(next(row for row in mandel['profiles'] if row['id'] == 'row16-aot-control'))
            else:
                control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == benchmark))
            assert preparation.sha(control['source']) == control['source_sha256'] and preparation.sha(control['binary']) == control['binary_sha256']
            pins[control['source']] = control['source_sha256']; pins[control['binary']] = control['binary_sha256']
            control.update(id='mech-' + benchmark + '-control', key=benchmark + ':pony-full:mech-control', role='unchanged current Mech fast source with fresh three canonical processes', measurement_status='pending', samples_seconds=[], sample_validation=[], attempts=[])
            rows.append(control)
        assert len(rows) == 30
        output.mkdir(parents=True)
        common = dict(pins=pins, status='prepared-awaiting-root-review', samples=3, canonical_full_workload=True,
                      shared_timing_lock=str(LOCK), timeout_seconds=14400, maximum_rss_mib=10240,
                      maximum_compute_workers=8, required_power_profile=harness.REQUIRED,
                      qualification=str(report_path), qualification_sha256=preparation.sha(report_path),
                      source_work_qualification=str(work_path), source_work_qualification_sha256=preparation.sha(work_path),
                      tree_allocation_prerequisite=tree_gate,
                      source_metric='LOC null pending qualified canonical LOC audit; no estimate is used',
                      timing_boundary='Fresh process runtime/input/all workload/output/shutdown plus identical authoritative PGID registration. Compiler/source review/validation/hash outside elapsed.',
                      ownership='Frozen separate sampler and native process inherit held common mutex, authoritative PGID registration, full cleanup before release.',
                      retry_policy='Every failure preserved. Confirmed source output/time/RSS failure marks that exact profile failed and independent sources continue. Environment/pin/process/ownership failure stops controller. No automatic resumption/retry; corrected source requires new reviewed study.',
                      full_performance_runs_required=90, chart_policy='User requested Pony publication after all ten workloads complete; report actual three samples and current fresh Mech control per benchmark.')
        manifests = {}
        for benchmark in harness.BENCHES:
            profiles = [row for row in rows if row['benchmark'] == benchmark]
            assert len(profiles) == 3
            identifiers = [row['id'] for row in profiles]
            orders = [dict(round=index + 1, profiles=identifiers[index:] + identifiers[:index]) for index in range(3)]
            manifest = dict(common, benchmark=benchmark, size=harness.WORK[benchmark][1], profiles=profiles, orders=orders,
                            full_output_reference=full_reference(harness, benchmark), required_processes=9)
            path = output / benchmark / 'manifest.json'
            preparation.new_json(path, manifest)
            manifests[benchmark] = dict(path=str(path), sha256=preparation.sha(path))
        guard.verify(pins)
        suite = dict(common, benchmark_order=harness.BENCHES, benchmarks=manifests, profiles=rows)
        preparation.new_json(output / 'manifest.json', suite)
        print(json.dumps(dict(manifest=str(output / 'manifest.json'), sha256=preparation.sha(output / 'manifest.json'), benchmarks=manifests), indent=2), flush=True)


def isolated_profile_failure(guard, attempt):
    """Require literal clean ownership gates before isolating a real source failure."""
    safe = ('sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
            'no_wrapper_integrity_error', 'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
            'no_monitor_finish_error', 'no_execute_cleanup_error', 'no_remaining_execute_group',
            'no_supervisor_cleanup', 'no_supervisor_remaining_group', 'valid_environment',
            'actual_full_workload', 'exact_command_and_group', 'pins_unchanged')
    checks = attempt.get('qualification_checks', {})
    return guard.benchmark_failure(attempt) and all(checks.get(key) is True for key in safe)


def run(revision, digest):
    output = COHORT / 'full-studies' / revision
    with (output / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        report_path = output / 'report.json'
        assert not report_path.exists(), 'Existing report requires new explicit recovery review'
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            guard = preparation.load_guard(); harness = guard.load_harness(); harness.install_interrupt_handlers()
            assert preparation.sha(output / 'manifest.json') == digest
            suite = json.loads((output / 'manifest.json').read_text())
            guard.verify(suite['pins'])
        report = dict(status='running', manifest=str(output / 'manifest.json'), manifest_sha256=digest,
                      sample_count=3, programs={row['key']: copy.deepcopy(row) for row in suite['profiles']},
                      attempts=[], active=None, completed_benchmarks=[], benchmark_order=suite['benchmark_order'],
                      full_performance_runs=0, source_metric=suite['source_metric'], source_work_qualification=suite['source_work_qualification'],
                      pins=suite['pins'], maximum_rss_mib=suite['maximum_rss_mib'], shared_timing_lock=str(LOCK))
        guard.OUT = output; guard.REPORT = report_path
        guard.save(report_path, report)
        try:
            for benchmark in suite['benchmark_order']:
                declared = suite['benchmarks'][benchmark]
                with LOCK.open('a+') as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    assert preparation.sha(declared['path']) == declared['sha256']
                    manifest = json.loads(Path(declared['path']).read_text())
                    guard.verify(manifest['pins'])
                rows = {row['id']: report['programs'][row['key']] for row in manifest['profiles']}
                for order in manifest['orders']:
                    for identifier in order['profiles']:
                        row = rows[identifier]
                        if row.get('full_failure'):
                            continue
                        with LOCK.open('a+') as lock:
                            fcntl.flock(lock, fcntl.LOCK_EX)
                            harness.compact.wait_for_environment(harness.REQUIRED)
                            guard.verify(manifest['pins'])
                            print('FULL PONY', benchmark, order['round'], identifier, manifest['size'], flush=True)
                            attempt = guard.one_attempt(harness, row, manifest, report, order['round'], lock.fileno())
                            report['attempts'].append(attempt); report['active'] = None
                            row['attempts'].append(attempt['attempt'])
                            report['full_performance_runs'] += 1
                            if attempt['qualified']:
                                row['samples_seconds'].append(attempt['seconds']); row['sample_validation'].append(attempt['validation'])
                                row.setdefault('full_validation', attempt['validation'])
                                row['measurement_status'] = 'measured' if len(row['samples_seconds']) == 3 else 'partial'
                            elif isolated_profile_failure(guard, attempt):
                                row['measurement_status'] = 'failed'
                                row['full_failure'] = dict(attempt)
                                print('CONFIRMED SOURCE FAILURE', benchmark, identifier, attempt.get('error'), flush=True)
                            else:
                                row['measurement_status'] = 'failed-qualification'
                            guard.save(report_path, report)
                            if not attempt['qualified'] and not row.get('full_failure'):
                                raise AssertionError(('Full integrity/environment/ownership failure retained; controller stops', benchmark, identifier, attempt.get('error')))
                assert all(len(row['samples_seconds']) == 3 or row.get('full_failure') for row in rows.values())
                for row in rows.values():
                    if not row.get('full_failure'):
                        row['median_seconds'] = statistics.median(row['samples_seconds']); row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
                        row['qualification_status'] = 'three-canonical-processes-qualified-awaiting-canonical-LOC-publication'
                report['completed_benchmarks'].append(benchmark); guard.save(report_path, report)
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX); guard.verify(suite['pins'])
                assert len(report['attempts']) == report['full_performance_runs']
                assert len(report['attempts']) == sum(len(row['samples_seconds']) + bool(row.get('full_failure')) for row in report['programs'].values())
                assert len(report['completed_benchmarks']) == 10
                assert all(len(row['samples_seconds']) == 3 or row.get('full_failure') for row in report['programs'].values())
                failed = [row['id'] for row in report['programs'].values() if row.get('full_failure')]
                if not failed:
                    assert len(report['attempts']) == 90
                report.update(status='complete-with-failures' if failed else 'complete', pins_unchanged_at_end=True, correctable_source_failures=failed, goal_complete=False if failed else None)
        except BaseException as error:
            report.update(status='stopped', error=str(error) or type(error).__name__, error_type=type(error).__name__)
            raise
        finally:
            guard.save(report_path, report)


def main():
    parser = argparse.ArgumentParser(); modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--seal', action='store_true'); modes.add_argument('--run', action='store_true')
    parser.add_argument('--revision', required=True); parser.add_argument('--qualification', default='private-v7')
    parser.add_argument('--work-revision', default='work-v2'); parser.add_argument('--manifest-sha256')
    parser.add_argument('--allocation-receipt'); parser.add_argument('--allocation-sha256')
    args = parser.parse_args(); assert args.revision and args.revision.replace('-', '').isalnum()
    if args.seal:
        assert args.allocation_receipt and args.allocation_sha256
        seal(args.revision, args.qualification, args.work_revision, args.allocation_receipt, args.allocation_sha256)
    else:
        assert args.manifest_sha256; run(args.revision, args.manifest_sha256)

if __name__ == '__main__': main()
