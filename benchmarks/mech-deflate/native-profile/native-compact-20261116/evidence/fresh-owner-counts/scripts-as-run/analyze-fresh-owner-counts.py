"""Per-case uncertainty for the two complete fresh owner count benchmark orders."""
from collections import Counter
from importlib import import_module
from pathlib import Path
import gzip
import json

q = Path(__file__).resolve().parent
stats = import_module('cold-uncertainty')
label = 'cranelift-fresh-owner-counts'
folders = [q / (label + suffix) for suffix in ('-full', '-full-reverse')]
for folder in folders:
    complete = json.loads((folder / 'complete.json').read_text())
    assert complete['cases'] == 81 and complete['balanced_blocks_per_case'] == 96
fixtures = json.loads((q / 'fixtures.json').read_text())
rows = {}
for index, fixture in enumerate(fixtures):
    name = fixture['name']
    strata = []
    for folder in folders:
        blocks = json.loads(gzip.decompress((folder / name / 'observations.json.gz').read_bytes()))
        strata.append(stats.strata(blocks))
    forward = stats.interval(strata[0], index)
    reverse = stats.interval(strata[1], index + 1000)
    combined = stats.interval(strata[0] + strata[1], index + 2000)
    if combined['upper'] <= stats.RESOLUTION:
        decision = 'bounded at resolution in the two full orders'
    elif combined['lower'] > stats.RESOLUTION:
        decision = 'slowdown above resolution'
    else:
        decision = 'inconclusive'
    rows[name] = {'forward': forward, 'reverse': reverse, 'combined': combined,
                  'decision': decision}
targets = {name for name, row in rows.items()
           if row['combined']['upper'] > stats.RESOLUTION}
targets.update(('empty', 'one-stored', 'seven-stored-prefix', '320-overlap', 'prefix-periodic-1024'))
record = {'resolution': stats.RESOLUTION, 'resamples': stats.RESAMPLES,
          'interval': '90% paired block bootstrap, stratified by run and control/candidate order',
          'scope': 'Per-case estimates; fresh same-worker follow-up remains separate.',
          'cases': rows, 'followup_cases': sorted(targets)}
(q / (label + '-uncertainty.json')).write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps({'decisions': dict(Counter(row['decision'] for row in rows.values())),
                  'followup_cases': sorted(targets)}, indent=2), flush=True)
