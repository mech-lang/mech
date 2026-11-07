from pathlib import Path
import gzip,hashlib,json
P=Path(__file__).parent
def blob(name):
 p=P/name
 if not p.exists():p=Path(str(p)+'.gz')
 b=p.read_bytes();return gzip.decompress(b)if p.suffix=='.gz'else b
def read(name):return json.loads(blob(name))
m=read('manifest.json')
for name,info in m['files'].items():
 b=(P/name).read_bytes();assert len(b)==info['bytes']and hashlib.sha256(b).hexdigest()==info['sha256'],name
 b=gzip.decompress(b)if info['gzip']else b;assert len(b)==info['decoded_bytes']and hashlib.sha256(b).hexdigest()==info['decoded_sha256'],name
r=read('report.json');assert r['diagnostic_only']and not r['selected']and not r['goal_complete']and not r['timing_claim']and not r['LLVM_mandatory']
a=read('diagnostics/census.json');b=read('diagnostics/census-repeat.json');assert a==b
assert len(a['rows'])==942 and sum(x['attempts']>0 for x in a['rows'])==209
assert all(x['successes']<=x['attempts']and x['failures']==x['attempts']-x['successes']for x in a['rows'])
assert sum(x['attempts']for x in a['rows']if x['key']['kind']=='funded-body'and x['key']['owner']==1282 and x['key']['version'])==109097
obj=read('diagnostics/Object-equivalence.json');assert obj['all_bundle_files_exact']and len(obj['files'])==13
for name,pin in obj['files'].items():assert hashlib.sha256(blob('Object/'+name)).hexdigest()==pin
assert all(x['static_vector']for x in read('geometry.json')if x['header']==1282)
work=read('diagnostics/total-work-observations.json');assert work['attribution_pending']and not work['finite_budget_equivalence_claim']and len(set(work['parallel_numeric_work'].values()))==1
assert len(read('output-observations.json'))==4
print(json.dumps(dict(files=len(m['files']),native_counter_rows=942,repeated_counter_rows_exact=True,Object_files_exact=13,exact5000_outputs=4,timing_claim=False,selected=False)))
