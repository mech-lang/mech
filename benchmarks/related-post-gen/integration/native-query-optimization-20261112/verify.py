#!/usr/bin/env python3
"""Offline source, timing, resource and correctness verification. No native execution."""
from functools import lru_cache
from pathlib import Path
import gzip
import hashlib
import json
import math
import random
import re
import statistics
import subprocess

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
report = json.loads((D / 'report.json').read_bytes())
manifest = json.loads((D / 'records.json').read_bytes())
records = {}
digests = {}
checked = set()
for row in manifest['records']:
    assert row['source'] not in records
    path = D / row['path']
    assert path.resolve().is_relative_to(D)
    if row['path'] not in checked:
        packed = path.read_bytes()
        assert len(packed) == row['compressed_bytes']
        assert hashlib.sha256(packed).hexdigest() == row['compressed_sha256']
        raw = gzip.decompress(packed)
        assert len(raw) == row['uncompressed_bytes']
        assert hashlib.sha256(raw).hexdigest() == row['uncompressed_sha256']
        checked.add(row['path'])
    records[row['source']] = row
    digests[row['uncompressed_sha256']] = row['path']
assert len(records) == manifest['logical_records']
assert len(checked) == manifest['unique_blobs']
assert sum((D / name).stat().st_size for name in checked) == manifest['compressed_bytes']
assert {str(p.relative_to(D)) for p in (D / 'records').glob('*.gz')} == checked


@lru_cache(maxsize=128)
def data(path):
    return gzip.decompress((D / path).read_bytes())


def blob(name):
    return data(records[name]['path'])


def read(name):
    return json.loads(blob(name))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def close(a, b):
    assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)


def pin(prefix, value):
    raw = blob(prefix + '/' + value['path'])
    assert sha(raw) == value['sha256'] and len(raw) == value['bytes']
    return raw


gold = read('publication-inputs/gold.json')
assert len(gold) == len(read('publication-inputs/posts.json')) == 5000
gold_digest = sha(blob('publication-inputs/gold.json'))
exact_digests = set()


def exact(raw, expected_digest=gold_digest):
    digest = sha(raw)
    assert digest == expected_digest
    if digest not in exact_digests:
        assert json.loads(raw) == gold
        exact_digests.add(digest)


TIMER = re.compile(r'^Processing time \(w/o IO\):\s*([0-9]+(?:\.[0-9]+)?)\s*(ns|µs|μs|us|ms|s)\s*$', re.MULTILINE)
UNITS = {'ns': 1e-9, 'µs': 1e-6, 'μs': 1e-6, 'us': 1e-6, 'ms': 1e-3, 's': 1}


def elapsed(variant, stdout):
    if variant['kind'] == 'mech':
        values = [json.loads(line) for line in stdout.splitlines() if line.startswith('{')]
        values = [v for v in values if 'post_json_seconds' in v]
        assert len(values) == 1 and values[0]['posts'] == 5000 and values[0]['sample'] == 0
        result = values[0]['post_json_seconds']
        assert 1 <= values[0]['parallel_peak_workers'] <= 4
        assert values[0]['parallel_batches'] == 1
        if 'parallel_tasks' in values[0]:
            assert values[0]['parallel_tasks'] == 4
    else:
        values = TIMER.findall(stdout)
        assert len(values) == 1
        result = float(values[0][0]) * UNITS[values[0][1]]
    assert math.isfinite(result) and result > 0
    return result


def process(text):
    result = {}
    for line in text.splitlines():
        tokens = line.strip().split()
        if len(tokens) == 2 and tokens[0] in ('real', 'user', 'sys'):
            result[{'real': 'wall_seconds', 'user': 'user_cpu_seconds', 'sys': 'system_cpu_seconds'}[tokens[0]]] = float(tokens[1])
        elif tokens and tokens[0].isdigit():
            key = '_'.join(tokens[1:])
            key = {'maximum_resident_set_size': 'peak_resident_bytes',
                   'peak_memory_footprint': 'peak_physical_footprint_bytes',
                   'cycles_elapsed': 'cycles'}.get(key, key)
            result[key] = int(tokens[0])
    return result


def medians(rows):
    result = {}
    for key in sorted(set.intersection(*(set(r) for r in rows))):
        values = [r[key] for r in rows]
        if all(isinstance(v, dict) for v in values):
            result[key] = medians(values)
        elif all(type(v) in (int, float) for v in values):
            result[key] = statistics.median(values)
    return result


def orders(variants, seed, rounds):
    rng = random.Random(seed)
    result = []
    for _ in range(rounds):
        names = [v['name'] for v in variants]
        rng.shuffle(names)
        result.append(names)
    return result


# Only packet files and published Git objects are read below. Historical absolute
# paths are opaque provenance labels, resolved by their archived content digest.
def pinned(value):
 raw=data(digests[value['sha256']]);assert sha(raw)==value['sha256']
 if 'bytes'in value:assert len(raw)==value['bytes']
 return raw

def ledger(name,value):
 assert value['posts']==5000 and value['sample']==0
 assert value['parallel_batches']==1 and 1<=value['parallel_peak_workers']<=4
 q=report['managed_work_qualifications'].get(name)
 if q:
  assert value['parallel_tasks']==4
  assert value['parallel_numeric_work']==q['numeric_work']
  assert value['parallel_numeric_regions']==q['numeric_regions']
  if q['exact_total_required']:assert value['work']==q['work'],name

def jstdout(stdout):return next(json.loads(line)for line in stdout.splitlines()if line.startswith('{'))
for name,q in report['managed_work_qualifications'].items():
 raw=blob(q['record']);assert sha(raw)==q['sha256'];r=json.loads(raw);assert r['exit']==0;value=jstdout(r['stdout']);ledger(name,value);assert value['work']==q['work']
assert [n for n,q in report['managed_work_qualifications'].items()if not q['exact_total_required']]==['mech-managed-typed-fixed-byte']

primary=read(report['primary']['record']);config=read(report['primary']['config']);protocol=read('first-native-query-gains/protocol.json');variants={v['name']:v for v in primary['implementations']}
assert len(variants)==51 and len(primary['samples'])==1581
assert primary['execution_order']['rounds']==orders(config['implementations'],protocol['seed'],31)
times={};indexed={};observations={}
def observe(cohort,name,value):
 if 'parallel_tasks'in value:ledger(name,value)
 else:assert value['parallel_batches']==1 and 1<=value['parallel_peak_workers']<=4
 row=observations.setdefault(cohort,{}).setdefault(name,{'peak_worker_histogram':{},'total_work_histogram':{}})
 for field,key in [('parallel_peak_workers','peak_worker_histogram'),('work','total_work_histogram')]:
  if field in value:s=str(value[field]);row[key][s]=row[key].get(s,0)+1
for i,order in enumerate(primary['execution_order']['rounds']):assert [r['variant']for r in primary['samples']if r['sample']==i]==order
for row in primary['samples']:
 name=row['variant'];assert row['exit']==0 and row['exact_rust_output']is True
 exact(blob(f'first-native-query-gains/runs/{name}-{row["sample"]}.json'),row['output_sha256']);value=elapsed(variants[name],row['stdout']);indexed[row['sample'],name]=value;times.setdefault(name,[]).append(value)
 if variants[name]['kind']=='mech':native=jstdout(row['stdout']);observe('primary',name,native);close(value,row['post_json_seconds'])
 else:close(value,row['seconds'])
 assert process(blob('first-native-query-gains/runs/'+Path(row['process_resources_raw']).name).decode())==row['process_resources']
med={name:statistics.median(v)for name,v in times.items()}
for name,v in variants.items():
 rows=[r for r in primary['samples']if r['variant']==name];s=primary['summary'][name];assert s['samples']==31 and s['all_exact_rust']is True
 close(s['median_seconds'],statistics.median(r['seconds']for r in rows));close(s['median_whole_process_seconds'],statistics.median(r['wall_seconds']for r in rows));assert s['process_resources_median']==medians([r['process_resources']for r in rows])
 if v.get('calculation_boundary')=='post_json_including_setup':close(s['median_post_json_seconds'],med[name])
eligible=[v['name']for v in variants.values()if v['kind']!='mech'and v.get('calculation_boundary')=='post_json_including_setup'];assert len(eligible)==22
reference=min(eligible,key=med.__getitem__);assert reference==primary['primary_reference']['variant'];close(med[reference],primary['primary_reference']['median_seconds'])
paired=read(report['primary']['paired']);assert paired['report_sha256']==sha(blob(report['primary']['record']))
for c in paired['comparisons']:
 values=[100*(indexed[i,c['candidate']]/indexed[i,c['control']]-1)for i in range(31)];close(statistics.median(values),c['median_percent']);assert c['candidate_faster_pairs']==sum(v<0 for v in values);rng=random.Random(c['bootstrap_seed']);draws=sorted(statistics.median(rng.choices(values,k=31))for _ in range(10000))
 for a,b in zip([draws[250],draws[9749]],c['percentile95']):close(a,b)
 assert c['gain_established']==(c['percentile95'][1]<0);assert c['regression_established']==(c['percentile95'][0]>0)

prefix='new-managed-resources/resources-query-first';resources=read(report['resources']['record']);plan=read(report['resources']['plan']);resource_variants={v['name']:v for v in plan['implementations']}
assert len(resource_variants)==49 and len(resources['samples'])==resources['exact_outputs']==343
assert sha(blob(report['resources']['plan']))==resources['plan_sha256'];assert plan['rounds']==orders(plan['implementations'],plan['seed'],7)
assert resources['samples']==[json.loads(x)for x in blob(prefix+'/samples.jsonl').splitlines()]
assert resources['latency_selection_forbidden']is True and resources['whole_process_is_acceptance_gate']is False
for i,order in enumerate(plan['rounds']):assert [r['variant']for r in resources['samples']if r['round']==i]==order
for row in resources['samples']:
 assert row['exit']==0 and row['timed_out']is False and row['exact_output']is True;exact(pin(prefix,row['output']));stdout=pin(prefix,row['stdout']).decode();pin(prefix,row['stderr']);assert process(pin(prefix,row['process_resources_raw']).decode())==row['whole_process_counters'];v=resource_variants[row['variant']]
 if v['kind']=='mech':
  native=jstdout(stdout);assert native==row['raw_adapter_record'];observe('resources',v.get('latency_control_name',v['name'].removesuffix('-instrumented')),native);assert row['windows']['post_json']==native['measurement'];close(row['instrumented_post_json_seconds_diagnostic_only'],native['post_json_seconds'])
  for raw,label in [('restore_measurement','restore'),('json_measurement','json'),('admission_measurement','admission')]:
   if raw in native:assert row['windows'][label]==native[raw]
 elif v['instrumented']:
  native=json.loads(next(line[len('PROCESS_METRICS '):]for line in stdout.splitlines()if line.startswith('PROCESS_METRICS ')));assert native==row['raw_adapter_record'];label='post_json'if v['post_json_counter_eligible']else'excluded_processing';assert row['windows'][label]==native.get('processing',native)
 for window in row['windows'].values():
  assert window['allocator_accounting_underflows']==window['allocator_lifetime_accounting_underflows']==0;assert window['hardware']['instructions_retired']>0 and window['hardware']['cycles']>0
for name,s in resources['summary'].items():
 rows=[r for r in resources['samples']if r['variant']==name];assert len(rows)==s['samples']==7 and s['all_exact']is True;assert medians([r['whole_process_counters']for r in rows])==s['whole_process_medians']
 for window,value in s['window_medians'].items():assert medians([r['windows'][window]for r in rows])==value

prefix='independent-managed-query-comparison-prepared-v3';confirmation=read(report['confirmation']['record']);cplan=read(report['confirmation']['plan']);cvars={v['name']:v for v in cplan['implementations']};ci={}
assert len(cvars)==30 and len(confirmation['samples'])==confirmation['exact_outputs']==390
assert cplan['rounds']==orders(cplan['implementations'],cplan['seed'],13);assert sha(blob(report['confirmation']['plan']))==confirmation['plan_sha256'];assert confirmation['same_machine_independent_orchestration']is True;assert confirmation['whole_process_is_acceptance_gate']is False
assert confirmation['samples']==[json.loads(x)for x in blob(prefix+'/samples.jsonl').splitlines()]
for i,order in enumerate(cplan['rounds']):assert [r['variant']for r in confirmation['samples']if r['round']==i]==order
for row in confirmation['samples']:
 assert row['exit']==0 and row['timed_out']is False and row['exact_output']is True;exact(pin(prefix,row['output']));stdout=pin(prefix,row['stdout']).decode();pin(prefix,row['stderr']);name=row['variant'];v=cvars[name];value=elapsed(v,stdout);close(row['whole_process_seconds'],row['whole_process_nanoseconds']/1e9)
 if v['kind']=='mech':observe('confirmation',name,jstdout(stdout))
 if row['post_json_eligible']:close(value,row['post_json_seconds']);ci[row['round'],name]=value
 else:assert row['post_json_seconds']is None
for name,s in confirmation['summary'].items():
 rows=[r for r in confirmation['samples']if r['variant']==name];assert len(rows)==s['samples']==s['exact_outputs']==13;close(s['median_whole_process_seconds'],statistics.median(r['whole_process_seconds']for r in rows))
 if s['post_json_eligible']:close(s['median_post_json_seconds'],statistics.median(r['post_json_seconds']for r in rows))
for label,ref in confirmation['references'].items():
 metric='median_whole_process_seconds'if label.startswith('whole_process')else'median_post_json_seconds';winner=min(ref['eligible_variants'],key=lambda n:confirmation['summary'][n][metric]);assert winner==ref['variant'];close(ref['median_seconds'],confirmation['summary'][winner][metric])
def confirmation_pair(c,candidate):
 values=[100*(ci[i,candidate]/ci[i,c['reference']]-1)for i in range(13)];assert c['paired_percent_changes']==values;close(c['median_paired_percent_change'],statistics.median(values));assert c['candidate_faster_pairs']==sum(x<0 for x in values);rng=random.Random(c['bootstrap_seed']);draws=sorted(statistics.median(rng.choices(values,k=13))for _ in range(c['bootstrap_draws']))
 for a,b in zip([draws[249],draws[9749]],c['bootstrap_95_interval_percent']):close(a,b)
for c in confirmation['candidate_paired_comparisons']:confirmation_pair(c,confirmation['candidate'])
additional=read(report['confirmation']['additional_paired']);assert additional['report_sha256']==sha(blob(report['confirmation']['record']))
for c in additional['comparisons']:
 candidates=[name for name in cvars if all((i,name)in ci for i in range(13))and [100*(ci[i,name]/ci[i,c['reference']]-1)for i in range(13)]==c['paired_percent_changes']];assert candidates==['mech-managed-dense-comparison-wide'];confirmation_pair(c,candidates[0])
assert confirmation['target_met_against_current_fastest_Rust']is False

def edge_set(prefix):
 edges=read(prefix+'/edge-fixtures.json');assert len(edges)==31
 for edge in edges:
  assert edge['exit']==0;name=edge['fixture']
  if edge.get('exact_rust_output'):assert read(prefix+'/edges/'+name+'/actual.json')==read(prefix+'/edges/'+name+'/related_posts_rust.json')
  else:
   assert edge['exact_expected_output']is True;posts=read('published-base/benchmarks/related-post-gen/fixtures/posts-'+name+'.json');indices=read('published-base/benchmarks/related-post-gen/fixtures/expected-'+name+'.json')['related_indices'];expected=[{'_id':p['_id'],'tags':p['tags'],'related':[posts[i]for i in neighbors]}for p,neighbors in zip(posts,indices)];assert read(prefix+'/actual-'+name+'.json')==expected

edge_prefixes=[n.removesuffix('/edge-fixtures.json')for n in records if n.endswith('/edge-fixtures.json')and any(n.startswith(p)for p in ['producer-element-qualified-v1/','comparison-chain-wide/','comparison-chain-u32/','dense-comparison-qualified-v1/','dense-producer-comparison-qualified-v1/','dense-producer-qualified-v1/'])]
assert len(edge_prefixes)==20
for prefix in edge_prefixes:edge_set(prefix)
full_count=0

def checked_audit(value):
 audit=json.loads(pinned(value));assert audit['status']=='passed';assert audit['every_native_symbol_has_exact_object_relative_address']is True;assert audit['all_text_equal_except_independently_verified_relocation_fields']is True;assert audit['object']['sha256']in digests;return audit

for name in report['qualification']['other_preparations']:
 q=read(name);assert len(q['results'])==4
 for row in q['results']:
  assert row['full_records']==5000 and row['exact_gold']is True;exact(pinned(row['output']));full_count+=1;audit=checked_audit(row['link_audit']);receipt=json.loads(pinned(row['build_receipt']));assert receipt['exit']==0;assert receipt['executable_sha256']==row['executable']['sha256']==audit['executable']['sha256'];assert all(v['sha256']in digests for v in receipt['bundle'].values())
q=read(report['qualification']['IR341_preparation']);assert q['status']=='passed'and len(q['variants'])==2
for variant in q['variants']:
 assert len(variant['adapters'])==2
 for row in variant['adapters']:
  exact(pinned(row['full_output']));full_count+=1;audit=checked_audit(row['same_object_link_audit']);receipt=json.loads(pinned(row['build_receipt']));assert receipt['exit']==0;assert receipt['executable_sha256']==row['executable']['sha256']==audit['executable']['sha256'];assert all(v['sha256']in digests for v in receipt['bundle'].values());assert json.loads(pinned(row['qualification']))['exit']==json.loads(pinned(row['edge_qualification']))['exit']==0
assert full_count==20 and full_count*(31+1)==report['qualification']['new_adapter_exact_outputs']==640

def counts(value):
 matches=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',value['stdout']);assert len(matches)==1;return tuple(map(int,matches[0]))
broad=read(report['qualification']['broad']);baseline=read('scoped-frame-compiler-v1/broad-native-tests.json');assert broad['exit']==baseline['exit']==101;assert counts(broad)==(1088,16,11)and counts(baseline)==(1056,16,11);assert '35 filtered out'in broad['stdout']
failures=lambda v:sorted(re.findall(r'^    (native_artifact::[^\n]+)$',v['stdout'],re.MULTILINE))
assert len(failures(broad))==16 and failures(broad)==failures(baseline)
distinct=read('dense-producer-comparison-compiler-v1/focused-distinct-tests.json');assert distinct['count']==len(set(distinct['passed']))==37

def stdout_records(value):
 if isinstance(value,dict):
  if 'stdout'in value and isinstance(value['stdout'],str):yield value
  for v in value.values():yield from stdout_records(v)
 elif isinstance(value,list):
  for v in value:yield from stdout_records(v)
focused=list(stdout_records(read('dense-producer-comparison-compiler-v1/focused-contracts.json')));assert focused and all(x['exit']==0 for x in focused);passed={n for value in focused for n in re.findall(r'^test (\S+) \.\.\. ok$',value['stdout'],re.MULTILINE)};assert passed==set(distinct['passed'])
native=read(report['qualification']['primary_native_contracts']);assert native['status']=='passed';n=json.loads(native['native_record']['stdout']);assert n=={'native_functions':463,'native_correctness_calls':1787824,'functions_with_ccmp':440,'CCMP_instructions':680,'all_exact':True,'native_benchmark_executions':0};assert native['native_record']['exit']==native['matcher_record']['exit']==0;assert counts(native['matcher_record'])==(4,0,0)
supplement=read(report['qualification']['constant_supplement']);n=supplement['result'];assert n['native_functions']==568 and n['native_correctness_calls']==59328 and n['all_exact']is True and n['pressure_functions']==8;assert n['functions_with_ccmp']==568 and n['CCMP_instructions']==1704;assert native['native_source_sha256']==sha(blob('comparison-chain-candidate-v3/native-correctness.rs'));assert sha(pinned(supplement['source']))==supplement['source']['sha256']
assert 463+568==report['qualification']['native_functions']==1031;assert 1787824+59328==report['qualification']['native_correctness_calls']==1847152
# Failed ISLE purity and two-producer designs are retained as failed candidates.
assert read('comparison-chain-compiler-v1/backend-build.json')['exit']==101
assert read('comparison-chain-backend-contracts-v1/native-tests.json')['exit']!=0
wrapper=read('dense-producer-comparison-compiler-v1/tests-build-initial-wrapper-failure.json');assert wrapper
for prefix in ['independent-managed-query-comparison-prepared','independent-managed-query-comparison-prepared-v2']:
 failure=read(prefix+'/harness-preflight-failure.json');assert failure['native_executions']==0;assert not any(n.startswith(prefix+'/runs/')for n in records)
assert read('independent-managed-query-comparison-prepared-v2/harness-preflight-failure.json')['exact_outputs']==0

# Complete source closures against two reachable, published Git commits.
import difflib

def git_files(commit,names,missing=False):
 raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,check=True,capture_output=True,input=''.join(commit+':'+n+'\n'for n in names).encode()).stdout;cursor=0;result={}
 for name in names:
  end=raw.index(b'\n',cursor);header=raw[cursor:end].split();cursor=end+1
  if header[-1]==b'missing':assert missing,(commit,name);continue
  assert len(header)==3 and header[1]==b'blob',(commit,name,header);size=int(header[2]);result[name]=raw[cursor:cursor+size];cursor+=size;assert raw[cursor:cursor+1]==b'\n';cursor+=1
 assert cursor==len(raw);return result
closure=report['source_closure'];bases={}
for label,base in closure['bases'].items():
 inv=read(base['inventory_record']);source=git_files(base['commit'],sorted(inv['files']));assert len(source)==len(inv['files']);assert all(sha(source[n])==h for n,h in inv['files'].items());bases[label]=source
reconstructed={};builds={}
for label,entry in closure['compilers'].items():
 inv=read(entry['inventory_record']);assert sha(json.dumps(inv['files'],sort_keys=True).encode())==inv['sha256'];base=bases[entry['base']];source={};parts=[]
 for name,expected in inv['files'].items():
  raw=blob('source-overrides/'+label+'/'+name)if name in entry['overrides']else base[name];assert sha(raw)==expected,(label,name);source[name]=raw
 assert {n:sha(raw)for n,raw in source.items()if base.get(n)!=raw}==entry['overrides'];assert len(source)==entry['source_count']
 for name in entry['overrides']:
  parts.append('diff --git a/'+name+' b/'+name+'\n');parts.extend(difflib.unified_diff(base.get(name,b'').decode().splitlines(True),source[name].decode().splitlines(True),fromfile='a/'+name if name in base else'/dev/null',tofile='b/'+name))
 assert ''.join(parts).encode()==blob(entry['patch_record']);reconstructed[label]=source;build=read(label+'/build.json');assert build['source_sha256']==inv['sha256'];assert len(build['stages'])==12 and all(s['exit']==0 for s in build['stages']);builds[label]=build
 IR={'producer-element-compiler-v1':339,'comparison-chain-compiler-v3':341,'dense-comparison-compiler-v2':342,'dense-producer-comparison-compiler-v1':343,'dense-producer-compiler-v1':344}[label];identity=source['hosts/gpu/src/native_artifact/program/image/identity.rs'].decode();assert re.search(r'NATIVE_PROGRAM_TYPED_IR_VERSION\s*:\s*u64\s*=\s*'+str(IR)+r'\s*;',identity)
vendor=read(closure['vendor_inventory']);base=git_files(closure['vendor_base_commit'],sorted(vendor),missing=True)
for name,h in vendor.items():
 raw=blob('vendor-overrides/'+name)if name in closure['vendor_overrides']else base[name];assert sha(raw)==h
 for label in ['comparison-chain-compiler-v3','dense-comparison-compiler-v2','dense-producer-comparison-compiler-v1']:
  if name in reconstructed[label]:assert reconstructed[label][name]==raw
literal=read(closure['literal_inventory'])['literal_includes'];assert len(literal)==closure['literal_includes']==164
for name,raw in git_files(closure['vendor_base_commit'],sorted(literal)).items():assert sha(raw)==literal[name]
fresh=read('comparison-chain-compiler-v3/backend-libraries.json')
def externs(command):return dict(command[i+1].split('=',1)for i,a in enumerate(command[:-1])if a=='--extern')
for label,build in builds.items():
 gpu=next(x for x in build['stages']if x['crate']=='mech_gpu');non_gpu={x['crate']:x for x in build['stages']if x['crate']!='mech_gpu'};parent=read('owned-capture-compiler-v1/build.json');parent_libs={x['crate']:x for x in parent['stages']}
 for name,stage in non_gpu.items():assert stage['library_sha256']==parent_libs[name]['library_sha256']
 actual=externs(gpu['command'])
 if label in ['comparison-chain-compiler-v3','dense-comparison-compiler-v2','dense-producer-comparison-compiler-v1']:
  assert build['backend_libraries']==fresh
  for name in ['cranelift_codegen','cranelift_frontend','cranelift_jit','cranelift_module','cranelift_native','cranelift_object']:assert actual[name]==fresh[name]['path']
 else:
  baseline_externs=externs(next(x for x in parent['stages']if x['crate']=='mech_gpu')['command'])
  for name in ['cranelift_codegen','cranelift_frontend','cranelift_jit','cranelift_module','cranelift_native','cranelift_object']:assert actual[name]==baseline_externs[name]
test_build=read('dense-producer-comparison-compiler-v1/tests.build.json');assert test_build['exit']==0 and test_build['all_backend_externs_from_fresh_family']is True
for name in ['cranelift_codegen','cranelift_frontend','cranelift_jit','cranelift_module','cranelift_native','cranelift_object']:assert externs(test_build['command'])[name]==fresh[name]['path']
# Raw machine audits tie the source proofs to exact compiler/Object bytes.
for name in ['comparison-chain-machine-audit-v3/audit.json','comparison-compositions-machine-audit-v1/audit.json']:
 audit=read(name);assert audit
for row in read('comparison-chain-machine-audit-v3/audit.json')['records']:
 assert len(row['five_predicate_filter_copies'])==2
 for gate in row['five_predicate_filter_copies']:assert gate['counts']=={'cmp':1,'ccmp':4,'branch':1}and len(gate['instructions'])==6
for row in read('comparison-chain-machine-audit-v3/audit.json')['pressure_register_allocation']:
 assert row['stack_load_store_count']>0 and row['no_intervening_flag_clobber_or_operand_materialization']is True
# Fresh sampled profiles retain the failed first sampler and retry; their
# arithmetic averages are diagnostics, never entered into timing authority.
profile=read('dense-worker-fullpath-profile-v1/complete.json');analysis=read('dense-worker-fullpath-profile-v1/analysis.json');assert profile and analysis
for name in records:
 if name.startswith('dense-worker-fullpath-profile-v1/')and name.endswith('/final-output.json'):exact(blob(name))
assert blob('dense-worker-fullpath-profile-v1/REPORT.md')
# Validate source extraction against the real compiled backend source, not a shim.
extraction=read('comparison-chain-backend-contracts-v3/source-extraction.json')
matcher_source=reconstructed['comparison-chain-compiler-v3']['vendor/cranelift-codegen/src/isa/aarch64/lower/boolean_conjunction.rs']
matcher_tests=reconstructed['comparison-chain-compiler-v3']['vendor/cranelift-codegen/src/isa/aarch64/lower/boolean_conjunction/tests.rs']
assert sha(matcher_source)==extraction['production_source_sha256']
assert sha(matcher_source.split(b'\n/// Flags that make')[0])==extraction['extracted_source_prefix_sha256']
assert sha(matcher_tests)==extraction['production_test_source_sha256']
assert sha(matcher_tests.split(b'\n// Independent architectural')[0])==extraction['exact_test_prefix_sha256']
assert sha(blob('comparison-chain-backend-contracts-v3/matcher-contracts.rs'))==extraction['exact_test_prefix_sha256']
assert wrapper['compiler_started'] is False and wrapper['native_executions']==0
for name,value in read('dense-worker-fullpath-profile-v1/analysis-receipt.json')['pins'].items():
 raw=blob('dense-worker-fullpath-profile-v1/'+name);assert sha(raw)==value['sha256'] and len(raw)==value['bytes']
assert profile['timing_claim'] is False
for run in profile['runs']:
 measurement=run['measurement'];assert run['exit']==0 and run['sampler_exit']==0 and run['diagnostic_only'] is True and run['nonquiet_concurrent_build_test_load'] is True
 assert measurement['iterations']==5001 and measurement['fresh_worker_tasks_each_iteration']==4
 assert sum(measurement['observed_peak_worker_histogram'].values())==5001
 assert set(measurement['observed_peak_worker_histogram'])<=set('1234')
 control='mech-managed-scoped-frame' if run['label']=='ir334-scoped-frame' else 'mech-managed-dense-worker-u32'
 assert measurement['native_work_per_iteration']==report['managed_work_qualifications'][control]['work']
 assert measurement['native_work_all_iterations']==measurement['native_work_per_iteration']*5001
# Every plan's archived content pin is checked without opening its historical path.
omitted=set(json.loads((D/'omitted-binaries.json').read_bytes()))
def verify_pins(value):
 if isinstance(value,dict):
  if isinstance(value.get('path'),str) and isinstance(value.get('sha256'),str):
   if value['sha256'] in digests:pinned(value)
   else:assert value['path'] in omitted,value['path']
  for child in value.values():verify_pins(child)
 elif isinstance(value,list):
  for child in value:verify_pins(child)
verify_pins(plan);verify_pins(cplan)
assert sha(blob('new-managed-resources/run-with-compact.py'))==plan['script']['sha256']
assert sha(blob('independent-managed-confirm-v6.py'))==confirmation['script_sha256']==cplan['script']['sha256']
for label in ['comparison-chain-compiler-v3','dense-comparison-compiler-v2','dense-producer-comparison-compiler-v1']:
 assert builds[label]['backend_build_sha256']==sha(blob('comparison-chain-compiler-v3/backend-build.json'))
assert sha(blob('packet-report.json'))==sha((D/'report.json').read_bytes())
assert report['timed_exact_outputs']==1581+343+390==2314
assert report['selection']['new_selected']is False and report['selection']['selected_checkpoint_IR']==334
assert report['selection']['fastest_Rust_target_met']is False
print(json.dumps({'verified_records':len(records),'unique_blobs':len(checked),'compressed_bytes':manifest['compressed_bytes'],'primary_exact_outputs':1581,'resource_exact_outputs':343,'independent_exact_outputs':390,'new_timed_exact_outputs':2314,'new_adapter_qualification_outputs':640,'compiler_configurations':5,'native_functions':1031,'native_correctness_calls':1847152,'literal_includes':164,'broad_passed':1088,'same_inherited_failures':16,'focused_distinct_passes':37,'source_overlay_files':{k:len(v['overrides'])for k,v in closure['compilers'].items()},'managed_observation_histograms':observations,'native_benchmark_executions':0},indent=2))
