from pathlib import Path
import hashlib,json,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-opt-20261106-v1')
roots={231:Path('/private/tmp/mech-related-native-reduction-terminal-transition-runners-20261106-v1/after-4/build'),232:Path('/private/tmp/mech-related-native-boolean-literal-branches-runners-20261106-v2/after-4/build'),233:Path('/private/tmp/mech-related-native-boolean-branch-width-runners-20261106-v1/after-4/build')}
summary=[]
for ir,source in roots.items():
 metadata=json.loads((source/'metadata.json').read_text());assert (metadata['typed_ir_version'],metadata['work_policy_version'],metadata['abi_version'])==(ir,115,41)
 p=P/str(ir);p.mkdir();(p/'build').symlink_to(source,target_is_directory=True)
 (p/'probe.rs').write_text((P/'probe.rs').read_text().replace('"old_typed_ir":197','"old_typed_ir":'+str(ir)))
 for name,cmd in [('build',['python3',str(R/'build-host.py'),'--compiler-build',str(O/'build.json'),'--source',str(p/'probe.rs'),'--out',str(p/'probe'),'--bundle',str(p/'build'),'--linked']),('run',[str(p/'probe')])]:
  start=time.monotonic();result=subprocess.run(cmd,capture_output=True,text=True);receipt=dict(command=cmd,exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr)
  (p/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n');assert result.returncode==0,result.stderr
 row=json.loads(receipt['stdout']);assert row['old_entry_never_invoked'] and row['object_bound_rejected'] and row['compiled_module_rejected'];summary.append(row)
 print(ir,'genuine same-ABI/work-policy Object rejected by both loaders',flush=True)
(P/'summary.json').write_text(json.dumps(dict(status='passed',fresh_old_objects=len(summary),rows=summary,older26_checks_retained_separately_not_counted_as_fresh=True),indent=2)+'\n')
