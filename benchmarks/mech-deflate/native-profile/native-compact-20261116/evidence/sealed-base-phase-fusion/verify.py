"""Verify archive bindings, unchanged native artifacts, parser identities and measured results."""
from pathlib import Path
from hashlib import sha256
from statistics import median
import gzip,json
H=Path(__file__).resolve().parent;P=H.parents[1];read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
for name,pin in read(H/'files.json')['files'].items():assert sha(H/name)==pin,name
for name,pin in read(H/'reused-runtime-evidence.json')['files'].items():assert sha(P/name)==pin,name
prior=H.parent/'encoder-size-gap/current-matcher-guard-facts';selected=read(prior/'candidate.json');identity=read(H/'exact-artifacts.json');keys={'object':'program.o','metadata':'metadata.json','bytecode':'program.mecb'}
assert identity['source_sha256']==sha(P/'program.mec')==selected['source_sha256']
assert identity['artifacts']=={k:selected['artifact_sha256'][v] for k,v in keys.items()}
for k,n in keys.items():assert sha256(gzip.decompress((prior/'guards-profiled'/(n+'.gz')).read_bytes())).hexdigest()==identity['artifacts'][k]
assert identity['worker_sha256']==selected['worker_sha256']==sha256(gzip.decompress((prior/'guards-profiled/mech-worker.gz').read_bytes())).hexdigest()
link=json.loads(gzip.decompress((H/'emitted/bound/runtime.json.gz').read_bytes()));assert link['worker_sha256']==identity['worker_sha256'] and link['object_sha256']==identity['artifacts']['object']
replay=json.loads(gzip.decompress((prior/'full/full-replay.json.gz').read_bytes()))
for order in replay['runs']:
 assert order['case_count']==81
 for peer,floor in [('roc',81),('libdeflate',62)]:
  for metric in ['point_estimate_wins','precision_supported_wins']:assert order['peers'][peer][metric]['mech-experiment']>=floor
assert read(H/'combined-tests/all-tests.json')['exit_code']==0 and '539 passed' in read(H/'combined-tests/all-tests.json')['summary'][0]
assert read(H/'compiler-build.json')['compiler_sha256']==identity['compiler_sha256']
source_build=read(H/'compiler-build.json')
assert sha(H/'variants/combined.rs')==source_build['source_files']['src/syntax/src/document/parser/canonical/base/continuation.rs']
assert sha(H/'variants/fused_tests.rs')==source_build['source_files']['src/syntax/src/document/parser/canonical/base/continuation/fused_tests.rs']
assert sha(H/'variants/continuation-before.rs')==read(H/'fusion-source.json')['original_sha256']
reference=json.loads(gzip.decompress((H/'control/run-0.0.json.gz').read_bytes()));fields=['source_sha256','source_bytes','tree_sha256','parser_steps','events']
assert reference['tree_sha256']=='bfa476cac02c0fa8ab487211e366b34cf0c27b13998958a358317222cedc0056' and reference['parser_steps']==3967302 and reference['events']==718353
for folder in ['screen','screen-fused-bounds']:
 report=read(H/folder/'results.json');assert len(report['records'])==8
 for row in report['records']:
  for actor,observation in row.items():assert all(observation[k]==reference[k] for k in fields)
factorial=read(H/'factorial/results.json');assert len(factorial['observations'])==8
for row in factorial['observations']:
 assert len(row)==4
 for observation in row.values():assert all(observation[k]==reference[k] for k in fields)
for candidate,control in [('fused','control'),('bounds-only','control'),('fused-bounds','control'),('fused-bounds','bounds-only')]:
 computed=median(row[candidate]['parse_seconds']-row[control]['parse_seconds'] for row in factorial['observations']);assert abs(computed-factorial['comparisons'][candidate+'_vs_'+control]['paired_median_seconds'])<1e-12
build=json.loads(gzip.decompress((H/'matched-builds/comparison.json.gz').read_bytes()));summary=read(H/'matched-builds/summary.json');assert len(build['observations'])==8
candidate='mech-sealed-base-fusion';control='mech-control-reference';boundary='source_through_link'
assert set(build['medians'])=={candidate,control,'roc','libdeflate'}
for role in build['medians']:
 assert abs(median(row[role]['seconds'][boundary] for row in build['observations'])-summary['medians'][role][boundary])<1e-12
 for row in build['observations']:assert row[role]['seconds']['source_through_strip']>=row[role]['seconds'][boundary]>=row[role]['seconds']['source']
diffs=[row[candidate]['seconds'][boundary]-row[control]['seconds'][boundary] for row in build['observations']];assert abs(median(diffs)-summary['paired_median_seconds'])<1e-12
plan=read(H/'matched-builds/plan.json');assert plan['actors'][candidate]['compiler_sha256']==identity['compiler_sha256']
# Explicit mapping: timing harness calls the native object 'encoder'.
for role in [candidate,control]:assert plan['actors'][role]['artifact_sha256']=={'encoder':identity['artifacts']['object'],'metadata':identity['artifacts']['metadata'],'bytecode':identity['artifacts']['bytecode']}
portable=read(H/'portable-source-replay/source-reproduction.json')
assert {row['mode'] for row in portable['variants']}=={'normal','checked'}
for row in portable['variants']:assert row['compiler_sha256']==identity['compiler_sha256'] and row['artifacts']==selected['artifact_sha256']
profile=read(H/'current-runtime-profiles/summary.json')
for name,row in profile['profiles'].items():
 for actor in ['mech','libdeflate']:
  p=H/'current-runtime-profiles'/name/actor;result=read(p/'result.json');assert row[actor]['encoder_stack_samples']>=1000 and row[actor]['worker_sha256']==result['worker_sha256'];assert sha256(gzip.decompress((p/'sample.txt.gz').read_bytes())).hexdigest()==result['sample_sha256'];assert len(result['verified_calls'])==2
 assert row['mech']['worker_sha256']==identity['worker_sha256']
print('Verified 539 tests, exact source/tree/IDs/work, four isolated parser variants, unchanged native object/metadata/bytecode/worker with prior 81/62 qualification, eight complete-build trials per actor, and four fresh selected-runtime profiles.')

# Check the selected target separately: debug inspection deliberately retains
# otherwise-refused general bodies and is not a shipping whole artifact.
from struct import unpack_from
def target_body(data,metadata):
    symbol='mech_compact_numeric_475_vector'
    rows=[r for p in metadata['numeric_phases'] for r in [p,*[p.get(k,{}) for k in ['vector_variant','no_growth_variant','composed_variant','grouped_histogram']]] if r.get('symbol')==symbol]
    assert len(rows)==1
    row=rows[0];at=32;section_id=1;text=None;table=None
    assert unpack_from('<II',data)==(0xfeedfacf,0x0100000c)
    for _ in range(unpack_from('<I',data,16)[0]):
        command,length=unpack_from('<II',data,at)
        if command==0x19:
            for i in range(unpack_from('<I',data,at+64)[0]):
                entry=at+72+80*i;name,segment,pc,size,offset=unpack_from('<16s16sQQI',data,entry)
                if name.rstrip(b'\0')==b'__text' and segment.rstrip(b'\0')==b'__TEXT':text=(section_id,pc,offset)
                section_id+=1
        elif command==2:table=unpack_from('<4I',data,at+8)
        at+=length
    symbols,n,strings,_=table
    for i in range(n):
        name,kind,section,_,pc=unpack_from('<IBBHQ',data,symbols+16*i);start=strings+name;end=data.index(b'\0',start);name=data[start:end].decode()
        if name=='_'+symbol:
            assert section==text[0];offset=text[2]+pc-text[1];return row,data[offset:offset+row['code_bytes']]
    raise AssertionError(symbol)
debug=H/'collector-inspection';check=read(debug/'body-check.json')
old_meta=json.loads(gzip.decompress((prior/'guards-profiled/metadata.json.gz').read_bytes()));debug_meta=json.loads(gzip.decompress((debug/'frontend/metadata.json.gz').read_bytes()))
old_row,old_body=target_body(gzip.decompress((prior/'guards-profiled/program.o.gz').read_bytes()),old_meta)
debug_row,debug_body=target_body(gzip.decompress((debug/'frontend/program.o.gz').read_bytes()),debug_meta)
assert old_row==debug_row and old_body==debug_body and len(old_body)==6092
assert sha256(old_body).hexdigest()==check['selected_body_sha256']==check['observed_body_sha256']
assert read(H/'collector-inspection-vcode/result.json')['status'].startswith('Diagnostic VCode capture failed')
print('Verified exact collector body binding for CLIF-only inspection; the failed optional VCode capture is not used.')
