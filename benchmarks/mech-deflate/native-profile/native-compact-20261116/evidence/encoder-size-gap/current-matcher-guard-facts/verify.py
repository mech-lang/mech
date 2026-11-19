"""Check archived source/graph bindings, both runtime orders and complete-build accounting."""
from pathlib import Path
from hashlib import sha256
import gzip,json,math,tarfile
here=Path(__file__).resolve().parent;packet=here.parents[2];read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(p.read_bytes()).hexdigest()
manifest=read(here/'files.json')
for name,pin in manifest['files'].items():assert sha(here/name)==pin,name
candidate=read(here/'candidate.json');result=read(here/'result.json');assert sha(packet/'program.mec')==candidate['source_sha256']
for name,pin in candidate['artifact_sha256'].items():assert sha256(gzip.decompress((here/'guards-profiled'/(name+'.gz')).read_bytes())).hexdigest()==pin
assert candidate['encoder_text_bytes']==result['encoder_bytes']==63692
checker=read(here/'guards-profiled-checked/checker-match.json')['artifacts']
assert candidate['artifact_sha256']==checker
proof=read(here/'layout-proof.json');shape=proof['shape']
matcher,=[p for p in candidate['profiles'] if p['symbol']=='mech_compact_numeric_475_vector']
profile=read(here/matcher['path']);assert sorted(profile['order'])==shape['heads'] and profile['order'][0]==0
assert sha(here/matcher['path'])==proof['profile_sha256']
training=json.loads(gzip.decompress((here/'matcher-counts/counts.json.gz').read_bytes()));assert sha256(gzip.decompress((here/'matcher-counts/counts.json.gz').read_bytes())).hexdigest()==proof['training_sha256']
assert training['object_sha256']==shape['object_sha256']
for p in proof['unchanged_other_profiles']:assert p['same_ordered_graph'] and p['control']['blocks']==p['candidate']['blocks'] and p['control']['heads']==p['candidate']['heads']
assert read(here/'unsigned-guard-tests.json')['exit_code']==0
assert read(here/'linked-checks.json')['matches_success_observations']==162 and result['source_exact_recoveries']==12 and result['source_failure_cases']==6 and result['peak_stack_bytes']==4848
replay=json.loads(gzip.decompress((here/'full/full-replay.json.gz').read_bytes()));screen=read(here/'full/screen.json');assert screen['decision']=='Retain' and screen['rejected']==screen['unresolved']==[]
assert len(replay['runs'])==2
for run in replay['runs']:
 assert run['case_count']==81
 for peer,floor in [('roc',81),('libdeflate',62)]:
  for field in ['point_estimate_wins','precision_supported_wins']:assert run['peers'][peer][field]['mech-experiment']>=floor
for row in read(here/'required-wins.json'):
 for peer in row['peers'].values():assert peer['lost']==[]
ratios={(r['order'],r['case']):r for r in read(here/'runtime-case-ratios.json')}
for order in ['forward','reverse']:
 with tarfile.open(here/'full'/(order+'-observations.tar.gz')) as tar:
  observations=[m for m in tar.getmembers() if m.name.endswith('/observations.json.gz')];assert len(observations)==81
  for member in observations:
   case=Path(member.name).parent.name;data=json.loads(gzip.decompress(tar.extractfile(member).read()));blocks=data['blocks'] if isinstance(data,dict) else data;assert len(blocks)==96
   # Independently recompute the paired log-mean ratios from the archived blocks.
   row=ratios[(order,case)]
   for actor,key in [('mech-control','candidate_control'),('roc','candidate_roc'),('libdeflate','candidate_c')]:
    ratio=math.exp(sum(math.log(b['means']['mech-experiment']/b['means'][actor]) for b in blocks)/len(blocks));assert abs(ratio-row[key])<1e-10,(case,key)
build=json.loads(gzip.decompress((here/'build-comparison.json.gz').read_bytes()));assert len(build['observations'])==8
assert set(build['medians'])=={'mech-control-reference','mech-current-matcher-guards','roc','libdeflate'}
for row in build['observations']:
 for actor,r in row.items():assert r['seconds']['source_through_link']>=r['seconds']['source'] and r['seconds']['source_through_strip']>=r['seconds']['source_through_link']
print('Verified source artifacts, checked allocation, fresh matcher layout and two unchanged profile graphs, all existing peer wins in both complete orders, raw paired ratios, stack/recovery and eight matched complete builds per actor.')
