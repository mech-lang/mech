"""Numerically reconstruct the retained static host excerpts, without binaries."""
from pathlib import Path
import json,re,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'variants':{}}
for label in ['before','after']:
 p=r/'host-assembly'/label;audit=json.loads((p/'audit.json').read_text());binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());assert audit['worker_sha256']==binding['worker_sha256'];assert audit['size_exit']==0
 text=(p/'host-section-sizes.txt').read_text();m=re.search(r'^__text\s+(\d+)\s+',text,re.M);assert m,text[:1000]
 rows=[]
 for region in audit['regions']:
  assert region['exit']==0 and region['static_instruction_count']>0
  rows.append({'needle':region['needle'],'symbol':region['symbol'],'static_instructions':region['static_instruction_count'],'calls':[i['operands'] for i in region['instructions'] if i['opcode'] in ['bl','blr']],'own_stack_adjustments':[i['opcode']+' '+i['operands'] for i in region['instructions'][:20] if 'sp' in i['operands']],'scope':'Own bounded function excerpt; not a call-graph stack maximum or dynamic instruction attribution.'})
 result['variants'][label]={'worker_sha256':audit['worker_sha256'],'worker_bytes':audit['worker_bytes'],'linked_text_bytes':int(m[1]),'compiler_source':binding['compiler_source'],'missing_symbols':audit['missing_symbols'],'regions':rows}
b=result['variants']['before'];a=result['variants']['after'];assert 'into_private_unsigned_matrix' in b['missing_symbols'] and 'into_private_unsigned_matrix' not in a['missing_symbols'];assert any(x['needle']=='into_private_updates' for x in b['regions']);assert 'into_private_updates' in a['missing_symbols'] and 'into_private_appends' in a['missing_symbols']
result['delta']={k:a[k]-b[k] for k in ['worker_bytes','linked_text_bytes']}
result['scope']='Actual linked ARM64 functions. Fresh confinement and the two private leaf entries are present; the old post-leaf into_private_updates/appends reconstruction entries are no longer linked. Input consuming-transfer machinery and ordinary canonical admission remain for their other real consumers. Generated compressor Object/image/metadata remain identical. Whole-call instructions are measured independently; static function sizes and sampling weights are not dynamic instruction counts.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'delta':result['delta'],'symbols':{k:{'missing':v['missing_symbols'],'regions':[(x['needle'],x['static_instructions']) for x in v['regions']]} for k,v in result['variants'].items()}},indent=2))
