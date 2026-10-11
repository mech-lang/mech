#!/usr/bin/env python3
"""Build general recursion-inlining policies using the unchanged Mech source."""
import argparse,json,os,subprocess
import run,conformance

def main():
 args=argparse.Namespace(mech=str(conformance.MECH),threads=8,timeout=600)
 env=conformance.ENV
 path=run.BUILD/'prepared.json';data=json.loads(path.read_text())
 data['programs']=[r for r in data['programs'] if r['id']!='mech-binarytrees-parallel-inline2']
 for budget in (0,1):
  p=dict(id=f'mech-binarytrees-parallel-inline{budget}',language='mech',path='binarytrees-parallel.mec',simd=True,recursive_inline=budget,upstream_starred=False,algorithm=dict(strict=True,allocation='library-pool',reason='Unchanged complete Mech source; general bounded recursive inlining policy; core planned record arena.'))
  row=run.prepare(p,args,env)
  for n in list(range(13))+[15]:
   expected,_=conformance.expected(n)
   assert subprocess.check_output([*row['command'],str(n)],env=env)==expected,(p['id'],n)
  data['programs']=[r for r in data['programs'] if r['id']!=p['id']]+[row]
  data['excluded']=[r for r in data['excluded'] if r['id']!=p['id']]
  path.write_text(json.dumps(data,indent=2)+'\n')
  print('prepared and validated 14 inputs:',p['id'],flush=True)
if __name__=='__main__':main()
