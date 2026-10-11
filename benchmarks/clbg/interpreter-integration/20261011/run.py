#!/usr/bin/env python3
"""Matched full-workload interpreter samples before/after the f32 batch import."""
import argparse, fcntl, importlib.util, json, math, os, random, statistics, subprocess
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]
spec = importlib.util.spec_from_file_location('mode_runner', CLBG/'execution-modes/20261010-final/run.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

IMMUTABLE_ROW_FIELDS = ('source_key', 'benchmark', 'mode', 'language', 'source',
    'source_sha256', 'loc', 'physical_loc', 'loc_expansions', 'runtime_environment',
    'timing_boundary', 'key', 'phase', 'binary', 'binary_sha256', 'command')


def validate_resume(report, rows, baseline, sample_count):
    """Reject changed execution metadata before continuing an existing report.

    This does not add pins to earlier samples or modify their stored metadata.
    Source/binary pins are checked separately before every future execution.
    """
    assert report['sample_count'] == sample_count
    assert set(report['programs']) == set(rows)
    assert report['code_path_audit'] == baseline['code_path_audit']
    assert report['power_profile'] == runner.POWER
    assert {key:list(value) for key,value in report['workloads'].items()} == {
        key:list(value) for key,value in runner.WORK.items()}
    completed = report['completed_benchmarks']
    assert len(completed) == len(set(completed)) and set(completed) <= set(runner.ORDER)
    for key, expected in rows.items():
        row = report['programs'][key]
        assert all(row[field] == expected[field] for field in IMMUTABLE_ROW_FIELDS), key
        timings, validations, details = (row[field] for field in
            ('samples_seconds', 'sample_validation', 'sample_details'))
        assert len(timings) == len(validations) == len(details) <= sample_count, key
        for seconds, validation, detail in zip(timings, validations, details):
            assert isinstance(seconds, (int, float)) and not isinstance(seconds, bool)
            assert math.isfinite(seconds) and seconds > 0
            assert validation['reference_match'] is True
            assert detail['seconds'] == detail['kernel_seconds'] == seconds
        if row['benchmark'] in completed:
            assert len(timings) == sample_count, key
    assert set(report['orders']) <= set(runner.ORDER)
    for benchmark, rounds in report['orders'].items():
        keys = {key for key,row in rows.items() if row['benchmark'] == benchmark}
        assert len(rounds) <= sample_count
        assert all(len(order) == len(keys) and set(order) == keys for order in rounds)
    if report['status'] == 'complete':
        assert set(completed) == set(runner.ORDER)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--benchmarks', default=','.join(runner.ORDER))
    parser.add_argument('--timeout', type=int, default=14400)
    args = parser.parse_args()
    assert args.samples >= 3
    baseline = json.loads((HERE/'baseline.json').read_text())
    path = HERE/'timings.json'
    rows = {}
    for source in baseline['programs']:
        for phase in ('before', 'after'):
            binary = baseline['binary' if phase == 'before' else 'after_binary']
            digest = baseline['binary_sha256' if phase == 'before' else 'after_binary_sha256']
            key = source['source_key']+':'+phase
            rows[key] = {k: source[k] for k in ('source_key','benchmark','mode','language','source','source_sha256','loc','physical_loc','loc_expansions','runtime_environment','timing_boundary')}
            rows[key] |= dict(key=key, phase=phase, binary=binary, binary_sha256=digest,
                              command=[binary]+source['command'][1:], samples_seconds=[], sample_validation=[], sample_details=[])
    report = json.loads(path.read_text()) if path.exists() else dict(
        status='running', created_at=datetime.now(timezone.utc).isoformat(), sample_count=args.samples,
        baseline_sha256=runner.sha(HERE/'baseline.json'), code_path_audit=baseline['code_path_audit'],
        power_profile=runner.POWER, workloads=runner.WORK,
        machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),
        programs=rows, orders={}, completed_benchmarks=[], discarded=[],
        protocol=f'{args.samples} fresh loaded-interpreter runs per source and revision at canonical full workloads. Matched randomized sequential rounds, shared timing lock, AC power and Low Power Mode off. Full IO and flush are inside execution time; preparation and whole-process elapsed time retained separately. No source or workload changes.')
    assert report['sample_count'] == args.samples and report['baseline_sha256'] == runner.sha(HERE/'baseline.json')
    validate_resume(report, rows, baseline, args.samples)
    pins = {str(ROOT/k):v for k,v in baseline['source_files'].items()}
    for row in rows.values():
        pins[row['source']] = row['source_sha256']
        pins[row['binary']] = row['binary_sha256']
    cached = {}
    def verify():
        for file,digest in pins.items():
            stat = os.stat(file)
            state = (stat.st_dev,stat.st_ino,stat.st_size,stat.st_mtime_ns,stat.st_ctime_ns)
            if cached.get(file) != state:
                assert runner.sha(file) == digest, file
                cached[file] = state
    verify()
    runner.save(path,report)
    for bench in args.benchmarks.split(','):
        assert bench in runner.ORDER
        candidates = [r for r in report['programs'].values() if r['benchmark']==bench]
        randomizer = random.Random(20261011+sum(map(ord,bench)))
        for sample in range(args.samples):
            order = candidates.copy()
            randomizer.shuffle(order)
            rounds = report['orders'].setdefault(bench,[])
            if len(rounds)<=sample:
                rounds.append([r['key'] for r in order])
            else:
                assert rounds[sample]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>sample:
                    continue
                while True:
                    with open('/tmp/mech-clbg-timing.lock','a') as lock:
                        fcntl.flock(lock,fcntl.LOCK_EX)
                        runner.guard.wait_for_environment(runner.POWER)
                        verify()
                        report['active_run'] = dict(key=row['key'],sample=sample+1,started_at=datetime.now(timezone.utc).isoformat())
                        runner.save(path,report)
                        print('RUN',sample+1,row['key'],flush=True)
                        result = runner.execute(row,True,args.timeout)
                    if result['competing_compilers'] or result['power_changes']:
                        report['discarded'].append(dict(key=row['key'],sample=sample+1,result=result))
                        runner.save(path,report)
                        continue
                    row['samples_seconds'].append(result['seconds'])
                    row['sample_validation'].append(result['validation'])
                    row['sample_details'].append({k:v for k,v in result.items() if k not in ('validation','competing_compilers','power_changes')})
                    row['median_seconds'] = statistics.median(row['samples_seconds'])
                    row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds']) if len(row['samples_seconds'])>1 else None
                    row['full_validation'] = result['validation']
                    report.pop('active_run',None)
                    runner.save(path,report)
                    print('SAMPLE',sample+1,row['key'],f"{result['seconds']:.6f}s",flush=True)
                    break
        if bench not in report['completed_benchmarks']:
            report['completed_benchmarks'].append(bench)
        runner.save(path,report)
    report['changes'] = {}
    for source in baseline['programs']:
        key = source['source_key']
        before,after = (report['programs'][key+':'+p] for p in ('before','after'))
        if before.get('median_seconds') and after.get('median_seconds'):
            report['changes'][key] = dict(benchmark=source['benchmark'],loc=source['loc'],
                before_seconds=before['median_seconds'],after_seconds=after['median_seconds'],
                after_over_before=after['median_seconds']/before['median_seconds'],
                before_stdev_seconds=before['sample_stdev_seconds'],after_stdev_seconds=after['sample_stdev_seconds'])
    report['status'] = 'complete' if set(report['completed_benchmarks'])==set(runner.ORDER) else 'partial'
    report['finished_at'] = datetime.now(timezone.utc).isoformat()
    verify()
    runner.save(path,report)
    print('REPORT',report['status'],flush=True)

if __name__=='__main__':
    main()
