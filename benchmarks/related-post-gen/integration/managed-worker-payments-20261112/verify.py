#!/usr/bin/env python3
"""Offline IR332/333 negative-checkpoint verification; launches no benchmark."""
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


spec=report['primary']
raw=read(spec['record']);config=read(spec['config']);paired=read(spec['paired_comparison'])
protocol=read(spec['paired_protocol'])
assert protocol['bootstrap_seed']==202611114700 and protocol['draws']==10000
assert protocol['seed_reset_each_variant']is protocol['all_observations_retained']is True
variants={row['name']:row for row in raw['implementations']}
assert len(variants)==30 and set(variants)=={row['name']for row in config['implementations']}
for row in variants.values():
    if row.get('source_sha256'):assert row['source_sha256']in by_digest
    if row.get('receipt_sha256'):assert row['receipt_sha256']in by_digest
    if row['kind']=='mech':
        receipt=read('references/'+row['name']+'/receipt.json')
        assert receipt['executable_sha256']==row['executable_sha256']
        assert receipt['source_sha256']in by_digest
        assert receipt['bundle']['source.mec']['sha256']==row['source_sha256']
assert len(raw['samples'])==spec['exact_outputs']==930
assert raw['execution_order']['seed']==spec['seed']==202611114600
orders=raw['execution_order']['rounds']
assert len(orders)==31
values={name:[]for name in variants}
for index,order in enumerate(orders):
    assert len(order)==30 and set(order)==set(variants)
    assert [sample['variant']for sample in raw['samples']if sample['sample']==index]==order
for sample in raw['samples']:
    name=sample['variant']
    assert sample['exit']==0 and sample['exact_rust_output']
    exact(f'first-paid-workers/work/{name}-{sample["sample"]}.json',sample['output_sha256'])
    value=elapsed(variants[name],sample['stdout'])
    close(value,sample['post_json_seconds']if variants[name]['kind']=='mech'else sample['seconds'])
    values[name].append(value)
medians={name:statistics.median(v)for name,v in values.items()}
eligible=[row['name']for row in variants.values()if row['kind']!='mech'and row.get('calculation_boundary')=='post_json_including_setup']
serial=[name for name in eligible if 'fresh4'not in name]
assert len(eligible)==22 and len(serial)==20
current=min(eligible,key=medians.__getitem__);original=min(serial,key=medians.__getitem__)
assert current==spec['current_fastest_Rust']['variant']==paired['current_fastest']['variant']
assert original==spec['original_20_serial_Rust']['variant']==paired['retained20serial']['variant']
close(medians[current],spec['current_fastest_Rust']['median_seconds'])
close(medians[original],spec['original_20_serial_Rust']['median_seconds'])
control=report['selected']
control_samples=sorted((s for s in raw['samples']if s['variant']==control),key=lambda s:s['sample'])
for row in spec['rows']:
    name=row['implementation']
    close(row['post_json_ms'],medians[name]*1000)
    close(row['times_current_fastest_Rust'],medians[name]/medians[current])
    close(row['times_original_20_serial_Rust'],medians[name]/medians[original])
    samples=sorted((s for s in raw['samples']if s['variant']==name),key=lambda s:s['sample'])
    close(row['whole_process_ms'],statistics.median(s['wall_seconds']for s in samples)*1000)
    changes=[100*(s['post_json_seconds']/c['post_json_seconds']-1)for s,c in zip(samples,control_samples)]
    close(row['paired_median_percent_vs_IR331'],statistics.median(changes))
    rng=random.Random(202611114700)
    draws=sorted(statistics.median(rng.choices(changes,k=31))for _ in range(10000))
    for actual,expected in zip((draws[250],draws[9749]),row['paired_95_interval_percent']):
        close(actual,expected)
cleanup=next(row for row in spec['rows']if row['implementation']=='mech-managed-paid-cleanup')
alias=next(row for row in spec['rows']if row['implementation']=='mech-managed-paid-alias-v2')
assert cleanup['paired_95_interval_percent'][0]>0
assert alias['paired_95_interval_percent'][0]<0<alias['paired_95_interval_percent'][1]
assert report['established_new_latency_gain']is False
assert report['independent_confirmation']is report['adverse_result_rerun']is False

spec=report['resources'];resource=read(spec['record']);plan=read(spec['plan'])
prefix=spec['record'].removesuffix('/report.json')
assert resource['status']=='complete'and resource['exact_outputs']==len(resource['samples'])==203
assert hashlib.sha256(blob(spec['plan'])).hexdigest()==resource['plan_sha256']
assert len(plan['implementations'])==29 and len(plan['rounds'])==7
assert plan['seed']==spec['seed']==202611115400
for pin in [plan['script'],plan['latency_authority'],plan['input'],plan['gold'],*plan['source_configs'],*plan['measurement_helpers']]:
    retained_pin(pin)
resource_variants={row['name']:row for row in plan['implementations']}
assert sum(row['kind']=='mech'for row in resource_variants.values())==4
assert sum(row['kind']!='mech'and row['post_json_counter_eligible']for row in resource_variants.values())==22
for row in resource_variants.values():
    for key in ('source','receipt','adapter_source','qualification'):
        if key in row['pins']:retained_pin(row['pins'][key])
    for pin in row['pins'].get('bundle',{}).values():retained_pin(pin)
for index,order in enumerate(plan['rounds']):
    assert len(order)==29 and set(order)==set(resource_variants)
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
for suffix,instructions in (('retire-overlap',29974067),('paid-cleanup',29749600),('paid-alias-v2',29691144)):
    row=managed['mech-managed-'+suffix+'-instrumented']
    assert (row['allocation_requests'],row['requested_allocation_bytes'],row['peak_additional_live_requested_heap_bytes'])==(725,6376193,4211067)
    assert row['hardware']['instructions_retired']==instructions
assert plan['scope']['free_call_count']is None


def counts(record):
    matches=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',record['stdout'])
    assert len(matches)==1
    return tuple(map(int,matches[0]))


qualification=report['qualification']
assert counts(read(qualification['R333_broad']))==(1059,16,11)
attribution=read(qualification['attribution'])
assert attribution['same16_as_IR326_and_unchangedbaseline']is True
assert attribution['explicit_new_fixture_skipped']is True
assert len(attribution['current_failures'])==16
baseline=read('short-worker-compiler/broad-native-failure-attribution.json')
assert sorted(attribution['current_failures'])==sorted(baseline['current_names'])
assert counts(read(qualification['corrected_positive_fixture']))==(1,0,0)
override=read(qualification['corrected_test_only_override'])
assert override['production_changes']==[]
assert read('paid-alias-fixture-tests-v3/build.json')['stages']==read('paid-alias-compiler-v2/build.json')['stages']
assert qualification['full_broad_green']is False
focused=read('paid-cleanup-compiler/focused-contracts.json')
assert any(value['exit']==101 and '(200, 0, 0)'in value['stdout']for value in focused)
new=read('paid-alias-compiler-v2/new-contracts.json')
assert sum(counts(value)[0]for value in new)==7
assert sum(counts(value)[1]for value in new)==1
assert any(value['exit']==101 and 'exercise an actual compiled cold-alias prefix'in value['stdout']for value in new)
for label in qualification['full5000_and31edges']:
    assert read(label+'/qualify-5000.json')['exit']==0
    exact(label+'/output.json',gold_digest)
    assert read(label+'/qualify-edges.json')['exit']==0
    edges=read(label+'/edges/edge-fixtures.json')
    assert len(edges)==31
    for edge in edges:
        assert edge['exit']==0
        name=edge['fixture']
        if 'exact_rust_output'in edge:
            assert edge['exact_rust_output']
            assert read(label+'/edges/edges/'+name+'/actual.json')==read(label+'/edges/edges/'+name+'/related_posts_rust.json')
        else:
            assert edge['exact_expected_output']
            data=read('base-commit/benchmarks/related-post-gen/fixtures/posts-'+name+'.json')
            indices=read('base-commit/benchmarks/related-post-gen/fixtures/expected-'+name+'.json')['related_indices']
            expected=[{'_id':post['_id'],'tags':post['tags'],'related':[data[index]for index in related]}for post,related in zip(data,indices)]
            assert read(label+'/edges/actual-'+name+'.json')==expected
for label,version in (('retire-overlap',331),('paid-cleanup',332),('paid-alias-v2',333)):
    metadata=read(label+'/build/metadata.json')
    assert (metadata['typed_ir_version'],metadata['work_policy_version'],metadata['abi_version'],metadata['native_descriptor_version'])==(version,126,42,11)
    assert (metadata['code_bytes'],metadata['stack_bytes'])==(1021740,1568)
    audit=read(label+'/same-object-audit.json')
    assert audit['status']=='passed'and audit['every_native_symbol_has_exact_object_relative_address']is True
    assert hashlib.sha256(blob(label+'/build/object.o')).hexdigest()==audit['object']['sha256']

closure=read(report['source_closure'])
assert closure['base']==report['base_source_commit']
overlay=read(report['overlay_receipt'])
all_files=set().union(*(read(spec['inventory'])['files'].keys()for spec in closure['inventories'].values()))
names=sorted(all_files)
raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,capture_output=True,check=True,
    input=''.join(closure['base']+':'+name+'\n'for name in names).encode()).stdout
offset=0;base_hashes={};base_overlay_bytes={}
for name in names:
    end=raw.index(b'\n',offset);header=raw[offset:end].split();offset=end+1
    if len(header)==2 and header[1]==b'missing':base_hashes[name]=None;continue
    assert len(header)==3 and header[1]==b'blob',(name,header)
    length=int(header[2]);base_data=raw[offset:offset+length]
    base_hashes[name]=hashlib.sha256(base_data).hexdigest()
    if name in overlay['paths']:base_overlay_bytes[name]=base_data
    offset+=length
    assert raw[offset:offset+1]==b'\n';offset+=1
assert offset==len(raw)
for label,spec in closure['inventories'].items():
    files=read(spec['inventory'])['files']
    if label=='prefix-compiler-v1':files={name:digest for name,digest in files.items()if not name.startswith('hosts/gpu/')}
    changed={name:digest for name,digest in files.items()if base_hashes[name]!=digest}
    assert changed==spec['overrides']
    assert len(files)==spec['source_files']
    assert len(files)-len(changed)==spec['base_identical_files']
    for name,digest in changed.items():
        assert hashlib.sha256(blob(label+'/source/'+name)).hexdigest()==digest
assert overlay['IR']==333 and len(overlay['paths'])==9
for name,digest in overlay['paths'].items():
    assert hashlib.sha256(blob('unselected-paid-workers-IR333/source/'+name)).hexdigest()==digest
assert override['after_sha256']==overlay['paths'][override['path']]
assert hashlib.sha256(blob('paid-alias-fixture-tests-v3/source/'+override['path'])).hexdigest()==override['after_sha256']
# Apply the retained patch to published base bytes in memory, without fuzz.
patched=set()
for block in blob(report['overlay']).decode().split('diff --git ')[1:]:
    lines=block.splitlines(keepends=True)
    path=next(line[6:].rstrip('\n')for line in lines if line.startswith('+++ b/'))
    original=base_overlay_bytes[path].decode().splitlines(keepends=True)
    output=[];cursor=0;index=0
    while index<len(lines):
        match=re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@',lines[index])
        if not match:index+=1;continue
        start=int(match[1])-1
        assert start>=cursor
        output.extend(original[cursor:start]);cursor=start;index+=1
        old_count=new_count=0
        while index<len(lines)and not lines[index].startswith('@@ '):
            line=lines[index]
            assert line and line[0]in ' +-'
            marker=line[0];value=line[1:]
            if marker in ' -':
                assert original[cursor]==value,(path,cursor)
                cursor+=1;old_count+=1
            if marker in ' +':output.append(value);new_count+=1
            index+=1
        assert old_count==int(match[2]or 1)and new_count==int(match[4]or 1)
    output.extend(original[cursor:])
    assert ''.join(output).encode()==blob('unselected-paid-workers-IR333/source/'+path)
    patched.add(path)
assert patched==set(overlay['paths'])
source_audit=read(report['source_audit'])
assert source_audit['base_commit']==closure['base']
assert source_audit['v3_product_stages_identical_to_v2']is True
for pin in source_audit['retention_files']:retained_pin(pin)
scope_audit=read(report['worker_scope_audit']['record'])
assert 'not a measured invocation total'in scope_audit['existing_estimate_warning']
assert report['worker_scope_audit']['dynamic_success_count_measured']is False
assert report['worker_scope_audit']['lock_savings_measured']is False
assert report['timed_exact_outputs']==930+203==1133
print(json.dumps({'verified_records':len(records),'unique_blobs':len(checked),'compressed_bytes':manifest['compressed_bytes'],
                  'base_source_commit':closure['base'],'primary_exact_outputs':930,'resource_exact_outputs':203,
                  'qualification_edges_per_candidate':31,'selected':'IR331','new_latency_gain_established':False,
                  'R332_instructions':29749600,'R333_instructions':29691144,'allocation_bytes_unchanged':True},indent=2))
