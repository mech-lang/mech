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


primary = read(report['primary']['record'])
prefix = 'first-dense-worker'
config = read(prefix + '/config.json')
variants = {v['name']: v for v in primary['implementations']}
assert len(variants) == 41 and len(primary['samples']) == 1271
assert primary['execution_order']['rounds'] == orders(config['implementations'], report['primary']['seed'], 31)
managed_work = report['primary']['managed_work_qualifications']
assert [n for n,q in managed_work.items() if not q['exact_total_required']] == ['mech-managed-typed-fixed-byte']
for n,q in managed_work.items():
    raw = blob(q['record']); assert sha(raw) == q['sha256']
    value = read(q['record']); assert value['exit'] == 0
    x = next(json.loads(s) for s in value['stdout'].splitlines() if s.startswith('{'))
    for key in ('work', 'parallel_numeric_work', 'parallel_numeric_regions'): assert x[key] == q[key]
    assert x['posts'] == 5000 and x['parallel_tasks'] == 4 and x['parallel_batches'] == 1
histograms = {}
def track(rows, kind, decode):
    result = {}
    for row in rows:
        n = row['variant'].removesuffix('-instrumented')
        if n not in managed_work: continue
        x = decode(row); q = managed_work[n]
        assert x['parallel_tasks'] == 4 and x['parallel_batches'] == 1 and 1 <= x['parallel_peak_workers'] <= 4
        assert x['parallel_numeric_work'] == q['parallel_numeric_work'] and x['parallel_numeric_regions'] == q['parallel_numeric_regions']
        if q['exact_total_required']: assert x['work'] == q['work'], (kind,n)
        out = result.setdefault(n, {'total_work':{},'peak_workers':{}})
        for field,key in [('work','total_work'),('parallel_peak_workers','peak_workers')]:
            k=str(x[field]); out[key][k]=out[key].get(k,0)+1
    assert result == report['observed_histograms'][kind]
    histograms[kind] = result
jsonline = lambda text: next(json.loads(s) for s in text.splitlines() if s.startswith('{'))
track(primary['samples'], 'primary', lambda r:jsonline(r['stdout']))
paired_rows = {}
for i,order in enumerate(primary['execution_order']['rounds']):
    assert [r['variant'] for r in primary['samples'] if r['sample']==i] == order
for row in primary['samples']:
    n=row['variant']; assert row['exit']==0 and row['exact_rust_output'] is True
    exact(blob(f'{prefix}/runs/{n}-{row["sample"]}.json'),row['output_sha256'])
    value=elapsed(variants[n],row['stdout'])
    close(value,row['post_json_seconds'] if variants[n]['kind']=='mech' else row['seconds'])
    paired_rows[row['sample'],n]=value
    assert process(blob(prefix+'/runs/'+Path(row['process_resources_raw']).name).decode())==row['process_resources']
timings={}
for n,v in variants.items():
    rows=[r for r in primary['samples'] if r['variant']==n]; x=primary['summary'][n]
    assert len(rows)==x['samples']==31 and x['all_exact_rust'] is True
    close(x['median_seconds'],statistics.median(r['seconds'] for r in rows))
    close(x['median_whole_process_seconds'],statistics.median(r['wall_seconds'] for r in rows))
    assert x['process_resources_median']==medians([r['process_resources'] for r in rows])
    timings[n]=statistics.median(paired_rows[i,n] for i in range(31))
    if v.get('calculation_boundary')=='post_json_including_setup': close(x['median_post_json_seconds'],timings[n])
eligible=[n for n,v in variants.items() if v['kind']!='mech' and v.get('calculation_boundary')=='post_json_including_setup']
assert len(eligible)==22
reference=min(eligible,key=timings.__getitem__)
assert reference==primary['primary_reference']['variant']==report['primary']['current_fastest_Rust']['variant']
close(timings[reference],primary['primary_reference']['median_seconds'])
for row in report['primary']['rows']:
    n=row['variant']; close(row['post_json_ms'],timings[n]*1000);close(row['times_current_fastest_Rust'],timings[n]/timings[reference])
    close(row['whole_process_ms'],primary['summary'][n]['median_whole_process_seconds']*1000)
    for k in ('code_bytes','stack_bytes','executable_file_bytes'):assert row[k]==variants[n].get(k)
    assert row['whole_process_peak_resident_bytes']==primary['summary'][n]['process_resources_median'].get('peak_resident_bytes')
paired=read(report['primary']['paired']); rng=random.Random(paired['seed'])
assert paired['seed']==report['primary']['bootstrap_seed']
for c in paired['comparisons']:
    changes=[100*(paired_rows[i,c['candidate']]/paired_rows[i,c['control']]-1) for i in range(31)]
    close(statistics.median(changes),c['median_percent']);assert c['faster_pairs']==sum(v<0 for v in changes) and c['pairs']==31
    draws=sorted(statistics.median(rng.choices(changes,k=31)) for _ in range(10000))
    for a,b in zip((draws[250],draws[9749]),c['ci95_percent']):close(a,b)
run=read(prefix+'/run.json'); assert run['exit']==0
for key,name in [('config_sha256','config.json'),('protocol_sha256','protocol.json')]:assert run[key]==sha(blob(prefix+'/'+name))
assert run['script_sha256']==sha(blob('base-commit/benchmarks/related-post-gen/compare-implementations.py'))

prefix='new-managed-resources/resources-dense-first'
resources=read(report['resources']['record']);plan=read(report['resources']['plan'])
assert sha(blob(report['resources']['plan']))==resources['plan_sha256']
assert plan['rounds']==orders(plan['implementations'],plan['seed'],7)
assert len(resources['samples'])==resources['exact_outputs']==273
assert resources['samples']==[json.loads(s) for s in blob(prefix+'/samples.jsonl').splitlines()]
assert resources['latency_selection_forbidden'] is True and resources['whole_process_is_acceptance_gate'] is False
resource_variants={v['name']:v for v in plan['implementations']};assert len(resource_variants)==39
for i,order in enumerate(plan['rounds']):assert [r['variant'] for r in resources['samples'] if r['round']==i]==order
for row in resources['samples']:
    assert row['exit']==0 and row['timed_out'] is False and row['exact_output'] is True
    exact(pin(prefix,row['output']));stdout=pin(prefix,row['stdout']).decode();pin(prefix,row['stderr'])
    assert process(pin(prefix,row['process_resources_raw']).decode())==row['whole_process_counters']
    v=resource_variants[row['variant']]
    if v['kind']=='mech':
        x=jsonline(stdout);assert x==row['raw_adapter_record']; assert row['windows']['post_json']==x['measurement']
        close(row['instrumented_post_json_seconds_diagnostic_only'],x['post_json_seconds'])
        for raw,label in [('restore_measurement','restore'),('json_measurement','json'),('admission_measurement','admission')]:
            if raw in x:assert row['windows'][label]==x[raw]
    elif v['instrumented']:
        x=json.loads(next(s[len('PROCESS_METRICS '):] for s in stdout.splitlines() if s.startswith('PROCESS_METRICS ')))
        assert x==row['raw_adapter_record']; label='post_json' if v['post_json_counter_eligible'] else 'excluded_processing';assert row['windows'][label]==x.get('processing',x)
    for w in row['windows'].values():
        assert w['allocator_accounting_underflows']==w['allocator_lifetime_accounting_underflows']==0
        assert w['hardware']['instructions_retired']>0 and w['hardware']['cycles']>0
track(resources['samples'],'resources',lambda r:r['raw_adapter_record'])
for n,x in resources['summary'].items():
    rows=[r for r in resources['samples'] if r['variant']==n];assert len(rows)==x['samples']==7 and x['all_exact'] is True
    assert medians([r['whole_process_counters'] for r in rows])==x['whole_process_medians']
    for window,value in x['window_medians'].items():assert medians([r['windows'][window] for r in rows])==value
for row in report['resources']['rows']:
    x=resources['summary'][row['variant']];w=x['window_medians']['post_json']
    for k in ('cycles','instructions_retired'):assert row[k]==w['hardware'][k]
    for k in ('allocation_requests','requested_allocation_bytes','peak_additional_live_requested_heap_bytes'):assert row[k]==w[k]
    assert row['whole_process_peak_resident_bytes']==x['whole_process_medians']['peak_resident_bytes']
for source in ('wide','u32'):
    parent='mech-managed-owned-capture'+('-u32' if source=='u32' else '-wide')+'-instrumented'
    candidate='mech-managed-dense-worker'+('-u32' if source=='u32' else '-wide')+'-instrumented'
    a=resources['summary'][parent]['window_medians']['post_json'];b=resources['summary'][candidate]['window_medians']['post_json']
    saving=report['physical_storage'][source]['four_worker_bank_saving_bytes']
    for k in ('requested_allocation_bytes','peak_additional_live_requested_heap_bytes'):assert a[k]-b[k]==saving
    assert a['allocation_requests']==b['allocation_requests']==725

prefix='independent-managed-dense-u32-prepared'
confirmation=read(report['confirmation']['record']);cp=read(report['confirmation']['plan'])
assert sha(blob(report['confirmation']['plan']))==confirmation['plan_sha256']
assert len(confirmation['samples'])==confirmation['exact_outputs']==confirmation['expected_outputs']==351
assert cp['rounds']==orders(cp['implementations'],cp['seed'],13)
cv={v['name']:v for v in cp['implementations']};assert len(cv)==27 and sum(v['kind']!='mech' for v in cv.values())==24
ct={}
for i,order in enumerate(cp['rounds']):assert [r['variant'] for r in confirmation['samples'] if r['round']==i]==order
for row in confirmation['samples']:
    n=row['variant'];assert row['exit']==0 and row['timed_out'] is False and row['exact_output'] is True
    exact(pin(prefix,row['output']));out=pin(prefix,row['stdout']).decode();pin(prefix,row['stderr'])
    value=elapsed(cv[n],out);close(value,row['post_json_seconds'] if row['post_json_eligible'] else row['reported_processing_seconds'])
    close(row['whole_process_seconds'],row['whole_process_nanoseconds']/1e9)
    ct[row['round'],n]=value
track(confirmation['samples'],'confirmation',lambda r:jsonline(pin(prefix,r['stdout']).decode()))
for n,x in confirmation['summary'].items():
    rows=[r for r in confirmation['samples'] if r['variant']==n];assert len(rows)==x['samples']==x['exact_outputs']==13
    close(x['median_whole_process_seconds'],statistics.median(r['whole_process_seconds'] for r in rows))
    key='median_post_json_seconds' if x['post_json_eligible'] else 'median_reported_excluded_processing_seconds'
    close(x[key],statistics.median(ct[i,n] for i in range(13)))
candidate=confirmation['candidate']
for c in confirmation['candidate_paired_comparisons']:
    changes=[100*(ct[i,candidate]/ct[i,c['reference']]-1) for i in range(13)]
    assert c['pairs']==13 and c['candidate_faster_pairs']==sum(v<0 for v in changes)
    for a,b in zip(changes,c['paired_percent_changes']):close(a,b)
    close(statistics.median(changes),c['median_paired_percent_change'])
    rng=random.Random(c['bootstrap_seed']);draws=sorted(statistics.median(rng.choices(changes,k=13)) for _ in range(c['bootstrap_draws']))
    for a,b in zip((draws[249],draws[9749]),c['bootstrap_95_interval_percent']):close(a,b)
refs=confirmation['references'];current=refs['current_22_eligible_Rust'];assert len(current['eligible_variants'])==22
assert current['variant']==min(current['eligible_variants'],key=lambda n:confirmation['summary'][n]['median_post_json_seconds'])
close(current['median_seconds'],confirmation['summary'][current['variant']]['median_post_json_seconds'])
assert report['confirmation']['candidate_summary']==confirmation['summary'][candidate]
assert report['confirmation']['paired']==confirmation['candidate_paired_comparisons']

# Frozen source authority is the published Git base and exact IR340 source commit.
import difflib
def git_files(commit,names):
    raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,check=True,capture_output=True,input=''.join(commit+':'+n+'\n' for n in names).encode()).stdout
    cursor,result=0,{}
    for n in names:
        end=raw.index(b'\n',cursor);header=raw[cursor:end].split();cursor=end+1
        assert len(header)==3 and header[1]==b'blob',(commit,n,header)
        size=int(header[2]);result[n]=raw[cursor:cursor+size];cursor+=size;assert raw[cursor:cursor+1]==b'\n';cursor+=1
    assert cursor==len(raw);return result
closure=report['source_closure'];base_commit=report['required_published_IR338_base'];final_commit=report['required_published_IR340_source']
assert base_commit=='b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164' and final_commit=='1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622'
base_inventory=read(closure['base_inventory']);base=git_files(base_commit,sorted(base_inventory['files']));assert len(base)==2817
assert {n:sha(raw) for n,raw in base.items()}==base_inventory['files']
inventories,reconstructed={},{}
for label,count in [('dense-worker-compiler-v1',2818),('dense-worker-compiler-v2',2818),('dense-worker-fixture-v3',2819),('dense-worker-fixture-v4',2819)]:
    inventory=read(label+'/source-inventory.json');assert len(inventory['files'])==count
    assert sha(json.dumps(inventory['files'],sort_keys=True).encode())==inventory['sha256']
    overrides=closure['inventory_overrides'][label]
    current={n:blob('inventory-overrides/'+label+'/'+n) if n in overrides else base[n] for n in inventory['files']}
    assert {n:sha(raw) for n,raw in current.items()}==inventory['files']
    assert {n:sha(raw) for n,raw in current.items() if base.get(n)!=raw}==overrides
    inventories[label],reconstructed[label]=inventory,current
final=reconstructed['dense-worker-compiler-v2'];assert git_files(final_commit,sorted(final))==final
assert len(closure['overlay'])==10 and len(final)==2818
assert closure['overlay']==closure['inventory_overrides']['dense-worker-compiler-v2']
for n,h in closure['overlay'].items():assert sha(blob('measured-source/'+n))==h==sha(final[n])
parts=[]
for n in sorted(closure['overlay']):
    parts.append('diff --git a/'+n+' b/'+n+'\n');parts.extend(difflib.unified_diff(base.get(n,b'').decode().splitlines(True),final[n].decode().splitlines(True),fromfile='a/'+n if n in base else '/dev/null',tofile='b/'+n))
assert ''.join(parts).encode()==blob(closure['patch'])
overlay=read(closure['overlay_record']);assert overlay['base_commit']==base_commit and overlay['source_commit']==final_commit
assert overlay['files']==closure['overlay'] and overlay['patch_sha256']==sha(blob(closure['patch']))
assert overlay['new_files']==sorted(set(final)-set(base))==['hosts/gpu/src/native_artifact/values/dense_tests.rs']
assert overlay['latency_checkpoint_selected'] is False
optional=report['optional_historical_IR338_source']
if subprocess.run(['git','cat-file','-e',optional+'^{commit}'],cwd=ROOT,capture_output=True).returncode==0:assert git_files(optional,sorted(base))==base
literal=read('typed-replay-compiler-v3/source-inventory.json')['literal_includes'];assert len(literal)==164
assert {n:sha(raw) for n,raw in git_files(base_commit,sorted(literal)).items()}==literal
for label,IR in [('dense-worker-compiler-v1',338),('dense-worker-compiler-v2',340),('dense-worker-fixture-v3',340),('dense-worker-fixture-v4',340)]:
    identity=reconstructed[label]['hosts/gpu/src/native_artifact/program/image/identity.rs'].decode()
    assert re.search(r'NATIVE_PROGRAM_TYPED_IR_VERSION\s*:\s*u64\s*=\s*'+str(IR)+r'\s*;',identity)
base_build=read('owned-capture-compiler-v1/build.json');base_stages={s['crate']:s for s in base_build['stages']};builds={}
for label in inventories:
    build=read(label+'/build.json');assert len(build['stages'])==12 and build['source_sha256']==inventories[label]['sha256']
    assert build['configuration_provenance']['non_gpu_sources_verified_unchanged'] is True
    for stage in build['stages']:
        assert stage['exit']==0
        if stage['crate']!='mech_gpu':
            for k in ('command','environment','library_sha256','source_sha256'):assert stage[k]==base_stages[stage['crate']][k]
    builds[label]=build
    if label.startswith('dense-worker-fixture-'):
        tb=read(label+'/tests.build.json');assert tb['exit']==0 and tb['source_sha256']==inventories[label]['sha256']
production_test_names={'hosts/gpu/src/native_artifact/program/runtime/session/mod.rs','hosts/gpu/src/native_artifact/program/runtime/session/dense_external_inventory_tests.rs'}
for label in ('dense-worker-fixture-v3','dense-worker-fixture-v4'):
    changed={n for n,raw in reconstructed[label].items() if final.get(n)!=raw};assert changed==production_test_names
assert read('dense-worker-fixture-v4/fixture-source-receipt.json')['production_runtime_unchanged'] is True
v3=reconstructed['dense-worker-fixture-v3'];v4=reconstructed['dense-worker-fixture-v4'];assert {n for n in v3 if v3[n]!=v4[n]}=={'hosts/gpu/src/native_artifact/program/runtime/session/dense_external_inventory_tests.rs'}
assert b'NoLimits.compilation()' in v4['hosts/gpu/src/native_artifact/program/runtime/session/dense_external_inventory_tests.rs']
publication=read('IR340-publication/source-audit.json');assert publication['base_source_commit']==base_commit and publication['overlay_files']==10 and publication['exact_source_inventory_entries']==2818
assert read('IR340-publication/source-commit.json')['source_commit']==final_commit

# Qualify the 12 exact ordinary/metrics, original/decoded/restored adapters.
def edge_set(prefix):
    edges=read(prefix+'/edge-fixtures.json');assert len(edges)==31
    for edge in edges:
        assert edge['exit']==0;n=edge['fixture']
        if edge.get('exact_rust_output'):assert read(prefix+'/edges/'+n+'/actual.json')==read(prefix+'/edges/'+n+'/related_posts_rust.json')
        else:
            assert edge['exact_expected_output'] is True
            posts=read('base-commit/benchmarks/related-post-gen/fixtures/posts-'+n+'.json');indices=read('base-commit/benchmarks/related-post-gen/fixtures/expected-'+n+'.json')['related_indices']
            expected=[{'_id':p['_id'],'tags':p['tags'],'related':[posts[i] for i in neighbors]} for p,neighbors in zip(posts,indices)]
            assert read(prefix+'/actual-'+n+'.json')==expected
    return edges
stages={**read('dense-worker-preparation/run.json')['stages'],**read('dense-worker-preparation/continuation-v2.json')['stages']}
qa=read(report['qualification']['audit']);assert qa['output_count']==384 and len(qa['rows'])==12 and qa['timing_claim'] is False
libraries={s['crate']:s['library_sha256'] for s in builds['dense-worker-compiler-v2']['stages']}
for row in qa['rows']:
    source,form,adapter=row['source'],row['form'],row['adapter'];label='dense-worker-'+source+('' if form=='original' else '-'+form)
    path=label+('/output.json' if form=='original' and adapter=='ordinary' else '/output-'+adapter+'.json')
    exact(blob(path),row['full_output_sha256'])
    stage=read(label+'/qualify-5000.json') if form=='original' and adapter=='ordinary' else stages['qualify5000-'+source+'-'+form+'-'+adapter]
    assert stage['exit']==0
    observed=jsonline(stage['stdout']);assert observed['posts']==5000
    for key in ('parallel_tasks','parallel_batches','parallel_peak_workers','parallel_numeric_regions','parallel_numeric_work','work'):assert observed[key]==row[key]
    edges=label+('/edges' if form=='original' and adapter=='ordinary' else '/edges-'+adapter)
    assert sha(blob(edges+'/edge-fixtures.json'))==row['edge_manifest_sha256'];edge_set(edges)
    assert row['parallel_tasks']==4 and row['parallel_batches']==1 and 1<=row['parallel_peak_workers']<=4 and row['parallel_numeric_regions']==8
    audit=read(label+'/same-object-'+adapter+'-audit.json');assert audit['status']=='passed'
    assert audit['every_native_symbol_has_exact_object_relative_address'] is True and audit['all_text_equal_except_independently_verified_relocation_fields'] is True
    assert sha(blob(label+'/build/object.o'))==audit['object']['sha256']
    receipt=read(label+'/runner-'+('aot' if adapter=='ordinary' else adapter)+'.build.json');assert receipt['exit']==0 and receipt['executable_sha256']==audit['executable']['sha256']
    assert receipt['compiler_build_sha256']==sha(blob('dense-worker-compiler-v2/build.json')) and receipt['compiler_source_sha256']==inventories['dense-worker-compiler-v2']['sha256']
    for n,p in receipt['libraries'].items():
        if n.startswith('mech_'):assert p['sha256']==libraries[n]
    for p in receipt['bundle'].values():assert p['sha256'] in digests
    metadata=read(label+'/build/metadata.json');assert metadata['typed_ir_version']==340 and metadata['work_policy_version']==127
assert sum(row['parallel_peak_workers']==3 for row in qa['rows'])==2
continuation=read('dense-worker-preparation/continuation-v2.json');assert continuation['previous_exact_outputs']==321 and continuation['remaining_expected_exact_outputs']==63 and continuation['total_exact_outputs']==384
complete=read(report['qualification']['complete']);assert complete['exact_outputs']==384 and complete['all_processes_terminal'] is True

def counts(value):
    found=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',value['stdout']);assert len(found)==1;return tuple(map(int,found[0]))
focused=read(report['qualification']['focused']);assert [counts(v) for v in focused.values()]==[(26,0,0),(5,0,0),(2,0,0)] and all(v['exit']==0 for v in focused.values())
names=[n for v in focused.values() for n in re.findall(r'^test (\S+) \.\.\. ok$',v['stdout'],re.MULTILINE)];assert len(names)==len(set(names))==33
broad=read(report['qualification']['broad']);baseline=read(report['qualification']['baseline_broad']);assert broad['exit']==baseline['exit']==101
assert counts(broad)==(1084,16,12) and counts(baseline)==(1079,16,11) and '35 filtered out' in broad['stdout']
failures=lambda x:sorted(re.findall(r'^    (native_artifact::[^\n]+)$',x['stdout'],re.MULTILINE))
attribution=read(report['qualification']['broad_attribution']);assert failures(broad)==failures(baseline)==sorted(attribution['failed_names']) and len(attribution['failed_names'])==16 and attribution['same_inherited_failures'] is True
assert read('native-failure-attribution.json')['unchanged_baseline_reproduces_all_failures'] is True
positive=read(report['qualification']['positive_wide']);assert positive['wide']['exit']==0 and positive['u32']['exit']==101
assert 'required: 4676, limit: 4096' in positive['u32']['stderr']
missing=read('dense-worker-fixture-v3/external-diagnostic-initial-wrapper-failure.json');assert missing['subprocess_result_checkpointed'] is False and missing['native_stdout_unavailable'] is True
for source,test in [('wide',positive['wide']),('u32',read(report['qualification']['positive_u32']))]:
    assert test['exit']==0 and counts(test)==(1,0,0)
    values=[json.loads(s.split('DENSE_WORKER_STORAGE ',1)[1]) for s in test['stdout'].splitlines() if 'DENSE_WORKER_STORAGE ' in s];assert len(values)==4
    assert {(x['artifact_form'],x['descriptor_form']) for x in values}=={(0,0),(0,1),(1,0),(1,1)}
    p=report['physical_storage'][source]
    for x in values:
        for raw,key in [('full_funded_types','full_funded_slots'),('global_logical_prefix','logical_prefix'),('physical_values','dense_slots'),('physical_generations','dense_slots'),('physical_banks_bytes','banks_bytes'),('descriptor_index_map_bytes','map_bytes'),('descriptor_map_record_bytes','map_record_bytes')]:assert x[raw]==p[key]
        assert x['bank_stride_bytes']==72 and x['physical_values']*72==x['physical_banks_bytes']
        assert x['global_logical_prefix']*4==x['descriptor_index_map_bytes']
    assert (p['logical_prefix']-p['dense_slots'])*72*4==p['four_worker_bank_saving_bytes']

# Replay the read-only exact-set derivation from its retained checked inputs.
import tempfile
with tempfile.TemporaryDirectory(prefix='mech-dense-proof-') as directory:
    root=Path(directory);inventory=read('dense-worker-audit/inventory.json')
    scope={'__name__':'dense_audit_offline_replay'}
    exec(compile(blob('dense-worker-audit/audit.py'),'retained-dense-worker-audit.py','exec'),scope)
    scope['M']=root
    for dataset in inventory['datasets']:
        for field,file in [('descriptor_path','descriptor.json'),('native_metadata_path','native-metadata.bin')]:
            raw=blob('audit-inputs/'+dataset['name']+'/build/'+file);assert sha(raw)==dataset[field.replace('_path','_sha256')]
            p=root/dataset['name']/'build'/file;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(raw)
        derived=scope['derive'](dataset['name'])
        for field in ('descriptor_path','native_metadata_path'):derived[field]=dataset[field]
        assert derived==dataset
        for region in derived['regions']:
            assert region['union_of_all_categories_not_sum'] is True and region['canonical_library_output_count']==50 and region['canonical_library_scalar_output_count']==14
            union=set().union(*(set(c['values']) for c in region['categories'].values()));assert sorted(union)==region['all_path_union']
            source='wide' if dataset['name']=='scoped-frame' else 'u32';assert len(union)==report['physical_storage'][source]['dense_slots']

omitted=json.loads((D/'omitted-binaries.json').read_bytes())
def verify_pins(value):
    if isinstance(value,dict):
        if isinstance(value.get('path'),str) and 'sha256' in value:
            name='referenced-inputs/'+value['path'].lstrip('/')
            if name in records:
                raw=blob(name);assert sha(raw)==value['sha256']
                if 'bytes' in value:assert len(raw)==value['bytes']
            else:assert value['path'] in omitted,value['path']
        for x in value.values():verify_pins(x)
    elif isinstance(value,list):
        for x in value:verify_pins(x)
verify_pins(plan);verify_pins(cp)
assert sha(blob('new-managed-resources/run-with-compact.py'))==plan['script']['sha256']
assert sha(blob(report['primary']['record']))==plan['latency_authority']['sha256']
assert sha(blob('independent-managed-confirm-v3.py'))==cp['script']['sha256']==confirmation['script_sha256']
assert report['new_timed_resource_exact_outputs']==1271+273+351==1895
assert report['qualification_exact_outputs_separate']==384
assert report['same_source_U32_gain_confirmed'] is True
assert report['new_selected_latency_checkpoint'] is report['wide_gain_established'] is report['fastest_Rust_target_met'] is report['whole_process_is_acceptance_gate'] is False
print(json.dumps({'verified_records':len(records),'unique_blobs':len(checked),'compressed_bytes':manifest['compressed_bytes'],'required_IR338_base':base_commit,'required_IR340_source':final_commit,'full_source_inventory_entries':2818,'literal_includes':164,'source_overlay_files':10,'primary_exact_outputs':1271,'resource_exact_outputs':273,'confirmation_exact_outputs':351,'new_timed_resource_exact_outputs':1895,'separate_qualification_exact_outputs':384,'adapters_qualified':12,'focused_passes':33,'broad_passed':1084,'broad_failures':16,'all_broad_failures_inherited':True,'managed_IR334_still_selected':True,'IR340_U32_same_source_parent_gain_confirmed':True,'fastest_Rust_target_met':False,'native_executions_by_verifier':0},indent=2))
