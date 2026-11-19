"""Rebase the selected source-defined layout only when ordered control graphs are equal."""
from pathlib import Path
from runpy import run_path
import json
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent
h=run_path(str(LAB.parent/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='natural4k_rebase_tools')
rt=h['runtime'];driver=rt.driver;rebase=rt.study.rebase
rebase.HERE=ROOT
read=lambda p:json.loads(p.read_text())
roles=['control-default-scorer-layout','deferred-default-scorer-layout','encoded-default-scorer-layout']
shapes={}
for role in roles:
 p=ROOT/role/'receipt.json';r=read(p);r['layout']=driver.symbols(ROOT/role/'frontend/program.o');p.write_text(json.dumps(r,indent=2)+'\n');shapes[role]=rebase.parse(role)
reference=shapes[roles[0]]
base=read(LAB/'early-codebook-gated-default-source/rebuild.json')
profile_file=Path(next(x.split('@',1)[1] for x in base['native_flags']['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';') if x.startswith('mech_compact_numeric_3009_no_growth_composed@')))
profile=read(profile_file);assert set(profile['order'])==set(reference['heads']);assert profile['literal_offset']==reference['pool']
proof={'source_commit':'b104d014da5da9b87ef270909507e9bfe35558f8','reference_profile':str(profile_file),'reference_profile_sha256':driver.sha(profile_file),'shapes':shapes,'results':{}}
for role in roles:
 shape=shapes[role];same=shape['blocks']==reference['blocks']
 proof['results'][role]={'same_ordered_control_graph':same,'blocks':len(shape['blocks'])}
 if same:
  table=dict(zip(reference['heads'],shape['heads'],strict=True));path=ROOT/(role+'-rebased-profile.json')
  result=profile|{'order':[table[k] for k in profile['order']],'literal_offset':shape['pool']}
  path.write_text(json.dumps(result,indent=2)+'\n');proof['results'][role].update(profile=str(path),profile_sha256=driver.sha(path))
 print(role,same,len(shape['blocks']),flush=True)
(ROOT/'rebase-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
