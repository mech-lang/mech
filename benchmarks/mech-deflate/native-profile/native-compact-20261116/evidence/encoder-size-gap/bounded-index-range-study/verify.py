"""Replay the experimental point results; never turn them into a qualified score."""
from pathlib import Path
from hashlib import sha256
from statistics import median
from runpy import run_path
from math import isclose
import gzip,json,subprocess,sys,tarfile,tempfile
E=Path(__file__).resolve().parent;P=E.parents[2];read=lambda p:json.loads(p.read_text());digest=lambda b:sha256(b).hexdigest()
sys.path.insert(0,str(P));import qualification_timings as t
from qualification_records import Evidence,Findings,load_fixtures

def main():
 c=read(E/'candidate.json');result=read(E/'result.json');assert c['runtime_qualified'] is False and result['promoted'] is False and result['qualified_selection_unchanged']
 assert digest((P/'program.mec').read_bytes())==c['source_sha256']
 for name in ['program.o','metadata.json']:assert digest(gzip.decompress((E/(name+'.gz')).read_bytes()))==c['artifact_sha256'][name]
 assert digest(gzip.decompress((E/'worker-mech-worker.gz').read_bytes()))==c['worker_sha256']
 size=run_path(str(P/'evidence/vector-mismatch-20261119/verify.py'),run_name='bounded_index_text')['text_size'];assert size(gzip.decompress((E/'program.o.gz').read_bytes()))==c['encoder_text_bytes']==63388
 for profile in c['profiles']:assert digest((E/profile['path']).read_bytes())==profile['sha256']
 for name,pin in c['source_inputs'].items():assert digest(subprocess.check_output(['git','show',c['source_integration_commit']+':'+name],cwd=P))==pin
 public=read(E/'public-source-reproduction.json');assert public['source_sha256']==c['source_sha256'] and [row['mode'] for row in public['variants']]==['normal','checked']
 for row in public['variants']:assert row['compiler_sha256']==c['compiler_sha256'] and row['artifacts']==c['artifact_sha256']
 named=read(E/'named-rebuild.json');assert named['variant']==c['recipe'] and named['object_sha256']==c['artifact_sha256']['program.o'] and named['metadata_sha256']==c['artifact_sha256']['metadata.json'] and named['bytecode_sha256']==c['artifact_sha256']['program.mecb'] and named['object_text_bytes']==63388
 policy=read(P/'qualification-policy-v3.json');findings=Findings();fixtures=load_fixtures(Evidence(P,findings),policy)
 with tempfile.TemporaryDirectory(prefix='mech-index-replay-') as temp:
  root=Path(temp);n=root/'checks';q=root/'raw';n.mkdir();q.mkdir()
  for archive,members in read(E/'archive-files.json').items():
   with tarfile.open(E/archive) as tar:
    assert sorted(tar.getnames())==sorted(members)
    for name,pin in members.items():
     part=Path(name);assert not part.is_absolute() and '..' not in part.parts
     raw=tar.extractfile(name).read();assert len(raw)==pin['bytes'] and digest(raw)==pin['sha256'];target=(q if archive=='raw-runtime.tar.gz' else n)/part;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
  build=read(n/'small-table-entry-guard-cleanup-compiler/build.json');assert build['exit_code']==0 and build['compiler_sha256']==c['compiler_sha256'] and build['source_sha256']==c['source_inputs']
  tests=read(n/'small-table-entry-guard-cleanup-repository-tests/result.json');log=(n/'small-table-entry-guard-cleanup-repository-tests/test.log').read_bytes();assert tests['exit_code']==0 and tests['source_sha256']==c['source_inputs'] and digest(log)==tests['test_log_sha256'] and b'2 passed; 0 failed' in log
  for name,pin in c['source_inputs'].items():assert digest((n/'small-table-entry-guard-cleanup-compiler/source-inputs'/name).read_bytes())==pin
  for name in ['qualification-policy-v3.json','qualification_records.py','qualification_timings.py']:assert (P/name).read_bytes()==(n/'small-table-qualification-source'/name).read_bytes()
  roles=['small-table-fast-word','small-table-fast-vector-complements'];workers={}
  for role in roles:
   r=read(n/role/'receipt.json');pin=read(n/role/'bound/runtime.json');workers[role]=pin['worker_sha256'];assert pin['object_sha256']==r['artifacts']['program.o'] and pin['metadata_sha256']==r['artifacts']['metadata.json']
   checked=read(n/role/'checked-emission/result.json');assert checked['exact_all_three'] and checked['artifacts']==r['artifacts']
   recovery=read(n/role/'promotion-recoveries/result.json');assert recovery['exact_recoveries']==12 and recovery['benchmark_worker_sha256']==pin['worker_sha256']
   link=read(n/role/'linked-proof.json');path=Path(link['result']);parts=path.parts;part=Path(*parts[parts.index('natural4k-scorer-repair')+1:]);raw=(n/part).read_bytes();assert digest(raw)==link['sha256'];proof=json.loads(raw);assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6 and proof['peak_stack_bytes'][role]==4720
  assert workers[roles[0]]==c['worker_sha256']
  required=read(n/'word-complements-prior-wins.json')['required_previous_individual_wins']
  def replay(run,role,full):
   folder=q/Path(run.get('folder',run.get('run'))).name;assert read(folder/'complete.json')['balanced_blocks_per_case']==24
   pins=read(folder/'pins.json');assert pins['actors']['mech-experiment']['binary_sha256']==read(n/role/'bound/runtime.json')['worker_sha256']
   rows=run.get('results');assert len(rows)==(81 if full else 19);names=set()
   for row in rows:
    name=row['case'];names.add(name);raw=json.loads(gzip.decompress((folder/name/'observations.json.gz').read_bytes()));blocks=t.case_blocks(raw,fixtures[name],24)[0]
    for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
     ratio=1+t.estimate(t.paired(blocks,'mech-experiment',peer));assert isclose(ratio,row[key]['mech-experiment'],rel_tol=2e-12),(name,peer,ratio,row[key])
   if full:
    assert names==set(fixtures)
    for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
     unsupported=[r['case'] for r in rows if r['case'] in required[peer] and r[key]['mech-experiment']>=1];assert unsupported==run['peers'][peer]['lost_previous_named_wins']==[]
    assert run['peers']['roc']['wins']==81 and run['peers']['libdeflate']['wins']==62
  for run in result['discovery']['runs']:replay(run,run['role'],False)
  for role,screen in result['full_point_screens'].items():
   assert screen['complete'] and len(screen['runs'])==2
   for run in screen['runs']:replay(run,role,True)
  for folder,summary in result['builds'].items():
   comparison=read(n/folder/'comparison.json');plan_path=n/folder/'plan.json';assert digest(plan_path.read_bytes())==comparison['plan_sha256']==summary['plan_sha256'];plan=read(plan_path);observations=comparison['observations'];assert len(observations)==summary['trials']==10
   for actor in plan['actors']:
    assert median(row[actor]['seconds']['source_through_link'] for row in observations)==summary['medians'][actor]['source_through_link']
    if actor.startswith('mech-'):
     for row in observations:
      assert row[actor]['generated_artifacts']==plan['actors'][actor]['artifact_sha256'] and row[actor]['stripped_executable']['bytes']==276072
  follow=result['followup_bitwise'];test=read(n/'bitwise-index-range-repository-tests/result.json');build=read(n/'bitwise-index-range-compiler/build.json');assert test['exit_code']==build['exit_code']==0 and test['source_sha256']==build['source_sha256'] and build['compiler_sha256']==follow['compiler_sha256']
  log=(n/'bitwise-index-range-repository-tests/test.log').read_bytes();assert digest(log)==test['test_log_sha256'] and log.count(b'3 passed; 0 failed')==2
  for name,pin in build['source_sha256'].items():assert digest(subprocess.check_output(['git','show',follow['source_commit']+':'+name],cwd=P))==pin
  disabled=read(n/'bitwise-disabled-control/receipt.json');assert disabled['artifacts']==c['artifact_sha256']
  for role in ['bitwise-word','bitwise-vector','bitwise-vector-complements']:
   r=read(n/role/'receipt.json');assert r['admitted_index_guards']==[{'header':475,'symbol':'mech_compact_numeric_475_vector','checks_removed':4}]
   link=read(n/role/'linked-proof.json');path=Path(link['result']);parts=path.parts;part=Path(*parts[parts.index('natural4k-scorer-repair')+1:]);raw=(n/part).read_bytes();assert digest(raw)==link['sha256'];proof=json.loads(raw);assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6 and proof['peak_stack_bytes'][role]<=5280
  assert 'allowlisted file does not exist' in (n/'small-table-static-contracts/2.log').read_text()
  assert findings.rejected==findings.unresolved==[]
 print('Verified source/artifacts,63388 B,4720 B stack, native tests,162 outputs/six failures,12 recoveries, checked emission, all six discovery orders, four full81-case point orders and three10-trial build cohorts. Experimental: full96 confidence qualification remains outstanding.')
if __name__=='__main__':main()
