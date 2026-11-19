from pathlib import Path
from hashlib import sha256
from runpy import run_path
import json,os,subprocess
ROOT=Path(__file__).resolve().parent;L=ROOT/'immutable-only';I=ROOT.parent;Q=I.parent;read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();base=read(I/'natural4k-runtime-target/hot-matcher-facts/guards-profiled/receipt.json');build=read(L/'compiler-build.json');flags=base['flags'];m=run_path(str(Q/'reachability-census.py'),run_name='immutable_default_screen');results={}
for role,extra,profile in [('disabled-control',{},True),('immutable-defaults',{'MECH_COMPACT_IMMUTABLE_DEFAULTED_READS':'mech_compact_numeric_475_vector'},False),('immutable-defaults-checked',{'MECH_COMPACT_IMMUTABLE_DEFAULTED_READS':'mech_compact_numeric_475_vector','MECH_COMPACT_PAIR_REGISTER_SPILLS':'checked'},False)]:
 out=L/role;out.mkdir();front=out/'frontend';env=flags|extra
 if not profile:env['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(item for item in env['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';') if not item.startswith('mech_compact_numeric_475_vector@'))
 command=[build['compiler'],base['compile'][1],str(front),'compact']
 with (out/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|env,stdout=log,stderr=log,check=True)
 image,sections,symbols=m['load'](front/'program.o');meta=read(front/'metadata.json');phase,=[r for p in meta['numeric_phases'] for r in [p.get('vector_variant',{})] if r.get('symbol')=='mech_compact_numeric_475_vector'];record={'source_base':'fb6bbe11695f05f7b071280e6f8dcc5648b198bf','compiler_sha256':sha(command[0]),'compile':command,'flags':env,'text_bytes':sections[m['TEXT']][2],'root_bytes':meta['code_bytes'],'collector':phase,'artifact_sha256':{n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']},'scope':'Untimed source-emitted preflight. Changed matcher has no stale block profile; no runtime or complete-build claim.'};(out/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');results[role]=record
 if role=='disabled-control':assert record['artifact_sha256']==base['artifacts']
 else:assert record['artifact_sha256']['program.mecb']==base['bytecode_sha256']
 print(role,record['text_bytes'],phase['code_bytes'],flush=True)
assert results['immutable-defaults']['artifact_sha256']==results['immutable-defaults-checked']['artifact_sha256'];(L/'immutable-screen.json').write_text(json.dumps(results,indent=2)+'\n')
