from pathlib import Path
import json,gzip,hashlib,os,subprocess,shutil
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-latest-deflate-compiler-20261108');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';L=S/'benchmarks/related-post-gen/trials/latest-deflate-specialized-rows-20261108';L.mkdir(parents=True,exist_ok=True);N=Path('/private/tmp/mech-related-latest-deflate-runners-20261108-v1')
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(Path(p).read_text())
def copy(f,n):
 b=Path(f).read_bytes()
 if Path(f).suffix in ['.rs','.o','.bin','.mecb','.toml','.lock','.log','.patch']or Path(f).name=='Cargo.lock'or len(b)>16000:
  raw=L/n
  if raw.exists():raw.unlink()
  n+='.gz';b=gzip.compress(b,mtime=0)
 q=L/n;q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(b)
def tree(root,prefix):
 root=Path(root)
 for path,dirs,files in os.walk(root):
  dirs[:]=sorted(n for n in dirs if n not in ['target','.git','build','edges','extra-edges','latency','resources','output-objects','__pycache__','runtime','source-preflight','witness-parent'])
  for n in sorted(files):
   f=Path(path)/n
   if f.suffix in ['.rs','.py','.json','.jsonl','.lock','.toml','.log','.txt']and(not n.startswith('output')or n.endswith('.intervals.json')):copy(f,prefix+'/'+str(f.relative_to(root)))
copy(D/'compiler-native-latest-deflate-integration-20261108.json','compiler-native-latest-deflate-integration-20261108.json')
for root,prefix in [(P,'protocol'),('/private/tmp/mech-related-specialized-rows-confirmation-20261108','confirmation'),('/private/tmp/mech-related-latest-deflate-tests-20261108-v2','tests'),('/private/tmp/mech-related-latest-deflate-regressions-20261108-v1','scalar-DEFLATE'),('/private/tmp/mech-related-latest-deflate-metrics-20261108-v1','metrics'),('/private/tmp/mech-related-latest-deflate-opt-20261108-v1','first-compiler-build'),('/private/tmp/mech-related-latest-deflate-opt-20261108-v2','compiler-build'),('/private/tmp/mech-related-specialized-rows-profile-20261108','diagnostics')]:tree(root,prefix)
for f in ['scalar-run.json','one-kib-run.json']:copy(Path('/private/tmp/mech-related-child-entry-facts-integration-regressions-20261107-v1')/f,'IR240-controls/'+f)
copy(D/'optimization-recovery-audit-20261108.json','optimization-recovery-audit-20261108.json')
copy('/private/tmp/mech-related-recovered-scalar-homes-tests-20261108-v1/ci-list.json','previous-native-CI-list.json')
copy('/private/tmp/mech-related-specialized-rows-profile-20261108/diagnostic-runtime.patch','diagnostics/diagnostic-runtime.patch')
tree(N,'runners');tree(N/'runtime','runtime-build')
old={'plain':Path('/private/tmp/mech-related-recovered-scalar-homes-runners-20261108-v1/plain')};labels=['plain','triple-rows','quartet-rows','both-rows']
for label in labels:
 for f in sorted((N/label/'build').iterdir()):
  if f.is_file():copy(f,'Objects/'+label+'/'+f.name)
for f in ['object.o','metadata.json']:copy(old['plain']/'build'/f,'Objects/IR279-plain/'+f)
copy(old['plain']/'native-link-proof.json','Objects/IR279-plain/native-link-proof.json')
seen=set()
def blob(p,kind='outputs'):
 p=Path(p);b=p.read_bytes();k=sha(b);token=(kind,k)
 if token not in seen:copy(p,kind+'/'+k+'.json');seen.add(token)
 return k
copy('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','inputs/posts.json');copy('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json','outputs/oracle.json')
for directory,prefix in [(P,'native-latest-deflate-integration'),(Path('/private/tmp/mech-related-specialized-rows-confirmation-20261108'),'native-specialized-rows-confirmation')]:
 for kind in ['latency','resources']:
  name='comparison-'+prefix+('-resources'if kind=='resources'else'')+'-20261108-5000-m3.json';c=read(D/name);copy(D/name,'cohorts/'+name)
  for row in c['samples']:
   k=row['output_sha256'];files=list((directory/kind/'output-objects').glob(k+'*'));assert len(files)==1 and blob(files[0])==k
for row in read('/private/tmp/mech-related-specialized-rows-profile-20261108/profiles.json'):assert blob(row['output_object'])==row['output_sha256']
# Retain every source/build pin for the22 unchanged Rust controls too.
for v in read(P/'latency-config.json')['implementations']:
 label=v['name'];src=Path(v['source']);assert src.exists();copy(src,'controls/'+label+'/'+src.name)
 if 'receipt'in v:copy(v['receipt'],'controls/'+label+'/build-receipt.json')
 else:
  q=L/'controls'/label/'source-and-executable-pins.json';q.write_text(json.dumps(v,indent=2)+'\n')
 if v['kind']!='mech':
  manifest=next((p/'Cargo.toml'for p in [src.parent,*src.parents]if (p/'Cargo.toml').exists()),None)
  if manifest:
   copy(manifest,'controls/'+label+'/Cargo.toml')
   lock=manifest.with_name('Cargo.lock')
   if lock.exists():copy(lock,'controls/'+label+'/Cargo.lock')
# Independently check and store actual inputs/oracles/outputs for all35 cases.
refs=[]
def add(label,case,inp,expected,actual,indices=None):
 assert read(expected)==read(actual);q=dict(variant=label,case=case,input_sha256=blob(inp,'inputs/qualification'),expected_sha256=blob(expected),actual_sha256=blob(actual))
 if indices:q['expected_indices_sha256']=blob(indices,'inputs/qualification-indices')
 refs.append(q)
for label in labels:
 root=N/label;add(label,'full5000','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json',root/'output.json')
 for row in read(root/'edges/edge-fixtures.json'):
  case=row['fixture'];actual=Path(row['command'][3]);inp=Path(row['command'][2]);indices=None
  if 'expected'in row:expected=Path(row['expected'])
  else:
   indices=R/'fixtures'/('expected-'+case+'.json');posts=read(inp);rows=read(indices)['related_indices'];expected=P/('derived-expected-'+case+'.json');expected.write_text(json.dumps([{'_id':p['_id'],'tags':p['tags'],'related':[posts[i]for i in ids]}for p,ids in zip(posts,rows)])+'\n')
  add(label,case,inp,expected,actual,indices)
 for case in ['sorted64','unordered64','fallback65']:
  root=P/'extra-edges'/case;variant='mech-ir280-latest-deflate-4'if label=='plain'else'mech-ir280-'+label+'-4';add(label,case,root/'posts.json',root/'related_posts_rust.json',root/variant/'output.json')
assert len(refs)==140;(L/'qualification').mkdir(exist_ok=True);(L/'qualification/refs.json').write_text(json.dumps(refs,indent=2)+'\n')
for name in ['compare-implementations.py','build-compiler.py','build-dependent-compiler.py','build-host.py','build-metrics.py','measurement.rs','audit-native-link.py','experiments/check_subset3.py']:copy(R/name,'recipes/'+name)
patch=subprocess.check_output(['git','-c','gc.auto=0','diff','0e1c6adb1565f0a1c3f55a38479a28af12b41efa','ccfadbdf0af69f741237f5d1099a3c3eef79386a'],cwd=S);(L/'source.patch.gz').write_bytes(gzip.compress(patch,mtime=0))
(L/'README.md').write_text("# IR280: current DEFLATE compiler and checked row specializations\n\nProduction compiler source ccfadbdf0af69f741237f5d1099a3c3eef79386a integrates latest\nPR973a7c2168 plus the previously unmerged nullary-state foundation, carrying\nits selection changes into the canonical phase proof.415actual CI names and\n7new upstream tests pass. Original arithmetic text remains exactIR279; compiler\nintegration alone has no established processing gain and its adverse whole\nresult is retained. Cranelift stays production; no mandatoryLLVM dependency.\n\nAll3source experiments specialize4/5tag rows, retain original other-width loops\nand preserve exact scoring/ties. The combined source97fc6c5a448aee096d19eb89839512974c724fee5efae5653ca88bed51b9c00a\nwins31/31processing pairs initially and independently. Confirmation2.358833ms/\n1.554335x fastest eligible measured Rust1.517583ms. Paired-9.528% vs matched\nIR280 control,95%[-10.115%,-8.948%]; instructions49.401M->40.770M(-17.471%).\nVersus previousIR278 working control, paired processing-10.773%, whole+4.424%,\n95%[+0.591%,+8.289%]. Adopted per user's phase-first policy with costs retained.\nRestoration7.239709->7.538334ms, processing requested memory+86880B, native\ncode+187712B and cumulative stack+592B. All code/metadata/heap/RSS/footprint\nmetrics and every first/adverse result remain. Rust parity remains unmet.\n\n2332exact cohort outputs,140qualification outputs,415actualnativeCI names,\n7new upstream/3mask/8row/27image/4explicitLLVM checks and133exact scalar/DEFLATE\nnon-timing/allocation rows qualify this working source/compiler. First count\nharness failure, missing nullary dependency witness and disk link failure are\npreserved, with final successful source-pinned qualification. No hostedCI-green\nclaim. No own build/test/profile/archive overlapped either performance cohort.\n\nFresh same-runtime diagnostics bind the same actual production Objects.48exact\noutputs/32balanced5thread hardware captures show main numeric21.060M->12.273M\ninstructions; worker numeric10.239M stays unchanged. Remaining22.512M numeric,\n8.839Mcallbacks and8.776Mother generated-entry instructions are diagnostic\nmedians, not wall-time shares. The profiler is not used for promotion timing.\n\nRun python3 verify.py for numerical and hash replay. Every copied Rust/manifest\nsource is compressed so source-inventory scans remain exact after publication.\n")
(L/'verify.py').write_bytes((P/'packet-verify-template.py').read_bytes());m={}
for f in sorted(L.rglob('*')):
 if f.is_file()and f.name not in ['manifest.json','verification-receipt.json']:
  b=f.read_bytes();g=f.suffix=='.gz';x=gzip.decompress(b)if g else b;m[str(f.relative_to(L))]=dict(bytes=len(b),sha256=sha(b),gzip=g,decoded_bytes=len(x),decoded_sha256=sha(x))
(L/'manifest.json').write_text(json.dumps(dict(files=m),indent=2)+'\n');print(json.dumps(dict(records=len(m),bytes=sum(v['bytes']for v in m.values()))),flush=True)
