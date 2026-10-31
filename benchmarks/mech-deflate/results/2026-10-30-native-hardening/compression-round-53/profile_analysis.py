from pathlib import Path
from collections import Counter
import json,re,hashlib,sys
r=Path(sys.argv[1]); output=Path(sys.argv[2])

def analyze(path):
 text=path.read_text();graph=text.split('Call graph:\n',1)[1].split('\nTotal number in stack',1)[0];nodes=[];stack=[]
 for line in graph.splitlines():
  m=re.match(r'^([ +!|:]*)(\d+) (.*)$',line)
  if not m:continue
  depth=len(m[1])
  while stack and nodes[stack[-1]]['depth']>=depth:stack.pop()
  node={'depth':depth,'count':int(m[2]),'symbol':m[3].split('  (in ',1)[0],'children':[],'ancestors':stack[:]}
  if stack:nodes[stack[-1]]['children'].append(len(nodes))
  stack.append(len(nodes));nodes.append(node)
 leaves=Counter();ancestry=Counter();frame_leaves=Counter();total=0
 groups={'frame copy actions':['12BufferFrames4copy','BufferFrames::copy'],'capacity-buffer calls':['unsigned_buffer_call','unsigned_buffer_with_sublist','copy_buffer_with_sublist'],'memory move/copy':['memcpy','memmove'],'frame actions':['buffer_frame_action'],'tail cleanup':['retire_tail_locals'],'canonical construction/admission':['finalized_value','from_numeric_matrix','into_memory_budget_inner'],'schema closure':['closed_body','close_schema_body'],'projection accounting':['NativeProjectionIndexContext','28NativeProjectionIndexContext'],'thread-local lookup':['_tlv_get_addr']}
 for node in nodes:
  weight=node['count']-sum(nodes[i]['count'] for i in node['children']);assert weight>=0;total+=weight;leaves[node['symbol']]+=weight
  symbols=[nodes[i]['symbol'] for i in node['ancestors']]+[node['symbol']]
  if any('buffer_frame_action' in symbol for symbol in symbols):frame_leaves[node['symbol']]+=weight
  for group,needles in groups.items():
   if any(needle in symbol for symbol in symbols for needle in needles):ancestry[group]+=weight
 return {'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'total_exclusive_weights':total,'exclusive_top':leaves.most_common(15),'frame_exclusive_top':frame_leaves.most_common(20),'ancestry_unions':{key:{'weights':ancestry[key],'percent':100*ancestry[key]/total} for key in groups},'ancestry_groups_overlap':True}
results={}
for label,name in [('baseline','before'),('candidate','after')]:
 path=r/'isolated-profile'/(name+'.resolved.sample.txt')
 if not path.exists():path=r/'isolated-profile'/(name+'.sample.txt')
 results[label]=analyze(path)
results['reference']=analyze(r/'reference-profile/sample.txt')
result={'scope':'Fresh matched serial sample windows. Each Mech worker completes 695 calls; reference 20,000. Sampling includes harness and is excluded from primary elapsed. Weights are samples, not instructions or exact timer fractions; ancestry unions overlap. Opaque reference hashes are not assigned unsupported source names.','profiles':results}
output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({name:{'total':data['total_exclusive_weights'],'ancestry':data['ancestry_unions'],'exclusive_top':data['exclusive_top'][:3]} for name,data in results.items()},indent=2))
