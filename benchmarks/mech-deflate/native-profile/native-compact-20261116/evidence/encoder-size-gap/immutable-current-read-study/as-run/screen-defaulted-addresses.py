from pathlib import Path
from hashlib import sha256
from runpy import run_path
import json,os,subprocess
L=Path(__file__).resolve().parent;I=L.parent;Q=I.parent;read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();prior=read(I/'natural4k-runtime-target/hot-matcher-facts/guards-profiled/receipt.json');build=read(I/'sealed-base-execution/compiler-build.json');flags=prior['flags'].copy();flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(item for item in flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';') if not item.startswith('mech_compact_numeric_475_vector@'));m=run_path(str(Q/'reachability-census.py'),run_name='defaulted_address_screen');results={}
for role,extra in [('control',{}),('safe-addresses',{'MECH_COMPACT_SELECT_PHASE_DEFAULTED_LOADS':'mech_compact_numeric_475_vector'})]:
 out=L/role;out.mkdir();front=out/'frontend';env=flags|extra;command=[build['compiler'],prior['compile'][1],str(front),'compact']
 with (out/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|env,stdout=log,stderr=log,check=True)
 image,sections,symbols=m['load'](front/'program.o');meta=read(front/'metadata.json');phase,=[r for p in meta['numeric_phases'] for r in [p.get('vector_variant',{})] if r.get('symbol')=='mech_compact_numeric_475_vector'];record={'compiler_sha256':sha(command[0]),'compile':command,'flags':env,'text_bytes':sections[m['TEXT']][2],'root_bytes':meta['code_bytes'],'collector':phase,'artifact_sha256':{n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']},'scope':'Untimed source-emitted preflight. Both matcher bodies unprofiled; no runtime qualification or complete-build claim.'};(out/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');results[role]=record;print(role,record['text_bytes'],phase['code_bytes'],flush=True)
(L/'defaulted-address-screen.json').write_text(json.dumps(results,indent=2)+'\n')
