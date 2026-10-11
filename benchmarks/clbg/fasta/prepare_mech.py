#!/usr/bin/env python3
"""Append complete source policies to the already prepared comparison cohort."""
import argparse,json,os,shutil
import run
args=argparse.Namespace(mech=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),threads=8,timeout=300)
env=os.environ|{'RAYON_NUM_THREADS':'8'}
path=run.BUILD/'prepared.json';d=json.loads(path.read_text());d['programs']=[r for r in d['programs'] if r['language']!='mech']
for policy,source,simd,unroll in [('compact','fasta.mec',True,1),('compact-u4','fasta.mec',True,4),('optimized','fasta-optimized.mec',True,1),('optimized-u4','fasta-optimized.mec',True,4),('optimized-u8','fasta-optimized.mec',True,8),('optimized-scalar','fasta-optimized.mec',False,1),('optimized-scalar-u4','fasta-optimized.mec',False,4),('compact-auto','fasta.mec',True,None),('optimized-auto','fasta-optimized.mec',True,None)]:
 p=dict(id='mech-fasta-'+policy,language='mech',path=source,simd=simd,scan_unroll=unroll,upstream_starred=False,algorithm=dict(strict=True,reason='Exact LCG arithmetic and f64 cumulative probability search in counted Mech source.'))
 binary=run.BUILD/p['id'];project=run.Path(str(binary)+'.project')
 if binary.exists():
  archive=run.BUILD/'preserved'/run.sha(binary);archive.mkdir(parents=True,exist_ok=True)
  shutil.copy2(binary,archive/p['id'])
  if project.exists() and not (archive/project.name).exists():shutil.copytree(project,archive/project.name)
 row=run.prepare(p,args,env);d['programs'].append(row);print('prepared',row['id'],flush=True)
path.write_text(json.dumps(d,indent=2)+'\n')
