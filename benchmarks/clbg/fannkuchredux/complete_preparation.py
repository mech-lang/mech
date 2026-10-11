#!/usr/bin/env python3
"""Rebuild current Mech policies and recover Java class-name preparation."""
import json,os
from types import SimpleNamespace
import run
args=SimpleNamespace(mech=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),clang='/opt/homebrew/opt/llvm@22/bin/clang-22',gcc='/opt/homebrew/bin/gcc-16',gpp='/opt/homebrew/bin/g++-16',julia='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia',java_home='/opt/homebrew/opt/openjdk',threads=8,timeout=300)
p=run.BUILD/'prepared.json';d=json.loads(p.read_text());byid={r['id']:r for r in d['programs']};excluded={r['id']:r for r in d['excluded']}
variants=[dict(id='mech-fannkuch-'+name,language='mech',path=source,simd=simd,upstream_starred=False) for name,source,simd in [('simd','fannkuchredux.mec',True),('scalar','fannkuchredux.mec',False),('coarse','fannkuchredux-coarse.mec',True),('paired','fannkuchredux-paired.mec',True),('cached','fannkuchredux-cached.mec',True)]]
variants += [r for r in json.loads((run.HERE/'manifest.json').read_text())['programs'] if r['language']=='java' and r['id'] not in byid]
env=os.environ|{k:'8' for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}
for r in variants:
 try:byid[r['id']]=run.prepare(r,args,env);excluded.pop(r['id'],None);print('prepared',r['id'],flush=True)
 except Exception as error:excluded[r['id']]=dict(id=r['id'],language=r['language'],reason=str(error));print('excluded',r['id'],str(error),flush=True)
p.write_text(json.dumps(dict(programs=list(byid.values()),excluded=list(excluded.values())),indent=2)+'\n')
