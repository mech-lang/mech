#!/usr/bin/env python3
"""Archive completed IR340 evidence; no builds, native executions or live-source authority."""
from pathlib import Path
import difflib,gzip,hashlib,json,subprocess
D=Path(__file__).resolve().parent; ROOT=D.parents[3]
M=Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
BASE='b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164'
FINAL='1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622'
records,blobs,omitted={},{},set()
sha=lambda raw:hashlib.sha256(raw).hexdigest()
def add(name,raw):
 key=sha(raw)
 if name in records:assert records[name]['uncompressed_sha256']==key,name;return
 if key not in blobs:
  packed=gzip.compress(raw,compresslevel=9,mtime=0);rel='records/'+key+'.gz';p=D/rel;p.parent.mkdir(exist_ok=True)
  if p.exists():assert p.read_bytes()==packed
  else:p.write_bytes(packed)
  blobs[key]={'path':rel,'uncompressed_sha256':key,'uncompressed_bytes':len(raw),'compressed_sha256':sha(packed),'compressed_bytes':len(packed)}
 records[name]={'source':name,**blobs[key]}
def evidence(path,name=None):
 if path.suffix in ('.rlib','.rmeta','.a','.dylib'):omitted.add(str(path));return
 with path.open('rb') as f:magic=f.read(4)
 if magic in (b'\xcf\xfa\xed\xfe',b'\xce\xfa\xed\xfe',b'\xca\xfe\xba\xbe',b'\x7fELF') and path.suffix!='.o':omitted.add(str(path));return
 add(name or str(path.relative_to(M)),path.read_bytes())
def tree(name,no_source=False):
 top=M/name;assert top.exists(),top
 for p in sorted(top.rglob('*')):
  if not p.is_file() or p.name=='.DS_Store' or '__pycache__' in p.parts:continue
  if no_source and p.relative_to(top).parts[0]=='source':continue
  evidence(p)
def read(name):return json.loads((M/name).read_bytes())
def git_files(commit,names):
 raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,check=True,input=''.join(commit+':'+n+'\n' for n in names).encode(),capture_output=True).stdout;at=0;out={}
 for n in names:
  end=raw.index(b'\n',at);head=raw[at:end].split();at=end+1;assert len(head)==3 and head[1]==b'blob',(n,head);size=int(head[2]);out[n]=raw[at:at+size];at+=size;assert raw[at:at+1]==b'\n';at+=1
 assert at==len(raw);return out
for name in ['first-dense-worker','new-managed-resources/resources-dense-first','independent-managed-dense-u32-prepared','dense-worker-audit','dense-worker-candidate','dense-worker-preparation','IR340-publication']:
 tree(name)
for dataset in read('dense-worker-audit/inventory.json')['datasets']:
 for field in ['descriptor_path','native_metadata_path']:
  p=Path(dataset[field]);evidence(p,'audit-inputs/'+str(p.relative_to(M)));assert sha(p.read_bytes())==dataset[field.replace('_path','_sha256')]
for label in ['dense-worker-wide','dense-worker-u32']:
 for suffix in ['','-decoded','-restored']:tree(label+suffix)
for label in ['dense-worker-compiler-v1','dense-worker-compiler-v2','dense-worker-fixture-v3','dense-worker-fixture-v4']:tree(label,no_source=True)
for name in ['owned-capture-compiler-v1/build.json','owned-capture-compiler-v1/source-inventory.json','owned-capture-compiler-v1/broad-native-tests.json','typed-replay-compiler-v3/build.json','typed-replay-compiler-v3/source-inventory.json','guarded-owner-compiler-v1/build.json','prefix-compiler-v1/build.json','native-failure-attribution.json','build-gpu-round-tests.py','prepare-source-round.py','new-managed-resources/run-with-compact.py','new-managed-resources/run.py','independent-managed-confirm-v3.py']:
 evidence(M/name)
for name in ['benchmarks/related-post-gen/compare-implementations.py','benchmarks/related-post-gen/measurement.rs','benchmarks/related-post-gen/experiments/check_subset3.py']:
 add('base-commit/'+name,git_files(BASE,[name])[name])
for p in sorted((ROOT/'benchmarks/related-post-gen/fixtures').rglob('*')):
 if p.is_file():name=str(p.relative_to(ROOT));add('base-commit/'+name,git_files(BASE,[name])[name])
primary=read('first-dense-worker/latency.json');resources=read('new-managed-resources/resources-dense-first/report.json');resource_plan=read('new-managed-resources/resources-dense-first/plan.json');confirm=read('independent-managed-dense-u32-prepared/report.json');confirm_plan=read('independent-managed-dense-u32-prepared/plan.json')
def referenced(value):
 if isinstance(value,dict):
  if isinstance(value.get('path'),str) and 'sha256' in value:
   p=Path(value['path'])
   if p.is_file():
    evidence(p,'referenced-inputs/'+str(p).lstrip('/'))
    if str(p) not in omitted:assert sha(p.read_bytes())==value['sha256'],p
  for child in value.values():referenced(child)
 elif isinstance(value,list):
  for child in value:referenced(child)
for variant in primary['implementations']+resource_plan['implementations']:
 for key in ['source','receipt','compiler_build','measurement_helper']:
  if isinstance(variant.get(key),str):
   p=Path(variant[key])
   if p.is_file():evidence(p,'referenced-inputs/'+str(p).lstrip('/'))
 referenced(variant.get('pins',{}))
referenced(resource_plan);referenced(confirm_plan)
for name,pin in [('posts.json',resource_plan['input']),('gold.json',resource_plan['gold'])]:
 raw=Path(pin['path']).read_bytes();assert sha(raw)==pin['sha256'];add('publication-inputs/'+name,raw)
# Reconstruct only changed source bytes over a required published base alias.
base_inventory=read('owned-capture-compiler-v1/source-inventory.json');base=git_files(BASE,sorted(base_inventory['files']))
assert len(base)==2817 and all(sha(base[n])==h for n,h in base_inventory['files'].items())
inventories={};overrides={};reconstructed={}
for label in ['dense-worker-compiler-v1','dense-worker-compiler-v2','dense-worker-fixture-v3','dense-worker-fixture-v4']:
 inventory=read(label+'/source-inventory.json');current={};ov={}
 for n,h in inventory['files'].items():
  if n in base and sha(base[n])==h:raw=base[n]
  else:
   raw=(M/label/'source'/n).read_bytes();assert sha(raw)==h,(label,n);ov[n]=h;add('inventory-overrides/'+label+'/'+n,raw)
  current[n]=raw
 inventories[label]=inventory;overrides[label]=ov;reconstructed[label]=current
final=reconstructed['dense-worker-compiler-v2'];assert git_files(FINAL,sorted(final))==final
assert len(final)==2818 and len(overrides['dense-worker-compiler-v2'])==10
for n,h in overrides['dense-worker-compiler-v2'].items():add('measured-source/'+n,final[n])
patch=[]
for n in sorted(overrides['dense-worker-compiler-v2']):
 patch.append('diff --git a/'+n+' b/'+n+'\n');patch.extend(difflib.unified_diff(base.get(n,b'').decode().splitlines(True),final[n].decode().splitlines(True),fromfile='a/'+n if n in base else '/dev/null',tofile='b/'+n))
patch=''.join(patch).encode();add('measured-source/IR338-to-IR340.patch',patch)
overlay={'base_commit':BASE,'source_commit':FINAL,'files':overrides['dense-worker-compiler-v2'],'new_files':sorted(set(final)-set(base)),'source_inventory':'dense-worker-compiler-v2/source-inventory.json','source_files':2818,'patch_sha256':sha(patch),'latency_checkpoint_selected':False};add('measured-source/overlay.json',(json.dumps(overlay,indent=2)+'\n').encode())
literal=read('typed-replay-compiler-v3/source-inventory.json')['literal_includes'];assert len(literal)==164
for n,raw in git_files(BASE,sorted(literal)).items():assert sha(raw)==literal[n]
# Qualifier comparators and all observed work/concurrency histograms, no filters.
work_quals={};histograms={}
for v in primary['implementations']:
 if not v['name'].startswith('mech-managed-'):continue
 n=v['name'];p=Path(v['receipt']).parent/'qualify-5000.json';raw=p.read_bytes();key='control-qualifications/'+n+'/qualify-5000.json';add(key,raw);q=json.loads(raw);assert q['exit']==0;x=next(json.loads(s) for s in q['stdout'].splitlines() if s.startswith('{'));work_quals[n]={'record':key,'sha256':sha(raw),'work':x['work'],'parallel_numeric_work':x['parallel_numeric_work'],'parallel_numeric_regions':x['parallel_numeric_regions'],'exact_total_required':n!='mech-managed-typed-fixed-byte'}
def histogram(rows,kind):
 result={}
 for row in rows:
  n=row['variant']
  if n not in work_quals:continue
  if kind=='primary':x=next(json.loads(s) for s in row['stdout'].splitlines() if s.startswith('{'))
  elif kind=='resource':x=row['raw_adapter_record']
  else:x=next(json.loads(s) for s in (M/'independent-managed-dense-u32-prepared'/row['stdout']['path']).read_text().splitlines() if s.startswith('{'))
  q=work_quals[n];assert x['parallel_tasks']==4 and x['parallel_batches']==1 and 1<=x['parallel_peak_workers']<=4;assert x['parallel_numeric_work']==q['parallel_numeric_work'] and x['parallel_numeric_regions']==q['parallel_numeric_regions']
  if q['exact_total_required']:assert x['work']==q['work'],(kind,n,x['work'],q['work'])
  out=result.setdefault(n,{'total_work':{},'peak_workers':{}})
  for field,key in [('work','total_work'),('parallel_peak_workers','peak_workers')]:k=str(x[field]);out[key][k]=out[key].get(k,0)+1
 return result
histograms['primary']=histogram(primary['samples'],'primary')
resource_rows_renamed=[{**r,'variant':r['variant'].removesuffix('-instrumented')} for r in resources['samples']];histograms['resources']=histogram(resource_rows_renamed,'resource');histograms['confirmation']=histogram(confirm['samples'],'confirmation')
reference=primary['primary_reference'];rows=[]
serial=min((v['name'] for v in primary['implementations'] if v['kind']!='mech' and v.get('calculation_boundary')=='post_json_including_setup' and 'fresh4' not in v['name']),key=lambda n:primary['summary'][n]['median_post_json_seconds'])
for v in primary['implementations']:
 n=v['name']
 if v['kind']=='mech' or n in [reference['variant'],serial]:
  x=primary['summary'][n];rows.append({'variant':n,'post_json_ms':x['median_post_json_seconds']*1000,'times_current_fastest_Rust':x['median_post_json_seconds']/reference['median_seconds'],'whole_process_ms':x['median_whole_process_seconds']*1000,'code_bytes':v.get('code_bytes'),'stack_bytes':v.get('stack_bytes'),'executable_file_bytes':v.get('executable_file_bytes'),'whole_process_peak_resident_bytes':x['process_resources_median'].get('peak_resident_bytes')})
resource_rows=[]
for n,x in resources['summary'].items():
 if n.startswith('mech-') or 'fresh4' in n:
  w=x['window_medians']['post_json'];resource_rows.append({'variant':n,'instructions_retired':w['hardware']['instructions_retired'],'cycles':w['hardware']['cycles'],'allocation_requests':w['allocation_requests'],'requested_allocation_bytes':w['requested_allocation_bytes'],'peak_additional_live_requested_heap_bytes':w['peak_additional_live_requested_heap_bytes'],'whole_process_peak_resident_bytes':x['whole_process_medians']['peak_resident_bytes']})
report={'schema':'mech.related-post-gen.managed-dense-storage-evidence.v1','required_published_IR338_base':BASE,'required_published_IR340_source':FINAL,'optional_historical_IR338_source':'6f7e0458e088cf7e50c97ebfe2d15e7bca02b22f','IR':340,'work_policy':127,'ABI':42,'image':11,'managed_selected':'IR334 scoped-frame','new_selected_latency_checkpoint':False,'same_source_U32_gain_confirmed':True,'wide_gain_established':False,'fastest_Rust_target_met':False,'whole_process_is_acceptance_gate':False,'new_timed_resource_exact_outputs':1895,'qualification_exact_outputs_separate':384,
'primary':{'record':'first-dense-worker/latency.json','paired':'first-dense-worker/paired-comparisons.json','rounds':31,'variants':41,'exact_outputs':1271,'seed':202611121041,'bootstrap_seed':202611121042,'current_fastest_Rust':reference,'rows':rows,'managed_work_qualifications':work_quals},
'resources':{'record':'new-managed-resources/resources-dense-first/report.json','plan':'new-managed-resources/resources-dense-first/plan.json','rounds':7,'variants':39,'exact_outputs':273,'seed':resource_plan['seed'],'rows':resource_rows,'instrumented_latency_selection_forbidden':True,'compact_contract':'Separate compact/unmetered contract; checked arithmetic/indexing; native payload counters retained separately from tracked Rust heap/RSS.'},
'confirmation':{'record':'independent-managed-dense-u32-prepared/report.json','plan':'independent-managed-dense-u32-prepared/plan.json','rounds':13,'variants':27,'exact_outputs':351,'seed':confirm_plan['seed'],'candidate':confirm['candidate'],'control':'mech-managed-scoped-frame','extra_control':'mech-managed-owned-capture-u32','references':confirm['references'],'candidate_summary':confirm['summary'][confirm['candidate']],'paired':confirm['candidate_paired_comparisons'],'same_machine_independent_orchestration':True},
'observed_histograms':histograms,'historical_IR335_total_work_variation':'Unattributed total-work variation in the retained unselected IR335 typed-fixed-byte control only; numeric worker bill remains exact. No samples removed or total-work constancy claimed for that control.',
'source_closure':{'base_inventory':'owned-capture-compiler-v1/source-inventory.json','base_files':2817,'inventory_overrides':overrides,'final_files':2818,'overlay':overrides['dense-worker-compiler-v2'],'patch':'measured-source/IR338-to-IR340.patch','overlay_record':'measured-source/overlay.json','literal_includes':164},
'qualification':{'complete':'dense-worker-preparation/complete.json','audit':'dense-worker-preparation/qualification-audit.json','exact_outputs':384,'forms':['original','decoded','restored'],'adapters':['ordinary','metrics'],'sources':['wide','u32'],'full_posts':5000,'edge_cases':31,'focused':'dense-worker-fixture-v3/focused-contracts.json','focused_passed':33,'broad':'dense-worker-fixture-v3/broad-native-tests.json','broad_attribution':'dense-worker-fixture-v3/broad-native-failure-attribution.json','baseline_broad':'owned-capture-compiler-v1/broad-native-tests.json','broad_counts':{'passed':1084,'failed':16,'ignored':12,'filtered':35},'all_16_failures_inherited':True,'positive_wide':'dense-worker-fixture-v3/positive-storage-receipts.json','positive_u32':'dense-worker-fixture-v4/positive-u32-storage-receipt.json'},
'physical_storage':{'wide':{'full_funded_slots':2420,'logical_prefix':1341,'dense_slots':329,'banks_bytes':23688,'map_bytes':5364,'map_record_bytes':24,'four_worker_bank_saving_bytes':291456},'u32':{'full_funded_slots':2840,'logical_prefix':1388,'dense_slots':331,'banks_bytes':23832,'map_bytes':5552,'map_record_bytes':24,'four_worker_bank_saving_bytes':304416}},
'retention':{'executables_and_libraries':'Recorded by immutable build/link/hash receipts; omitted large binaries','source':'Complete frozen inventories plus minimal override bytes over required published IR338 base alias','all_observations_retained':True,'native_executions_by_assembler':0,'prior_packet':'managed-setup-reuse-20261112; unchanged'}}
(D/'report.json').write_text(json.dumps(report,indent=2)+'\n')
manifest={'schema':'mech.related-post-gen.deduplicated-records.v1','logical_records':len(records),'unique_blobs':len(blobs),'compressed_bytes':sum(v['compressed_bytes'] for v in blobs.values()),'uncompressed_logical_bytes':sum(v['uncompressed_bytes'] for v in records.values()),'records':[records[n] for n in sorted(records)]};(D/'records.json').write_text(json.dumps(manifest,indent=2)+'\n');(D/'omitted-binaries.json').write_text(json.dumps(sorted(omitted),indent=2)+'\n');print(json.dumps({k:v for k,v in manifest.items() if k!='records'},indent=2))
