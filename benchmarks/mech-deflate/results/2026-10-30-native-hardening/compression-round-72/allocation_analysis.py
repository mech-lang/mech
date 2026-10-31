"""Compare two freshly collected full censuses against their independent heap cohorts."""
from pathlib import Path
from collections import Counter
import hashlib,json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'schema':'mech.shared-descriptor-semantics-census.v1'}
for label in ['before','after']:
 relative='sites-'+label+'/census.json';raw=(r/relative).read_bytes();d=json.loads(raw);sites=d['sites'];counter=d['counter']
 assert sum(s['count'] for s in sites)==d['requests']==counter['allocation_requests']
 assert sum(s['requested_bytes'] for s in sites)==counter['requested_bytes']
 assert sum(s['count'] for s in sites if s['kind']=='realloc')==counter['reallocations']
 assert all(s['count']*s['bytes_each']==s['requested_bytes'] and len(s['frames'])==len(s['pcs']) for s in sites)
 observations=[json.loads(line.removeprefix('ALLOC_PROBE ')) for line in (r/('counters/'+label+'-memory/run.stderr')).read_text().splitlines() if line.startswith('ALLOC_PROBE ')][1:-1]
 assert len(observations)==8 and all((v['allocation_requests'],v['requested_bytes'],v['reallocations'])==(d['requests'],counter['requested_bytes'],counter['reallocations']) for v in observations)
 histogram=Counter()
 for site in sites:histogram[(site['kind'],site['bytes_each'])]+=site['count']
 result[label]={'receipt':relative,'sha256':hashlib.sha256(raw).hexdigest(),'requests':d['requests'],'requested_bytes':counter['requested_bytes'],'reallocations':counter['reallocations'],'unique_pcs':d['unique_pcs'],'unique_stacks_with_size_and_kind':d['unique_stacks_with_size_and_kind'],'size_kind_histogram':[{'kind':kind,'bytes_each':size,'count':count} for (kind,size),count in sorted(histogram.items())],'cohort':'Fresh R72 full census; independent of primary timing and heap-counter cohort.'}
b=result['before'];a=result['after'];result['whole_call_delta']={key:a[key]-b[key] for key in ['requests','requested_bytes','reallocations']}
result['size_kind_histogram_unchanged']=b['size_kind_histogram']==a['size_kind_histogram'];result['successful_allocation_totals_unchanged']=all(v==0 for v in result['whole_call_delta'].values())
result['scope']='Both fresh full traces independently reconcile to separate heap cohorts. Immutable descriptor type/schema authority is shared while every current shape and invocation permission stays dynamic. Per-family deltas are measured, not inferred from whole counts. Requested bytes are not initialized bytes copied; sample weights are not instructions.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k] for k in ['whole_call_delta','size_kind_histogram_unchanged','successful_allocation_totals_unchanged']},indent=2))
