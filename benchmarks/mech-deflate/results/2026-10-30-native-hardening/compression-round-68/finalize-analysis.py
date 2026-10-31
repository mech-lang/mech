"""After all cohorts: derive the review packet. Archive only with explicit --archive."""
from pathlib import Path
import json,subprocess,sys
r=Path(__file__).resolve().parent
assert json.loads((r/'decision.json').read_text())['status'] in ['retained','rejected'],'An observed retain/reject decision is required'
assert (r/'prior-failures.json').exists(),'Preserve and classify this round\'s failed setup attempts before closure'
for name,args in [('binary_identity.py',[str(r),str(r/'binary-audit.json')]),('profile_analysis.py',[str(r),str(r/'profile-numbers.json')]),('allocation_analysis.py',[str(r),str(r/'allocation-comparison.json')]),('mechanism_analysis.py',[str(r),str(r/'allocation-mechanism.json')]),('host_analysis.py',[str(r),str(r/'host-assembly-comparison.json')]),('instruction_analysis.py',[str(r),str(r/'instruction-window-comparison.json')]),('validation.py',[str(r),str(r/'validation.json')]),('analyze.py',[str(r),str(r/'analysis')]),('render-report.py',[str(r/'analysis/scorecard.json'),str(r/'report.md')])]:
 print('ANALYZE',name,flush=True);subprocess.run([sys.executable,str(r/name),*args],check=True)
 if name=='validation.py':assert json.loads((r/'validation.json').read_text())['all_required_checks_pass'],'Required qualification is incomplete or failed'
files=[r/name for name in ['analyze.py', 'allocation_analysis.py', 'binary_identity.py', 'validation.py', 'render-report.py', 'reconstruct.py', 'profile_analysis.py', 'finalize-analysis.py', 'implementation.json', 'configuration.json', 'allocation-target.json', 'allocation-comparison.json', 'decision.json', 'baseline', 'binary-audit.json', 'README.md', 'profile-numbers.json', 'validation.json', 'analysis', 'report.md', 'findings-packet', 'source-scope.json', 'source.patch', 'production.patch', 'prior-failures.json', 'engine-focused.py', 'core-focused.py', 'ci.py', 'initialize-baseline.py', 'qualify-all.py', 'run-cohorts.py', 'run-remaining.py', 'mechanism_analysis.py', 'host_analysis.py', 'allocation-mechanism.json', 'host-assembly-comparison.json', 'host-assembly.py', 'host-assembly', 'source-contracts.py', 'source-contracts-focused', 'source-contracts-focused-initial', 'type-architecture', 'constructor-ci.py', 'constructor-ci.json', 'allocation-test.py', 'instruction_analysis.py', 'instruction-windows.py', 'instruction-window-comparison.json', 'restoration-check.py', 'restoration-scope.json', 'restoration', 'retained-build-commands.json', 'build-changed.py', 'frozen-inputs.json']]
files += [r/'linked-primary/before/binding.json',r/'linked-primary/after/binding.json']
files += [r/'baseline-contracts'/name for name in ['results.json','memory-planner.stdout','memory-planner.stderr','memory-runtime.stdout','memory-runtime.stderr']]
for name in ['architecture-initial','engine-parent-initial','engine-after-initial','core-focused-initial']:
 if (r/name).exists():
  # Failed attempt logs and source remain, but never archive compiler output objects.
  files += [f for f in (r/name).iterdir() if f.is_file() and f.suffix in {'.json','.stdout','.stderr','.rs'}]
if '--archive' in sys.argv:
 subprocess.run([sys.executable,str(r/'archive.py'),'analysis-final',*map(str,files)],check=True)
print('R68 packet ready. Public artifacts: analysis/scorecard.json, analysis/scorecard.csv, report.md, allocation-comparison.json, profile-numbers.json, validation.json, binary-audit.json and replay scripts. No publication occurs unless --archive is supplied.',flush=True)
