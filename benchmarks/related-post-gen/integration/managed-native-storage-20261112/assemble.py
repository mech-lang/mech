#!/usr/bin/env python3
"""Preserve the completed IR331 experiment without reading the current compiler.

Historical absolute paths identify original evidence. Relocate those roots to
reassemble elsewhere. This script never launches a compiler or benchmark.
"""
from pathlib import Path
import gzip
import hashlib
import json
import statistics
import subprocess

D = Path(__file__).resolve().parent
S = D.parents[3]
M = Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
COMMIT = '708511df16e803b5947e84574ba61e3f6d994737'
PUBLISHED_EQUIVALENT = 'f361f91f298b87ebb9bee880930951fdf79b0d5f'
BASELINE = 'e5f324e87c6dadaf0f7bb341cca7b8ef7e69a9ef'
INPUT = Path('/private/tmp/mech-related-narrow-updates-20261111/frozen-inputs/posts.json')
GOLD = Path('/private/tmp/mech-related-compiler-catchup-evidence-20261111/runners/selected/output.json')
CANDIDATE = 'mech-managed-retire-overlap'
CONTROL = 'mech-managed-host-v2-4'
SHORT = 'mech-managed-short-overlap'
CONFIRM = 'independent-managed-retire-overlap-prepared'
RESOURCE = 'new-managed-resources/prepared-retire'
if (D/'records.json').exists():
    raise SystemExit('Refusing to overwrite an assembled evidence packet')
(D/'records').mkdir(exist_ok=True)
records = []
blobs = {}
inodes = {}
logical_names = set()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(Path(path).read_bytes())


def add_bytes(data, name):
    if name in logical_names:
        return
    h = sha(data)
    if h not in blobs:
        packed = gzip.compress(data, compresslevel=9, mtime=0)
        dest = 'records/'+h+'.gz'
        (D/dest).write_bytes(packed)
        blobs[h] = {'path':dest, 'compressed_sha256':sha(packed), 'compressed_bytes':len(packed),
                    'uncompressed_sha256':h, 'uncompressed_bytes':len(data)}
    logical_names.add(name)
    records.append({'source':name, **blobs[h]})


def add(path, name=None):
    path = Path(path)
    if not path.is_file():
        return
    name = name or str(path.relative_to(M))
    if name in logical_names:
        return
    stat = path.stat()
    identity = (stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns)
    if identity in inodes:
        logical_names.add(name)
        records.append({'source':name, **blobs[inodes[identity]]})
    else:
        data = path.read_bytes()
        add_bytes(data, name)
        inodes[identity] = sha(data)


def tree(root, allowed=None):
    root = Path(root)
    if root.exists():
        for path in sorted(root.rglob('*')):
            if path.is_file() and (allowed is None or path.suffix in allowed or path.name == 'Cargo.lock'):
                add(path)


def git_bytes(path, commit=PUBLISHED_EQUIVALENT):
    return subprocess.run(['git','show',commit+':'+path], cwd=S, capture_output=True, check=True).stdout


for label in ('first-short-worker','first-prefix-overlap','first-arena-retire'):
    # Includes prospective rules, orders, all original stdout/stderr, per-round
    # complete JSON outputs and the content-addressed full Gold producer bytes.
    tree(M/label, {'.json','.txt','.jsonl','.py'})
for label in (CONFIRM, RESOURCE):
    tree(M/label)
for label in ('host-v2-4','short-keys','worker-omit','short-worker','short-overlap',
              'prefix-short','prefix-short-worker','arena-overlap','retire-overlap'):
    folder = M/label
    if not folder.exists():
        continue
    for path in sorted(folder.iterdir()):
        if path.is_file() and path.suffix in ('.json','.rs','.mec','.txt'):
            add(path)
    tree(folder/'build', {'.json','.rs','.mec','.mecb','.o','.bin','.txt'})
    tree(folder/'edges', {'.json','.rs','.mec','.txt'})
# This is an IR328 diagnostic profile, not a new IR331 qualification profile.
tree(M/'short-overlap/profile', {'.json','.txt','.rs'})
for label in ('host-compiler-v2','prefix-compiler-v1','short-key-compiler','short-worker-compiler',
              'short-overlap-compiler','prefix-short-compiler','arena-overlap-compiler','retire-overlap-compiler'):
    folder = M/label
    if not folder.exists():
        continue
    for path in sorted(folder.glob('*.json')):
        add(path)
    # Frozen GPU source snapshots retain unselected/compiler-control revisions.
    # The selected complete production source is also verified from Git history.
    tree(folder/'source')
for name in ('native-failure-attribution.json','unchanged-native-failure-comparison.json',
             'independent-managed-confirm.py','independent-managed-confirm-v2.py',
             'independent-managed-protocol-review.json','measurement-source-pin.json'):
    add(M/name)
add(M/'new-managed-resources/run.py')
add(M/'rebase-latest973/after.json')
add(M/'upstream-compiler-audit-latest.json')
# Fresh four-worker Rust controls retain source, manifests/lockfiles, measurement
# helper, exact tests/qualification/build receipts. Executables/libraries are
# represented by hashes and sizes rather than duplicated in this packet.
allowed = {'.json','.rs','.toml','.lock','.py','.patch','.txt'}
for path in sorted((M/'rust-fresh4-control').rglob('*')):
    if path.is_file() and 'target' not in path.parts and path.suffix in allowed:
        add(path)
for label in ('string-geometry-audit','short-overlap-candidate'):
    tree(M/label)
add(INPUT, 'publication-inputs/posts.json')
add(GOLD, 'publication-inputs/gold.json')
add('/private/tmp/mech-related-compact-native-development-evidence-20261111/first-v30/resources-config.json', 'publication-inputs/original-resource-config.json')
for label in ('first-short-worker','first-prefix-overlap','first-arena-retire'):
    config = read(M/label/'config.json')
    for row in config['implementations']:
        if row['kind'] != 'mech':
            for key in ('source','receipt'):
                if row.get(key) and Path(row[key]).is_file():
                    suffix = 'source.rs' if key=='source' else 'receipt.json'
                    add(row[key], 'references/'+row['name']+'/'+suffix)
resource_plan = read(M/RESOURCE/'plan.json')
for row in resource_plan['implementations']:
    if row['kind'] != 'mech':
        for key in ('source','receipt'):
            pin = row['pins'].get(key)
            if pin:
                add(pin['path'], 'references/'+row['name']+'/'+('source.rs' if key=='source' else 'receipt.json'))
for path in ('benchmarks/related-post-gen/mech/related_optimized.mec',
             'benchmarks/related-post-gen/hosts/managed-4.rs',
             'benchmarks/related-post-gen/measurement.rs',
             'benchmarks/related-post-gen/compare-implementations.py',
             'benchmarks/related-post-gen/audit-native-link.py'):
    add_bytes(git_bytes(path), 'production-commit/'+path)


for name in ('first-10','first-100','first-500','unicode-ties','empty-tags'):
    for prefix in ('posts-','expected-'):
        path = 'benchmarks/related-post-gen/fixtures/'+prefix+name+'.json'
        add_bytes(git_bytes(path), 'production-commit/'+path)
add_bytes(git_bytes('benchmarks/related-post-gen/experiments/check_subset3.py'),
          'production-commit/benchmarks/related-post-gen/experiments/check_subset3.py')


def timer(sample):
    return sample['post_json_seconds'] if sample['variant'].startswith('mech') else sample['seconds']


def primary_summary(label):
    raw = read(M/label/'latency.json')
    values = {row['name']:statistics.median(timer(s) for s in raw['samples'] if s['variant']==row['name'])
              for row in raw['implementations']}
    eligible = [row['name'] for row in raw['implementations']
                if row['kind']!='mech' and row.get('calculation_boundary')=='post_json_including_setup']
    serial = [name for name in eligible if 'fresh4' not in name]
    fastest = min(eligible,key=values.__getitem__)
    fastest_serial = min(serial,key=values.__getitem__)
    names = [name for name in values if name.startswith('mech-managed')]
    rows = [{'implementation':name, 'post_json_ms':values[name]*1000,
             'times_current_fastest_Rust':values[name]/values[fastest],
             'times_original_20_serial_Rust':values[name]/values[fastest_serial],
             'whole_process_ms':statistics.median(s['wall_seconds'] for s in raw['samples'] if s['variant']==name)*1000}
            for name in names]
    return {'record':label+'/latency.json','config':label+'/config.json',
            'rounds':31,'implementations':len(raw['implementations']),'exact_outputs':len(raw['samples']),
            'Rust_reference':fastest,'Rust_ms':values[fastest]*1000,'eligible_Rust_controls':eligible,
            'original_20_serial_Rust_reference':fastest_serial,'original_20_serial_Rust_ms':values[fastest_serial]*1000,
            'rows':rows}


primary = primary_summary('first-arena-retire')
confirm = read(M/CONFIRM/'report.json')
confirm_plan = read(M/CONFIRM/'plan.json')
resource = read(M/RESOURCE/'report.json')
metadata = read(M/'retire-overlap/build/metadata.json')
resource_rows = []
for name in ('mech-managed-host-v2-4-instrumented','mech-managed-short-overlap-instrumented',
             'mech-managed-arena-overlap-instrumented','mech-managed-retire-overlap-instrumented',
             *resource_plan['resource_references'].values()):
    if any(row['implementation']==name for row in resource_rows):
        continue
    row = resource['summary'][name]
    declared = next(value for value in resource_plan['implementations'] if value['name']==name)
    resource_rows.append({'implementation':name,'post_json':row['window_medians']['post_json'],
                          'separate_restore':row['window_medians'].get('restore'),
                          'separate_JSON':row['window_medians'].get('json'),
                          'whole_process':row['whole_process_medians'],
                          'executable_sizes':{key:value['bytes'] for key,value in declared['pins'].items()
                                              if key in ('executable','ordinary_latency_executable')},
                          'native_sizes':declared['native_sizes'], 'section_sizes':declared['sizes']})
focused = read(M/'retire-overlap-compiler/focused-contracts.json')
report = {'schema':'mech.related-post-gen.managed-native-storage.v1',
          'compiler_source_commit':COMMIT,'published_source_equivalent_commit':PUBLISHED_EQUIVALENT,
          'baseline_host_source_commit':BASELINE,
          'production_source_inventory':'retire-overlap-compiler/source-inventory.json',
          'compiler':{'typed_ir':metadata['typed_ir_version'],'work':metadata['work_policy_version'],
                      'abi':metadata['abi_version'],'image':metadata['native_descriptor_version'],
                      'backend':'Cranelift','mandatory_dependency_added':False},
          'benchmark_source':'benchmarks/related-post-gen/mech/related_optimized.mec',
          'benchmark_source_sha256':sha(git_bytes('benchmarks/related-post-gen/mech/related_optimized.mec')),
          'selected_adapter':'benchmarks/related-post-gen/hosts/managed-4.rs',
          'selected_adapter_sha256':sha(git_bytes('benchmarks/related-post-gen/hosts/managed-4.rs')),
          'input_sha256':sha(INPUT.read_bytes()),'gold_sha256':sha(GOLD.read_bytes()),
          'candidate':CANDIDATE,'control':CONTROL,'short_key_control':SHORT,
          'primary':primary,'earlier_primary_cohorts':[primary_summary('first-short-worker'),primary_summary('first-prefix-overlap')],
          'primary_paired_comparison':'first-arena-retire/paired-comparison.json',
          'confirmation':{'record':CONFIRM+'/report.json','plan':CONFIRM+'/plan.json',
                          'script':'independent-managed-confirm-v2.py','rounds':13,'implementations':27,
                          'exact_outputs':confirm['exact_outputs'],'references':confirm['references'],
                          'rows':{name:confirm['summary'][name] for name in (CONTROL,SHORT,CANDIDATE)},
                          'paired_comparisons':confirm['candidate_paired_comparisons'],
                          'same_machine_independent_orchestration':True},
          'resources':{'record':RESOURCE+'/report.json','plan':RESOURCE+'/plan.json','script':'new-managed-resources/run.py',
                       'rounds':7,'implementations':29,'exact_outputs':resource['exact_outputs'],
                       'references':resource_plan['resource_references'],'rows':resource_rows,
                       'scope':resource_plan['scope'],'free_call_count_available':False},
          'contracts':{'focused_record':'retire-overlap-compiler/focused-contracts.json','focused_GPU_passed':129,
                       'focused_GPU_ignored':1,'core_record':'prefix-compiler-v1/full-core-contracts.json','core_passed':399,
                       'broad_record':'short-worker-compiler/broad-native-contracts.json','broad_typed_ir':326,
                       'broad_GPU_passed':1051,'broad_GPU_failed':16,'broad_GPU_ignored':11,
                       'broad_failure_attribution':'short-worker-compiler/broad-native-failure-attribution.json',
                       'full_broad_green':False,'final_full5000':'retire-overlap/qualify-5000.json',
                       'final_edge_cases':'retire-overlap/qualify-edges.json','edge_cases':31},
          'native_Object_sha256':sha((M/'retire-overlap/build/object.o').read_bytes()),
          'native_link_audit':'retire-overlap/same-object-audit.json',
          'target':'managed complete postJSON <= fastest eligible measured Rust, including fresh four-worker Rust controls',
          'target_met':False,'beats_original_20_serial_Rust':True,'whole_process_is_acceptance_gate':False,
          'storage_incremental_latency_gain_established':False,
          'broad_host_to_selected_latency_gain_established':True,
          'logical_resource_contract':'Exact admission, work/comparison charging, arithmetic/index checks, current handles, failure ordering and retry remain enabled; physical storage shrinks only under checked inventory proofs.',
          'profile':{'record':'short-overlap/profile/receipt.json','typed_ir':328,'diagnostic_only':True,
                     'scope':'Retained repeated full-path IR328 profile includes JSON IO and final destruction as labeled; warm diagnostic timings and leaf samples are not wall-time shares. It does not profile final IR331 storage changes.'},
          'limitations':['Current fastest measured Rust uses four fresh scoped workers. Twenty eligible serial controls and two preinitialized/ineligible controls remain; no global-optimum claim.',
                         'Both confirmation and primary run on the same Apple M3 machine. Independent orchestration is not an independent-machine replication.',
                         'Incremental storage confirmation vs short-overlap: paired median -0.972%, bootstrap95%[-2.075%,+1.445%]. This does not establish a latency improvement.',
                         'Managed postJSON retains historical final input/output destruction boundary. Whole process includes restore, JSON IO, all cleanup and process exit; it is secondary and non-gating.',
                         'Requested allocator heap excludes mappings, OS stacks and allocator internals. Freed bytes are measured; number of free calls is unavailable.',
                         'Broad native qualification is IR326, not IR331. All16 named inherited failures reproduce in the unchanged baseline; no broad-green or hosted-CI-green claim.',
                         'Five historical instrumented Rust source/receipt pairs are unavailable locally; their prior published provenance hashes and unchanged executable identities remain explicitly labeled.',
                         'No IR332 experimental compiler source, metadata or result is included in this IR331 packet.'],
          'timed_exact_outputs':sum(x['exact_outputs'] for x in [primary,*[primary_summary(n)for n in ('first-short-worker','first-prefix-overlap')]])+confirm['exact_outputs']+resource['exact_outputs']}
(D/'report.json').write_text(json.dumps(report,indent=2)+'\n')
manifest = {'schema':'mech.related-post-gen.content-records.v1','logical_records':len(records),'unique_blobs':len(blobs),
            'compressed_bytes':sum(value['compressed_bytes']for value in blobs.values()),'records':records}
(D/'records.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'logical_records':len(records),'unique_blobs':len(blobs),'compressed_bytes':manifest['compressed_bytes'],
                  'compiler_source_commit':COMMIT,'timed_exact_outputs':report['timed_exact_outputs']},indent=2))
