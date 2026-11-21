"""Replay unpromoted span/probe observations without inventing a new score."""
from pathlib import Path
from hashlib import sha256
from statistics import median
from runpy import run_path
from math import isclose
import gzip,json,subprocess,sys,tarfile,tempfile
E=Path(__file__).resolve().parent;P=E.parents[2]
read=lambda p:json.loads(p.read_text());digest=lambda b:sha256(b).hexdigest()
sys.path.insert(0,str(P));import qualification_timings as timing
from qualification_records import Evidence,Findings,load_fixtures

def main():
 result=read(E/'result.json');assert result['promoted'] is False and result['qualified_selection_unchanged']
 assert result['qualified']['roc_wins']==81 and result['qualified']['libdeflate_wins']==62
 assert digest((P/'program.mec').read_bytes())==result['source_sha256']
 assert read(E/'host.json')['cpu']=='Apple M1'
 for name in ['word','probe']:
  replay=read(E/('public-source-reproduction-'+name+'.json'));pin=result['roles'][replay['role']]
  assert replay['source_sha256']==result['source_sha256'] and [r['mode'] for r in replay['variants']]==['normal','checked']
  for variant in replay['variants']:assert variant['artifacts']==pin['artifact_sha256'] and variant['compiler_sha256']==pin['compiler_sha256']
 size=run_path(str(P/'evidence/vector-mismatch-20261119/verify.py'),run_name='span_text_size')['text_size']
 for role,pin in result['roles'].items():
  raw=gzip.decompress((E/(role+'-program.o.gz')).read_bytes());assert digest(raw)==pin['artifact_sha256']['program.o'] and size(raw)==pin['text_bytes']
  assert digest(gzip.decompress((E/(role+'-metadata.json.gz')).read_bytes()))==pin['artifact_sha256']['metadata.json']
  for profile in pin['profiles']:assert digest((E/profile['path']).read_bytes())==profile['sha256']
  for name,h in pin['source_inputs'].items():assert digest(subprocess.check_output(['git','show',pin['source_basis']+':'+name],cwd=P))==h
 findings=Findings();fixtures=load_fixtures(Evidence(P,findings),read(P/'qualification-policy-v3.json'))
 with tempfile.TemporaryDirectory(prefix='mech-span-replay-') as temporary:
  root=Path(temporary);checks=root/'checks';raw=root/'raw'
  for archive,members in read(E/'archive-files.json').items():
   with tarfile.open(E/archive) as tar:
    assert sorted(tar.getnames())==sorted(members)
    for name,pin in members.items():
     part=Path(name);assert not part.is_absolute() and '..' not in part.parts
     data=tar.extractfile(name).read();assert len(data)==pin['bytes'] and digest(data)==pin['sha256']
     target=(raw if archive=='raw-runtime.tar.gz' else checks)/part;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data)
  for role,pin in result['roles'].items():
   receipt=read(checks/role/'receipt.json');bound=read(checks/role/'bound/runtime.json');assert receipt['artifacts']==pin['artifact_sha256'] and bound['worker_sha256']==pin['worker_sha256']
   assert digest((checks/role/'bound/mech-worker').read_bytes())==pin['worker_sha256']
  tests=read(checks/'admitted-input-span-composed-repository-tests/result.json');log=(checks/'admitted-input-span-composed-repository-tests/test.log').read_bytes();assert tests['exit_code']==0 and digest(log)==tests['test_log_sha256'] and b'3 passed; 0 failed' in log and b'2 passed; 0 failed' in log
  wide=read(checks/'span-composed-wide-equal-tests/result.json');assert wide['exit_code']==0 and b'10 passed; 0 failed' in (checks/'span-composed-wide-equal-tests/test.log').read_bytes()
  build=read(checks/'admitted-input-span-composed-compiler/build.json');assert build['exit_code']==0 and build['source_sha256']==tests['source_sha256']
  for name,pin in build['source_sha256'].items():assert digest((checks/'admitted-input-span-composed-compiler/source-inputs'/name).read_bytes())==pin
  assert read(checks/'span-composed-retained-replay/result.json')['qualified_all_three_exact']
  for role in ['span-late-word-profiled','span-late-vector-complements-profiled','probe-span-vector-complements-profiled']:
   receipt=read(checks/role/'receipt.json');assert read(checks/role/'checked-emission/result.json')['artifacts']==receipt['artifacts']
   recovery=read(checks/role/'promotion-recoveries/result.json');assert recovery['exact_recoveries']==12 and recovery['benchmark_worker_sha256']==result['roles'][role]['worker_sha256']
  for role in [r for r in result['roles'] if r.startswith('span-late-') or r.startswith('probe-')]:
   link=read(checks/role/'linked-proof.json');parts=Path(link['result']).parts;relative=Path(*parts[parts.index('natural4k-scorer-repair')+1:]);packet=(checks/relative).read_bytes();assert digest(packet)==link['sha256'];proof=json.loads(packet);assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6 and proof['peak_stack_bytes'][role]<=5280
  panels=0
  for stage in result['stages'].values():
   runs=list(stage['discovery']['runs'])
   for screen in stage.get('full_point_screens',{}).values():runs+=screen['runs']
   for run in runs:
    folder=raw/Path(run.get('run',run.get('folder'))).name;assert read(folder/'complete.json')['balanced_blocks_per_case']==24
    rows=run['results'];assert len(rows) in [19,81]
    for row in rows:
     name=row['case'];observations=json.loads(gzip.decompress((folder/name/'observations.json.gz').read_bytes()));blocks=timing.case_blocks(observations,fixtures[name],24)[0]
     for actor in ['mech-control','mech-experiment']:
      for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:assert isclose(1+timing.estimate(timing.paired(blocks,actor,peer)),row[key][actor],rel_tol=2e-12)
    panels+=1
  assert panels==28
  for name,stage in [('span-matched-builds','early_span'),('span-composed-matched-builds','composed_span')]:
   summary=result['stages'][stage]['builds'];comparison=read(checks/name/'comparison.json');plan=read(checks/name/'plan.json');assert digest((checks/name/'plan.json').read_bytes())==comparison['plan_sha256']==summary['plan_sha256'];observations=comparison['observations'];assert len(observations)==summary['trials']
   for actor,job in plan['actors'].items():
    assert median(row[actor]['seconds']['source_through_link'] for row in observations)==summary['medians'][actor]['source_through_link']
    if actor.startswith('mech-'):
     for row in observations:assert row[actor]['generated_artifacts']==job['artifact_sha256'] and row[actor]['stripped_executable']['bytes']==276072
  assert findings.rejected==findings.unresolved==[]
 print('Verified source/artifact pins,15 repository tests,162 outputs/six failures and stack gates,checked emissions,12 recoveries per finalist,28 two-order point acquisitions and10/12-trial complete-build cohorts. No new qualified score is claimed.')
if __name__=='__main__':main()
