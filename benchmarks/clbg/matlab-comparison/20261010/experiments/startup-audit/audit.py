"""Separate fresh MATLAB launch overhead from full, unchanged benchmark calls."""
import hashlib
import importlib.util
import json
import re
import statistics
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
D = HERE.parents[1]
spec = importlib.util.spec_from_file_location('matlab_runner', D / 'run.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
compact = runner.compact
built = json.loads((D / 'results/builds.json').read_text())
prior = json.loads((D / 'results/timings.json').read_text())
rows = [next(r for r in built['programs'] if r['benchmark'] == bench and r['profile'] == 'optimized')
        for bench in ['nbody', 'spectralnorm']]
pins = {r['source']: r['source_sha256'] for r in rows}
pins.update({r['path']: r['sha256'] for r in built['runtimes'].values()})
pins[str(D / 'results/builds.json')] = runner.sha(D / 'results/builds.json')
pins[str(D / 'results/timings.json')] = runner.sha(D / 'results/timings.json')

report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
    protocol='Diagnostic only; no chart timings replaced. Three sequential fresh-process no-op launches per mode. '
        'One fresh session per mode calls each unchanged full-size benchmark three times, resetting all benchmark state each call. '
        'MATLAB tic/toc surrounds the complete benchmark function, including its output. First call includes first-use JIT; '
        'later calls are warm. No-op startup is reported separately, never subtracted to invent kernel timings. '
        'Default mode uses the same command and environment as the chart. The no-JVM mode adds only -nojvm and cannot serve pi digits. '
        'AC power, Low Power Mode off, compiler/power guards, complete output validation after timing.',
    power=runner.REQUIRED, source_pins=pins, no_op=[], sessions=[],
    chart_fresh_process_seconds={r['benchmark']: prior['programs'][r['key']]['median_seconds'] for r in rows},
    chart_full_validation={r['benchmark']: prior['programs'][r['key']]['full_validation'] for r in rows})
if (HERE / 'report.json').exists():
    previous = json.loads((HERE / 'report.json').read_text())
    assert previous['source_pins'] == pins and previous['protocol'] == report['protocol']
    report = previous


def save():
    path = HERE / 'report.json'
    tmp = path.with_suffix('.json.tmp')
    tmp.write_text(json.dumps(report, indent=2) + '\n')
    tmp.replace(path)


def check_pins():
    for path, digest in pins.items():
        assert runner.sha(path) == digest, path


def command(expression, no_jvm):
    argv = rows[0]['command'][:-1] + [expression]
    if no_jvm:
        argv.insert(1, '-nojvm')
    return argv


def execute(expression, label, no_jvm):
    compact.wait_for_environment(runner.REQUIRED)
    check_pins()
    argv = command(expression, no_jvm)
    env = compact.ENV | rows[0]['runtime_environment']
    out_path = HERE / (label + '.stdout.txt')
    err_path = HERE / (label + '.stderr.txt')
    monitor = compact.CompilerMonitor(runner.REQUIRED)
    monitor.start()
    try:
        with out_path.open('wb') as out, err_path.open('wb') as err:
            start = time.perf_counter_ns()
            compact.run_process(argv, subprocess.DEVNULL, out, err, 300, env)
            seconds = (time.perf_counter_ns() - start) / 1e9
    finally:
        monitor.finish()
    assert not monitor.observations and not monitor.power_changes, 'Discard contaminated diagnostic'
    check_pins()
    return dict(mode='no_jvm' if no_jvm else 'original', command=argv,
        process_seconds=seconds, stdout=str(out_path), stderr=str(err_path),
        competing_compilers=monitor.observations, power_changes=monitor.power_changes)


def validate_session(result):
    text = Path(result['stdout']).read_text()
    thread_info = re.search(r'AUDIT THREADS (\d+) JVM (\d+)', text)
    assert thread_info, text
    result.update(computation_threads=int(thread_info[1]), jvm_enabled=bool(int(thread_info[2])), calls=[])
    matches = list(re.finditer(r'AUDIT BEGIN (\w+) (\d+)\n(.*?)AUDIT END \1 \2 ([0-9.eE+-]+)', text, re.S))
    assert len(matches) == 6, text
    for match in matches:
        bench, run, output, seconds = match.groups()
        reference = report['chart_full_validation'][bench]
        digest = hashlib.sha256(output.encode()).hexdigest()
        assert len(output.encode()) == reference['bytes'] and digest == reference['sha256'], (bench, run, output)
        result['calls'].append(dict(benchmark=bench, run=int(run), seconds=float(seconds),
            bytes=reference['bytes'], sha256=digest, reference_match=True))
    result['first_call_seconds'] = {r['benchmark']: next(c['seconds'] for c in result['calls']
        if c['benchmark'] == r['benchmark'] and c['run'] == 1) for r in rows}
    result['warm_median_seconds'] = {r['benchmark']: statistics.median(c['seconds'] for c in result['calls']
        if c['benchmark'] == r['benchmark'] and c['run'] > 1) for r in rows}


def main():
    save()
    for run in range(1, 4):
        for no_jvm in [False, True]:
            mode = 'no_jvm' if no_jvm else 'original'
            if any(r['mode'] == mode and r['run'] == run for r in report['no_op']):
                continue
            print('RUN startup', mode, run, flush=True)
            result = execute("fprintf('AUDIT NOOP\\n')", f'noop-{mode}-{run}', no_jvm)
            assert Path(result['stdout']).read_text().strip() == 'AUDIT NOOP'
            result['run'] = run
            report['no_op'].append(result)
            save()
            print('PASS startup', mode, run, f"{result['process_seconds']:.6f}s", flush=True)
    report['no_op_median_seconds'] = {mode: statistics.median(r['process_seconds']
        for r in report['no_op'] if r['mode'] == mode) for mode in ['original', 'no_jvm']}
    probe = HERE / 'session_probe.m'
    source = ["addpath('" + str(D / 'sources').replace("'", "''") + "');\n", "fprintf('AUDIT THREADS %d JVM %d\\n',maxNumCompThreads,usejava('jvm'));\n"]
    for row in rows:
        bench = row['benchmark']
        n = runner.WORK[bench][1]
        source.append(f"for audit_run = 1:3\n"
            f"fprintf('AUDIT BEGIN {bench} %d\\n',audit_run);\n"
            f"audit_start = tic;\n{row['id']}({n});\n"
            f"audit_elapsed = toc(audit_start);\n"
            f"fprintf('AUDIT END {bench} %d %.17g\\n',audit_run,audit_elapsed);\nend\n")
    probe.write_text(''.join(source))
    report['harness_sha256'] = runner.sha(Path(__file__))
    report['probe_sha256'] = runner.sha(probe)
    save()
    for no_jvm in [False, True]:
        mode = 'no_jvm' if no_jvm else 'original'
        if any(r['mode'] == mode for r in report['sessions']):
            continue
        print('RUN full session', mode, flush=True)
        expression = "run('" + str(probe).replace("'", "''") + "')"
        result = execute(expression, 'session-' + mode, no_jvm)
        validate_session(result)
        report['sessions'].append(result)
        save()
        print('PASS full session', mode, json.dumps(result['warm_median_seconds']), flush=True)
    report.update(status='complete', finished_at=datetime.now(timezone.utc).isoformat())
    save()
    print('REPORT', HERE / 'report.json', flush=True)


if __name__ == '__main__':
    main()
