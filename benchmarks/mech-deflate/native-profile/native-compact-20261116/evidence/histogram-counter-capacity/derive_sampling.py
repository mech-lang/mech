"""Compute exclusive sampled shares strictly within each encoder's call-tree roots."""
from collections import Counter
from pathlib import Path
import json,re

def one(path,root_name):
    text=path.read_text().split('Call graph:\n',1)[1].split('\nTotal number in stack',1)[0]
    nodes=[];stack=[]
    for line in text.splitlines():
        match=re.match(r'^([ +!:|]*)(\d+) (.*?)  \(in ',line)
        if not match:continue
        prefix,count,name=match.groups();depth=len(prefix)
        while stack and nodes[stack[-1]]['depth']>=depth:stack.pop()
        parent=stack[-1] if stack else None
        node={'depth':depth,'samples':int(count),'name':name,'children':[],'parent':parent}
        if parent is not None:nodes[parent]['children'].append(len(nodes))
        stack.append(len(nodes));nodes.append(node)
    roots=[]
    for i,node in enumerate(nodes):
        if node['name']!=root_name:continue
        parent=node['parent'];nested=False
        while parent is not None:
            if nodes[parent]['name']==root_name:nested=True;break
            parent=nodes[parent]['parent']
        if not nested:roots.append(i)
    assert roots,'encoder root missing'
    counts=Counter()
    def visit(i):
        node=nodes[i];exclusive=node['samples']-sum(nodes[j]['samples'] for j in node['children'])
        assert exclusive>=0,(node,exclusive)
        counts[node['name']]+=exclusive
        for child in node['children']:visit(child)
    for root in roots:visit(root)
    total=sum(nodes[root]['samples'] for root in roots);assert sum(counts.values())==total
    return {'encoder_samples':total,'exclusive_samples':dict(counts.most_common()),
            'exclusive_fraction':{name:n/total for name,n in counts.most_common() if n},
            'scope':'Exclusive top-of-stack samples only inside the named encoder roots; child samples subtracted once. Root totals include called helpers. These are sampled shares, not exact phase elapsed times.'}

def derive(root):
    return {'scope':'Separate five-second1ms sampling. Compiler code boundaries differ, so shares guide optimization and are not exact matched phase durations. All diagnostic outputs are verified independently of primary timing.',
            'groups':{folder:{actor:one(root/folder/actor/'sample.txt',name)
                             for actor,name in [('mech-selected','mech_compact_closed_v2'),('libdeflate','libdeflate_deflate_compress')]}
                      for folder in ['runtime-sampling','runtime-sampling-small']}}

if __name__=='__main__':
    import sys
    root=Path(sys.argv[1]);out=Path(sys.argv[2]);out.write_text(json.dumps(derive(root),indent=2)+'\n')
