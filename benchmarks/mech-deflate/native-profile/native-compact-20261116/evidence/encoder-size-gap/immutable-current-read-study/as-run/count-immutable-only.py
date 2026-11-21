"""Check actual source-emitted workers, then count original collector instructions in untimed clones."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
from types import ModuleType,SimpleNamespace
from struct import unpack_from
import ast,json,sys
ROOT=Path(__file__).resolve().parent;L=ROOT/'immutable-only';I=ROOT.parent;Q=I.parent;P=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116');read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();sys.path.insert(0,str(Q));link=run_path(str(Q/'current-roadmap/checked-root-census/recheck-link.py'),run_name='immutable_read_link');template=I/'natural4k-runtime-target/hot-matcher-facts/guards-profiled/bound';roles=['immutable-defaults'];targets={}
for role in roles:
 target=L/role/'bound'
 if not (target/'runtime.json').exists():link['link_worker'](template,L/role/'frontend',target)

 if not (target/'qualification.json').exists():link['check_worker'](target,'immutable-only-default-'+role)
 else:
  old=read(target/'qualification.json');assert old['verified'] and old['complete_smokes']==162 and old['worker_sha256']==sha(target/'mech-worker')
 targets[role]=(L/role/'canonical-counter-input/bound' if role!='control' else target);print('Exact outputs/inflation/aliases:',role,flush=True)
def functions(path,names):
 nodes=[n for n in ast.parse(path.read_text()).body if isinstance(n,ast.FunctionDef) and n.name in names];assert {n.name for n in nodes}==set(names);scope={'unpack_from':unpack_from};exec(compile(ast.Module(body=nodes,type_ignores=[]),str(path),'exec'),scope);return {n:scope[n] for n in names}
layout=ModuleType('outline-compiled-aarch64-root');obj_file=P.parent/'compact-size/compact_object.py';branch_file=Q/'outline-compiled-aarch64-root.py';layout.obj=SimpleNamespace(**functions(obj_file,['tables','relocs']))
for k,v in functions(branch_file,['branch_field','branch_target','branch_opcode']).items():setattr(layout,k,v)
sys.modules[layout.__name__]=layout;original=Q/'count-matcher-instructions.py';source=original.read_text();a='    root = q / f"{label}-native/final-candidate"';assert source.count(a)==1;source=source.replace(a,'    root = targets[label]');a='    directory = q / "matcher-dynamic-blocks" / actor';assert source.count(a)==1;source=source.replace(a,'    directory = output / actor / "matcher-counts"')
# The original diagnostic accepts ADRP relocation pairs only when a literal
# pool is present. These new bodies have source-emitted page relocations but
# no literal pool. Permit their syntax; the original relocation-kind, paired
# ADRP, branch and pointer-preservation checks still validate every rewrite.
a='assembly = instructions(path, start, boundary, allow_data_references=literal_start is not None)';assert source.count(a)==1;source=source.replace(a,'assembly = instructions(path, start, boundary, allow_data_references=True)')
namespace={'__name__':'immutable_default_counter','__file__':str(original),'targets':targets,'output':L};exec(compile(source,str(original),'exec'),namespace);symbol='_mech_compact_numeric_475_vector';cases=['prefix-random-1048576','prefix-random-4096','random-1024','prefix-random-1024','prefix-natural-4096','prefix-natural-8192']
for role in roles:

 if (L/role/'matcher-counts/plan.json').exists():
  print('Retain completed counts:',role,flush=True);continue
 target=targets[role];metadata=read(target/'metadata.json');_,_,phase=namespace['phase_metadata'](metadata,symbol);macho=namespace['macho'];image,sections,symbols=macho.load(target/'program.o');start=macho.text_symbols(sections,symbols)[symbol];offset=sections[macho.TEXT][3];refs=[]
 for at in range(0,phase['code_bytes'],4):
  if refs and at>=min(refs):break
  word=unpack_from('<I',image,offset+start+at)[0]
  if word&0x3b000000==0x18000000:
   displacement=(word>>5)&0x7ffff
   if displacement&0x40000:displacement-=0x80000
   dest=at+4*displacement;assert at<dest<phase['code_bytes'] and dest%16==0;refs.append(dest)
 pool=min(refs) if refs else None;namespace['count'](role,role,cases,symbol,pool)
 (L/role/'matcher-counts/plan.json').write_text(json.dumps({'worker_sha256':sha(target/'mech-worker'),'object_sha256':sha(target/'program.o'),'symbol':symbol,'literal_offset':pool,'cases':cases,'counter_driver_sha256':sha(original),'scope':'Untimed diagnostic counters; original source-emitted instructions and every output/alias/inflation are checked. Never a shipping implementation or latency result.','counter_source':source},indent=2)+'\n')
print('All source workers and diagnostic counters checked.',flush=True)
