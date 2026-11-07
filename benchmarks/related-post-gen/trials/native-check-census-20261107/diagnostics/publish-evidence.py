from pathlib import Path
import datetime,gzip,hashlib,json,shutil,subprocess
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-native-check-census-compiler-20261107');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';O=Path('/private/tmp/mech-related-native-check-census-opt-20261107-v1');L=S/'benchmarks/related-post-gen/trials/native-check-census-20261107'
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(Path(p).read_text())
def write(p,j):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(j,indent=2)+'\n')
source=read(P/'source-freeze.json')['source_commit'];assert source=='eb5556fb58f760c347dc8465444d0e3fd8f3fc47'
summary=read(P/'summary.json');geometry=read(P/'geometry-Object/geometry.json')
report=dict(schema='mech.related-post-gen.native-executed-check-census-report.v1',updated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),source_commit=source,parent='0784205e43177e03dd07cb8e4d2bafbbbe912ffc',branch='codex/related-native-check-census-20261107',diagnostic_only=True,selected=False,goal_complete=False,default_backend_changed=False,LLVM_mandatory=False,LLVM_tools_used=False,summary=summary,geometry_facts=geometry,
interpretation='Executed selected forward-window loops in numeric tree1282 still retain109097 funding gates. Their Version.funding is always None; the existing captured-only child funding additionally requires a phase proof absent from this tree. Successful checks do not authorize deletion. Derive an activation bound for independently proved leaves, retain all actual per-header debits and source errors, and use original checked fallback on failed admission.',
ruled_out='All10 views in the main query already have a statically unit matrix axis. linear_address already emits direct flat addressing, so adding another generic contiguous-address fast path cannot explain the backend gap. Investigate whether these known shape identities reach SSA/cold snapshots and register liveness.',
qualification='Two fresh5000 outputs match Rust; all942 native counter rows repeat exactly, with209 executed rows. Two unchanged-parent JIT outputs also match. All other non-timing fields and parallel native work agree. Total reported work varies between fresh runs of BOTH binaries; exact observations retained and attribution pending, so no finite-budget equivalence claim. All13 emitted uninstrumented Object bundle files remain byte-exact under the custom diagnostic compiler.',
latest_speed_report=str(D/'compiler-native-positional-metadata-20261107.json'),selected_compiler='4cc266ae0d05c69432a4c15ffe99d5e586685647',timing_claim=False,next_actions=['Propagate admitted unit-axis shape identities into native SSA homes and cold reloads, then measure code/stack/register changes.','Derive bounded funding for forward numeric-tree leaves with exact existing debits, capacity invariants and finite fallback.','Recover remaining backend instruction-combining gains; keep LLVM optional.','Meet fastest eligible measured Rust at both continuous post-JSON and fresh launch-to-exit boundaries.'])
report_path=D/'compiler-native-executed-check-census-20261107.json';write(report_path,report)
def copy(f,rel):
 f=Path(f);b=f.read_bytes();rel=Path(rel)
 if f.suffix in ['.rs','.o','.bin','.mecb','.toml']or len(b)>16000:rel=Path(str(rel)+'.gz');b=gzip.compress(b,mtime=0)
 p=L/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b)
L.mkdir(parents=True,exist_ok=True);copy(report_path,'report.json')
for f in sorted(P.iterdir()):
 if f.is_file()and f.suffix in ['.json','.py','.rs']and f.name not in ['output.json','output-repeat.json','parent-output.json','parent-repeat-output.json']:copy(f,'diagnostics/'+f.name)
for name in ['build.json','source-inventory.json','mech_gpu.build.json']:copy(O/name,'compiler/'+name)
for f in sorted((P/'uninstrumented-Object').iterdir()):
 if f.is_file():copy(f,'Object/'+f.name)
copy(P/'geometry-Object/geometry.json','geometry.json')
copy('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','inputs/posts.json')
copy('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json','outputs/Rust.json')
for n in ['output.json','output-repeat.json','parent-output.json','parent-repeat-output.json']:
 b=(P/n).read_bytes();assert json.loads(b)==read('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
write(L/'output-observations.json',{n:dict(bytes=(P/n).stat().st_size,sha256=sha((P/n).read_bytes()),exact_Rust_JSON=True)for n in ['output.json','output-repeat.json','parent-output.json','parent-repeat-output.json']})
patch=subprocess.check_output(['git','-c','gc.auto=0','diff',report['parent'],source],cwd=S);(L/'source.patch.gz').write_bytes(gzip.compress(patch,mtime=0))
(L/'README.md').write_text('# Executed native check census\n\nDiagnostic custom build; selected IR242 and default Cranelift dependency remain unchanged. Native atomic counters execute only in explicitly instrumented JIT compilation. Normal Object emission is byte-exact for all13 bundle files. Counters retain process-lifetime storage and use per-compilation keys. No diagnostic timings are benchmark results. Funding components and the combined gate overlap. Total reported work varies in both unchanged-parent and diagnostic fresh processes; exact values are retained and this is not a finite-budget equivalence proof. Run python3 verify.py for hashes, repeated actual counts and qualification receipts. Recipes record local paths and require relocation for another machine.\n')
verify='''from pathlib import Path
import gzip,hashlib,json
P=Path(__file__).parent
def blob(name):
 p=P/name
 if not p.exists():p=Path(str(p)+'.gz')
 b=p.read_bytes();return gzip.decompress(b)if p.suffix=='.gz'else b
def read(name):return json.loads(blob(name))
m=read('manifest.json')
for name,info in m['files'].items():
 b=(P/name).read_bytes();assert len(b)==info['bytes']and hashlib.sha256(b).hexdigest()==info['sha256'],name
 b=gzip.decompress(b)if info['gzip']else b;assert len(b)==info['decoded_bytes']and hashlib.sha256(b).hexdigest()==info['decoded_sha256'],name
r=read('report.json');assert r['diagnostic_only']and not r['selected']and not r['goal_complete']and not r['timing_claim']and not r['LLVM_mandatory']
a=read('diagnostics/census.json');b=read('diagnostics/census-repeat.json');assert a==b
assert len(a['rows'])==942 and sum(x['attempts']>0 for x in a['rows'])==209
assert all(x['successes']<=x['attempts']and x['failures']==x['attempts']-x['successes']for x in a['rows'])
assert sum(x['attempts']for x in a['rows']if x['key']['kind']=='funded-body'and x['key']['owner']==1282 and x['key']['version'])==109097
obj=read('diagnostics/Object-equivalence.json');assert obj['all_bundle_files_exact']and len(obj['files'])==13
for name,pin in obj['files'].items():assert hashlib.sha256(blob('Object/'+name)).hexdigest()==pin
assert all(x['static_vector']for x in read('geometry.json')if x['header']==1282)
work=read('diagnostics/total-work-observations.json');assert work['attribution_pending']and not work['finite_budget_equivalence_claim']and len(set(work['parallel_numeric_work'].values()))==1
assert len(read('output-observations.json'))==4
print(json.dumps(dict(files=len(m['files']),native_counter_rows=942,repeated_counter_rows_exact=True,Object_files_exact=13,exact5000_outputs=4,timing_claim=False,selected=False)))
'''
(L/'verify.py').write_text(verify)
files={}
for f in sorted(L.rglob('*')):
 if not f.is_file()or f.name=='manifest.json':continue
 b=f.read_bytes();decoded=gzip.decompress(b)if f.suffix=='.gz'else b;files[str(f.relative_to(L))]=dict(bytes=len(b),sha256=sha(b),gzip=f.suffix=='.gz',decoded_bytes=len(decoded),decoded_sha256=sha(decoded))
write(L/'manifest.json',dict(schema='mech.related-post-gen.evidence-packet.v1',source_commit=source,files=files))
print('Packet',len(files),'records',sum(f['bytes']for f in files.values()),'bytes')
