"""Classify recorded encoder-stack samples and rank qualified losses without forecasting latency."""
from pathlib import Path
from collections import Counter
from hashlib import sha256
import json,re,math,gzip
OUT=Path(__file__).resolve().parent;ROOT=OUT
PACKET=OUT.parents[1]
read=lambda p:json.loads(p.read_text())
def payload(p):
 p=Path(p)
 return p.read_bytes() if p.exists() else gzip.decompress(p.with_name(p.name+'.gz').read_bytes())
sha=lambda p:sha256(payload(p)).hexdigest()
nearest=read(PACKET/'evidence/early-codebook-emission/runtime-targets/nearest.json');plan=read(OUT/'plan.json')
def parse(path,actor):
 s=payload(path).decode().split('Call graph:\n',1)[1].split('\nTotal number',1)[0];rows=[];stack=[]
 for line in s.splitlines():
  m=re.match(r'^([ +!:|]*)(\d+) (.+)$',line)
  if not m:continue
  depth=len(m[1]);count=int(m[2]);name=m[3].split('  (in ',1)[0]
  while stack and rows[stack[-1]]['depth']>=depth:stack.pop()
  parent=stack[-1] if stack else None
  row={'depth':depth,'count':count,'name':name,'parent':parent,'children':0,'frame':m[3]}
  if parent is not None:rows[parent]['children']+=count
  rows.append(row);stack.append(len(rows)-1)
 assert rows and all(r['count']>=r['children'] for r in rows)
 def root(r):return r['name']=='mech_compact_closed_v2' if actor=='mech' else r['name']=='deflate_compress_near_optimal'
 inside=[False]*len(rows);buckets=Counter();functions=Counter();all_functions=Counter();roots=[]
 for i,r in enumerate(rows):
  inherited=inside[r['parent']] if r['parent'] is not None else False
  inside[i]=inherited or root(r)
  if root(r) and not inherited:roots.append(i)
  own=r['count']-r['children'];all_functions[r['name']]+=own
  if not inside[i] or not own:continue
  functions[r['name']]+=own
  if actor=='mech':
   if r['name'].startswith('mech_compact_numeric_475'):bucket='matcher'
   elif r['name'].startswith('mech_compact_numeric_3009'):bucket='scorer'
   elif r['name']=='mech_compact_closed_checked':bucket='controller self'
   elif r['name'] in ['__bzero','_platform_memset','_platform_memset_pattern16']:bucket='zeroing'
   elif r['name'] in ['_platform_memmove','_platform_memcpy']:bucket='copying'
   elif r['name'].startswith('mech_compact_numeric_3447'):bucket='histogram'
   elif r['name'].startswith('mech_compact_numeric_'):bucket='other numeric kernels'
   elif 'allocate' in r['name'] or '_mi_' in r['name'] or r['name'].startswith('mi_'):bucket='allocation'
   elif 'retire' in r['name'] or 'release' in r['name']:bucket='retirement'
   else:bucket='other encoder calls'
  else:
   if r['name']=='deflate_find_min_cost_path':bucket='scorer'
   elif r['name']=='deflate_compress_near_optimal':bucket='near-optimal body (includes inlined matching)'
   elif 'huffman' in r['name'] or 'precompute_huffman' in r['name']:bucket='Huffman'
   elif r['name'] in ['__bzero','_platform_memset','_platform_memset_pattern16']:bucket='zeroing'
   elif r['name'] in ['_platform_memmove','_platform_memcpy']:bucket='copying'
   else:bucket='other encoder calls'
  buckets[bucket]+=own
 total=sum(rows[i]['count'] for i in roots);assert total>0 and sum(buckets.values())==total,(str(path),actor,total,sum(buckets.values()),[r['name'] for r in rows if 'deflate' in r['name']][:12])
 return {'encoder_roots':'mech_compact_closed_v2' if actor=='mech' else 'deflate_compress_near_optimal','encoder_stack_samples':total,'all_main_thread_samples':rows[0]['count'],'buckets':{k:{'samples':v,'percent':100*v/total} for k,v in buckets.most_common()},'top_encoder_functions':functions.most_common(15),'top_all_functions':all_functions.most_common(15),'scope':'Exclusive leaf samples within the named encoder function tree. Constructor/final-transfer/cleanup outside that tree and caller validation are not all represented. C near-optimal self samples include inlined matching. These are not elapsed phase times or forecasts.'}
profiles={};aliases={}
for group in plan['distinct_contracts']:
 case=group['case'];profiles[case]={}
 for alias in group['aliases']:aliases[alias]=case
 for actor in ['mech','libdeflate']:
  folder=OUT/'profiles'/case/actor;r=read(folder/'result.json');assert r['sample_exit_code']==0 and sha(folder/'sample.txt')==r['sample_sha256'];stats=parse(folder/'sample.txt',actor)
  calls=r['verified_calls'];assert len(calls)==2
  fields=['payload_allocation_requests','requested_payload_bytes','initialized_bytes_copied','zeroed_bytes','peak_private_payload_bytes','raw_bytes']
  stats['counters']={k:calls[0][k] for k in fields if k in calls[0]};assert all(all(c[k]==v for k,v in stats['counters'].items()) for c in calls)
  stats.update(worker_sha256=r['worker_sha256'],sample_sha256=r['sample_sha256']);profiles[case][actor]=stats
ranking=[]
for c in nearest['cases']:
 med=c['median_seconds'];gaps={o:med[o]['mech-experiment']-med[o]['libdeflate'] for o in ['forward','reverse']};key=aliases[c['case']];stats=profiles[key]['mech']
 ranking.append({'case':c['case'],'profile_contract':key,'required_time_cut_percent_both_orders':c['required_time_cut_percent_both_orders'],'paired_mech_over_c':c['paired_mech_over_libdeflate'],'median_seconds':med,'excess_seconds':gaps,'worst_excess_seconds':max(gaps.values()),'mech_profile_percent':{k:v['percent'] for k,v in stats['buckets'].items()}})
ranking.sort(key=lambda r:r['worst_excess_seconds'],reverse=True)
# A descriptive fit to independently measured actor medians, not a phase model.
points=[]
for c in ranking:
 if c['case'].startswith('prefix-random-'):
  n=int(c['case'].rsplit('-',1)[1]);y=sum(c['excess_seconds'].values())/2;points.append((n,y))
xbar=sum(x for x,y in points)/len(points);ybar=sum(y for x,y in points)/len(points);slope=sum((x-xbar)*(y-ybar) for x,y in points)/sum((x-xbar)**2 for x,y in points);intercept=ybar-slope*xbar
r2=1-sum((y-intercept-slope*x)**2 for x,y in points)/sum((y-ybar)**2 for x,y in points)
summary={'source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','qualified_worker_sha256':plan['qualified_reference_worker'],'named_losing_cases':19,'distinct_profiled_contracts':18,'captures':36,'scope':'Qualified medians determine gaps. New samples/counters identify work to investigate; they do not establish an optimization gain. Mech and C root scopes are explicit and are not directly interchangeable phase-time measurements.','profiles':profiles,'ranking_by_absolute_gap':ranking,'random_prefix_descriptive_fit':{'n':len(points),'intercept_seconds':intercept,'slope_seconds_per_byte':slope,'r_squared':r2,'scope':'Descriptive excess-time scaling across current prefix-random fixture sizes. This does not identify the cause or predict savings.'},'selected_worker_changed':False,'roc_floor':81,'libdeflate_floor':62,'next_investigation':'Prioritize the match collector on large random input and monitor natural 4KiB. Map its hot PCs and source reads; revisit literal-read reuse and short-match rejection using current proofs/packet/spill improvements. Preserve the cursor-countdown/exit-pointer repair as a smaller parallel line of investigation, performed serially on this host.'}
(OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
for name in ['prefix-random-1048576','prefix-random-4096','16384-distribution-change','prefix-natural-1024','prefix-natural-4096']:
 s=profiles[aliases[name]]['mech'];print(name,'samples',s['encoder_stack_samples'],{k:round(v['percent'],2) for k,v in s['buckets'].items()})
print('random excess fit ns/byte',slope*1e9,'intercept us',intercept*1e6,'R2',r2)
