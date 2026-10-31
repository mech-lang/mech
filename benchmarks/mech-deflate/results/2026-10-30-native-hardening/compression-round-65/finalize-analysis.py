"""After all cohorts: derive the review packet. Archive only with explicit --archive."""
from pathlib import Path
import json,subprocess,sys
r=Path(__file__).resolve().parent
assert json.loads((r/'decision.json').read_text())['status'] in ['retained','rejected'],'An observed retain/reject decision is required'
assert (r/'prior-failures.json').exists(),'Preserve and classify this round\'s failed setup attempts before closure'
for name,args in [('host_analysis.py',[str(r),str(r/'host-assembly-comparison.json')]),('binary_identity.py',[str(r),str(r/'binary-audit.json')]),('profile_analysis.py',[str(r),str(r/'profile-numbers.json')]),('allocation_analysis.py',[str(r),str(r/'allocation-comparison.json')]),('validation.py',[str(r),str(r/'validation.json')]),('analyze.py',[str(r),str(r/'analysis')]),('render-report.py',[str(r/'analysis/scorecard.json'),str(r/'report.md')])]:
 print('ANALYZE',name,flush=True);subprocess.run([sys.executable,str(r/name),*args],check=True)
 if name=='validation.py':assert json.loads((r/'validation.json').read_text())['all_required_checks_pass'],'Required qualification is incomplete or failed'
files=[r/name for name in ['analyze.py','allocation_analysis.py','binary_identity.py','validation.py','render-report.py','reconstruct.py','profile_analysis.py','finalize-analysis.py','host_analysis.py','host-assembly.py','host-assembly','host-assembly-comparison.json','implementation.json','configuration.json','allocation-target.json','allocation-comparison.json','decision.json','baseline','binary-audit.json','README.md','profile-numbers.json','validation.json','analysis','report.md','findings-packet','source-scope.json','source.patch','production.patch','prior-failures.json','engine-focused.py','reuse-parent-engine.py','reuse-tests.py','ci.py','initialize-baseline.py','qualify-all.py','run-cohorts.py','run-remaining.py']]
for name in ['architecture-initial','engine-parent-initial','engine-after-initial','core-focused-initial']:
 if (r/name).exists():
  # Failed attempt logs and source remain, but never archive compiler output objects.
  files += [f for f in (r/name).iterdir() if f.is_file() and f.suffix in {'.json','.stdout','.stderr','.rs'}]
if '--archive' in sys.argv:
 subprocess.run([sys.executable,str(r/'archive.py'),'analysis-final',*map(str,files)],check=True)
print('R65 packet ready. Public artifacts: analysis/scorecard.json, analysis/scorecard.csv, report.md, allocation-comparison.json, profile-numbers.json, validation.json, binary-audit.json host-assembly-comparison.json and replay scripts. No publication occurs unless --archive is supplied.',flush=True)
