from itertools import permutations
import errno
import gzip
import json
import math
import os
import select
import statistics
import subprocess
import sys
import time

from pathlib import Path
from hashlib import file_digest

ROOT = Path(__file__).resolve().parent
BASELINE, CANDIDATE = 'mech-control', 'mech-experiment'
FIXTURES = ROOT / 'fixtures.json'
PUBLISHED = Path('/private/tmp/mech-overwrite-reuse-20261114')
LIBDEFLATE = Path('/private/tmp/mech-wide-tier-round270-20261110/libdeflate-comparison')
prefix, before, after = sys.argv[1:4]
reverse = sys.argv[4:] == ['reverse']
def digest(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()
pins = json.loads((ROOT / 'pinned-peers' / 'pins.json').read_text())
ACTORS = [BASELINE, CANDIDATE, 'roc', 'libdeflate']
if reverse:
    ACTORS.reverse()
for actor, folder in [(BASELINE,Path(before)), (CANDIDATE,Path(after))]:
    command = [str(folder / 'mech-worker'), str(folder / 'metadata.json'), str(FIXTURES)]
    pins['actors'][actor] = dict(command=command, binary_sha256=digest(command[0]),
                                 metadata_sha256=digest(command[1]))
for actor in ('roc', 'libdeflate'):
    spec = pins['actors'][actor]
    assert digest(spec['command'][0]) == spec['binary_sha256'], actor
assert digest(FIXTURES) == pins['input_inventory_sha256']
root, lib, fixtures = ROOT, LIBDEFLATE, FIXTURES
cases = json.loads(fixtures.read_text())
assert len(cases) == 81
commands = {actor: pins['actors'][actor]['command'] for actor in ACTORS}


class Worker:
    def __init__(self, actor, folder):
        self.actor = actor
        self.buffer = b''
        self.stderr = (folder/(actor+'.stderr')).open('wb')
        self.child = subprocess.Popen(commands[actor], stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=self.stderr)
        assert self.line() == 'SESSION_READY'

    def line(self):
        while b'\n' not in self.buffer:
            assert select.select([self.child.stdout], [], [], 60)[0], self.actor
            chunk = os.read(self.child.stdout.fileno(),65536)
            assert chunk,(self.actor,self.child.poll())
            self.buffer += chunk
        line,self.buffer = self.buffer.split(b'\n',1)
        return line.decode()

    def send(self, payload):
        self.child.stdin.write(payload.encode())
        self.child.stdin.flush()

    def close(self):
        if self.child.poll() is None:
            self.child.terminate()
            self.child.wait()
        self.stderr.close()

def valid(value, fixture, actor):
    assert value['case']==fixture['name']
    for key in ('input_bytes','prefix_bytes'):
        assert value[key]==fixture[key], (actor,key,value)
    for key in ('independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged'):
        assert value[key], (actor,key,value)
    if actor=='libdeflate':
        assert value['level']==12 and value['version']=='1.26'
        assert value['reference_capacity']==fixture['capacity']
        assert value['raw_bytes']>0 and value['capacity']>=fixture['capacity']
    else:
        assert value['exact_reference'] and value['raw_bytes']==fixture['raw_bytes']
        assert value['capacity']==fixture['capacity']
    return value

def persist(action):
    until=time.monotonic()+45
    while True:
        try:
            return action()
        except OSError as error:
            if error.errno!=errno.ENOSPC or time.monotonic()>=until:
                raise
            time.sleep(.25)

out = root / prefix
persist(out.mkdir)
smoke={value['case']: value for value in json.loads((ROOT/'pinned-peers'/'libdeflate-output-oracle.json').read_text())}
assert len(smoke)==81
persist(lambda:(out/'pins.json').write_text(json.dumps(pins, indent=2)+'\n'))
workers = {}
rows = []
orders = list(permutations(commands))
selected = set(('prefix-periodic-1024', 'prefix-periodic-4096', 'prefix-periodic-8192', 'prefix-periodic-16384', 'prefix-periodic-32767', 'prefix-natural-4096', 'prefix-natural-8192', 'prefix-natural-16384', 'random-1024', 'prefix-random-4096', 'heldout-random-65537'))
try:
    for actor in commands:
        workers[actor] = Worker(actor,out)
    for index,case in reversed(list(enumerate(cases))) if reverse else enumerate(cases):
        name = case['name']
        if name not in selected:
            continue
        block_size = 512 if name in ('empty','one-stored','seven-stored-prefix') else (
                     64 if case['input_bytes']<=1024 else 1)
        for actor,worker in workers.items():
            worker.send(f'C\t{index}\t{block_size*len(orders)}\n')
            assert worker.line()=='CASE_READY\t'+name
        warmup_order=list(commands)
        if index%2:
            warmup_order.reverse()
        for actor in warmup_order:
            worker=workers[actor]
            worker.send('W')
            state,warm_case,calls,seconds=worker.line().split('\t')
            assert (state,warm_case)==('WARMUP_DONE',name) and int(calls)>0 and float(seconds)>=.2
            worker.send('N')
            assert valid(json.loads(worker.line()),case,actor)['warmup']
        offset=index%len(orders)
        case_orders=orders[offset:]+orders[:offset]
        blocks=[]
        for order in case_orders:
            observations={}
            for actor in order:
                worker=workers[actor]
                worker.send('N'*block_size)
                values=[valid(json.loads(worker.line()),case,actor) for _ in range(block_size)]
                assert all(not value['warmup'] and not value['profile'] for value in values)
                if actor=='libdeflate':
                    assert all(all(value[k]==smoke[name][k] for k in
                                   ('output_sha256','capacity','raw_bytes','matches_reference_bytes'))
                               for value in values)
                observations[actor]=values
            means={actor:statistics.mean(v['seconds'] for v in values) for actor,values in observations.items()}
            assert all(value>0 for value in means.values()), (name,means)
            blocks.append({'order':order,'observations':observations,'means':means})
        for actor,worker in workers.items():
            worker.send('N')
            assert valid(json.loads(worker.line()),case,actor)['profile']
            assert worker.line()=='CASE_DONE\t'+name
        medians={actor:statistics.median(
            value['seconds'] for block in blocks for value in block['observations'][actor])
            for actor in commands}
        paired={
            actor:math.exp(statistics.mean(math.log(block['means'][actor]/block['means']['roc']) for block in blocks))
            for actor in commands
        }
        relative_libdeflate={
            actor:math.exp(statistics.mean(math.log(block['means'][actor]/block['means']['libdeflate']) for block in blocks))
            for actor in commands
        }
        row=dict(case=name, input_bytes=case['input_bytes'], reference_raw_bytes=case['raw_bytes'],
                 libdeflate_raw_bytes=smoke[name]['raw_bytes'], reference_capacity=case['capacity'],
                 libdeflate_capacity=smoke[name]['capacity'],
                 libdeflate_same_bytes=smoke[name]['matches_reference_bytes'],
                 raw_stream_sha256=smoke[name]['output_sha256'],
                 median_seconds=medians, paired_x_roc=paired, paired_x_libdeflate=relative_libdeflate,
                 timed_calls=block_size*len(orders)*len(commands))
        rows.append(row)
        folder=out/name
        persist(lambda:folder.mkdir(exist_ok=True))
        packet=json.dumps(blocks,separators=(',',':')).encode()
        compressed=gzip.compress(packet,mtime=0)
        persist(lambda:(folder/'observations.json.gz').write_bytes(compressed))
        persist(lambda:(out/'summary.json').write_text(json.dumps(rows,indent=2)+'\n'))
        print(name, {actor:round(ratio,6) for actor,ratio in paired.items()},
              f"raw bytes {row['reference_raw_bytes']} / {row['libdeflate_raw_bytes']}",flush=True)
    for worker in workers.values():
        worker.send('Q\n')
        assert worker.child.wait()==0
finally:
    for worker in workers.values():
        worker.close()
for actor in (BASELINE, CANDIDATE):
    assert (out/(actor+'.stderr')).read_text().endswith('CLEANUP private_payload_bytes=0 live_owner_records=0\n')
assert (out/'roc.stderr').read_text().endswith('CLEANUP reference_input_and_output_roots_released=1\n')
assert (out/'libdeflate.stderr').read_text().endswith('CLEANUP libdeflate_compressor_and_output_released=1\n')
assert len(rows)==len(selected)
persist(lambda:(out/'complete.json').write_text(json.dumps(dict(
    cases=len(rows), balanced_blocks_per_case=len(orders), timed_calls=sum(v['timed_calls'] for v in rows),
    reference_capacity_matches=sum(v['reference_capacity']==v['libdeflate_capacity'] for v in rows),
    libdeflate_exact_bytes_cases=sum(v['libdeflate_same_bytes'] for v in rows),
    fresh_compression_all_cases=True, all_outputs_independently_inflated=True,
    all_input_and_output_aliases_preserved=True),indent=2)+'\n'))
