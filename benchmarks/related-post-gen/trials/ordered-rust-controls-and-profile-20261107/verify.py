from pathlib import Path
import gzip,hashlib,json,statistics
P=Path(__file__).resolve().parent
m=json.loads((P/'manifest.json').read_text())
for name,v in m['files'].items():
 b=(P/name).read_bytes();assert len(b)==v['bytes']and hashlib.sha256(b).hexdigest()==v['sha256'],name
 raw=gzip.decompress(b)if name.endswith('.gz')else b
 assert len(raw)==v['decoded_bytes']and hashlib.sha256(raw).hexdigest()==v['decoded_sha256'],name
read=lambda n:json.loads(gzip.decompress((P/n).read_bytes())if n.endswith('.gz')else(P/n).read_bytes())
t=read('trial.json');a=read('latency.json.gz');b=read('resources.json.gz');assert len(a['samples'])==341 and len(b['samples'])==84 and all(x['exit']==0 and x['exact_rust_output']for x in a['samples']+b['samples'])
rows={(x['sample'],x['variant']):x for x in a['samples']};assert len(rows)==341
for name,v in t['trials'].items():
 for base,pairs in v['paired_comparisons'].items():
  for field,pair in pairs.items():
   ds=[x[field]-rows[(x['sample'],base)][field]for x in a['samples']if x['variant']==name];assert len(ds)==31 and sum(x<0 for x in ds)==pair['wins']and abs(statistics.median(ds)-pair['median_paired_delta_seconds'])<1e-12
q=read('rust-controls/qualification.json.gz');assert len(q)==192 and all(x['exact_original_Rust_output']and x['exit']==0 for x in q)
for name,v in t['Rust_builds'].items():
 raw=gzip.decompress((P/('rust-controls/'+name+'/src/main.rs.gz')).read_bytes());assert hashlib.sha256(raw).hexdigest()==v['source_sha256'];test=read('rust-controls/'+name+'/test.json.gz');assert test['exit']==0 and '4 passed; 0 failed'in test['stdout']
for name,r in t['fresh_profile']['rows'].items():assert r['correct_evaluations']==1500 and r['native_Object_text_unchanged']and r['all_records_in_one_active_post_JSON_epoch']and r['sampler']['full']==0 and r['sampler']['suspended']==0
assert all(x['all13_bundle_files_byte_exact']for x in t['compiler_backend_work']['Object_finalizer_equivalence'].values())
assert not t['selected_changed']and not t['compiler_backend_work']['LLVM_program_qualified']
print(json.dumps(dict(files=len(m['files']),latency_exact=341,resources_exact=84,Rust_correctness_outputs=192,profile_evaluations=3000,paired_results_verified=True,selected_changed=False)))
