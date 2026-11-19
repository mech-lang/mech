"""Publish source guard proof plus measured matcher layout, without transferring old qualification."""
from pathlib import Path
from hashlib import sha256
from shutil import copyfile
from statistics import median
import gzip,json,tarfile,random
T=Path(__file__).resolve().parent;Q=T.parent.parent;F=T/'hot-matcher-facts';S=T/'retest-register-spills';trial=F/'guards-profiled'
P=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116');E=P/'evidence/encoder-size-gap/current-matcher-guard-facts';E.mkdir(exist_ok=True)
read=lambda p:json.loads(Path(p).read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,indent=2)+'\n')
def copy(a,b,z=False):
 b=E/b;b.parent.mkdir(parents=True,exist_ok=True)
 if z:b.write_bytes(gzip.compress(a.read_bytes(),mtime=0))
 else:copyfile(a,b)
def archive(dest,items):
 (E/dest).parent.mkdir(parents=True,exist_ok=True)
 with tarfile.open(E/dest,'w:gz') as tar:
  for source,name in items:tar.add(source,arcname=name)
runtime=read(trial/'full/full-replay.json');gate=read(trial/'full/screen.json');build=read(F/'matched-builds/summary.json');receipt=read(trial/'receipt.json');proof=read(trial/'linked-proof.json');recoveries=read(trial/'recoveries/result.json')
assert gate['decision']=='Retain' and gate['rejected']==gate['unresolved']==[]
assert proof['peak_stack_bytes']<=5280 and recoveries['exact_recoveries']==12
for run in runtime['runs']:
 assert run['case_count']==81
 for peer,count in [('roc',81),('libdeflate',62)]:
  assert run['peers'][peer]['point_estimate_wins']['mech-experiment']>=count
  assert run['peers'][peer]['precision_supported_wins']['mech-experiment']>=count
retained=[];rows=[]
for order in ['forward','reverse']:
 old=read(Q/f'spill-pair-current-paired-profiled-full-{order}/summary.json');new=read(Q/f'matcher-guard-current-guards-profiled-full-{order}/summary.json');a={r['case']:r for r in old};b={r['case']:r for r in new};assert a.keys()==b.keys() and len(a)==81;peers={}
 for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
  required=[n for n,r in a.items() if r[key]['mech-experiment']<1];lost=[n for n in required if b[n][key]['mech-experiment']>=1];assert not lost,(order,peer,lost);peers[peer]={'required':required,'lost':lost}
 retained.append({'order':order,'peers':peers})
 for r in new:rows.append({'order':order,'case':r['case'],'candidate_control':r['paired_x_roc']['mech-experiment']/r['paired_x_roc']['mech-control'],'candidate_roc':r['paired_x_roc']['mech-experiment'],'candidate_c':r['paired_x_libdeflate']['mech-experiment'],'control_c':r['paired_x_libdeflate']['mech-control'],'medians_seconds':r['median_seconds']})
save(E/'required-wins.json',retained);save(E/'runtime-case-ratios.json',rows)
for n in ['plan.md','preflight.json','layout-proof.json','profile-v1.log','profile-v2.log','profile-v3.log','profile-refinement-restart.json','unsigned-guard-tests.json','unsigned-guard-tests.log']:copy(F/n,Path(n))
for role in ['unsigned-guards','unsigned-guards-checked','guards-profiled']:
 folder=F/role;copy(folder/'receipt.json',Path(role)/'receipt-as-run.json');copy(folder/'compile.log',Path(role)/'compile.log.gz',True)
 for n in ['program.o','metadata.json','program.mecb']:copy(folder/'frontend'/n,Path(role)/(n+'.gz'),True)
copy(F/'guards-profiled-checked/checker-match.json',Path('guards-profiled-checked/checker-match.json'));copy(F/'guards-profiled-checked/compile.log',Path('guards-profiled-checked/compile.log.gz'),True)
for n in ['runtime.json','mech-worker']:copy(trial/'bound'/n,Path('guards-profiled')/(n+('.gz' if n=='mech-worker' else '')),n=='mech-worker')
copy(trial/'linked-proof.json',Path('linked-proof.json'));copy(Path(proof['result']),Path('linked-checks.json'));archive('linked-checks.tar.gz',[(F/'linked-current-guards-profiled','linked-current-guards-profiled'),(trial/'recoveries','recoveries')])
for name in ['matcher-counts','profiled-matcher-counts']:
 copy(F/name/'counts.json',Path(name)/'counts.json.gz',True);copy(F/name/'plan.json',Path(name)/'plan.json.gz',True);archive(name+'/raw.tar.gz',[(p,p.name) for p in sorted((F/name).iterdir()) if p.name not in ['counts.json','plan.json']])
copy(S/'unprofiled-paired/receipt.json',Path('unprofiled-control-receipt.json'));copy(S/'matcher-counts/counts.json',Path('control-counts.json.gz'),True)
for kind in ['focus','full']:
 for n in ['plan.json','screen.json','environment.json','contract.json','fixture-content.json']:copy(trial/kind/n,Path(kind)/n)
 for n in [kind+'-replay.json','acquire.log','replay.log']:copy(trial/kind/n,Path(kind)/(n+'.gz'),True)
 for run in read(trial/kind/'plan.json')['runs']:archive(kind+'/'+run['order']+'-observations.tar.gz',[(Q/run['id'],run['id'])])
for n,dest,z in [('plan.json','build-plan.json',False),('summary.json','deployment.json',False),('comparison.json','build-comparison.json.gz',True)]:copy(F/'matched-builds'/n,Path(dest),z)
archive('build-logs.tar.gz',[(p,str(p.relative_to(F))) for p in (F/'matched-builds').rglob('*') if p.is_file() and p.suffix in ['.log','.txt']])
for n in ['screen-matcher-facts.py','count-guard-matcher.py','profile-matcher-guards-v1.py','profile-matcher-guards-v2.py','profile-matcher-guards.py','count-profiled-guard-matcher.py','matcher-guards-qualify.py','matcher-guards-recoveries.py','measure-matcher-guards.py','publish-matcher-guards.py']:copy(T/n,Path('as-run')/n)
base=read(P/'evidence/encoder-size-gap/current-register-spill-pairs/candidate.json');candidate=base|{'review_parent':'5a89f646664b86a0b808137c01c7627b2142552a','source_commit':'5a89f646664b86a0b808137c01c7627b2142552a','compiler_sha256':receipt['compiler_sha256'],'encoder_text_bytes':receipt['text_bytes'],'artifact_sha256':receipt['artifacts'],'worker_sha256':read(trial/'bound/runtime.json')['worker_sha256'],'profiles':[],'override_flags':base['override_flags']|{'MECH_PRIVATE_VECTOR_UNSIGNED_GUARDS':'1'},'native_flags_as_run':receipt['flags'],'qualification_source_commit':'5a89f646664b86a0b808137c01c7627b2142552a','qualification_basis':'Fresh both-order actual-worker runtime, allocation checker, outputs/errors/recovery, stack, complete builds. Guard proof and fresh layout are measured as one combination.','status':'Qualified current matcher guard facts and layout; no new C win.'}
for value in receipt['flags']['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';'):
 symbol,path=value.split('@',1);copy(Path(path),Path('profiles')/Path(path).name);candidate['profiles'].append({'symbol':symbol,'path':'profiles/'+Path(path).name,'sha256':sha(path)})
save(E/'candidate.json',candidate)
control=read(S/'matcher-counts/counts.json');paired=read(F/'profiled-matcher-counts/counts.json');counter=[]
for name in control['cases'].keys()&paired['cases'].keys():
 a=control['cases'][name];b=paired['cases'][name];counter.append({'case':name,'calls':a['calls'],'control':a['counts'],'candidate':b['counts'],'instructions_removed':a['counts']['instructions']-b['counts']['instructions'],'branches_removed':a['counts']['branches']-b['counts']['branches'],'stack_store_instructions_removed':a['counts']['stack_stores']-b['counts']['stack_stores']})
raw=read(F/'matched-builds/comparison.json');delta=[(row['mech-current-matcher-guards']['seconds']['source_through_link']-row['mech-control-reference']['seconds']['source_through_link'])*1000 for row in raw['observations']];rng=random.Random(9951119);boots=sorted(median(rng.choices(delta,k=len(delta))) for _ in range(20000))
result={'source_commit':candidate['source_commit'],'compiler_sha256':receipt['compiler_sha256'],'object_sha256':receipt['object_sha256'],'metadata_sha256':receipt['metadata_sha256'],'bytecode_sha256':receipt['bytecode_sha256'],'worker_sha256':candidate['worker_sha256'],'decision':gate['decision'],'encoder_bytes':receipt['text_bytes'],'encoder_bytes_saved':63756-receipt['text_bytes'],'encoder_x_libdeflate':receipt['text_bytes']/37080,'encoder_x_roc':receipt['text_bytes']/57696,'root_bytes':read(trial/'frontend/metadata.json')['code_bytes'],'root_stack_bytes':read(trial/'frontend/metadata.json')['stack_bytes'],'peak_stack_bytes':proof['peak_stack_bytes'],'source_success_observations':162,'source_failure_cases':recoveries['cases'],'source_exact_recoveries':recoveries['exact_recoveries'],'wins':runtime['runs'],'counter_differences':counter,'complete_builds':build,'paired_complete_build_cost_ms':{'values':delta,'median':median(delta),'interval90':[boots[999],boots[18999]],'bootstrap_resamples':20000,'seed':9951119},'runtime_tradeoffs':runtime['candidate_control_timing_observations'],'recipe':'m1-matcher-guard-facts','limitation':'No new C win. All observed differences belong to the guard proof plus fresh layout, without a separate effects ablation. Profile edge estimates only chose layout; actual counters and complete timing are reported separately.'};save(E/'result.json',result)
print('Archived current matcher guard facts and full qualification:',result['encoder_bytes'],result['worker_sha256'])
