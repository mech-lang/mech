"""Reconcile complete before/after censuses; no compiler or compressor execution."""
from pathlib import Path
from collections import Counter,defaultdict
import hashlib,json,re,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);ledger=json.loads((r/'findings-packet/allocation-target-ledger.json').read_text())

def analyze(relative):
 p=r/relative;raw=p.read_bytes();d=json.loads(raw);sites=d['sites'];counts=d['counter']
 assert sum(s['count'] for s in sites)==d['requests']==counts['allocation_requests']
 assert sum(s['requested_bytes'] for s in sites)==counts['requested_bytes']
 assert sum(s['count'] for s in sites if s['kind']=='realloc')==counts['reallocations']
 assert all(s['requested_bytes']==s['count']*s['bytes_each'] and len(s['frames'])==len(s['pcs']) for s in sites)
 def has(s,key):return any(key in f for f in s['frames'])
 flat={i for i,s in enumerate(sites) if has(s,'admit_buffer_frame_inputs')}
 composed={i for i,s in enumerate(sites) if has(s,'admit_buffer_frame_composition')}
 solver={i for i,s in enumerate(sites) if has(s,'shape_for_resolved_extents')}
 assert not flat&composed and not (flat|composed)&solver
 def summarize(indices):
  rows=[sites[i] for i in sorted(indices)];hist=Counter()
  for s in rows:hist[(s['kind'],s['bytes_each'])]+=s['count']
  return {'requests':sum(s['count'] for s in rows),'requested_bytes':sum(s['requested_bytes'] for s in rows),'unique_stacks':len(rows),'sizes':[{'kind':kind,'bytes_each':size,'count':n} for (kind,size),n in sorted(hist.items())]}
 groups=defaultdict(set)
 for i in flat|composed:
  entry=next(f for f in sites[i]['frames'] if 'mech_canonical_program_entry_v1' in f)
  offset=int(re.search(r'\+ (\d+)$',entry)[1]);groups[('flat' if i in flat else 'composed',offset)].add(i)
 result={'receipt':relative,'sha256':hashlib.sha256(raw).hexdigest(),'requests':d['requests'],'requested_bytes':counts['requested_bytes'],'reallocations':counts['reallocations'],'unique_pcs':d['unique_pcs'],'families':{'flat_frame_shape_validation':summarize(flat),'composed_frame_admission':summarize(composed),'combined_candidate':summarize(flat|composed),'separate_resolved_extent_solver':summarize(solver),'outside_candidate':summarize(set(range(len(sites)))-(flat|composed))},'entry_return_groups':[{'family':family,'entry_return_offset':offset,**summarize(indices)} for (family,offset),indices in sorted(groups.items())],'candidate_stacks':[{'census_index':i,**sites[i]} for i in sorted(flat|composed)]}
 return result
before=analyze('baseline/census.json');after=analyze('sites-after/census.json')
assert before['sha256']==ledger['census']['sha256']
for key in ['flat_frame_shape_validation','composed_frame_admission','separate_resolved_extent_solver']:
 for measure in ['requests','requested_bytes']:assert before['families'][key][measure]==ledger['families'][key][measure]
assert before['families']['combined_candidate']['requests']==68 and before['families']['combined_candidate']['requested_bytes']==1024
for label,summary in [('before',before),('after',after)]:
 probe=[json.loads(s.removeprefix('ALLOC_PROBE ')) for s in (r/('counters/'+label+'-memory/run.stderr')).read_text().splitlines() if s.startswith('ALLOC_PROBE ')][1:-1]
 assert len(probe)==8
 assert all((p['allocation_requests'],p['requested_bytes'],p['reallocations'])==(summary['requests'],summary['requested_bytes'],summary['reallocations']) for p in probe)
result={'schema':'mech.current-frame-shape-allocation.v1','before':before,'after':after,'family_deltas':{key:{m:after['families'][key][m]-before['families'][key][m] for m in ['requests','requested_bytes']} for key in before['families']},'whole_call_delta':{m:after[m]-before[m] for m in ['requests','requested_bytes','reallocations']},'all_attributed_candidate_requests_disappeared':after['families']['combined_candidate']['requests']==0,'source_attribution_caveat':'Flat stacks explicitly name FrameShapeRecipe::validate. Composed stacks retain complete composition-admission ancestry but their immediate allocation callee is deduplicated; attribution to shape validation uses source and allocation-pattern evidence. The full family comparison establishes request/byte effects, not all branch visits or per-branch eligibility counts.','baseline_trace_provenance':'Before-family attribution uses the retained R62 complete census (SHA-256 ae2452719ce2d535ab0528d207d1cf9109da2d5b9b96f0ae3bbe449f5e3a1cee), not a fresh R63 before trace. Its total requests/bytes/reallocations reconcile with freshly executed R63 before heap counters. Candidate family attribution is the fresh R63 after census.', 'scope':'Retained before and fresh after allocation traces reconcile independently to fresh heap counters. Requested capacity is not initialized bytes copied, and allocation events are not CPU samples or retired instructions.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'whole_call_delta':result['whole_call_delta'],'family_deltas':result['family_deltas'],'all_candidate_requests_disappeared':result['all_attributed_candidate_requests_disappeared']},indent=2))
