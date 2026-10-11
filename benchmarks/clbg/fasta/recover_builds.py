#!/usr/bin/env python3
"""Retry excluded source builds after recording platform/compiler settings."""
import argparse,json,os
import run

def main():
 args=argparse.Namespace(mech=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),julia='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia',java_home='/opt/homebrew/opt/openjdk',gcc='/opt/homebrew/bin/gcc-16',gpp='/opt/homebrew/bin/g++-16',clang='/opt/homebrew/opt/llvm@22/bin/clang-22',threads=8,timeout=300)
 env=os.environ|{k:'8' for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}
 path=run.BUILD/'prepared.json';d=json.loads(path.read_text());audit=json.loads((run.HERE/'algorithm-audit.json').read_text())['programs']
 sources=[dict(p,algorithm=audit[p['id']]) for p in json.loads((run.HERE/'manifest.json').read_text())['programs']]
 candidates={p['id']:p for source in sources for p in ([dict(source,id=source['id']+'-'+cc,cc_backend=cc) for cc in ('gcc','llvm')] if source['language'] in ('c','cpp') else [source,dict(source,id=source['id']+'-thin',lto='thin')] if source['language']=='rust' else [source])}
 excluded=[]
 for old in d['excluded']:
  p=candidates[old['id']]
  try:d['programs'].append(run.prepare(p,args,env));print('recovered',p['id'],flush=True)
  except Exception as error:
   reason=(str(error)+'\n'+str(getattr(error,'stderr','') or getattr(error,'stdout','')))[-3500:];excluded.append(dict(id=p['id'],language=p['language'],reason=reason));print('excluded',p['id'],reason[-700:],flush=True)
 d['excluded']=excluded;path.write_text(json.dumps(d,indent=2)+'\n')
if __name__=='__main__':main()
