#!/usr/bin/env python3
"""Freeze IR334 evidence only. No compiler, benchmark or live compiler source input."""
from pathlib import Path
import gzip
import hashlib
import json
import statistics
import subprocess

D=Path(__file__).resolve().parent
S=D.parents[3]
M=Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
BASE='f361f91f298b87ebb9bee880930951fdf79b0d5f'
SOURCE='55a47b080eba45e46596976279d196b9c97eba2e'
PRIMARY='first-scoped-frame-v2'
CONFIRM='independent-managed-scoped-frame-prepared'
RESOURCES='new-managed-resources/resources-scoped-first'
COMPILER='scoped-frame-compiler-v1'
CANDIDATE='mech-managed-scoped-frame'
CONTROL='mech-managed-retire-overlap'
if (D/'records.json').exists():raise SystemExit('Historical packet already exists')
(D/'records').mkdir(exist_ok=True)
records=[];blobs={};inodes={};names=set()
def sha(data):return hashlib.sha256(data).hexdigest()
def read(path):return json.loads(Path(path).read_bytes())
def add_bytes(data,name):
    if name in names:return
    digest=sha(data)
    if digest not in blobs:
        packed=gzip.compress(data,compresslevel=9,mtime=0);dest='records/'+digest+'.gz';(D/dest).write_bytes(packed)
        blobs[digest]={'path':dest,'compressed_sha256':sha(packed),'compressed_bytes':len(packed),'uncompressed_sha256':digest,'uncompressed_bytes':len(data)}
    records.append({'source':name,**blobs[digest]});names.add(name)
def add(path,name=None):
    path=Path(path)
    if not path.is_file():return
    name=name or str(path.relative_to(M))
    if name in names:return
    stat=path.stat();identity=(stat.st_dev,stat.st_ino,stat.st_size,stat.st_mtime_ns)
    if identity in inodes:records.append({'source':name,**blobs[inodes[identity]]});names.add(name)
    else:
        data=path.read_bytes();add_bytes(data,name);inodes[identity]=sha(data)
def tree(root,suffixes=None):
    for path in sorted(Path(root).rglob('*')):
        if path.is_file()and 'target'not in path.parts and '__pycache__'not in path.parts and(suffixes is None or path.suffix in suffixes):add(path)
def git_bytes(commit,path):return subprocess.run(['git','show',commit+':'+path],cwd=S,capture_output=True,check=True).stdout
for label in ('first-scoped-frame',PRIMARY,CONFIRM,RESOURCES):tree(M/label)
for name in ('finalize-scoped-frame-offline.py','independent-managed-confirm-v2.py','new-managed-resources/run.py','IR334-published-source-commit.json','native-failure-attribution.json','unchanged-native-failure-comparison.json','host-native-contracts.json','build-engine-round.py','build-engine-tests-round.py','build-gpu-round-tests.py','IR334-packet-audit/latency-provenance-audit.json'):
    add(M/name)
for label in ('scoped-frame','retire-overlap','host-v2-4'):
    for path in sorted((M/label).glob('*')):
        if path.suffix in {'.json','.rs','.txt','.mec'}:add(path)
    tree(M/label/'build',{'.json','.rs','.mec','.mecb','.bin','.o','.txt'})
    if label=='scoped-frame':tree(M/label/'edges',{'.json','.txt'})
for label in (COMPILER,'prefix-compiler-v1','retire-overlap-compiler'):
    for path in sorted((M/label).glob('*.json')):add(path)
for label in ('scoped-frame-prefix-candidate','scoped-frame-prefix-design'):
    tree(M/label,{'.json','.patch','.rs','.md'})
tree(M/'rust-fresh4-control',{'.json','.rs','.toml','.lock','.patch','.py','.txt'})
config=read(M/PRIMARY/'config.json');plan=read(M/RESOURCES/'plan.json');confirm_plan=read(M/CONFIRM/'plan.json')
for row in config['implementations']:
    for key in ('source','receipt'):
        if row.get(key):add(row[key],'references/'+row['name']+'/'+('source.mec'if key=='source'and row['kind']=='mech'else 'source.rs'if key=='source'else 'receipt.json'))
    if row.get('receipt')and Path(row['receipt']).is_file()and row['kind']=='mech':
        receipt=read(row['receipt'])
        if receipt.get('source_path'):add(receipt['source_path'],'references/'+row['name']+'/adapter.rs')
        for name,pin in receipt.get('bundle',{}).items():add(pin['path'],'references/'+row['name']+'/bundle/'+name)
for prefix,rows in (('resource-references',plan['implementations']),('confirmation-references',confirm_plan['implementations'])):
    for row in rows:
        for key in ('source','receipt','adapter_source','qualification'):
            pin=row['pins'].get(key)
            if pin:add(pin['path'],prefix+'/'+row['name']+'/'+key)
        for key,pin in row['pins'].get('bundle',{}).items():add(pin['path'],prefix+'/'+row['name']+'/bundle/'+key)
for pin in plan['source_configs']+plan['measurement_helpers']:
    add(pin['path'],'resource-inputs/'+Path(pin['path']).name+'-'+pin['sha256'])
add(plan['input']['path'],'publication-inputs/posts.json');add(plan['gold']['path'],'publication-inputs/gold.json')
for path in ('benchmarks/related-post-gen/compare-implementations.py','benchmarks/related-post-gen/measurement.rs','benchmarks/related-post-gen/audit-native-link.py','benchmarks/related-post-gen/mech/related_optimized.mec','benchmarks/related-post-gen/hosts/managed-4.rs'):
    add_bytes(git_bytes(SOURCE,path),'published-commit/'+path)
for name in ('first-10','first-100','first-500','unicode-ties','empty-tags'):
    for prefix in ('posts-','expected-'):
        path='benchmarks/related-post-gen/fixtures/'+prefix+name+'.json';add_bytes(git_bytes(SOURCE,path),'published-commit/'+path)
# Only three delta files; all remaining measured source files are authenticated by Git.
receipt=read(M/'IR334-published-source-commit.json');inventory=read(M/COMPILER/'source-inventory.json')
overlays={}
for path,pin in receipt['source_files'].items():
    data=Path(pin['snapshot']).read_bytes();assert sha(data)==pin['sha256']==inventory['files'][path]
    add_bytes(data,'measured-source/'+path);overlays[path]=pin['sha256']
patch=subprocess.run(['git','diff','--binary',BASE,SOURCE,'--',*sorted(overlays)],cwd=S,capture_output=True,check=True).stdout
add_bytes(patch,'publication-source.patch')
closure={'measured_source_commit':SOURCE,'published_source_equivalent_commit':SOURCE,'previous_selected_source_commit':BASE,
         'inventory':COMPILER+'/source-inventory.json','source_files':len(inventory['files']),'literal_includes':len(inventory['literal_includes']),
         'measured_source_sha256':inventory['sha256'],'three_file_overlay':overlays,'patch':'publication-source.patch',
         'scope':'Complete measured compiler inventory verified against the published commit. Three IR334 source deltas retained; no executable or library binaries retained.',
         'reused_non_GPU_inventory':'prefix-compiler-v1/source-inventory.json','reused_non_GPU_scope':'Non-GPU files only; the historical GPU in this reused-closure inventory was not compiled for IR334.'}
add_bytes(json.dumps(closure,indent=2).encode()+b'\n','publication-source-closure.json')
raw=read(M/PRIMARY/'derived-latency.json');paired=read(M/PRIMARY/'paired-comparison.json');confirm=read(M/CONFIRM/'report.json');resource=read(M/RESOURCES/'report.json')
variants={row['name']:row for row in raw['implementations']}
eligible=[name for name,row in variants.items()if row['kind']!='mech'and row.get('calculation_boundary')=='post_json_including_setup']
serial=[name for name in eligible if 'fresh4'not in name]
serial_name=min(serial,key=lambda n:raw['summary'][n]['median_post_json_seconds'])
def latency_rows(summary,independent=False):
    rows=[]
    for name,row in variants.items():
        if row['kind']!='mech'or name not in summary:continue
        value=summary[name];sec=value['median_post_json_seconds']
        current=confirm['references']['current_22_eligible_Rust']['median_seconds']if independent else raw['primary_reference']['median_seconds']
        serial_sec=confirm['references']['original_20_serial_Rust']['median_seconds']if independent else raw['summary'][serial_name]['median_post_json_seconds']
        rows.append({'implementation':name,'post_json_ms':sec*1000,'times_current_fastest_Rust':sec/current,'times_original_20_serial_Rust':sec/serial_sec,'whole_process_ms':value['median_whole_process_seconds']*1000})
    return rows
resource_rows=[]
for row in plan['implementations']:
    if row['kind']=='mech'or row['name']in plan['resource_references'].values():
        value=resource['summary'][row['name']]
        resource_rows.append({'implementation':row['name'],'post_json':value['window_medians']['post_json'],'separate_windows':{k:v for k,v in value['window_medians'].items()if k!='post_json'},'whole_process':value['whole_process_medians'],'executable_bytes':row['pins']['executable']['bytes'],'ordinary_executable_bytes':row['pins'].get('ordinary_latency_executable',{}).get('bytes'),'native_sizes':row['native_sizes']})
report={'schema':'mech.related-post-gen.managed-worker-footprint.v1','measured_source_commit':SOURCE,'published_source_equivalent_commit':SOURCE,'previous_selected_source_commit':BASE,'selected':'IR334','candidate':CANDIDATE,'control':CONTROL,'compiler_backend':'Cranelift','mandatory_dependency_added':False,'source_closure':'publication-source-closure.json','independent_offline_audit':'IR334-packet-audit/latency-provenance-audit.json',
 'primary':{'raw_checkpoint':PRIMARY+'/latency.json','record':PRIMARY+'/derived-latency.json','config':PRIMARY+'/config.json','protocol':PRIMARY+'/protocol.json','paired_comparison':PRIMARY+'/paired-comparison.json','offline_recovery_script':'finalize-scoped-frame-offline.py','rounds':31,'implementations':28,'exact_outputs':868,'seed':202611120519,'bootstrap_seed':202611120520,'current_fastest_Rust':raw['primary_reference'],'original_20_serial_Rust':{'variant':serial_name,'median_seconds':raw['summary'][serial_name]['median_post_json_seconds']},'rows':latency_rows(raw['summary']),'paired_incremental':paired,'observations_changed_in_recovery':False,'executables_rerun_in_recovery':False},
 'partial_initial_attempt':{'record':'first-scoped-frame/latency.json','protocol':'first-scoped-frame/protocol.json','observations':1,'candidate_observations':0,'included_in_primary':False,'failure':'macOS time wrapper sysctl kern.clockrate access denied; Rust stdout retained; no candidate observation'},
 'independent_confirmation':{'record':CONFIRM+'/report.json','plan':CONFIRM+'/plan.json','script':'independent-managed-confirm-v2.py','rounds':13,'implementations':27,'exact_outputs':351,'seed':202611120521,'references':confirm['references'],'paired_comparisons':confirm['candidate_paired_comparisons'],'rows':latency_rows(confirm['summary'],True),'same_machine':True},
 'resources':{'record':RESOURCES+'/report.json','plan':RESOURCES+'/plan.json','script':'new-managed-resources/run.py','rounds':7,'implementations':28,'exact_outputs':196,'seed':202611120522,'scope':plan['scope'],'references':plan['resource_references'],'rows':resource_rows},
 'timed_exact_outputs':1415,'incremental_latency_gain_confirmed':True,'fastest_Rust_target_met':False,'logical_work_and_checks_enabled':True,'allocation_bytes_and_requests_unchanged':True,
 'qualification':{'compiler_build':COMPILER+'/build.json','engine':COMPILER+'/focused-engine-contracts.json','focused_engine_passed':68,'GPU':COMPILER+'/focused-gpu-contracts.json','focused_GPU_passed':97,'broad':COMPILER+'/broad-native-tests.json','broad_passed':1056,'broad_inherited_failures':16,'broad_ignored':11,'broad_filtered':35,'attribution':COMPILER+'/broad-native-failure-attribution.json','full_broad_green':False,'full5000':'scoped-frame/qualify-5000.json','edges':'scoped-frame/qualify-edges.json','edge_count':31,'object_link_audit':'scoped-frame/same-object-audit.json'},
 'limitations':['One Apple M3 machine; independent orchestration is same-machine confirmation, not separate-machine replication or a globally fastest implementation claim.','Complete continuous post-JSON is authoritative. Restoring the artifact and JSON IO remain in whole-process diagnostics; whole process is not an acceptance gate.','Two preinitialized Rayon controls are retained but ineligible for complete post-JSON comparison. Current reference is the fastest of22eligible Rust variants, with20originalserial variants reported separately.','All868 primary observations were retained before a final stale resource-reference summary failure. Offline recovery changed no samples or orders and reran no executable; the prior infrastructure-failed partial attempt remains separate.','All-thread counters and requested-heap accounting cover the complete post-JSON resource window. Freed bytes are recorded; free-call count is unavailable. Whole-process peak RSS and window endpoint RSS are distinct from requested heap.','Broad tests retain16 failures reproduced on the unchanged baseline; this packet makes no full-broad-green claim.','Only measured IR334 source and results are included. Later IR335/336 compiler experiments are excluded.']}
manifest={'schema':'mech.related-post-gen.content-records.v1','logical_records':len(records),'unique_blobs':len(blobs),'compressed_bytes':sum(v['compressed_bytes']for v in blobs.values()),'records':records}
(D/'records.json').write_text(json.dumps(manifest,indent=2)+'\n');(D/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({key:manifest[key]for key in ('logical_records','unique_blobs','compressed_bytes')},indent=2))
