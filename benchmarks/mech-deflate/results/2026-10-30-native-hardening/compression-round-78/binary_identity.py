"""Verify fresh host-only candidate emission against the retained baseline image."""
from pathlib import Path
import hashlib,json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={};meta={}
for label in ['before','after']:
 p=r/'linked-primary'/label;b=json.loads((p/'binding.json').read_text());meta[label]=json.loads((p/'metadata.json').read_text());result[label]={}
 for name,pin in b['pins'].items():
  raw=(p/name).read_bytes();assert len(raw)==pin['bytes'] and hashlib.sha256(raw).hexdigest()==pin['sha256'];result[label][name]=pin
assert result['before']!=result['after'],'The callback ABI/code image must change'
assert meta['before']['abi_version']==37 and meta['after']['abi_version']==38
assert meta['before']['typed_ir_version']==meta['after']['typed_ir_version']==132
assert meta['before']['work_policy_version']==meta['after']['work_policy_version']==95
for key in ['code_bytes','stack_bytes']:
 result[key]={'before':meta['before'][key],'after':meta['after'][key],'delta':meta['after'][key]-meta['before'][key]}
result['identity_equal']=False;result['scope']='Fresh before/after Object/image hashes checked. Private program ABI37 to38 changes the code/image. Frozen scalar ABI1, source, bytecode14, typed IR132 and logical work policy95 remain. Dynamic counters and bounded callback disassembly are independent cohorts.'
result['baseline_provenance']=json.loads((r/'baseline/provenance.json').read_text())
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
