"""Count current collector instructions on representative losses; never use this clone for latency."""
from pathlib import Path
from runpy import run_path
from hashlib import sha256
from types import ModuleType,SimpleNamespace
from struct import unpack_from
import ast,json,sys
ROOT=Path(__file__).resolve().parent;Q=ROOT.parent.parent;OUT=ROOT/'history-review';PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
sys.path.insert(0,str(Q));sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
def functions(path,names):
 tree=ast.parse(path.read_text());nodes=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names];assert {n.name for n in nodes}==set(names)
 scope={'unpack_from':unpack_from};exec(compile(ast.Module(body=nodes,type_ignores=[]),str(path),'exec'),scope);return {n:scope[n] for n in names}
layout=ModuleType('outline-compiled-aarch64-root');obj_file=PACKET.parent/'compact-size/compact_object.py';branch_file=Q/'outline-compiled-aarch64-root.py';layout.obj=SimpleNamespace(**functions(obj_file,['tables','relocs']))
for k,v in functions(branch_file,['branch_field','branch_target','branch_opcode']).items():setattr(layout,k,v)
sys.modules[layout.__name__]=layout
original=Q/'count-matcher-instructions.py';source=original.read_text();target=ROOT.parent/'preserved-registers/preserved/bound';folder=OUT/'matcher-counts'
a='    root = q / f"{label}-native/final-candidate"';assert source.count(a)==1;source=source.replace(a,f'    root = Path({str(target)!r})')
a='    directory = q / "matcher-dynamic-blocks" / actor';assert source.count(a)==1;source=source.replace(a,f'    directory = Path({str(folder)!r})')
namespace={'__name__':'current_matcher_counter','__file__':str(original)};exec(compile(source,str(original),'exec'),namespace)
metadata=json.loads((target/'metadata.json').read_text());_,_,phase=namespace['phase_metadata'](metadata,'_mech_compact_numeric_475_vector')
macho=namespace['macho'];image,sections,symbols=macho.load(target/'program.o');start=macho.text_symbols(sections,symbols)['_mech_compact_numeric_475_vector'];offset=sections[macho.TEXT][3];refs=[]
for at in range(0,phase['code_bytes'],4):
 if refs and at>=min(refs):break
 word=unpack_from('<I',image,offset+start+at)[0]
 if word&0x3b000000==0x18000000:
  displacement=(word>>5)&0x7ffff
  if displacement&0x40000:displacement-=0x80000
  dest=at+4*displacement;assert at<dest<phase['code_bytes'] and dest%16==0;refs.append(dest)
pool=min(refs) if refs else None
cases=['prefix-random-1048576','prefix-random-4096','prefix-natural-4096']
namespace['count']('selected-current','selected-current',cases,'_mech_compact_numeric_475_vector',pool)
plan={'worker_sha256':sha(target/'mech-worker'),'object_sha256':sha(target/'program.o'),'symbol':'_mech_compact_numeric_475_vector','literal_offset':pool,'cases':cases,'counter_source':source,'counter_driver_sha256':sha(original),'object_helpers':{'path':str(obj_file),'sha256':sha(obj_file)},'branch_helpers':{'path':str(branch_file),'sha256':sha(branch_file)},'scope':'Untimed diagnostic clone of the exact current collector. Counters preserve flags/registers; the original normal worker supplies runtime ratios.'}
(folder/'plan.json').write_text(json.dumps(plan,indent=2)+'\n')
print('Current matcher counts complete.',flush=True)
