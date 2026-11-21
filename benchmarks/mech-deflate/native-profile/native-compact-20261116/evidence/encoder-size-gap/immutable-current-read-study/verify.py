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

selected=read('candidate.json');result=read('placement/result.json');assert selected['encoder_text_bytes']==result['encoder_bytes']==63532 and result['bytes_saved_vs_selected']==160
assert selected['artifact_sha256']==result['artifacts'];assert selected['worker_sha256']==result['worker_sha256']=='834c7a7e423a69c46483a3dae74d9450949fdf142b5ad2dede894e0da3ca8693'
for name,pin in selected['artifact_sha256'].items():
 source='inputs/program.mecb' if name=='program.mecb' else 'placement/immutable-profiled/frontend/'+name
 assert sha(blob(source))==pin
for name,pin in selected['source_inputs'].items():assert sha(blob('final-integration/source-inputs/'+name))==pin
assert read('placement/immutable-profiled-checked/checker-match.json')['artifacts']==selected['artifact_sha256']
assert all(r['artifacts']==selected['artifact_sha256'] for r in read('source-reproduction.json')['variants'])
proof=read('placement/immutable-profiled/linked-proof.json');linked=read('placement/immutable-profiled/linked-checks.json');assert proof['peak_stack_bytes']==4848 and sha(blob('placement/immutable-profiled/linked-checks.json'))==proof['sha256'];assert linked['matches_success_observations']==162 and len(linked['checked_failure_cases'])==6
recovery=read('placement/immutable-profiled/recoveries/result.json');assert recovery['cases']==6 and recovery['exact_recoveries']==12 and recovery['benchmark_worker_sha256']==selected['worker_sha256']
full=read('placement/immutable-profiled/full/full-replay.json');screen=read('placement/immutable-profiled/full/screen.json');assert full['decision']==screen['decision']=='Retain';assert full['rejected']==full['unresolved']==[];assert len(full['runs'])==2
for run in full['runs']:
 assert run['case_count']==81
 for peer,floor in [('roc',81),('libdeflate',62)]:
  assert all(run['peers'][peer][field]['mech-experiment']>=floor for field in ['point_estimate_wins','precision_supported_wins']);assert run['peers'][peer]['lost_cases']==[]
ratios={(r['order'],r['case']):r for r in read('placement/runtime-case-ratios.json')}
for order in ['forward','reverse']:
 prefix='placement/immutable-profiled/full/';pins=read(prefix+order+'/pins.json');assert pins['actors']['mech-experiment']['binary_sha256']==selected['worker_sha256']
 with tarfile.open(H/prefix/(order+'-observations.tar.gz')) as archive:
  files=[m for m in archive.getmembers() if m.name.endswith('/observations.json.gz')];assert len(files)==81
  for member in files:
   case=Path(member.name).parent.name;data=json.loads(gzip.decompress(archive.extractfile(member).read()));blocks=data['blocks'] if isinstance(data,dict) else data;assert len(blocks)==96
   for actor,key in [('mech-control','candidate_control'),('roc','candidate_roc'),('libdeflate','candidate_c')]:
    ratio=math.exp(sum(math.log(b['means']['mech-experiment']/b['means'][actor]) for b in blocks)/len(blocks));assert abs(ratio-ratios[order,case][key])<1e-10
for row in read('placement/required-wins.json'):assert all(not p['lost'] for p in row['peers'].values())
build=read('placement/matched-builds/comparison.json');plan=read('placement/matched-builds/plan.json');assert len(build['observations'])==len(plan['orders'])==16 and len(build['medians'])==8
assert sha(blob('placement/matched-builds/plan.json'))==build['plan_sha256']
for row in build['observations']:
 assert len(row)==8
 for actor,run in row.items():
  assert run['seconds']['source_through_link']>=run['seconds']['source'];assert run['seconds']['source_through_strip']>=run['seconds']['source_through_link'];assert run['stripped_executable']==build['warmup'][actor]['stripped_executable']
summary=read('placement/current-runtime-profiles/summary.json');assert summary['plan']['actors']['mech']['sha256']==selected['worker_sha256']
for case,rows in summary['profiles'].items():
 for actor in ['mech','libdeflate']:
  r=read('placement/current-runtime-profiles/'+case+'/'+actor+'/result.json');assert sha(blob('placement/current-runtime-profiles/'+case+'/'+actor+'/sample.txt'))==r['sample_sha256'];assert rows[actor]['encoder_stack_samples']>=1000;assert len(r['verified_calls'])==2
assert not read('placement/native-detail/result.json')['detail_scopes_available'];assert read('placement/native-sample/result.json')['artifacts']==selected['artifact_sha256']

final=read('final-integration/compiler.json');assert final['source_commit']==selected['source_integration_commit'] and final['compiler_sha256']==selected['compiler_sha256'];assert final['artifacts']==selected['artifact_sha256']
assert read('final-integration/fixed-support-relink/qualification-reuse.json')['worker_sha256']==selected['worker_sha256'];assert read('final-integration/fixed-support-relink/runtime.json')['worker_sha256']==selected['worker_sha256']
for key in ['format','module-layout','warning-policy']:assert read('final-integration/'+key+'.json')['exit_code']==0
assert read('final-integration/unsafe-boundary.json')['exit_code']!=0 and 'allowlisted file does not exist' in read('final-integration/unsafe-boundary.json')['stderr']
assert read('final-integration/syntax-tests.json')['exit_code']==0 and '536 passed' in blob('final-integration/syntax-tests.log').decode()
assert read('integration-layout-focused-tests/result.json')['exit_code']==0
final_build=read('final-integration/matched-builds/comparison.json');assert len(final_build['observations'])==8 and len(final_build['medians'])==4
assert sha(blob('final-integration/matched-builds/plan.json'))==final_build['plan_sha256']
for row in final_build['observations']:
 for actor,run in row.items():assert run['seconds']['source_through_strip']>=run['seconds']['source_through_link']>=run['seconds']['source']
 assert row['mech-final-source']['encoder_sha256']==selected['artifact_sha256']['program.o']
 assert row['mech-final-source']['stripped_executable']==row['mech-control-reference']['stripped_executable']
named=read('final-integration/named-rebuild/rebuild.json');assert named['full_integrated_source_identity'] and named['object_text_bytes']==63532 and named['verified_exact_calls']==162 and named['checked_failure_probe']['exact_recoveries']==12
assert not named['worker_matches_archive'] and named['worker_sha256']!=selected['worker_sha256']
print('Verified adverse controls, final compiler source/replay and fixed-support identity, 63,532 B, 81 Roc/62 C point+precision in both full orders, all raw paired ratios, errors/recovery/stack, both matched build cohorts and fresh runtime profiles. Known unsafe-audit blocker remains recorded.')
