"""Refresh measured owners on the current selected worker; diagnostics only."""
from pathlib import Path
from hashlib import sha256
from collections import Counter
import ast,json,os,re,subprocess,time
L=Path(__file__).resolve().parent;I=L.parent;Q=I.parent;P=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116');O=L/'current-runtime-profiles';O.mkdir(exist_ok=True);read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();selected=I/'natural4k-runtime-target/hot-matcher-facts/guards-profiled/bound';peers=read(Q/'pinned-peers/pins.json');fixtures=read(Q/'fixtures.json');actors={'mech':{'command':[str(selected/'mech-worker'),str(selected/'metadata.json'),str(Q/'fixtures.json')],'sha256':'88321c097d29b7219f639abbc5fcce727f0602de2d05ab24e5e2b47f2996c3b4','warmups':32},'libdeflate':{'command':peers['actors']['libdeflate']['command'],'sha256':peers['actors']['libdeflate']['binary_sha256'],'warmups':80}};cases=['prefix-random-1048576','prefix-natural-4096']
plan={'source_commit':'2bc4014587fda4f31bdade02abd0cd5144d80e23','cases':cases,'actors':actors,'sample_seconds':3,'interval_ms':1,'load_before':os.getloadavg(),'scope':'Fresh untimed diagnostic captures on the byte-identical selected 63692B worker and pinned C. Refresh hot-owner attribution after retained spill/guard combinations. Existing full-order ratios remain the latency evidence; no new optimization or runtime campaign is claimed.'};(O/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
for name in cases:
 index=next(i for i,c in enumerate(fixtures) if c['name']==name)
 for actor,job in actors.items():
  assert sha(job['command'][0])==job['sha256'];folder=O/name/actor;folder.mkdir(parents=True)
  with (folder/'stderr.log').open('w') as err,(folder/'protocol.log').open('w') as protocol:
   child=subprocess.Popen(job['command'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err,text=True)
   try:
    assert child.stdout.readline().strip()=='SESSION_READY';child.stdin.write(f'C\t{index}\t0\n');child.stdin.flush();assert child.stdout.readline().strip()==f'CASE_READY\t{name}';child.stdin.write('W'*job['warmups']+'NN');child.stdin.flush();time.sleep(.4)
    with (folder/'sample.log').open('w') as log:result=subprocess.run(['/usr/bin/sample',str(child.pid),'3','1','-file',str(folder/'sample.txt')],stdout=log,stderr=log,check=True)
    rows=[]
    while True:
     line=child.stdout.readline();assert line,(name,actor,child.poll());protocol.write(line)
     if line.startswith('{'):rows.append(json.loads(line))
     if line.strip()==f'CASE_DONE\t{name}':break
    assert len(rows)==2 and all(all(row[k] for k in ['independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for row in rows);child.stdin.write('Q\n');child.stdin.flush();assert child.wait(timeout=10)==0
    record={'case':name,'actor':actor,'command':job['command'],'worker_sha256':sha(job['command'][0]),'sample_sha256':sha(folder/'sample.txt'),'verified_calls':rows,'load_after':os.getloadavg()};(folder/'result.json').write_text(json.dumps(record,indent=2)+'\n');print('Captured',name,actor,flush=True)
   finally:
    if child.poll() is None:child.terminate();child.wait(timeout=10)
# Reuse only the pure sample classifier; do not execute its old campaign analysis.
script=P/'evidence/history-and-profiles-20261119/analyze.py';tree=ast.parse(script.read_text());function=[x for x in tree.body if isinstance(x,ast.FunctionDef) and x.name=='parse'];assert len(function)==1;scope={'re':re,'Counter':Counter,'payload':lambda p:Path(p).read_bytes()};exec(compile(ast.Module(body=function,type_ignores=[]),str(script),'exec'),scope);parse=scope['parse'];ratios=read(P/'evidence/encoder-size-gap/current-matcher-guard-facts/runtime-case-ratios.json');summary={'plan':plan,'profile_classifier_sha256':sha(script),'profiles':{}}
for name in cases:
 summary['profiles'][name]={}
 for actor in actors:
  folder=O/name/actor;row=read(folder/'result.json');stats=parse(folder/'sample.txt',actor);assert stats['encoder_stack_samples']>=1000,(name,actor,stats);summary['profiles'][name][actor]=stats|{'worker_sha256':row['worker_sha256'],'sample_sha256':row['sample_sha256']}
 summary['profiles'][name]['qualified_runtime']=[r for r in ratios if r['case']==name]
(O/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
for name,row in summary['profiles'].items():print(name,{k:round(v['percent'],2) for k,v in row['mech']['buckets'].items()},flush=True)
