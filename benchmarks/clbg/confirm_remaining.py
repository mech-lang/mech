#!/usr/bin/env python3
"""Confirm general optimizations against the measured native-language winners."""
import argparse, json, random, statistics
from datetime import datetime, timezone
from pathlib import Path
import run_remaining as r

HERE=r.CLBG/'remaining-conformance'
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,default=HERE/'final-confirmation.json')
    parser.add_argument('--tag',default='')
    args=parser.parse_args();output=args.output
    if output.exists():raise RuntimeError('refusing to overwrite confirmation')
    rows=[];references={}
    for benchmark in ('regexredux','pidigits'):
        here=r.CLBG/benchmark;path=here/'results/apple-m3-ac-normal-20261009.json'
        report=json.loads(path.read_text());references[benchmark]=dict(path=str(path.relative_to(r.CLBG)),sha256=r.sha(path))
        for lang in ('c','cpp','rust','chapel','julia'):
            winner=min((p for p in report['programs'] if p['language']==lang),key=lambda p:p['median_seconds'])
            rows.append(dict(winner,benchmark=benchmark,samples_seconds=[]))
        if benchmark=='pidigits':
            # A native-width GNU typedef compatibility define now admits this
            # additional unchanged source on Darwin; confirm it alongside #4.
            source=next(p for p in json.loads((here/'manifest.json').read_text())['programs'] if p['id']=='pidigits-gpp-5')
            peer=r.prepare(here,source);r.execute(here,peer,False)
            rows.append(dict(peer,benchmark=benchmark))
        if benchmark=='regexredux':
            variants=[('mech-regex-final-sequential','regexredux.mec',{}),
                      ('mech-regex-final-async','regexredux.mec',{'MECH_SEQUENCE_ASYNC':'1'}),
                      ('mech-regex-final-reuse','regexredux.mec',{'MECH_SEQUENCE_ASYNC':'1','MECH_SEQUENCE_BUFFER_REUSE':'1'})]
        else:
            variants=[('mech-pi-final-compact','pidigits.mec',{}),
                      ('mech-pi-final-gated','pidigits-gated.mec',{}),
                      ('mech-pi-final-advance','pidigits-advance.mec',{})]
        for id,source,env in variants:
            id+=args.tag
            row=r.prepare(here,dict(id=id,path=source,language='mech',runtime_environment=env))
            rows.append(dict(row,benchmark=benchmark));print('prepared',id,flush=True)
    for row in rows:
        here=r.CLBG/row['benchmark']
        assert r.sha(here/row['path'])==row['source']['sha256']
        if row.get('executable_sha256'):assert r.sha(row['command'][0])==row['executable_sha256']
        r.execute(here,row,False);r.execute(here,row,True)
    report=dict(created_at=datetime.now(timezone.utc).isoformat(),programs=rows,references=references,
       protocol=dict(samples=7,power=r.POWER,selection='Final Mech policies and the fastest passing C/C++/Rust/Chapel/Julia variants from the preceding all-native source sweep, plus unchanged C++ pidigits #5 with the GNU native-width ulong alias supplied on Darwin.',
       workloads={'regexredux':'FASTA N=5000000','pidigits':'N=10000'},
       timed_region='Shuffled sequential fresh processes through exit; full output; startup/JIT included; builds, full warmup and byte comparison excluded',
       validation='Complete reference byte equality on small input, full warmup and every sample',source_metric='Nonblank noncomment physical LOC'),
       compiler_snapshot=r.snapshot(output),toolchains=r.toolchains(),sample_order=[],discarded_samples=[])
    rng=random.Random(20261010)
    for sample in range(7):
        order=rows.copy();rng.shuffle(order);report['sample_order'].append([p['id'] for p in order])
        for row in order:
            while True:
                r.guard.wait_for_environment(r.POWER)
                seconds=r.execute(r.CLBG/row['benchmark'],row,True,True)
                if seconds is not None:break
                report['discarded_samples'].append(dict(id=row['id'],sample=sample+1))
            row['samples_seconds'].append(seconds);row['median_seconds']=statistics.median(row['samples_seconds'])
            if len(row['samples_seconds'])>1:row['stdev_seconds']=statistics.stdev(row['samples_seconds'])
            output.write_text(json.dumps(report,indent=2)+'\n');print(sample+1,row['id'],round(seconds,6),flush=True)

if __name__=='__main__':main()
