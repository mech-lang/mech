from pathlib import Path
import collections,json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={}
for label,p in [('before',r/'census.json'),('after',r/'sites-after/census.json')]:
 d=json.loads(p.read_text());g=collections.Counter();b=collections.Counter();n=collections.Counter()
 for x in d['sites']:
  fs=x['frames']
  if any('PrivateUnsignedMatrix6finish' in f for f in fs):name='private numeric publication lock'
  elif any('from_numeric_matrix' in f for f in fs):name='typed-matrix construction (includes retained owners)'
  elif any('into_memory_budget_inner' in f for f in fs):name='budget admission: constant/setup' if any('15new_with_budget' in f for f in fs) else 'budget admission: execution'
  elif any('unlink_budget_registration' in f for f in fs):name='budget-registration removal'
  elif any('budget_import' in f for f in fs):name='budget-import lookup'
  elif any('15prepare_private' in f for f in fs):name='private frame preparation'
  elif any('finalized_value' in f for f in fs):name='other immutable owner construction'
  else:name='other'
  g[name]+=x['count'];b[name]+=x['requested_bytes']
  if any('from_numeric_matrix' in f for f in fs):n[x['bytes_each']]+=x['count']
 result[label]={'requests':d['requests'],'requested_bytes':d['counter']['requested_bytes'],'complete_stack_events':d['trace']['events'],'unique_pcs':d['unique_pcs'],'categories':[{'category':k,'requests':v,'requested_bytes':b[k]} for k,v in g.most_common()],'typed_matrix_sizes':dict(n)}
assert result['before']['requests']==8773 and result['after']['requests']==8773
before=json.loads((r/'census.json').read_text());after=json.loads((r/'sites-after/census.json').read_text())
bs={(x['kind'],x['bytes_each']):x['count'] for x in before['sizes']};as_={(x['kind'],x['bytes_each']):x['count'] for x in after['sizes']}
delta={str(key):as_.get(key,0)-bs.get(key,0) for key in bs.keys()|as_.keys() if as_.get(key,0)!=bs.get(key,0)}
assert delta=={},delta
result['size_kind_deltas']=delta
result['scope']='Retained R56 second complete-call census for the exact R57 baseline, and a fresh second complete eight-byte candidate call; both use the respective optimized graph and Object/image. All events reconcile exactly to fresh uninstrumented counters. Categories use ordered mutually exclusive stack predicates, not elapsed samples. Raw PCs and same-executable atos results are preserved; unclassified requests stay other.'
out.write_text(json.dumps(result,indent=2)+'\n')
