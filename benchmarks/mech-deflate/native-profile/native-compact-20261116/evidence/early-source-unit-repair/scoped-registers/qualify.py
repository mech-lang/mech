"""Bind and check actual ordered-reduction workers against the qualified 63,692 B worker."""
from pathlib import Path
from runpy import run_path
import inspect,json,sys
root=Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995/controller-source-boundaries/scoped-registers');lab=root.parent.parent
h=run_path(str(lab.parent/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='natural4k_ordered_gate')
rt=h['runtime'];rt.HERE=root;rt.UNCHANGED=lab/'collector-read-lifetimes/placement/immutable-profiled/bound';rt.BASE='249a4c7e0d15079f59a7d65aee7a034775332a4d plus source-boundary/owner-packet source snapshots'
rt.__dict__.update(HERE=root,UNCHANGED=rt.UNCHANGED,BASE=rt.BASE)
rt.timing.prepare.__globals__['control']=rt.UNCHANGED
source=inspect.getsource(rt.measure)
source=source.replace('Measure the exact checked worker after keeping shared cold returns in the caller.','Measure opt-in pre-SSA source units with caller-owned storage and root-scoped register preference. No runtime wins are inherited.')
source=source.replace("prefix = f'shared-terminal-{role}-{kind}'","prefix = f'early-unit-repair-{role}-{kind}'")
rt.__dict__['__file__']=str(Path(__file__).resolve())
exec(compile(source,str(Path(__file__).resolve())+':measure','exec'),rt.__dict__)
source=inspect.getsource(rt.check).replace("'linked-' + '-'.join(roles)","'linked-current-' + '-'.join(roles)").replace("f'shared-terminal-","f'early-unit-repair-")
exec(compile(source,str(Path(__file__).resolve())+':check','exec'),rt.__dict__)
# The original temporary C source is gone. Reuse the exact previously qualified
# stack probe object, while probing every newly linked worker as usual.
probe=lab/'preserved-registers/linked-current-preserved/linked-proofs/stack-support/stack.o'
probe_sha='47629aa045ac454421a6f8412b9a50fd52eebc76cb4a19191aee9f4c6d7aec22'
assert rt.sha(probe)==probe_sha
old_compile="subprocess.run(['cc','-std=gnu11','-O2','-Wall','-Wextra','-Werror','-c',\n                q/'controller-stack-probe.c','-o',c_object],check=True)"
new_compile="__import__('shutil').copyfile("+repr(str(probe))+",c_object)\nassert __import__('hashlib').sha256(c_object.read_bytes()).hexdigest()=="+repr(probe_sha)
old_copy="    original = (q / name).read_text()"
new_copy=old_copy+"\n    if name=='check-layout-stack.py':\n        old="+repr(old_compile)+"\n        assert original.count(old)==1\n        original=original.replace(old,"+repr(new_compile)+")"
source=inspect.getsource(rt.driver.link.check_contracts)
needle_driver="    driver = (Q / 'backend-work-hardened/verify-linked.py').read_text()"
assert source.count(needle_driver)==1
source=source.replace(needle_driver,needle_driver+"\n    assert driver.count("+repr(old_copy)+")==1\n    driver=driver.replace("+repr(old_copy)+","+repr(new_copy)+")")
exec(compile(source,str(Path(__file__).resolve())+':check_contracts', 'exec'),rt.driver.link.check_contracts.__globals__)
# Add the two large-random contracts that motivated this experiment.
rt.timing.focus=[*rt.timing.focus,'prefix-random-262144','prefix-random-1048576']
action,*roles=sys.argv[1:]
if action=='check':
 for role in roles:
  r=json.loads((root/role/'receipt.json').read_text())
  if not (root/role/'bound/runtime.json').exists():rt.driver.link.link_worker(rt.UNCHANGED,root/role/'frontend',root/role/'bound')
  pin=json.loads((root/role/'bound/runtime.json').read_text())
  assert pin['object_sha256']==r['object_sha256']
  assert rt.driver.link.sha(root/role/'bound/mech-worker')==pin['worker_sha256']
 rt.check(*roles)
 for role in roles:
  proof=json.loads((root/role/'linked-proof.json').read_text());assert proof['peak_stack_bytes']<=5280
elif action in ('focus','full'):
 for role in roles:rt.measure(role,action)
else:raise ValueError(action)
