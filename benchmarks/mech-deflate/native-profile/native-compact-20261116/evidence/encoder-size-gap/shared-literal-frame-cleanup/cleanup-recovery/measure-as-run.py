"""Measure exact full/short-schedule workers against the same-compiler control and peers."""

import json
from pathlib import Path
from runpy import run_path
from statistics import median
import subprocess
import sys


HERE = Path(__file__).resolve().parent
extra = [arg for arg in sys.argv[2:] if arg != '--summarize-only']
ROOT = HERE/(extra[0] if extra else 'literal-frame-route-schedule')
CANDIDATE = extra[1] if len(extra) > 1 else 'routes'
OUTPUT_NAME = extra[2] if len(extra) > 2 else 'matched-cleanup-schedule-recovery'
ROAD = HERE.parent/'current-roadmap'
read = lambda path: json.loads(Path(path).read_text())
script = ROAD/'shared-terminal-returns/measure.py'
sys.path.insert(0, str(script.parent))
lab = run_path(str(script), run_name='cleanup_schedule_recovery')
sha = lab['sha']
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=sys.argv[1], text=True).strip()
prior = read(HERE/'matched-heap-recovery/plan.json')
fixed = prior['actors']['mech-m1-shared-heap-layout']
base = read(ROOT/'reference/rebuild.json')
original = read(HERE/'literal-frame-cleanup/receipt.json')


def prepare(controls, variants):
    assert controls == ['layout'] and variants == ['full-cleanup', 'route-cleanup']
    assert base['source_commit'] == head
    full, routes = read(ROOT/'full/receipt.json'), read(ROOT/CANDIDATE/'receipt.json')
    for row in [full, routes]:
        assert row['source_commit'] == head and row['matches_qualified_64444_artifacts']
        assert sha(row['command'][0]) == row['compiler_sha256'] == base['compiler_sha256']
        assert row['object_sha256'] == original['layout']['object_sha256']
        assert row['metadata_sha256'] == original['metadata_sha256']
        assert row['bytecode_sha256'] == original['bytecode_sha256']
    compiler = full['command'][0]
    command = [compiler, *base['phases']['source_to_native']['command'][1:]]
    specs = {
        'mech-control-layout': (ROOT/'reference/native/program.o', base['native_flags'],
                                base['object_sha256'], base['metadata_sha256'], base['bytecode_sha256'],
                                64652, HERE/'heap-match-preserve/linked-proof.json'),
        'mech-full-cleanup': (ROOT/'full/frontend/program.o', full['flags'], full['object_sha256'],
                             full['metadata_sha256'], full['bytecode_sha256'], 64444,
                             HERE/'literal-frame-cleanup/linked-proof.json'),
        'mech-route-cleanup': (ROOT/CANDIDATE/'frontend/program.o', routes['flags'], routes['object_sha256'],
                              routes['metadata_sha256'], routes['bytecode_sha256'], 64444,
                              HERE/'literal-frame-cleanup/linked-proof.json'),
    }
    actors = {}
    for name, (obj, flags, encoder, metadata, bytecode, size, proof_path) in specs.items():
        assert sha(obj) == encoder and 'MECH_COMPACT_FINISH_CFG_REPORT' not in flags
        proof = read(proof_path)
        assert sha(proof['result']) == proof['sha256']
        linked = read(proof['result'])
        assert linked['matches_success_observations'] == 162 and len(linked['checked_failure_cases']) == 6
        actors[name] = fixed | {
            'compile': command, 'compiler_sha256': sha(compiler), 'reference_object': str(obj), 'flags': flags,
            'link': [str(obj) if arg == fixed['reference_object'] else arg for arg in fixed['link']],
            'artifact_sha256': {'encoder': encoder, 'metadata': metadata, 'bytecode': bytecode},
            'text_bytes': size, 'linked_proof_sha256': proof['sha256'],
        }
    actors.update({peer: prior['actors'][peer] for peer in lab['PEERS']})
    for actor in actors.values():
        assert sha(actor['compile'][0]) == actor['compiler_sha256']
        assert all(sha(path) == pin for path, pin in actor['host_inputs_sha256'].items())
    timer = lab['runtime'].MATCHED.with_name('timing-driver.py')
    assert sha(timer) == prior['timer_sha256']
    return actors, timer


lab['acquire'].__globals__.update(BASE=head, construct=prepare)
output = HERE/OUTPUT_NAME
if '--summarize-only' not in sys.argv:
    lab['acquire'](output, ['layout'], ['full-cleanup', 'route-cleanup'])
raw = read(output/'comparison.json')
rows, roles = raw['observations'], raw['medians']
result = {key: raw[key] for key in ['plan_sha256', 'medians', 'phases', 'paired_ratios']}
section = 'source_through_link'
result.update({
    'source_commit': head, 'compiler_sha256': base['compiler_sha256'], 'trials': len(rows),
    'candidate_schedule': CANDIDATE,
    'source_cpu_seconds': {role: median(sum(row[role]['phase_cpu_seconds']['source'].values())
                                       for row in rows) for role in roles},
    'source_rss_bytes': {role: median(row[role]['phase_peak_rss_bytes']['source'] for row in rows)
                         for role in roles},
    'stripped_executables': {role: raw['warmup'][role]['stripped_executable'] for role in roles},
    'paired_ms_vs_64652': {role: median((row[role]['seconds'][section]-row['mech-control-layout']['seconds'][section])*1000
                                      for row in rows) for role in roles if role.startswith('mech-')},
    'paired_ms_routes_vs_full': median((row['mech-route-cleanup']['seconds'][section]-row['mech-full-cleanup']['seconds'][section])*1000
                                      for row in rows),
    'routes_faster_than_full': sum(row['mech-route-cleanup']['seconds'][section]<row['mech-full-cleanup']['seconds'][section]
                                  for row in rows),
})
(output/'summary.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2), flush=True)
