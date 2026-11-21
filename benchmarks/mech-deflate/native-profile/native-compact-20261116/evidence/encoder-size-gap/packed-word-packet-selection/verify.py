"""Verify the new full96 result and its previously verified source-artifact binding."""
from pathlib import Path
from hashlib import sha256
from math import isclose
import gzip,json,shutil,subprocess,sys,tarfile,tempfile
E=Path(__file__).resolve().parent;P=E.parents[2]
read=lambda p:json.loads(p.read_text());digest=lambda b:sha256(b).hexdigest()
sys.path.insert(0,str(P))
import qualification_timings as timing
from qualification_records import Evidence,Findings,load_fixtures,PEERS,CANDIDATE

def main():
 r=read(E/'result.json');prior=E/r['source_study']['path'];old=read(prior/'result.json');v=read(prior/'verification.json')
 assert digest((prior/'result.json').read_bytes())==r['source_study']['result_sha256']==v['result_sha256']
 assert digest((prior/'verification.json').read_bytes())==r['source_study']['verification_sha256'] and v['exit_code']==0
 assert digest((prior/'verify.py').read_bytes())==v['verifier_sha256'] and digest((prior/'verification.log').read_bytes())==v['log_sha256']
 pin=r['pin'];assert pin==old['roles'][r['selected_role']] and r['builds']==old['builds']
 assert digest((P/'program.mec').read_bytes())==r['source_sha256'];assert pin['artifact_sha256']['program.mecb']==r['bytecode_sha256']
 for leaf in ['program.o','metadata.json']:assert digest(gzip.decompress((prior/(r['selected_role']+'-'+leaf+'.gz')).read_bytes()))==pin['artifact_sha256'][leaf]
 for path,h in pin['source_inputs'].items():assert digest(subprocess.check_output(['git','show',pin['source_basis']+':'+path],cwd=P))==h
 manifest=read(E/'candidate.json');assert manifest['artifact_sha256']==pin['artifact_sha256'] and manifest['worker_sha256']==pin['worker_sha256'] and manifest['encoder_text_bytes']==63804
 for row in manifest['profiles']:assert digest((E/row['path']).read_bytes())==row['sha256']
 named=read(E/'working-selection-replay.json');assert named['manifest_sha256']==digest((E/'candidate.json').read_bytes()) and named['production_recipe_assembly_checked'];assert {row['mode'] for row in named['runs']}=={'normal','checked'}
 for row in named['runs']:assert row['artifacts']==pin['artifact_sha256']
 with tempfile.TemporaryDirectory(prefix='packet-full96-') as temporary:
  root=Path(temporary);checks=root/'checks';raw=root/'raw'
  for archive,pins in read(E/'archive-files.json').items():
   cached={};seen=set()
   with tarfile.open(E/archive) as tar:
    for item in tar:
     name=item.name;part=Path(name);assert name in pins and name not in seen and not part.is_absolute() and '..' not in part.parts and (item.isfile() or item.islnk());seen.add(name)
     data=cached[item.linkname] if item.islnk() else tar.extractfile(item).read();cached[name]=data;assert digest(data)==pins[name]['sha256'] and len(data)==pins[name]['bytes']
     path=(raw if archive=='raw-runtime.tar.gz' else checks)/part;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
    assert seen==set(pins)
  name=r['full96_folder'];full=r['full96'];plan=read(checks/name/'plan.json');policy=read(checks/name/'policy.json');assert policy==r['policy']
  assert read(checks/name/'result.json')==full and digest((checks/name/'plan.json').read_bytes())==full['plan_sha256']
  assert plan['candidate_artifacts']==pin['artifact_sha256'] and plan['identities']['mech-experiment']['worker']==pin['worker_sha256']
  assert plan['identities']['mech-experiment']['metadata']==pin['artifact_sha256']['metadata.json']
  assert {x['blocks_per_case'] for x in plan['runs']}=={96} and {x['order'] for x in plan['runs']}=={'forward','reverse'}
  path=raw/'consolidation-995/natural4k-scorer-repair'/name;shutil.copytree(checks/name,path);shutil.copyfile(checks/'as-run-fixtures.json',raw/'fixtures.json')
  f=Findings();fixtures=load_fixtures(Evidence(P,f),policy);assert len(fixtures)==81 and not f.rejected and not f.unresolved
  evidence=Evidence(raw,f);runs=[]
  for entry,record in zip(plan['runs'],full['receipts'],strict=True):
   actual=timing.runtime_run(record,fixtures,evidence,plan['identities'],policy,entry);assert actual is not None;runs.append(actual)
  assert {'rejected':f.rejected,'unresolved':f.unresolved}==full['raw_validation']
  points=[]
  for run in runs:
   peers={}
   for peer in PEERS:
    wins=[name for name,ratios in run['ratios'].items() if ratios[peer][CANDIDATE]<1];missing=sorted(set(policy['required_peer_wins'][peer])-set(wins));peers[peer]={'wins':len(wins),'required_missing':missing,'passed':len(wins)>=policy['peer_win_floors'][peer] and not missing}
   points.append({'id':run['id'],'peers':peers})
  assert points==full['point_runs'] and full['passed']==(not f.rejected and not f.unresolved and all(peer['passed'] for run in points for peer in run['peers'].values()))
  report=timing.decide_runtime(runs,[],policy,fixtures,f);assert report==full['runtime_report'] and f.rejected==full['rejected'] and f.unresolved==full['unresolved']
  for actual,audit in zip(runs,full['named_win_audit'],strict=True):
   for peer,row in audit['peers'].items():
    missing=[]
    for index,(name,ratios) in enumerate(actual['ratios'].items()):
     if name not in row['cases']:continue
     parts=timing.paired(actual['cases'][name],CANDIDATE,peer)
     if max(map(max,parts))>=0:
      interval=timing.interval(parts,index,policy)
      for k in ['estimate','lower','upper']:assert isclose(interval[k],row['cases'][name]['interval'][k],rel_tol=2e-12,abs_tol=1e-14)
      supported=ratios[peer][CANDIDATE]<1 and interval['upper']<0
     else:supported=ratios[peer][CANDIDATE]<1
     assert supported==row['cases'][name]['supported']
     if not supported:missing.append(name)
    assert missing==row['unsupported']
  assert full['confidence_passed']==(not f.rejected and not f.unresolved and all(not peer['unsupported'] for run in full['named_win_audit'] for peer in run['peers'].values()))
 print('Verified63804B source/artifact binding,actual named normal/checked recipe,both full81-case96-block orders,every81Roc/63C named point and confidence win,and unchanged18-trial build evidence.')
if __name__=='__main__':main()
