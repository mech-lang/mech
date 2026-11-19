"""Check archived source/graph bindings, both runtime orders and complete-build accounting."""
from pathlib import Path
from hashlib import sha256
import gzip,json,math,tarfile
here=Path(__file__).resolve().parent;packet=here.parents[2];read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(p.read_bytes()).hexdigest()
manifest=read(here/'files.json')
for name,pin in manifest['files'].items():assert sha(here/name)==pin,name
candidate=read(here/'candidate.json');result=read(here/'result.json');assert sha(packet/'program.mec')==candidate['source_sha256']
for name,pin in candidate['artifact_sha256'].items():assert sha256(gzip.decompress((here/'paired-profiled'/(name+'.gz')).read_bytes())).hexdigest()==pin
assert candidate['encoder_text_bytes']==result['encoder_bytes']==63756
checker=read(here/'paired-profiled-checked/checker-match.json')['matched']
for name,k in [('program.o','object_sha256'),('metadata.json','metadata_sha256'),('program.mecb','bytecode_sha256')]:assert candidate['artifact_sha256'][name]==checker[k]
proof=read(here/'profile-rebase.json')
for p in proof:
 assert p['control']['code_blocks']==p['paired']['code_blocks']
 a=p['control'];b=p['paired'];old_file=here/'profiles-reference'/Path(p['old_profile']).name;assert sha(old_file)==p['old_profile_sha256'];old=read(old_file)
 assert sorted(old['order'])==a['heads'];mapping=dict(zip(a['code_heads'],b['code_heads'],strict=True))
 new=read(here/'profiles'/Path(p['new_profile']).name);assert sorted(new['order'])==b['heads'];assert sha(here/'profiles'/Path(p['new_profile']).name)==p['new_profile_sha256']
 assert new['order']==[mapping[k] for k in old['order'] if k in mapping]+b['zero_literal_padding']
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
assert set(build['medians'])=={'mech-control-reference','mech-current-spill-pairs','roc','libdeflate'}
for row in build['observations']:
 for actor,r in row.items():assert r['seconds']['source_through_link']>=r['seconds']['source'] and r['seconds']['source_through_strip']>=r['seconds']['source_through_link']
print('Verified source artifacts, checked allocation, three layout graphs, all existing peer wins in both complete orders, raw paired ratios, stack/recovery and eight matched complete builds per actor.')
