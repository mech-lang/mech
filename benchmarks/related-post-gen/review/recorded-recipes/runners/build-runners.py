from pathlib import Path
import copy,hashlib,json,shutil,subprocess,time
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-native-fixed-frame-arenas-opt-20261106-v2')
B=Path('/private/tmp/mech-related-native-frozen-type-descriptors-runners-20261106-v2')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text())
def run(c,p):
    c=list(map(str,c));t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True)
    p.write_text(json.dumps(dict(command=c,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
    assert r.returncode==0,r.stdout+r.stderr
shutil.copyfile('/private/tmp/mech-related-native-numeric-collections-opt-20261106-v6/compiler-facts.rs',O/'compiler-facts.rs')
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',O/'compiler-facts.rs','--out',O/'compiler'],O/'compiler-build-run.json')
config=read(B/'comparison-config.json');rows=[]
for row in config['implementations']:
    row=copy.deepcopy(row);name=row['name']
    if name.startswith('mech-') and not name.startswith(('mech-before-','mech-after-','mech-u8-ids-u32-')):continue
    if name.startswith('mech-after-'):row['name']=name.replace('mech-after-','mech-parent-')
    if name.startswith('mech-u8-ids-u32-'):row['name']=name.replace('mech-u8-ids-u32-','mech-compact-before-')
    rows.append(row)
assert len(rows)==41 and len({x['name']for x in rows})==41
config['implementations']=rows;proof={}
for w in [4,8]:
    for label in ['after','compact']:
        p=U/f'{label}-{w}';p.mkdir();prior=B/f'{label}-{w}'
        run([O/'compiler',prior/'build/source.mec',p/'build'],p/'compile.json')
        files={f.name:sha(f)for f in (p/'build').iterdir()if f.is_file()};assert len(files)==13
        for file in ['source.mec','artifact.mecb','catalog.rs','catalog-profile.json','map-facts.txt','frame-summary.json']:assert files[file]==sha(prior/'build'/file),file
        before=read(prior/'build/metadata.json');after=read(p/'build/metadata.json')
        assert after['typed_ir_version']==223 and after['abi_version']==40 and after['work_policy_version']==114 and after['native_descriptor_version']==8
        assert len(after['compiled_numeric_functions'])==53
        removed=0
        for key in ['compiled_numeric_write_loops','compiled_readonly_numeric_loops']:
            assert len(before[key])==len(after[key])
            for a,b in zip(before[key],after[key]):
                assert a['header']==b['header']
                assert b['capacity_guards']==a['capacity_guards']
                removed+=sum(x[1]==0 for x in a['capacity_guards'])
        folded=sum(r['cleanup']['constant_branches_removed']for r in after['compiled_numeric_functions'])
        identities=sum(r['cleanup']['neutral_operations_removed']for r in after['compiled_numeric_functions'])
        assert identities>0
        shutil.copyfile(prior/'runner.rs',p/'runner.rs')
        run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
        run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
        run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edge-run.json')
        source_row=next(x for x in rows if x['name']==(f'mech-parent-{w}'if label=='after'else f'mech-compact-before-{w}'))
        x=copy.deepcopy(source_row)
        x.update(name=f'mech-after-{w}'if label=='after'else f'mech-u8-ids-u32-{w}',executable=str(p/'runner-aot'),source=str(p/'build/source.mec'),receipt=str(p/'runner-aot.build.json'),executable_sha256=sha(p/'runner-aot'),receipt_sha256=sha(p/'runner-aot.build.json'),category='native_fixed_frame_arenas_IR223_'+label)
        config['implementations'].append(x)
        projected=sum(r['projected_child_debit_nodes']for key in ['compiled_numeric_write_loops','compiled_readonly_numeric_loops']for r in after[key])
        assert projected>0
        old_guards=sum(r['unsigned_guards']for r in before['compiled_numeric_functions'])
        new_guards=sum(r['unsigned_guards']for r in after['compiled_numeric_functions'])
        print('unsigned guards folded',old_guards,new_guards,flush=True)
        old_functions={r['header']:r for r in before['compiled_numeric_functions']}
        proof[f'{label}-{w}']=dict(files=files,source_artifact_catalog_frames_unchanged=True,numeric_functions=53,projected_child_debit_nodes=projected,before_unsigned_guards_folded=old_guards,after_unsigned_guards_folded=new_guards,constant_branches_folded=folded,neutral_operations_removed=identities,before_code_bytes=before['code_bytes'],after_code_bytes=after['code_bytes'],functions=[dict(header=f['header'],before_code_bytes=old_functions[f['header']]['code_bytes'],after_code_bytes=f['code_bytes'],before_stack_bytes=old_functions[f['header']]['stack_bytes'],after_stack_bytes=f['stack_bytes'])for f in after['compiled_numeric_functions']])
        print(label,w,'31 exact edges; constant branches folded',folded,'code',before['code_bytes'],after['code_bytes'],flush=True)
assert len(config['implementations'])==45
(U/'comparison-config.json').write_text(json.dumps(config,indent=2)+'\n')
(U/'native-code-comparison.json').write_text(json.dumps(proof,indent=2)+'\n')
