"""Verify, count, then time identical captured Huffman inputs in serial."""
import argparse
from hashlib import sha256
from itertools import permutations
import json
import math
import os
from pathlib import Path
import select
import statistics
import subprocess

ACTORS = ['c-native', 'c-mech', 'mech-shell', 'mech-bucket']


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


class Worker:
    def __init__(self, command, error_path):
        self.buffer = b''
        self.error = error_path.open('w')
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.error)
        assert self.line() == 'SESSION_READY'

    def line(self):
        while b'\n' not in self.buffer:
            assert select.select([self.process.stdout], [], [], 30)[0], 'worker observation timed out'
            chunk = os.read(self.process.stdout.fileno(), 65536)
            assert chunk, self.process.poll()
            self.buffer += chunk
        line, self.buffer = self.buffer.split(b'\n', 1)
        return line.decode()

    def send(self, command):
        self.process.stdin.write(command.encode())
        self.process.stdin.flush()

    def close(self):
        if self.process.poll() is None:
            self.send('Q\n')
            assert self.process.wait(timeout=10) == 0
        self.error.close()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('native', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--mode', choices=['work','smoke','timing'], required=True)
    parser.add_argument('--repeats', type=int, default=32)
    args = parser.parse_args()
    native, out = args.native.resolve(), args.output.resolve()
    build = json.loads((native/'build.json').read_text())
    assert build['complete'] and digest(native/'vectors.json') == build['vectors_sha256']
    vectors = json.loads((native/'vectors.json').read_text())
    out.mkdir(parents=True, exist_ok=False)
    actors = ACTORS[:2] if args.mode == 'work' else ACTORS
    commands = {}
    for actor in actors:
        role = 'bucket' if actor == 'mech-bucket' else 'shell'
        version = 'counts' if args.mode == 'work' else 'timing'
        entry = build['roles'][role]
        binary = entry[version+'_worker']
        assert digest(binary) == entry[version+'_worker_sha256']
        assert digest(entry['metadata']) == entry['artifacts']['metadata.json']
        commands[actor] = [binary, entry['metadata'], str(native/'vectors.json'), actor]
    repeats = args.repeats if args.mode == 'timing' else 1
    plan = {'mode':args.mode,'build_sha256':digest(native/'build.json'),
        'vectors_sha256':digest(native/'vectors.json'),'controller_sha256':digest(__file__),
        'commands':commands,'worker_sha256':{a:digest(c[0]) for a,c in commands.items()},
        'repeats':repeats,'actors':actors,'load_before':os.getloadavg(),
        'scope':'Separate standalone phases with fresh scratch/context-to-cleanup. Exact source-extracted algorithms; not actual shipping helper code or a whole-codec speed forecast.'}
    (out/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
    records = []
    for direction in (['forward','reverse'] if args.mode == 'timing' else ['forward']):
        folder = out/direction
        folder.mkdir()
        workers = {a:Worker(c,folder/(a+'.stderr')) for a,c in commands.items()}
        try:
            case_order = list(enumerate(vectors))
            if direction == 'reverse':
                case_order.reverse()
            orders = list(permutations(actors)) if args.mode == 'timing' else [tuple(actors)]
            if direction == 'reverse':
                orders.reverse()
            for index, case in case_order:
                count = len(orders)+(1 if args.mode == 'timing' else 0)
                for actor, worker in workers.items():
                    worker.send(f'C\t{index}\t{repeats}\t{count}\n')
                    assert worker.line() == 'CASE_READY\t'+case['name']
                warmups = {}
                if args.mode == 'timing':
                    for actor in (list(actors) if index%2 == 0 else list(reversed(actors))):
                        workers[actor].send('W')
                        warmup = json.loads(workers[actor].line())
                        assert warmup['warmup_seconds'] >= .2 and warmup['warmup_calls'] > 0
                        warmups[actor] = warmup
                blocks = []
                for order in orders:
                    values = {}
                    for actor in order:
                        workers[actor].send('T' if args.mode == 'work' else 'N')
                        value = json.loads(workers[actor].line())
                        assert value['case'] == case['name'] and value['actor'] == actor
                        if args.mode != 'work':
                            assert value['exact_sorted_keys'] and value['canaries_unchanged']
                            assert value['verified_calls'] == repeats*len(case['vectors'])
                            assert value['vectors'] == len(case['vectors']) and value['batch_nanoseconds'] > 0
                        else:
                            assert len(value['work']) == len(case['vectors'])
                        values[actor] = value
                    blocks.append({'order':order,'observations':values})
                for worker in workers.values():
                    assert worker.line() == 'CASE_DONE\t'+case['name']
                row = {'case':case['name'],'direction':direction,'warmups':warmups,'blocks':blocks}
                if args.mode == 'timing':
                    row['median_ns_per_captured_sequence'] = {actor:statistics.median(
                        block['observations'][actor]['batch_nanoseconds']/repeats for block in blocks) for actor in actors}
                    row['paired_x_C_native'] = {actor:math.exp(statistics.mean(math.log(
                        block['observations'][actor]['batch_nanoseconds']/block['observations']['c-native']['batch_nanoseconds'])
                        for block in blocks)) for actor in actors}
                records.append(row)
                (out/'results.json').write_text(json.dumps({'plan':plan,'records':records,'complete':False},indent=2)+'\n')
                print(direction,case['name'],json.dumps(row.get('median_ns_per_captured_sequence',{'vectors':len(case['vectors'])})),flush=True)
        finally:
            for worker in workers.values():
                worker.close()
    result = {'plan':plan,'records':records,'complete':True,'load_after':os.getloadavg()}
    (out/'results.json').write_text(json.dumps(result,indent=2)+'\n')
    print('Complete',args.mode,len(records),'case batches',flush=True)


if __name__ == '__main__':
    main()
