from pathlib import Path
import gzip,hashlib,json,os,shutil,subprocess,tarfile
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
A=D/'native-fixed-frame-arenas-20261106';G=D/'raw-output-copies-20261105';A.mkdir(exist_ok=True)
read=lambda p:json.loads(Path(p).read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def write(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
def cp(p,r):
 q=A/r;q.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(p,q)
def gz(p,r):
 q=A/r;q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(gzip.compress(p.read_bytes(),compresslevel=6,mtime=0));assert hashlib.sha256(gzip.decompress(q.read_bytes())).hexdigest()==sha(p)
report=read(D/'compiler-native-fixed-frame-arenas-candidate-20261106.json')
S=Path('/private/tmp/mech-related-native-fixed-frame-arenas-compiler-20261106')
assert report['compiler']==subprocess.check_output(['git','rev-parse','HEAD'],cwd=S,text=True).strip()
for p in D.glob('*native-fixed-frame-arenas*.json'):cp(p,p.name)
cp(D/'compiler-native-fixed-frame-arenas.patch','compiler-native-fixed-frame-arenas.patch')
for p in R.glob('*.py'):cp(p,Path('tools')/p.name)
for f in ['measurement.rs','experiments/check_subset3.py']:cp(R/f,Path('tools')/f)
for p in (R/'fixtures').iterdir():
 if p.is_file():cp(p,Path('fixtures')/p.name)
folders=[(U,'runners')]
for suffix,label in [('opt','compiler'),('tests','tests'),('regressions','controls'),('runtime-check','runtime-managed'),('old-object','old-objects')]:folders.append((Path('/private/tmp')/f'mech-related-native-fixed-frame-arenas-{suffix}-20261106-v2',label))
for suffix,version,label in [('post-json-pc',1,'post-json-pc-profile'),('post-json-allocation',2,'post-json-allocation-profile'),('complete-phase-diagnostic',1,'complete-phase-diagnostic')]:
 p=Path('/private/tmp')/f'mech-related-native-fixed-frame-arenas-{suffix}-20261106-v{version}'
 if suffix!='complete-phase-diagnostic':p=Path('/private/tmp')/f'mech-related-native-fixed-frame-arenas-{suffix}-profile-20261106-v{version}'
 folders.append((p,label))
for version in [1]:
 for suffix in ['opt','tests','runners','regressions']:
  p=Path('/private/tmp')/f'mech-related-native-fixed-frame-arenas-{suffix}-20261106-v{version}'
  if p.exists():folders.append((p,f'excluded-prototypes/v{version}/{suffix}'))
for folder,label in folders:
 for directory,dirs,files in os.walk(folder,followlinks=False):
  directory=Path(directory);dirs[:]=[d for d in dirs if d not in ['target','latency','confirmation-latency','resources','__pycache__'] and not (directory/d).is_symlink() and d!='build' and not d.endswith('-bundle')]
  for name in files:
   p=directory/name
   if p.is_symlink() or p.stat().st_size>=25_000_000 or p.suffix not in ['.json','.rs','.py','.txt','.log','.mec','.bin','.asm','.c','.toml','.lock','.tsv','.patch','.gz','.o']:continue
   if name.startswith(('actual','related_posts_','full-output')) or name=='output.json' or name.endswith('-output.json'):continue
   cp(p,Path(label)/p.relative_to(folder))
for phase in ['latency','confirmation-latency','resources']:cp(U/phase/'run.json',Path('runners')/phase/'run.json')
for kind in ['after','compact']:
 for w in [4,8]:
  p=U/f'{kind}-{w}'
  for bundle,label in [(p/'build',f'{kind}-{w}'),(U/f'{kind}-{w}-decoded-bundle',f'{kind}-{w}-decoded')]:
   files=list(bundle.iterdir());assert len(files)==13
   for f in files:cp(f,Path('bundles')/label/f.name)
  gz(p/'runner-aot',Path('executables')/f'{kind}-{w}.gz')
p=Path('/private/tmp/mech-related-native-fixed-frame-arenas-post-json-pc-profile-20261106-v1')
for label in ['unchanged','compact']:
 gz(p/label/'runner-pc',Path('executables')/f'post-json-pc-{label}.gz');gz(p/label/'output.json',Path('post-json-pc-profile')/label/'output.json.gz')
p=Path('/private/tmp/mech-related-native-fixed-frame-arenas-post-json-allocation-profile-20261106-v2/after-4')
gz(p/'runner-profile','executables/post-json-allocation.gz');gz(p/'allocations.jsonl','post-json-allocation-profile/after-4/allocations.jsonl.gz');gz(p/'related_posts_profile.json','post-json-allocation-profile/after-4/related_posts_profile.json.gz')
p=Path('/private/tmp/mech-related-native-fixed-frame-arenas-complete-phase-diagnostic-20261106-v1')
for label in ['mech-parent-4','mech-after-4','mech-u8-ids-u32-8']:
 gz(p/label/'runner-phases',Path('executables')/f'phase-{label}.gz');gz(p/label/'output.json',Path('complete-phase-diagnostic')/label/'output.json.gz')
cp(D/'processing-allocation-gap-20261106.json','processing-allocation-gap-20261106.json')
for w in ['four','eight']:
 for f in [f'related-tiered-subsets-mask-eligibility-{w}-workers.mec',f'related-u8-ids-u32-combinadic-{w}-workers.mec']:cp(R/'experiments'/f,Path('sources')/f)
cp(R/'experiments/runner-input-string-ownership.rs','sources/runner-input-string-ownership.rs')
app=Path('/private/tmp/mech-related-native-fixed-frame-arenas-runtime-check-20261106-v2/managed-app')
for f,h in read(app/'linked-manifest.json')['pins'].items():assert sha(app/f)==h;cp(app/f,Path('runtime-managed/managed-app')/f)
for folder,label in [('mech-related-u32-rank-rust-control-20261106-v1','u32'),('mech-related-u8-ids-u32-rank-rust-control-20261106-v1','u8-ids')]:
 p=Path('/private/tmp')/folder
 for f in ['Cargo.toml','Cargo.lock','src/main.rs','build.json','checks.json']:cp(p/f,Path('rust-controls')/label/f)
 gz(p/'target/release/rust_mech',Path('executables')/f'rust-{label}.gz')
cp(Path('/private/tmp/mech-related-post-rust-local-20261102/posts.json'),'input/posts.json');gz(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json'),'expected/related_posts_rust.json.gz')
cp(S/'tests/architecture/bytecode-v1/scalar-add-f64.mecb','required-fixtures/scalar-add-f64.mecb')
cp(D/'native-frozen-type-descriptors-20261106/archive-index.json','references/parent-IR222-archive-index.json')
with tarfile.open(A/'changed-sources.tar.gz','w:gz') as tar:
 for f in subprocess.check_output(['git','diff','--name-only',report['base'],report['compiler']],cwd=S,text=True).splitlines():tar.add(S/f,arcname=f)
rows=[json.loads(l) for l in (G/'manifest.jsonl').read_text().splitlines()];assert len({x['path'] for x in rows})==len(rows)
selected=[x for x in rows if '/mech-related-native-fixed-frame-arenas-runners-20261106-v2/' in x['path']];assert len(selected)==1473
(A/'raw-output-copies.jsonl').write_text(''.join(json.dumps(x)+'\n' for x in selected));cp(G/'native-fixed-frame-arenas-20261106-compaction.log','raw-output-compaction.log')
write(A/'raw-output-restoration.json',dict(copies=len(selected),shared_archive=str(G),restore_tool=str(G/'restore.py'),restore_tool_sha256=sha(G/'restore.py'),manifest='raw-output-copies.jsonl',all_original_output_hashes_verified=True))
for blob,h in {x['blob']:x['sha256'] for x in rows}.items():assert hashlib.sha256(gzip.decompress((G/blob).read_bytes())).hexdigest()==h
idx=read(G/'archive-index.json');raw=sum(x['bytes'] for x in rows);idx.update(output_copies=len(rows),copies=len(rows),preserved_output_copies=len(rows),raw_bytes=raw,preserved_raw_output_bytes=raw,raw_bytes_compacted=raw,unique_output_blobs=len({x['blob'] for x in rows}));idx['files']=[dict(path=str(p.relative_to(G)),bytes=p.stat().st_size,sha256=sha(p)) for p in sorted(G.rglob('*')) if p.is_file() and p.name!='archive-index.json'];write(G/'archive-index.json',idx)
for f in ['progress.json','compiler-work.json','next-native-gains-plan-20261105.json','plan-native-lowering-20261105.json','compiler-branches-20261105.json']:cp(D/f,Path('checkpoint')/f)
cp(R/'README.md','checkpoint/README.md')
files=[dict(path=str(p.relative_to(A)),bytes=p.stat().st_size,sha256=sha(p)) for p in sorted(A.rglob('*')) if p.is_file() and p.name!='archive-index.json'];assert all(sha(A/x['path'])==x['sha256'] for x in files)
write(A/'archive-index.json',dict(schema='mech.related-post-gen.evidence-archive.v1',compiler=report['compiler'],base=report['base'],selected=False,selected_speed_compiler=report['selected_speed_compiler'],exact_patch_tree_verified=True,all_file_sha256_verified=True,files=files))
print(json.dumps(dict(archive_files=len(files),all_file_sha256_verified=True,preserved_output_copies=len(selected),total_preserved_output_copies=len(rows))))
