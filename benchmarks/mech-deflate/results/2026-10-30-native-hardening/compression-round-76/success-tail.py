"""Reconstruct two selected actual successful tails; no binary execution or dynamic attribution."""
from pathlib import Path
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);variants={}
for label,spans in [('before',[(0x1cc,0x1d4),(0x1fc,0x22c)]),('after',[(0x1b0,0x1d4)])]:
 audit=json.loads((r/'host-assembly'/label/'audit.json').read_text());binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());assert audit['worker_sha256']==binding['worker_sha256']
 region=[x for x in audit['regions'] if x['needle']=='ordinary-budget'];assert len(region)==1;region=region[0]
 instructions=[x for x in region['instructions'] if any(lo<=x['address']-region['start']<=hi for lo,hi in spans)]
 assert len(instructions)==(16 if label=='before' else 10)
 first=region['instructions'][0];assert first['opcode']=='sub' and first['operands'].startswith('sp, sp, #0x');stack=int(first['operands'].split('#')[1],16)
 variants[label]={'worker_sha256':audit['worker_sha256'],'compiler_source':binding['compiler_source'],'symbol':region['symbol'],'whole_function_own_static_instructions':region['static_instruction_count'],'own_stack_frame_bytes':stack,'selected_relative_ranges':spans,'success_tail_static_instructions':len(instructions),'instructions':instructions}
assert variants['before']['own_stack_frame_bytes']==128 and variants['after']['own_stack_frame_bytes']==176
result={'variants':variants,'saved_selected_tail_instructions':6,'stack_frame_delta_bytes':48,'full_inactive_result_payload_still_transferred_on_success':True,'public_result_bytes':40,'scope':'Exact selected commit/borrow-release/result-write tails in the actual accepted/candidate linked ordinary debit. The before tail follows its unconditional branch between the two ranges; the after tail is contiguous. Both are reached on successful no-worker debit. The whole function grows158→183 own static instructions; cold forwarding is5 own instructions. No whole-function dynamic instruction ledger or complete causal attribution follows. Four-call diagnostic attempts and whole-call hardware counters remain separate evidence.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='variants'},indent=2))
