from pathlib import Path
import gzip,hashlib,json,struct,statistics,random,math
P=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def blob(n):
 p=P/n
 if not p.exists():p=Path(str(p)+'.gz')
 b=p.read_bytes();return gzip.decompress(b)if p.suffix=='.gz'else b
def read(n):return json.loads(blob(n))
m=read('manifest.json')
for n,x in m['files'].items():
 b=(P/n).read_bytes();assert len(b)==x['bytes']and sha(b)==x['sha256'],n
 d=gzip.decompress(b)if x['gzip']else b;assert len(d)==x['decoded_bytes']and sha(d)==x['decoded_sha256'],n
r=read('report.json');assert not r['selected']and not r['goal_complete']and not r['new_production_dependency']
for x in r['analysis']['decisions'].values():assert not x['first_cohort_passed']and not x['selected']
for kind,v,count in [('paired-scalar-selects',3,401),('compact-visit-work',2,399)]:
 ci=read(f'{kind}/tests-v{v}/ci-execute.json');assert ci['exit']==0 and f'{count} passed; 0 failed'in ci['stdout']
 for n in ['scalar','one-kib']:assert read(f'{kind}/regressions/{n}-comparison.json')['all_nontiming_fields_equal_IR240']
 assert read(f'{kind}/tests-v{v}/optional-llvm-regressions.json')['exit']==0
assert read('loaders/summary.json')['finite_denials']==24
assert read('loaders/summary.json')['full5000_plus_edges_exact']==96
assert read('compact-visit-work/tests-v1/all-image-tests.json')['exit']!=0
assert read('paired-scalar-selects/tests-v1/scalar-select-tests.json')['exit']!=0
assert read('paired-scalar-selects/tests-v2/test-build.json')['exit']!=0
oracle=read('outputs/Rust.json');checked=set();observed=0
latency=read('cohorts/comparison-native-paired-select-and-visit-work-20261107-5000-m3.json')
rule=r['prospective_rule']
def quant(a,q):
 p=(len(a)-1)*q;i=math.floor(p);j=math.ceil(p);return a[i]+(a[j]-a[i])*(p-i)
for candidate,decision in r['analysis']['decisions'].items():
 for ref,comparisons in decision['paired'].items():
  for boundary,expected in comparisons.items():
   vals={n:{x['sample']:x[boundary]for x in latency['samples']if x['variant']==n}for n in [candidate,ref]}
   assert all(set(v)==set(range(31))for v in vals.values())
   a=[vals[candidate][i]/vals[ref][i]-1 for i in range(31)]
   rng=random.Random(rule['bootstrap']['seed']);boots=sorted(statistics.median(a[rng.randrange(31)]for _ in range(31))for _ in range(rule['bootstrap']['resamples']))
   assert sum(v<0 for v in a)==expected['wins'] and statistics.median(a)==expected['median_paired_relative_change']
   assert [quant(boots,.025),quant(boots,.975)]==expected['two_sided_95_interval']
   assert quant(boots,.95)==expected['one_sided_95_upper']

for n,count in [('comparison-native-paired-select-and-visit-work-20261107-5000-m3.json',806),('comparison-native-paired-select-and-visit-work-resources-20261107-5000-m3.json',189)]:
 c=read('cohorts/'+n);assert len(c['samples'])==count
 for row in c['samples']:
  key=row['output_sha256'];assert row['exit']==0 and row['exact_rust_output']
  if key not in checked:
   data=blob('outputs/'+key+'.json');assert sha(data)==key and json.loads(data)==oracle;checked.add(key)
  observed+=1
texts={}
for label,key in [('parent','parent'),('paired','paired'),('compact','compact'),('paired-LLVM','paired_LLVM')]:
 b=blob('Objects/'+label+'/object.o');a=32;magic,cpu,_,_,ncmd,_,_,_=struct.unpack_from('<8I',b);assert magic==0xfeedfacf and cpu==0x100000c;found=[]
 for _ in range(ncmd):
  cmd,size=struct.unpack_from('<II',b,a)
  if cmd==0x19:
   for i in range(struct.unpack_from('<I',b,a+64)[0]):
    q=a+72+i*80
    if b[q:q+16].split(bytes([0]))[0]==b'__text':
     length=struct.unpack_from('<Q',b,q+40)[0];offset=struct.unpack_from('<I',b,q+48)[0];found.append(b[offset:offset+length])
  a+=size
 assert len(found)==1 and sha(found[0])==r['actual_machine'][key]['text_sha256'];texts[label]=sha(found[0])
assert texts['parent']==texts['compact']and texts['paired']!=texts['parent']
assert read('representation-probes/execute.json')['exit']==0
assert read('comparison-probes/execute.json')['exit']==0
assert read('lifecycle-diagnostic/summary.json')['exact_outputs']==22
print(json.dumps(dict(records=len(m['files']),exact_cohort_output_references=observed,source_commits=r['source_commits'],actual_machine_facts_rederived=True,all_source_test_failures_retained=True,selected=False)))
