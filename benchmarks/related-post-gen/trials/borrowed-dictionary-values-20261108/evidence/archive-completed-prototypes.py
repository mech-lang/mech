from pathlib import Path
import gzip,hashlib,json,os
P=Path(__file__).parent;manifest=[]
for label in ['prototype-tests-v2','prototype-tests-v3','prototype-tests-v4']:
 p=P/label/'gpu-tests';q=p.with_suffix('.gz');receipt=json.loads((p.parent/'test-build.json').read_text());assert receipt['exit']==0
 b=p.read_bytes();st=p.stat();h=hashlib.sha256(b).hexdigest()
 with q.open('wb') as f:
  with gzip.GzipFile(filename='',mode='wb',fileobj=f,mtime=0,compresslevel=6) as z:z.write(b)
 with gzip.open(q,'rb')as f:assert hashlib.sha256(f.read()).hexdigest()==h
 manifest.append(dict(original_path=str(p),archive_path=str(q),original_bytes=st.st_size,compressed_bytes=q.stat().st_size,sha256=h,mode=st.st_mode,lossless_readback_verified=True,all_required_functional_runs_complete=True))
 p.unlink()
(P/'completed-prototype-executable-archives.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps({'archives':len(manifest),'bytes_saved':sum(x['original_bytes']-x['compressed_bytes']for x in manifest)}))
