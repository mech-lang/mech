"""Reconcile fresh materialization counts; no compiler or compressor execution."""
from pathlib import Path
from collections import Counter
import hashlib,json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'schema':'mech.private-numeric-lifetime-mechanism.v1','variants':{}}
for label in ['before','after']:
 p=r/('sites-'+label)/'census.json';raw=p.read_bytes();census=json.loads(raw);sites=census['sites']
 def first_leaf(s):
  f='\n'.join(s['frames']);return ('unsigned_matrix_replace' in f or 'unsigned_buffer_with' in f or 'unsigned_matrix_append_private' in f) and 'prepared_library_call' not in f and 'Invocation6helper' in f
 leaf=[s for s in sites if first_leaf(s)]
 old_private=[s for s in sites if any('OwnedSnapshotUnsignedMatrix' in f for f in s['frames'])]
 constructor=[s for s in leaf if any('finalized_value' in f for f in s['frames']) and s['bytes_each'] in [32,112,160]]
 histogram=Counter()
 for s in sites:histogram[s['kind'],s['bytes_each']]+=s['count']
 result['variants'][label]={'census_sha256':hashlib.sha256(raw).hexdigest(),'whole_requests':census['requests'],'whole_requested_bytes':census['counter']['requested_bytes'],'direct_first_change_leaf_requests':sum(s['count'] for s in leaf),'direct_first_change_leaf_bytes':sum(s['requested_bytes'] for s in leaf),'direct_final_shape_data_root_requests':sum(s['count'] for s in constructor),'old_owned_to_private_requests':sum(s['count'] for s in old_private),'old_owned_to_private_bytes':sum(s['requested_bytes'] for s in old_private),'histogram':[{'kind':k[0],'bytes_each':k[1],'count':v} for k,v in sorted(histogram.items())]}
b=result['variants']['before'];a=result['variants']['after'];assert b['direct_first_change_leaf_requests']==1134 and a['direct_first_change_leaf_requests']==966
assert b['direct_final_shape_data_root_requests']==a['direct_final_shape_data_root_requests']==504
assert b['old_owned_to_private_requests']==672 and b['old_owned_to_private_bytes']==68544 and a['old_owned_to_private_requests']==0
bh={(x['kind'],x['bytes_each']):x['count'] for x in b['histogram']};ah={(x['kind'],x['bytes_each']):x['count'] for x in a['histogram']};delta={k:ah.get(k,0)-bh.get(k,0) for k in bh.keys()|ah.keys()};delta={k:v for k,v in delta.items() if v}
expected={('alloc',size):-168 for size in [8,32,64,104,112,160]};assert delta==expected,(delta,expected)
result['removed_request_families']=[{'purpose':name,'bytes_each':size,'requests_removed':168,'requested_bytes_removed':168*size} for name,size in [('runtime handoff shape-parameter clone',8),('reconstructed final shape owner',32),('intermediate canonical import registry lock',64),('reconstructed final registration',104),('reconstructed final root owner',112),('reconstructed final data owner',160)]]
result['whole_delta']={'requests':a['whole_requests']-b['whole_requests'],'requested_bytes':a['whole_requested_bytes']-b['whole_requested_bytes']};assert result['whole_delta']=={'requests':-1008,'requested_bytes':-80640}
result['transitions']=168;result['owner_triples_per_transition']={'before':2,'after':1};result['registrations_per_transition']={'before':2,'after':1}
result['scope']='Complete fresh censuses independently reconcile to heap cohorts. The retained final owner triple stays 504 requests (168 transitions); its additional reconstruction and registration disappear. A first-import lock and runtime shape clone also disappear. All other size/kind families are identical. Counts describe actual constructor/registration events, not a slab-backed substitute. This audit covers the first-change entry, not every compressor materialization or a promise of allocation parity.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'whole_delta':result['whole_delta'],'transitions':result['transitions'],'variants':{k:{x:v[x] for x in ['direct_first_change_leaf_requests','direct_final_shape_data_root_requests','old_owned_to_private_requests']} for k,v in result['variants'].items()}},indent=2))
