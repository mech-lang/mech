from pathlib import Path
import json,gzip,hashlib,os,subprocess,shutil
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-matrix-length-lowering-compiler-20261107');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';L=S/'benchmarks/related-post-gen/trials/packed-row-payload-20261107';L.mkdir(parents=True,exist_ok=True);N=Path('/private/tmp/mech-related-packed-row-payload-runners-20261107-v2')
sha=lambda b:hashlib.sha256(b).hexdigest()
def read(p):return json.loads(Path(p).read_text())
def copy(f,n):
 b=Path(f).read_bytes()
 if Path(f).suffix in ['.rs','.o','.bin','.mecb','.toml','.lock','.log']or Path(f).name=='Cargo.lock'or len(b)>16000:
  raw=L/n
  if raw.exists():raw.unlink()
  n+='.gz';b=gzip.compress(b,mtime=0)
 q=L/n;q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(b)
def tree(root,prefix):
 root=Path(root)
 for path,dirs,files in os.walk(root):
  dirs[:]=sorted(n for n in dirs if n not in ['target','.git','build','edges','extra-edges','latency','resources','output-objects','__pycache__','runtime'])
  for n in sorted(files):
   f=Path(path)/n
   if f.suffix in ['.rs','.py','.json','.jsonl','.lock','.toml','.log','.txt']and not n.startswith('output'):copy(f,prefix+'/'+str(f.relative_to(root)))
copy(D/'compiler-native-packed-row-payload-20261107.json','compiler-native-packed-row-payload-20261107.json')
for root,prefix in [(P,'protocol'),('/private/tmp/mech-related-packed-row-payload-tests-20261107-v1','first-tests'),('/private/tmp/mech-related-packed-row-payload-tests-20261107-v2','first-tests-v2'),('/private/tmp/mech-related-packed-row-payload-tests-20261107-v3','first-tests-v3'),('/private/tmp/mech-related-packed-row-payload-tests-20261107-v4','first-tests-v4'),('/private/tmp/mech-related-packed-row-payload-tests-20261107-v5','tests'),('/private/tmp/mech-related-packed-row-payload-runners-20261107-v1','first-runners'),('/private/tmp/mech-related-packed-row-payload-regressions-20261107-v1','first-regressions'),('/private/tmp/mech-related-packed-row-payload-regressions-20261107-v2','scalar-DEFLATE'),('/private/tmp/mech-related-packed-row-payload-metrics-20261107-v1','metrics'),('/private/tmp/mech-related-packed-row-payload-opt-20261107-v3','compiler-build')]:tree(root,prefix)
tree(N,'runners');tree(N/'runtime','runtime-build')
old={'plain':Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4')}
for label in old:
 for tag,root in [(label,N/label),('IR264-'+label,old[label])]:
  for f in sorted((root/'build').iterdir()):
   if f.is_file():copy(f,'Objects/'+tag+'/'+f.name)
  if tag.startswith('IR264-'):
   audit=next(f for f in root.glob('*native*json')if read(f).get('schema')=='mech.related-post-gen.native-link-audit.v9');copy(audit,'Objects/'+tag+'/native-link-proof.json')
seen=set()
def blob(p,kind='outputs'):
 p=Path(p);b=p.read_bytes();k=sha(b);token=(kind,k)
 if token not in seen:copy(p,kind+'/'+k+'.json');seen.add(token)
 return k
copy('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','inputs/posts.json');copy('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json','outputs/oracle.json')
for kind in ['latency','resources']:
 name='comparison-native-packed-row-payload'+('-resources'if kind=='resources'else'')+'-20261107-5000-m3.json';c=read(D/name);copy(D/name,'cohorts/'+name)
 for row in c['samples']:
  k=row['output_sha256'];files=list((P/kind/'output-objects').glob(k+'*'));assert len(files)==1 and blob(files[0])==k
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
for label in old:
 root=N/label;add(label,'full5000','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json',root/'output.json')
 for row in read(root/'edges/edge-fixtures.json'):
  case=row['fixture'];actual=Path(row['command'][3]);inp=Path(row['command'][2]);indices=None
  if 'expected'in row:expected=Path(row['expected'])
  else:
   indices=R/'fixtures'/('expected-'+case+'.json');posts=read(inp);rows=read(indices)['related_indices'];expected=P/('derived-expected-'+case+'.json');expected.write_text(json.dumps([{'_id':p['_id'],'tags':p['tags'],'related':[posts[i]for i in ids]}for p,ids in zip(posts,rows)])+'\n')
  add(label,case,inp,expected,actual,indices)
 for case in ['sorted64','unordered64','fallback65']:
  root=P/'extra-edges'/case;variant='mech-ir277-packed-row-4';add(label,case,root/'posts.json',root/'related_posts_rust.json',root/variant/'output.json')
assert len(refs)==35;(L/'qualification').mkdir(exist_ok=True);(L/'qualification/refs.json').write_text(json.dumps(refs,indent=2)+'\n')
for name in ['compare-implementations.py','build-compiler.py','build-dependent-compiler.py','build-host.py','build-metrics.py','measurement.rs','audit-native-link.py','experiments/check_subset3.py']:copy(R/name,'recipes/'+name)
patch=subprocess.check_output(['git','-c','gc.auto=0','diff','3f9fa5a8c34bb39cfebbcfd7199786263b5a2a4b','80c91ec89ec0b57ddc1820a57fec4063c920de06'],cwd=S);(L/'source.patch.gz').write_bytes(gzip.compress(patch,mtime=0))
r=read(D/'compiler-native-packed-row-payload-20261107.json');a=r['analysis'];c=a['medians']['mech-ir277-packed-row-4'];selected=a['medians']['mech-ir242-reuse-4']
(L/'README.md').write_text(f"""# IR277: checked packed row payload sizing, unselected

Fixed-row transport uses checked count*width for canonical packed scalar banks
instead of reading every element to total its payload. Text/generic cells retain
the scan. Every original shape, account, alias, capacity, output and work check
remains; the exact original work prices are retained. No source algorithm change.
This composes with IR276 matrix length, uses Cranelift0.131.3 and adds no dependency.
All51 native arithmetic functions and raw Object text are exactly parentIR276/264.

Source80c91ec89ec0b57ddc1820a57fec4063c920de06 passes405 actual CI names,8 focused
original/decoded/restored/generic-scan ablation checks,25 image/caller checks,
4 explicit optionalLLVM checks,133 exact scalar/DEFLATE rows and35 actual related
outputs. Test fixture mistakes and disk-full postprocessing failures are retained
with corrections. No complete GPU/hostedCI claim; no scalar entry retired.

All837 latency and196 resource outputs match Rust with no exclusions or overlapping
builds/tests/profiles/archives. All22Rust controls remain. Preparation, allocation,
worker creation/run/join and materialization stay in continuous postJSON; restoration,
JSONIO and teardown stay in launch-to-exit. References are selected independently.

Processing is{c['post_json_seconds']*1000:.6f}ms/{c['times_fastest_processing_Rust']:.6f}x
fastestRust. Selected242 is{selected['post_json_seconds']*1000:.6f}ms/
{selected['times_fastest_processing_Rust']:.6f}x in this cohort. Processing beats all
three required controls; whole-process noninferiority passes against immediate276,
but fails against264 and selected242. No unchanged confirmation or promotion.

Run python3 verify.py to rederive1033 cohort comparisons,35 qualification outputs,
bootstrap intervals, fastestRust at both boundaries, actual test names and Object
text. Resources retain allocations, requested/peak heap, instructions/cycles, RSS,
restore metrics and binary/code/stack. Rust parity remains unmet and the goal active.
""")
(L/'verify.py').write_bytes((P/'packet-verify-template.py').read_bytes());m={}
for f in sorted(L.rglob('*')):
 if f.is_file()and f.name not in ['manifest.json','verification-receipt.json']:
  b=f.read_bytes();g=f.suffix=='.gz';x=gzip.decompress(b)if g else b;m[str(f.relative_to(L))]=dict(bytes=len(b),sha256=sha(b),gzip=g,decoded_bytes=len(x),decoded_sha256=sha(x))
(L/'manifest.json').write_text(json.dumps(dict(files=m),indent=2)+'\n');print(json.dumps(dict(records=len(m),bytes=sum(v['bytes']for v in m.values()))),flush=True)
