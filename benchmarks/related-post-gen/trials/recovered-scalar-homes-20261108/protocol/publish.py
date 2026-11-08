from pathlib import Path
import json,gzip,hashlib,os,subprocess,shutil
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-recovered-scalar-homes-compiler-20261108');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results';L=S/'benchmarks/related-post-gen/trials/recovered-scalar-homes-20261108';L.mkdir(parents=True,exist_ok=True);N=Path('/private/tmp/mech-related-recovered-scalar-homes-runners-20261108-v1')
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
copy(D/'compiler-native-recovered-scalar-homes-20261108.json','compiler-native-recovered-scalar-homes-20261108.json')
for root,prefix in [(P,'protocol'),('/private/tmp/mech-related-recovered-scalar-homes-tests-20261108-v1','tests'),('/private/tmp/mech-related-recovered-scalar-homes-regressions-20261108-v1','scalar-DEFLATE'),('/private/tmp/mech-related-recovered-scalar-homes-metrics-20261108-v1','metrics'),('/private/tmp/mech-related-recovered-scalar-homes-opt-20261108-v1','compiler-build')]:tree(root,prefix)
copy(D/'optimization-recovery-audit-20261108.json','optimization-recovery-audit-20261108.json')
tree(N,'runners');tree(N/'runtime','runtime-build')
old={'plain':Path('/private/tmp/mech-related-grouped-metadata-runners-20261108-v1/plain')}
for label in old:
 for tag,root in [(label,N/label),('IR278-'+label,old[label])]:
  for f in sorted((root/'build').iterdir()):
   if f.is_file():copy(f,'Objects/'+tag+'/'+f.name)
  if tag.startswith('IR278-'):
   audit=next(f for f in root.glob('*native*json')if read(f).get('schema')=='mech.related-post-gen.native-link-audit.v9');copy(audit,'Objects/'+tag+'/native-link-proof.json')
seen=set()
def blob(p,kind='outputs'):
 p=Path(p);b=p.read_bytes();k=sha(b);token=(kind,k)
 if token not in seen:copy(p,kind+'/'+k+'.json');seen.add(token)
 return k
copy('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','inputs/posts.json');copy('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json','outputs/oracle.json')
for kind in ['latency','resources']:
 name='comparison-native-recovered-scalar-homes'+('-resources'if kind=='resources'else'')+'-20261108-5000-m3.json';c=read(D/name);copy(D/name,'cohorts/'+name)
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
  root=P/'extra-edges'/case;variant='mech-ir279-scalar-home-masks-4';add(label,case,root/'posts.json',root/'related_posts_rust.json',root/variant/'output.json')
assert len(refs)==35;(L/'qualification').mkdir(exist_ok=True);(L/'qualification/refs.json').write_text(json.dumps(refs,indent=2)+'\n')
for name in ['compare-implementations.py','build-compiler.py','build-dependent-compiler.py','build-host.py','build-metrics.py','measurement.rs','audit-native-link.py','experiments/check_subset3.py']:copy(R/name,'recipes/'+name)
patch=subprocess.check_output(['git','-c','gc.auto=0','diff','7e2b75c5b0317f610af2a83e46dc0a4deccb67c6','3ad8bb9210d322124eb5df913f5652c10336cb4c'],cwd=S);(L/'source.patch.gz').write_bytes(gzip.compress(patch,mtime=0))
(L/'README.md').write_text('# Recovered scalar-home masks: IR279\n\nSource 3ad8bb9210d322124eb5df913f5652c10336cb4c recovers the missing IR241\ncompiler optimization on IR278. Compiler-local SSA masks skip physical clearing\nof homes already cleared since entry or the last resumable helper. Every helper\nconservatively resets all bits. No runtime mask allocation, weaker admission,\naccounting change, source algorithm change or scalar-entry retirement.\nCranelift stays production; LLVM remains an optional comparison.\n\nFresh unchanged-parent processing instructions fall 51,147,313 -> 49,520,546\n(-1,626,767 / -3.18055%). Processing medians are 2.912833 -> 2.881333 ms;\n20/31 paired wins, paired change -0.84666%, 95% [-1.38028%, +0.24786%].\nElapsed-time superiority remains inconclusive. Fastest eligible measured Rust\nis 1.631083 ms; IR279 is 1.766515x versus IR278 1.785828x in this cohort.\n\nWhole-process medians 37.220791 -> 37.352500 ms; paired change -0.92980%,\n95% [-3.08132%, +2.00840%]. Fastest independent whole Rust is 24.964875 ms;\nIR279 is 1.496202x. Restore medians are 7.736709 -> 7.733959 ms. All samples\nare retained, including adverse ones. Whole-process timing is reported without\na mandatory acceptance gate. No unchanged favorable retry.\n\nRetained for composition as a verified instruction-saving compiler candidate;\nIR278 remains the working latency control. The frozen processing target remains\nfalse. This does not claim a timing promotion or Rust parity. Do not sum\nhistorical instruction gains across different parents.\n\nAllocation requests/requested bytes/peak additional heap remain equal in both\nphases. Post-JSON 2945 / 7919355 B / 5566127 B; restore 26837 / 12830999 B /\n7003853 B. Native code 886344 -> 889768 B, cumulative stack 1536 -> 1552 B,\n51 functions, metadata 4335172 B unchanged. All memory/cycle metrics are in\nthe reports. RSS/footprint observations are not a complete causal attribution.\n\n899 latency + 210 separate resource outputs match Rust; 35 qualification outputs,\n408 actual native CI names, 3 focused mask/finite ablations, 8 row/route tests,\n27 image/caller tests, 4 explicitly executed optional LLVM checks, and 133 exact\nscalar/DEFLATE non-timing/allocation rows pass. First harness count mismatch\n(expected407, actual408) is retained; no production failure or timing discarded.\nNo full hosted-CI-green claim. No build/test/profile overlapped the cohorts.\n\nThe recovery audit covers 120 reports and 18 prioritized entries. Paired wide\nclears and conditional compact table storage remain composition hypotheses;\ntheir unfavorable standalone processing results remain recorded. Newest PR973\ngeometry/address staging is separate work not present in this frozen compiler.\n\nRun python3 verify.py to independently verify records, outputs, actual test names,\nbootstrap calculations, fastest Rust, resource medians and native Object pins.\n')
(L/'verify.py').write_bytes((P/'packet-verify-template.py').read_bytes());m={}
for f in sorted(L.rglob('*')):
 if f.is_file()and f.name not in ['manifest.json','verification-receipt.json']:
  b=f.read_bytes();g=f.suffix=='.gz';x=gzip.decompress(b)if g else b;m[str(f.relative_to(L))]=dict(bytes=len(b),sha256=sha(b),gzip=g,decoded_bytes=len(x),decoded_sha256=sha(x))
(L/'manifest.json').write_text(json.dumps(dict(files=m),indent=2)+'\n');print(json.dumps(dict(records=len(m),bytes=sum(v['bytes']for v in m.values()))),flush=True)
