"""Measure the indexed loop-use query as a same-compiler source-to-link ablation."""

import json
from pathlib import Path
from runpy import run_path
from statistics import median
import subprocess
import sys


HERE = Path(__file__).resolve().parent
ROOT = HERE/'loop-liveout-index'
ROAD = HERE.parent/'current-roadmap'
read = lambda path: json.loads(Path(path).read_text())
script = ROAD/'shared-terminal-returns/measure.py'
sys.path.insert(0, str(script.parent))
lab = run_path(str(script), run_name='indexed_loop_live_outs')
sha = lab['sha']
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=sys.argv[1], text=True).strip()
prior = read(HERE/'matched-heap-recovery/plan.json')
fixed = prior['actors']['mech-m1-shared-heap-layout']
ROLE = 'mech-small-indexed'
BEFORE = 'mech-small-unindexed'
CONTROL = 'mech-control-reference'


def prepare(controls, variants):
    assert controls == ['reference'] and variants == ['small-unindexed', 'small-indexed']
    actors = {}
    specifications = {
        CONTROL: ('control', HERE/'heap-match-preserve/linked-proof.json'),
        BEFORE: ('unindexed', HERE/'literal-frame-cleanup/linked-proof.json'),
        ROLE: ('indexed', HERE/'literal-frame-cleanup/linked-proof.json'),
    }
    compiler = None
    exact = read(ROOT/'exact-workers.json')
    for name, (directory, proof_path) in specifications.items():
        trial = ROOT/directory
        receipt = read(trial/'receipt.json')
        assert receipt['source_commit'] == head and receipt['matches_recorded_artifacts']
        assert sha(receipt['command'][0]) == receipt['compiler_sha256']
        if compiler is None:
            compiler = receipt['compiler_sha256']
        assert receipt['compiler_sha256'] == compiler
        assert exact[directory]['object_sha256'] == receipt['object_sha256']
        proof = read(proof_path)
        assert sha(proof['result']) == proof['sha256']
        linked = read(proof['result'])
        assert linked['matches_success_observations'] == 162 and len(linked['checked_failure_cases']) == 6
        obj = trial/'frontend/program.o'
        assert sha(obj) == receipt['object_sha256']
        actors[name] = fixed | {
            'compile': receipt['command'], 'compiler_sha256': compiler,
            'reference_object': str(obj), 'flags': receipt['flags'],
            'link': [str(obj) if arg == fixed['reference_object'] else arg for arg in fixed['link']],
            'artifact_sha256': {'encoder': receipt['object_sha256'], 'metadata': receipt['metadata_sha256'],
                                'bytecode': receipt['bytecode_sha256']},
            'text_bytes': receipt['text_bytes'], 'linked_proof_sha256': proof['sha256'],
        }
    assert actors[BEFORE]['flags'] | {'MECH_COMPACT_INDEX_LOOP_LIVE_OUTS': '1'} == actors[ROLE]['flags']
    actors.update({peer: prior['actors'][peer] for peer in lab['PEERS']})
    for actor in actors.values():
        assert sha(actor['compile'][0]) == actor['compiler_sha256']
        assert all(sha(path) == pin for path, pin in actor['host_inputs_sha256'].items())
    timer = lab['runtime'].MATCHED.with_name('timing-driver.py')
    assert sha(timer) == prior['timer_sha256']
    return actors, timer


lab['acquire'].__globals__.update(BASE=head, construct=prepare)
output = HERE/'matched-loop-liveout-index'
if '--summarize-only' not in sys.argv:
    lab['acquire'](output, ['reference'], ['small-unindexed', 'small-indexed'])
raw = read(output/'comparison.json')
rows, roles = raw['observations'], raw['medians']
result = {key: raw[key] for key in ['plan_sha256', 'medians', 'phases', 'paired_ratios']}
section = 'source_through_link'
result.update({
    'source_commit': head, 'trials': len(rows),
    'compiler_sha256': read(ROOT/'indexed/receipt.json')['compiler_sha256'],
    'source_cpu_seconds': {role: median(sum(row[role]['phase_cpu_seconds']['source'].values())
                                       for row in rows) for role in roles},
    'source_rss_bytes': {role: median(row[role]['phase_peak_rss_bytes']['source'] for row in rows)
                         for role in roles},
    'stripped_executables': {role: raw['warmup'][role]['stripped_executable'] for role in roles},
    'paired_ms_vs_64652': {role: median((row[role]['seconds'][section]-row[CONTROL]['seconds'][section])*1000
                                      for row in rows) for role in roles if role.startswith('mech-')},
    'paired_ms_indexed_vs_unindexed': median((row[ROLE]['seconds'][section]-row[BEFORE]['seconds'][section])*1000
                                            for row in rows),
    'indexed_faster_than_unindexed': sum(row[ROLE]['seconds'][section]<row[BEFORE]['seconds'][section]
                                        for row in rows),
})
(output/'summary.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2), flush=True)
