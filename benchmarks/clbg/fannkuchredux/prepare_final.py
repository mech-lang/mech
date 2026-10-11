#!/usr/bin/env python3
"""Prepare default ARM lowering and explicit paired-checksum/range variants."""
import json, os
from types import SimpleNamespace
import run

args=SimpleNamespace(mech=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),
    clang='/opt/homebrew/opt/llvm@22/bin/clang-22',gcc='/opt/homebrew/bin/gcc-16',
    gpp='/opt/homebrew/bin/g++-16',julia='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia',
    java_home='/opt/homebrew/opt/openjdk',threads=8,timeout=300)
path=run.BUILD/'prepared.json';data=json.loads(path.read_text())
rows={r['id']:r for r in data['programs']};excluded={r['id']:r for r in data['excluded']}
env=os.environ|{'RAYON_NUM_THREADS':'8'}
for name,source in [('default','fannkuchredux.mec'),('even','fannkuchredux-even.mec'),('aligned','fannkuchredux-aligned.mec')]:
    p=dict(id='mech-fannkuch-'+name,language='mech',path=source,simd=True,
           byte_index='auto',upstream_starred=False)
    try:
        rows[p['id']]=run.prepare(p,args,env);excluded.pop(p['id'],None)
        print('prepared',p['id'],flush=True)
    except Exception as error:
        excluded[p['id']]=dict(id=p['id'],language='mech',reason=str(error))
        print('excluded',p['id'],str(error),flush=True)
path.write_text(json.dumps(dict(programs=list(rows.values()),excluded=list(excluded.values())),indent=2)+'\n')
