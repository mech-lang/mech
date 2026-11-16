#!/usr/bin/env python3
"""Archive the unselected IR332/333 checkpoint. No compiler/benchmark launch.

Only frozen evidence and the published IR331 Git base are read. Live IR334
sources are deliberately excluded. Historical absolute paths are provenance.
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
BASE = 'f361f91f298b87ebb9bee880930951fdf79b0d5f'
PRIMARY = 'first-paid-workers'
RESOURCES = 'new-managed-resources/resources-paid-first'
if (D/'records.json').exists():
    raise SystemExit('Refusing to replace an existing historical packet')
(D/'records').mkdir(exist_ok=True)
records = []
blobs = {}
inodes = {}
names = set()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(Path(path).read_bytes())


def add_bytes(data, name):
    if name in names:
        return
    digest = sha(data)
    if digest not in blobs:
        packed = gzip.compress(data, compresslevel=9, mtime=0)
        dest = 'records/'+digest+'.gz'
        (D/dest).write_bytes(packed)
        blobs[digest] = {'path':dest,'compressed_sha256':sha(packed),'compressed_bytes':len(packed),
                         'uncompressed_sha256':digest,'uncompressed_bytes':len(data)}
    records.append({'source':name,**blobs[digest]})
    names.add(name)


def add(path, name=None):
    path = Path(path)
    if not path.is_file():
        return
    name = name or str(path.relative_to(M))
    if name in names:
        return
    stat = path.stat()
    identity = (stat.st_dev,stat.st_ino,stat.st_size,stat.st_mtime_ns)
    if identity in inodes:
        records.append({'source':name,**blobs[inodes[identity]]})
        names.add(name)
    else:
        data = path.read_bytes()
        add_bytes(data,name)
        inodes[identity] = sha(data)


def tree(root, suffixes=None):
    root = Path(root)
    if root.exists():
        for path in sorted(root.rglob('*')):
            if path.is_file() and (suffixes is None or path.suffix in suffixes):
                add(path)


def git_bytes(path):
    return subprocess.run(['git','show',BASE+':'+path],cwd=S,capture_output=True,check=True).stdout


tree(M/PRIMARY)
tree(M/RESOURCES)
add(M/'new-managed-resources/run.py')
for label in ('paid-cleanup','paid-alias','paid-alias-v2','retire-overlap','host-v2-4'):
    folder = M/label
    for path in sorted(folder.glob('*')):
        if path.is_file() and path.suffix in {'.json','.rs','.txt','.mec'}:
            add(path)
    tree(folder/'build',{'.json','.rs','.mec','.mecb','.bin','.o','.txt'})
    if label.startswith('paid'):
        tree(folder/'edges',{'.json','.txt'})
for label in ('paid-cleanup-compiler','paid-alias-compiler','paid-alias-compiler-v2',
              'paid-alias-fixture-tests-v3','prefix-compiler-v1','retire-overlap-compiler'):
    for path in sorted((M/label).glob('*.json')):
        add(path)
for label in ('unselected-paid-workers-IR333','worker-budget-test-owner-fix',
              'cold-alias-positive-fixture-fix','cold-alias-prefix-candidate'):
    tree(M/label,{'.json','.patch','.rs','.md'})
for name in ('native-failure-attribution.json','unchanged-native-failure-comparison.json',
             'short-worker-compiler/broad-native-failure-attribution.json',
             'measurement-source-pin.json','negative-worker-payments-source-audit.json'):
    add(M/name)
source_audit=read(M/'negative-worker-payments-source-audit.json')
for retained in source_audit['retention_files']:
    path=Path(retained['path'])
    assert sha(path.read_bytes())==retained['sha256']
    add(path,str(path.relative_to(M))if path.is_relative_to(M)else 'audit-retention/'+retained['sha256']+'-'+path.name)
config = read(M/PRIMARY/'config.json')
resource_plan = read(M/RESOURCES/'plan.json')
for row in config['implementations']:
    for key in ('source','receipt'):
        if row.get(key):
            add(row[key],'references/'+row['name']+'/'+('source.mec'if key=='source'and row['kind']=='mech'else 'source.rs'if key=='source'else 'receipt.json'))
    if row.get('receipt') and Path(row['receipt']).is_file() and row['kind']=='mech':
        receipt = read(row['receipt'])
        if receipt.get('source_path'):
            add(receipt['source_path'],'references/'+row['name']+'/adapter.rs')
        for name,pin in receipt.get('bundle',{}).items():
            add(pin['path'],'references/'+row['name']+'/bundle/'+name)
for row in resource_plan['implementations']:
    for key in ('source','receipt','adapter_source','qualification'):
        pin = row['pins'].get(key)
        if pin:
            add(pin['path'],'resource-references/'+row['name']+'/'+key)
    for key,pin in row['pins'].get('bundle',{}).items():
        add(pin['path'],'resource-references/'+row['name']+'/bundle/'+key)
for pin in resource_plan['source_configs']+resource_plan['measurement_helpers']:
    add(pin['path'],'resource-inputs/'+Path(pin['path']).name+'-'+pin['sha256'])
add(resource_plan['input']['path'],'publication-inputs/posts.json')
add(resource_plan['gold']['path'],'publication-inputs/gold.json')
for path in ('benchmarks/related-post-gen/compare-implementations.py',
             'benchmarks/related-post-gen/measurement.rs',
             'benchmarks/related-post-gen/audit-native-link.py',
             'benchmarks/related-post-gen/mech/related_optimized.mec',
             'benchmarks/related-post-gen/hosts/managed-4.rs'):
    add_bytes(git_bytes(path),'base-commit/'+path)
for name in ('first-10','first-100','first-500','unicode-ties','empty-tags'):
    for prefix in ('posts-','expected-'):
        path='benchmarks/related-post-gen/fixtures/'+prefix+name+'.json'
        add_bytes(git_bytes(path),'base-commit/'+path)

# Retain only source deltas, not several thousand duplicate GPU source files.
inventory_labels = ('paid-cleanup-compiler','paid-alias-compiler','paid-alias-compiler-v2',
                    'prefix-compiler-v1','retire-overlap-compiler')
inventories = {label:read(M/label/'source-inventory.json')['files']for label in inventory_labels}
# This closure supplies Core/engine/stdlib, not its historical uncompiled GPU.
inventories['prefix-compiler-v1'] = {path:digest for path,digest in inventories['prefix-compiler-v1'].items()
                                    if not path.startswith('hosts/gpu/')}
all_paths = sorted(set().union(*(files.keys()for files in inventories.values())))
process = subprocess.run(['git','cat-file','--batch'],cwd=S,capture_output=True,check=True,
    input=''.join(BASE+':'+name+'\n'for name in all_paths).encode()).stdout
offset = 0
base_hashes = {}
for name in all_paths:
    end=process.index(b'\n',offset);header=process[offset:end].split();offset=end+1
    if len(header)==2 and header[1]==b'missing':
        base_hashes[name]=None
        continue
    assert len(header)==3 and header[1]==b'blob',(name,header)
    length=int(header[2]);data=process[offset:offset+length];offset+=length+1
    base_hashes[name]=sha(data)
assert offset==len(process)
overlays = {}
for label,files in inventories.items():
    changed = {path:digest for path,digest in files.items()if base_hashes[path]!=digest}
    for path,digest in changed.items():
        frozen=M/label/'source'/path
        assert frozen.is_file(),(label,path)
        assert sha(frozen.read_bytes())==digest
        add(frozen)
    overlays[label]={'inventory':label+'/source-inventory.json','source_files':len(files),
                     'base_identical_files':len(files)-len(changed),'overrides':changed,
                     'scope':'non-GPU Core/engine/stdlib closure'if label=='prefix-compiler-v1'else 'complete captured inventory'}
# v3 is a test-only replacement over the compiled v2 closure.
override=read(M/'paid-alias-fixture-tests-v3/override.json')
add(M/'paid-alias-fixture-tests-v3/source'/override['path'])
add_bytes(json.dumps({'base':BASE,'inventories':overlays,'v3_test_override':override},indent=2).encode()+b'\n',
          'publication-source-closure.json')

latency=read(M/PRIMARY/'latency.json')
paired=read(M/PRIMARY/'paired-comparison.json')
resource=read(M/RESOURCES/'report.json')
rows=[]
for row in config['implementations']:
    if row['kind']=='mech':
        summary=paired['rows'][row['name']]
        rows.append({'implementation':row['name'],'post_json_ms':summary['seconds']*1000,
                     'times_current_fastest_Rust':summary['times_fastest_Rust'],
                     'times_original_20_serial_Rust':summary['times_retained20serial'],
                     'whole_process_ms':summary['whole_process_seconds']*1000,
                     'paired_median_percent_vs_IR331':summary['paired_median_pct_vs_IR331'],
                     'paired_95_interval_percent':summary['paired95bootstrap']})
resource_rows=[]
for row in resource_plan['implementations']:
    if row['kind']=='mech'or row['name']in resource_plan['resource_references'].values():
        summary=resource['summary'][row['name']]
        resource_rows.append({'implementation':row['name'],'post_json':summary['window_medians']['post_json'],
                              'separate_windows':{k:v for k,v in summary['window_medians'].items()if k!='post_json'},
                              'whole_process':summary['whole_process_medians'],
                              'executable_bytes':row['pins']['executable']['bytes'],
                              'ordinary_executable_bytes':row['pins'].get('ordinary_latency_executable',{}).get('bytes'),
                              'native_sizes':row['native_sizes']})
report={'schema':'mech.related-post-gen.managed-worker-payments-negative.v1','base_source_commit':BASE,
        'selected':'mech-managed-retire-overlap','unselected':['mech-managed-paid-cleanup','mech-managed-paid-alias-v2'],
        'compiler_backend':'Cranelift','mandatory_dependency_added':False,'source_closure':'publication-source-closure.json',
        'source_audit':'negative-worker-payments-source-audit.json',
        'overlay':'unselected-paid-workers-IR333/source.patch','overlay_receipt':'unselected-paid-workers-IR333/receipt.json',
        'primary':{'record':PRIMARY+'/latency.json','config':PRIMARY+'/config.json','paired_comparison':PRIMARY+'/paired-comparison.json',
                   'paired_protocol':PRIMARY+'/paired-bootstrap-protocol.json',
                   'rounds':31,'implementations':30,'exact_outputs':930,'seed':202611114600,
                   'current_fastest_Rust':paired['current_fastest'],
                   'original_20_serial_Rust':paired['retained20serial'],'rows':rows},
        'resources':{'record':RESOURCES+'/report.json','plan':RESOURCES+'/plan.json','script':'new-managed-resources/run.py',
                     'rounds':7,'implementations':29,'exact_outputs':203,'seed':202611115400,
                     'references':resource_plan['resource_references'],'scope':resource_plan['scope'],'rows':resource_rows},
        'timed_exact_outputs':1133,'independent_confirmation':False,'adverse_result_rerun':False,
        'established_new_latency_gain':False,'allocations_unchanged':True,'logical_work_and_checks_enabled':True,
        'qualification':{'R333_broad':'paid-alias-compiler-v2/broad-native-contracts.json','passed':1059,'inherited_failures':16,'ignored':11,
                         'new_positive_fixture_skipped_in_broad':True,'attribution':'paid-alias-compiler-v2/broad-failure-attribution.json',
                         'corrected_test_only_override':'paid-alias-fixture-tests-v3/override.json',
                         'corrected_positive_fixture':'paid-alias-fixture-tests-v3/positive-fixture.json',
                         'full_broad_green':False,'full5000_and31edges':['paid-cleanup','paid-alias-v2']},
        'worker_scope_audit':{'record':'cold-alias-prefix-candidate/real-worker-scope-audit.json',
                              'kind':'read-only static source eligibility','estimated_shared_locks_saved':180,
                              'dynamic_success_count_measured':False,'lock_savings_measured':False},
        'limitations':['Single Apple M3 machine, exploratory same-machine paired cohort; no global-fastest implementation claim.',
                       'IR332 is slower in paired primary result; IR333 paired interval crosses zero. Neither is selected and no independent confirmation or adverse-result rerun was launched.',
                       'Instruction reductions are measured complete post-JSON all-thread totals, not proof of latency improvement; cycles vary with scheduling and instrumentation.',
                       'Allocator bytes/requests/peak remain unchanged across IR331/332/333; free-call count unavailable. RSS is whole-process peak, separate from requested heap and endpoint residency.',
                       'Broad1059/16/11 is the v2 test closure with the broken new positive fixture explicitly skipped. v3 corrects only its test source and passes that one test; no full broad v3 green claim.',
                       'There is one observed200-byte cleanup test-owner failure. The equivalent alias helper pattern was corrected statically before its first test run; no second owner-leak failed run exists. A later new positive cold-alias fixture failed, then its test-only v3 correction passed. These are test corrections, not production ownership fixes.',
                       'The 180 shared-lock estimate is static for one successful cold-alias suffix across four workers. Dynamic batching counts and actual lock savings are not measured.',
                       'No live IR334 source, engine candidate, metadata or timing is an input to this historical packet.']}
manifest={'schema':'mech.related-post-gen.content-records.v1','logical_records':len(records),'unique_blobs':len(blobs),
          'compressed_bytes':sum(row['compressed_bytes']for row in blobs.values()),'records':records}
(D/'records.json').write_text(json.dumps(manifest,indent=2)+'\n')
(D/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({key:manifest[key]for key in ('logical_records','unique_blobs','compressed_bytes')},indent=2))
