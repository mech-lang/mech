"""Reconstruct actual linked planning-function excerpts, without executable inputs."""
from pathlib import Path
import json,re,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'variants':{}}
for label,inner in [('before','16plan_turn_memory'),('after','18derive_turn_memory')]:
 p=r/'host-assembly'/label;audit=json.loads((p/'audit.json').read_text());binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());assert audit['worker_sha256']==binding['worker_sha256'] and audit['size_exit']==0
 m=re.search(r'^__text\s+(\d+)\s+',(p/'host-section-sizes.txt').read_text(),re.M);assert m
 selected=[x for x in audit['regions'] if x['symbol'].endswith('12plan_current') or x['symbol'].endswith(inner)]
 assert len(selected)==2
 rows=[]
 for x in selected:
  assert x['exit']==0 and x['static_instruction_count']>0
  rows.append({'symbol':x['symbol'],'needle':x['needle'],'static_instructions':x['static_instruction_count'],'calls':[i['operands'] for i in x['instructions'] if i['opcode'] in ['bl','blr']],'scope':'Exact own function excerpt only; no transitive instruction or stack total.'})
 result['variants'][label]={'worker_sha256':audit['worker_sha256'],'worker_bytes':audit['worker_bytes'],'linked_text_bytes':int(m[1]),'compiler_source':binding['compiler_source'],'regions':rows}
b=result['variants']['before'];a=result['variants']['after'];old=next(x for x in b['regions'] if x['needle']=='plan_current');new=next(x for x in a['regions'] if x['needle']=='plan_current')
assert old['static_instructions']==679 and new['static_instructions']==83
assert any('BoundCall' in x and 'Clone' in x for x in old['calls'])
assert any('___rust_alloc' in x for x in old['calls'])
assert not any('Clone' in x or '___rust_alloc' in x for x in new['calls'])
result['delta']={k:a[k]-b[k] for k in ['worker_bytes','linked_text_bytes']}
result['adapter_static_instructions']={'before':old['static_instructions'],'after':new['static_instructions'],'delta':new['static_instructions']-old['static_instructions']}
result['scope']='The actual host adapter removes its deep-copy/allocation calls and shrinks from 679 to 83 own static instructions. The whole linked host can still grow; report its measured delta separately. Fresh generated compressor Object/image/metadata remain identical. Dynamic whole-call counters are separate; do not multiply static counts or sample shares by hardware instructions.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='variants'},indent=2))
