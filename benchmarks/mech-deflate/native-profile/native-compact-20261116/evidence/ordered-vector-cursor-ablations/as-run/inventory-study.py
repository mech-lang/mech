"""Measure repeated Tail inventory work with exact emitted artifacts."""

from collections import defaultdict
from importlib import import_module
import os
from pathlib import Path
from statistics import median
import subprocess
import sys

from build import CONTROL_PLAN, read, save, sha
from narrow import SOURCE
from stage import BASE, HERE

OUT = SOURCE / 'inventory-study'
COMPILER = SOURCE / 'probe-compiler/compile_profile'
BUILD = SOURCE / 'probe-compiler/result.json'
REUSE = 'MECH_REUSE_NUMERIC_PHASE_INVENTORY'
TRACE = 'MECH_TRACE_NUMERIC_PHASE_INVENTORY'
COVERAGE = ('complete', 'scalar-cold', 'intervals', 'fueled', 'hybrid')
ROLE = 'mech-ascii-miss'


def actor():
    control = read(CONTROL_PLAN)['actors'][ROLE]
    built = read(BUILD)
    assert built['base'] == BASE and built['compiler_sha256'] == sha(COMPILER)
    assert built['disabled_artifacts'] == control['artifact_sha256']
    return control | {'compile': [str(COMPILER), *control['compile'][1:]], 'compiler_sha256': sha(COMPILER)}


def extract(log):
    rows = []
    for line in log.read_text().splitlines():
        if line.startswith('MECH_PHASE_INVENTORY\t'):
            _, root, mode, reuse, ns, sites = line.split('\t')
            rows.append({'header': int(root), 'mode': COVERAGE[int(mode)], 'reused': reuse == 'true',
                         'nanoseconds': int(ns), 'sites': int(sites)})
    assert rows
    return rows


def trace():
    common = actor()
    output = OUT / 'trace'
    output.mkdir(parents=True)
    clean = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    rows = {}
    logs = {}
    for role in ('control', 'reuse'):
        directory = output / role
        directory.mkdir()
        _, input, _, kind = common['compile']
        frontend = directory / 'frontend'
        logs[role] = directory / 'compile.log'
        args = [COMPILER, input, frontend, kind]
        flags = common['flags'] | {TRACE: '1'} | ({REUSE: '1'} if role == 'reuse' else {})
        with logs[role].open('wb') as log:
            subprocess.run(args, env=clean | flags, check=True, stdout=log, stderr=subprocess.STDOUT)
        paths = {'program.o': 'encoder', 'metadata.json': 'metadata', 'program.mecb': 'bytecode'}
        measured = {label: sha(frontend / name) for name, label in paths.items()}
        assert measured == common['artifact_sha256']
        rows[role] = extract(logs[role])
    assert len(rows['control']) == len(rows['reuse'])
    summary = defaultdict(lambda: {'attempts': 0, 'hits': 0, 'control_ns': 0, 'reuse_ns': 0,
                                  'shared_control_ns': 0, 'shared_reuse_ns': 0})
    hits = []
    for old, new in zip(rows['control'], rows['reuse'], strict=True):
        key = ('header', 'mode', 'sites')
        assert all(old[k] == new[k] for k in key) and not old['reused']
        bucket = summary[new['mode']]
        bucket['attempts'] += 1
        bucket['control_ns'] += old['nanoseconds']
        bucket['reuse_ns'] += new['nanoseconds']
        if new['reused']:
            assert new['mode'] == 'fueled'
            bucket['hits'] += 1
            bucket['shared_control_ns'] += old['nanoseconds']
            bucket['shared_reuse_ns'] += new['nanoseconds']
            hits.append({'control': old, 'reuse': new, 'paired_inventory_ns_saved': old['nanoseconds'] - new['nanoseconds']})
    result = {'source_pin': BASE, 'compiler_sha256': common['compiler_sha256'],
              'artifact_sha256': common['artifact_sha256'], 'summary': summary,
              'trace_logs_sha256': {role: sha(log) for role, log in logs.items()},
              'paired_trace_rows': rows, 'shared_attempts': hits}
    save(output / 'result.json', result)
    print('Inventory trace', dict(summary), flush=True)
    print('Exact repeat count', len(hits), 'unrounded measured repeated inventory ns',
          sum(r['paired_inventory_ns_saved'] for r in hits), flush=True)


def matched():
    control_plan = read(CONTROL_PLAN)
    reference = read(OUT / 'trace/result.json')
    assert reference['shared_attempts'], 'no exact repeat in the traced build'
    clean = actor()
    candidate = clean | {'flags': clean['flags'] | {REUSE: '1'}}
    actors = {'mech-control': clean, 'mech-reuse': candidate,
              'roc': control_plan['actors']['roc'], 'libdeflate': control_plan['actors']['libdeflate']}
    for role, actor_info in actors.items():
        assert sha(Path(actor_info['compile'][0])) == actor_info['compiler_sha256'], role
        assert all(sha(Path(path)) == value for path, value in actor_info['host_inputs_sha256'].items())
        assert TRACE not in actor_info['flags']
    sys.path.insert(0, str(HERE.parent))
    runner = import_module('measure-deployment-builds')
    dest = OUT / 'matched'
    dest.mkdir()
    roles = tuple(actors)
    orders = [[roles[(i + shift) % len(roles)] for i in (0, 1, 3, 2)]
              for shift in range(len(roles))] * 2
    plan = {'source_pin': BASE, 'trace_sha256': sha(OUT / 'trace/result.json'),
            'actors': actors, 'order': orders, 'boundary': 'fresh process source through final deployment link'}
    save(dest / 'plan.json', plan)

    def trial(role, name):
        entry = actors[role]
        return runner.compile_and_link(role, dest / name / role, entry['compile'], entry['link'],
                                       entry['reference_object'], entry['flags'], entry['artifact_sha256'])

    warmup = {role: trial(role, 'warmup') for role in actors}
    trials = [{role: trial(role, f'order-{index}') for role in order}
              for index, order in enumerate(orders, 1)]
    for sample in [warmup, *trials]:
        assert sample['mech-control']['stripped_executable'] == sample['mech-reuse']['stripped_executable']
        assert all(sample[role]['stripped_executable'] == warmup[role]['stripped_executable'] for role in actors)
    interval = 'source_through_link'
    seconds = {role: {field: median(row[role]['seconds'][field] for row in trials)
                      for field in warmup[role]['seconds']} for role in actors}
    paired = {role: median(row['mech-reuse']['seconds'][interval] / row[role]['seconds'][interval] for row in trials)
              for role in actors if role != 'mech-reuse'}
    phase_medians = {}
    for role in ('mech-control', 'mech-reuse'):
        samples = [{entry['phase']: entry['seconds'] for entry in row[role]['compiler_phases']} for row in trials]
        phase_medians[role] = {phase: median(row[phase] for row in samples) for phase in samples[0]}
    gains = [1000 * (row['mech-control']['seconds'][interval] - row['mech-reuse']['seconds'][interval]) for row in trials]
    record = {'plan_sha256': sha(dest / 'plan.json'), 'median_seconds': seconds, 'paired_ratios': paired,
              'compiler_phase_medians': phase_medians, 'paired_gains_ms': gains, 'median_paired_save_ms': median(gains),
              'warmup': warmup, 'trials': trials}
    save(dest / 'comparison.json', record)
    print('Build comparison', paired, gains, phase_medians, flush=True)


if __name__ == '__main__':
    actions = {'trace': trace, 'matched': matched}
    for action in sys.argv[1:]:
        actions[action]()
