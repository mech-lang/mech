#!/usr/bin/env python3
import collections,json,re,subprocess
from pathlib import Path
root=Path('/private/tmp/mech-astra-audit-20261031/r60-stack-audit');root.mkdir(exist_ok=True)
base=Path('/private/tmp/mech-cached-schema-key-round59-20261031/linked-primary/after/object.o')
after=Path('/private/tmp/mech-vector-address-round60-20261031/linked-primary/after/object.o')
for label,path in [('before',base),('after',after)]:
 counters={name:collections.Counter() for name in ['stack_load','stack_store','indexed_stack_load','indexed_stack_store']}
 examples={};x16=None
 cmd=['xcrun','llvm-objdump','-d','--no-show-raw-insn',str(path)]
 proc=subprocess.Popen(cmd,stdout=subprocess.PIPE,text=True)
 for line in proc.stdout:
  m=re.match(r'\s*([0-9a-f]+):\s+(\w+)\s+(.*)',line)
  if not m:continue
  address,opcode,operand=m.groups()
  if opcode=='mov' and (v:=re.match(r'w16, #(0x[0-9a-f]+)',operand)):x16=int(v[1],16)
  elif opcode=='movk' and (v:=re.match(r'w16, #(0x[0-9a-f]+), lsl #16',operand)) and x16 is not None:x16=(x16&0xffff)|(int(v[1],16)<<16)
  elif opcode in ['ldr','str','ldur','stur','ldrb','strb']:
   kind='load' if opcode.startswith('ld') else 'store'
   if re.search(r'\[sp, x16, sxtx\]',operand) and x16 is not None:
    counters['stack_'+kind][hex(x16)]+=1;counters['indexed_stack_'+kind][hex(x16)]+=1
    examples.setdefault(hex(x16),[])
    if len(examples[hex(x16)])<6:examples[hex(x16)].append(line.strip())
   elif v:=re.search(r'\[sp(?:, #(0x[0-9a-f]+))?\]',operand):
    counters['stack_'+kind][hex(int(v[1] or '0',16))]+=1
 proc.wait()
 report={'path':str(path),'counts':{k:dict(v.most_common()) for k,v in counters.items()},'examples':examples}
 (root/(label+'.json')).write_text(json.dumps(report,indent=2))
 print(label,json.dumps({k:v.most_common(15) for k,v in counters.items()},indent=2))
