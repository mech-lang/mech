"""Verify fresh host-only candidate emission against the retained baseline image."""
from pathlib import Path
import hashlib,json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={};meta={}
for label in ['before','after']:
 p=r/'linked-primary'/label;b=json.loads((p/'binding.json').read_text());meta[label]=json.loads((p/'metadata.json').read_text());result[label]={}
 for name,pin in b['pins'].items():
  raw=(p/name).read_bytes();assert len(raw)==pin['bytes'] and hashlib.sha256(raw).hexdigest()==pin['sha256'];result[label][name]=pin
assert result['before']==result['after'],'Projection budget-only change must preserve emitted Object/image/metadata'
for key in ['code_bytes','stack_bytes']:
 result[key]={'before':meta['before'][key],'after':meta['after'][key],'delta':meta['after'][key]-meta['before'][key]};assert result[key]['delta']==0
result['identity_equal']=True;result['scope']='Fresh candidate emission verified byte-for-byte against R63 Object/image/metadata. No new disassembly is required to infer identical static instruction bytes; dynamic whole-call hardware counters are measured separately.'
result['baseline_provenance']=json.loads((r/'baseline/provenance.json').read_text())
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
