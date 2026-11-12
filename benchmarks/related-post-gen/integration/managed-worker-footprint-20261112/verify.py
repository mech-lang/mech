#!/usr/bin/env python3
"""Offline IR334 measured-source and evidence verification; launches no benchmark."""
from functools import lru_cache
from pathlib import Path
import ast
import copy
import gzip
import hashlib
import json
import math
import random
import re
import statistics
import subprocess

D=Path(__file__).resolve().parent
ROOT=D.parents[3]
report=json.loads((D/'report.json').read_bytes())
manifest=json.loads((D/'records.json').read_bytes())
records={}
by_digest={}
checked=set()
for row in manifest['records']:
    assert row['source']not in records
    path=D/row['path']
    assert path.resolve().is_relative_to(D)
    if row['path']not in checked:
        packed=path.read_bytes()
        assert len(packed)==row['compressed_bytes']
        assert hashlib.sha256(packed).hexdigest()==row['compressed_sha256']
        raw=gzip.decompress(packed)
        assert len(raw)==row['uncompressed_bytes']
        assert hashlib.sha256(raw).hexdigest()==row['uncompressed_sha256']
        checked.add(row['path'])
    records[row['source']]=row
    by_digest[row['uncompressed_sha256']]=row['path']
assert len(records)==manifest['logical_records']
assert len(checked)==manifest['unique_blobs']
assert sum((D/path).stat().st_size for path in checked)==manifest['compressed_bytes']


@lru_cache(maxsize=128)
def uncompressed(path):
    return gzip.decompress((D/path).read_bytes())


def blob(name):
    return uncompressed(records[name]['path'])


def read(name):
    return json.loads(blob(name))


def close(a,b):
    assert math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-12),(a,b)


def retained_pin(pin):
    assert pin['sha256']in by_digest,pin['path']
    assert len(uncompressed(by_digest[pin['sha256']]))==pin['bytes']


gold=json.loads(blob('publication-inputs/gold.json'))
assert len(gold)==len(read('publication-inputs/posts.json'))==5000
gold_digest=hashlib.sha256(blob('publication-inputs/gold.json')).hexdigest()
validated_outputs=set()


def exact(name,digest):
    raw=blob(name)
    assert hashlib.sha256(raw).hexdigest()==digest
    if digest not in validated_outputs:
        assert json.loads(raw)==gold
        validated_outputs.add(digest)


timer_re=re.compile(r'^Processing time \(w/o IO\):\s*([0-9]+(?:\.[0-9]+)?)\s*(ns|µs|μs|us|ms|s)\s*$',re.MULTILINE)
units={'ns':1e-9,'µs':1e-6,'μs':1e-6,'us':1e-6,'ms':1e-3,'s':1}


def elapsed(row,stdout):
    if row['kind']=='mech':
        values=[json.loads(line)for line in stdout.splitlines()if line.startswith('{')]
        values=[v for v in values if 'post_json_seconds'in v]
        assert len(values)==1 and values[0]['posts']==5000 and values[0]['sample']==0
        result=values[0]['post_json_seconds']
    else:
        values=timer_re.findall(stdout)
        assert len(values)==1
        result=float(values[0][0])*units[values[0][1]]
    assert math.isfinite(result)and result>0
    return result


def parse_process(text):
    result={}
    for line in text.splitlines():
        tokens=line.strip().split()
        if len(tokens)==2 and tokens[0]in ('real','user','sys'):
            result[{'real':'wall_seconds','user':'user_cpu_seconds','sys':'system_cpu_seconds'}[tokens[0]]]=float(tokens[1])
        elif tokens and tokens[0].isdigit():
            key='_'.join(tokens[1:]);key={'maximum_resident_set_size':'peak_resident_bytes','peak_memory_footprint':'peak_physical_footprint_bytes','instructions_retired':'instructions_retired','cycles_elapsed':'cycles'}.get(key,key)
            result[key]=int(tokens[0])
    return result

def pinned_object(prefix,pin):
    value=blob(prefix+'/'+pin['path']);assert hashlib.sha256(value).hexdigest()==pin['sha256']and len(value)==pin['bytes'];return value

def nested_medians(items):
    result={}
    for key in sorted(set.intersection(*(set(item)for item in items))):
        values=[item[key]for item in items]
        if all(isinstance(value,dict)for value in values):result[key]=nested_medians(values)
        elif all(type(value)in (int,float)for value in values):result[key]=statistics.median(values)
    return result

def shuffled_orders(rows,seed,count):
    rng=random.Random(seed);orders=[]
    for _ in range(count):
        names=[r['name']for r in rows];rng.shuffle(names);orders.append(names)
    return orders

def summary_rows(rows,values,raw_samples,current,serial,round_key='sample',wall_key='wall_seconds'):
    for row in rows:
        name=row['implementation'];close(row['post_json_ms'],values[name]*1000)
        close(row['times_current_fastest_Rust'],values[name]/values[current]);close(row['times_original_20_serial_Rust'],values[name]/values[serial])
        close(row['whole_process_ms'],statistics.median(s[wall_key]for s in raw_samples if s['variant']==name)*1000)

spec=report['primary'];raw=read(spec['raw_checkpoint']);derived=read(spec['record']);config=read(spec['config']);protocol=read(spec['protocol']);paired=read(spec['paired_comparison'])
assert raw['summary']=={}
for key in ('samples','implementations','execution_order','input','output_retention','machine','platform'):assert raw[key]==derived[key],key
recovery=derived['post_processing_recovery']
assert recovery['changed_observations']is recovery['rerun_executables']is False
assert recovery['raw_checkpoint_sha256']==hashlib.sha256(blob(spec['raw_checkpoint'])).hexdigest()
assert recovery['comparison_module_sha256']==hashlib.sha256(blob('published-commit/benchmarks/related-post-gen/compare-implementations.py')).hexdigest()
assert derived['resource_reference']is derived['whole_process_resource_reference']is None
# Replay only the frozen pure summary function, never importing or invoking a runner.
helper_ast=ast.parse(blob('published-commit/benchmarks/related-post-gen/compare-implementations.py').decode())
function=next(n for n in helper_ast.body if isinstance(n,ast.FunctionDef)and n.name=='add_timing_comparisons')
namespace={'statistics':statistics};exec(compile(ast.Module(body=[function],type_ignores=[]),'<retained pure summary>','exec'),namespace)
replayed=copy.deepcopy(raw)
for variant in replayed['implementations']:
    samples=[s for s in replayed['samples']if s['variant']==variant['name']]
    value={'median_seconds':statistics.median(s['seconds']for s in samples),'samples':len(samples),'all_exact_rust':True,'category':variant['category'],'process_resources_median':nested_medians([s['process_resources']for s in samples])}
    if variant['kind']=='mech'and all('processing_plus_admission_seconds'in s for s in samples):value['median_processing_plus_admission_seconds']=statistics.median(s['processing_plus_admission_seconds']for s in samples)
    replayed['summary'][variant['name']]=value
namespace['add_timing_comparisons'](replayed)
replayed['post_processing_recovery']=derived['post_processing_recovery']
replayed['latency_qualification']=True;replayed['resource_reference']=None;replayed['whole_process_resource_reference']=None
replayed['resource_boundaries']['latency']='All executables in this roster are ordinary uninstrumented binaries. macOS time wrapper records separate whole-process counters; no postJSON hardware instrumentation was enabled.'
assert replayed==derived
assert len(raw['samples'])==spec['exact_outputs']==868
assert protocol['config_sha256']==hashlib.sha256(blob(spec['config'])).hexdigest()
assert protocol['seed']==spec['seed']==202611120519 and protocol['paired_bootstrap_seed']==spec['bootstrap_seed']==202611120520
variants={r['name']:r for r in raw['implementations']};assert len(variants)==28
assert set(variants)=={r['name']for r in config['implementations']}
assert raw['execution_order']['rounds']==shuffled_orders(config['implementations'],spec['seed'],31)
values={name:[]for name in variants}
for index,order in enumerate(raw['execution_order']['rounds']):
    assert len(order)==28 and set(order)==set(variants)
    assert [s['variant']for s in raw['samples']if s['sample']==index]==order
for sample in raw['samples']:
    name=sample['variant'];assert sample['exit']==0 and sample['exact_rust_output']
    exact(f'first-scoped-frame-v2/work/{name}-{sample["sample"]}.json',sample['output_sha256'])
    value=elapsed(variants[name],sample['stdout']);close(value,sample['post_json_seconds']if variants[name]['kind']=='mech'else sample['seconds']);values[name].append(value)
    process_name='first-scoped-frame-v2/work/'+Path(sample['process_resources_raw']).name
    assert parse_process(blob(process_name).decode())==sample['process_resources']
for name,row in variants.items():
    if row.get('source_sha256'):assert row['source_sha256']in by_digest
    if row.get('receipt_sha256'):assert row['receipt_sha256']in by_digest
    samples=[s for s in raw['samples']if s['variant']==name];summary=derived['summary'][name]
    assert summary['samples']==31 and summary['all_exact_rust']is True
    close(summary['median_seconds'],statistics.median(s['seconds']for s in samples))
    close(summary['median_whole_process_seconds'],statistics.median(s['wall_seconds']for s in samples))
    assert summary['process_resources_median']==nested_medians([s['process_resources']for s in samples])
    if row.get('calculation_boundary')=='post_json_including_setup':close(summary['median_post_json_seconds'],statistics.median(values[name]))
medians={name:statistics.median(items)for name,items in values.items()}
eligible=[r['name']for r in variants.values()if r['kind']!='mech'and r.get('calculation_boundary')=='post_json_including_setup'];serial=[n for n in eligible if 'fresh4'not in n]
assert len(eligible)==22 and len(serial)==20
current=min(eligible,key=medians.__getitem__);original=min(serial,key=medians.__getitem__)
assert current==spec['current_fastest_Rust']['variant']==derived['primary_reference']['variant']
assert original==spec['original_20_serial_Rust']['variant']
close(medians[current],spec['current_fastest_Rust']['median_seconds']);close(medians[original],spec['original_20_serial_Rust']['median_seconds'])
summary_rows(spec['rows'],medians,raw['samples'],current,original)
by_pair={(s['sample'],s['variant']):elapsed(variants[s['variant']],s['stdout'])for s in raw['samples']}
changes=[100*(by_pair[i,report['candidate']]/by_pair[i,report['control']]-1)for i in range(31)]
close(statistics.median(changes),paired['paired_percent']);assert sum(v<0 for v in changes)==paired['wins']==23
rng=random.Random(spec['bootstrap_seed']);draws=sorted(statistics.median(rng.choices(changes,k=31))for _ in range(10000))
for a,b in zip((draws[250],draws[9749]),paired['confidence_95']):close(a,b)
assert paired==spec['paired_incremental']and paired['confidence_95'][1]<0
partial=read(report['partial_initial_attempt']['record']);assert len(partial['samples'])==1
assert partial['samples'][0]['exit']==1 and partial['samples'][0]['variant'].startswith('rust-')
assert 'sysctl kern.clockrate: Operation not permitted'in partial['samples'][0]['stderr']
assert timer_re.search(partial['samples'][0]['stdout'])and not any(s['variant']==report['candidate']for s in partial['samples'])
assert read('first-scoped-frame/config.json')==config

spec=report['independent_confirmation'];confirm=read(spec['record']);plan=read(spec['plan']);prefix=spec['record'].removesuffix('/report.json')
assert confirm['status']=='complete'and confirm['exact_outputs']==len(confirm['samples'])==351
assert [json.loads(line)for line in blob(prefix+'/samples.jsonl').decode().splitlines()]==confirm['samples']
assert hashlib.sha256(blob(spec['plan'])).hexdigest()==confirm['plan_sha256']
assert plan['seed']==spec['seed']==202611120521 and len(plan['implementations'])==27 and len(plan['rounds'])==13
assert plan['candidate']==report['candidate']and plan['extra_controls']==[report['control']]
assert {r['name']for r in plan['implementations']if r['kind']=='mech'}=={report['candidate'],report['control'],'mech-managed-host-v2-4'}
assert sum(r['original_20_serial_reference']for r in plan['implementations'])==20
assert sum(r['kind']!='mech'and r['post_json_eligible']for r in plan['implementations'])==22
assert plan['rounds']==shuffled_orders(plan['implementations'],plan['seed'],13)
for pin in (plan['script'],plan['source_config'],plan['input'],plan['gold']):retained_pin(pin)
assert confirm['script_sha256']==plan['script']['sha256']
assert 'compare-implementations'not in blob(spec['script']).decode()
variants={r['name']:r for r in plan['implementations']}
for row in variants.values():
    for key in ('source','receipt','adapter_source'):
        if key in row['pins']:retained_pin(row['pins'][key])
    for pin in row['pins'].get('bundle',{}).values():retained_pin(pin)
for index,order in enumerate(plan['rounds']):assert [s['variant']for s in confirm['samples']if s['round']==index]==order
for sample in confirm['samples']:
    folder=prefix+f'/runs/{sample["round"]:02d}-{sample["position"]:02d}-{sample["variant"]}'
    assert read(folder+'/result.json')==sample
    invocation=read(folder+'/invocation.json');assert all(sample[k]==v for k,v in invocation.items())
    row=variants[sample['variant']];assert sample['exit']==0 and sample['exact_output']and not sample['timed_out']
    stdout=pinned_object(prefix,sample['stdout']).decode();pinned_object(prefix,sample['stderr']);exact(prefix+'/'+sample['output']['path'],sample['output']['sha256'])
    value=elapsed(row,stdout)
    if row['post_json_eligible']:close(value,sample['post_json_seconds'])
    else:assert sample['post_json_seconds']is None
    assert sample['executable_sha256']==row['pins']['executable']['sha256']and sample['source_sha256']==row['pins']['source']['sha256']
    close(sample['whole_process_nanoseconds']/1e9,sample['whole_process_seconds'])
medians={}
for name,value in confirm['summary'].items():
    samples=[s for s in confirm['samples']if s['variant']==name];assert len(samples)==13
    close(value['median_whole_process_seconds'],statistics.median(s['whole_process_seconds']for s in samples))
    if variants[name]['post_json_eligible']:
        medians[name]=statistics.median(s['post_json_seconds']for s in samples);close(value['median_post_json_seconds'],medians[name])
for key,count in (('original_20_serial_Rust',20),('current_22_eligible_Rust',22)):
    ref=confirm['references'][key];assert len(ref['eligible_variants'])==count
    winner=min(ref['eligible_variants'],key=medians.__getitem__);assert winner==ref['variant'];close(medians[winner],ref['median_seconds'])
current=confirm['references']['current_22_eligible_Rust']['variant'];original=confirm['references']['original_20_serial_Rust']['variant']
summary_rows(spec['rows'],medians,confirm['samples'],current,original,wall_key='whole_process_seconds')
assert spec['references']==confirm['references']and spec['paired_comparisons']==confirm['candidate_paired_comparisons']
by_pair={(s['round'],s['variant']):s['post_json_seconds']for s in confirm['samples']if s['post_json_seconds']is not None}
for index,comparison in enumerate(confirm['candidate_paired_comparisons']):
    assert comparison['bootstrap_seed']==plan['seed']+1+index and comparison['bootstrap_draws']==10000
    changes=[100*(by_pair[i,report['candidate']]/by_pair[i,comparison['reference']]-1)for i in range(13)]
    for a,b in zip(changes,comparison['paired_percent_changes']):close(a,b)
    close(statistics.median(changes),comparison['median_paired_percent_change']);assert sum(v<0 for v in changes)==comparison['candidate_faster_pairs']
    rng=random.Random(comparison['bootstrap_seed']);draws=sorted(statistics.median(rng.choices(changes,k=13))for _ in range(10000))
    for a,b in zip((draws[249],draws[9749]),comparison['bootstrap_95_interval_percent']):close(a,b)
incremental=next(c for c in confirm['candidate_paired_comparisons']if c['reference']==report['control'])
assert incremental['bootstrap_95_interval_percent'][1]<0 and incremental['candidate_faster_pairs']==10
assert not confirm['target_met_against_current_fastest_Rust']and report['fastest_Rust_target_met']is False
assert report['incremental_latency_gain_confirmed']is True

spec=report['resources'];resource=read(spec['record']);plan=read(spec['plan'])
prefix=spec['record'].removesuffix('/report.json')
assert resource['status']=='complete'and resource['exact_outputs']==len(resource['samples'])==196
assert hashlib.sha256(blob(spec['plan'])).hexdigest()==resource['plan_sha256']
assert len(plan['implementations'])==28 and len(plan['rounds'])==7
assert plan['seed']==spec['seed']==202611120522
for pin in [plan['script'],plan['latency_authority'],plan['input'],plan['gold'],*plan['source_configs'],*plan['measurement_helpers']]:
    retained_pin(pin)
resource_variants={row['name']:row for row in plan['implementations']}
assert sum(row['kind']=='mech'for row in resource_variants.values())==3
assert sum(row['kind']!='mech'and row['post_json_counter_eligible']for row in resource_variants.values())==22
for row in resource_variants.values():
    for key in ('source','receipt','adapter_source','qualification'):
        if key in row['pins']:retained_pin(row['pins'][key])
    for pin in row['pins'].get('bundle',{}).values():retained_pin(pin)
for index,order in enumerate(plan['rounds']):
    assert len(order)==28 and set(order)==set(resource_variants)
    assert [s['variant']for s in resource['samples']if s['round']==index]==order
for sample in resource['samples']:
    assert sample['exit']==0 and sample['exact_output']and not sample['timed_out']
    for key in ('stdout','stderr','process_resources_raw'):
        value=sample[key];data=blob(prefix+'/'+value['path'])
        assert hashlib.sha256(data).hexdigest()==value['sha256']and len(data)==value['bytes']
    row=resource_variants[sample['variant']]
    stdout=blob(prefix+'/'+sample['stdout']['path']).decode()
    if row['kind']=='mech':
        emitted=[json.loads(line)for line in stdout.splitlines()if line.startswith('{')]
        assert len(emitted)==1 and emitted[0]['posts']==5000
        emitted=emitted[0]
        windows={'post_json':emitted['measurement']}
        for key,label in (('restore_measurement','restore'),('json_measurement','json'),('admission_measurement','admission')):
            if key in emitted:windows[label]=emitted[key]
        assert windows==sample['windows']
    else:
        emitted=[json.loads(line.removeprefix('PROCESS_METRICS '))for line in stdout.splitlines()if line.startswith('PROCESS_METRICS ')]
        if row['instrumented']:
            assert len(emitted)==1
            value=emitted[0]
            windows={'post_json'if row['post_json_counter_eligible']else 'excluded_processing':value.get('processing',value)}
            if 'preparation'in value:windows['reported_preparation']=value['preparation']
            assert windows==sample['windows']
        else:assert not emitted and sample['windows']=={}
    process={}
    for line in blob(prefix+'/'+sample['process_resources_raw']['path']).decode().splitlines():
        tokens=line.strip().split()
        if len(tokens)==2 and tokens[0]in ('real','user','sys'):
            process[{'real':'wall_seconds','user':'user_cpu_seconds','sys':'system_cpu_seconds'}[tokens[0]]]=float(tokens[1])
        elif tokens and tokens[0].isdigit():
            key='_'.join(tokens[1:])
            key={'maximum_resident_set_size':'peak_resident_bytes','peak_memory_footprint':'peak_physical_footprint_bytes',
                 'instructions_retired':'instructions_retired','cycles_elapsed':'cycles'}.get(key,key)
            process[key]=int(tokens[0])
    assert process==sample['whole_process_counters']
    exact(prefix+'/'+sample['output']['path'],sample['output']['sha256'])
    assert 'post_json'in sample['windows']or not resource_variants[sample['variant']]['post_json_counter_eligible']
    for value in sample['windows'].values():
        assert value['allocator_accounting_underflows']==value['allocator_lifetime_accounting_underflows']==0
        assert value['hardware']['cycles']>0 and value['hardware']['instructions_retired']>0
    assert sample['whole_process_counters']['peak_resident_bytes']>0
for row in spec['rows']:
    samples=[s for s in resource['samples']if s['variant']==row['implementation']]
    assert len(samples)==7
    for key in ('allocation_requests','requested_allocation_bytes','freed_allocation_bytes','peak_additional_live_requested_heap_bytes'):
        close(row['post_json'][key],statistics.median(s['windows']['post_json'][key]for s in samples))
    for key in ('instructions_retired','cycles','resident_bytes_after','physical_footprint_bytes_after'):
        close(row['post_json']['hardware'][key],statistics.median(s['windows']['post_json']['hardware'][key]for s in samples))
    close(row['whole_process']['peak_resident_bytes'],statistics.median(s['whole_process_counters']['peak_resident_bytes']for s in samples))
managed={row['implementation']:row['post_json']for row in spec['rows']if row['implementation'].startswith('mech-managed')}
for suffix,instructions in (('retire-overlap',29912576),('scoped-frame',29679792)):
    row=managed['mech-managed-'+suffix+'-instrumented']
    assert (row['allocation_requests'],row['requested_allocation_bytes'],row['peak_additional_live_requested_heap_bytes'])==(725,6376193,4211067)
    assert row['hardware']['instructions_retired']==instructions
assert plan['scope']['free_call_count']is None

assert managed['mech-managed-retire-overlap-instrumented']['hardware']['cycles']==9072521
assert managed['mech-managed-scoped-frame-instrumented']['hardware']['cycles']==8844701
# Verify all nested window medians, including restore/JSON scopes, not only selected rows.
for name,summary in resource['summary'].items():
    samples=[s for s in resource['samples']if s['variant']==name];assert len(samples)==7
    assert summary['whole_process_medians']==nested_medians([s['whole_process_counters']for s in samples])
    assert summary['window_medians']==nested_medians([s['windows']for s in samples])
assert plan['rounds']==shuffled_orders(plan['implementations'],plan['seed'],7)

def counts(record):
    matches=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',record['stdout'])
    assert len(matches)==1
    return tuple(map(int,matches[0]))
q=report['qualification']
for key,total in (('engine',68),('GPU',97)):
    tests=read(q[key]);assert all(r['exit']==0 for r in tests)
    assert tuple(sum(counts(r)[i]for r in tests)for i in range(3))==(total,0,0)
broad=read(q['broad']);assert broad['exit']==101 and counts(broad)==(1056,16,11)
assert '; 35 filtered out;'in broad['stdout']
attribution=read(q['attribution']);baseline=read('native-failure-attribution.json')
assert attribution['identical_to_retained_unchanged_baseline']is True and attribution['no_broad_green_claim']is True
assert sorted(attribution['failed_names'])==sorted(baseline['failed_names'])and baseline['unchanged_baseline_reproduces_all_failures']is True
assert hashlib.sha256(blob(q['broad'])).hexdigest()==attribution['receipt_sha256']
assert read(q['full5000'])['exit']==read(q['edges'])['exit']==0
exact('scoped-frame/output.json',gold_digest)
edges=read('scoped-frame/edges/edge-fixtures.json');assert len(edges)==31
for edge in edges:
    assert edge['exit']==0;name=edge['fixture']
    if 'exact_rust_output'in edge:
        assert edge['exact_rust_output']
        assert read('scoped-frame/edges/edges/'+name+'/actual.json')==read('scoped-frame/edges/edges/'+name+'/related_posts_rust.json')
    else:
        assert edge['exact_expected_output']
        data=read('published-commit/benchmarks/related-post-gen/fixtures/posts-'+name+'.json');indices=read('published-commit/benchmarks/related-post-gen/fixtures/expected-'+name+'.json')['related_indices']
        expected=[{'_id':p['_id'],'tags':p['tags'],'related':[data[i]for i in related]}for p,related in zip(data,indices)]
        assert read('scoped-frame/edges/actual-'+name+'.json')==expected
metadata=read('scoped-frame/build/metadata.json');assert(metadata['typed_ir_version'],metadata['work_policy_version'],metadata['abi_version'],metadata['native_descriptor_version'])==(334,126,42,11)
assert(metadata['code_bytes'],metadata['stack_bytes'])==(1021740,1568)
audit=read(q['object_link_audit']);assert audit['status']=='passed'and audit['every_native_symbol_has_exact_object_relative_address']is True
assert audit['all_text_equal_except_independently_verified_relocation_fields']is True
assert hashlib.sha256(blob('scoped-frame/build/object.o')).hexdigest()==audit['object']['sha256']
for kind in ('aot','metrics'):
    receipt=read('scoped-frame/runner-'+kind+'.build.json')
    assert receipt['exit']==0 and receipt['compiler_source_sha256']==read('scoped-frame-compiler-v1/source-inventory.json')['sha256']
    assert receipt['compiler_build_sha256']==hashlib.sha256(blob(q['compiler_build'])).hexdigest()
    for pin in receipt['bundle'].values():assert pin['sha256']in by_digest
    if kind=='aot':assert receipt['executable_sha256']==audit['executable']['sha256']
# Logical source work and fresh-worker schedule stay identical across the control/candidate.
primary=read(report['primary']['raw_checkpoint'])
for name in (report['candidate'],report['control']):
    observations=[json.loads(line)for s in primary['samples']if s['variant']==name for line in s['stdout'].splitlines()if line.startswith('{')]
    assert len(observations)==31
    assert {o['work']for o in observations}=={36864870728}
    assert all(o['parallel_peak_workers']==o['parallel_tasks']==4 and o['parallel_batches']==1 for o in observations)
# Qualify fresh-four-worker Rust latency and metrics sources independently of the measurement cohort.
for metric in (False,True):
    prefix='rust-fresh4-control/'+('metrics/'if metric else '')
    qualification=read(prefix+'qualification-summary.json')
    for control in qualification['controls']:
        folder=prefix+control['label']+'/'
        assert hashlib.sha256(blob(folder+'src/main.rs')).hexdigest()==control['source_sha256']
        assert control['exact_full5000']is True and control['unit_tests_passed']==5
        assert counts(read(folder+'tests.json'))==(5,0,0)
        exact(folder+'full5000-fresh4-work/related_posts_rust_mech.json',gold_digest)
        if not metric:
            assert control['exact_edges']==31 and control['full5000_fresh_query_workers']==4

closure=read(report['source_closure']);inventory=read(closure['inventory'])
assert len(inventory['files'])==closure['source_files']==2814
assert hashlib.sha256(json.dumps(inventory['files'],sort_keys=True).encode()).hexdigest()==inventory['sha256']==closure['measured_source_sha256']
assert report['measured_source_commit']==closure['measured_source_commit']=='55a47b080eba45e46596976279d196b9c97eba2e'
# A docs-only rebase may replace this reachable alias; original measured commit remains provenance.
commit=report.get('published_source_equivalent_commit',report['measured_source_commit'])
def git_data(commit,names,allow_missing=False):
    raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,capture_output=True,check=True,input=''.join(commit+':'+name+'\n'for name in names).encode()).stdout
    cursor=0;result={}
    for name in names:
        end=raw.index(b'\n',cursor);header=raw[cursor:end].split();cursor=end+1
        if allow_missing and len(header)==2 and header[1]==b'missing':result[name]=None;continue
        assert len(header)==3 and header[1]==b'blob',(name,header)
        size=int(header[2]);result[name]=raw[cursor:cursor+size];cursor+=size;assert raw[cursor:cursor+1]==b'\n';cursor+=1
    assert cursor==len(raw);return result
all_files={**inventory['files'],**inventory['literal_includes']};published=git_data(commit,sorted(all_files))
for name,digest in all_files.items():assert hashlib.sha256(published[name]).hexdigest()==digest,name
assert len(inventory['literal_includes'])==closure['literal_includes']==164
base=git_data(closure['previous_selected_source_commit'],sorted(inventory['files']),True)
changed={name:digest for name,digest in inventory['files'].items()if base[name]is None or hashlib.sha256(base[name]).hexdigest()!=digest}
assert changed==closure['three_file_overlay']and len(changed)==3
for name,digest in changed.items():assert hashlib.sha256(blob('measured-source/'+name)).hexdigest()==digest
# The retained Git patch is reproducible from the published source, restricted to the three deltas.
patch=subprocess.run(['git','diff','--binary',closure['previous_selected_source_commit'],commit,'--',*sorted(changed)],cwd=ROOT,capture_output=True,check=True).stdout
assert patch==blob(closure['patch'])
reused_inventory=read(closure['reused_non_GPU_inventory'])['files']
# Only seven unchanged crates are reused; engine/compute/stdlib/runtime/GPU are rebuilt.
reused_prefixes=('src/core/','src/syntax/','src/bytecode/','src/math/','src/compare/','src/logical/')
build=read(q['compiler_build']);old_build=read('prefix-compiler-v1/build.json');old={s['crate']:s for s in old_build['stages']}
assert len(build['stages'])==12 and set(build['rebuild'])=={'mech_engine','mech_compute','mech_stdlib','mech_runtime','mech_gpu'}
for stage in build['stages']:
    assert stage['exit']==0
    if stage.get('reused'):
        assert stage['configuration_unchanged']is True
        original=old[stage['crate']]
        for key in ('command','environment','library_sha256','source_sha256'):assert stage[key]==original[key]
    else:assert stage['source_sha256']==inventory['sha256']
# Every non-GPU source change vs the reused closure is confined to the rebuilt engine.
non_gpu_changes={name for name,digest in inventory['files'].items()if not name.startswith('hosts/gpu/')and reused_inventory.get(name)!=digest}
assert non_gpu_changes=={'src/engine/src/resident/budget.rs','src/engine/src/resident/native_projection_prefix_tests.rs'}
for kind in ('aot','metrics'):
    receipt=read('scoped-frame/runner-'+kind+'.build.json')
    stage_libraries={s['crate']:s['library_sha256']for s in build['stages']}
    for name,pin in receipt['libraries'].items():
        if name.startswith('mech_'):assert stage_libraries[name]==pin['sha256']
assert report['timed_exact_outputs']==868+351+196==1415
print(json.dumps({'verified_records':len(records),'unique_blobs':len(checked),'compressed_bytes':manifest['compressed_bytes'],'measured_source_commit':report['measured_source_commit'],'published_source_equivalent_commit':commit,'source_inventory_entries':2814,'literal_include_entries':164,'primary_exact_outputs':868,'independent_exact_outputs':351,'resource_exact_outputs':196,'qualification_edges':31,'selected':'IR334','incremental_latency_gain_confirmed':True,'fastest_Rust_target_met':False,'post_JSON_instructions':29679792,'allocation_bytes_and_requests_unchanged':True},indent=2))
