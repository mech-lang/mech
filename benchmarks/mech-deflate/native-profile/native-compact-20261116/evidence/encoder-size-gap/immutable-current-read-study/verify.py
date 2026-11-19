"""Check archived variants and measurements; no new native qualification is inferred."""
from pathlib import Path
from hashlib import sha256
import gzip,json,math,tarfile
H=Path(__file__).resolve().parent
sha=lambda b:sha256(b).hexdigest()
def blob(name):
 p=H/name
 return p.read_bytes() if p.is_file() else gzip.decompress(Path(str(p)+'.gz').read_bytes())
def read(name):return json.loads(blob(name))
manifest=read('files.json')
for name,pin in manifest['files'].items():assert sha((H/name).read_bytes())==pin,name
assert read('gpu-tests/build.json')['exit_code']!=0
assert read('pinned-gpu-tests/build.json')['exit_code']==0
for name in ['defaulted-loads','private-loan','private-contracts']:assert read('pinned-gpu-tests/'+name+'.json')['exit_code']==0
for name,pin in read('pinned/compiler-build.json')['source_inputs'].items():assert sha(blob('pinned/source-inputs/'+name))==pin,name
for stage,expected,byte_loads in [('repair',63484,4002154),('pinned',63692,2957769)]:
 prefix=stage+'/immutable-profiled/';r=read(prefix+'receipt.json');assert r['text_bytes']==expected
 for name,key in [('program.o','object_sha256'),('metadata.json','metadata_sha256')]:assert sha(blob(prefix+'frontend/'+name))==r[key]
 runtime=read(prefix+'bound/runtime.json');assert sha(blob(prefix+'bound/mech-worker'))==runtime['worker_sha256']
 linked=read(prefix+'linked-checks.json');assert linked['matches_success_observations']==162 and len(linked['checked_failure_cases'])==6
 proof=read(prefix+'linked-proof.json');assert proof['peak_stack_bytes']==4848 and sha(blob(prefix+'linked-checks.json'))==proof['sha256']
 recovery=read(prefix+'recoveries/result.json');assert recovery['cases']==6 and recovery['exact_recoveries']==12 and recovery['benchmark_worker_sha256']==runtime['worker_sha256']
 counts=read(prefix+'matcher-counts/counts.json');normalization=read(prefix+'canonical-counter-input/canonicalization.json');assert counts['object_sha256']==normalization['diagnostic_sha256'] and normalization['source_sha256']==r['object_sha256'];assert counts['cases']['prefix-random-1048576']['counts']['ldrb']==byte_loads
 layout=read(stage+'/layout-proof.json');assert all(p['same_ordered_graph'] for p in layout['unchanged_other_profiles']);assert sha(blob(stage+'/matcher-layout.json'))==layout['profile_sha256']
 focus=prefix+'focus/';screen=read(focus+'screen.json');assert len(screen['runs'])==2
 for order in ['forward','reverse']:
  pins=read(focus+order+'/pins.json');assert pins['actors']['mech-experiment']['binary_sha256']==runtime['worker_sha256'];assert pins['actors']['mech-control']['binary_sha256']=='88321c097d29b7219f639abbc5fcce727f0602de2d05ab24e5e2b47f2996c3b4'
  summary={r['case']:r for r in read(focus+order+'/summary.json')}
  with tarfile.open(H/focus/(order+'-observations.tar.gz')) as archive:
   files=[m for m in archive.getmembers() if m.name.endswith('/observations.json.gz')];assert len(files)==len(summary)==(17 if stage=='repair' else 15)
   for member in files:
    case=Path(member.name).parent.name;data=json.loads(gzip.decompress(archive.extractfile(member).read()));blocks=data['blocks'] if isinstance(data,dict) else data;assert len(blocks)==96
    for actor,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
     ratio=math.exp(sum(math.log(b['means']['mech-experiment']/b['means'][actor]) for b in blocks)/len(blocks));assert abs(ratio-summary[case][key]['mech-experiment'])<1e-10
 if stage=='pinned':assert screen['preserves_peer_wins']
 else:
  assert not screen['preserves_peer_wins'];assert screen['runs'][0]['losses']['libdeflate']==['320-overlap'];assert screen['runs'][1]['losses']['libdeflate']==[]
print('Verified source snapshots, actual worker checks/recoveries/stack, fresh graph/counter bindings and all paired focused observations. Both variants remain unselected; full runtime/build qualification is not claimed.')
