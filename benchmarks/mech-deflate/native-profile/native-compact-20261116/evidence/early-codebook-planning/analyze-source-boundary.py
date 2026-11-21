"""Discover a source boundary; these debug records are not compiler admission."""
from argparse import ArgumentParser
from collections import deque
from hashlib import sha256
from pathlib import Path
import json,re
p=ArgumentParser(description=__doc__)
p.add_argument('source_cfg',type=Path);p.add_argument('--entry',type=int,default=4119);p.add_argument('--exit',type=int,default=5031);p.add_argument('--output',type=Path,required=True)
a=p.parse_args();data=json.loads(a.source_cfg.read_text());sites=data['sites'];values={v['value']:v for v in data['values']}
active={s['site'] for s in sites if s['block'] is not None}

def successors(text):
 if text in ['Scalar(Complete)','Scalar(Reject)','Scalar(IntegrityFailure)']:return []
 m=re.fullmatch(r'Scalar\(Jump\((\d+)\)\)',text)
 if m:return [int(m[1])]
 if text.startswith('Scalar(Branch {') or text.startswith('Scalar(LiteralBranch {'):
  yes=re.search(r'yes: (\d+)',text);no=re.search(r'no: (\d+)',text)
  assert yes and no,text
  return [int(yes[1]),int(no[1])]
 if text.startswith(('Header {','TailHeader {')):
  body=re.search(r'body: (\d+)',text);assert body,text
  exits=re.search(r'exit: (\d+)',text)
  return [int(body[1])]+([int(exits[1])] if exits else [])
 raise ValueError('Unrecognized diagnostic terminator: '+text)

edges={s:successors(sites[s]['terminator']) for s in active};pred={s:[] for s in active}
for s,targets in edges.items():
 for target in targets:
  assert target in active,(s,target)
  pred[target].append(s)
assert a.entry in active and a.exit in active
region=set();todo=[a.entry]
while todo:
 s=todo.pop()
 if s==a.exit or s in region:continue
 region.add(s);todo.extend(edges[s])
external_entries={str(s):[n for n in pred[s] if n not in region] for s in sorted(region) if any(n not in region for n in pred[s])}
normal_exits={str(s):[n for n in edges[s] if n not in region] for s in sorted(region) if any(n not in region for n in edges[s])}
assert set(map(int,external_entries)).issubset({a.entry}),external_entries
assert all(n==a.exit for ns in normal_exits.values() for n in ns),normal_exits
failure_sites=[s for s in sorted(region) if not edges[s]]
assert all(sites[s]['terminator'] in ['Scalar(Reject)','Scalar(IntegrityFailure)'] for s in failure_sites),'source-level completion inside proposed region'

def liveness(nodes,terminal=None):
 live={s:set() for s in nodes};queue=deque(sorted(nodes,reverse=True));pending=set(nodes)
 while queue:
  s=queue.popleft();pending.remove(s)
  out=set().union(*(live[n] if n in nodes else (terminal or {}).get(n,set()) for n in edges[s]))
  before=set(sites[s]['uses']) | (out-set(sites[s]['defs']))
  if before!=live[s]:
   live[s]=before
   for prior in pred[s]:
    if prior in nodes and prior not in pending:queue.append(prior);pending.add(prior)
 return live

root_live=liveness(active)
defs=set().union(*(set(sites[s]['defs']) for s in region))
outputs=defs & root_live[a.exit]
phase_live=liveness(region,{a.exit:outputs})
inputs=phase_live[a.entry]
used=set().union(*(set(sites[s]['uses']) | set(sites[s]['defs']) for s in region))
# This is a conservative value-level discovery interface. Owner retirement and
# hidden collection cursor state still require typed compiler admission.
owner_slots={v['base']+lane:v['value'] for v in values.values() for lane in v['owners']}
retired_outside={owner_slots[slot] for s in active-region for slot in sites[s]['retire_after'] if slot in owner_slots}
owner_candidates=defs & retired_outside
spec=lambda ids:[values[v] for v in sorted(ids)]
result={'status':'source-CFG discovery only; no early emission or compiler admission yet','source_cfg_sha256':sha256(a.source_cfg.read_bytes()).hexdigest(),'canonical_entry':a.entry,'canonical_exit':a.exit,'native_boundary':{'entry':sites[a.entry]['block'],'exit':sites[a.exit]['block']},'all_source_sites':len(sites),'reachable_source_sites':len(active),'region_source_sites':len(region),'region_sites':sorted(region),'external_entries':external_entries,'normal_exits':normal_exits,'explicit_failure_sites':failure_sites,'region_distinct_definitions':len(defs),'region_touched_values':len(used),'region_touched_value_words':sum(values[v]['words'] for v in used),'global_layout_values':len(values),'global_layout_words':sum(v['words'] for v in values.values()),'value_level_inputs':spec(inputs),'value_level_outputs':spec(outputs),'input_words':sum(values[v]['words'] for v in inputs),'output_words':sum(values[v]['words'] for v in outputs),'owner_definitions_retired_outside':spec(owner_candidates),'region_action_kinds':sorted({sites[s]['action'].split('(')[0].split(' {')[0] for s in region}),'unproved_requirements':['typed ownership/effects and hidden collection cursor captures','source failure origins and ordering, partial writes, cleanup and recovery','local slots/SSA and liveness for root plus helper','total constructed blocks/instructions/SSA slots and peak unit size across all bodies','CFG/dominance and full-validation counts across all bodies','whole-encoder bytes, complete build time/RSS and actual-worker qualification if changed']}
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['region_sites','value_level_inputs','value_level_outputs','owner_definitions_retired_outside']},indent=2))
