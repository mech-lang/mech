from pathlib import Path
import gzip,hashlib,json,time,stat
p=Path(__file__).parent
names=['compact-native-fact-rows-20261105-v1','native-adaptive-histogram-schema-reuse-20261105-v1','native-adaptive-histogram-schema-reuse-20261105-v2','native-frozen-type-descriptors-20261106-v2','native-immutable-view-homes-20261106-v2','native-vector-fuel-debits-20261106-v1','native-vector-ledger-20261106-v1','native-projected-debits-20261106-v4','native-arithmetic-ranges-20261106-v1','native-constant-conditions-20261106-v1','native-early-identities-20261106-v1']
rows=[]
for name in names:
 f=Path('/private/tmp')/('mech-related-'+name.replace('-2026','-regressions-2026',1))/'scalar'
 if not f.is_file():continue
 z=f.with_name('scalar.qualified.gz');assert not z.exists(),z
 n=f.stat().st_size;mode=stat.S_IMODE(f.stat().st_mode);h=hashlib.sha256()
 with f.open('rb')as a,gzip.open(z,'wb',compresslevel=6)as b:
  while chunk:=a.read(1024*1024):h.update(chunk);b.write(chunk)
 v=hashlib.sha256()
 with gzip.open(z,'rb')as a:
  while chunk:=a.read(1024*1024):v.update(chunk)
 assert h.digest()==v.digest()
 row=dict(original=str(f),archive=str(z),original_bytes=n,archive_bytes=z.stat().st_size,mode=mode,sha256=h.hexdigest(),losslessly_verified=True,scope='Completed scalar/DEFLATE functional-regression executable; primary latency/resource runners untouched')
 (f.parent/'scalar-binary-archive.json').write_text(json.dumps(row,indent=2)+'\n');f.unlink();rows.append(row)
 (p/'completed-regression-archives.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,'saved',n-z.stat().st_size,flush=True)
