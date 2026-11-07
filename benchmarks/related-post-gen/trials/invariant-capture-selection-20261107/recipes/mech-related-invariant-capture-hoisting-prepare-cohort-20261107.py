from pathlib import Path
import copy, hashlib, json

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R/'results'
base = Path('/private/tmp/mech-related-movable-capture-reads-cohort-20261107-v3')
P = Path('/private/tmp/mech-related-invariant-capture-hoisting-cohort-20261107-v2')
P.mkdir(exist_ok=True)
data = json.loads((base/'latency-config.json').read_text())
old = {v['name']: v for v in data['implementations']}
keep = ['rust', 'rust-preinitialized', 'rust-mech-control-streaming', 'rust-mech-control-u32-casts',
        'rust-mech-control-u8-ids-u32', 'mech-ir242-reuse-4', 'mech-ir247-reuse-4', 'mech-ir242-invariant-reads-4']
data['implementations'] = [copy.deepcopy(old[n]) for n in keep]
new = [
    ('mech-ir249-reuse-4', '/private/tmp/mech-related-invariant-capture-integration-runners-20261107-v1', 'reuse-4', 249),
    ('mech-ir250-reuse-4', '/private/tmp/mech-related-invariant-capture-hoisting-runners-20261107-v2', 'reuse-4', 250),
    ('mech-ir250-reads-4', '/private/tmp/mech-related-invariant-capture-hoisting-runners-20261107-v2', 'reads-4', 250),
    ('mech-ir250-partial-4', '/private/tmp/mech-related-invariant-capture-hoisting-runners-20261107-v2', 'partial-4', 250),
]
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
for name, root, source, epoch in new:
    p = Path(root)/source
    assert json.loads((p/'native-link-audit.json').read_text())['status'] == 'passed'
    assert json.loads((p/'edge-run.json').read_text())['exit'] == 0
    assert json.loads((p/'build/metadata.json').read_text())['typed_ir_version'] == epoch
    v = copy.deepcopy(old['mech-ir248-reuse-4'])
    v.update(name=name, category=f'IR{epoch}_invariant_captures_{source}',
             executable=str(p/'runner-aot'), source=str(p/'build/source.mec'),
             receipt=str(p/'runner-aot.build.json'))
    for key in ['executable', 'source', 'receipt']:
        v[key+'_sha256'] = sha(v[key])
    v['executable_file_bytes'] = Path(v['executable']).stat().st_size
    data['implementations'].append(v)
latency = D/'comparison-native-invariant-captures-randomized-20261107-5000-m3.json'
data['authoritative_latency_comparison'] = str(latency)
data['resource_reference'] = 'rust-mech-control-u8-ids-u32-instrumented'
data['whole_process_resource_reference'] = 'rust-mech-control-u32-casts-instrumented'
(P/'latency-config.json').write_text(json.dumps(data,indent=2)+'\n')
(P/'new-variant-names.json').write_text(json.dumps([n for n,_,_,_ in new],indent=2)+'\n')
print(json.dumps(dict(config=str(P/'latency-config.json'), variants=len(data['implementations']), new_variants=4)))
