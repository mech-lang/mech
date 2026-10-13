#!/usr/bin/env python3
"""One locked bounded qualification; all pilot durations are planning only."""
from datetime import datetime, timezone
import fcntl
from copy import deepcopy
import importlib.util
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import time

from prepare import BASE, BASE_SHA, ENV, HERE, LOCK, SUITE, RESULT, WORKER, RANGES, prepare, sha
from task_threads import TaskThreads

RESULTS = HERE / 'results'
FAMILIES = ('compact', 'fast')
CAPS = tuple(range(1, 9))
GATE_INIT = '◌ ⍏ ◿16001 ×7919 ⇡16000'
LOCK_FD = None
JOB_STARTED = None
V4 = SUITE / 'uiua-fann-parallel-v4'
V4_SHA = '231e3f950ed63a7e7f3e293d8f5e0aeebf68986247aeff9ad21426d34502828e'


def save(path, value):
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')
    temporary.replace(path)


def fixture(path, text):
    path.write_text('# Experimental!\n' + text)
    return path


def cap_source(text, cap):
    assert text.count('↧8Total') == 3
    return text.replace('↧8Total', f'↧{cap}Total')


def expanded_source(text):
    # Expand only the production Result composition for diagnostic recording.
    # Runtime probes above always use the complete production expression.
    line = next(line for line in text.splitlines() if line.startswith('Result '))
    assert line.startswith('Result       ← ≡°□ wait ≡spawn(')
    worker, ranges = line.split('≡spawn(', 1)[1].split(') ⍉⊟ ', 1)
    assert worker == WORKER
    cap = ranges.split('↧', 1)[1].split('Total', 1)[0]
    assert cap in tuple(str(i) for i in range(1, 9))
    assert '⍉⊟ ' + ranges == RANGES.replace('↧8Total', f'↧{cap}Total')
    footer = '\n'.join([
        'Run ← ⍢(Work|<⊃F.End F.Index)', f'T ← ↧{cap} Total',
        'Bounds ← ⌊÷T ×Total⇡+1T', 'Ranges ← ⍉⊟ ⊃(↙T|↘1) Bounds',
        'Worker ← ' + WORKER.replace('⍢(Work|<⊃F.End F.Index)', 'Run'),
        'Handles ← ≡spawnWorker Ranges', 'Joined ← ≡°□ wait Handles',
        '&p /+ ≡F.Checksum Joined', '&p $"Pfannkuchen(_) = _" N /↥ ≡F.Maximum Joined'])
    prefix = text[:text.index(line)]
    return prefix + footer + '\n'


def execute(runtime, source, n, tasks, stem, gate=False):
    assert time.monotonic() - JOB_STARTED < 900, 'Bounded qualification exceeded 15 minutes'
    command = [str(runtime), 'run', '--no-format', str(source), str(n)]
    output_path, error_path = RESULTS / (stem + '.stdout'), RESULTS / (stem + '.stderr')
    observations, blocked = [], []
    started = time.monotonic()
    with output_path.open('w+b') as output, error_path.open('w+b') as error:
        process = subprocess.Popen(command, cwd=SUITE, env=dict(os.environ, **ENV),
                                   stdin=subprocess.PIPE if gate else subprocess.DEVNULL,
                                   stdout=output, stderr=error, start_new_session=True,
                                   pass_fds=(LOCK_FD,))
        released = not gate
        try:
            while process.poll() is None:
                assert time.monotonic() - started < 120, 'Bounded probe timed out'
                assert time.monotonic() - JOB_STARTED < 900, 'Bounded qualification exceeded 15 minutes'
                observed = tasks.read(process.pid)
                if observed:
                    observations.append(observed)
                    assert observed['resident_bytes'] <= 2 * 1024**3, 'Bounded probe exceeded 2 GiB'
                if gate and not released:
                    output.seek(0)
                    if output.read(6) == b'READY\n':
                        for _ in range(3):
                            value = tasks.read(process.pid)
                            assert value is not None
                            blocked.append(value)
                            time.sleep(.005)
                        process.stdin.write(b'release\n')
                        process.stdin.close()
                        released = True
                time.sleep(.005)
            assert released, 'Worker gate never reached READY'
        except BaseException:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise
        status = process.wait()
        output.seek(0)
        error.seek(0)
        stdout, stderr = output.read(), error.read()
    row = dict(source=str(source), source_sha256=sha(source), n=n, command=command,
               cwd=str(SUITE), runtime_environment=ENV, returncode=status,
               stdout_file=str(output_path), stdout_sha256=sha(output_path), bytes=len(stdout),
               stderr_file=str(error_path), stderr_sha256=sha(error_path),
               native_thread_observations=observations, blocked_observations=blocked,
               peak_total_os_threads=max((v['total_os_threads'] for v in observations), default=0),
               peak_rss_bytes=max((v['resident_bytes'] for v in observations), default=0),
               seconds=time.monotonic() - started, shared_mutex_inherited_by_native=True)
    assert status == 0, (source, n, stderr.decode())
    return stdout, row


def traced_source(text):
    text = expanded_source(text)
    helpers = '''~FlipRecorder {State Frames}
ToFlip   ← FlipRecorder ⊃(FlipStep FlipRecorder.State|FlipRecorder.Frames)
Capture  ← FlipRecorder ⊃(FlipRecorder.State|⊂⊃FlipRecorder.Frames(□ FlipRecorder.State))
Logged   ← Capture ToFlip
InitTrace ← FlipRecorder ⊃(∘|¤□) FlipState ⊃(∘|↯⊃(⧻|⋅0))
CountsOf ← FlipState.Counts FlipRecorder.State
~TraceBlock {Indices Recorder}
Flips ← ⍢(Logged|/↥>0≡⊢FlipState.Permutations FlipRecorder.State) InitTrace
'''
    lines = []
    for line in text.splitlines():
        if line.startswith('Flips '):
            lines.extend(helpers.rstrip().splitlines())
        elif line.startswith('~F '):
            lines.append('~F {Index End Checksum Maximum Logs}')
        elif line.startswith('Work '):
            line = line.replace('Block.Counts', '(CountsOf Block.Counts)')
            assert line.endswith(') Candidate')
            lines.append(line[:-len(') Candidate')] + '|⊂⊃(F.Logs Block.State)(□ TraceBlock ⊃Block.Indices Block.Counts)) Candidate')
        elif line.startswith('Worker '):
            lines.append(line.replace('|⋅0)', '|⋅0|⋅[])'))
        elif line.startswith('&p '):
            continue
        else:
            lines.append(line)
    lines.append('&p json Joined')
    return '\n'.join(lines) + '\n'


def check_trace(output, n, cap, expected):
    permutations, counts, expected_output = expected
    total, workers = math.factorial(n), min(cap, math.factorial(n))
    returned = json.loads(output)
    assert len(returned) == workers
    visited, flips, checksum, maximum, block_sizes = [], 0, 0, 0, []
    for j, worker in enumerate(returned):
        index, end, observed_checksum, observed_maximum, logs = worker
        start, expected_end = j * total // workers, (j + 1) * total // workers
        assert index == end == expected_end
        worker_ranks, worker_counts = [], []
        for ranks, (final_state, frames) in logs:
            assert ranks == list(range(start + len(worker_ranks), start + len(worker_ranks) + len(ranks)))
            assert 0 < len(ranks) <= 1024 and ranks[-1] < end
            previous, current_counts = frames[0]
            assert previous == [permutations[rank] for rank in ranks]
            assert current_counts == [0] * len(ranks)
            for current, observed_counts in frames[1:]:
                next_permutations = []
                for lane, permutation in enumerate(previous):
                    following = permutation.copy()
                    if permutation[0]:
                        following[:permutation[0] + 1] = reversed(permutation[:permutation[0] + 1])
                        current_counts[lane] += 1
                        flips += 1
                    next_permutations.append(following)
                assert current == next_permutations and observed_counts == current_counts
                previous = current
            assert all(permutation[0] == 0 for permutation in previous)
            assert final_state == [previous, current_counts]
            assert current_counts == [counts[rank] for rank in ranks]
            worker_ranks.extend(ranks)
            worker_counts.extend(current_counts)
            block_sizes.append(len(ranks))
        assert worker_ranks == list(range(start, end))
        assert observed_checksum == sum(count * (-1 if rank % 2 else 1) for rank, count in zip(worker_ranks, worker_counts))
        assert observed_maximum == max(worker_counts)
        checksum += observed_checksum
        maximum = max(maximum, observed_maximum)
        visited.extend(worker_ranks)
    assert visited == list(range(total)) and flips == sum(counts)
    assert f'{checksum}\nPfannkuchen({n}) = {maximum}\n'.encode() == expected_output
    local_parity_checksum = sum(counts[rank] * (-1 if (rank - j * total // workers) % 2 else 1)
                                for j in range(workers) for rank in range(j * total // workers, (j + 1) * total // workers))
    return dict(reference_match=True, every_global_rank_once=True, permutations=total,
                every_initial_permutation_verified=True, every_actual_prefix_reversal_verified=True,
                actual_prefix_reversals=flips, all_finished_lanes_unchanged=True,
                exact_global_rank_parity=True, worker_returned_trace_only=True,
                concurrent_worker_stdout=False, block_sizes=block_sizes,
                worker_start_ranks=[j * total // workers for j in range(workers)],
                detects_worker_local_parity_error=local_parity_checksum != checksum)


def gate_source(text):
    text = expanded_source(text)
    lines = [GATE_INIT]
    for line in text.splitlines():
        if line.startswith('Worker '):
            lines += ['Worker ← (', '  ◌ recv 0', '  ' + line.split('←', 1)[1].strip(), ')']
        else:
            lines.append(line)
        if line.startswith('Handles '):
            lines += ['&p "READY"', '◌ &fras "/dev/stdin"', 'send Handles 1']
    return '\n'.join(lines) + '\n'


def range_source(text, total, cap):
    text = expanded_source(text)
    shared = {line.split('←')[0].strip(): line for line in text.splitlines() if '←' in line}
    return '\n'.join([f'Total ← {total}', '~F {Index End Ranks}', shared['BlockIndices'],
                      'Work ← F ⊃(+⊃F.Index(⧻BlockIndices)|F.End|⊂⊃F.Ranks BlockIndices)',
                      shared['Run'], f'T ← ↧{cap} Total', shared['Bounds'], shared['Ranges'],
                      'Worker ← □ Run F ⊃(⊢|⊣|⋅[])', shared['Handles'], shared['Joined'], '&p json Joined']) + '\n'


def matrix_source(text, family, ranks):
    stop = '~F '
    prefix = text[:text.index(stop)]
    expression = 'Permutation'
    return prefix + '&p json ' + expression + ' ' + json.dumps(ranks).replace(',', '') + '\n'


def unrank(n, rank):
    permutation = list(range(n))
    for width in range(n, 1, -1):
        rotation = rank // math.factorial(width - 1) % width
        permutation[:width] = permutation[rotation:width] + permutation[:rotation]
    return permutation


def main():
    global RESULTS, LOCK_FD, JOB_STARTED
    RESULTS = RESULTS / ('validation-' + datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ'))
    RESULTS.mkdir(parents=True, exist_ok=True)
    print('Waiting for common mutex before preparation, hashing or probes.', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        LOCK_FD = lock.fileno()
        JOB_STARTED = time.monotonic()
        print('ACQUIRED mutex for one bounded composed compact/fast qualification job.', flush=True)
        prepare()
        assert sha(V4 / 'manifest.json') == V4_SHA
        prior = json.loads((V4 / 'manifest.json').read_text())
        for path, digest in prior['pins'].items():
            assert sha(path) == digest, path
        prior_validation_path = Path(prior['validation_evidence']['path'])
        prior_validation = json.loads(prior_validation_path.read_text())
        assert prior_validation['status'] == 'passed'
        base_manifest = json.loads((SUITE / 'uiua-fannkuch-v2/manifest.json').read_text())
        runtime = Path(base_manifest['runtime']['path'])
        frozen_pins = dict(prior['pins'])
        frozen_pins[str(V4 / 'manifest.json')] = V4_SHA
        frozen_pins.update(base_manifest['pins'])
        frozen_pins.update({str(SUITE / name): sha(SUITE / name) for name in
                            ['source-audit-uiua.json', 'uiua-manifest.json', 'run.py', 'supervise_v4.py']})
        for path, digest in frozen_pins.items():
            assert sha(path) == digest, path
        spec = importlib.util.spec_from_file_location('independent_fann_reference', SUITE / 'uiua-fannkuch-v2/validate.py')
        reference = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(reference)
        assert sha(BASE) == BASE_SHA
        texts = {family: (HERE / 'sources' / (family + '.ua')).read_text() for family in FAMILIES}
        source_pins = {str(HERE / 'sources' / (family + '.ua')): sha(HERE / 'sources' / (family + '.ua')) for family in FAMILIES}
        report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
                      scope='Bounded n1..8 qualification and n9 planning only; no full n12 run or suite migration.',
                      shared_timing_lock=str(LOCK), attempt_directory=str(RESULTS), source_pins=source_pins,
                      retained_pins=frozen_pins, runtime=str(runtime), runtime_sha256=sha(runtime),
                      runtime_environment=ENV, validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
                      cases=[], baseline_cases=[], traces=[], worker_gates=[], ranges=[], matrices=[], planning_pilots=[],
                      full_performance_runs=0, eligible_for_chart=False)
        snapshots = {}
        for path in [HERE / name for name in ['prepare.py', 'validate.py', 'prepare_manifest.py', 'task_threads.py', 'SPEC.md']] + [HERE / 'sources' / (family + '.ua') for family in FAMILIES]:
            snapshot = RESULTS / ('input-' + path.name)
            snapshot.write_bytes(path.read_bytes())
            snapshots[str(path)] = dict(path=str(snapshot), sha256=sha(snapshot))
        report['input_snapshots'] = snapshots
        tasks = TaskThreads()
        try:
            expected = {n: reference.reference(n) for n in range(1, 9)}
            for n, (permutations, _, _) in expected.items():
                assert permutations == [unrank(n, rank) for rank in range(math.factorial(n))]
            # Retained v2 output controls are already qualified and unchanged.
            report['baseline_cases'] = deepcopy(prior_validation['baseline_cases'])
            assert len(report['baseline_cases']) == 8
            report['reused_evidence'] = [dict(original_validation_file=str(prior_validation_path),
                original_validation_sha256=sha(prior_validation_path), original_manifest=str(V4 / 'manifest.json'),
                original_manifest_sha256=V4_SHA, scope='Only eight unchanged v2 output controls; all new production candidates receive fresh outputs/traces/gates/matrices.')]
            parent = fixture(RESULTS / 'parent-gate.ua', GATE_INIT + '\n&p "READY"\n◌ &fras "/dev/stdin"\n')
            output, row = execute(runtime, parent, 0, tasks, 'parent-gate', gate=True)
            assert output == b'READY\n'
            baseline_threads = row['blocked_observations'][0]['total_os_threads']
            assert all(v['total_os_threads'] == baseline_threads for v in row['blocked_observations'])
            report.update(parent_thread_baseline=row, parent_total_os_threads=baseline_threads)
            for family, text in texts.items():
                for cap in CAPS:
                    source = HERE / 'sources' / (family + '.ua') if cap == 8 else fixture(RESULTS / f'{family}-cap-{cap}.ua', cap_source(text, cap))
                    for n in range(1, 9):
                        output, row = execute(runtime, source, n, tasks, f'{family}-{n}-workers-{cap}')
                        assert output == expected[n][2], (family, n, cap)
                        assert row['peak_total_os_threads'] <= baseline_threads + min(cap, math.factorial(n))
                        row.update(family=family, requested_workers=cap, reference_match=True,
                                   worker_limit=min(cap, math.factorial(n)), parent_only_output=True)
                        report['cases'].append(row)
                    print(family, cap, 'COMPLETE n1..8 OUTPUT PASS', flush=True)
                for n, cap in [(2, 8), (5, 7), (6, 7), (7, 3), (8, 8)]:
                    source = fixture(RESULTS / f'{family}-trace-{n}-{cap}.ua', traced_source(cap_source(text, cap)))
                    output, row = execute(runtime, source, n, tasks, f'{family}-trace-{n}-{cap}')
                    row.update(check_trace(output, n, cap, expected[n]), family=family, requested_workers=cap,
                               production_source=str(HERE / 'sources' / (family + '.ua')), diagnostic_only=True)
                    report['traces'].append(row)
                    print(family, n, cap, 'RETURNED EVERY-RANK/FLIP TRACE PASS', flush=True)
                assert any(row['detects_worker_local_parity_error'] for row in report['traces'] if row['family'] == family)
                for n, cap in [(6, cap) for cap in CAPS] + [(1, 8), (2, 8), (3, 8)]:
                    source = fixture(RESULTS / f'{family}-gate-{n}-{cap}.ua', gate_source(cap_source(text, cap)))
                    output, row = execute(runtime, source, n, tasks, f'{family}-gate-{n}-{cap}', gate=True)
                    workers = min(cap, math.factorial(n))
                    assert output == b'READY\n' + expected[n][2]
                    assert all(v['total_os_threads'] - baseline_threads == workers for v in row['blocked_observations'])
                    row.update(family=family, requested_workers=cap, observed_additional_os_worker_threads=workers,
                               actual_worker_bound_verified=True, reference_match=True, diagnostic_only=True)
                    report['worker_gates'].append(row)
                print(family, 'ACTUAL 1..8 WORKER GATES PASS', flush=True)
            for total in [1, 2, 7, 8, 9, 1023, 1024, 1025, 1031, 8191, 8192, 8193]:
                for cap in [3, 7, 8]:
                    source = fixture(RESULTS / f'ranges-{total}-{cap}.ua', range_source(texts['fast'], total, cap))
                    output, row = execute(runtime, source, 12, tasks, f'ranges-{total}-{cap}')
                    observed, workers, visited = json.loads(output), min(cap, total), []
                    assert len(observed) == workers
                    for j, (index, end, ranks) in enumerate(observed):
                        assert index == end == (j + 1) * total // workers
                        assert ranks == list(range(j * total // workers, end))
                        visited.extend(ranks)
                    assert visited == list(range(total))
                    row.update(total=total, requested_workers=cap, exact_coverage=True, diagnostic_only=True,
                               production_range_expressions_unchanged=True, no_permutation_or_flip_computation=True)
                    report['ranges'].append(row)
            for n, start, length in [(1, 0, 1), (7, 0, 1023), (7, 1, 1024), (7, 1023, 1025),
                                     (12, 0, 1024), (12, 1, 1024), (12, 1023, 1024),
                                     (12, 1024, 1024), (12, math.factorial(12) - 1025, 1024)]:
                ranks = list(range(start, start + length))
                values = []
                for family in FAMILIES:
                    source = fixture(RESULTS / f'{family}-matrix-{n}-{start}-{length}.ua', matrix_source(texts[family], family, ranks))
                    output, row = execute(runtime, source, n, tasks, f'{family}-matrix-{n}-{start}-{length}')
                    values.append(json.loads(output))
                    row.update(family=family, first_global_rank=start, ranks=length, diagnostic_only=True,
                               no_full_n12_enumeration=True)
                    report['matrices'].append(row)
                retained = next(row for row in prior_validation['matrices'] if row['family'] == 'parallel' and row['n'] == n and row['first_global_rank'] == start and row['ranks'] == length)
                assert sha(retained['stdout_file']) == retained['stdout_sha256']
                assert values[0] == values[1] == json.loads(Path(retained['stdout_file']).read_text()) == [unrank(n, rank) for rank in ranks]
                for row in report['matrices'][-2:]:
                    row.update(reference_match=True, exact_v2_permutation_matrix=True, descending_width_order=True)
            expected9 = reference.reference(9)[2]
            for name, source in [('compact-8', HERE / 'sources/compact.ua'),
                                  ('fast-8', HERE / 'sources/fast.ua'),
                                  ('fast-1', fixture(RESULTS / 'fast-pilot-one-worker.ua', cap_source(texts['fast'], 1)))]:
                output, row = execute(runtime, source, 9, tasks, 'planning-' + name)
                assert output == expected9
                row.update(name=name, reference_match=True, planning_only=True, never_chart_eligible=True,
                           full_workload=False, full_n=12, workload_n=9, performance_sample=False)
                report['planning_pilots'].append(row)
                print(name, 'N9 PLANNING ONLY', row['seconds'], flush=True)
            for path, digest in {**frozen_pins, **source_pins}.items():
                assert sha(path) == digest, path
            assert report['validator_sha256'] == sha(__file__)
            report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                          outputs_passed=len(report['cases']), baseline_checks_passed=len(report['baseline_cases']),
                          traces_passed=len(report['traces']), worker_gates_passed=len(report['worker_gates']),
                          range_checks_passed=len(report['ranges']), matrix_checks_passed=len(report['matrices']))
            save(RESULTS / 'validation.json', report)
            save(HERE / 'results/validation.json', report)
            manifest_spec = importlib.util.spec_from_file_location('fann_composed_manifest', HERE / 'prepare_manifest.py')
            module = importlib.util.module_from_spec(manifest_spec)
            manifest_spec.loader.exec_module(module)
            module.main()
            print('ALL BOUNDED QUALIFICATION PASSED; no full run or frozen suite edits.', flush=True)
        except BaseException as error:
            report.update(status='failed', failed_at=datetime.now(timezone.utc).isoformat(), error=repr(error))
            save(RESULTS / 'validation.json', report)
            save(HERE / 'results/validation.json', report)
            raise


if __name__ == '__main__':
    main()
