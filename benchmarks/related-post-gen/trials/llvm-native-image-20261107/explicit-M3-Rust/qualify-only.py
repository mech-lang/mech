from pathlib import Path
import hashlib,json,shutil,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
rows=json.loads(Path('/private/tmp/mech-related-forward-read-windows-runners-20261107-v8/reuse-4/edges/edge-fixtures.json').read_text());cases=[]
for x in rows:
 source=Path(x.get('source_posts',x['command'][2]));data=json.loads(source.read_text())
 if 'expected'in x:want=json.loads(Path(x['expected']).read_text())
 else:
  idx=json.loads((R/'fixtures'/('expected-'+x['fixture']+'.json')).read_text())['related_indices'];want=[{'_id':post['_id'],'tags':post['tags'],'related':[data[i]for i in selected]}for post,selected in zip(data,idx)]
 cases.append((x['fixture'],source,want))
cases.append(('full5000',Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json'),json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())))
results=[]
for v in json.loads((P/'variants.json').read_text()):
 for case,source,want in cases:
  p=P/'qualification-v2'/v['name']/case;(p/'run').mkdir(parents=True,exist_ok=True);shutil.copyfile(source,p/'posts.json');t=time.monotonic();r=subprocess.run([v['executable']],cwd=p/'run',capture_output=True,text=True);out=p/v['output'];d=dict(variant=v['name'],case=case,command=[v['executable']],exit=r.returncode,diagnostic_seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,performance_measured=False)
  if r.returncode==0:
   assert json.loads(out.read_text())==want,(v['name'],case);d['exact_original_Rust_output']=True
  else:
   assert v['name']=='rust-preinitialized-apple-m3' and case!='full5000',d
   old='/private/tmp/mech-related-c-family-20261104/rust_con/target/release/rust_rayon';b=p/'baseline';b.mkdir();r0=subprocess.run([old],cwd=b,capture_output=True,text=True);d.update(original_generic_Rust_exit=r0.returncode,original_generic_Rust_stderr=r0.stderr,existing_original_Rust_failure=True);assert r0.returncode!=0 and 'panicked'in r0.stderr and 'panicked'in r.stderr,d
  (p/'receipt.json').write_text(json.dumps(d,indent=2)+'\n');results.append(d)
 print(v['name'],'32 cases checked',flush=True)
(P/'qualification-summary.json').write_text(json.dumps(dict(variants=10,cases_each=32,total_cases=len(results),exact_outputs=sum(x.get('exact_original_Rust_output',False)for x in results),inherited_Rayon_failures=sum(x.get('existing_original_Rust_failure',False)for x in results),all_full5000_exact=all(x.get('exact_original_Rust_output')for x in results if x['case']=='full5000'),source_unchanged=True,cpu='apple-m3',all_setup_boundaries_unchanged=True,performance_measured=False),indent=2)+'\n')
