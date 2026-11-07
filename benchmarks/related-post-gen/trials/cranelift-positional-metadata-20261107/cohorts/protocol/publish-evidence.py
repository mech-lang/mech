from pathlib import Path
import datetime,gzip,hashlib,importlib.util,json,os,shutil,subprocess,statistics
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
S=Path('/private/tmp/mech-related-positional-metadata-compiler-20261107')
P=Path(__file__).parent
L=S/'benchmarks/related-post-gen/trials/cranelift-positional-metadata-20261107'
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(Path(p).read_text())
def write(p,d):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(d,indent=2)+'\n')
def head(s):return subprocess.check_output(['git','-c','gc.auto=0','rev-parse','HEAD'],cwd=s,text=True).strip()
source=head(S);assert source=='ed66537a2a519ddc14f162dbc06a3cff7bcc5208'
latency=read(D/'comparison-native-positional-metadata-20261107-5000-m3.json')
resources=read(D/'comparison-native-positional-metadata-resources-20261107-5000-m3.json')
decision=read(P/'paired-analysis.json')
N=Path('/private/tmp/mech-related-positional-metadata-runners-20261107-v1')
Q3=Path('/private/tmp/mech-related-restore-instruction-phases-20261107-v3')
names=['mech-ir242-reuse-4','mech-ir264-cranelift-runtime-control','mech-llvm-reuse-4','mech-ir266-reuse-4']
report=dict(schema='mech.related-post-gen.rejected-Cranelift-positional-metadata.v1',updated_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),source_commit=source,parent='0784205e43177e03dd07cb8e4d2bafbbbe912ffc',branch='codex/related-positional-metadata-20261107',identity='IR266/work115/ABI41/image9',selected=False,goal_complete=False,default_backend_changed=False,LLVM_required_for_Cranelift=False,LLVM_tools_used_for_Cranelift_build_or_execution=False,optional_LLVM_regression_tools_explicitly_executed=True,
change='Private Rust-declared positional struct/enum consumers check the entire original field envelope, then invoke unchanged eager typed value readers. Reordered/partial/non-opted readers keep map decoding. No image-format or native-code change.',
qualification=dict(CI=read('/private/tmp/mech-related-positional-metadata-tests-20261107-v1/summary.json'),engine=read('/private/tmp/mech-related-positional-metadata-engine-tests-20261107-v1/summary.json'),engine_new_positional_tests=3,GPU_image_tests=24,benchmark=read(N/'summary.json'),loaders=read('/private/tmp/mech-related-positional-metadata-loader-tests-20261107-v1/summary.json'),all133_scalar_DEFLATE_nontiming_fields_exact=all(read(Path('/private/tmp/mech-related-positional-metadata-regressions-20261107-v1')/(k+'-comparison.json'))['all_nontiming_fields_equal_IR240']for k in ['scalar','one-kib']),optional_LLVM_4_native_tests_pass=True,full_GPU_suite_not_run_for_unselected_trial=True),
cohort=dict(samples=len(latency['samples']),all_outputs_exact=all(r['exact_rust_output']for r in latency['samples']),post_JSON_Rust_reference=latency['matched_native_reference'],whole_process_Rust_reference=latency['whole_process_reference'],rows={n:latency['summary'][n]for n in names}),resources={n+'-instrumented':resources['summary'][n+'-instrumented']for n in names},prospective_rule=read(P/'prospective-rule.json'),decision=decision,
source_freeze=read(P/'source-freeze.json'),normalized_native_text_exact_parent=True,normalized_native_text_sha256='a89fd61ff00cf98854fdc474ff3f57dbc0b2ac61412caffaca3db2f6dd66c3da',native_code_bytes=886344,native_stack_bytes=1536,
conclusion='Rejected:125.883M to127.105M restoration instructions (+0.97%);26,837 allocations/12,830,999B unchanged. Whole-process paired median+2.823% and12/31wins versus matched parent. No source or native processing gain. Do not rerun unchanged.',
first_and_adverse_results_retained=True,first_corrections=['An initial loader budget harness compared Cranelift1536B actual stack to LLVM1568B. Corrected to executed unchanged-parent Cranelift budget runners, then24 entire denial objects exact; no relaxed field comparisons.','Initial analysis looked for a separate preparation measurement; source confirms the single measurement begins before admission and already covers continuous post-JSON. Corrected analysis uses that whole measurement. No cohort overwritten.'],
metadata_field_profile=dict(source_commit=head(Path('/private/tmp/mech-related-restore-instruction-phases-compiler-20261107')),summary=read(Q3/'summary.json'),labels=read(Q3/'field-labels.json'),diagnostic_only=True,instrumented_fields=31,unseparated_field='utf8_reads; regex did not include digit8, so it remains in the enclosing phase',hook_instructions_and_nesting_perturbation='Exclusive130.572M versus enclosing134.418M; not uninstrumented latency. Seven fresh outputs exact; restore allocations and first-evaluation allocations/instructions checked separately; no work deferred.'),
selected_compiler_unchanged='4cc266ae0d05c69432a4c15ffe99d5e586685647',same_binary_controls_are_not_compiler_gains=True,
next_actions=['Recover conditional-comparison/instruction-combining and register-pressure gains in existing Cranelift.','Measure compact typed metadata representations; eliminating named-field dispatch alone did not help. Preserve eager independent validation.','Complete executed proof/refusal/consumption and successful-funding-check census.','Retain fastest eligible measured Rust separately at both boundaries and qualify unordered unique rows.'])
report_path=D/'compiler-native-positional-metadata-20261107.json';write(report_path,report)
def put(root,relative,data):
    p=root/relative;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
def copy(root,path,relative=None):
    path=Path(path);relative=relative or path.name;data=path.read_bytes()
    if path.suffix in ['.rs','.ll','.o','.mecb','.bin','.toml'] or len(data)>16000:
        relative+='.gz';data=gzip.compress(data,mtime=0)
    put(root,relative,data)
def tree(root,path,prefix):
    path=Path(path)
    if not path.exists():return
    for parent,dirs,files in os.walk(path):
        dirs[:]=sorted(x for x in dirs if x not in ['target','cargo-target','.git','__pycache__','latency','resources','edges','output-objects','llvm-build'])
        for name in sorted(files):
            f=Path(parent)/name;rel=f.relative_to(path)
            if len(rel.parts)>3:continue
            if name.endswith(('.rs','.py','.mec','.toml')) or (f.suffix=='.json'and('output'not in name and not name.startswith(('actual-','full5000.json','full5000-success.json','posts.json')))):
                copy(root,f,prefix+'/'+str(rel))

L.mkdir(parents=True,exist_ok=True)
copy(L,report_path,'report.json')
write(L/'source-and-selection.json',dict(source_commit=source,parent=report['parent'],selected=False,selected_IR242_unchanged=True,LLVM_mandatory=False,goal_complete=False,input_sha256=latency['input']['sha256']))
copy(L,'/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','inputs/posts.json')
copy(L,'/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json','outputs/related_posts_rust.json')
for n in ['compare-implementations.py','build-compiler.py','build-dependent-compiler.py','build-host.py','build-metrics.py','measurement.rs','audit-native-link.py','experiments/check_subset3.py']:
 copy(L,R/n,'recipes/'+n)
for n in ['comparison-native-positional-metadata-20261107-5000-m3.json','comparison-native-positional-metadata-resources-20261107-5000-m3.json']:
 copy(L,D/n,'cohorts/'+n)
tree(L,P,'cohorts/protocol')
for label,path in [
 ('build','/private/tmp/mech-related-positional-metadata-opt-20261107-v1'),
 ('tests','/private/tmp/mech-related-positional-metadata-tests-20261107-v1'),
 ('engine-tests','/private/tmp/mech-related-positional-metadata-engine-tests-20261107-v1'),
 ('runners',N),
 ('metrics','/private/tmp/mech-related-positional-metadata-metrics-20261107-v1'),
 ('loaders','/private/tmp/mech-related-positional-metadata-loader-tests-20261107-v1'),
 ('regressions','/private/tmp/mech-related-positional-metadata-regressions-20261107-v1'),
 ('field-profile',Q3),
 ('field-profile-first-unchanged-attempt','/private/tmp/mech-related-restore-instruction-phases-opt-20261107-v3-first-unchanged-attempt'),
 ('field-profile-build','/private/tmp/mech-related-restore-instruction-phases-opt-20261107-v3')]:tree(L,Path(path),label)
for f in (N/'build').iterdir():
 if f.is_file():copy(L,f,'object/'+f.name)
# Preserve executable hashes/sizes and final same-Object audit; avoid shipping host binaries.
write(L/'binaries.json',{str(f):dict(sha256=sha(f.read_bytes()),bytes=f.stat().st_size)for f in [N/'runner-aot',Path('/private/tmp/mech-related-positional-metadata-metrics-20261107-v1/mech-ir266-reuse-4/runner-metrics')]})
put(L,'source.patch.gz',gzip.compress(subprocess.check_output(['git','-c','gc.auto=0','diff',report['parent'],source],cwd=S),mtime=0))
SR=Path('/private/tmp/mech-related-restore-instruction-phases-compiler-20261107')
put(L,'field-profile/source.patch.gz',gzip.compress(subprocess.check_output(['git','-c','gc.auto=0','diff','8b6074e1c2d536d33ebc19b585947b00c7b67304',head(SR)],cwd=SR),mtime=0))
(L/'README.md').write_text('# Rejected Cranelift positional metadata experiment\n\nThe selected IR242 compiler is unchanged. LLVM remains optional. This existing-backend change passes correctness, but increases restoration instructions and fails its prospectively declared whole-process gate. Both latency boundaries retain all20 Rust controls,744 exact outputs and175 exact separate resource outputs. Run python3 verify.py for integrity, arithmetic and qualification checks. Recipes retain original local paths/toolchain receipts; relocation is required for rebuilding elsewhere. Compressed Rust probes and manifests are evidence, not compiler source inputs. The field profile is diagnostic only;31 named fields are separated, with utf8_reads in the enclosing remainder. No work is deferred into first evaluation.\n')
# Add the verifier before final manifest creation.
copy(L,P/'verify-packet.py','verify.py')
files={}
for f in sorted(L.rglob('*')):
 if not f.is_file()or f.name=='manifest.json'or'__pycache__'in f.parts:continue
 b=f.read_bytes();decoded=gzip.decompress(b)if f.suffix=='.gz'else b
 files[str(f.relative_to(L))]=dict(bytes=len(b),sha256=sha(b),gzip=f.suffix=='.gz',decoded_bytes=len(decoded),decoded_sha256=sha(decoded))
write(L/'manifest.json',dict(schema='mech.related-post-gen.evidence-packet.v1',source_commit=source,selected=False,goal_complete=False,files=files))
print('published local packet',len(files),'records',sum(x['bytes']for x in files.values()),'bytes',flush=True)
