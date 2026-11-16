#!/usr/bin/env python3
"""Freeze completed evidence only. No builds, native executions or live source authority."""
from pathlib import Path
import difflib,gzip,hashlib,json,subprocess
D=Path(__file__).resolve().parent;ROOT=D.parents[3];M=Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
records={};blobs={};omitted=set()
if (D/'records.json').exists():
 for row in json.loads((D/'records.json').read_text())['records']:blobs[row['uncompressed_sha256']]={k:v for k,v in row.items()if k!='source'}
sha=lambda x:hashlib.sha256(x).hexdigest()
def add(name,raw):
 key=sha(raw)
 if name in records:assert records[name]['uncompressed_sha256']==key,name;return
 if key not in blobs:
  packed=gzip.compress(raw,compresslevel=9,mtime=0);relative='records/'+key+'.gz';p=D/relative;p.parent.mkdir(exist_ok=True)
  if p.exists():assert p.read_bytes()==packed
  else:p.write_bytes(packed)
  blobs[key]={'path':relative,'uncompressed_sha256':key,'uncompressed_bytes':len(raw),'compressed_sha256':sha(packed),'compressed_bytes':len(packed)}
 records[name]={'source':name,**blobs[key]}
def evidence(p,name=None):
 p=Path(p)
 if not p.is_file():return
 if p.suffix in ('.rlib','.rmeta','.a','.dylib') or any(x in p.parts for x in ('backend-target','target','.git')):omitted.add(str(p));return
 with p.open('rb')as f:magic=f.read(4)
 if magic in (b'\xcf\xfa\xed\xfe',b'\xce\xfa\xed\xfe',b'\xca\xfe\xba\xbe',b'\x7fELF')and p.suffix!='.o':omitted.add(str(p));return
 add(name or str(p.relative_to(M)),p.read_bytes())
def tree(name,no_source=False):
 top=M/name;assert top.exists(),top
 for p in sorted(top.rglob('*')):
  rel=p.relative_to(top)
  if not p.is_file()or p.name=='.DS_Store' or any(x in rel.parts for x in ('backend-target','target','.git')):continue
  if no_source and rel.parts[0]=='source':continue
  evidence(p)
def read(name):return json.loads((M/name).read_bytes())
def git_files(commit,names,missing=False):
 raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,check=True,input=''.join(commit+':'+n+'\n'for n in names).encode(),capture_output=True).stdout;cursor=0;result={}
 for n in names:
  end=raw.index(b'\n',cursor);h=raw[cursor:end].split();cursor=end+1
  if h[-1]==b'missing':assert missing,(commit,n);continue
  assert len(h)==3 and h[1]==b'blob',(n,h);size=int(h[2]);result[n]=raw[cursor:cursor+size];cursor+=size;assert raw[cursor:cursor+1]==b'\n';cursor+=1
 assert cursor==len(raw);return result

trees=['first-native-query-gains','new-managed-resources/resources-query-first','independent-managed-query-comparison-prepared','independent-managed-query-comparison-prepared-v2','independent-managed-query-comparison-prepared-v3','producer-element-range-candidate','producer-element-qualified-v1','comparison-chain-candidate','comparison-chain-candidate-v2','comparison-chain-candidate-v3','comparison-chain-backend-contracts-v1','comparison-chain-backend-contracts-v3','comparison-chain-native-baseline-compile-v1','comparison-chain-native-baseline-compile-v2','comparison-chain-native-baseline-compile-v3','comparison-chain-matcher-baseline-contracts-v1','comparison-chain-matcher-baseline-contracts-v2','comparison-chain-constant-contracts-source-v1','comparison-chain-constant-contracts-v1','comparison-chain-clif-audit','comparison-chain-wide','comparison-chain-u32','comparison-chain-preparation-v3','comparison-chain-machine-audit-v3','comparison-compositions-machine-audit-v1','dense-comparison-qualified-v1','dense-comparison-preparation-v1','dense-producer-comparison-qualified-v1','dense-producer-comparison-preparation-v1','dense-producer-qualified-v1','rust-native-query-audit','indirect-range-machine-audit','dense-worker-fullpath-profile-v1']
for name in trees:tree(name)
compiler_labels=['producer-element-compiler-v1','comparison-chain-compiler-v3','dense-comparison-compiler-v2','dense-producer-comparison-compiler-v1','dense-producer-compiler-v1']
for name in compiler_labels+['comparison-chain-compiler-v1','comparison-chain-compiler-v2','dense-comparison-compiler-v1']:tree(name,no_source=True)
for name in ['owned-capture-compiler-v1/build.json','owned-capture-compiler-v1/source-inventory.json','dense-worker-compiler-v2/build.json','dense-worker-compiler-v2/source-inventory.json','typed-replay-compiler-v3/source-inventory.json','native-failure-attribution.json','scoped-frame-compiler-v1/broad-native-tests.json','new-managed-resources/run-with-compact.py','build-fresh-backend-tests-v1.py','build-fresh-backend-tests-v2.py','build-comparison-backend-v1.py','build-comparison-backend-v2.py','build-comparison-backend-v3.py','build-comparison-clif-audit.py','build-dense-comparison-v1.py','build-dense-comparison-v2.py','build-dense-producer-comparison-v1.py','build-dense-producer-v1.py','test-dense-producer-comparison-v1.py','independent-managed-confirm-v4.py','independent-managed-confirm-v5.py','independent-managed-confirm-v6.py','prepare-source-round.py','qualify-comparison-metrics-v3.py']:
 if (M/name).exists():evidence(M/name)
primary=read('first-native-query-gains/latency.json');resource_plan=read('new-managed-resources/resources-query-first/plan.json');confirmation_plan=read('independent-managed-query-comparison-prepared-v3/plan.json')
# Plan inputs and external historical controls retain their exact source/receipt
# identities. Libraries/executables are hashes only; never needed by verifier.
def referenced(value):
 if isinstance(value,dict):
  if isinstance(value.get('path'),str)and isinstance(value.get('sha256'),str):
   p=Path(value['path'])
   if p.is_file():
    if p.suffix not in ('.rlib','.rmeta','.a','.dylib'):assert sha(p.read_bytes())==value['sha256'],str(p)
    evidence(p,'referenced-inputs/'+str(p).lstrip('/'))
  for v in value.values():referenced(v)
 elif isinstance(value,list):
  for v in value:referenced(v)
for plan in [resource_plan,confirmation_plan]:referenced(plan)
for name in ['dense-worker-fullpath-profile-v1/recipe.json','dense-worker-fullpath-profile-v1/complete.json','dense-worker-fullpath-profile-v1/analysis-receipt.json','comparison-compositions-machine-audit-v1/audit.json','comparison-chain-machine-audit-v3/audit.json','producer-element-qualified-v1/qualification.json','dense-comparison-qualified-v1/qualification.json','dense-producer-comparison-qualified-v1/qualification.json','dense-producer-qualified-v1/qualification.json']:
 referenced(read(name))
for variant in primary['implementations']:
 for field in ['source','receipt','compiler_build','measurement_helper']:
  p=variant.get(field)
  if isinstance(p,str)and Path(p).is_file():evidence(Path(p),'referenced-inputs/'+p.lstrip('/'))
 referenced(variant.get('pins',{}))
for n,pin in [('posts.json',resource_plan['input']),('gold.json',resource_plan['gold'])]:raw=Path(pin['path']).read_bytes();assert sha(raw)==pin['sha256'];add('publication-inputs/'+n,raw)
# Published source authorities; only changed candidate files are stored.
bases={}
for label,commit in [('owned-capture-compiler-v1','b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164'),('dense-worker-compiler-v2','1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622')]:
 inv=read(label+'/source-inventory.json');raw=git_files(commit,sorted(inv['files']));assert all(sha(raw[n])==v for n,v in inv['files'].items());bases[label]={'commit':commit,'inventory_record':label+'/source-inventory.json','files':raw}
closures={}
for label in compiler_labels:
 inv=read(label+'/source-inventory.json');rank=[(sum(b['files'].get(n,b'')==b'__never__' or n not in b['files'] or sha(b['files'][n])!=h for n,h in inv['files'].items()),name)for name,b in bases.items()];_,base_name=min(rank);base=bases[base_name];overrides={};parts=[]
 for n,h in inv['files'].items():
  if n in base['files']and sha(base['files'][n])==h:continue
  p=M/label/'source'/n;raw=p.read_bytes();assert sha(raw)==h,(label,n);add('source-overrides/'+label+'/'+n,raw);overrides[n]=h
  parts.append('diff --git a/'+n+' b/'+n+'\n');parts.extend(difflib.unified_diff(base['files'].get(n,b'').decode().splitlines(True),raw.decode().splitlines(True),fromfile='a/'+n if n in base['files']else'/dev/null',tofile='b/'+n))
 patch=''.join(parts).encode();add('source-overrides/'+label+'/candidate.patch',patch);closures[label]={'base':base_name,'base_commit':base['commit'],'inventory_record':label+'/source-inventory.json','overrides':overrides,'patch_record':'source-overrides/'+label+'/candidate.patch','source_count':len(inv['files'])}
# Complete configured vendor closure includes non-Rust ISLE/package inputs.
vendor=read('comparison-chain-compiler-v3/vendor-full-source-inventory.json');vendor_base=git_files(bases['owned-capture-compiler-v1']['commit'],sorted(vendor),missing=True);vendor_overrides={}
for n,h in vendor.items():
 if n in vendor_base and sha(vendor_base[n])==h:continue
 raw=(M/'comparison-chain-compiler-v3/source'/n).read_bytes();assert sha(raw)==h;add('vendor-overrides/'+n,raw);vendor_overrides[n]=h
literal=read('typed-replay-compiler-v3/source-inventory.json')['literal_includes']
for n,raw in git_files(bases['owned-capture-compiler-v1']['commit'],sorted(literal)).items():assert sha(raw)==literal[n]
for n in ['benchmarks/related-post-gen/compare-implementations.py','benchmarks/related-post-gen/measurement.rs','benchmarks/related-post-gen/experiments/check_subset3.py']:
 add('published-base/'+n,git_files(bases['dense-worker-compiler-v2']['commit'],[n])[n])
for p in sorted((ROOT/'benchmarks/related-post-gen/fixtures').rglob('*')):
 if p.is_file():n=str(p.relative_to(ROOT));add('published-base/'+n,git_files(bases['dense-worker-compiler-v2']['commit'],[n])[n])
work={}
for v in primary['implementations']:
 if not v['name'].startswith('mech-managed-'):continue
 p=Path(v.get('qualification',{}).get('qualify-5000.json',str(Path(v['receipt']).parent/'qualify-5000.json')));assert p.is_file(),p;raw=p.read_bytes();j=json.loads(raw);assert j['exit']==0;value=next(json.loads(line)for line in j['stdout'].splitlines()if line.startswith('{'));name='control-qualifications/'+v['name']+'/full.json';add(name,raw);work[v['name']]={'record':name,'sha256':sha(raw),'work':value['work'],'numeric_work':value['parallel_numeric_work'],'numeric_regions':value['parallel_numeric_regions'],'exact_total_required':v['name']!='mech-managed-typed-fixed-byte'}
report={'schema':'mech.related-post-gen.native-query-evidence.v1','scope':'Completed IR339/341/342/343/344 exploratory compiler candidates only. IR345/346 and later work excluded. Offline evidence/source verifier; no binaries or native execution required.','timed_exact_outputs':2314,'primary':{'record':'first-native-query-gains/latency.json','config':'first-native-query-gains/config.json','paired':'first-native-query-gains/paired-comparisons.json','outputs':1581,'rounds':31,'variants':51},'resources':{'record':'new-managed-resources/resources-query-first/report.json','plan':'new-managed-resources/resources-query-first/plan.json','outputs':343,'rounds':7,'variants':49},'confirmation':{'record':'independent-managed-query-comparison-prepared-v3/report.json','plan':'independent-managed-query-comparison-prepared-v3/plan.json','additional_paired':'independent-managed-query-comparison-prepared-v3/additional-paired-comparisons.json','outputs':390,'rounds':13,'variants':30},'managed_work_qualifications':work,'historical_IR335_work_variation':'Retain every observation and exact per-variant numeric-worker bill. Historical unselected IR335 typed-fixed-byte total work varies; physical cause remains unattributed. All other managed totals required exact per matching same-source/compiler qualification. Worker contract tasks4/batch1, observed peak1..4; no filtering or rerun.','source_closure':{'bases':{k:{n:v for n,v in x.items()if n!='files'}for k,x in bases.items()},'compilers':closures,'vendor_inventory':'comparison-chain-compiler-v3/vendor-full-source-inventory.json','vendor_base_commit':bases['owned-capture-compiler-v1']['commit'],'vendor_overrides':vendor_overrides,'literal_inventory':'typed-replay-compiler-v3/source-inventory.json','literal_includes':len(literal)},'qualification':{'new_adapter_exact_outputs':640,'new_managed_sources':10,'ordinary_and_metrics_adapters':20,'edges_per_adapter':31,'IR341_preparation':'comparison-chain-preparation-v3/qualification-audit.json','other_preparations':[x+'/qualification.json'for x in ['producer-element-qualified-v1','dense-comparison-qualified-v1','dense-producer-comparison-qualified-v1','dense-producer-qualified-v1']],'native_functions':1031,'native_correctness_calls':1847152,'matcher_contracts':4,'primary_native_contracts':'comparison-chain-backend-contracts-v3/completed.json','constant_supplement':'comparison-chain-constant-contracts-v1/completed.json','broad':'dense-producer-comparison-compiler-v1/broad-native-tests.json','broad_passed':1088,'broad_inherited_failures':16,'ignored':11,'filtered':35},'selection':{'new_selected':False,'selected_checkpoint_IR':334,'fastest_Rust_target_met':False,'IR341_u32':'Confirmed gain only against matching IR338 U32 parent; no gain established over current selected checkpoint or fastest Rust.','IR342_wide':'Primary paired gain against IR340 wide failed independent confirmation; unselected.','IR339_IR344_IR343':'Producer proof correct but relevant paired regressions retained; unselected.','best_primary_diagnostic':{'variant':'mech-managed-dense-worker-u32','post_json_ms':1.433625,'current_fastest_Rust_ms':1.227917,'multiple':1.433625/1.227917}}}
add('packet-report.json',(json.dumps(report,indent=2)+'\n').encode());(D/'report.json').write_text(json.dumps(report,indent=2)+'\n');(D/'omitted-binaries.json').write_text(json.dumps(sorted(omitted),indent=2)+'\n');used={x['uncompressed_sha256'] for x in records.values()};blobs={k:v for k,v in blobs.items()if k in used}
for path in (D/'records').glob('*.gz'):
 if path.stem not in used:path.unlink()
manifest={'logical_records':len(records),'unique_blobs':len(blobs),'compressed_bytes':sum(x['compressed_bytes']for x in blobs.values()),'records':sorted(records.values(),key=lambda x:x['source'])};(D/'records.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps({k:v for k,v in manifest.items()if k!='records'}))
