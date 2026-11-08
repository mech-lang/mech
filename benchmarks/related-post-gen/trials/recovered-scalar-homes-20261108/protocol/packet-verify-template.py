from pathlib import Path
import json,gzip,hashlib,math,random,statistics,struct
P=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def blob(n):
 p=P/n
 if not p.exists():p=Path(str(p)+'.gz')
 b=p.read_bytes();return gzip.decompress(b)if p.suffix=='.gz'else b
def read(n):return json.loads(blob(n))
m=read('manifest.json')
for n,v in m['files'].items():
 b=(P/n).read_bytes();assert sha(b)==v['sha256']and len(b)==v['bytes'];x=gzip.decompress(b)if v['gzip']else b;assert sha(x)==v['decoded_sha256']and len(x)==v['decoded_bytes']
r=read('compiler-native-recovered-scalar-homes-20261108.json');a=r['analysis'];rule=r['prospective_rule'];assert not r['goal_complete']
oracle=read('outputs/oracle.json');checked=set();total=0
for kind,n in [('latency',899),('resources',210)]:
 name='comparison-native-recovered-scalar-homes'+('-resources'if kind=='resources'else'')+'-20261108-5000-m3.json';c=read('cohorts/'+name);assert len(c['samples'])==n
 for x in c['samples']:
  k=x['output_sha256'];assert x['exit']==0 and x['exact_rust_output']
  if k not in checked:
   b=blob('outputs/'+k+'.json');assert sha(b)==k and json.loads(b)==oracle;checked.add(k)
  total+=1
lat=read('cohorts/comparison-native-recovered-scalar-homes-20261108-5000-m3.json');config=read('protocol/latency-config.json')
non_mech=[v for v in config['implementations']if v['kind']!='mech'];assert len(non_mech)==22
post_eligible=[v['name']for v in non_mech if v['calculation_boundary']=='post_json_including_setup']
post_best=min(post_eligible,key=lambda n:statistics.median(x.get('post_json_seconds',x['seconds'])for x in lat['samples']if x['variant']==n))
whole_best=min([v['name']for v in non_mech],key=lambda n:statistics.median(x['wall_seconds']for x in lat['samples']if x['variant']==n))
assert a['fastest_processing_Rust']['variant']==post_best and a['fastest_whole_Rust']['variant']==whole_best
def quant(a,q):
 pos=(len(a)-1)*q;i=math.floor(pos);j=math.ceil(pos);return a[i]+(a[j]-a[i])*(pos-i)
for candidate,d in a['decisions'].items():
 for ref,comparisons in d['paired'].items():
  for boundary,expected in comparisons.items():
   values={n:{x['sample']:x[boundary]for x in lat['samples']if x['variant']==n}for n in [candidate,ref]};changes=[values[candidate][i]/values[ref][i]-1 for i in range(31)];rng=random.Random(rule['bootstrap']['seed']);boots=sorted(statistics.median(changes[rng.randrange(31)]for _ in range(31))for _ in range(rule['bootstrap']['resamples']));assert expected['wins']==sum(x<0 for x in changes)and expected['median_paired_relative_change']==statistics.median(changes);assert expected['two_sided_95_interval']==[quant(boots,.025),quant(boots,.975)]and expected['one_sided_95_upper']==quant(boots,.95)
 target=d['paired'][rule['matched_parent']]['post_json_seconds']['two_sided_95_interval'][1]<0;assert d['first_cohort_passed']==target;assert d['whole_process_required_gate']is False

for n,v in a['medians'].items():
 rows=[x for x in lat['samples']if x['variant']==n]
 for k in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']:assert v[k]==statistics.median(x[k]for x in rows)
 rust_post=statistics.median(x.get('post_json_seconds',x['seconds'])for x in lat['samples']if x['variant']==post_best);rust_whole=statistics.median(x['wall_seconds']for x in lat['samples']if x['variant']==whole_best)
 assert math.isclose(v['times_fastest_processing_Rust'],v['post_json_seconds']/rust_post)and math.isclose(v['times_fastest_whole_Rust'],v['wall_seconds']/rust_whole)
def text_bytes(b):
 assert struct.unpack_from('<I',b)[0]==0xfeedfacf;pos=32
 for _ in range(struct.unpack_from('<I',b,16)[0]):
  cmd,size=struct.unpack_from('<II',b,pos)
  if cmd==0x19:
   for i in range(struct.unpack_from('<I',b,pos+64)[0]):
    s=pos+72+80*i
    if b[s:s+16].split(b'\0')[0]==b'__text':
     length=struct.unpack_from('<Q',b,s+40)[0];offset=struct.unpack_from('<I',b,s+48)[0];return b[offset:offset+length]
  pos+=size
 raise AssertionError('Object text absent')
assert text_bytes(blob('Objects/plain/object.o'))!=text_bytes(blob('Objects/IR278-plain/object.o'))
meta=read('Objects/plain/metadata.json');assert (meta['typed_ir_version'],meta['work_policy_version'],meta['native_descriptor_version'],meta['abi_version'])==(279,116,11,41);assert len(meta['compiled_numeric_functions'])==51
for tag in ['plain','IR278-plain']:
 audit=read(('runners/plain'if tag=='plain'else'Objects/IR278-plain')+'/native-link-proof.json');assert audit['status']=='passed'and sha(blob('Objects/'+tag+'/object.o'))==audit['object']['sha256'];assert audit['normalized_text_sha256']==(r['native_code']['normalized_text_sha256'] if tag=='plain' else 'a89fd61ff00cf98854fdc474ff3f57dbc0b2ac61412caffaca3db2f6dd66c3da')
refs=read('qualification/refs.json');assert len(refs)==35
for q in refs:
 inp=blob('inputs/qualification/'+q['input_sha256']+'.json');expected=blob('outputs/'+q['expected_sha256']+'.json');actual=blob('outputs/'+q['actual_sha256']+'.json');assert sha(inp)==q['input_sha256']and sha(expected)==q['expected_sha256']and sha(actual)==q['actual_sha256'];assert json.loads(actual)==json.loads(expected)
 if 'expected_indices_sha256'in q:
  posts=json.loads(inp);indices=read('inputs/qualification-indices/'+q['expected_indices_sha256']+'.json')['related_indices'];assert json.loads(expected)==[{'_id':p['_id'],'tags':p['tags'],'related':[posts[i]for i in ids]}for p,ids in zip(posts,indices)]
listed=read('tests/ci-list.json');executed=read('tests/ci-execute.json');names=[x.removesuffix(': test')for x in listed['stdout'].splitlines()if x.endswith(': test')];passed=[x.removeprefix('test ').removesuffix(' ... ok')for x in executed['stdout'].splitlines()if x.startswith('test ')and x.endswith(' ... ok')];assert listed['exit']==executed['exit']==0 and sorted(names)==sorted(passed)and len(names)==408
for file,count in [('packed-row-and-route-tests.json',8),('image-and-caller-geometry-tests.json',27),('optional-llvm-regressions.json',4),('recovered-scalar-home-tests.json',3)]:
 q=read('tests/'+file);assert q['exit']==0 and f'{count} passed'in q['stdout']
for kind,count in [('scalar',128),('one-kib',5)]:
 q=read('scalar-DEFLATE/'+kind+'-comparison.json');assert q['rows']==count and q['all_nontiming_fields_equal_IR240']
assert read('protocol/first-native-tests.json')['exit']!=0
assert read('protocol/first-qualification-harness-failure.json')
assert read('tests/test-source-inventory.json')['files']==read('compiler-build/source-inventory.json')['files']
assert read('protocol/latency-quiet.json')['quiet']and read('protocol/resources-quiet.json')['quiet']
resource=read('cohorts/comparison-native-recovered-scalar-homes-resources-20261108-5000-m3.json')
for name,expected in a['resource_medians'].items():
 rows=[x for x in resource['samples']if x['variant']==name+'-instrumented'];assert len(rows)==7
 for label,field in [('restore','restore_measurement'),('post_JSON','measurement')]:
  for key,value in expected[label].items():
   actual=statistics.median(x[field]['hardware'][key]if key in ['instructions_retired','cycles']else x[field][key]for x in rows);assert actual==value
candidate='mech-ir279-scalar-home-masks-4';parent=rule['matched_parent']
ratio=a['resource_medians'][candidate]['post_JSON']['instructions_retired']/a['resource_medians'][parent]['post_JSON']['instructions_retired'];assert ratio==a['decisions'][candidate]['processing_instruction_ratio']
assert ratio<1 and not r['selected'] and r['retained_for_composition']
assert not a['decisions'][candidate]['processing_latency_superiority_established']
for phase in ['restore','post_JSON']:
 for metric in ['allocation_requests','requested_allocation_bytes','peak_additional_live_requested_heap_bytes']:
  assert a['resource_medians'][candidate][phase][metric]==a['resource_medians'][parent][phase][metric]
assert r['native_code']['code_bytes']==889768 and r['native_code']['stack_bytes']==1552
print(json.dumps(dict(records=len(m['files']),cohort_outputs_exact=total,qualification_outputs_exact=len(refs),bootstrap_rederived=True,fastest_Rust_rederived=True,native_text_changed=True,native_CI_names_executed=408,scalar_DEFLATE_rows_exact=133,first_failures_retained=True,selected=r['selected'],goal_complete=False),indent=2))
