from pathlib import Path
import hashlib,json,shutil,datetime
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-late-key-additions-compiler-20261108');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');Q=S/'benchmarks/related-post-gen/trials/late-key-additions-20261108'
read=lambda p:json.loads(Path(p).read_text())
def write(p,d):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(d,indent=2)+'\n')
seen={}
def copy(src,dst):
 st=src.stat()
 if src.suffix=='.json'and st.st_nlink>1 and st.st_size>500000:
  key=(st.st_dev,st.st_ino)
  if key not in seen:
   data=src.read_bytes();sha=hashlib.sha256(data).hexdigest();blob=Q/'evidence/output-objects'/(sha+'.json');blob.parent.mkdir(parents=True,exist_ok=True)
   if not blob.exists():blob.write_bytes(data)
   assert hashlib.sha256(blob.read_bytes()).hexdigest()==sha;seen[key]=(sha,blob)
  sha,blob=seen[key];write(Path(str(dst)+'.ref.json'),dict(schema='mech.lossless.output-alias.v1',original_path=str(src),alias_relative_to_packet=str(dst.relative_to(Q)),blob_relative_to_packet=str(blob.relative_to(Q)),output_sha256=sha,bytes=st.st_size));return
 dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(src,dst)
for f in P.rglob('*'):
 if not f.is_file()or f.is_symlink()or f.suffix not in ['.json','.py','.txt','.log','.patch','.rs']:continue
 rel=f.relative_to(P)
 if rel.parts[0].startswith('metrics')and'build'in rel.parts:continue
 if f.suffix=='.rs':rel=Path(str(rel)+'.txt')
 copy(f,Q/'evidence'/rel)
for label,prefix in [('default','/private/tmp/mech-related-late-key-additions-opt-20261108-v'),('control','/private/tmp/mech-related-late-key-additions-control-opt-20261108-v'),('diagnostic','/private/tmp/mech-related-late-key-additions-diagnostic-opt-20261108-v')]:
 for version in [1,2]:
  root=Path(prefix+str(version))
  if not root.is_dir():continue
  for f in root.iterdir():
   if f.suffix in ['.json','.log']:copy(f,Q/'builds'/label/f'v{version}'/f.name)
for label,root in [('default',Path('/private/tmp/mech-related-late-key-additions-runners-20261108-v1')),('emission-control',Path('/private/tmp/mech-related-late-key-additions-control-20261108-v1'))]:
 for rel in ['summary.json','recipe.json','one-line-source-ablation.json','gpu-build.json','build.json','compiler-build.json','object-build.json','runner-build.json','link-audit.json','full5000-run.json','edge-run.json','histogram-reuse/summary.json','histogram-reuse/object-build.json','histogram-reuse/runner-build.json','histogram-reuse/link-audit.json','histogram-reuse/full5000-run.json','histogram-reuse/edge-run.json','histogram-reuse/native-link-proof.json','histogram-reuse/runner-aot.build.json','runtime/build.json','runtime/runtime-build.json','runtime/source-inventory.json','histogram-reuse/build/metadata.json','histogram-reuse/build/descriptor.json','histogram-reuse/build/catalog-profile.json','histogram-reuse/build/frame-summary.json']:
  f=root/rel
  if f.is_file():copy(f,Q/'runners'/label/rel)
 for rel in ['histogram-reuse/runner.rs','histogram-reuse/runner-native-profile.rs','compiler-facts.rs','build-control.py','build-runner.py']:
  f=root/rel
  if f.is_file():copy(f,Q/'runners'/label/(rel+'.txt'if f.suffix=='.rs'else rel))
copy(Path('/private/tmp/mech-related-late-key-additions-runners-20261108-v1/histogram-reuse/build/source.mec'),Q/'program/source.mec')
for name in ['build-dependent-compiler.py','build-compiler.py','build-host.py','build-metrics.py','audit-native-link.py','compare-implementations.py','measurement.rs']:copy((P if name in ['build-dependent-compiler.py','build-compiler.py']else R)/name,Q/'tools'/(name+'.txt'if name.endswith('.rs')else name))
root=Path('/private/tmp/mech-related-late-key-additions-regressions-20261108-v2')
for name in ['scalar-run.json','one-kib-run.json','scalar-comparison.json','one-kib-comparison.json','scalar.build.json','one-kib.build.json']:copy(root/name,Q/'regressions'/name)
copy(P/'run-regression-controls.py',Q/'regressions/run.py')
copy(Path('/private/tmp/mech-related-late-key-additions-regressions-20261108-v1/scalar.build.json'),Q/'regressions/first-failed-harness/scalar.build.json')
for suffix in['']:
 for stem in['comparison-late-key-additions','comparison-late-key-additions-resources']:
  name=stem+suffix+'-20261108-5000-m3.json';copy(R/'results'/name,Q/'measurements'/name)

report=read(P/'report.json');write(Q/'report.json',report)
shutil.copyfile(P/'packet-readme.md',Q/'README.md')
files={str(f.relative_to(Q)):dict(bytes=f.stat().st_size,sha256=hashlib.sha256(f.read_bytes()).hexdigest())for f in Q.rglob('*')if f.is_file()and f.name!='manifest.json'}
write(Q/'manifest.json',dict(files=files,source_commit=report['source_commit'],goal_complete=False))
print('packet files',len(files),'bytes',sum(v['bytes']for v in files.values()))
