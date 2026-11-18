"""Price the qualified smaller root with the same compiler and link support."""
from pathlib import Path
from runpy import run_path
from statistics import median
import json,os,subprocess,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE/'preserved-registers'
read=lambda p:json.loads(p.read_text())
script=HERE.parent/'current-roadmap/shared-terminal-returns/measure.py'
sys.path.insert(0,str(script.parent));lab=run_path(str(script),run_name='preserved_root_matched_builds')
sha=lab['sha'];prior=read(HERE/'matched-heap-recovery/plan.json');fixed=prior['actors']['mech-m1-shared-heap-layout']
base=read(ROOT/'control-source/rebuild.json');candidate=read(ROOT/'preserved/receipt.json')
compiler=candidate['compile'][0];assert sha(compiler)==base['compiler_sha256']==candidate['compiler_sha256']
CONTROL='mech-control-reference';ROLE='mech-preserved';BASE=candidate['source_commit']

def prepare(controls,variants):
 assert controls==['reference'] and variants==['preserved']
 proof_pin=read(ROOT/'preserved/linked-proof.json');proof=read(Path(proof_pin['result']))
 assert sha(proof_pin['result'])==proof_pin['sha256']
 assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6
 actors={}
 for role,tag,flags,pins,size in [
  (CONTROL,'control',base['native_flags'],{'encoder':base['object_sha256'],'metadata':base['metadata_sha256'],'bytecode':base['bytecode_sha256']},64444),
  (ROLE,'preserved',candidate['flags'],{'encoder':candidate['object_sha256'],'metadata':candidate['metadata_sha256'],'bytecode':candidate['bytecode_sha256']},63804),
 ]:
  trial=ROOT/('timing-'+tag);trial.mkdir();output=trial/'frontend'
  command=[compiler,base['phases']['source_to_native']['command'][1],str(output),'compact']
  with (trial/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
  assert all(sha(output/name)==pins[key] for name,key in [('program.o','encoder'),('metadata.json','metadata'),('program.mecb','bytecode')])
  obj=output/'program.o'
  actors[role]=fixed|{'compile':command,'compiler_sha256':base['compiler_sha256'],'reference_object':str(obj),'flags':flags,'link':[str(obj) if arg==fixed['reference_object'] else arg for arg in fixed['link']],'artifact_sha256':pins,'text_bytes':size,'linked_proof_sha256':proof_pin['sha256']}
 assert actors[CONTROL]['flags']|{'MECH_COMPACT_PREFER_PRESERVED_REGS':'1'}==actors[ROLE]['flags']
 actors.update({peer:prior['actors'][peer] for peer in lab['PEERS']})
 for actor in actors.values():
  assert sha(actor['compile'][0])==actor['compiler_sha256']
  assert all(sha(path)==pin for path,pin in actor['host_inputs_sha256'].items())
 timer=lab['runtime'].MATCHED.with_name('timing-driver.py');assert sha(timer)==prior['timer_sha256']
 return actors,timer

lab['acquire'].__globals__.update(BASE=BASE,construct=prepare)
output=HERE/'matched-preserved-registers'
lab['acquire'](output,['reference'],['preserved'])
raw=read(output/'comparison.json');rows=raw['observations'];roles=raw['medians'];section='source_through_link'
result={key:raw[key] for key in ('plan_sha256','medians','phases','paired_ratios')}
result.update(source_commit=BASE,compiler_sha256=base['compiler_sha256'],trials=len(rows),source_cpu_seconds={r:median(sum(row[r]['phase_cpu_seconds']['source'].values()) for row in rows) for r in roles},source_rss_bytes={r:median(row[r]['phase_peak_rss_bytes']['source'] for row in rows) for r in roles},stripped_executables={r:raw['warmup'][r]['stripped_executable'] for r in roles},paired_ms_preserved_vs_control=median((row[ROLE]['seconds'][section]-row[CONTROL]['seconds'][section])*1000 for row in rows),preserved_faster_than_control=sum(row[ROLE]['seconds'][section]<row[CONTROL]['seconds'][section] for row in rows))
(output/'summary.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
