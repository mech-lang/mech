"""Confirm the observed k-nucleotide gap; preserve explicit algorithm exclusions."""
import copy
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import random
import statistics

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[2]
spec = importlib.util.spec_from_file_location('compact', CLBG/'compact/run.py')
compact = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compact)
POWER = dict(source='AC Power', low_power_mode=0)

def sha(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def main():
    old = json.loads((CLBG/'retime/results/apple-m3-ac-normal-20261008.json').read_text())
    arrays = json.loads((HERE.parent/'results/timings.json').read_text())
    selected = [('knucleotide-rust-7', 'rust'), ('knucleotide-rust-6-panicabort', 'rust')]
    rows = []
    for identifier, language in selected:
        original = next(p for p in old['programs'] if p['id'] == identifier)
        source = CLBG/'knucleotide'/original['path']
        assert sha(source) == original['source']['sha256']
        assert sha(original['command'][0]) == old['asset_hashes'][original['command'][0]]
        rows.append(dict(id=identifier, benchmark='knucleotide', language=language,
            source=str(source), source_sha256=sha(source),
            command=original['command'], executable_sha256=sha(original['command'][0]),
            compile_command=original['compile_command'],
            loc=original['source']['nonblank_noncomment_lines'], strict=True))
    for key in ['knucleotide:bqn-knucleotide', 'knucleotide:k-knucleotide.growler', 'knucleotide:mech-control']:
        original = arrays['programs'][key]
        p = {k:copy.deepcopy(original[k]) for k in ['id','benchmark','language','command','source','source_sha256']}
        p['executable_sha256'] = sha(p['command'][0])
        assert p['executable_sha256'] == original.get('runtime_sha256',original.get('binary_sha256'))
        p['loc'] = original.get('loc',17)
        p['strict'] = p['language'] == 'mech'
        if not p['strict']:
            p['exclusion_reason'] = 'Correct output, but selective matching replaces the required all-key hash-table lookup/update work at k=1,2,3,4,6,12,18.'
        rows.append(p)
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
        requirements_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/knucleotide.html',
        protocol='Five randomized sequential full-workload fresh-process samples, AC/Low Power Mode off; full output checked; startup and input parsing included; builds and verification excluded.',
        full_input=dict(path=str(compact.FULL_INPUT),sha256=sha(compact.FULL_INPUT),
            bytes=compact.FULL_INPUT.stat().st_size,three_letters=125_000_000,
            required_window_updates=sum(125_000_000-k+1 for k in [1,2,3,4,6,12,18])),
        programs=rows,orders=[],discarded=[])
    for p in rows:
        p['samples_seconds']=[]
        p['sample_validation']=[]
        compact.wait_for_environment(POWER)
        p['small_validation']=compact.execute(p,False,60)['validation']
    rng=random.Random(202610091)
    def save():
        (HERE/'confirmation.json').write_text(json.dumps(report,indent=2)+'\n')
    save()
    for n in range(5):
        order=rows[:];rng.shuffle(order)
        report['orders'].append([p['id'] for p in order])
        for p in order:
            while True:
                compact.wait_for_environment(POWER)
                assert sha(p['source']) == p['source_sha256']
                assert sha(p['command'][0]) == p['executable_sha256']
                r=compact.execute(p,True,60,watch_compilers=True,expected_power=POWER)
                if r['competing_compilers'] or r['power_changes']:
                    report['discarded'].append(dict(program=p['id'],round=n+1,result=r));save();continue
                p['samples_seconds'].append(r['seconds'])
                p['sample_validation'].append(r['validation'])
                print(n+1,p['id'],f"{r['seconds']:.6f}",flush=True)
                save();break
    for p in rows:
        p['median_seconds']=statistics.median(p['samples_seconds'])
        p['sample_stdev_seconds']=statistics.stdev(p['samples_seconds'])
    report['status']='complete'
    report['finished_at']=datetime.now(timezone.utc).isoformat();save()
    for p in sorted(rows,key=lambda p:p['median_seconds']):
        print(p['id'],p['median_seconds'],p['sample_stdev_seconds'],'strict=',p['strict'])

if __name__=='__main__':main()
