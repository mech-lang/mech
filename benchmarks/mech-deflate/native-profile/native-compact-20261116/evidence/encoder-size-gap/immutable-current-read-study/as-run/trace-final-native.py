"""Untimed detailed native-scope attribution; prove instrumentation preserves every artifact."""
from pathlib import Path
from hashlib import sha256
import json,os,subprocess
ROOT=Path(__file__).resolve().parent;L=ROOT/'placement';read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();O=L/'native-detail';O.mkdir();results={}
for role in ['disabled-control','immutable-profiled']:
 r=read(L/role/'receipt.json');folder=O/role;folder.mkdir();front=folder/'frontend';command=r['compile'].copy();command[2]=str(front)
 with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|r['flags']|{'MECH_PROFILE_COMPACT_DETAIL':'1'},stdout=log,stderr=log,check=True)
 artifacts={n:sha(front/n) for n in r['artifacts']};assert artifacts==r['artifacts'];scopes=[line.split() for line in (folder/'compile.log').read_text().splitlines() if line.startswith('compact-detail ')];assert len({x[1] for x in scopes})==len(scopes);results[role]={'command':command,'compiler_sha256':sha(command[0]),'flags':r['flags']|{'MECH_PROFILE_COMPACT_DETAIL':'1'},'artifacts':artifacts,'nanoseconds':{name:int(ns) for _,name,ns in scopes}}
(O/'result.json').write_text(json.dumps({'scope':'Instrumented diagnostic scope durations, one sample/actor. Nested/overlapping scopes do not add; these are not matched elapsed-time measurements or causal savings. Both artifacts reproduce exactly.','variants':results},indent=2)+'\n')
for name,row in results.items():print(name,sorted([(n,round(t/1e6,3)) for n,t in row['nanoseconds'].items()],key=lambda x:-x[1])[:16],flush=True)
