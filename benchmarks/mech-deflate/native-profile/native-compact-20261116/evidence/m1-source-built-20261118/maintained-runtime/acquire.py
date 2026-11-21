"""Compare exact M1 workers directly with the maintained frame-copies worker."""

import argparse
from contextlib import ExitStack
from hashlib import file_digest
from itertools import permutations
import gzip
import json
from pathlib import Path
import platform
from statistics import fmean
import subprocess
import sys
import time


def digest(path):
    with path.open('rb') as source:
        return file_digest(source, 'sha256').hexdigest()


def load(path):
    return json.loads(path.read_text())


def save(path, result):
    path.write_text(json.dumps(result, indent=2) + '\n')


class Worker:
    def __init__(self, command, errors):
        self.process = subprocess.Popen(
            command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, text=True,
        )

    def send(self, value):
        self.process.stdin.write(value)
        self.process.stdin.flush()

    def line(self):
        line = self.process.stdout.readline()
        assert line, self.process.poll()
        return line.rstrip('\n')

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
        self.process.wait(timeout=30)
        self.process.stdin.close()
        self.process.stdout.close()


def one_order(order, output, actors, fixtures, cases, compare, policy, resolution):
    work = output / order
    work.mkdir()
    ordering = tuple(actors)
    indices = list(enumerate(cases))
    if order == 'reverse':
        ordering = ordering[::-1]
        indices.reverse()
    schedule = list(permutations(ordering)) * 16
    cleanup = 'CLEANUP private_payload_bytes=0 live_owner_records=0\n'
    results = {}
    raw = {}
    start = time.monotonic()
    with ExitStack() as stack:
        workers = {}
        for role in ordering:
            errors = stack.enter_context((work/(role+'.stderr')).open('w'))
            binding = actors[role]
            worker = Worker([binding['worker'], binding['metadata'], fixtures], errors)
            stack.callback(worker.close)
            assert worker.line() == 'SESSION_READY', role
            workers[role] = worker
        for index, case in indices:
            name = case['name']
            size = case['input_bytes']
            batch = 512 if name in ('empty', 'one-stored', 'seven-stored-prefix') else 64 if size <= 1024 else 1
            count = batch * len(schedule)
            for worker in workers.values():
                worker.send(f'C\t{index}\t{count}\n')
                assert worker.line() == f'CASE_READY\t{name}'
            warm = ordering if index % 2 == 0 else ordering[::-1]
            for role in warm:
                worker = workers[role]
                worker.send('W')
                state, warm_case, calls, seconds = worker.line().split('\t')
                assert (state, warm_case) == ('WARMUP_DONE', name)
                assert int(calls) > 0 and float(seconds) >= .2
                worker.send('N')
                observation = json.loads(worker.line())
                assert observation['warmup']
                compare.verify_call(observation, case, compare.CONTROL, timed=False)
            blocks = []
            offset = index % len(schedule)
            for actors_in_block in schedule[offset:] + schedule[:offset]:
                means = {}
                for role in actors_in_block:
                    worker = workers[role]
                    worker.send('N' * batch)
                    seconds = []
                    for _ in range(batch):
                        observation = json.loads(worker.line())
                        compare.verify_call(observation, case, compare.CONTROL, timed=True)
                        seconds.append(observation['seconds'])
                    means[role] = fmean(seconds)
                blocks.append((actors_in_block, means))
            for worker in workers.values():
                worker.send('N')
                observation = json.loads(worker.line())
                assert observation['profile']
                compare.verify_call(observation, case, compare.CONTROL, timed=False)
                assert worker.line() == f'CASE_DONE\t{name}'
            raw[name] = {'block_size':batch, 'blocks':blocks}
            results[name] = {}
            for role_index, role in enumerate(actors):
                if role == 'frame-copies':
                    continue
                paired = compare.paired(blocks, role, 'frame-copies')
                seed = 20261118 + index * len(actors) + role_index
                delta = compare.interval(paired, seed, policy)
                if delta['lower'] > resolution:
                    status = 'established_slowdown'
                elif delta['upper'] <= resolution:
                    status = 'within_resolution_or_faster'
                else:
                    status = 'unresolved'
                results[name][role] = delta | {'status': status}
            print(order, name, {role:round(row['estimate'] * 100, 3)
                                for role, row in results[name].items()}, flush=True)
        for worker in workers.values():
            worker.send('Q\n')
            assert worker.process.wait(timeout=30) == 0
    assert all((work/(role+'.stderr')).read_text().endswith(cleanup) for role in actors)
    assert len(results) == len(cases)
    (work/'balanced-blocks.json.gz').write_bytes(
        gzip.compress(json.dumps(raw, separators=(',', ':')).encode(), mtime=0),
    )
    summary = {}
    for role in actors:
        if role != 'frame-copies':
            summary[role] = {
                status: [case for case, rows in results.items() if rows[role]['status'] == status]
                for status in ('established_slowdown', 'unresolved', 'within_resolution_or_faster')
            }
    record = {'wall_seconds':time.monotonic() - start, 'cases':len(results),
              'blocks_per_case':len(schedule), 'results':results, 'classification':summary,
              'all_checks_passed':True}
    save(work/'result.json', record)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository_packet', type=Path)
    parser.add_argument('plan', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    sys.path.insert(0, str(args.repository_packet))
    import qualification_timings as compare

    plan = load(args.plan)
    fixtures = Path(plan['fixtures']['path'])
    assert digest(fixtures) == plan['fixtures']['sha256']
    cases = load(fixtures)
    assert len(cases) == 81
    content = Path(plan['fixture_content']['path'])
    assert digest(content) == plan['fixture_content']['sha256']
    fixture_content = load(content)
    for case in cases:
        for field, pin in fixture_content[case['name']].items():
            assert digest(Path(case[field])) == pin, (case['name'], field)
    actors = plan['actors']
    policy_path = args.repository_packet/'qualification-policy-v3.json'
    assert digest(policy_path) == plan['policy_sha256']
    assert digest(Path(__file__)) == plan['driver_sha256']
    for record in actors.values():
        for field in ('worker', 'metadata', 'encoder'):
            assert digest(Path(record[field])) == record[field+'_sha256']
    assert platform.system() == 'Darwin' and platform.machine() == 'arm64'
    args.output.mkdir()
    save(args.output/'plan.json', plan)
    results = {
        order:one_order(order, args.output, actors, fixtures, cases, compare, plan['interval'],
                        plan['relative_resolution'])
        for order in plan['orders']
    }
    save(args.output/'results.json', results)


if __name__ == '__main__':
    main()
