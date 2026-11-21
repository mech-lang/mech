"""One balanced five-actor cohort; a precision study, separate from full qualification."""
from pathlib import Path
from hashlib import sha256
import ast,json,os,sys,subprocess,time
N=Path(__file__).resolve().parent;Q=N.parent.parent;timer=Q/'compare-library.py'
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
source=(N/'probe-vector-mismatch-clean.py').read_text();tree=ast.parse(source)
cases=next(ast.literal_eval(node.value) for node in ast.walk(tree) if isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='cases' for t in node.targets))
assert len(cases)==19
control=N.parent/'collector-read-lifetimes/placement/immutable-profiled/bound';word=N/'word-current-complements-profiled/bound';vector=N/'vector-mismatch-narrow-profiled/bound'
folders={'mech-control':control,'mech-experiment':word,'mech-vector':vector};out=N/'finalist-precision-20261119';out.mkdir(exist_ok=True)
pins={actor:{name:{'path':str(folder/name),'sha256':sha(folder/name)} for name in ('mech-worker','metadata.json','program.o','runtime.json')} for actor,folder in folders.items()}
plan={'cases':cases,'actors':list(folders)+['roc','libdeflate'],'orders':['forward','reverse'],'blocks_per_case':120,'timer':{'path':str(timer),'sha256':sha(timer)},'source_driver_sha256':sha(Path(__file__)),'bindings':pins,'scope':'All five actors in every block; all 120 permutations per case. Both case orders. Pairwise runtime precision only; no full-score or win promotion.'}
plan_file=out/'plan.json'
if plan_file.exists():assert read(plan_file)==plan
else:plan_file.write_text(json.dumps(plan,indent=2)+'\n')
if '--run' not in sys.argv:print('Prepared',plan_file);raise SystemExit
program=timer.read_text()
patches={"ACTORS = [BASELINE, CANDIDATE, 'roc', 'libdeflate']":"VECTOR = 'mech-vector'\nACTORS = [BASELINE, CANDIDATE, VECTOR, 'roc', 'libdeflate']", "[(BASELINE,Path(before)), (CANDIDATE,Path(after))]":"[(BASELINE,Path(before)), (CANDIDATE,Path(after)), (VECTOR,Path("+repr(str(vector))+"))]", "selected = {case['name'] for case in cases}":'selected = '+repr(set(cases)), 'for actor in (BASELINE, CANDIDATE):':'for actor in (BASELINE, CANDIDATE, VECTOR):'}
for old,new in patches.items():assert program.count(old)==1,old;program=program.replace(old,new)
(out/'timer-as-run.py').write_text(program)
runs=[]
for order in plan['orders']:
 for actor,folder in folders.items():
  for name,pin in pins[actor].items():assert sha(folder/name)==pin['sha256']
 label='finalist-precision-20261119-'+order
 sys.argv=[str(timer),label,str(control),str(word)]+(['reverse'] if order=='reverse' else [])
 exec(compile(program,str(out/'timer-as-run.py'),'exec'),{'__file__':str(timer),'__name__':'__main__'})
 assert read(Q/label/'complete.json')['balanced_blocks_per_case']==120
 runs.append({'order':order,'folder':str(Q/label),'summary_sha256':sha(Q/label/'summary.json'),'results':read(Q/label/'summary.json')})
 (out/'result.json').write_text(json.dumps({'plan':plan,'runs':runs,'complete':len(runs)==2},indent=2)+'\n')
 print('Finished',order,flush=True)
