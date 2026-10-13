"""Independent report-only reconciliation of actual V full-study records."""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import json
import math
from pathlib import Path
import statistics

import full_study

HERE = Path(__file__).resolve().parent
POWER = dict(source='AC Power', low_power_mode=0)
EMPTY = ('remaining_group_members', 'monitor_errors', 'resource_monitor_errors',
         'supervisor_remaining_members')
FATAL = ('error', 'error_type', 'resource_violation', 'cleanup_error',
         'supervisor_cleanup', 'sampler_failure', 'supervisor_failure',
         'supervisor_pin_failure', 'supervisor_registration_error',
         'monitor_finish_error', 'source_work_error', 'supervisor_error',
         'sampler_error', 'registration_error', 'supervisor_cleanup_error')


def require(value, message):
    if not value:
        raise ValueError(message)


def expected_output(harness, benchmark):
    if benchmark == 'nbody':
        return dict(energies=[-0.169075164, -0.169059907], tolerance=1e-8)
    if benchmark == 'spectralnorm':
        data = b'1.274224153\n'
    elif benchmark == 'fannkuchredux':
        data = b'3968050\nPfannkuchen(12) = 65\n'
    elif benchmark in ('pidigits', 'regexredux'):
        path = full_study.CLBG / benchmark / 'inputs/output-full.txt'
        return dict(bytes=path.stat().st_size, sha256=harness.sha(path))
    else:
        size, digest = harness.compact.HASHES[benchmark][1]
        return dict(bytes=size, sha256=digest)
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def validate_data(manifest, report, digest, harness, reference, read_raw):
    """Recompute three medians from exact saved executions; never score probes."""
    benchmark, size = manifest['benchmark'], manifest['size']
    require(report['status'] == 'complete' and report['active'] is None,
            'Incomplete full study')
    require(report['manifest_sha256'] == digest and report['pins_unchanged_at_end'] is True,
            'Different manifest or changed pins')
    require(not any(report.get(k) for k in FATAL), 'Report retained a fatal error')
    require(manifest['report_only'] is True and manifest['never_chart_eligible'] is True,
            'V report-only policy changed')
    require(type(manifest['samples']) is int and manifest['samples'] == 3,
            'Three actual full samples required')
    require(size == harness.WORK[benchmark][1] and type(size) is int,
            'Canonical full workload changed')
    require(manifest['required_power_profile'] == POWER and
            manifest['shared_timing_lock'] == str(harness.LOCK), 'Environment policy changed')
    rows = {r['id']: r for r in manifest['profiles']}
    require(len(rows) == len(manifest['profiles']) == 3, 'Exact three-profile inventory required')
    require(sum(r['language'] == 'mech' for r in rows.values()) == 1 and
            sum(r['language'] == 'v' for r in rows.values()) == 2, 'Candidate/control inventory changed')
    require({r['variant'] for r in rows.values() if r['language'] == 'v'} == {'compact', 'optimized'},
            'Both V forms required')
    require(set(report['profiles']) == set(rows), 'Summary profile inventory changed')
    require(report['orders'] == manifest['orders'] and len(manifest['orders']) == 3,
            'Recorded order differs')
    queue = []
    for number, order in enumerate(manifest['orders'], 1):
        require(order['round'] == number and len(order['profiles']) == 3 and
                set(order['profiles']) == set(rows), 'Duplicate or missing profile in a round')
        queue.extend((number, identity) for identity in order['profiles'])
    accepted = {identity: [] for identity in rows}
    discarded, position, retry = [], 0, 0
    for number, attempt in enumerate(report['attempts'], 1):
        require(position < len(queue), 'Extra full execution')
        round_index, identity = queue[position]
        row = rows[identity]
        require(attempt['attempt'] == number and attempt['round'] == round_index,
                'Actual execution ordering changed')
        for field in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd'):
            require(attempt[field] == row[field], 'Actual identity changed: ' + field)
        command = [p.replace('{n}', str(size)) for p in row['command']]
        require(attempt['command'] == command and
                attempt['runtime_environment'] == row.get('runtime_environment', {}),
                'Actual command or environment changed')
        require(json.loads(attempt['execution_fingerprint']) ==
                json.loads(harness.execution_fingerprint(row)), 'Execution fingerprint changed')
        require(attempt['full_workload'] is True and type(attempt['workload_n']) is int and
                attempt['workload_n'] == size and not attempt.get('preparation_only'),
                'A bounded or preparation process cannot be a full sample')
        require(type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int and
                attempt['exit_status'] == attempt['sampler_wait_status'] == 0,
                'Native process/sampler did not exit normally')
        require(attempt['pins_unchanged_at_end'] is True and attempt['fatal_qualification_fields'] == [] and
                not any(attempt.get(k) for k in FATAL), 'Failed source/process qualification')
        require(all(attempt[k] == [] for k in EMPTY), 'Cleanup/resource monitor failure')
        require(attempt['maximum_rss_mib'] == manifest['maximum_rss_mib'] and
                attempt['timeout_seconds'] == manifest['timeout_seconds'], 'Resource policy changed')
        peak = attempt['peak_process_group_rss_kib']
        require(type(peak) is int and 0 <= peak <= manifest['maximum_rss_mib'] * 1024,
                'Measured process group exceeded memory bound')
        native = attempt['supervised_native_group']
        require(native['type'] == 'native-group-started' and type(native['pid']) is int and
                native['pid'] > 0 and native['pid'] == native['pgid'] and
                native['command'] == command and native['inherited_common_mutex'] is True,
                'Authoritative native process ownership missing')
        validation = attempt['validation']
        require(validation['reference_match'] is True and
                validation['bytes'] == attempt['stdout_bytes'] and
                validation['sha256'] == attempt['stdout_sha256'], 'Complete output evidence changed')
        if benchmark == 'nbody':
            require(len(validation['energies']) == 2 and validation['tolerance'] == reference['tolerance'],
                    'Energy oracle shape/tolerance changed')
            require(all(math.isfinite(v) and abs(v-e) <= reference['tolerance'] for v, e in
                        zip(validation['energies'], reference['energies'])), 'Full energy oracle failed')
        else:
            require(validation['bytes'] == reference['bytes'] and validation['sha256'] == reference['sha256'],
                    'Whole output differs from independent canonical reference')
        elapsed = attempt['seconds']
        require(type(elapsed) in (int, float) and math.isfinite(elapsed) and elapsed > 0,
                'Invalid actual elapsed time')
        raw = read_raw(attempt['result_path'])
        require('environment_attempt' not in raw, 'Raw record unexpectedly contains parent retry metadata')
        require(dict(raw, environment_attempt=attempt['environment_attempt']) == attempt,
                'Summary changed original saved execution')
        if attempt['qualified'] is not True:
            require(attempt['qualified'] is False and attempt['valid_environment'] is False and
                    attempt['retryable_environment_only'] is True, 'Nonenvironment failure cannot be retried')
            require(attempt['competing_compilers'] or attempt['power_changes'] or
                    attempt['power_at_start'] != POWER or attempt['power_at_end'] != POWER,
                    'Missing evidence for environment exclusion')
            retry += 1
            require(retry < manifest['environment_retry_limit'] and attempt['environment_attempt'] == retry,
                    'Environment retry allowance changed')
            discarded.append(deepcopy(attempt))
            continue
        require(attempt['valid_environment'] is True and attempt['retryable_environment_only'] is False and
                attempt['power_at_start'] == attempt['power_at_end'] == POWER and
                attempt['competing_compilers'] == attempt['power_changes'] == [], 'Invalid sample environment')
        require(attempt['environment_attempt'] == retry + 1, 'Successful retry numbering changed')
        accepted[identity].append(attempt)
        position, retry = position + 1, 0
    require(position == len(queue) and retry == 0, 'Missing full executions')
    result = {}
    for identity, row in rows.items():
        actual = [a['seconds'] for a in accepted[identity]]
        summary = report['profiles'][identity]
        require(len(actual) == 3 and summary['samples_seconds'] == actual and summary['loc'] == row['loc'],
                'Sample/source summary changed')
        require(summary['median_seconds'] == statistics.median(actual) and
                summary['sample_stdev_seconds'] == statistics.stdev(actual), 'Summary statistics do not reconcile')
        result[identity] = dict(language=row['language'], variant=row.get('variant'), source=row['source'],
            source_sha256=row['source_sha256'], samples_seconds=actual,
            median_seconds=statistics.median(actual), loc=row['loc'])
    return result, discarded


def reconcile(benchmark):
    module = full_study.configure(benchmark)
    manifest = json.loads(module.MANIFEST.read_text())
    report = json.loads(module.REPORT.read_text())
    digest = module.sha(module.MANIFEST)
    full_study.review_manifest(module, digest, benchmark)
    harness = module.load_harness()
    require(Path(report['manifest']).resolve() == module.MANIFEST, 'Report names another manifest')
    attempt_root = (module.OUT / 'attempts').resolve()

    def raw(path):
        path = Path(path).resolve(strict=True)
        require(path.is_relative_to(attempt_root), 'Raw execution path escaped its study')
        return json.loads(path.read_text())

    rows, discarded = validate_data(manifest, report, digest, harness,
                                    expected_output(harness, benchmark), raw)
    control = next(r for r in rows.values() if r['language'] == 'mech')
    for row in rows.values():
        row['time_ratio_to_matched_mech'] = row['median_seconds'] / control['median_seconds']
    return dict(benchmark=benchmark, status='reconciled-three-full-samples', profiles=rows,
                environment_exclusions=discarded, manifest=str(module.MANIFEST), manifest_sha256=digest,
                report=str(module.REPORT), report_sha256=module.sha(module.REPORT),
                report_only=True, never_chart_eligible=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=full_study.BENCHMARKS, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    module = full_study.configure(args.benchmark)
    print('WAIT common mutex: independent V full-record reconciliation', args.benchmark, flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        require(not args.output.exists(), 'Never overwrite a result reconciliation')
        result = reconcile(args.benchmark)
        result['reconciler_sha256'] = module.sha(__file__)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open('x') as stream:
            json.dump(result, stream, indent=2)
            stream.write('\n')
        for row in result['profiles'].values():
            print(args.benchmark, row['language'], row['variant'],
                  f"{row['median_seconds']:.6f} s", f"{row['time_ratio_to_matched_mech']:.4f} x Mech", flush=True)


if __name__ == '__main__':
    main()
