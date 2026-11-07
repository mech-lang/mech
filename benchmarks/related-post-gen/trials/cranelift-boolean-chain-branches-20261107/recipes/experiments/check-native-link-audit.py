#!/usr/bin/env python3
"""Exercise complete native-link verification and deliberate linked corruption.

Copies are data-only and never executed. Supply the actual pinned Object and
executable from either maintained Cranelift or LLVM program compilation.
"""
import argparse, importlib.util, json, pathlib, struct

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--object',type=pathlib.Path,required=True)
p.add_argument('--executable',type=pathlib.Path,required=True)
p.add_argument('--out',type=pathlib.Path,required=True)
a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
spec=importlib.util.spec_from_file_location('native_audit',pathlib.Path(__file__).resolve().parents[1]/'audit-native-link.py')
audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
proof=audit.audit(a.object,a.executable)
data,sections,symbols=audit.macho(a.executable);lookup={s['name']:s for s in symbols}
def position(address):
 s=next(s for s in sections if s['address']<=address<s['address']+s['size'])
 return s['offset']+address-s['address']
text=proof['linked_text_file_offset'];changes=[('unrelocated_opcode',text,1)]
page=next((r for r in proof['relocations']if r.get('relocation_kind')==3),None)
low=next((r for r in proof['relocations']if r.get('relocation_kind')==4),None)
branch=next((r for r in proof['relocations']if 'resolved_target'in r and 'paired_offset'not in r and 'relocation_kind'not in r),None)
if page:changes.append(('wrong_page',text+page['offset'],1<<23))
if low:changes.append(('wrong_page_offset',text+low['offset'],0xfff<<10))
if branch:changes.append(('wrong_call_target',text+branch['offset'],1<<20))
identity=next(s for s in symbols if s['name'].startswith('_mech_canonical_program_identity_v1_')and not s['name'].endswith(('_compiled_module_v1','_descriptor_binding_v1','_native_metadata_v1','_native_metadata_binding_v1')))
changes.append(('identity_byte',position(lookup[identity['name']]['value']),1))
native=identity['name']+'_native_metadata_v1'
if native in lookup:changes.append(('metadata_byte',position(lookup[native]['value'])+32,1))
module=identity['name']+'_compiled_module_v1'
if module in lookup:changes.append(('module_pointer',position(lookup[module]['value'])+32,8))
rows=[]
for name,offset,mask in changes:
 altered=bytearray(data);word=struct.unpack_from('<I',altered,offset)[0];struct.pack_into('<I',altered,offset,word^mask)
 path=a.out/(name+'.data');path.write_bytes(altered)
 try:
  audit.audit(a.object,path)
 except (AssertionError,KeyError,StopIteration,struct.error) as error:
  rows.append(dict(case=name,rejected=True,reason=str(error)))
 else:raise AssertionError('corrupted linked image accepted: '+name)
 path.unlink()
summary=dict(schema='mech.native-link-audit.negative-checks.v1',positive_audit_passed=True,negative_cases=rows,all_rejected=True,executable_copies_never_executed=True)
(a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(dict(positive_audit_passed=True,negative_cases=len(rows),all_rejected=True)))
