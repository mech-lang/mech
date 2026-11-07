from pathlib import Path
import gzip,hashlib,json,subprocess,importlib.util
S=Path('/private/tmp/mech-related-llvm-object-backend-compiler-20261107');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';Q=Path('/private/tmp/mech-related-ordered-rust-controls-20261107-v1');P=Path('/private/tmp/mech-related-current-post-json-all-thread-profile-20261107-v1');C=Path('/private/tmp/mech-related-ordered-rust-controls-cohort-20261107-v1');M=Path('/private/tmp/mech-related-ordered-rust-controls-metrics-20261107-v1');L=Path('/private/tmp/mech-related-llvm-native-backend-exploration-20261107-v1');O=Path('/private/tmp/mech-related-llvm-object-backend-opt-20261107-v1');U=Path('/private/tmp/mech-related-llvm-object-backend-runners-20261107-v1')
relative='benchmarks/related-post-gen/trials/ordered-rust-controls-and-profile-20261107';out=S/relative
sha=lambda b:hashlib.sha256(b).hexdigest();git=lambda *a:subprocess.check_output(['git','-c','gc.auto=0',*a],cwd=S,text=True).strip();source_head=git('rev-parse','HEAD');assert source_head=='5bb64a80ba91ed52364d54ad7e8cd040b0f9830e' and not git('status','--porcelain')
spec=importlib.util.spec_from_file_location('inv',R/'build-compiler.py');inv=importlib.util.module_from_spec(spec);spec.loader.exec_module(inv);inventory=json.loads((O/'source-inventory.json').read_text())['files'];assert inv.source_inventory(S)==inventory
files={'trial.json':D/'compiler-ordered-rust-controls-and-current-profile-20261107.json','latency.json.gz':D/'comparison-ordered-rust-controls-randomized-20261107-5000-m3.json','resources.json.gz':D/'comparison-ordered-rust-controls-resources-20261107-5000-m3.json','sizes.json.gz':D/'size-ordered-rust-controls-20261107.json'}
for name in ['prepare-build.py','qualify.py','builds.json','qualification.json','qualification-summary.json','qualification-first-attempt.json']:files['rust-controls/'+name+'.gz']=Q/name
for name in json.loads((Q/'builds.json').read_text()):
 for file in ['src/main.rs','Cargo.toml','Cargo.lock','build.json','test.json']:files['rust-controls/'+name+'/'+file+'.gz']=Q/name/file
for file in (Q/'qualification').rglob('run.json'):files['rust-controls/'+str(file.relative_to(Q))+'.gz']=file
for file in P.iterdir():
 if file.is_file()and file.name!='quiet-start-gate.json'and file.suffix in ['.json','.py','.c']:files['profile/'+file.name+'.gz']=file
for name in ['selected-ir242','candidate-ir264']:
 for file in ['runner.rs','build.json','native-link-audit.json','audit-run.json','run.json','summary.json','output.json.sampler.json']:files['profile/'+name+'/'+file+'.gz']=P/name/file
 for file in (P/name).glob('numeric-*.asm'):files['profile/'+name+'/'+file.name+'.gz']=file
for name in ['metrics-config.json','source-pins.json']:files['metrics/'+name+'.gz']=M/name
for folder in M.iterdir():
 if folder.is_dir()and folder.name.startswith('rust-ordered-'):
  for f in ['src/main.rs','build.json','Cargo.toml','Cargo.lock']:files['metrics/'+folder.name+'/'+f+'.gz']=folder/f
for folder,label in [(O,'backend/compiler'),(U,'backend/equivalence')]:
 for file in folder.rglob('*'):
  if file.is_file()and file.suffix in ['.json','.py']and'build'not in file.relative_to(folder).parts:files[label+'/'+str(file.relative_to(folder))+'.gz']=file
for name in ['probe.ll','probe-run.json','probe.o','remarks.yaml','definitions.json','machine-facts.json','facts-run.json','refusals.json','maintained-tests.json']:files['backend/LLVM-probe/'+name+'.gz']=L/name
for name in ['report','publish']:files['recipes/'+name+'.py']=Path('/private/tmp/mech-related-ordered-rust-controls-and-profile-'+name+'-20261107.py')
records={}
def put(name,raw,source=None,unredacted=None):
 b=gzip.compress(raw,mtime=0)if name.endswith('.gz')else raw;p=out/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b);record=dict(sha256=sha(b),bytes=len(b),decoded_sha256=sha(raw),decoded_bytes=len(raw))
 if source:record['retained_source']=str(source)
 if unredacted:record['unredacted_source_sha256']=sha(unredacted)
 records[name]=record
assert not out.exists();assert all(p.is_file()for p in files.values()),[str(p)for p in files.values()if not p.is_file()]
for name,p in files.items():put(name,p.read_bytes(),p)
for folder,label in [(C,'cohort'),(P,'profile')]:
 for p in folder.iterdir():
  if p.is_file()and(p.suffix=='.py'or p.suffix=='.json')and(label=='cohort'or p.name=='quiet-start-gate.json'):
   raw=p.read_bytes()
   if 'start-gate'in p.name:
    d=json.loads(raw);snapshot=d.pop('process_snapshot');d.update(process_snapshot_sha256=sha(snapshot.encode()),process_snapshot_retained_locally=str(p),process_snapshot_omission='Unrelated workstation arguments remain local; original quiet predicates unchanged.');put(label+'/'+p.name,(json.dumps(d,indent=2)+'\n').encode(),p,raw)
   else:put(label+'/'+p.name,raw,p)
raw_registers={}
for name in ['selected-ir242','candidate-ir264']:
 for file in ['output.json.samples.bin','output.json.samples.bin.symbols.tsv']:
  p=P/name/file;raw_registers[name+'/'+file]=dict(sha256=sha(p.read_bytes()),bytes=p.stat().st_size,retained_locally=str(p),reason='Raw register/address observations remain local; phase/run-state validation and analyzed exact-native PC offsets are published.')
put('profile/local-register-records.json',(json.dumps(raw_registers,indent=2)+'\n').encode())
put('README.md',b'''# Stronger Rust controls, current native profile and LLVM backend work

Selected IR242/four native workers is unchanged. IR264 remains rejected on its retained21/6 processing/whole-process paired result; it is not repeated in this new timing cohort.

Five Rust variants test bitmask eligibility alone, ordered combinadic keys, retained masks, their combination and small binomial tables. Runtime eligibility, generic sorting for unordered unique rows, all coverage/CSR fallbacks, checked arithmetic, allocations/indexing/scoring/materialization stay inside the post-JSON timer. No unsafe operation is added.20 unit tests include subset permutations and independent old-algorithm comparisons. All six controls (old reference plus five new) match31 edge cases and the full5000-post input:192 outputs. The first copy omitted executable mode; no new binary ran before correcting that. Passed old-reference results remain and binary hashes are unchanged.

All341 randomized latency outputs (31 rounds, seed990270) and84 separate resource outputs (seven rounds, seed9902707) match Rust. All first/adverse samples remain. Builds/tests/profiles/compression are idle during measurement. Fastest processing is the ordered/mask/table variant at1.526792ms. Fastest complete process is bitmask scanning at22.852083ms. The old U8/U32 reference takes1.601250ms and23.717334ms. Unchanged selected Mech remeasures2.715875ms/1.778811x and34.380250ms/1.504469x. This is not a new Mech compiler gain. Processing instructions are55,012,810 versus17,876,348 (3.077x); allocation requests3758 versus58. Every resource/profile/qualification timing is excluded from latency claims. References are fastest eligible measured controls, not a global optimum.

Fresh all-thread register observations are gated to one unchanged odd post-JSON epoch. Clone, verification, serialization and teardown are outside; restore and original JSON parsing occur once. Both exact linked native Objects remain unchanged.1500 evaluations each match every index and first/last full JSON. Selected records4633 observations,2423 sampled running;1723 are in emitted native code. IR264 records4036,2101 running;1614 native. These are statistical observations, not CPU-time shares or retired instructions. Running-state filtering distinguishes waiting threads; disappearing endpoints and unknown state remain explicit. No user thread is suspended. Root/native preprocessing and query functions are prominent; allowance/Boolean normalization and stack traffic are frequent observed assembly locations. Raw register/address records stay local with hashes; analyzed offsets and source/commands are published.

The LLVM branch has started implementation, not produced or qualified an LLVM program. Object metadata/identity finalization is separate from machine definition; its default path emits all13 files byte-for-byte identically for both benchmark sources, including full Object and descriptor. LLVM's real stack reports include0/0/544-byte probe frames. A generic compile-time machine-facts reader validates a closed definition set against actual Mach-O text/constants and stack reports;7 maintained tests cover real output and malformed metadata. No source/layout authority is granted. Correct scalar/vector lowering, a distinct backend/toolchain/target fingerprint, actual machine admission, genuine Object assembly, finite/error/ownership/loader/scalar qualification and both performance boundaries remain required. The old diagnostic's descriptor cannot be reused under LLVM code.

Compiler/ABI/work versions remain unchanged in this foundation. Embedded backend scripts must enter the frozen build inventory before being embedded or invoked by compilation. LLVM tools do not enter executable runtime paths. Native scalar Bool/U64/I64/F64 remains maintained; none is retired or interpreted. Whole-process descriptor restoration remains a separate performance gap.

verify.py checks every stored/decoded hash,341/84 exact outputs,192 correctness outputs, source/unit pins, raw Rust paired results, profile gates and default finalizer equivalence. Evidence changes no Rust source/build manifest and preserves the frozen compiler inventory. Source code is under codex/related-llvm-object-backend-20261107; no selection, merge or Rust parity is claimed.
''')
verify='''from pathlib import Path
import gzip,hashlib,json,statistics
P=Path(__file__).resolve().parent
m=json.loads((P/'manifest.json').read_text())
for name,v in m['files'].items():
 b=(P/name).read_bytes();assert len(b)==v['bytes']and hashlib.sha256(b).hexdigest()==v['sha256'],name
 raw=gzip.decompress(b)if name.endswith('.gz')else b
 assert len(raw)==v['decoded_bytes']and hashlib.sha256(raw).hexdigest()==v['decoded_sha256'],name
read=lambda n:json.loads(gzip.decompress((P/n).read_bytes())if n.endswith('.gz')else(P/n).read_bytes())
t=read('trial.json');a=read('latency.json.gz');b=read('resources.json.gz');assert len(a['samples'])==341 and len(b['samples'])==84 and all(x['exit']==0 and x['exact_rust_output']for x in a['samples']+b['samples'])
rows={(x['sample'],x['variant']):x for x in a['samples']};assert len(rows)==341
for name,v in t['trials'].items():
 for base,pairs in v['paired_comparisons'].items():
  for field,pair in pairs.items():
   ds=[x[field]-rows[(x['sample'],base)][field]for x in a['samples']if x['variant']==name];assert len(ds)==31 and sum(x<0 for x in ds)==pair['wins']and abs(statistics.median(ds)-pair['median_paired_delta_seconds'])<1e-12
q=read('rust-controls/qualification.json.gz');assert len(q)==192 and all(x['exact_original_Rust_output']and x['exit']==0 for x in q)
for name,v in t['Rust_builds'].items():
 raw=gzip.decompress((P/('rust-controls/'+name+'/src/main.rs.gz')).read_bytes());assert hashlib.sha256(raw).hexdigest()==v['source_sha256'];test=read('rust-controls/'+name+'/test.json.gz');assert test['exit']==0 and '4 passed; 0 failed'in test['stdout']
for name,r in t['fresh_profile']['rows'].items():assert r['correct_evaluations']==1500 and r['native_Object_text_unchanged']and r['all_records_in_one_active_post_JSON_epoch']and r['sampler']['full']==0 and r['sampler']['suspended']==0
assert all(x['all13_bundle_files_byte_exact']for x in t['compiler_backend_work']['Object_finalizer_equivalence'].values())
assert not t['selected_changed']and not t['compiler_backend_work']['LLVM_program_qualified']
print(json.dumps(dict(files=len(m['files']),latency_exact=341,resources_exact=84,Rust_correctness_outputs=192,profile_evaluations=3000,paired_results_verified=True,selected_changed=False)))
'''
put('verify.py',verify.encode());(out/'manifest.json').write_text(json.dumps(dict(schema='mech.related-post-gen.trial-evidence-manifest.v3',source_commit=source_head,LLVM_program_qualified=False,selected=False,files=records),indent=2)+'\n')
subprocess.run(['python3',str(out/'verify.py')],check=True);assert inv.source_inventory(S)==inventory;git('add','--',relative);git('diff','--cached','--check');assert all(n.startswith(relative+'/')for n in git('diff','--cached','--name-only').splitlines());git('commit','-m','Record stronger Rust controls and exact-window native profiles')
receipt=dict(source_commit=source_head,evidence_commit=git('rev-parse','HEAD'),packet=relative,files=len(records),hashes_verified=True,paired_results_verified=True,source_inventory_exact=True,pushed=False,selected_changed=False,LLVM_program_qualified=False);Path('/private/tmp/mech-related-ordered-rust-controls-and-profile-publication-20261107.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))
