"""Summarize completed receipts only; never execute the compiler or tests."""
from pathlib import Path
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);rows=[]
def receipt(path,count=None,inventory=False):
 d=json.loads((r/path).read_text());value=d.get('listed_tests' if inventory else 'executed_tests',d.get('passed_tests'));passed=d.get('exit')==0 and (count is None or value==count)
 rows.append({'receipt':path,'command':d.get('command'),'exit':d.get('exit'),'expected_tests':count,'observed_tests':value,'inventory_only':inventory,'passed':passed})
for path,count,inventory in [('units-after/build.json',None,False),('units-after/vector-address-inventory.json',1,True),('units-after/vector-address.json',1,False),('units-after/preparation-inventory.json',2,True),('units-after/preparation.json',2,False),('units-after/runtime-inventory.json',327,True),('units-after/runtime.json',327,False),('units-after/work-values-inventory.json',32,True),('units-after/work-values.json',32,False),('qualification/component-build.json',None,False),('qualification/path-mechanism-inventory.json',1,True),('qualification/path-mechanism.json',1,False),('qualification/components-inventory.json',236,True),('qualification/components.json',236,False),('qualification/composition-inventory.json',8,True),('qualification/composition.json',8,False),('ci/workflow-python.json',132,False),('linked-acceptance/emit-build.json',None,False),('linked-acceptance/emit.json',None,False),('linked-acceptance/worker-build.json',None,False),('linked-acceptance/run.json',None,False)]:receipt(path,count,inventory)
ci=json.loads((r/'ci/results.json').read_text());assert len(ci)==3
for row in ci[1:]:
 inventory='listed_tests' in row;value=row.get('listed_tests',row.get('executed_tests'));rows.append({'receipt':'ci/results.json','command':row['command'],'exit':row['exit'],'expected_tests':33,'observed_tests':value,'inventory_only':inventory,'passed':row['exit']==0 and value==33})
architecture=json.loads((r/'architecture/results.json').read_text())
for row in architecture:rows.append({'receipt':'architecture/results.json','command':row['command'],'exit':row['exit'],'passed':row['exit']==0})
scalar=json.loads((r/'primary/scalar-results.json').read_text());frame=json.loads((r/'frame-allocation/results.json').read_text());linked=json.loads((r/'linked-acceptance/run.stdout').read_text())
rows.append({'receipt':'primary/scalar-results.json','cases':len(scalar['controls']),'passed':len(scalar['controls'])==12 and scalar['direct_controls_pass'] and scalar['scalar_loop_allocation_slope_zero'] and all(all(x['checks'].values()) for x in scalar['controls'])})
rows.append({'receipt':'frame-allocation/results.json','cases':len(frame),'passed':len(frame)==8 and all(x['no_request_byte_or_work_increase'] for x in frame)})
rows.append({'receipt':'linked-acceptance/run.stdout','cases':len(linked),'passed':[x.get('generated_success') for x in linked]==[36,30,None,None,None,17]*2})
parent=r/'parent-cases/cases.json'
if parent.exists():
 d=json.loads(parent.read_text());rows.append({'receipt':'parent-cases/cases.json','command':d.get('command'),'exit':d.get('exit'),'expected_tests':2,'observed_tests':d.get('passed'),'passed':d.get('exit')==0 and d.get('passed')==2 and d.get('failed')==0})
result={'all_required_checks_pass':all(x['passed'] for x in rows),'checks':rows,'skipped_coverage':[{'test':'complete_unchanged_knucleotide_body_uses_the_owned_emitted_map','disposition':'Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.'},{'coverage':'full remote CI','disposition':'Not awaited as requested. Local exact inventory and architecture checks reported separately.'},{'coverage':'larger compression inputs/full-scale timing','disposition':'Not run; this optimization is measured on the complete pinned eight-byte level-12 call only.'}],'scope':'Counts are observed from actual archived inventories and execution receipts, never inferred from source annotations. Test batches overlap; counts must not be summed as unique tests.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'all_required_checks_pass':result['all_required_checks_pass'],'checks':len(rows),'failures':[x for x in rows if not x['passed']]},indent=2))
