"""Repeat the original affected order exactly; retain every adverse original block."""
from pathlib import Path
from hashlib import sha256
import ast,json,os,subprocess,sys
N=Path(__file__).resolve().parent;Q=N.parent.parent;timer=Q/'compare-library.py';role='word-reuse-complements-profiled';bound=N/role/'bound';control=N.parent/'collector-read-lifetimes/placement/immutable-profiled/bound'
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();read=lambda p:json.loads(p.read_text())
assert sha(bound/'mech-worker')=='3625b0427631d818f3a6731202e75eca70f3ab0551b9ef74e4fbae802d2fb6e5'
tree=ast.parse((N/'probe-vector-mismatch-clean.py').read_text());cases=next(ast.literal_eval(node.value) for node in ast.walk(tree) if isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='cases' for t in node.targets));assert len(cases)==19
out=N/'review-word-reuse-reverse-repeat';out.mkdir(exist_ok=True)
plan={'candidate_worker_sha256':sha(bound/'mech-worker'),'control_worker_sha256':sha(control/'mech-worker'),'timer_sha256':sha(timer),'source_driver_sha256':sha(Path(__file__)),'cases':cases,'order':'reverse','blocks_per_case':24,'adverse_original_diagnosis':read(N/'review-word-reuse-regime-change.json'),'scope':'Repeat original worker and complete affected 19-case reverse order, all 24 blocks retained in original and repeat. This secondary candidate differs from word-current/complement.'}
(out/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
if '--run' not in sys.argv:print('Prepared',out/'plan.json');raise SystemExit
def environment():
 return {'load':os.getloadavg(),'power':subprocess.run(['pmset','-g','batt'],capture_output=True,text=True).stdout,'thermal':subprocess.run(['pmset','-g','therm'],capture_output=True,text=True).stdout}
(out/'environment-before.json').write_text(json.dumps(environment(),indent=2)+'\n')
source=timer.read_text();needle="selected = {case['name'] for case in cases}";assert source.count(needle)==1;source=source.replace(needle,'selected = '+repr(set(cases)));(out/'timer-as-run.py').write_text(source)
label='review-word-reuse-reverse-repeat-20261119';sys.argv=[str(timer),label,str(control),str(bound),'reverse'];exec(compile(source,str(out/'timer-as-run.py'),'exec'),{'__file__':str(timer),'__name__':'__main__'})
(out/'environment-after.json').write_text(json.dumps(environment(),indent=2)+'\n')
(out/'result.json').write_text(json.dumps({'plan':plan,'folder':str(Q/label),'summary_sha256':sha(Q/label/'summary.json'),'results':read(Q/label/'summary.json'),'complete':True},indent=2)+'\n')
