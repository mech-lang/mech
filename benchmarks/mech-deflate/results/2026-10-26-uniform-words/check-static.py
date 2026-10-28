import subprocess,json,sys,time
commands=[['cargo','+nightly-2026-03-03','fmt','--all','--','--check'],['git','diff','--check']]+[['python3',f'scripts/{name}.py'] for name in ['check-warning-policy','check-rust-module-layout','check-native-build-analysis-layout','check-value-execution-boundary','check-program-artifact-contract','check-type-memory-boundary','check-interactive-architecture','check-production-resident-routing','check-compiler-planning-quarantine']]+[['scripts/check-unsafe-boundaries.sh']]
results=[]
for command in commands:
    start=time.monotonic();r=subprocess.run(command)
    d={'command':command,'exit':r.returncode,'seconds':time.monotonic()-start}
    results.append(d);print(json.dumps(d),flush=True)
print(json.dumps({'checks':results}),flush=True)
sys.exit(any(row['exit'] for row in results))
