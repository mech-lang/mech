from pathlib import Path
import copy,hashlib,json

R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');D=R/'results'
P=Path('/private/tmp/mech-related-selected-ranges-cohort-20261107-v1');P.mkdir()
base=Path('/private/tmp/mech-related-invariant-capture-hoisting-cohort-20261107-v2')
data=json.loads((base/'latency-config.json').read_text());old={x['name']:x for x in data['implementations']}
keep=['rust','rust-preinitialized','rust-mech-control-streaming','rust-mech-control-u32-casts','rust-mech-control-u8-ids-u32','mech-ir242-reuse-4','mech-ir250-reuse-4']
data['implementations']=[copy.deepcopy(old[name])for name in keep]
root=Path('/private/tmp/mech-related-selected-range-integration-runners-20261107-v1')
new=[(f'mech-ir252-{name}',root/name,8 if name.endswith('-8')else 4)for name in ['reuse-4','reads-4','partial-4','compact-reuse-4','compact-reuse-8']]
new += [(f'mech-ir{epoch}-borrowed-json-4',Path('/private/tmp/mech-related-selected-ranges-borrowed-json-runners-20261107-v1')/str(epoch),4)for epoch in [242,252]]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
for name,p,workers in new:
 assert json.loads((p/'native-link-audit.json').read_text())['status']=='passed'
 assert json.loads((p/'edge-run.json').read_text())['exit']==0
 v=copy.deepcopy(old['mech-ir250-reuse-4']);v.update(name=name,category=name.replace('mech-',''),executable=str(p/'runner-aot'),source=str(p/'build/source.mec'),receipt=str(p/'runner-aot.build.json'),configured_workers=workers,query_partitions=workers)
 for key in ['executable','source','receipt']:v[key+'_sha256']=sha(v[key])
 v['executable_file_bytes']=Path(v['executable']).stat().st_size
 data['implementations'].append(v)
data['authoritative_latency_comparison']=str(D/'comparison-native-selected-ranges-randomized-20261107-5000-m3.json')
(P/'latency-config.json').write_text(json.dumps(data,indent=2)+'\n')
(P/'new-variant-names.json').write_text(json.dumps([x[0]for x in new],indent=2)+'\n')
assert len(data['implementations'])==14
print('latency config variants14, new7')
