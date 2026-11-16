#!/usr/bin/env python3
"""Archive completed observations only; no compilation or native execution."""
from pathlib import Path
import difflib
import gzip
import hashlib
import json
import subprocess
import shutil
import zipfile

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
M = Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
BASE338 = 'b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164'
BASE340 = '1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622'
BASE349 = '7a6686c6790b0d8e0a282a5619bb884f8ffe08d0'
records, blobs, omitted = {}, {}, set()
BLOB_CACHE = M / 'u16-host-source-packet-blob-cache-v1'
BLOB_CACHE.mkdir(exist_ok=True)
cache_path = BLOB_CACHE / 'cohort-cache.json'
old_cache_path = D / '.assembly-cohort-cache.json'
if old_cache_path.exists() and not cache_path.exists():
    shutil.move(old_cache_path, cache_path)
if (D / 'records').exists():
    for old in (D / 'records').glob('*.gz'):
        target = BLOB_CACHE / 'records' / old.name
        target.parent.mkdir(exist_ok=True)
        if target.exists():
            assert old.read_bytes() == target.read_bytes()
            old.unlink()
        else:
            shutil.move(old, target)
    (D / 'records').rmdir()
cohort_cache = json.loads(cache_path.read_bytes()) if cache_path.exists() else {}
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def add(name, raw):
    digest = sha(raw)
    if name in records:
        assert records[name]['uncompressed_sha256'] == digest, name
        return
    if digest not in blobs:
        rel = 'records/' + digest + '.gz'
        target = BLOB_CACHE / rel
        target.parent.mkdir(exist_ok=True)
        if target.exists():
            packed = target.read_bytes()
            assert gzip.decompress(packed) == raw
        else:
            previous = M / 'packed-scalar-packet-blob-cache-v1' / rel
            if previous.exists() and gzip.decompress(previous.read_bytes()) == raw:
                packed = previous.read_bytes()
            else:
                packed = gzip.compress(raw, compresslevel=9, mtime=0)
            target.write_bytes(packed)
        blobs[digest] = dict(path=rel, uncompressed_sha256=digest,
                             uncompressed_bytes=len(raw), compressed_sha256=sha(packed),
                             compressed_bytes=len(packed))
    records[name] = dict(source=name, **blobs[digest])


def evidence(path, name=None):
    path = Path(path)
    if not path.is_file():
        return
    logical_name = name or str(path.relative_to(M))
    if logical_name in cohort_cache:
        row = cohort_cache[logical_name]
        records[logical_name] = row
        blobs[row['uncompressed_sha256']] = {k: v for k, v in row.items() if k != 'source'}
        return
    if path.suffix in ('.rlib', '.rmeta', '.a', '.dylib') or any(
        part in path.parts for part in ('backend-target', 'target', '.git', '__pycache__')
    ):
        omitted.add(str(path))
        return
    with path.open('rb') as stream:
        magic = stream.read(4)
    if magic in (b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\x7fELF') and path.suffix != '.o':
        omitted.add(str(path))
        return
    add(logical_name, path.read_bytes())


def tree(name, no_source=False):
    top = M / name
    assert top.exists(), top
    for path in sorted(top.rglob('*')):
        rel = path.relative_to(top)
        if not path.is_file() or path.name == '.DS_Store':
            continue
        if any(part in rel.parts for part in ('target', 'backend-target', '.git', '__pycache__')):
            continue
        if no_source and rel.parts[0] == 'source':
            continue
        evidence(path)


def read(name):
    return json.loads((M / name).read_bytes())


def git_files(commit, names, missing=False):
    raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True,
                         input=''.join(commit + ':' + name + '\n' for name in names).encode(),
                         capture_output=True).stdout
    cursor, result = 0, {}
    for name in names:
        end = raw.index(b'\n', cursor)
        header = raw[cursor:end].split()
        cursor = end + 1
        if header[-1] == b'missing':
            assert missing, (commit, name)
            continue
        assert len(header) == 3 and header[1] == b'blob', (name, header)
        size = int(header[2])
        result[name] = raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(raw)
    return result


cohorts = [
 dict(kind='primary',prefix='first-selected-increment-lazy-map-gains-v2',outputs=1457,rounds=31,variants=47),
 dict(kind='resources',prefix='new-managed-resources/resources-selected-increment-lazy-map-first',outputs=336,rounds=7,variants=48),
 dict(kind='confirmation',prefix='independent-selected-increment-lazy-map-confirmation-v1',outputs=611,rounds=13,variants=47),
 dict(kind='primary',prefix='first-u16-host-mask-source-gains-v1',outputs=1612,rounds=31,variants=52),
 dict(kind='resources',prefix='new-managed-resources/resources-u16-host-mask-first',outputs=371,rounds=7,variants=53),
 dict(kind='confirmation',prefix='independent-u16-host-mask-confirmation-v1',outputs=676,rounds=13,variants=52),
]
assert sum(x['outputs']for x in cohorts)==5063
for row in cohorts:
 row['record']=row['prefix']+('/latency.json'if row['kind']=='primary'else'/report.json')
 row['plan']=row['prefix']+('/config.json'if row['kind']=='primary'else'/plan.json')
 tree(row['prefix'])
cache_path.write_text(json.dumps(records,sort_keys=True)+'\n')
compiler_labels=[
 'selected-increment-compiler-v4','selected-increment-clif350-compiler-v1',
 'lazy-map-path-compiler-v1','selected-increment-lazy-map-compiler-v1',
 'u16-arithmetic-compiler-v1','u16-arithmetic-compiler-v2',
 'exact-histogram-key-compiler-v1','u16-exact-histogram-compiler-v2','u16-exact-histogram-compiler-v3',
]
for label in compiler_labels:tree(label,no_source=True)
prefixes=('selected-increment-','lazy-map-path-','u16-arithmetic-','u16-exact-histogram-','exact-histogram-key-',
 'guarded-u16-','mask-reuse-','mask-reuse355-','compact-increment350-','compact-idiom350-',
 'compact-idiom352-','compact-lazy-map-path-','compact-exact-histogram-key-',
 'compact-composition350-','compact-composition352-','serial-geometry-machine-audit-',
 'u16-parity-machine-audit-','u16-355-parity-static-audit-','map-pair-ingress-design-')
collected=[]
for path in sorted(M.iterdir()):
 if path.is_dir() and path.name.startswith(prefixes) and path.name not in compiler_labels:
  # Failed compiler/preflight snapshots have no accepted inventory. Preserve
  # their non-source records and their explicit candidate overlay trees.
  tree(path.name,no_source='compiler' in path.name)
  collected.append(path.name)
for name in ['first-selected-increment-lazy-map-gains-v1','IR355-source-publication','fixed-benchmark-provenance-v1','u16-host-source-independent-audit-v1']:
 if (M/name).exists():tree(name);collected.append(name)
for name in ['assembly-v1.stdout','assembly-v1.stderr','assembly-v2.stdout','assembly-v2.stderr','verify-v1.stdout','verify-v1.stderr','verify-v2.stdout','verify-v2.stderr']:
 evidence(M/'u16-host-source-packet-development-v1'/name,'packet-development-attempts/'+name)
for p in sorted(M.glob('*.py')):
 if any(s in p.name for s in ('selected-increment','lazy-map','u16','exact-histogram')):evidence(p)
for name in ['benchmark-target-policy-20261112.json','symbol-provenance-correction.json',
 'comparison-chain-compiler-v3/backend-build.json','comparison-chain-compiler-v3/backend-libraries.json',
 'comparison-chain-compiler-v3/vendor-full-source-inventory.json','typed-replay-compiler-v3/source-inventory.json',
 'packed-selected-homes-compiler-v1/source-inventory.json','packed-selected-homes-compiler-v1/build.json',
 'owned-capture-compiler-v1/build.json','new-managed-resources/run.py','new-managed-resources/run-with-compact.py',
 'independent-compact-confirm-v1.py','independent-managed-confirm-v7.py','native-failure-attribution.json']:
 evidence(M/name)
for name in ['benchmarks/related-post-gen/mech/related_guarded_u16.mec']:
 evidence(ROOT/name,'publication-source/'+name)
print('Completed cohort/qualification trees collected:',len(records),flush=True)
seen_json=set();pin_mismatches=[]
def referenced(value):
 if isinstance(value,dict):
  path,digest=value.get('path'),value.get('sha256')
  if isinstance(path,str)and isinstance(digest,str):
   p=Path(path)
   if p.is_file():
    evidence(p,'referenced-inputs/'+str(p).lstrip('/'))
    if str(p)not in omitted:
     raw=p.read_bytes()
     if sha(raw)!=digest:
      pin_mismatches.append(dict(path=str(p),expected_sha256=digest,current_sha256=sha(raw),expected_digest_already_archived=digest in blobs))
     else:
      if 'bytes'in value:assert len(raw)==value['bytes'],p
      if p.suffix=='.json'and digest not in seen_json:
       seen_json.add(digest);referenced(json.loads(raw))
  for child in value.values():referenced(child)
 elif isinstance(value,list):
  for child in value:referenced(child)
for name in list(records):
 if name.endswith('.json'):
  digest=records[name]['uncompressed_sha256']
  if digest in seen_json:continue
  seen_json.add(digest)
  try:value=json.loads(gzip.decompress((BLOB_CACHE/records[name]['path']).read_bytes()))
  except(UnicodeDecodeError,json.JSONDecodeError):continue
  referenced(value)
for cohort in cohorts:
 value=read(cohort['record'])
 for variant in value.get('implementations',[])+read(cohort['plan']).get('implementations',[]):
  for field in ['source','receipt','compiler_build','measurement_helper']:
   if isinstance(variant.get(field),str):evidence(Path(variant[field]),'referenced-inputs/'+str(variant[field]).lstrip('/'))
  referenced(variant.get('pins',{}))
add('pin-discovery-mismatches.json',(json.dumps(pin_mismatches,indent=2)+'\n').encode())
print('Referenced inputs collected:',len(records),flush=True)
plan=read(cohorts[1]['plan'])
for label,value in [('posts.json',plan['input']),('gold.json',plan['gold'])]:
 raw=Path(value['path']).read_bytes();assert sha(raw)==value['sha256'];add('publication-inputs/'+label,raw)
base_inv=read('packed-selected-homes-compiler-v1/source-inventory.json');base_files=git_files(BASE349,sorted(base_inv['files']))
assert all(sha(base_files[name])==digest for name,digest in base_inv['files'].items())
def source_bytes(label,inventory,name,digest):
 paths=[M/label/'source'/name]
 if inventory.get('gpu_snapshot')and name.startswith('hosts/gpu/'):
  paths.append(Path(inventory['gpu_snapshot'])/name.removeprefix('hosts/gpu/'))
 if inventory.get('vendor_snapshot')and name.startswith('vendor/'):
  paths.append(Path(inventory['vendor_snapshot'])/name.removeprefix('vendor/'))
 if name.startswith('vendor/'):paths.append(M/'comparison-chain-compiler-v3/source'/name)
 for p in paths:
  if p.is_file()and sha(p.read_bytes())==digest:return p.read_bytes()
 raise AssertionError((label,name,digest,paths))
closures={}
for label in compiler_labels:
 inv=read(label+'/source-inventory.json');overrides={};parts=[]
 for name,digest in inv['files'].items():
  if name in base_files and sha(base_files[name])==digest:continue
  raw=source_bytes(label,inv,name,digest);add('source-overrides/'+label+'/'+name,raw);overrides[name]=digest
  parts.append('diff --git a/'+name+' b/'+name+'\n');parts.extend(difflib.unified_diff(base_files.get(name,b'').decode().splitlines(True),raw.decode().splitlines(True),fromfile='a/'+name if name in base_files else'/dev/null',tofile='b/'+name))
 patch='source-overrides/'+label+'/candidate.patch';add(patch,''.join(parts).encode())
 closures[label]=dict(base='IR349',base_commit=BASE349,inventory_record=label+'/source-inventory.json',source_count=len(inv['files']),overrides=overrides,patch_record=patch)
vendor=read('comparison-chain-compiler-v3/vendor-full-source-inventory.json');assert len(vendor)==246;vendorbase=git_files(BASE349,sorted(vendor),missing=True);vendoroverrides={}
for name,digest in vendor.items():
 if name in vendorbase and sha(vendorbase[name])==digest:continue
 raw=(M/'comparison-chain-compiler-v3/source'/name).read_bytes();assert sha(raw)==digest;add('vendor-overrides/'+name,raw);vendoroverrides[name]=digest
literal=read('typed-replay-compiler-v3/source-inventory.json')['literal_includes'];literalbase=git_files(BASE349,sorted(literal));assert all(sha(literalbase[n])==digest for n,digest in literal.items())
for name in ['benchmarks/related-post-gen/compare-implementations.py','benchmarks/related-post-gen/measurement.rs','benchmarks/related-post-gen/experiments/check_subset3.py']:
 add('published-base/'+name,git_files(BASE349,[name])[name])
for p in sorted((ROOT/'benchmarks/related-post-gen/fixtures').rglob('*')):
 if p.is_file():
  n=str(p.relative_to(ROOT));add('published-base/'+n,git_files(BASE349,[n])[n])
qualifications=[]
for label in collected:
 top=M/label
 for p in [top/'qualification.json',top/'resources/qualification.json']:
  if not p.is_file():continue
  value=json.loads(p.read_bytes());name=str(p.relative_to(M))
  if isinstance(value.get('results'),list)and value['results']and all('full_records'in r for r in value['results']):
   qualifications.append(dict(record=name,kind='managed',outputs=len(value['results'])*32))
  elif 'correctness_executions'in value and isinstance(value.get('records'),list):
   qualifications.append(dict(record=name,kind='compact',outputs=value['correctness_executions']))
  else:
   add('qualification-other/'+name,p.read_bytes())
work={}
for cohort in cohorts:
 for variant in read(cohort['plan'])['implementations']:
  name=variant['name'].removesuffix('-instrumented')
  if not name.startswith('mech-managed-')or name in work:continue
  path=variant.get('qualification',{})
  path=path.get('qualify-5000.json')if isinstance(path,dict)else None
  if not path:continue
  p=Path(path)
  if not p.is_file():continue
  raw=p.read_bytes();value=json.loads(raw);assert value['exit']==0
  obj=next(json.loads(line)for line in value['stdout'].splitlines()if line.startswith('{'))
  rec='control-qualifications/'+name+'/full.json';add(rec,raw)
  work[name]=dict(record=rec,sha256=sha(raw),work=obj['work'],numeric_work=obj['parallel_numeric_work'],numeric_regions=obj['parallel_numeric_regions'],exact_total_required=name!='mech-managed-typed-fixed-byte')
# Merge exact current-source authorities; earlier controls remain independently
# anchored to their own source observations, never a borrowed wide bill.
source_work=read('first-u16-host-mask-source-gains-v1/source-work.json')
for name,value in source_work.items():
 authority=value['authority'];raw=Path(authority['path']).read_bytes();assert sha(raw)==authority['sha256']
 rec='managed-work-authorities/'+name+'/authority.json';add(rec,raw)
 work[name]=dict(record=rec,sha256=sha(raw),work=value['work'],numeric_work=value['parallel_numeric_work'],numeric_regions=value['parallel_numeric_regions'],tasks=value['parallel_tasks'],batches=value['parallel_batches'],exact_total_required=True,authority_kind='retained_latency'if 'samples'in json.loads(raw)else'qualification_receipt')
managed_names=set()
for cohort in cohorts:
 for v in read(cohort['plan'])['implementations']:
  if v['kind']=='mech'and '-compact-'not in v['name']:
   managed_names.add(v['name'].removesuffix('-instrumented'))
assert managed_names==set(work),(sorted(managed_names-set(work)),sorted(set(work)-managed_names))
report=dict(schema='mech.related-post-gen.u16-host-source-evidence.v1',status='completed_evidence_with_offline_verifier',timed_resource_exact_outputs=5063,cohorts=cohorts,qualifications=qualifications,qualification_exact_outputs_separate=sum(x['outputs']for x in qualifications),managed_work_qualifications=work,
 source_closure=dict(bases={'IR349':dict(commit=BASE349,inventory_record='packed-selected-homes-compiler-v1/source-inventory.json')},compilers=closures,vendor_inventory='comparison-chain-compiler-v3/vendor-full-source-inventory.json',vendor_base_commit=BASE349,vendor_overrides=vendoroverrides,vendor_files=246,literal_inventory='typed-replay-compiler-v3/source-inventory.json',literal_includes=len(literal)),
 pin_discovery_mismatches_record='pin-discovery-mismatches.json',collected_trees=collected,
 goal_policy=dict(fixed_existing_examples=True,pinned_benchmark_revision='fc2982b0c929b1b701209d401a1dc30c773ef4ab',custom_Rust_controls='Additional frozen observations only; not completion gate. Historical raw original_20_serial_Rust reference label is not a provenance claim.',raw_historical_reports_unchanged=True,postJSON_eligibility_unchanged=True,no_further_Rust_optimization=True),
 selection=dict(promotion_claim=None,baseline_provenance_pending=False,fixed_example_goal_met_per_root=True,managed355_U16_gain_vs334_confirmed_per_root=True,managed353_primary_gain_failed_confirmation=True,compact353_U16_vs_own353U32_gain_confirmed_per_root=True,compact353_U16_vs_prior349_confirmation_interval_crosses_zero=True,compact355_confirmed_gain_claim=False,masks='V1/V2 and combinations regressed/unselected; all observations retained'),
 observations=dict(all_retained=True,whole_process_is_acceptance_gate=False,tasks=4,batches=1,observed_peak_workers='1..4; no filtering',historical_IR335_work_variation='Retained unselected total-work variation; exact numeric bill required.',resource_latency_selection_forbidden=True,static_counts='Static native counts/cold code; no dynamic spill or speed prediction.',compact_contract='Separate unmetered contract, arithmetic/indexing checked; native payload accounting distinct from tracked Rust heap/RSS.'),
 retention=dict(no_native_execution_by_assembler=True,published_dependencies_only=[BASE349],executables_libraries_omitted_with_build_hash_receipts=True,object_files_retained=True))
report['timing_tables']=[];report['resource_tables']=[]
for cohort in cohorts:
 value=read(cohort['record']);cfg=read(cohort['plan'])
 if cohort['kind']=='resources':
  report['resource_tables'].append(dict(cohort=cohort['prefix'],rows=[dict(variant=n,post_json_window=s['window_medians'].get('post_json'),whole_process=s['whole_process_medians'])for n,s in value['summary'].items()]));continue
 variants={v['name']:v for v in cfg['implementations']};reference=value.get('primary_reference',value.get('references',{}).get('current_22_eligible_Rust'));rows=[]
 for n,s in value['summary'].items():
  seconds=s.get('median_post_json_seconds');rows.append(dict(variant=n,post_json_ms=seconds*1000 if seconds is not None else None,times_extra_current_fastest_custom_Rust=seconds/reference['median_seconds']if seconds is not None else None,whole_process_ms=s['median_whole_process_seconds']*1000,code_bytes=variants[n].get('code_bytes'),stack_bytes=variants[n].get('stack_bytes'),executable_file_bytes=variants[n].get('executable_file_bytes'),whole_process_resources=s.get('process_resources_median')))
 report['timing_tables'].append(dict(cohort=cohort['prefix'],legacy_extra_reference=reference,rows=rows))
report['fixed_benchmark_provenance']='fixed-benchmark-provenance-v1/audit.json'
report['completion_authority']='first-u16-host-mask-source-gains-v1/completion.json'
report['fixed_example_comparisons']=read('first-u16-host-mask-source-gains-v1/fixed-benchmark-comparison.json')
report['selection']['claim_authority']=read(report['completion_authority'])
report['test_authority']=dict(final355_focused=79,final355_broad_passed=1108,inherited_failures=16,ignored=11,filtered=35,backend='Exact V3 in final355focused/broad test executable. Initial351 historical-backend tests retained separately; corrected fresh-backend-tests-v2 authority retained. Initial353/355unsafe-image negative assumption failures and allsource-only fixture fixes retained.')
report['source_closure_roles']={
 'selected-increment-compiler-v4':'IR350 final production and coherent tests',
 'selected-increment-clif350-compiler-v1':'IR350 diagnostic CLIF-dump build; not a timing candidate',
 'lazy-map-path-compiler-v1':'IR351 production; corrected fresh-backend-tests-v2 is coherent test authority',
 'selected-increment-lazy-map-compiler-v1':'IR352 production composition',
 'u16-arithmetic-compiler-v1':'IR353 production used for native qualifications and timings',
 'u16-arithmetic-compiler-v2':'IR353 test-only correction snapshot; production source unchanged',
 'exact-histogram-key-compiler-v1':'IR354 production',
 'u16-exact-histogram-compiler-v2':'IR355 superseded test-assumption snapshot; no native qualifications ran on it',
 'u16-exact-histogram-compiler-v3':'IR355 final coherent qualification/test authority; production behavior unchanged by cfg(test) corrections',
}
report['qualification_accounting']=dict(outputs=3040,adapters=95,later_authoritative_outputs=2912,genuine_additional_initial_IR350_outputs=128,initial_IR350_adapter_suites=4,additional_focused_full_scoring_outputs=30,separate_guard_cast_helper_calls=120)
report['independent_QA']='u16-host-source-independent-audit-v1/audit.json'
report['material_tradeoffs']=dict(
 authority='Retained current primary config/latency and independent plan/report; sizes also retained in native build/descriptor records.',
 managed334=dict(variant='mech-managed-scoped-frame',native_code_bytes=1021740,root_stack_bytes=1568,ordinary_executable_bytes=17095936,primary_peak_resident_bytes=27017216,confirmation_whole_process_ms=40.379292),
 managed355_U16=dict(variant='mech-ir355-managed-guarded-u16-4',native_code_bytes=1117996,root_stack_bytes=1696,ordinary_executable_bytes=18164960,primary_peak_resident_bytes=30015488,confirmation_whole_process_ms=48.851250),
 compact353_U16=dict(variant='mech-ir353-compact-guarded-u16-4',executable_bytes=930512,native_Object_bytes=200352,confirmation_whole_process_ms=19.878875,root_stack_comparable_to_managed=None),
 original_rust_con=dict(variant='rust-preinitialized',confirmation_whole_process_ms=17.980292),
 interpretation='Post-JSON wins do not imply whole-process, code-size, restoration or RSS wins. Compact and managed stack metadata have different contracts; no equivalence is inferred.')
(D/'report.json').write_text(json.dumps(report,indent=2)+'\n');add('packet-report.json',(D/'report.json').read_bytes());(D/'omitted-binaries.json').write_text(json.dumps(sorted(omitted),indent=2)+'\n')
manifest=dict(schema='mech.related-post-gen.deduplicated-record-shards.v1',logical_records=len(records),unique_blobs=len(blobs),compressed_bytes=sum(x['compressed_bytes']for x in blobs.values()),records=[records[n]for n in sorted(records)])
MAX_SHARD=44_000_000;containers=[];current=[];size=0
for path in sorted({r['path']for r in records.values()}):
 n=(BLOB_CACHE/path).stat().st_size
 if current and size+n+len(current)*250>MAX_SHARD:containers.append(current);current=[];size=0
 current.append(path);size+=n
if current:containers.append(current)
manifest['containers']=[]
for i,members in enumerate(containers,1):
 p=D/f'records-{i:02d}.zip'
 with zipfile.ZipFile(p,'w',compression=zipfile.ZIP_STORED)as z:
  for path in members:
   info=zipfile.ZipInfo(path,date_time=(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_STORED;info.create_system=3;info.external_attr=0o100644<<16;z.writestr(info,(BLOB_CACHE/path).read_bytes())
 raw=p.read_bytes();assert len(raw)<45_000_000
 manifest['containers'].append(dict(path=p.name,format='ZIP_STORED gzip members',sha256=sha(raw),bytes=len(raw),members=len(members)))
 member_set=set(members)
 for row in manifest['records']:
  if row['path']in member_set:row['container']=p.name
(D/'records.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps({k:v for k,v in manifest.items()if k!='records'}),flush=True)
