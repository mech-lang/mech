import gzip, hashlib, importlib.util, json, pathlib, re, shutil, subprocess, sys

S=pathlib.Path('/private/tmp/mech-related-llvm-object-backend-compiler-20261107')
Q=pathlib.Path('/private/tmp/mech-related-llvm-typed-ssa-qualification-20261107-v6')
U=pathlib.Path('/private/tmp/mech-related-llvm-typed-ssa-default-equivalence-20261107-v6')
O=pathlib.Path('/private/tmp/mech-related-llvm-typed-ssa-opt-20261107-v6')
R=pathlib.Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
P=S/'benchmarks/related-post-gen/trials/llvm-typed-ssa-20261107'
P.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git','-c','gc.auto=0',*args],cwd=S,text=True).strip()
source=git('rev-parse','HEAD')
assert source=='ca5366746ee5c3c8fcf0579519606a1393bf3d98'
assert git('status','--porcelain')==''
summary=json.loads((Q/'summary.json').read_text())
assert summary['source_inventory_exact']and summary['inherited_failures_exact']
assert summary['full_GPU_summary']==['test result: FAILED. 896 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 236.04s']
assert not summary['LLVM_program_qualified']and not summary['performance_measured']
ci=json.loads((Q/'ci-selected-inventory-audit.json').read_text());assert ci['expected']==ci['listed']==ci['executed_passed']==399 and ci['all_names_match']
native=json.loads((Q/'native-semantics-tests.json').read_text());assert native['exit']==0 and '3 passed; 0 failed' in native['stdout']
equivalence=json.loads((U/'equivalence.json').read_text());assert all(x['all13_bundle_files_byte_exact']and len(x['files'])==13 for x in equivalence.values())
spec=importlib.util.spec_from_file_location('compiler_helper',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
inventory=json.loads((O/'source-inventory.json').read_text());assert h.source_inventory(S)==inventory['files']

upstream='5f5740bd850c1a6875dac0f00ae392943079801c'
paths=git('diff','--name-only','b6a42fde80dbc011dfefa9de5d9659be334411b0',upstream).splitlines()
assert paths and all(p.startswith('docs/')or p=='benchmarks/mech-deflate/experiments.json' for p in paths)
up=dict(latest_fetched_upstream=upstream,integrated_production_code='b6a42fde80dbc011dfefa9de5d9659be334411b0',changed_paths=paths,compiler_source_manifests_lock_flags_CI_unchanged=True,production_compiler_already_integrated=True,latest_scalar_cost_candidate_rejected=True,history_merge_required=False)
(Q/'upstream-code-equivalence.json').write_text(json.dumps(up,indent=2)+'\n')

files={}
def add(path,destination,compress=False):
    path=pathlib.Path(path);original=path.read_bytes();encoded=gzip.compress(original,mtime=0)if compress else original
    dest=P/destination;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(encoded)
    files[destination]=dict(sha256=sha(encoded),bytes=len(encoded),decoded_sha256=sha(original),decoded_bytes=len(original),gzip=compress)

for path in Q.rglob('*'):
    if not path.is_file()or path.name in ['gpu-tests','compiler-llvm-ir']:continue
    rel=str(path.relative_to(Q));compress=path.suffix in ['.ll','.rs','.o']
    add(path,'qualification/'+rel+('.gz'if compress else ''),compress)
for path in U.glob('*'):
    if path.is_file():add(path,'default-equivalence/'+path.name)
for name in ['after-4','reuse-4']:
    add(U/name/'compile.json','default-equivalence/'+name+'-compile.json')
for v in [1,2,3,4,5,6]:
    base=pathlib.Path(f'/private/tmp/mech-related-llvm-typed-ssa-opt-20261107-v{v}')
    for name in ['build.json','source-inventory.json','mech_gpu.build.json','mech_gpu.stdout.log','mech_gpu.stderr.log']:
        p=base/name
        if p.exists():add(p,f'builds/v{v}/'+name)
for v in [3,4,5]:
    base=pathlib.Path(f'/private/tmp/mech-related-llvm-typed-ssa-qualification-20261107-v{v}')
    for path in base.glob('*.json'):
        add(path,f'development/v{v}/'+path.name)
    for path in base.glob('*.rs'):add(path,f'development/v{v}/'+path.name+'.gz',True)
for v in [3,4,5,6]:
    recipe=pathlib.Path(f'/private/tmp/mech-related-llvm-typed-ssa-qualification-20261107-v{v}.py')
    add(recipe,f'recipes/qualify-v{v}.py')
add('/private/tmp/mech-related-publish-llvm-typed-ssa-20261107.py','recipes/publish.py')
for name in ['build-compiler.py','build-dependent-compiler.py','build-host.py']:add(R/name,'recipes/'+name)
changed=git('diff-tree','--no-commit-id','--name-only','-r',source).splitlines()
for name in changed:
    path=S/name;compress=path.suffix=='.rs'
    add(path,'source/'+name+('.gz'if compress else ''),compress)
for name in ['machine_facts.py','test_machine_facts.py']:add(S/'hosts/gpu/src/native_artifact/program/llvm_backend'/name,'machine-reader/'+name)
toolchain={}
for tool in ['opt','llc','clang']:
    p=pathlib.Path('/opt/homebrew/opt/llvm/bin')/tool
    r=subprocess.run([str(p),'--version'],capture_output=True,text=True);assert r.returncode==0
    toolchain[tool]=dict(path=str(p),sha256=sha(p.read_bytes()),version=r.stdout)
source_receipt=dict(source_commit=source,parent=git('rev-parse',source+'^'),branch=git('branch','--show-current'),compiler_identity='IR264/work115/ABI41/image9 default path only',LLVM_image_identity_not_implemented=True,source_inventory_sha256=inventory['sha256'],toolchain=toolchain,selected_compiler='4cc266ae0d05c69432a4c15ffe99d5e586685647',selected_changed=False,upstream=up)
receipt=Q/'source-and-toolchain.json';receipt.write_text(json.dumps(source_receipt,indent=2)+'\n');add(receipt,'source-and-toolchain.json')

readme='''# Typed native SSA to LLVM

Compiler IR lowering and machine compilation are implemented; no LLVM Mech
executable image is qualified or timed. The selected compiler remains IR242.

Both benchmark sources compile through the owned typed SSA recorder and LLVM O3.
The closed machine audit accepts55 original and53 mask-reuse definitions, with
actual code/constants/frames. Mask reuse has555436B of machine code/constants,
root656B and largest function frame912B. These are size observations only.

Three real native helper tests pass, including10000 integer/Boolean cases and
10000 ABI cases. Full GPU896pass plus exactly two inherited failures; three LLVM
native tests are explicitly run separately rather than counted as default passes.
All399 CI-selected names are actually listed and executed. Both default bundles
retain all13 files byte-for-byte, including the complete Object and descriptor.

The initial verifier API error, stale SDK setup, C remainder-name conflict and
successive unsupported ABI/rotate/minmax refusals are retained. No failed attempt
is relabeled passed. Unsupported lowering fails explicitly.

Remaining: complete backend/toolchain/target identity, final machine admission,
same-Object assembly, independent loader and complete source/finite/ownership/
publication/runtime/old-Object/forgery qualification. The original descriptor is
never linked beneath this machine code. No resource or latency result is claimed.

Latest fetched DEFLATE5f5740 changes evidence/design only; production remains
equivalent to the already integrated b6a42. Its scalar low-word candidate was
rejected. The exact path/code-equivalence receipt is included.

Run `python3 verify.py` to verify all files and rederive machine metadata.
Executables stay local. LLVM IR and Rust source evidence are losslessly gzipped.
'''
temp=Q/'packet-README.md';temp.write_text(readme);add(temp,'README.md')
manifest=dict(schema='mech.related-post-gen.llvm-typed-ssa-evidence.v1',source_commit=source,files=files,selected_changed=False,LLVM_program_qualified=False,performance_measured=False)
(P/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
verifier=r'''import gzip,hashlib,importlib.util,json,pathlib,re
P=pathlib.Path(__file__).parent;m=json.loads((P/'manifest.json').read_text());read=lambda n:gzip.decompress((P/n).read_bytes())if n.endswith('.gz')else(P/n).read_bytes()
for n,v in m['files'].items():
 b=(P/n).read_bytes();assert len(b)==v['bytes']and hashlib.sha256(b).hexdigest()==v['sha256'],n
 d=gzip.decompress(b)if v['gzip']else b;assert len(d)==v['decoded_bytes']and hashlib.sha256(d).hexdigest()==v['decoded_sha256'],n
spec=importlib.util.spec_from_file_location('facts',P/'machine-reader/machine_facts.py');f=importlib.util.module_from_spec(spec);spec.loader.exec_module(f)
for name,count in [('after-4',55),('reuse-4',53)]:
 base='qualification/'+name+'/';names=json.loads(read(base+'definitions.json'));assert len(names)==count
 source=read(base+'native.ll.gz').decode();assert re.findall(r'^define .*? @"([^"\n]+)"\(',source,re.M)==names
 facts=f.extract(read(base+'native.o.gz'),read(base+'remarks.yaml').decode(),names);assert facts==json.loads(read(base+'machine-facts.json'))
 summary=json.loads(read('qualification/summary.json'));assert summary[name]['facts']==facts
assert '896 passed; 2 failed; 3 ignored' in summary['full_GPU_summary'][0]and summary['inherited_failures_exact']and summary['source_inventory_exact']
native=json.loads(read('qualification/native-semantics-tests.json'));assert native['exit']==0 and '3 passed; 0 failed' in native['stdout']
ci=json.loads(read('qualification/ci-selected-inventory-audit.json'));assert ci['expected']==ci['listed']==ci['executed_passed']==399 and ci['all_names_match']
executed=json.loads(read('qualification/ci-selected-execute.json'));passed=[l[len('test '):].split(' ... ')[0]for l in executed['stdout'].splitlines()if l.startswith('test ')and l.endswith(' ... ok')];assert sorted(passed)==sorted(ci['names'])
eq=json.loads(read('default-equivalence/equivalence.json'));assert len(eq)==2 and all(x['all13_bundle_files_byte_exact']and len(x['files'])==13 for x in eq.values())
assert not m['selected_changed']and not m['LLVM_program_qualified']and not m['performance_measured']
print(json.dumps(dict(files=len(m['files']),native_helper_tests=3,GPU_passed=896,inherited_failures=2,actual_CI_passed=399,original_definitions=55,reuse_definitions=53,default_bundle_files=26,LLVM_program_qualified=False,performance_measured=False)))
'''
(P/'verify.py').write_text(verifier)
result=subprocess.run([sys.executable,P/'verify.py'],capture_output=True,text=True);print(result.stdout,flush=True);assert result.returncode==0,result.stderr
assert h.source_inventory(S)==inventory['files'],'evidence changed compiler source inventory'
git('add',str(P.relative_to(S)));tracked=set(git('ls-files').splitlines());assert all(str((P/n).relative_to(S)) in tracked for n in files);print(git('commit','-m','Record native LLVM SSA compilation and semantic qualification evidence'),flush=True)
publication=dict(source_commit=source,evidence_commit=git('rev-parse','HEAD'),packet=str(P.relative_to(S)),files=len(files),hashes_verified=True,actual_machine_facts_rederived=True,source_inventory_exact=True,selected_changed=False,LLVM_program_qualified=False,performance_measured=False,pushed=False)
path=pathlib.Path('/private/tmp/mech-related-llvm-typed-ssa-publication-20261107.json');path.write_text(json.dumps(publication,indent=2)+'\n');print(json.dumps(publication),flush=True)
