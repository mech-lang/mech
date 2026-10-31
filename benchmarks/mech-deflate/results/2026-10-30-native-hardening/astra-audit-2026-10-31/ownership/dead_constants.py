"""Read-only R56 plan audit. Mirrors the existing semantic read-role visitor.
Fails closed for action/iteration/collection variants absent from this pinned plan.
This reports static non-use, not an execution/allocation optimization result.
"""
import collections, hashlib, json
from pathlib import Path
P = Path('/private/tmp/mech-buffer-liveness-round56-20261031/linked-primary/after/descriptor.json')
D = json.loads(P.read_text())['descriptor']; p = D['plan']; constants = p['constants']
assert hashlib.sha256(P.read_bytes()).hexdigest() == '1b4dbdd2fb0c1f9f51ae7e9dd0bc931bbc2804e6cb949653461d6348c31615c9'
uses = collections.defaultdict(list)
def read(value,role):
    assert isinstance(value,int) and 0 <= value < len(p['values'])
    uses[value].append(role)
def variant(value):
    assert isinstance(value,dict) and len(value)==1,value
    return next(iter(value.items()))
for site, block in enumerate(p['blocks']):
    action, body=variant(block['action'])
    if action=='Scalar':
        if body!='None':
            scalar,args=variant(body)
            if scalar=='Primitive':
                for value in args['operands']:read(value,['Action',site])
            elif scalar=='Copy':read(args['source'],['Action',site])
            else:raise ValueError(scalar)
    elif action=='LibraryCall':
        for value in body['operands']:read(value,['Action',site])
    elif action=='Structural':
        read(body['source'],['Action',site]); observation=body['observation']
        if isinstance(observation,dict) and 'StringEqual' in observation:
            read(observation['StringEqual']['peer'],['Action',site])
        else:assert observation in ['CanonicalValue','U64'] or 'EnumOrdinal' in observation,observation
    elif action=='CollectionPush':read(body['value'],['Action',site])
    elif action=='PrepareU64Range':
        generator=p['iterations'][body['iteration']]['Collection']
        cursor=generator['source']['U64Range']
        for value in cursor['operands']:read(value,['Action',site])
    else:assert action in ['Initialize','Latch','CollectionBegin','CollectionElement','CollectionFinish'],action
    term,body=variant(block['terminator'])
    if term=='Scalar' and isinstance(body,dict):
        scalar,args=variant(body)
        if scalar=='Branch':read(args['condition'],['Terminator',site])
        elif scalar=='LiteralBranch':
            for key in ['left','right']:read(args[key],['Terminator',site])
        else:assert scalar=='Jump',scalar
    else:assert term in ['TailHeader','Header'] or body in ['Reject','Complete','IntegrityFailure'],(term,body)
for i,loop in enumerate(p['iterations']):
    tag,body=variant(loop)
    if tag=='Collection':
        source,value=variant(body['source'])
        if source=='Value':read(value,['CollectionSource',i])
        elif source=='U64Range':read(value['output'],['CollectionSource',i])
        else:raise ValueError(source)
    elif tag in ['Tail','Iteration']:
        for name,role in [('seed','LoopSeed'),('next','LoopNext')]+([('returned','LoopReturned')] if tag=='Tail' else []):read(body[name],[role,i])
    else:raise ValueError(tag)
for i,output in enumerate(p['outputs']):read(output['value'],['Output',i])
for i,effect in enumerate(p['effects']):read(effect['value'],['Effect',i])
assert all(c['result']=='Materialized' and c['string_span_projection'] is None and c['string_element_map_projection'] is None for c in p['collections'])
assert p['observations']==[] and p['effects']==[] and p['activation_nodes']==[]
assert D['scalar_aggregates']['constants']==[]
assert all(c['value']==i+len(p['inputs']) for i,c in enumerate(constants))
dead=[c for c in constants if not uses[c['value']]]; deadids={c['value'] for c in dead}
# Template source provenance uses artifact ConstantId, not native value IDs.
# These are setup reads of immutable descriptor constants, not arena imports.
# Verify even this broader inventory has no dead candidates.
template_refs=[]
def source_constant(node):
    if isinstance(node,dict):
        if set(node)=={'Constant'}:
            assert isinstance(node['Constant'],int)
            yield node['Constant']
        else:
            for value in node.values():yield from source_constant(value)
    elif isinstance(node,list):
        for value in node:yield from source_constant(value)
for site,call in enumerate(D['calls']):
    if call is None:continue
    _,selected=variant(call['template']['selected'])
    for ordinal in source_constant(selected['sources']):
        value=constants[ordinal]['value']
        template_refs.append({'site':site,'constant_ordinal':ordinal,'value':value})
assert not [r for r in template_refs if r['value'] in deadids]
# Root-scope constants are not in any lexical retirement row.
assert all(p['value_scopes'][c['value']]=={'Immutable':{'iteration':None}} for c in constants)
ret=D['retirements']; assert isinstance(ret,dict); words=ret['words']; prefix=ret['iterations']*3+1
assert words[0]==prefix
retired=set(words[prefix:])
assert not deadids & retired
result={
 'descriptor_sha256':hashlib.sha256(P.read_bytes()).hexdigest(),
 'scope':'Static exhaustive original-semantic-use inventory for the pinned descriptor; no compiler or compressor executed.',
 'constants':len(constants),'constant_scalar_kind_counts':dict(collections.Counter(p['values'][c['value']]['scalar']['kind'] if p['values'][c['value']]['scalar'] else 'Managed' for c in constants)),
 'all_block_actions':dict(collections.Counter(next(iter(b['action'])) for b in p['blocks'])),
 'all_iterations':dict(collections.Counter(next(iter(x))for x in p['iterations'])),
 'dead_constant_count':len(dead),'dead_kind_counts':dict(collections.Counter(next(iter(c['constant']['data']))for c in dead)),
 'dead_native_value_ids':sorted(deadids),
 'dead_artifact_constant_ordinals':[i for i,c in enumerate(constants)if c['value'] in deadids],
 'dead_constants':dead,
 'checked_template_source_references':len(template_refs),
 'template_source_dead_intersection':[],
 'lexical_retirement_dead_intersection':[],
 'potential_import_requests_avoided':2*len(dead),
 'potential_request_bytes_avoided':(104+112)*len(dead),
 'prediction_caveat':'This is a mechanism ceiling, not a measured allocation reduction. Any new claim storage may add requests. Preserve all original current-account admission and work charges.',
 'logical_setup_envelopes_must_remain':sum(D['constant_bytes'][i] for i,c in enumerate(constants) if c['value'] in deadids),
}
Path('/private/tmp/mech-r56-owner-audit-20261031/dead-constants.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items()if k not in ['dead_constants','dead_native_value_ids','dead_artifact_constant_ordinals']},indent=2))
