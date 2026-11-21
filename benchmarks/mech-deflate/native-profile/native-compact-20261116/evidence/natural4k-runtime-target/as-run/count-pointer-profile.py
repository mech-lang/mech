"""Untimed diagnostic block counters; shipping objects come only from source compilation."""
from pathlib import Path
from runpy import run_path
from hashlib import sha256
from types import ModuleType,SimpleNamespace
import ast,json,struct,sys
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;Q=LAB.parent
PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
sys.path.insert(0,str(Q));sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
# Reuse only the original parser/relocation helpers. Importing the historical
# outlining script as a whole would open unrelated, now-missing temp artifacts.
obj_file=PACKET.parent/'compact-size/compact_object.py';branch_file=Q/'outline-compiled-aarch64-root.py'
def functions(path,names):
 tree=ast.parse(path.read_text());nodes=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names];assert {n.name for n in nodes}==set(names)
 scope={'unpack_from':struct.unpack_from};exec(compile(ast.Module(body=nodes,type_ignores=[]),str(path),'exec'),scope);return {n:scope[n] for n in names}
layout=ModuleType('outline-compiled-aarch64-root');layout.obj=SimpleNamespace(**functions(obj_file,['tables','relocs']))
for key,value in functions(branch_file,['branch_field','branch_target','branch_opcode']).items():setattr(layout,key,value)
sys.modules[layout.__name__]=layout
h=run_path(str(Q/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='natural4k_counter_tools');rt=h['runtime'];study=rt.study;driver=rt.driver
study.HERE=ROOT;study.rebase.HERE=ROOT
training=('prefix-natural-4096','prefix-natural-8192','reymont-prefix-32769','prefix-natural-32769')
roles=sys.argv[1:] or ['pointer-default-layout','pointer-lanes-default-layout']
for role in roles:
 target=ROOT/role/'bound'
 if not (target/'runtime.json').exists():driver.link.link_worker(LAB/'preserved-registers/preserved/bound',ROOT/role/'frontend',target)
 ref=study.rebase.parse(role);folder=ROOT/role/'instruction-counts'
 original=Q/'count-matcher-instructions.py';program=original.read_text();needle='    root = q / f"{label}-native/final-candidate"';assert program.count(needle)==1;program=program.replace(needle,f'    root = Path({str(target)!r})')
 needle='    directory = q / "matcher-dynamic-blocks" / actor';assert program.count(needle)==1;program=program.replace(needle,f'    directory = Path({str(folder)!r})')
 headers={'__name__':'natural4k_source_body_counters','__file__':str(original)};exec(compile(program,str(original),'exec'),headers)
 headers['count'](role,role,training,driver.SCORER,ref['pool'])
 record={'untimed':True,'shipping_encoder_sha256':sha(target/'program.o'),'counter_driver_sha256':sha(original),'branch_helpers':{'source':str(branch_file),'sha256':sha(branch_file)},'object_helpers':{'source':str(obj_file),'sha256':sha(obj_file)},'training_cases':training,'counter_source':program,'scope':'Instrumentation adds counters only to a diagnostic clone. Never use it for latency or deployment.'}
 (folder/'plan.json').write_text(json.dumps(record,indent=2)+'\n')
 profile=study.profiles(role);p=json.loads(profile.read_text());p['thread_jumps']=True;profile.write_text(json.dumps(p,indent=2)+'\n')
 print(role,'profile',profile,flush=True)
