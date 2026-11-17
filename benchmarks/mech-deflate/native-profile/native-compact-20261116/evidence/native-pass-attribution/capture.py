"""Measure annotated native stages in fresh, isolated compiler processes."""
from collections import defaultdict
import json
import os
from pathlib import Path
from statistics import median
import subprocess
from time import perf_counter

from build import BASE, HERE, REFERENCE, SOURCE, digest, read, write

ELAPSED = 'trace'
CONTROL = 'reference'
ORDERS = [(CONTROL, ELAPSED), (ELAPSED, CONTROL),
          (ELAPSED, CONTROL), (CONTROL, ELAPSED)]
FILES = {'program.mecb': 'bytecode', 'program.o': 'encoder', 'metadata.json': 'metadata'}


def summarize(events):
    children = defaultdict(list)
    for i,event in enumerate(events):
        parent = event['parent']
        assert 0 <= event['start_ns'] <= event['end_ns'], i
        if parent is not None:
            assert 0 <= parent < i
            children[parent].append(event)
    assert [e['pass'] for e in events if e['parent'] is None] == ['native']
    assert sum(e['pass'] == 'root.cleanup' for e in events) == 1
    scopes = defaultdict(lambda:defaultdict(lambda:[0,0,0]))
    memberships = []
    roots = {
        'native': 'native', 'root.cleanup': 'cleanup',
        'prepare.PreparedNativeProgram::prepare': 'preparation',
    }
    for i,event in enumerate(events):
        span = event['end_ns'] - event['start_ns']
        end = event['start_ns']
        child_ns = 0
        for child in children[i]:
            assert end <= child['start_ns']
            end = child['end_ns']
            child_ns += child['end_ns'] - child['start_ns']
        assert end <= event['end_ns']
        group = roots.get(event['pass'])
        parent = event['parent']
        inherited = memberships[parent] if parent is not None else []
        groups = inherited + ([group] if group else [])
        memberships.append(groups)
        for category in groups:
            row = scopes[category][event['pass']]
            row[0] += span
            row[1] += span - child_ns
            row[2] += 1
    return scopes


def main():
    baseline = read(REFERENCE)['actors']['mech-early-error-pruning']
    traced = read(HERE / 'compiler/build.json')
    variants = {CONTROL:Path(baseline['compile'][0]),ELAPSED:HERE / 'compiler/compile_profile'}
    assert digest(variants[CONTROL]) == baseline['compiler_sha256']
    assert digest(variants[ELAPSED]) == traced['compiler_sha256']
    source = Path(baseline['compile'][1])
    pins = baseline['artifact_sha256']
    flags = baseline['flags']
    assert pins == traced['artifacts']
    assert all('LLVM' not in key for key in flags)
    capture = HERE / 'observed'
    capture.mkdir()
    plan = {
        'source_pin': BASE, 'source_sha256': digest(source), 'orders': ORDERS,
        'compiler_sha256': {role:digest(path) for role,path in variants.items()},
        'compiler_flags': flags, 'matched_plan_sha256': digest(REFERENCE), 'artifacts': pins,
        'scope': 'source through object in fresh processes, separate from matched build timing',
        'units': 'coordinator wall nanoseconds, exclusive of registered direct child spans',
    }
    write(capture / 'plan.json', plan)

    def run(label, role):
        root = capture / label / role
        root.mkdir(parents=True)
        env = {key:value for key,value in os.environ.items() if not key.startswith('MECH_')}
        env |= flags
        trace = root / 'native-pass-trace.json'
        if role == ELAPSED:
            env['MECH_NATIVE_PASS_ATTRIBUTION'] = str(trace)
        argv = [variants[role],source,root / 'compile','compact']
        with (root / 'stderr.log').open('wb') as log:
            start = perf_counter()
            subprocess.run(argv, env=env, cwd=SOURCE, stdout=log, stderr=subprocess.STDOUT, check=True)
            seconds = perf_counter()-start
        evidence = {
            'wall_s':seconds, 'command':list(map(str, argv)),
            'phases':read(root / 'compile/compile.json')['phases'],
            'artifacts':{name:digest(root / 'compile' / path) for path,name in FILES.items()},
        }
        assert evidence['artifacts'] == pins
        if role == ELAPSED:
            events = read(trace)
            evidence['spans'] = summarize(events)
            evidence['trace_sha256'] = digest(trace)
            meta = read(root / 'compile/metadata.json')
            evidence['root_ir'] = meta['cleanup']
        write(root / 'run.json', evidence)
        return evidence

    warmup = {role:run('warmup',role) for role in variants}
    rows = [{role:run(f'order-{i}',role) for role in order}
             for i,order in enumerate(ORDERS,1)]
    details = [row[ELAPSED]['spans'] for row in rows]
    assert all(set(row) == set(details[0]) for row in details)
    measurements = {}
    for category, data in details[0].items():
        assert all(set(row[category]) == set(data) for row in details)
        ranked = []
        for name in data:
            inclusive, exclusive, count = zip(*(row[category][name] for row in details),strict=True)
            assert min(count) == max(count)
            ranked.append({'label':name, 'inclusive_ms':median(inclusive)/1e6,
                           'exclusive_ms':median(exclusive)/1e6, 'calls':count[0]})
        measurements[category] = sorted(ranked,key=lambda result:result['exclusive_ms'], reverse=True)
    native = lambda trial: next(p['seconds'] for p in trial['phases'] if p['phase']=='native')
    result = {
        'plan_sha256':digest(capture / 'plan.json'), 'warmup':warmup, 'trials':rows, 'pass_medians':measurements,
        'median_native_s':{role:median(native(r[role]) for r in rows) for role in variants},
        'median_paired_diagnostic_native_x':median(native(r[ELAPSED])/native(r[CONTROL]) for r in rows),
    }
    write(capture / 'summary.json', result)
    print(result['median_native_s'])
    for category in ('cleanup','preparation'):
        print(category, [(e['label'],round(e['exclusive_ms'],3),e['calls'])
                         for e in measurements[category][:14]], flush=True)


if __name__ == '__main__':
    main()
