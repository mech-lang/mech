#!/usr/bin/env python3
"""Rebuild both Rust policies against the same locked, native, bitcode-enabled dependencies."""
import argparse,json,os
from pathlib import Path
import run

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--mech-only',action='store_true');options=parser.parse_args()
 args=argparse.Namespace(mech=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),julia='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia',java_home='/opt/homebrew/opt/openjdk',gcc='/opt/homebrew/bin/gcc-16',gpp='/opt/homebrew/bin/g++-16',clang='/opt/homebrew/opt/llvm@22/bin/clang-22',threads=8,timeout=600,rust_deps=run.BUILD/'rust-deps-bitcode/release/deps')
 env=os.environ|{k:'8' for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}
 prepared=run.BUILD/'prepared.json';data=json.loads(prepared.read_text());manifest=json.loads((run.HERE/'manifest.json').read_text())['programs'];audit=json.loads((run.HERE/'algorithm-audit.json').read_text())['programs']
 programs=[dict(p,algorithm=audit[p['id']]) for p in manifest if not options.mech_only and (p['language']=='rust' or p['id'] in ('binarytrees-gpp-4','binarytrees-gpp-6'))]
 programs=[q for p in programs for q in ([p,dict(p,id=p['id']+'-thin',lto='thin')] if p['language']=='rust' else [dict(p,id=p['id']+'-'+cc,cc_backend=cc) for cc in ('gcc','llvm')])]
 programs += [dict(id='mech-binarytrees-'+policy,language='mech',path=source,simd=True,upstream_starred=False,algorithm=dict(strict=True,allocation='library-pool',reason='Source constructs and traverses every node, including leaves; existing core planned arena storage is retired by scope.')) for policy,source in [('compact','binarytrees.mec'),('parallel','binarytrees-parallel.mec')]]
 for p in programs:
  data['programs']=[r for r in data['programs'] if r['id']!=p['id']];data['excluded']=[r for r in data['excluded'] if r['id']!=p['id']]
  try:
   row=run.prepare(p,args,env)
   if row['language']=='rust':row['dependency_build']={'manifest_sha256':run.sha(run.HERE/'rust-deps/Cargo.toml'),'lock_sha256':run.sha(run.HERE/'rust-deps/Cargo.lock'),'rustflags':'-C target-cpu=native -C embed-bitcode=yes','rlib_sha256':{v.name:run.sha(v) for v in args.rust_deps.glob('*.rlib')}}
   data['programs'].append(row);print('prepared',p['id'],flush=True)
  except Exception as error:
   data['excluded'].append(dict(id=p['id'],language=p['language'],reason=str(error)+'\n'+str(getattr(error,'stderr','') or getattr(error,'stdout',''))));print('excluded',p['id'],str(error),flush=True)
  prepared.write_text(json.dumps(data,indent=2)+'\n')
if __name__=='__main__':main()
