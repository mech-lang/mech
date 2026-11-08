from pathlib import Path
import gzip,hashlib,json,shutil
P=Path(__file__).parent;rows=[]
roots=[Path('/private/tmp/mech-related-borrowed-dictionary-values-runners-20261108-v1'),Path('/private/tmp/mech-related-borrowed-dictionary-values-runners-20261108-v2'),Path('/private/tmp/mech-related-borrowed-dictionary-values-emission-control-20261108')]
for root in roots:
 for p in [root/'compiler',root/'histogram-reuse/runner-aot']:
  assert json.loads((root/'histogram-reuse/summary.json'if 'runners' in root.name else root/'summary.json').read_text())['exact_outputs']==32
  st=p.stat();b=p.read_bytes();h=hashlib.sha256(b).hexdigest();q=p.with_suffix('.gz')
  with q.open('wb')as f:
   with gzip.GzipFile(filename='',mode='wb',fileobj=f,mtime=0,compresslevel=6)as z:z.write(b)
  with gzip.open(q,'rb')as z:assert hashlib.sha256(z.read()).hexdigest()==h
  rows.append(dict(original_path=str(p),archive_path=str(q),original_bytes=st.st_size,compressed_bytes=q.stat().st_size,sha256=h,mode=st.st_mode,lossless_readback_verified=True,unmeasured_superseded_candidate=True,all32_qualification_outputs_exact=True));p.unlink()
(P/'completed-superseded-runner-archives.json').write_text(json.dumps(rows,indent=2)+'\n');print('archived',len(rows),'freeGiB',shutil.disk_usage(P).free/2**30)
