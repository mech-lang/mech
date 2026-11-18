"""Compare the recovered cleanup and combined heap/literal recipes from one compiler."""

from pathlib import Path
from runpy import run_path
from statistics import median
import json
import subprocess
import sys


HERE = Path(__file__).resolve().parent
ROAD = HERE.parent/'current-roadmap'
read = lambda path: json.loads(path.read_text())
source = Path(sys.argv[1]).resolve()
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=source, text=True).strip()
script = ROAD/'shared-terminal-returns/measure.py'
sys.path.insert(0, str(script.parent))
lab = run_path(str(script), run_name='reconsidered_size_builds')
sha = lab['sha']
prior = read(HERE/'matched-heap-recovery/plan.json')
fixed = prior['actors']['mech-m1-shared-heap-layout']
current = read(HERE/'literal-frame-cleanup/receipt.json')
base = read(HERE/'literal-frame-control/rebuild.json')


def prepare(controls, variants):
    assert controls == ['layout'] and variants == ['final-cfg', 'literal-frame-cleanup']
    assert current['source_commit'] == base['source_commit'] == head
    assert sha(current['compile'][0]) == current['compiler_sha256'] == base['compiler_sha256']
    assert sha(current['compile'][1]) == sha(fixed['compile'][1])
    expected_changes = {'MECH_COMPACT_FINISH_CFG_CLEANUP': '1',
                        'MECH_COMPACT_LITERAL_ALLOCATION_FROM_FRAME': '1'}
    assert base['native_flags'] | expected_changes == current['flags']
    specifications = {
        'mech-control-layout': ('heap-match-preserve', base['native_flags']),
        'mech-final-cfg': ('final-cfg-revisit', base['native_flags'] | {'MECH_COMPACT_FINISH_CFG_CLEANUP': '1'}),
        'mech-literal-frame-cleanup': ('literal-frame-cleanup', current['flags']),
    }
    actors = {}
    for name, (directory, flags) in specifications.items():
        trial = HERE/directory
        receipt = read(trial/'receipt.json')
        proof = read(trial/'linked-proof.json')
        linked = read(Path(proof['result']))
        assert sha(proof['result']) == proof['sha256']
        assert linked['matches_success_observations'] == 162 and len(linked['checked_failure_cases']) == 6
        obj = trial/'bound/program.o'
        assert sha(obj) == receipt['layout']['object_sha256']
        assert fixed['link'].count(fixed['reference_object']) == 1
        actors[name] = fixed | {
            'compile': current['compile'], 'compiler_sha256': current['compiler_sha256'],
            'flags': flags, 'reference_object': str(obj),
            'link': [str(obj) if arg == fixed['reference_object'] else arg for arg in fixed['link']],
            'artifact_sha256': {'encoder': sha(obj), 'metadata': receipt['metadata_sha256'],
                                'bytecode': receipt['bytecode_sha256']},
            'text_bytes': receipt['text_bytes'], 'linked_proof_sha256': proof['sha256'],
        }
    actors.update({peer: prior['actors'][peer] for peer in lab['PEERS']})
    for name, actor in actors.items():
        assert sha(actor['compile'][0]) == actor['compiler_sha256'], name
        assert all(sha(path) == pin for path, pin in actor['host_inputs_sha256'].items()), name
    timer = lab['runtime'].MATCHED.with_name('timing-driver.py')
    assert sha(timer) == prior['timer_sha256']
    return actors, timer


lab['acquire'].__globals__.update(BASE=head, construct=prepare)
output = HERE/'matched-literal-frame-cleanup'
if '--summarize-only' not in sys.argv:
    lab['acquire'](output, ['layout'], ['final-cfg', 'literal-frame-cleanup'])
raw = read(output/'comparison.json')
roles = raw['medians']
rows = raw['observations']
control = 'mech-control-layout'
section = 'source_through_link'
result = {key: raw[key] for key in ('plan_sha256', 'medians', 'phases', 'paired_ratios')}
result.update({
    'source_commit': head,
    'compiler_sha256': current['compiler_sha256'],
    'source_cpu_seconds': {role: median(sum(row[role]['phase_cpu_seconds']['source'].values())
                                       for row in rows) for role in roles},
    'source_rss_bytes': {role: median(row[role]['phase_peak_rss_bytes']['source'] for row in rows)
                         for role in roles},
    'stripped_executables': {role: raw['warmup'][role]['stripped_executable'] for role in roles},
    'paired_ms_vs_64652': {role: median((row[role]['seconds'][section]-row[control]['seconds'][section])*1000
                                      for row in rows) for role in roles if role.startswith('mech-')},
    'trials': len(rows),
})
(output/'summary.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(result, indent=2), flush=True)
