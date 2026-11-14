from pathlib import Path
import gzip,hashlib,json,os,time
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results/native-child-entry-facts-integration-20261107';D.mkdir(exist_ok=True);objects=D/'objects';objects.mkdir(exist_ok=True);roots={}
for ir,prefix in [(240,'child-entry-facts-integration')]:
 for kind in ['opt','tests','runners','regressions','runtime-check','feature-check','old-objects']:
  roots[f'ir{ir}-{kind}']=Path('/private/tmp/mech-related-'+prefix+'-'+kind+'-20261107-v1')
roots.update(latency=Path('/private/tmp/mech-related-native-child-entry-facts-integration-latency-20261107-v1'),resources=Path('/private/tmp/mech-related-native-child-entry-facts-integration-resources-20261107-v1'))

files={};links={};verified=set();start=time.monotonic()
def save(source,name):
 raw=source.read_bytes();digest=hashlib.sha256(raw).hexdigest();blob=objects/(digest+'.gz')
 if not blob.exists():
  with blob.open('wb')as out:out.write(gzip.compress(raw,compresslevel=6,mtime=0));out.flush();os.fsync(out.fileno())
 if digest not in verified:assert hashlib.sha256(gzip.decompress(blob.read_bytes())).hexdigest()==digest;verified.add(digest)
 files[name]=dict(source=str(source),sha256=digest,bytes=len(raw),mode=source.stat().st_mode&0o777,blob=str(blob.relative_to(D)),compressed_bytes=blob.stat().st_size)
for label,root in roots.items():
 for p in sorted(root.rglob('*')):
  name=label+'/'+str(p.relative_to(root))
  if p.is_symlink():links[name]=str(p.readlink());continue
  if p.is_file():save(p,name)
 print(label,'losslessly archived',flush=True)
for p in [R/'results'/f for f in ['compiler-native-child-entry-facts-integration-20261107.json','comparison-native-child-entry-facts-integration-randomized-20261107-5000-m3.json','comparison-native-child-entry-facts-integration-resources-20261107-5000-m3.json','size-native-child-entry-facts-integration-20261107.json','compiler-native-24-hour-progress-20261107.json']]:save(p,'final/'+p.name)
for pattern in ['*schema*20261107.py','*child-entry*20261107.py']:
 for p in sorted(Path('/private/tmp').glob(pattern)):save(p,'recipes/'+p.name)

external=[]
for line in (R/'results/archived-superseded-executables-20261106/manifest.jsonl').read_text().splitlines():
 row=json.loads(line)
 if any('/mech-related-'+p+'-'in row.get('original_path','')for p in ['child-entry-facts-integration']):assert hashlib.sha256(gzip.decompress(Path(row['blob']).read_bytes())).hexdigest()==row['sha256'];external.append(row)
index=dict(schema='mech.related-post-gen.qualified-compiler-archive.v1',active_compiler='deebf08df690e37ddf763635d70fd25aaf0bedb4',active_identity='IR240/work115/ABI41/image9',root=str(D),files=files,symlinks=links,losslessly_retained_completed_test_executables=external,verified_files=len(files),unique_blobs=len(verified),raw_archives_remotely_uploaded=False,performance_cohort_run=True,seconds=time.monotonic()-start)
(D/'archive-index.json').write_text(json.dumps(index,indent=2)+'\n');print(json.dumps(dict(status='verified',files=len(files),unique_blobs=len(verified),external_test_executables=len(external))),flush=True)
