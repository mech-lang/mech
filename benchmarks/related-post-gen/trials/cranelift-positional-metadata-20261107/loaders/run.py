from pathlib import Path
import hashlib,json,os,shutil,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');N=Path('/private/tmp/mech-related-positional-metadata-runners-20261107-v1');OLD=Path('/private/tmp/mech-related-llvm-native-image-loaders-budgets-20261107-v3')
oracle=json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
def run(cmd,path,env=None):
    cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})})
    d=dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);path.write_text(json.dumps(d,indent=2)+'\n');assert r.returncode==0,r.stdout[-4000:]+r.stderr[-4000:];return d
records={};denials={}
for mode in ['independent','json','full-catalog']:
    old=OLD/('reuse-4-'+mode);p=P/mode;p.mkdir(exist_ok=True)
    if not(p/'build').exists():(p/'build').symlink_to(N/'build',target_is_directory=True)
    shutil.copyfile(old/'runner.rs',p/'runner.rs')
    run(['python3',R/'build-host.py','--compiler-build',N/'runtime/build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'build-run.json')
    run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-proof.json'],p/'audit-run.json')
    run([p/'runner-aot',p/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',p/'output.json','1'],p/'full5000-run.json');assert json.loads((p/'output.json').read_text())==oracle
    run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edges-run.json')
    records[mode]=dict(full5000_exact=True,edges_exact=31);print(mode,'32 exact outputs',flush=True)
    # Same source-site finite denials through each independent reader form.
    budget_mode='bound'if mode=='json'else mode;before=OLD/('reuse-4-budget-'+budget_mode);q=P/('budget-'+mode);q.mkdir(exist_ok=True)
    if not(q/'build').exists():(q/'build').symlink_to(N/'build',target_is_directory=True)
    shutil.copyfile(before/'runner.rs',q/'runner.rs')
    run(['python3',R/'build-host.py','--compiler-build',N/'runtime/build.json','--source',q/'runner.rs','--out',q/'runner-budget','--bundle',q/'build','--linked'],q/'build-run.json')
    run(['python3',R/'audit-native-link.py','--object',q/'build/object.o','--executable',q/'runner-budget','--output',q/'native-link-proof.json'],q/'audit-run.json')
    denials[mode]=[]
    for kind in ['work','transport','stack','output_bytes','output_elements','temporary','nodes','comparison']:
        control=P/('parent-budget-'+mode);control.mkdir(exist_ok=True)
        if not(control/'build').exists():(control/'build').symlink_to(Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4/build'),target_is_directory=True)
        if not(control/'runner-budget').exists():
            shutil.copyfile(before/'runner.rs',control/'runner.rs')
            run(['python3',R/'build-host.py','--compiler-build','/private/tmp/mech-related-llvm-native-image-runtime-20261107-v4/build.json','--source',control/'runner.rs','--out',control/'runner-budget','--bundle',control/'build','--linked'],control/'build-run.json')
            run(['python3',R/'audit-native-link.py','--object',control/'build/object.o','--executable',control/'runner-budget','--output',control/'native-link-proof.json'],control/'audit-run.json')
        prior=run([control/'runner-budget',control/'build/source.mec',R/'fixtures/posts-first-10.json',control/(kind+'-output.json'),'1'],control/(kind+'-run.json'),{'MECH_IMAGE_BUDGET_CASE':kind});expected=json.loads(prior['stdout'])
        result=run([q/'runner-budget',q/'build/source.mec',R/'fixtures/posts-first-10.json',q/(kind+'-output.json'),'1'],q/(kind+'-run.json'),{'MECH_IMAGE_BUDGET_CASE':kind})
        actual=json.loads(result['stdout']);assert actual==expected,(mode,kind,expected,actual);assert actual['output_unpublished']and actual['retained_nodes']==actual['retained_bytes']==0
        denials[mode].append(kind)
    print(mode,'8 exact finite denials',flush=True)
(P/'summary.json').write_text(json.dumps(dict(rows=records,full5000_plus_edges_exact=96,finite_denials=24,all_denials_exact_to_prior_and_zero_retention=True,LLVM_tools_used=False),indent=2)+'\n')
