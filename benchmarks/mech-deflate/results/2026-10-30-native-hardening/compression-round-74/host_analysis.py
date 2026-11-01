"""Reconstruct measured linked host debit and callback functions without executing binaries."""
from pathlib import Path
import json,re,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'variants':{}}
for label in ['before','after']:
 p=r/'host-assembly'/label;audit=json.loads((p/'audit.json').read_text());binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());assert audit['worker_sha256']==binding['worker_sha256'] and audit['size_exit']==0
 m=re.search(r'^__text\s+(\d+)\s+',(p/'host-section-sizes.txt').read_text(),re.M);assert m
 rows=[]
 for x in audit['regions']:
  assert x['exit']==0 and x['static_instruction_count']>0
  if x['needle'] in ['scoped-budget','ordinary-budget','checked-total','local-next','commit-debit','hot-frame','hot-helper','hot-visit','hot-budget-compute']:
   rows.append({'symbol':x['symbol'],'needle':x['needle'],'static_instructions':x['static_instruction_count'],'opcode_sequence':[i['opcode'] for i in x['instructions']],'calls':[i['operands'] for i in x['instructions'] if i['opcode'] in ['bl','blr']],'scope':'Bounded own function only. Inlined or deduplicated functions may lack standalone symbols.'})
 result['variants'][label]={'worker_sha256':audit['worker_sha256'],'worker_bytes':audit['worker_bytes'],'linked_text_bytes':int(m[1]),'compiler_source':binding['compiler_source'],'missing_symbols':audit['missing_symbols'],'regions':rows}
b=result['variants']['before'];a=result['variants']['after'];result['delta']={key:a[key]-b[key] for key in ['worker_bytes','linked_text_bytes']}
result['scope']='Actual linked ARM64 host functions and whole host text, after all timing. Scoped current-context control admission changes host code while generated compressor Object/image/metadata remain identical. Static own instruction counts, missing/inlined symbols and CPU-time samples are not executed instruction counts; whole-call hardware counters are measured separately.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'delta':result['delta'],'regions':{k:[(x['needle'],x['static_instructions']) for x in v['regions']] for k,v in result['variants'].items()}},indent=2))
