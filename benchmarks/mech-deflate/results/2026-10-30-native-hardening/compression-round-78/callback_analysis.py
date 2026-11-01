"""Replay bounded callback code and exact diagnostic event counts."""
from pathlib import Path
import json,re,sys,hashlib
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'variants':{}}
for label in ['before','after']:
 root=r/'callback-assembly'/label;receipt=json.loads((root/'audit.json').read_text());regions={}
 for row in receipt['regions']:
  text=(root/(row['kind']+'.asm')).read_text();instructions=[]
  for line in text.splitlines():
   m=re.match(r'^\s*([0-9a-fA-F]+):\s+([a-z][a-z0-9.]*)\s*(.*)$',line)
   if m and row['start']<=int(m[1],16)<row['stop']:instructions.append({'relative_address':int(m[1],16)-row['start'],'opcode':m[2],'operands':m[3]})
  assert instructions==row['instructions'];assert len(instructions)==row['own_static_instructions'];first=instructions[0];assert first['opcode']=='sub';m=re.fullmatch(r'sp, sp, #(0x[0-9a-f]+|\d+)',first['operands']);assert m
  prefix=[]
  for inst in instructions:
   if inst['opcode'] in ['cbz','cbnz','b','b.eq','b.ne','ret']:break
   prefix.append(inst)
  returns=[i for i,x in enumerate(instructions) if x['opcode']=='ret'];assert len(returns)==1
  restored=0
  for inst in reversed(instructions[:returns[0]]):
   if inst['opcode']=='ldp' and '[sp' in inst['operands']:restored+=1
   elif inst['opcode']=='add' and inst['operands'].startswith('sp, sp, '):continue
   else:break
  regions[row['kind']]={'own_static_instructions':len(instructions),'own_frame_bytes':int(m[1],0),'entry_stack_pair_stores':sum(x['opcode']=='stp' and '[sp' in x['operands'] for x in prefix),'exit_stack_pair_loads':restored,'assembly_sha256':hashlib.sha256(text.encode()).hexdigest()}
 result['variants'][label]=regions
counts=json.loads((r/'diagnostic-counts/summary.json').read_text())['consistent_per_call_counts'];fused=counts['helper_AdmitMove']+counts['helper_AdmitStructuralObservation'];assert counts['complete_logical_admission_attempts']==counts['admit_callback_attempts']+fused
result['dynamic']={'fused_moves':counts['helper_AdmitMove'],'fused_structural':counts['helper_AdmitStructuralObservation'],'fused_total':fused,'original_logical_admissions':counts['complete_logical_admission_attempts'],'remaining_admission_callbacks':counts['admit_callback_attempts'],'helper_callbacks':counts['helper_callback_attempts'],'original_split_callback_total':counts['complete_logical_admission_attempts']+counts['helper_callback_attempts'],'current_callback_total':counts['admit_callback_attempts']+counts['helper_callback_attempts'],'shared_compute_debits':counts['shared_compute_debit_attempts']}
result['dynamic']['callback_reduction_percent']=100*fused/result['dynamic']['original_split_callback_total']
b=result['variants']['before']['helper'];a=result['variants']['after']['helper'];result['helper_common_entry_extra_pair_store_instructions']=counts['helper_callback_attempts']*(a['entry_stack_pair_stores']-b['entry_stack_pair_stores'])
extra=counts['helper_callback_attempts']*(a['entry_stack_pair_stores']+a['exit_stack_pair_loads']-b['entry_stack_pair_stores']-b['exit_stack_pair_loads']);admit=result['variants']['before']['admit'];saved=fused*(admit['entry_stack_pair_stores']+admit['exit_stack_pair_loads']);result['common_callback_stack_pair_instructions']={'added_helper_saves_restores':extra,'removed_admission_saves_restores':saved,'net_added':extra-saved,'scope':'Verified common entry and single common exit only, multiplied by exact successful callback counts. Other emitted instructions and counter noise are not attributed.'}
result['scope']='Own static routine sizes and frames from actual bounded machine-code disassembly; dynamic counters from four identical successful diagnostic calls. The extra pair-store count covers the unconditional helper entry prologue only; it is not net whole-call instruction attribution, omits removed admission prologues and other offsets, and is not a speed forecast. Original split callback total is reconstructed from current logical visits and checked against the retained baseline protocol, not a newly rerun baseline census.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
