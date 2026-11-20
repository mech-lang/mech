"""Replay source pins, archived observations and the point decision; confidence stays separate."""
from pathlib import Path
from hashlib import sha256
from statistics import median
from math import isclose
import gzip,json,shutil,subprocess,sys,tarfile,tempfile
E=Path(__file__).resolve().parent;P=E.parents[2]
read=lambda p:json.loads(Path(p).read_text());digest=lambda b:sha256(b).hexdigest()
sys.path.insert(0,str(P));sys.path.insert(0,str(P.parent/'compact-size'))
import mach_o
import qualification_timings as timing
from qualification_records import Evidence,Findings,load_fixtures,PEERS,CANDIDATE

def validate_counts(counts):
 for row in counts['cases'].values():
  assert sum(b['visits']*len(b['instructions']) for b in row['blocks'])==row['counts'].get('instructions',0)
  assert sum(b['visits']*sum(op=='b' or op.startswith('b.') or op in ['cbz','cbnz','tbz','tbnz'] for op,args in b['instructions']) for b in row['blocks'])==row['counts'].get('branches',0)

def main():
 result=read(E/'result.json');assert digest((P/'program.mec').read_bytes())==result['source_sha256']
 assert sum(result['groups'].values(),[])==list(result['roles'])
 for role,pin in result['roles'].items():
  obj=gzip.decompress((E/(role+'-program.o.gz')).read_bytes());assert digest(obj)==pin['artifact_sha256']['program.o']
  with tempfile.NamedTemporaryFile() as f:
   f.write(obj);f.flush();_,sections,_=mach_o.load(Path(f.name));assert sections[mach_o.TEXT][2]==pin['text_bytes']
  assert digest(gzip.decompress((E/(role+'-metadata.json.gz')).read_bytes()))==pin['artifact_sha256']['metadata.json']
  assert pin['artifact_sha256']['program.mecb']==result['bytecode_sha256']
  for profile in pin['profiles']:assert digest((E/profile['path']).read_bytes())==profile['sha256']
  for path,h in pin['source_inputs'].items():assert digest(subprocess.check_output(['git','show',pin['source_basis']+':'+path],cwd=P))==h
 portable=read(E/'portable-replays.json');assert {r['role'] for r in portable['runs']}==set(result['roles'])
 for row in portable['runs']:
  assert row['source_sha256']==result['source_sha256'] and {r['mode'] for r in row['variants']}=={'normal','checked'}
  for run in row['variants']:assert run['artifacts']==result['roles'][row['role']]['artifact_sha256'] and run['compiler_sha256']==portable['compiler_sha256']
 with tempfile.TemporaryDirectory(prefix='packed-word-evidence-') as temporary:
  root=Path(temporary);checks=root/'checks';raw=root/'raw'
  for archive,members in read(E/'archive-files.json').items():
   with tarfile.open(E/archive) as tar:
    cached={};seen=set()
    for member in tar:
     name=member.name;part=Path(name);assert name in members and name not in seen and not part.is_absolute() and '..' not in part.parts and (member.isfile() or member.islnk());seen.add(name)
     data=cached[member.linkname] if member.islnk() else tar.extractfile(member).read();cached[name]=data
     assert len(data)==members[name]['bytes'] and digest(data)==members[name]['sha256']
     target=(raw if archive=='raw-runtime.tar.gz' else checks)/part;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data)
    assert seen==set(members)
  for role,pin in result['roles'].items():
   receipt=read(checks/role/'receipt.json');assert receipt['artifacts']==pin['artifact_sha256']
   for name,h in pin['artifact_sha256'].items():assert digest((checks/role/'frontend'/name).read_bytes())==h
   assert digest((checks/role/'bound/mech-worker').read_bytes())==pin['worker_sha256']
   linked=read(checks/role/'linked-proof.json');parts=Path(linked['result']).parts;relative=Path(*parts[parts.index('natural4k-scorer-repair')+1:]);data=(checks/relative).read_bytes();assert digest(data)==linked['sha256'];proof=json.loads(data)
   assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6 and proof['peak_stack_bytes'][role]==linked['peak_stack_bytes']<=5280
   recovery=read(checks/role/'promotion-recoveries/result.json');assert recovery['observations']==recovery['exact_recoveries']==12 and recovery['benchmark_worker_sha256']==pin['worker_sha256']
   assert read(checks/role/'checked-emission/result.json')['artifacts']==pin['artifact_sha256']
   raw_role=role.removesuffix('-profiled');counts=read(checks/raw_role/'instruction-counts/counts.json');assert counts['object_sha256']==read(checks/raw_role/'receipt.json')['artifacts']['program.o'];validate_counts(counts)
  for group,pin in result['native_tests'].items():
   folder=checks/(group+'-repository-tests');tests=read(folder/'result.json');log=(folder/'test.log').read_bytes();assert tests['exit_code']==0 and digest(log)==tests['test_log_sha256'] and f"{pin['tests']} passed; 0 failed".encode() in log
   build=read(checks/(group+'-compiler/build.json'));assert build['exit_code']==0 and build['source_sha256']==tests['source_sha256']
   for path,h in build['source_sha256'].items():assert digest((checks/(group+'-compiler/source-inputs')/path).read_bytes())==h and digest(subprocess.check_output(['git','show',build['source_base']+':'+path],cwd=P))==h
   replay=read(checks/(group+'-disabled-replays/result.json'));assert replay['all_three_exact'] and len(replay['runs'])==pin['older_replays']
   for row in replay['runs']:
    for name,h in row['artifacts'].items():assert digest((checks/(group+'-disabled-replays')/row['role']/'frontend'/name).read_bytes())==h
   enabled=read(checks/(group+'-enabled-tests/result.json'));assert enabled['complete'] and len(enabled['runs'])==3 and enabled['source_sha256']==build['source_sha256']
   for row in enabled['runs']:
    log=(checks/(group+'-enabled-tests')/(row['label']+'.log')).read_bytes();assert row['exit_code']==0 and digest(log)==row['log_sha256'] and f"{pin['tests']} passed; 0 failed".encode() in log
  for row in result['counter_comparisons']:
   a=checks/row['previous_role']/'instruction-counts/counts.json';b=checks/row['current_role']/'instruction-counts/counts.json';assert digest(a.read_bytes())==row['before_sha256'] and digest(b.read_bytes())==row['after_sha256'];before=read(a);after=read(b)
   for name,values in row['cases'].items():
    assert values['before']=={k:before['cases'][name]['counts'].get(k,0) for k in values['before']};assert values['after']=={k:after['cases'][name]['counts'].get(k,0) for k in values['after']};assert values['delta']=={k:values['after'][k]-values['before'][k] for k in values['before']}
  findings=Findings();policy=result['working_policy'];fixtures=load_fixtures(Evidence(P,findings),policy);panels=0
  for group,stage in result['stages'].items():
   assert stage['complete']
   for run in stage['runs']:
    folder=raw/Path(run['run']).name;assert read(folder/'complete.json')['balanced_blocks_per_case']==24 and len(run['results'])==21;pin=read(folder/'pins.json')['actors']
    assert pin['mech-experiment']['binary_sha256']==result['roles'][run['role']]['worker_sha256'] and pin['mech-control']['binary_sha256']==stage['plan']['control_worker_sha256_by_role'][run['role']]
    for row in run['results']:
     blocks=timing.case_blocks(json.loads(gzip.decompress((folder/row['case']/'observations.json.gz').read_bytes())),fixtures[row['case']],24)[0]
     for actor in ['mech-control','mech-experiment']:
      for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:assert isclose(1+timing.estimate(timing.paired(blocks,actor,peer)),row[key][actor],rel_tol=2e-12)
    audit=next(x for x in result['panel_audit'] if x['group']==group and x['role']==run['role'] and x['order']==run['order'])
    assert audit['required_point_wins_missing']=={peer:[row['case'] for row in run['results'] if row['case'] in policy['required_peer_wins'][peer] and row[key]['mech-experiment']>=1] for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]};panels+=1
  assert panels==24 and not findings.rejected and not findings.unresolved
  full=result['full96'];name=result['full96_folder'];assert read(checks/name/'result.json')==full;plan=read(checks/name/'plan.json');assert digest((checks/name/'plan.json').read_bytes())==full['plan_sha256'];assert read(checks/name/'policy.json')==policy
  destination=raw/'consolidation-995/natural4k-scorer-repair'/name;shutil.copytree(checks/name,destination);shutil.copyfile(checks/'as-run-fixtures.json',raw/'fixtures.json');qfind=Findings();qev=Evidence(raw,qfind);runs=[]
  for entry,record in zip(plan['runs'],full['receipts'],strict=True):
   actual=timing.runtime_run(record,fixtures,qev,plan['identities'],policy,entry);assert actual is not None;runs.append(actual)
  assert {'rejected':qfind.rejected,'unresolved':qfind.unresolved}==full['raw_validation']
  point=[]
  for run in runs:
   peers={}
   for peer in PEERS:
    wins=[name for name,ratios in run['ratios'].items() if ratios[peer][CANDIDATE]<1];missing=sorted(set(policy['required_peer_wins'][peer])-set(wins));peers[peer]={'wins':len(wins),'required_missing':missing,'passed':len(wins)>=policy['peer_win_floors'][peer] and not missing}
   point.append({'id':run['id'],'peers':peers})
  assert point==full['point_runs'];assert full['passed']==(not qfind.rejected and not qfind.unresolved and all(p['passed'] for r in point for p in r['peers'].values()))
  report=timing.decide_runtime(runs,[],policy,fixtures,qfind);assert report==full['runtime_report'] and qfind.rejected==full['rejected'] and qfind.unresolved==full['unresolved']
  for actual,audit in zip(runs,full['named_win_audit'],strict=True):
   for peer,row in audit['peers'].items():
    unsupported=[]
    for index,(name,ratios) in enumerate(actual['ratios'].items()):
     if name not in row['cases']:continue
     parts=timing.paired(actual['cases'][name],CANDIDATE,peer)
     if max(map(max,parts))>=0:
      ci=timing.interval(parts,index,policy)
      for k in ['estimate','lower','upper']:assert isclose(ci[k],row['cases'][name]['interval'][k],rel_tol=2e-12,abs_tol=1e-14)
      supported=ratios[peer][CANDIDATE]<1 and ci['upper']<0
     else:supported=ratios[peer][CANDIDATE]<1
     assert supported==row['cases'][name]['supported']
     if not supported:unsupported.append(name)
    assert unsupported==row['unsupported']
  assert full['confidence_passed']==(not qfind.rejected and not qfind.unresolved and all(not peer['unsupported'] for r in full['named_win_audit'] for peer in r['peers'].values()))
  cohort=checks/result['build_folder'];comparison=read(cohort/'comparison.json');plan=read(cohort/'plan.json');assert digest((cohort/'plan.json').read_bytes())==comparison['plan_sha256']==result['builds']['plan_sha256'];assert len(comparison['observations'])==result['builds']['trials']==18
  for actor,job in plan['actors'].items():
   assert median(row[actor]['seconds']['source_through_link'] for row in comparison['observations'])==result['builds']['medians'][actor]['source_through_link']
   if actor.startswith('mech-'):
    for row in comparison['observations']:assert row[actor]['generated_artifacts']==job['artifact_sha256']
 print('Verified12 source-built repairs,4x20 and4x21 optimized tests,45/53 older exact recipes,12 portable normal/checked replays,24 discovery orders,both full81/96 point and confidence outcomes,and18 balanced complete-build trials per actor.')
if __name__=='__main__':main()
