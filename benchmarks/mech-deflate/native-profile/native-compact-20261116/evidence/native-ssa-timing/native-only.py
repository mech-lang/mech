"""Time the same Cranelift executable with and without the cached-join proof."""
import json
import os
import re
import statistics
import subprocess
from hashlib import file_digest
from pathlib import Path
from time import perf_counter

study = Path(__file__).resolve().parent
q = study.parent
emitter = q / 'host-opt-levels/opt3-cargo/release/emit_compact_size'
image = q / 'stale-cache-accounting/integrated-rebuild/frontend/program.mecb'
pin = json.loads((study / 'smaller-control/receipt.json').read_text())
runtime = json.loads((study / 'smaller-control/runtime.json').read_text())
env = {k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
flag = 'MECH_COMPACT_REUSE_KNOWN_JOIN_VALUES'
modes = {'control': {}, 'candidate': {flag:'1'}}
env.update(runtime['flags'], PATH='/usr/bin:/bin')
assert not any(k.startswith('MECH_COMPACT_LLVM') for k in env)
assert flag not in env

def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()

def time(actor, trial):
    out = study / 'native-only' / trial / actor
    out.mkdir(parents=True)
    command = list(map(str,(emitter,image,out)))
    with (out/'native.log').open('wb') as log:
        start = perf_counter()
        with subprocess.Popen(command,env=env|modes[actor],stdout=log,stderr=log) as proc:
            _, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = os.waitstatus_to_exitcode(status)
            elapsed = perf_counter()-start
            if proc.returncode != 0:
                raise subprocess.CalledProcessError(proc.returncode, command)
    observed = (out/'native.log').read_text()
    values = int(re.search(r'^compact-stage: compact values before=(\d+)$', observed, re.M)[1])
    compact = int(re.search(r'^compact-values: retained=(\d+)$', observed, re.M)[1])
    diff = {name:sha(out/name) for name in ('program.o','metadata.json')}
    assert diff == {'program.o':pin['object_sha256'], 'metadata.json':pin['metadata_sha256']}
    return {'actor':actor, 'seconds':elapsed, 'peak_rss_bytes':usage.ru_maxrss, 'command':command,
            'values_before_compaction':values, 'values_after_compaction':compact, 'sha256':diff,
            'flags': runtime['flags'] | modes[actor]}

assert sha(image) == runtime['image_sha256']
orders = [['control','candidate'], ['candidate','control']] * 6
warmup = [time(actor, 'warmup') for actor in modes]
trials = []
for number, order in enumerate(orders, 1):
    row = {actor:time(actor, f'pair-{number:02d}') for actor in order}
    trials.append(row)
    print(number, {actor:sample['seconds'] for actor,sample in row.items()}, flush=True)
summary = {actor: {key: statistics.median([row[actor][key] for row in trials])
                   for key in ('seconds','peak_rss_bytes')} for actor in modes}
ratios = [row['candidate']['seconds']/row['control']['seconds'] for row in trials]
report = {'emitter_sha256':sha(emitter),'image_sha256':sha(image),'orders':orders,'warmup':warmup,
          'trials':trials,'medians':summary,'paired_ratios':ratios,'paired_median':statistics.median(ratios)}
(study/'native-only/timing.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'medians':summary,'paired_ratios':ratios,'paired_median':report['paired_median']},indent=2))
