"""Resume the failed stack stage with the exact previously qualified probe object."""
from pathlib import Path
from hashlib import sha256
import json,os,shutil,subprocess,sys
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;Q=LAB.parent
folder=ROOT/'linked-current-pointer-profiled-pointer-lanes-profiled';out=folder/'linked-proofs'
roles=['maintained','pointer-profiled','pointer-lanes-profiled'];actors=[folder/'worker-checks'/r for r in roles]
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
smokes=[read(a/'smokes.json') for a in actors];assert len(smokes[0])==162 and all(x==smokes[0] for x in smokes)
for actor in actors:
 r=read(actor/'runtime.json');assert sha(actor/'program.o')==r['object_sha256'];assert sha(actor/'mech-worker')==r['worker_sha256']
failed=read(out/'checked-failures.json');assert len(failed['cases'])==6
probe=LAB/'preserved-registers/linked-current-preserved/linked-proofs/stack-support/stack.o';expected='47629aa045ac454421a6f8412b9a50fd52eebc76cb4a19191aee9f4c6d7aec22';assert sha(probe)==expected
object_file=out/'stack-support/stack.o';shutil.copyfile(probe,object_file)
source=(out/'check-layout-stack.py').read_text();old="subprocess.run(['cc','-std=gnu11','-O2','-Wall','-Wextra','-Werror','-c',\n                q/'controller-stack-probe.c','-o',c_object],check=True)";assert source.count(old)==1
source=source.replace(old,"from hashlib import sha256\nassert sha256(c_object.read_bytes()).hexdigest() == "+repr(expected))
driver=out/'check-layout-stack-reuse-probe.py';driver.write_text(source)
command=[sys.executable,'-B',str(driver),*map(str,actors)];env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')};env.update(MECH_STACK_REPORT=str(out/'stack.json'),PYTHONPATH=str(Q))
with (out/'check-layout-stack-reuse-probe.log').open('w') as log:subprocess.run(command,env=env,stdout=log,stderr=log,check=True)
depths=read(out/'stack.json');peaks={a.name:max(v for sample in depths[str(a)].values() for v in sample.values()) for a in actors}
assert all(n<=5280 for n in peaks.values())
report={'commands':{'stack':command},'peak_stack_bytes':peaks,'matches_success_observations':162,'checked_failure_cases':list(failed['cases']),'failure_report_sha256':sha(out/'checked-failures.json'),'stack_probe':{'source':str(probe),'sha256':expected,'scope':'Same prebuilt probe object used for the qualified selected-worker capture. Original temporary C source is missing; this stage resumes with identical probe bytes and validates all actual new workers.'}}
(out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
for role in roles[1:]:(ROOT/role/'linked-proof.json').write_text(json.dumps({'result':str(out/'result.json'),'sha256':sha(out/'result.json'),'peak_stack_bytes':peaks[role]},indent=2)+'\n')
print(peaks,flush=True)
