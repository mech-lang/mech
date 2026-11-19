"""Bind and check actual ordered-reduction workers against the qualified 63,804 B worker."""
from pathlib import Path
from runpy import run_path
import inspect,json,sys
root=Path(__file__).resolve().parent;lab=root.parent
h=run_path(str(lab.parent/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='natural4k_ordered_gate')
rt=h['runtime'];rt.HERE=root;rt.UNCHANGED=lab/'preserved-registers/preserved/bound';rt.BASE='b104d014da5da9b87ef270909507e9bfe35558f8'
rt.__dict__.update(HERE=root,UNCHANGED=rt.UNCHANGED,BASE=rt.BASE)
rt.timing.prepare.__globals__['control']=rt.UNCHANGED
source=inspect.getsource(rt.measure)
source=source.replace('Measure the exact checked worker after keeping shared cold returns in the caller.','Measure existing generic deferred or offset-encoded ordered winner lowering; hold all qualified peer wins.')
source=source.replace("prefix = f'shared-terminal-{role}-{kind}'","prefix = f'natural4k-ordered-{role}-{kind}'")
rt.__dict__['__file__']=str(Path(__file__).resolve())
exec(compile(source,str(Path(__file__).resolve())+':measure','exec'),rt.__dict__)
source=inspect.getsource(rt.check).replace("'linked-' + '-'.join(roles)","'linked-current-' + '-'.join(roles)").replace("f'shared-terminal-","f'natural4k-ordered-")
exec(compile(source,str(Path(__file__).resolve())+':check','exec'),rt.__dict__)
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
