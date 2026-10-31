"""Reconstruct static host evidence from retained bounded raw disassembly."""
from pathlib import Path
from collections import Counter
import hashlib,json,re,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'variants':{}}
for label in ['before','after']:
 p=r/'host-assembly'/label;raw=(p/'audit.json').read_bytes();audit=json.loads(raw);binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());assert audit['worker_sha256']==binding['worker_sha256']
 assert audit['size_exit']==0 and all(x['exit']==0 for x in audit['regions'])
 size=(p/'host-section-sizes.txt').read_text();text=re.findall(r'^__text\s+(\d+)\s+\d+\s*$',size,re.M);assert len(text)==1
 functions=[]
 for row in audit['regions']:
  asm=p/(row['needle']+'-'+str(sum(x['needle']==row['needle'] for x in functions))+'.asm')
  instructions=[]
  for line in asm.read_text().splitlines():
   m=re.match(r'^\s*([0-9a-fA-F]+):\s+([a-z][a-z0-9.]*)\s*(.*)$',line)
   if m and row['start']<=int(m[1],16)<row['stop']:instructions.append({'address':int(m[1],16),'opcode':m[2],'operands':m[3]})
  assert instructions==row['instructions'] and len(instructions)==row['static_instruction_count']
  assert dict(sorted(Counter(x['opcode'] for x in instructions).items()))==row['opcodes']
  calls=[x for x in instructions if x['opcode'] in ['bl','blr']]
  functions.append({'needle':row['needle'],'symbol':row['symbol'],'start':row['start'],'stop':row['stop'],'static_instruction_count':len(instructions),'opcodes':row['opcodes'],'calls':calls,'outlined_debit_calls':[x for x in calls if 'charge_in_context' in x['operands']],'tls_calls':[x for x in calls if '_tlv_get_addr' in x['operands']],'stack_pointer_adjustments':[x for x in instructions if re.match(r'sp,\s*sp,',x['operands']) or re.search(r'\[sp[^\]]*\]!',x['operands']) or re.search(r'\[sp\],',x['operands'])],'assembly_sha256':hashlib.sha256(asm.read_bytes()).hexdigest()})
 result['variants'][label]={'worker_sha256':audit['worker_sha256'],'worker_bytes':audit['worker_bytes'],'compiler_source':binding['compiler_source'],'linked_text_bytes':int(text[0]),'audit_sha256':hashlib.sha256(raw).hexdigest(),'functions':functions,'missing_symbols':audit['missing_symbols']}
b=result['variants']['before'];a=result['variants']['after'];result['linked_text_byte_delta']=a['linked_text_bytes']-b['linked_text_bytes'];result['worker_byte_delta']=a['worker_bytes']-b['worker_bytes']
result['scope']='Actual linked host caller/callee static instruction records and raw assembly. Counts include failure/panic paths, not a dynamic or successful-path instruction ledger. Stack adjustments are recorded, not summed into an invented simultaneous stack depth. Missing helper symbols alone do not establish cost removal. Generated Object text/stack is measured separately and expected unchanged. TLS source is unchanged; no TLS sample weight is claimed removed.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'linked_text_byte_delta':result['linked_text_byte_delta'],'worker_byte_delta':result['worker_byte_delta'],'functions':{label:[{'needle':v['needle'],'instructions':v['static_instruction_count'],'outlined_debit_calls':len(v['outlined_debit_calls'])} for v in data['functions']] for label,data in result['variants'].items()}},indent=2))
