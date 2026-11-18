"""Fresh source-to-executable comparison with system and maintained allocator frontends at the same opt-level 1."""
from importlib import import_module
from pathlib import Path
from statistics import median
import hashlib,json,subprocess,sys
measure=import_module('compare-builds');q=measure.q
out=measure.work/'apple-m1-compiler-allocator-current';out.mkdir()
candidate=q/'cranelift-continuation-registers-word-pages-native/final-candidate'
measure.runtime=json.loads((candidate/'runtime.json').read_text());measure.reference_pins=json.loads((candidate/'receipt.json').read_text())
measure.emitter=q/'native-review-pr/baseline-build/compiler/release/emit_compact_size'
frontends={'mech-control':q/'compiler-allocator-profile/control','mech-compiler-allocator':q/'native-review-pr/baseline-build/compiler/release/native_compact_frontend'}
library,=(v.removeprefix('mech_gpu=') for v in measure.runtime['link'] if v.startswith('mech_gpu='))
measure.mech_link=[library if x.endswith('/libmech_gpu_runtime.rlib') else x for x in measure.mech_link]
actors=['mech-control','mech-compiler-allocator','roc','libdeflate']
def run(actor,directory):
 if actor in frontends:measure.frontend=frontends[actor]
 r=measure.measure('mech' if actor in frontends else actor,directory);r['actor']=actor
 if actor in frontends:
  profile=json.loads((directory/'frontend/compile.json').read_text())
  r['seconds']['parse']=next(p['seconds'] for p in profile['phases'] if p['phase']=='parse')
 (directory/'result.json').write_text(json.dumps(r,indent=2)+'\n')
 return r
warmup=[run(actor,out/'warmup'/actor) for actor in actors]
trials=[]
# Every actor occupies every position once, and each ordered pair is adjacent
# once. All output directories and compiler processes are fresh.
orders=((0,1,3,2),(1,2,0,3),(2,3,1,0),(3,0,2,1))
for i,order in enumerate(orders,1):
 for index in order:
  actor=actors[index];trials.append(run(actor,out/f'order-{i}'/actor))
for row in [*warmup,*trials]:
 if row['actor'] not in frontends:measure.verify_peer(Path(row['executable']))
new=out/'warmup/mech-compiler-allocator';old=out/'warmup/mech-control'
for file in ['frontend/program.mecb','native/program.o','native/metadata.json','compressor']:
 assert (new/file).read_bytes()==(old/file).read_bytes(),file
subprocess.run([sys.executable,'-B',str(q/'verify-worker.py'),str(new/'compressor'),str(new/'native/metadata.json'),'matched-compiler-allocator-current'],check=True)
sys.path.insert(0,str(q));macho=import_module('reachability-census');sizes={}
for actor in actors:
 d=out/'warmup'/actor;executable=d/'compressor'
 if actor=='roc':
  archive=d/'encoder.a';members=subprocess.check_output(['/usr/bin/ar','-t',str(archive)],text=True).splitlines();member,=(m for m in members if m.startswith('roc_app_'))
  encoder=d/'encoder-code.o';encoder.write_bytes(subprocess.check_output(['/usr/bin/ar','-p',str(archive),member]))
 else:encoder=d/('native/program.o' if actor in frontends else 'encoder.o')
 _,sections,_=macho.load(encoder);_,linked,_=macho.load(executable)
 sizes[actor]={'emitted_text_bytes':sections[macho.TEXT][2],'encoder_file_bytes':encoder.stat().st_size,'linked_text_bytes':linked[macho.TEXT][2],'executable_file_bytes':executable.stat().st_size,'encoder_sha256':measure.sha(encoder)}
medians={actor:{key:median(r['seconds'][key] for r in trials if r['actor']==actor) for key in next(r['seconds'] for r in trials if r['actor']==actor)} for actor in actors}
record={'machine':'Apple M1','scope':'Fresh encoder source through final executable link; compilers, runtimes, and adapters prebuilt','mech_compiler_build':'Rust opt-level 1 throughout, no PGO; frontend source differs only by importing the maintained compiler allocator; native emitter, runtime and adapters identical for both variants','source_sha256':measure.sha(measure.source_mech),'frontends':{k:{'path':str(v),'sha256':measure.sha(v)} for k,v in frontends.items()},'emitter_sha256':measure.sha(measure.emitter),'runtime_sha256':measure.sha(Path(library)),'roc_compiler_sha256':measure.sha(measure.roc),'roc_source_sha256':measure.sha(measure.roc_app),'c_source_sha256':measure.sha(measure.source_c/'lib/deflate_compress.c'),'warmup':warmup,'trials':trials,'medians':medians,'sizes':sizes,'byte_identical':['canonical bytecode','native object','complete native metadata','complete executable'],'verified_exact_calls':162,'retained_runtime_wins':{'roc':[81,81],'libdeflate':[61,61]},'runtime_score_scope':'Historical baseline evidence only. This compiler allocator change produces byte-identical encoder objects, metadata and linked executables to its control; no new peer runtime timing is claimed.'}
(out/'timing.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'medians':medians,'sizes':sizes,'byte_identical':record['byte_identical']},indent=2),flush=True)
