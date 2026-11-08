from pathlib import Path
import gzip,hashlib,json,subprocess,shutil
P=Path(__file__).parent;current=json.loads(Path('/private/tmp/mech-related-borrowed-dictionary-values-opt-20261108-v5/build.json').read_text());needed={s['command'][-1]for s in current['stages']}
for s in current['stages']:
 needed.update(a.split('=',1)[1]for a in s['command']if '='in a and '.rlib'in a)
rows=[]
def digest(p,compressed=False):
 h=hashlib.sha256()
 with (gzip.open(p,'rb')if compressed else p.open('rb'))as f:
  while b:=f.read(8<<20):h.update(b)
 return h.hexdigest()
for root in sorted(Path('/private/tmp').glob('mech-related-numeric-invariant-arguments-dev-20261104-v*')):
 p=root/'libmech_engine.rlib'
 if not p.is_file():continue
 assert str(p)not in needed
 snapshot=subprocess.check_output(['ps','-axo','command'],text=True);assert str(root)not in snapshot
 receipt=json.loads((root/'mech_engine.build.json').read_text());assert receipt['exit']==0
 st=p.stat();h=digest(p);assert h==receipt['library_sha256'];q=p.with_suffix('.rlib.gz');assert not q.exists()
 with p.open('rb')as f,q.open('wb')as out:
  with gzip.GzipFile(filename='',mode='wb',fileobj=out,mtime=0,compresslevel=6)as z:shutil.copyfileobj(f,z,8<<20)
 assert digest(q,True)==h
 rows.append(dict(original_path=str(p),archive_path=str(q),original_bytes=st.st_size,compressed_bytes=q.stat().st_size,sha256=h,mode=st.st_mode,lossless_readback_verified=True,not_referenced_by_current_production_closure=True,no_active_command_references=True,receipt=str(root/'mech_engine.build.json'),restore='gzip -dk archive; restore recorded mode'))
 p.unlink();(P/'completed-old-dev-Engine-library-archives.json').write_text(json.dumps(rows,indent=2)+'\n');print('archived',root.name,'freeGiB',round(shutil.disk_usage(P).free/2**30,3),flush=True)
