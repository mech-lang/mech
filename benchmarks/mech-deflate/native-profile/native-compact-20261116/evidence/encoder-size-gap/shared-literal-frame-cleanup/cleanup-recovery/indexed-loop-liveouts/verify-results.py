"""Recompute the published cohort from its raw records on any Python host."""
from hashlib import sha256
from pathlib import Path
from statistics import median
import gzip
import json
import random

HERE = Path(__file__).resolve().parent
read = lambda path: json.loads(path.read_text())
raw = json.loads(gzip.decompress((HERE/'build-comparison.json.gz').read_bytes()))
summary = read(HERE/'deployment.json')
plan = read(HERE/'build-plan.json')
assert sha256((HERE/'build-plan.json').read_bytes()).hexdigest() == raw['plan_sha256'] == summary['plan_sha256']
rows = raw['observations']
assert len(rows) == summary['trials'] == 10
assert len(plan['orders']) == 10
for order, row in zip(plan['orders'], rows):
    assert list(row) == order and set(row) == set(plan['actors'])
roles = list(plan['actors'])
for position in range(len(roles)):
    assert all(sum(order[position] == actor for order in plan['orders']) == 2 for actor in roles)
medians = {
    actor: {phase: median(row[actor]['seconds'][phase] for row in rows)
            for phase in raw['warmup'][actor]['seconds']}
    for actor in roles
}
assert medians == raw['medians'] == summary['medians']
for actor in roles:
    assert summary['source_cpu_seconds'][actor] == median(sum(row[actor]['phase_cpu_seconds']['source'].values()) for row in rows)
    assert summary['source_rss_bytes'][actor] == median(row[actor]['phase_peak_rss_bytes']['source'] for row in rows)
    assert all(row[actor]['stripped_executable'] == summary['stripped_executables'][actor] for row in rows)
for actor, values in summary['paired_ratios'].items():
    for peer, value in values.items():
        assert value == median(row[actor]['seconds']['source_through_link']/row[peer]['seconds']['source_through_link'] for row in rows)
selected, original, control = 'mech-small-indexed', 'mech-small-unindexed', 'mech-control-reference'
saving = [(row[original]['seconds']['source_through_link']-row[selected]['seconds']['source_through_link'])*1000 for row in rows]
assert saving == summary['paired_ms_saved_vs_unindexed']
assert median(saving) == -summary['paired_ms_indexed_vs_unindexed']
assert sum(value > 0 for value in saving) == summary['indexed_faster_than_unindexed'] == 9
rng = random.Random(9951118)
boot = sorted(median(rng.choices(saving, k=len(saving))) for _ in range(20000))
assert [boot[999], boot[18999]] == summary['paired_median_ms_saved_90_percent_interval']
for actor, difference in summary['paired_ms_vs_64652'].items():
    assert difference == median((row[actor]['seconds']['source_through_link']-row[control]['seconds']['source_through_link'])*1000 for row in rows)
phase = lambda row: {part['phase']: part['seconds'] for part in row['compiler_phases']}
for actor, phases in summary['phases'].items():
    assert phases == {name: median(phase(row[actor])[name] for row in rows) for name in phases}
    assert summary['explicit_phase_residual_seconds'][actor] == median(row[actor]['seconds']['source']-sum(phase(row[actor]).values()) for row in rows)
emissions = {name: read(HERE/(name+'-emission.json')) for name in ('control', 'unindexed', 'indexed')}
assert emissions['unindexed']['flags'] | {'MECH_COMPACT_INDEX_LOOP_LIVE_OUTS': '1'} == emissions['indexed']['flags']
links = read(HERE/'exact-workers.json')
for name, receipt in emissions.items():
    assert receipt['matches_recorded_artifacts']
    assert receipt['source_commit'] == summary['source_commit']
    assert receipt['compiler_sha256'] == summary['compiler_sha256']
    assert receipt['object_sha256'] == links[name]['object_sha256']
    assert receipt['metadata_sha256'] == links[name]['metadata_sha256']
for artifact in ('object_sha256', 'metadata_sha256', 'bytecode_sha256', 'text_bytes'):
    assert emissions['indexed'][artifact] == emissions['unindexed'][artifact]
assert emissions['indexed']['text_bytes'] == 64444
assert links['indexed']['worker_sha256'] == links['unindexed']['worker_sha256'] == read(HERE.parent.parent/'candidate.json')['worker_sha256']
rebuilt = read(HERE/'source-rebuild.json')
assert rebuilt['verified_exact_calls'] == 162 and rebuilt['full_integrated_source_identity']
assert rebuilt['checked_failure_probe']['cases'] == 6 and rebuilt['checked_failure_probe']['exact_recoveries'] == 12
print(f'Verified {len(rows)} balanced trials, {median(saving):.6f} ms paired saving, 9/10 faster, exact 64,444 B worker, and source checks.')
