#!/usr/bin/env python3
"""Verify retained IR331 bytes, pinned source, exact outputs and complete scopes.

This verifier does not compile or execute any benchmark. Executables/libraries
are represented by build/link receipts, rather than retained binary copies.
"""
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
report = json.loads((D/'report.json').read_bytes())
manifest = json.loads((D/'records.json').read_bytes())
records = {}
by_digest = {}
checked = set()
for record in manifest['records']:
    assert record['source'] not in records
    path = D/record['path']
    assert path.resolve().is_relative_to(D)
    if record['path'] not in checked:
        packed = path.read_bytes()
        assert len(packed) == record['compressed_bytes']
        assert hashlib.sha256(packed).hexdigest() == record['compressed_sha256']
        value = gzip.decompress(packed)
        assert len(value) == record['uncompressed_bytes']
        assert hashlib.sha256(value).hexdigest() == record['uncompressed_sha256']
        checked.add(record['path'])
    records[record['source']] = record
    by_digest[record['uncompressed_sha256']] = record['path']
assert len(records) == manifest['logical_records']
assert len(checked) == manifest['unique_blobs']
assert sum((D/path).stat().st_size for path in checked) == manifest['compressed_bytes']


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
    # Large executable/library files are intentionally represented by their
    # receipts. Only demand retained bytes for actual retained data sources.
    assert pin['sha256'] in by_digest,pin['path']
    value = uncompressed(by_digest[pin['sha256']])
    assert len(value) == pin['bytes']


input_bytes = blob('publication-inputs/posts.json')
gold_bytes = blob('publication-inputs/gold.json')
assert hashlib.sha256(input_bytes).hexdigest() == report['input_sha256']
assert hashlib.sha256(gold_bytes).hexdigest() == report['gold_sha256']
gold = json.loads(gold_bytes)
assert len(json.loads(input_bytes)) == len(gold) == 5000
checked_output_digests = set()


def exact_output(name, expected_digest):
    value = blob(name)
    digest = hashlib.sha256(value).hexdigest()
    assert digest == expected_digest
    if digest not in checked_output_digests:
        assert json.loads(value) == gold
        checked_output_digests.add(digest)


RUST_TIMER = re.compile(r'^Processing time \(w/o IO\):\s*([0-9]+(?:\.[0-9]+)?)\s*(ns|µs|μs|us|ms|s)\s*$',re.MULTILINE)
UNITS = {'ns':1e-9,'µs':1e-6,'μs':1e-6,'us':1e-6,'ms':1e-3,'s':1}


def timer(variant,stdout):
    if variant['kind']=='mech':
        values = [json.loads(line) for line in stdout.splitlines() if line.startswith('{')]
        values = [value for value in values if 'post_json_seconds' in value]
        assert len(values)==1 and values[0]['posts']==5000 and values[0]['sample']==0
        value = values[0]['post_json_seconds']
    else:
        values = RUST_TIMER.findall(stdout)
        assert len(values)==1
        value = float(values[0][0])*UNITS[values[0][1]]
    assert type(value) in (int,float) and math.isfinite(value) and value>0
    return value


def verify_primary(spec):
    raw = read(spec['record'])
    config = read(spec['config'])
    variants = {row['name']:row for row in raw['implementations']}
    assert set(variants)=={row['name']for row in config['implementations']}
    assert len(raw['samples'])==spec['exact_outputs']==31*len(variants)
    orders = raw['execution_order']['rounds']
    assert len(orders)==31
    values = {name:[]for name in variants}
    for index,order in enumerate(orders):
        assert len(order)==len(variants) and set(order)==set(variants)
        assert [row['variant']for row in raw['samples']if row['sample']==index]==order
    prefix = spec['record'].removesuffix('/latency.json')
    for sample in raw['samples']:
        name = sample['variant']
        assert sample['exit']==0 and sample['exact_rust_output']
        exact_output(f'{prefix}/work/{name}-{sample["sample"]}.json',sample['output_sha256'])
        elapsed = timer(variants[name],sample['stdout'])
        close(elapsed,sample['post_json_seconds']if variants[name]['kind']=='mech'else sample['seconds'])
        values[name].append(elapsed)
    medians = {name:statistics.median(samples)for name,samples in values.items()}
    eligible = [row['name']for row in variants.values()if row['kind']!='mech'
                and row.get('calculation_boundary')=='post_json_including_setup']
    serial = [name for name in eligible if 'fresh4'not in name]
    assert len(serial)==20
    assert eligible==spec['eligible_Rust_controls']
    reference = min(eligible,key=medians.__getitem__)
    serial_reference = min(serial,key=medians.__getitem__)
    assert reference==spec['Rust_reference']==raw['primary_reference']['variant']
    assert serial_reference==spec['original_20_serial_Rust_reference']
    close(medians[reference]*1000,spec['Rust_ms'])
    close(medians[serial_reference]*1000,spec['original_20_serial_Rust_ms'])
    for row in spec['rows']:
        name = row['implementation']
        close(row['post_json_ms'],medians[name]*1000)
        close(row['times_current_fastest_Rust'],medians[name]/medians[reference])
        close(row['times_original_20_serial_Rust'],medians[name]/medians[serial_reference])
        close(row['whole_process_ms'],statistics.median(sample['wall_seconds']for sample in raw['samples']if sample['variant']==name)*1000)
    return medians,reference,len(raw['samples'])


primary_values,primary_reference,primary_count = verify_primary(report['primary'])
primary_total = primary_count
for spec in report['earlier_primary_cohorts']:
    primary_total += verify_primary(spec)[2]
assert primary_total==2697

confirm_spec = report['confirmation']
confirm = read(confirm_spec['record'])
confirm_plan = read(confirm_spec['plan'])
assert hashlib.sha256(blob(confirm_spec['plan'])).hexdigest()==confirm['plan_sha256']
assert confirm['status']=='complete' and confirm['exact_outputs']==351 and len(confirm['samples'])==351
assert confirm_plan['candidate']==report['candidate']
assert confirm_plan['extra_controls']==[report['short_key_control']]
assert len(confirm_plan['implementations'])==27 and len(confirm_plan['rounds'])==13
assert {row['name']for row in confirm_plan['implementations']if row['kind']=='mech'}=={
    report['candidate'],report['control'],report['short_key_control']}
assert sum(row['original_20_serial_reference']for row in confirm_plan['implementations'])==20
assert sum(row['kind']!='mech'and row['post_json_eligible']for row in confirm_plan['implementations'])==22
retained_pin(confirm_plan['script']);retained_pin(confirm_plan['source_config'])
for pin in (confirm_plan['input'],confirm_plan['gold']):retained_pin(pin)
variants = {row['name']:row for row in confirm_plan['implementations']}
for row in variants.values():
    for key in ('source','receipt','adapter_source'):
        if key in row['pins']:retained_pin(row['pins'][key])
    for pin in row['pins'].get('bundle',{}).values():retained_pin(pin)
for index,order in enumerate(confirm_plan['rounds']):
    assert [row['variant']for row in confirm['samples']if row['round']==index]==order
confirm_prefix = confirm_spec['record'].removesuffix('/report.json')
for sample in confirm['samples']:
    name = sample['variant'];variant = variants[name]
    assert sample['exit']==0 and sample['exact_output'] and not sample['timed_out']
    stdout = blob(confirm_prefix+'/'+sample['stdout']['path']).decode()
    assert hashlib.sha256(stdout.encode()).hexdigest()==sample['stdout']['sha256']
    exact_output(confirm_prefix+'/'+sample['output']['path'],sample['output']['sha256'])
    elapsed = timer(variant,stdout)
    if variant['post_json_eligible']:close(elapsed,sample['post_json_seconds'])
    else:assert sample['post_json_seconds']is None
    assert sample['executable_sha256']==variant['pins']['executable']['sha256']
    assert sample['source_sha256']==variant['pins']['source']['sha256']
for name,value in confirm['summary'].items():
    rows = [row for row in confirm['samples']if row['variant']==name]
    assert len(rows)==13
    if variants[name]['post_json_eligible']:
        close(value['median_post_json_seconds'],statistics.median(row['post_json_seconds']for row in rows))
for label,count in (('original_20_serial_Rust',20),('current_22_eligible_Rust',22)):
    reference = confirm['references'][label]
    assert len(reference['eligible_variants'])==count
    winner = min(reference['eligible_variants'],key=lambda name:confirm['summary'][name]['median_post_json_seconds'])
    assert reference['variant']==winner
    close(reference['median_seconds'],confirm['summary'][winner]['median_post_json_seconds'])
for comparison in confirm['candidate_paired_comparisons']:
    by_pair = {(row['round'],row['variant']):row['post_json_seconds']for row in confirm['samples']if row.get('post_json_seconds')is not None}
    changes = [100*(by_pair[index,report['candidate']]/by_pair[index,comparison['reference']]-1)for index in range(13)]
    for a,b in zip(changes,comparison['paired_percent_changes']):close(a,b)
    close(statistics.median(changes),comparison['median_paired_percent_change'])
    assert sum(value<0 for value in changes)==comparison['candidate_faster_pairs']
    rng = random.Random(comparison['bootstrap_seed'])
    draws = sorted(statistics.median(rng.choices(changes,k=13))for _ in range(10_000))
    for a,b in zip((draws[249],draws[9749]),comparison['bootstrap_95_interval_percent']):close(a,b)
short_comparison = next(value for value in confirm['candidate_paired_comparisons']if value['reference']==report['short_key_control'])
assert short_comparison['bootstrap_95_interval_percent'][0]<0<short_comparison['bootstrap_95_interval_percent'][1]
assert report['storage_incremental_latency_gain_established']is False
host_comparison = next(value for value in confirm['candidate_paired_comparisons']if value['reference']==report['control'])
assert host_comparison['bootstrap_95_interval_percent'][1]<0
assert report['broad_host_to_selected_latency_gain_established']is True

resource_spec = report['resources']
resource = read(resource_spec['record']);resource_plan = read(resource_spec['plan'])
resource_prefix = resource_spec['record'].removesuffix('/report.json')
assert resource['status']=='complete'and resource['exact_outputs']==203 and len(resource['samples'])==203
assert hashlib.sha256(blob(resource_spec['plan'])).hexdigest()==resource['plan_sha256']
assert len(resource_plan['implementations'])==29 and len(resource_plan['rounds'])==7
retained_pin(resource_plan['script']);retained_pin(resource_plan['latency_authority'])
for pin in resource_plan['source_configs']+resource_plan['measurement_helpers']:retained_pin(pin)
variants = {row['name']:row for row in resource_plan['implementations']}
for index,order in enumerate(resource_plan['rounds']):
    assert [row['variant']for row in resource['samples']if row['round']==index]==order
for sample in resource['samples']:
    assert sample['exit']==0 and sample['exact_output'] and not sample['timed_out']
    exact_output(resource_prefix+'/'+sample['output']['path'],sample['output']['sha256'])
    assert 'post_json'in sample['windows'] or not variants[sample['variant']]['post_json_counter_eligible']
    for measurement in sample['windows'].values():
        assert measurement['allocator_accounting_underflows']==measurement['allocator_lifetime_accounting_underflows']==0
        assert measurement['hardware']['instructions_retired']>0 and measurement['hardware']['cycles']>0
    assert sample['whole_process_counters']['peak_resident_bytes']>0
for row in resource_spec['rows']:
    samples = [value for value in resource['samples']if value['variant']==row['implementation']]
    assert len(samples)==7
    for key in ('allocation_requests','requested_allocation_bytes','freed_allocation_bytes',
                'peak_additional_live_requested_heap_bytes'):
        close(row['post_json'][key],statistics.median(value['windows']['post_json'][key]for value in samples))
    for key in ('instructions_retired','cycles','resident_bytes_after','physical_footprint_bytes_after'):
        close(row['post_json']['hardware'][key],statistics.median(value['windows']['post_json']['hardware'][key]for value in samples))
    close(row['whole_process']['peak_resident_bytes'],statistics.median(value['whole_process_counters']['peak_resident_bytes']for value in samples))
assert report['resources']['free_call_count_available']is False


def summary_counts(record):
    matches = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',record['stdout'])
    assert len(matches)==1
    return tuple(map(int,matches[0]))


contracts = report['contracts']
focused = read(contracts['focused_record'])
assert all(value['exit']==0 for value in focused)
counts = [summary_counts(value)for value in focused]
assert tuple(sum(value[index]for value in counts)for index in range(3))==(129,0,1)
assert any(value['command'][1]=='program::image::'and summary_counts(value)==(27,0,0)for value in focused)
assert summary_counts(read(contracts['core_record']))==(399,0,0)
assert summary_counts(read(contracts['broad_record']))==(1051,16,11)
assert read(contracts['broad_failure_attribution'])['same_names']is True
assert contracts['broad_typed_ir']==326 and contracts['full_broad_green']is False
assert read(contracts['final_full5000'])['exit']==0
exact_output('retire-overlap/output.json',report['gold_sha256'])
edges = read('retire-overlap/edges/edge-fixtures.json')
assert len(edges)==31 and read(contracts['final_edge_cases'])['exit']==0
for edge in edges:
    assert edge['exit']==0
    name = edge['fixture']
    if 'exact_rust_output'in edge:
        assert edge['exact_rust_output']
        actual = read('retire-overlap/edges/edges/'+name+'/actual.json')
        expected = read('retire-overlap/edges/edges/'+name+'/related_posts_rust.json')
        assert actual==expected
    else:
        assert edge['exact_expected_output']
        data = read('production-commit/benchmarks/related-post-gen/fixtures/posts-'+name+'.json')
        expected = read('production-commit/benchmarks/related-post-gen/fixtures/expected-'+name+'.json')['related_indices']
        wanted = [{'_id':post['_id'],'tags':post['tags'],'related':[data[index]for index in indices]}
                  for post,indices in zip(data,expected)]
        assert read('retire-overlap/edges/actual-'+name+'.json')==wanted

metadata = read('retire-overlap/build/metadata.json')
assert (metadata['typed_ir_version'],metadata['work_policy_version'],metadata['abi_version'],metadata['native_descriptor_version'])==(331,126,42,11)
assert metadata['code_bytes']==1021740 and metadata['stack_bytes']==1568
assert hashlib.sha256(blob('retire-overlap/build/object.o')).hexdigest()==report['native_Object_sha256']
audit = read(report['native_link_audit'])
assert audit['status']=='passed'and audit['object']['sha256']==report['native_Object_sha256']
assert audit['every_native_symbol_has_exact_object_relative_address']is True
assert read('short-overlap/build/metadata.json')['typed_ir_version']==report['profile']['typed_ir']==328
assert read(report['profile']['record'])['diagnostic_only']is True


def verify_source(commit, files):
    names = sorted(files)
    process = subprocess.run(['git','cat-file','--batch'],cwd=ROOT,capture_output=True,check=True,
                             input=''.join(commit+':'+name+'\n'for name in names).encode()).stdout
    offset = 0
    for name in names:
        end = process.index(b'\n',offset);header = process[offset:end].split()
        assert len(header)==3 and header[1]==b'blob',(name,header)
        length = int(header[2]);offset=end+1
        assert hashlib.sha256(process[offset:offset+length]).hexdigest()==files[name],name
        offset+=length
        assert process[offset:offset+1]==b'\n';offset+=1
    assert offset==len(process)
    return len(names)


source_commit = report.get('published_source_equivalent_commit',report['compiler_source_commit'])
rebase = read('rebase-latest973/after.json')
assert rebase['old_source_commit']==report['compiler_source_commit']
assert rebase['published_equivalent_source_commit']==source_commit
assert rebase['all_2813_source_entries_identical']is True
upstream = read('upstream-compiler-audit-latest.json')
assert upstream['latest_PR973_base']==rebase['base']
assert upstream['production_source_changes']==[]
assert upstream['measured_production_includes_latest_PR973_compiler']is True
source_count = verify_source(source_commit,read(report['production_source_inventory'])['files'])
assert source_count==2813
verify_source(source_commit,{report['benchmark_source']:report['benchmark_source_sha256'],
                                               report['selected_adapter']:report['selected_adapter_sha256']})
ratio = primary_values[report['candidate']]/primary_values[primary_reference]
assert report['target_met']==(ratio<=1)and not report['target_met']
assert not report['whole_process_is_acceptance_gate']
assert primary_total+len(confirm['samples'])+len(resource['samples'])==report['timed_exact_outputs']==3251
result = {'verified_records':len(records),'unique_blobs':len(checked),'compressed_bytes':manifest['compressed_bytes'],
          'verified_source_commit':source_commit,
          'production_source_files':source_count,'primary_exact_outputs':primary_total,'confirmation_exact_outputs':351,
          'resource_exact_outputs':203,'qualification_edges':31,'focused_GPU_passed':129,'core_passed':399,
          'managed_primary_ms':primary_values[report['candidate']]*1000,'times_current_fastest_Rust':ratio,
          'managed_target_met':False,'incremental_storage_latency_gain_established':False}
print(json.dumps(result,indent=2))
