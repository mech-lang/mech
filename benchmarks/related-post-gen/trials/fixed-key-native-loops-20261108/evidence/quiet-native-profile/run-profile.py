from pathlib import Path
import json,subprocess,time,hashlib,shutil
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');N=Path('/private/tmp/mech-related-fixed-key-native-loops-runners-20261108-v1/histogram-reuse');RT=N.parent/'runtime/build.json'
shutil.copyfile('/private/tmp/mech-related-private-write-first-runners-20261108-v1/histogram-reuse/runner-native-profile.rs',N/'runner-native-profile.rs')
def run(c,n):
 c=list(map(str,c));t=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);(P/(n+'.json')).write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');print(n,x.returncode,flush=True);assert x.returncode==0,x.stderr;return x
run(['python3',R/'build-host.py','--compiler-build',RT,'--source',N/'runner-native-profile.rs','--out',N/'runner-native-profile','--bundle',N/'build','--linked'],'profile-host-build')
run(['python3',R/'audit-native-link.py','--object',N/'build/object.o','--executable',N/'runner-native-profile','--output',P/'native-profile-link-proof.json'],'native-profile-audit-run')
snapshot=subprocess.check_output(['ps','-axo','pid,comm'],text=True)
busy=[line for line in snapshot.splitlines()[1:]if any(line.rstrip().endswith('/'+name)for name in ['rustc','cargo','sample','gzip','gpu-tests','engine-tests'])]
assert not busy,busy
(P/'quiet-before-sample.json').write_text(json.dumps(dict(quiet=True,process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest()),indent=2)+'\n')
c=list(map(str,[N/'runner-native-profile',N/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',P/'native-profile-output.json','1']));t=time.monotonic()
with(P/'native-profile.stdout.txt').open('w')as o,(P/'native-profile.stderr.txt').open('w')as e:
 proc=subprocess.Popen(c,stdout=o,stderr=e);sc=['/usr/bin/sample',str(proc.pid),'10','1','-file',str(P/'native-profile.sample.txt')];sampler=subprocess.run(sc,capture_output=True,text=True);exit=proc.wait()
assert exit==sampler.returncode==0
assert json.loads((P/'native-profile-output.json').read_text())==json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
(P/'native-profile.json').write_text(json.dumps(dict(command=c,exit=exit,seconds=time.monotonic()-t,sampler_command=sc,sampler_exit=sampler.returncode,sampler_stdout=sampler.stdout,sampler_stderr=sampler.stderr,same_Object_audit_passed=True,repeated_native_invocations=10001,exact_final5000_output=True,qualification_times_ineligible=True,scope='Repeated IR307 evaluate_into with fresh invocations/loans/workers/cleanup and immutable admitted inputs. Repeated admission/export/materialization excluded. Inclusive overlapping stacks are diagnostic, not full processing timings or additive savings.',output_sha256=hashlib.sha256((P/'native-profile-output.json').read_bytes()).hexdigest()),indent=2)+'\n');print('fresh IR307 same-Object profile exact final5000 output',flush=True)
