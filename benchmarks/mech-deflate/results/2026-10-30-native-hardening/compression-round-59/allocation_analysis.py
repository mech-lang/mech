"""Reconcile the exact removed family and every allocation size/kind delta."""
from pathlib import Path
from collections import Counter
import json,sys
root=Path(sys.argv[1]);output=Path(sys.argv[2])
target=json.loads((root/'allocation-target.json').read_text());result={};histograms={};target_histograms={};other_histograms={}
def schema(frames):return any('schema8encoding' in f or 'Schema3key' in f for f in frames)
def composition(frames):return any('admit_buffer_frame_composition' in f for f in frames)
for label,path in [('before',root/'census.json'),('after',root/'sites-after/census.json')]:
 data=json.loads(path.read_text());counts=Counter();byte_counts=Counter();hist=Counter();target_hist=Counter();other_hist=Counter()
 for site in data['sites']:
  frames=site['frames'];key=(site['kind'],site['bytes_each']);hist[key]+=site['count']
  if composition(frames) and schema(frames):category='composition canonical schema encoding';target_hist[key]+=site['count']
  elif composition(frames):category='other composition validation'
  elif schema(frames):category='other canonical schema encoding'
  else:category='remaining complete-call requests'
  if category!='composition canonical schema encoding':other_hist[key]+=site['count']
  counts[category]+=site['count'];byte_counts[category]+=site['requested_bytes']
 assert sum(counts.values())==data['requests']==data['trace']['events']==data['counter']['allocation_requests']
 assert sum(byte_counts.values())==data['counter']['requested_bytes']
 assert hist=={(s['kind'],s['bytes_each']):s['count'] for s in data['sizes']}
 histograms[label]=hist;target_histograms[label]=target_hist;other_histograms[label]=other_hist
 result[label]={'requests':data['requests'],'requested_bytes':data['counter']['requested_bytes'],'complete_stack_events':data['trace']['events'],'unique_pcs':data['unique_pcs'],'categories':[{'category':name,'requests':counts[name],'requested_bytes':byte_counts[name]} for name in ['composition canonical schema encoding','other composition validation','other canonical schema encoding','remaining complete-call requests']],'large_requests':sum(v for (_,size),v in hist.items() if size>=65536),'large_requested_bytes':sum(size*v for (_,size),v in hist.items() if size>=65536)}
expected={(row['kind'],row['bytes_each']):row['count'] for row in target['expected_size_kind_removals']}
delta={k:histograms['after'].get(k,0)-histograms['before'].get(k,0) for k in histograms['before'].keys()|histograms['after'].keys() if histograms['after'].get(k,0)!=histograms['before'].get(k,0)}
assert delta=={k:-v for k,v in expected.items()},delta
assert target_histograms['before']==expected and not target_histograms['after']
assert other_histograms['before']==other_histograms['after']
assert result['before']['requests']==8773 and result['after']['requests']==8173
assert result['before']['requested_bytes']-result['after']['requested_bytes']==15447
result['size_kind_deltas']=[{'kind':kind,'bytes_each':size,'after_minus_before':delta[(kind,size)]} for kind,size in sorted(delta)]
result['exact_target_delta']={'requests_removed':600,'requested_bytes_removed':15447,'allocations_removed':222,'reallocations_removed':378,'remaining_size_kind_histogram_unchanged':True}
result['scope']='Retained accepted R56 second complete-call census and fresh R59 second complete-call census. Every successful allocation/reallocation event is symbolized and reconciles with its independent heap counter. Classification is mutually exclusive; the exact target requires both composition-admission and canonical schema-encoding frames. All remaining size/kind counts are identical. Requested bytes include reallocations and reserved capacity; they are neither live bytes nor copied bytes. Census instruction/cycle counters are instrumented and excluded from CPU comparisons.'
output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
