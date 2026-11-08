from pathlib import Path
import gzip,hashlib,json,stat
p=Path(__file__).parent
items=[p/'native-tests-v3/gpu-native-tests',Path('/private/tmp/mech-related-generic-native-dictionary-runners-20261108-v1/compiler'),Path('/private/tmp/mech-related-generic-native-dictionary-emission-control-20261108/compiler')]
rows=[]
for f in items:
 z=f.with_name(f.name+'.qualified.gz');assert f.exists()and not z.exists();n=f.stat().st_size;mode=stat.S_IMODE(f.stat().st_mode);h=hashlib.sha256()
 with f.open('rb')as r,gzip.open(z,'wb',compresslevel=6)as w:
  while b:=r.read(1024*1024):h.update(b);w.write(b)
 v=hashlib.sha256()
 with gzip.open(z,'rb')as r:
  while b:=r.read(1024*1024):v.update(b)
 assert h.digest()==v.digest()
 rows.append(dict(path=str(f),archive=str(z),sha256=h.hexdigest(),bytes=n,archive_bytes=z.stat().st_size,mode=mode,losslessly_verified=True,scope='Completed qualification/compiler tool; all actual measurement runners remain raw',known_expanded_collection_parent_failures=5 if f.name=='gpu-native-tests'else None));(p/'qualified-binary-archives.json').write_text(json.dumps(dict(items=rows,measurement_runners_untouched=True),indent=2)+'\n');f.unlink();print(f.name,'archived losslessly',flush=True)
