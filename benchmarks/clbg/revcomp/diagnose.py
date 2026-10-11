#!/usr/bin/env python3
"""Paired precise timing of preserved baseline, current policies and fast controls."""
import json,os,random,statistics
from pathlib import Path
import run
report=json.loads((run.HERE/'results/apple-m3-precise-25000000.json').read_text())
prepared=json.loads((run.BUILD/'prepared.json').read_text())['programs']
rows=[r.copy() for r in prepared if r['language']=='mech']
for lang in ('c','rust'):
 id=report['endpoints']['languages'][lang]['fastest'];rows.append(next(r.copy() for r in prepared if r['id']==id))
rows.append({'id':'mech-preserved-line-input-library-format','language':'mech','command':[str(run.BUILD/'mech-profile')],'runtime_environment':{'MECH_SEQUENCE_PARTITIONS':'4'},'execution_mode':'Preserved first implementation: identical transform composition, per-line input and library wrapping; source was densely formatted.'})
env=os.environ|{'RAYON_NUM_THREADS':'8'};small,full,small_sha,full_sha,_=run.references()
for r in rows:
 r['samples_seconds']=[]
 _,digest,_,_=run.execute(r['command'],env|r['runtime_environment'],full,180);assert digest==full_sha
rng=random.Random(905)
for sample in range(7):
 order=rows.copy();rng.shuffle(order)
 for r in order:
  elapsed,digest,_,_=run.execute(r['command'],env|r['runtime_environment'],full,180);assert digest==full_sha;r['samples_seconds'].append(elapsed)
  print(f"{sample+1}/7 {r['id']} {elapsed:.6f}",flush=True)
for r in rows:r['median_seconds']=statistics.median(r['samples_seconds']);r['executable_sha256']=run.sha(r['command'][0])
output=run.HERE/'results/paired-optimizations.json'
output.write_text(json.dumps({'samples':7,'timing_wait':'Blocking waitpid with POSIX deadline; file hashing outside timer.','all_full_outputs_passed':True,'programs':rows,'compiler_snapshot':run.snapshot(output)},indent=2)+'\n')
