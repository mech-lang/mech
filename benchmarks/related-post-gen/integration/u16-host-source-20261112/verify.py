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
import zipfile

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
report = json.loads((D / 'report.json').read_bytes())
manifest = json.loads((D / 'records.json').read_bytes())
archives={}
member_container={}
for container in manifest['containers']:
 p=D/container['path'];assert p.resolve().is_relative_to(D)
 raw=p.read_bytes();assert len(raw)==container['bytes']<45_000_000
 assert hashlib.sha256(raw).hexdigest()==container['sha256']
 z=zipfile.ZipFile(p);archives[container['path']]=z
 members=z.infolist();assert len(members)==container['members']
 for member in members:
  assert member.filename not in member_container
  assert member.compress_type==zipfile.ZIP_STORED
  member_container[member.filename]=container['path']
records={};digests={};checked=set()
for row in manifest['records']:
 assert row['source'] not in records
 assert not row['path'].startswith('/') and '..' not in Path(row['path']).parts
 assert member_container[row['path']]==row['container']
 if row['path'] not in checked:
  packed=archives[row['container']].read(row['path'])
  assert len(packed)==row['compressed_bytes']
  assert hashlib.sha256(packed).hexdigest()==row['compressed_sha256']
  raw=gzip.decompress(packed)
  assert len(raw)==row['uncompressed_bytes']
  assert hashlib.sha256(raw).hexdigest()==row['uncompressed_sha256']
  checked.add(row['path'])
 records[row['source']]=row;digests[row['uncompressed_sha256']]=row['path']
assert len(records)==manifest['logical_records']
assert len(checked)==manifest['unique_blobs']
assert set(member_container)==checked
assert sum(archives[c].getinfo(p).file_size for p,c in member_container.items())==manifest['compressed_bytes']


@lru_cache(maxsize=128)
def data(path):
    return gzip.decompress(archives[member_container[path]].read(path))


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
 # Every ordinary managed variant must have an exact own-source authority.
 if '-compact-' not in name:
  assert name in report['managed_work_qualifications'],name
  q=report['managed_work_qualifications'][name]
  assert value['parallel_tasks']==4
  assert value['parallel_numeric_work']==q['numeric_work']
  assert value['parallel_numeric_regions']==q['numeric_regions']==8
  if q['exact_total_required']:assert value['work']==q['work'],name

def jstdout(stdout):return next(json.loads(line)for line in stdout.splitlines()if line.startswith('{'))
observations = {}
all_indexed = {}


def observe(cohort, name, value):
    ledger(name, value)
    result = observations.setdefault(cohort, {}).setdefault(name, dict(peak_workers={}, total_work={}))
    for field, key in [('parallel_peak_workers', 'peak_workers'), ('work', 'total_work')]:
        if field in value:
            text = str(value[field])
            result[key][text] = result[key].get(text, 0) + 1


def primary(cohort):
    prefix = cohort['prefix']
    value, config = read(cohort['record']), read(cohort['plan'])
    run = read(prefix + '/run.json')
    protocol = {'seed': int(run['command'][run['command'].index('--seed') + 1])}
    variants = {v['name']: v for v in value['implementations']}
    assert len(variants) == cohort['variants']
    assert len(value['samples']) == cohort['outputs']
    rounds = cohort['rounds']
    assert value['execution_order']['rounds'] == orders(config['implementations'], protocol['seed'], rounds)
    for i, order in enumerate(value['execution_order']['rounds']):
        assert [r['variant'] for r in value['samples'] if r['sample'] == i] == order
    indexed, timings = {}, {}
    for row in value['samples']:
        name = row['variant']
        assert row['exit'] == 0 and row['exact_rust_output'] is True
        exact(blob(f'{prefix}/runs/{name}-{row["sample"]}.json'), row['output_sha256'])
        seconds = elapsed(variants[name], row['stdout'])
        indexed[row['sample'], name] = seconds
        timings.setdefault(name, []).append(seconds)
        if variants[name]['kind'] == 'mech':
            observe(prefix, name, jstdout(row['stdout']))
            close(seconds, row['post_json_seconds'])
        else:
            close(seconds, row['seconds'])
        assert process(blob(prefix + '/runs/' + Path(row['process_resources_raw']).name).decode()) == row['process_resources']
    med = {name: statistics.median(rows) for name, rows in timings.items()}
    for name, variant in variants.items():
        rows = [r for r in value['samples'] if r['variant'] == name]
        summary = value['summary'][name]
        assert len(rows) == summary['samples'] == rounds and summary['all_exact_rust'] is True
        close(summary['median_seconds'], statistics.median(r['seconds'] for r in rows))
        close(summary['median_whole_process_seconds'], statistics.median(r['wall_seconds'] for r in rows))
        assert summary['process_resources_median'] == medians([r['process_resources'] for r in rows])
        if variant.get('calculation_boundary') == 'post_json_including_setup':
            close(summary['median_post_json_seconds'], med[name])
    eligible = [v['name'] for v in variants.values() if v['kind'] != 'mech' and v.get('calculation_boundary') == 'post_json_including_setup']
    assert len(eligible) == 22
    reference = min(eligible, key=med.__getitem__)
    assert reference == value['primary_reference']['variant']
    close(med[reference], value['primary_reference']['median_seconds'])
    paired = read(prefix + '/paired-comparisons.json')
    assert paired['report_sha256'] == sha(blob(cohort['record']))
    for pair in paired['comparisons']:
        changes = [100 * (indexed[i, pair['candidate']] / indexed[i, pair['control']] - 1) for i in range(rounds)]
        close(statistics.median(changes), pair['median_percent'])
        assert pair['candidate_faster_pairs'] == sum(x < 0 for x in changes)
        rng = random.Random(pair['bootstrap_seed'])
        draws = sorted(statistics.median(rng.choices(changes, k=rounds)) for _ in range(paired['bootstrap_replicates']))
        for actual, expected in zip([draws[250], draws[9749]], pair['percentile95']):
            close(actual, expected)
        assert pair['gain_established'] == (pair['percentile95'][1] < 0)
        assert pair['regression_established'] == (pair['percentile95'][0] > 0)
    run = read(prefix + '/run.json')
    assert run['exit'] == 0
    assert run['config_sha256'] == sha(blob(prefix + '/config.json'))
    assert run['expected_outputs'] == cohort['outputs']
    assert run['command'][run['command'].index('--seed') + 1] == str(protocol['seed'])
    if 'protocol_sha256' in run:
        assert run['protocol_sha256'] == sha(blob(prefix + '/protocol.json'))
    assert run['script_sha256'] == sha(blob('published-base/benchmarks/related-post-gen/compare-implementations.py'))
    all_indexed[prefix] = indexed


def resources(cohort):
    prefix = cohort['prefix']
    value, plan = read(cohort['record']), read(cohort['plan'])
    variants = {v['name']: v for v in plan['implementations']}
    assert len(variants) == cohort['variants']
    assert len(value['samples']) == value['exact_outputs'] == cohort['outputs']
    assert sha(blob(cohort['plan'])) == value['plan_sha256']
    assert plan['rounds'] == orders(plan['implementations'], plan['seed'], cohort['rounds'])
    assert value['samples'] == [json.loads(line) for line in blob(prefix + '/samples.jsonl').splitlines()]
    assert value['latency_selection_forbidden'] is True and value['whole_process_is_acceptance_gate'] is False
    for i, order in enumerate(plan['rounds']):
        assert [r['variant'] for r in value['samples'] if r['round'] == i] == order
    for row in value['samples']:
        assert row['exit'] == 0 and row['timed_out'] is False and row['exact_output'] is True
        exact(pin(prefix, row['output']))
        stdout = pin(prefix, row['stdout']).decode()
        pin(prefix, row['stderr'])
        assert process(pin(prefix, row['process_resources_raw']).decode()) == row['whole_process_counters']
        variant = variants[row['variant']]
        if variant['kind'] == 'mech':
            native = jstdout(stdout)
            assert native == row['raw_adapter_record']
            observe(prefix, variant.get('latency_control_name', variant['name'].removesuffix('-instrumented')), native)
            assert row['windows']['post_json'] == native['measurement']
            close(row['instrumented_post_json_seconds_diagnostic_only'], native['post_json_seconds'])
            for raw, label in [('restore_measurement', 'restore'), ('json_measurement', 'json'), ('admission_measurement', 'admission')]:
                if raw in native:
                    assert row['windows'][label] == native[raw]
        elif variant['instrumented']:
            native = json.loads(next(line[len('PROCESS_METRICS '):] for line in stdout.splitlines() if line.startswith('PROCESS_METRICS ')))
            assert native == row['raw_adapter_record']
            label = 'post_json' if variant['post_json_counter_eligible'] else 'excluded_processing'
            assert row['windows'][label] == native.get('processing', native)
        for window in row['windows'].values():
            assert window['allocator_accounting_underflows'] == window['allocator_lifetime_accounting_underflows'] == 0
            assert window['hardware']['instructions_retired'] > 0 and window['hardware']['cycles'] > 0
    for name, summary in value['summary'].items():
        rows = [r for r in value['samples'] if r['variant'] == name]
        assert len(rows) == summary['samples'] == cohort['rounds'] and summary['all_exact'] is True
        assert medians([r['whole_process_counters'] for r in rows]) == summary['whole_process_medians']
        for label, expected in summary['window_medians'].items():
            assert medians([r['windows'][label] for r in rows]) == expected


def confirmation_pair(pair, candidate, indexed, rounds):
    changes = [100 * (indexed[i, candidate] / indexed[i, pair['reference']] - 1) for i in range(rounds)]
    assert changes == pair['paired_percent_changes']
    close(statistics.median(changes), pair['median_paired_percent_change'])
    assert pair['candidate_faster_pairs'] == sum(x < 0 for x in changes)
    rng = random.Random(pair['bootstrap_seed'])
    draws = sorted(statistics.median(rng.choices(changes, k=rounds)) for _ in range(pair['bootstrap_draws']))
    for actual, expected in zip([draws[249], draws[9749]], pair['bootstrap_95_interval_percent']):
        close(actual, expected)


def confirmation(cohort):
    prefix = cohort['prefix']
    value, plan = read(cohort['record']), read(cohort['plan'])
    variants = {v['name']: v for v in plan['implementations']}
    rounds = cohort['rounds']
    assert len(variants) == cohort['variants']
    assert len(value['samples']) == value['exact_outputs'] == value['expected_outputs'] == cohort['outputs']
    assert sha(blob(cohort['plan'])) == value['plan_sha256']
    assert plan['rounds'] == orders(plan['implementations'], plan['seed'], rounds)
    assert value['same_machine_independent_orchestration'] is True and value['whole_process_is_acceptance_gate'] is False
    assert value['samples'] == [json.loads(line) for line in blob(prefix + '/samples.jsonl').splitlines()]
    for i, order in enumerate(plan['rounds']):
        assert [r['variant'] for r in value['samples'] if r['round'] == i] == order
    indexed = {}
    for row in value['samples']:
        assert row['exit'] == 0 and row['timed_out'] is False and row['exact_output'] is True
        exact(pin(prefix, row['output']))
        stdout = pin(prefix, row['stdout']).decode()
        pin(prefix, row['stderr'])
        name = row['variant']
        seconds = elapsed(variants[name], stdout)
        close(row['whole_process_seconds'], row['whole_process_nanoseconds'] / 1e9)
        if variants[name]['kind'] == 'mech':
            observe(prefix, name, jstdout(stdout))
        if row['post_json_eligible']:
            close(seconds, row['post_json_seconds'])
            indexed[row['round'], name] = seconds
        else:
            assert row['post_json_seconds'] is None
    for name, summary in value['summary'].items():
        rows = [r for r in value['samples'] if r['variant'] == name]
        assert len(rows) == summary['samples'] == summary['exact_outputs'] == rounds
        close(summary['median_whole_process_seconds'], statistics.median(r['whole_process_seconds'] for r in rows))
        if summary['post_json_eligible']:
            close(summary['median_post_json_seconds'], statistics.median(r['post_json_seconds'] for r in rows))
    for label, reference in value['references'].items():
        metric = 'median_whole_process_seconds' if label.startswith('whole_process') else 'median_post_json_seconds'
        winner = min(reference['eligible_variants'], key=lambda n: value['summary'][n][metric])
        assert winner == reference['variant']
        close(reference['median_seconds'], value['summary'][winner][metric])
    for pair in value['candidate_paired_comparisons']:
        confirmation_pair(pair, value['candidate'], indexed, rounds)
    if prefix + '/additional-paired-comparisons.json' in records:
        additional = read(prefix + '/additional-paired-comparisons.json')
        assert additional['report_sha256'] == sha(blob(cohort['record']))
        for pair in additional['comparisons']:
            confirmation_pair(pair, pair['candidate'], indexed, rounds)
    # Historical extra-control target flags are retained verbatim, not the fixed-example completion gate.
    all_indexed[prefix] = indexed


for name,q in report['managed_work_qualifications'].items():
 raw=blob(q['record']);assert sha(raw)==q['sha256'];value=json.loads(raw)
 if q.get('authority_kind')=='retained_latency':
  rows=[r for r in value['samples']if r['variant']==name];assert rows
  for row in rows:ledger(name,jstdout(row['stdout']))
 else:
  assert value['exit']==0;ledger(name,jstdout(value['stdout']))
managed_names={v['name'].removesuffix('-instrumented')for c in report['cohorts']for v in read(c['plan'])['implementations']if v['kind']=='mech'and '-compact-'not in v['name']}
assert managed_names==set(report['managed_work_qualifications'])
assert len(managed_names)==22
for cohort in report['cohorts']:globals()[cohort['kind']](cohort)
assert sum(x['outputs']for x in report['cohorts'])==report['timed_resource_exact_outputs']==5063
for table in report['timing_tables']:
 value=read(table['cohort']+('/latency.json'if table['cohort'].startswith('first-')else'/report.json'))
 for row in table['rows']:
  s=value['summary'][row['variant']];seconds=s.get('median_post_json_seconds')
  if seconds is None:assert row['post_json_ms'] is row['times_extra_current_fastest_custom_Rust'] is None
  else:
   close(row['post_json_ms'],seconds*1000);close(row['times_extra_current_fastest_custom_Rust'],seconds/table['legacy_extra_reference']['median_seconds'])
  close(row['whole_process_ms'],s['median_whole_process_seconds']*1000)
for table in report['resource_tables']:
 value=read(table['cohort']+'/report.json')
 for row in table['rows']:
  assert row['post_json_window']==value['summary'][row['variant']]['window_medians'].get('post_json')
  assert row['whole_process']==value['summary'][row['variant']]['whole_process_medians']

# Qualifiers are separate executions, not timing samples. Focused derivative
# source contracts retain their own non-32-output schema and are not mixed in.
def edge_set(prefix):
 edges=read(prefix+'/edge-fixtures.json');assert len(edges)==31
 for edge in edges:
  assert edge['exit']==0;name=edge['fixture']
  if edge.get('exact_rust_output'):
   assert read(prefix+'/edges/'+name+'/actual.json')==read(prefix+'/edges/'+name+'/related_posts_rust.json')
  else:
   assert edge['exact_expected_output']is True
   posts=read('published-base/benchmarks/related-post-gen/fixtures/posts-'+name+'.json')
   ids=read('published-base/benchmarks/related-post-gen/fixtures/expected-'+name+'.json')['related_indices']
   expected=[{'_id':p['_id'],'tags':p['tags'],'related':[posts[i]for i in neighbors]}for p,neighbors in zip(posts,ids)]
   assert read(prefix+'/actual-'+name+'.json')==expected
qprefixes={str(Path(q['record']).parent)for q in report['qualifications']}
edgeprefixes=[n.removesuffix('/edge-fixtures.json')for n in records if n.endswith('/edge-fixtures.json')and any(n.startswith(p+'/')for p in qprefixes)and not n.startswith('referenced-inputs/')]
for prefix in edgeprefixes:edge_set(prefix)
qualified_adapters=0
for q in report['qualifications']:
 value=read(q['record']);prefix=str(Path(q['record']).parent)
 if q['kind']=='managed':
  assert len(value['results'])*32==q['outputs']
  for row in value['results']:
   assert row['full_records']==5000 and row['exact_gold']is True;exact(pinned(row['output']))
   audit=json.loads(pinned(row['link_audit']));assert audit['status']=='passed'
   assert audit['every_native_symbol_has_exact_object_relative_address']is True
   assert audit['all_text_equal_except_independently_verified_relocation_fields']is True
   receipt=json.loads(pinned(row['build_receipt']));assert receipt['exit']==0
   assert receipt['executable_sha256']==row['executable']['sha256']==audit['executable']['sha256']
  qualified_adapters+=len(value['results'])
 else:
  assert value['status']=='passed'and value['correctness_executions']==q['outputs']
  assert len(value['records'])*32==q['outputs']
  for row in value['records']:
   assert row.get('full5000_exact',row.get('full_exact'))is True and row['edge_cases']==31
   label=row['label'];form=row.get('form');output=prefix+'/'+label+('/'+form+'.output.json'if form else'/output.json');exact(blob(output))
   audit=json.loads(pinned(row['link_audit']));assert audit['status']=='passed'
   assert json.loads(pinned(row['build_receipt']))['exit']==0
  qualified_adapters+=len(value['records'])
assert qualified_adapters*32==report['qualification_exact_outputs_separate']
assert len(edgeprefixes)==qualified_adapters

# Literal source closure uses published Git objects plus archived minimal overrides.
import difflib
def git_files(commit,names,missing=False):
 raw=subprocess.run(['git','cat-file','--batch'],cwd=ROOT,check=True,capture_output=True,input=''.join(commit+':'+n+'\n'for n in names).encode()).stdout;cursor=0;result={}
 for name in names:
  end=raw.index(b'\n',cursor);head=raw[cursor:end].split();cursor=end+1
  if head[-1]==b'missing':assert missing;continue
  assert len(head)==3 and head[1]==b'blob';size=int(head[2]);result[name]=raw[cursor:cursor+size];cursor+=size;assert raw[cursor:cursor+1]==b'\n';cursor+=1
 assert cursor==len(raw);return result
closure=report['source_closure'];bases={};reconstructed={};builds={}
for label,base in closure['bases'].items():
 inv=read(base['inventory_record']);source=git_files(base['commit'],sorted(inv['files']));assert all(sha(source[n])==h for n,h in inv['files'].items());bases[label]=source
for label,entry in closure['compilers'].items():
 inv=read(entry['inventory_record']);assert sha(json.dumps(inv['files'],sort_keys=True).encode())==inv['sha256']
 base=bases[entry['base']];source={};parts=[]
 for name,digest in inv['files'].items():
  raw=blob('source-overrides/'+label+'/'+name)if name in entry['overrides']else base[name];assert sha(raw)==digest,(label,name);source[name]=raw
 assert len(source)==entry['source_count']
 assert {n:sha(raw)for n,raw in source.items()if base.get(n)!=raw}==entry['overrides']
 for name in entry['overrides']:
  parts.append('diff --git a/'+name+' b/'+name+'\n');parts.extend(difflib.unified_diff(base.get(name,b'').decode().splitlines(True),source[name].decode().splitlines(True),fromfile='a/'+name if name in base else'/dev/null',tofile='b/'+name))
 assert ''.join(parts).encode()==blob(entry['patch_record']);reconstructed[label]=source
 build=read(label+'/build.json');assert build['source_sha256']==inv['sha256'];assert len(build['stages'])==12 and all(s['exit']==0 for s in build['stages']);builds[label]=build
vendor=read(closure['vendor_inventory']);assert len(vendor)==closure['vendor_files']==246
vbase=git_files(closure['vendor_base_commit'],sorted(vendor),True)
for n,h in vendor.items():
 raw=blob('vendor-overrides/'+n)if n in closure['vendor_overrides']else vbase[n];assert sha(raw)==h
 for source in reconstructed.values():
  if n in source:assert source[n]==raw
literal=read(closure['literal_inventory'])['literal_includes'];assert len(literal)==closure['literal_includes']
assert all(sha(raw)==literal[n]for n,raw in git_files(closure['vendor_base_commit'],sorted(literal)).items())
def externs(cmd):return dict(cmd[i+1].split('=',1)for i,v in enumerate(cmd[:-1])if v=='--extern')
fresh=read('comparison-chain-compiler-v3/backend-libraries.json')
for label,build in builds.items():
 assert build['backend_libraries']==fresh
 actual=externs(next(s for s in build['stages']if s['crate']=='mech_gpu')['command'])
 for n in ['cranelift_codegen','cranelift_frontend','cranelift_jit','cranelift_module','cranelift_native','cranelift_object']:assert actual[n]==fresh[n]['path']

# Final coherent355 test authority: all focused starts and summaries, plus
# inherited failure identities. These are retained runs, never rerun here.
def counts(v):
 found=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',v['stdout']);assert len(found)==1
 return tuple(map(int,found[0]))
def stdout_records(v):
 if isinstance(v,dict):
  if isinstance(v.get('stdout'),str):yield v
  for child in v.values():yield from stdout_records(child)
 elif isinstance(v,list):
  for child in v:yield from stdout_records(child)
focused=list(stdout_records(read('u16-exact-histogram-compiler-v3/focused-contracts.json')))
assert focused and all(v['exit']==0 and counts(v)[1:]==(0,0)for v in focused)
starts=[n for v in focused for n in re.findall(r'\btest (native_artifact::\S+) \.\.\.',v['stdout'])]
assert len(starts)==sum(counts(v)[0]for v in focused)
distinct=read('u16-exact-histogram-compiler-v3/focused-distinct-tests.json')
assert set(starts)==set(distinct['passed'])and distinct['count']==79
broad=read('u16-exact-histogram-compiler-v3/broad-native-tests.json')
assert broad['exit']==101 and counts(broad)==(1108,16,11)and'35 filtered out'in broad['stdout']
failed=lambda v:sorted(re.findall(r'^    (native_artifact::[^\n]+)$',v['stdout'],re.MULTILINE))
baseline=read('selected-increment-compiler-v4/broad-native-tests.json')
assert len(failed(broad))==16 and failed(broad)==failed(baseline)
backend_test=read('u16-exact-histogram-compiler-v3/tests.build.json');assert backend_test['exit']==0
actual=externs(backend_test['command'])
for n in ['cranelift_codegen','cranelift_frontend','cranelift_jit','cranelift_module','cranelift_native','cranelift_object']:assert actual[n]==fresh[n]['path']

# Formal fixed-example provenance. Preserve all historical reference names and
# timer classifications; the20 custom source experiments are extra data only.
prov='fixed-benchmark-provenance-v1/'
classification=read(prov+'classification.json');audit=read(prov+'audit.json')
assert audit['classification_counts']=={'current24':24,'custom_source20':20,'unchanged_repository_source4':4}
assert audit['upstream_commit']=='fc2982b0c929b1b701209d401a1dc30c773ef4ab'
provenance_manifest=read(prov+'manifest.json')
assert len(provenance_manifest['files'])==33
for name,pin_value in provenance_manifest['files'].items():
 raw=blob(prov+name);assert len(raw)==pin_value['bytes']and sha(raw)==pin_value['sha256']
current_config=read('first-u16-host-mask-source-gains-v1/config.json');current_variants={v['name']:v for v in current_config['implementations']}
assert len(classification['variants'])==24
original_count=custom_count=0
for variant in classification['variants']:
 raw=blob(prov+variant['source_artifact']);assert len(raw)==variant['source_bytes']and sha(raw)==variant['source_sha256']
 actual=current_variants[variant['name']]
 for key in ['source_sha256','executable_sha256','executable_file_bytes']:assert actual[key]==variant[key]
 if variant['classification']=='unchanged_repository_example_source':
  original_count+=1
  assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==variant['git_blob_sha1']
 else:
  custom_count+=1;assert variant['classification']=='local_custom_experiment'
assert(original_count,custom_count)==(4,20)
for pin_value in audit['reports'].values():
 packed=blob(prov+pin_value['artifact']);assert sha(packed)==pin_value['stored_sha256']
 raw=gzip.decompress(packed);assert len(raw)==pin_value['bytes']and sha(raw)==pin_value['sha256']
qa_prefix='u16-host-source-independent-audit-v1/'
qa_manifest=read(qa_prefix+'manifest.json')
for name,pin_value in qa_manifest['files'].items():
 raw=blob(qa_prefix+name);assert len(raw)==pin_value['bytes']and sha(raw)==pin_value['sha256']
qa=read(report['independent_QA']);assert qa['status']=='PASS'and not qa['findings']
assert qa['checks']['six_cohorts_exact_outputs']==5063 and qa['checks']['qualification_output_total']==3040
assert qa['checks']['managed_own_source_ledgers']==22 and qa['checks']['packet_merged_22_label_map_matches_original_authorities']is True
assert set(report['source_closure_roles'])==set(reconstructed)
assert sha(blob('publication-source/benchmarks/related-post-gen/mech/related_guarded_u16.mec'))==current_variants['mech-ir355-managed-guarded-u16-4']['source_sha256']=='6a3022a0385efd1bf3dcdc9a76409607d20237260b21bf6745416d84da3d027a'
tradeoffs=report['material_tradeoffs'];primary_summary=read('first-u16-host-mask-source-gains-v1/latency.json')['summary'];confirmation_summary=read('independent-u16-host-mask-confirmation-v1/report.json')['summary']
for name in ['managed334','managed355_U16']:
 entry=tradeoffs[name];assert primary_summary[entry['variant']]['process_resources_median']['peak_resident_bytes']==entry['primary_peak_resident_bytes']
for name in ['managed334','managed355_U16','compact353_U16','original_rust_con']:
 entry=tradeoffs[name];assert abs(confirmation_summary[entry['variant']]['median_whole_process_seconds']*1000-entry['confirmation_whole_process_ms'])<0.000001
entry=tradeoffs['managed355_U16'];variant=current_variants[entry['variant']]
assert(entry['native_code_bytes'],entry['root_stack_bytes'],entry['ordinary_executable_bytes'])==(variant['code_bytes'],variant['stack_bytes'],variant['executable_file_bytes'])
assert tradeoffs['compact353_U16']['executable_bytes']==current_variants['mech-ir353-compact-guarded-u16-4']['executable_file_bytes']
assert len(blob('guarded-u16-compact-preparation-v2/guarded-u16/emitted/original.o'))==tradeoffs['compact353_U16']['native_Object_bytes']
fixed=read('first-u16-host-mask-source-gains-v1/fixed-benchmark-comparison.json')
assert fixed['raw_reports_unchanged']is True and fixed['all31_and13_observations_retained']is True
assert fixed['primary_sha256']==sha(blob('first-u16-host-mask-source-gains-v1/latency.json'))
assert fixed['independent_sha256']==sha(blob('independent-u16-host-mask-confirmation-v1/report.json'))
for pair in fixed['comparisons']:
 prefix='first-u16-host-mask-source-gains-v1'if pair['cohort']=='primary'else'independent-u16-host-mask-confirmation-v1'
 c=next(x for x in report['cohorts']if x['prefix']==prefix);value=read(c['record']);plan=read(c['plan']);variants={v['name']:v for v in plan['implementations']};rounds=c['rounds'];indexed={}
 for r in value['samples']:
  name=r['variant']
  if name not in [pair['variant'],pair['reference']]:continue
  if c['kind']=='primary':stdout=r['stdout'];i=r['sample']
  else:stdout=pin(prefix,r['stdout']).decode();i=r['round']
  indexed[i,name]=elapsed(variants[name],stdout)
 changes=[100*(indexed[i,pair['variant']]/indexed[i,pair['reference']]-1)for i in range(rounds)]
 reference=statistics.median(indexed[i,pair['reference']]for i in range(rounds));candidate=statistics.median(indexed[i,pair['variant']]for i in range(rounds))
 close(pair['reference_median_ms'],reference*1000);close(pair['median_ms'],candidate*1000);close(pair['multiple_of_fixed_example'],candidate/reference);close(pair['speedup'],reference/candidate);close(pair['median_paired_percent'],statistics.median(changes));assert pair['faster_pairs']==sum(x<0 for x in changes)
 rng=random.Random(pair['seed']);draws=sorted(statistics.median(rng.choices(changes,k=rounds))for _ in range(10000))
 for actual,expected in zip([draws[249],draws[9749]],pair['bootstrap95_percent']):close(actual,expected)
assert sha(blob('packet-report.json'))==sha((D/'report.json').read_bytes())
print(json.dumps(dict(status='passed',verified_records=len(records),unique_blobs=len(checked),archive_shards=len(archives),compressed_bytes=manifest['compressed_bytes'],timed_resource_exact_outputs=5063,qualification_exact_outputs=report['qualification_exact_outputs_separate'],qualified_adapters=qualified_adapters,managed_work_labels=len(report['managed_work_qualifications']),reconstructed_compiler_snapshots=len(reconstructed),vendor_files=246,literal_includes=len(literal),focused355_distinct_passed=79,broad355_passed=1108,inherited_failure_names_matched=16,exact355_test_backend_V3=True,fixed_benchmark_goal_met=True,native_executions_by_verifier=0),indent=2))
