"""Use the unchanged v3 linked/runtime gates for the preserved-register trial."""
from pathlib import Path
from runpy import run_path
import inspect,json,sys
lab=Path(__file__).resolve().parent
root=lab/'preserved-registers'
h=run_path(str(lab.parent/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='preserved_register_gate')
rt=h['runtime'];rt.HERE=root;rt.UNCHANGED=lab/'literal-frame-cleanup/bound'
receipt_path=root/'preserved/receipt.json';r=json.loads(receipt_path.read_text())
rt.BASE=r['source_commit']
rt.check.__globals__.update(HERE=root,UNCHANGED=rt.UNCHANGED,BASE=rt.BASE)
rt.timing.prepare.__globals__['control']=rt.UNCHANGED
base=json.loads((root/'control-source/rebuild.json').read_text())
compiler=Path(r['command'][0])
compiler.with_name('result.json').write_text(json.dumps({'source_commit':r['source_commit'],'compiler_sha256':r['compiler_sha256'],'compiler_build':base['compiler_build'],'build_scope':'Warm-cache source compiler construction; not a clean compiler-build timing.'},indent=2)+'\n')
r['compile']=r['command'];r['source_pin']=r['source_commit'];r['layout']={'object_sha256':r['object_sha256'],'text_bytes':r['text_bytes']}
receipt_path.write_text(json.dumps(r,indent=2)+'\n')
# Update only the inherited experiment description; keep all v3 policy and acquisition code.
source=inspect.getsource(rt.measure)
old='Measure the exact checked worker after keeping shared cold returns in the caller.'
assert source.count(old)==1
source=source.replace(old,'Measure the checked root with preserved integer register preferences; hold 81 Roc and 62 C point wins.')
old_prefix="prefix = f'shared-terminal-{role}-{kind}'"
assert source.count(old_prefix)==1
source=source.replace(old_prefix,"prefix = f'preserved-register-root-{role}-{kind}'")
rt.__dict__['__file__']=str(Path(__file__).resolve())
exec(compile(source,str(Path(__file__).resolve())+':measure','exec'),rt.__dict__)
check_source=inspect.getsource(rt.check)
assert check_source.count("'linked-' + '-'.join(roles)")==1
check_source=check_source.replace("'linked-' + '-'.join(roles)","'linked-current-' + '-'.join(roles)")
assert check_source.count("f'shared-terminal-")==1
check_source=check_source.replace("f'shared-terminal-","f'preserved-register-root-")
exec(compile(check_source,str(Path(__file__).resolve())+':check','exec'),rt.__dict__)
action=sys.argv[1]
if action=='check':
 if not (root/'preserved/bound/runtime.json').exists():
  rt.driver.link.link_worker(rt.UNCHANGED,root/'preserved/frontend',root/'preserved/bound')
 pin=json.loads((root/'preserved/bound/runtime.json').read_text())
 assert pin['object_sha256']==r['object_sha256']
 assert rt.driver.link.sha(root/'preserved/bound/mech-worker')==pin['worker_sha256']
 rt.check('preserved')
elif action in ('focus','full'):
 rt.measure('preserved',action)
else:raise ValueError(action)
