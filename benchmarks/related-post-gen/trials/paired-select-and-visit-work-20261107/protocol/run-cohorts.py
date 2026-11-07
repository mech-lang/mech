from pathlib import Path
import hashlib, importlib.util, json, os, re, subprocess, time
P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D=R/'results';S=Path('/private/tmp/mech-related-positional-metadata-compiler-20261107')
O=Path('/private/tmp/mech-related-positional-metadata-opt-20261107-v1')
def write(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
def quiet(name):
    snapshot=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
    for line in snapshot.splitlines()[1:]:
        parts=line.split(None,4)
        if len(parts)!=5 or int(parts[0])in[os.getpid(),os.getppid()]:continue
        if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|gpu-tests|one-kib|scalar|runner-(?:aot|metrics|pc|trace|phases))(?:\s|$)',parts[4]):busy.append(dict(pid=int(parts[0]),program=parts[4].split()[0]))
    assert not busy,busy
    write(P/(name+'-quiet.json'),dict(quiet=True,process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest(),no_other_own_benchmark_build_test_profile_archive=True))
def run(cmd,name):
    cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True)
    write(P/(name+'.json'),dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr))
    print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-4000:]+r.stderr[-4000:]
spec=importlib.util.spec_from_file_location('build_helper',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
sources=[]
for kind,version in [('paired-scalar-selects',3),('compact-visit-work',2)]:
    source=Path('/private/tmp/mech-related-'+kind+'-compiler-20261107')
    build=Path('/private/tmp/mech-related-'+kind+'-opt-20261107-v'+str(version))
    assert h.source_inventory(source)==json.loads((build/'source-inventory.json').read_text())['files']
    sources.append(dict(kind=kind,source=str(source),build=str(build),source_inventory_exact=True,commit=subprocess.check_output(['git','-c','gc.auto=0','rev-parse','HEAD'],cwd=source,text=True).strip()))
assert json.loads(Path('/private/tmp/mech-related-paired-scalar-selects-tests-20261107-v3/summary.json').read_text())['actual_CI_passed']==401
assert json.loads(Path('/private/tmp/mech-related-compact-visit-work-tests-20261107-v2/summary.json').read_text())['actual_CI_passed']==399
assert json.loads(Path('/private/tmp/mech-related-compact-visit-work-tests-20261107-v2/summary.json').read_text())['all_image_tests_passed']
assert json.loads(Path('/private/tmp/mech-related-compact-visit-work-engine-tests-20261107-v1/summary.json').read_text())['all_image_tests_passed']
loader=json.loads(Path('/private/tmp/mech-related-compact-visit-work-loader-tests-20261107-v1/summary.json').read_text())
assert loader['full5000_plus_edges_exact']==96 and loader['finite_denials']==24 and loader['all_denials_exact_to_prior_and_zero_retention']
for group,version in [('paired-scalar-selects',2),('compact-visit-work',2),('paired-scalar-selects-llvm-reference',1)]:
    directory='/private/tmp/mech-related-'+group+('-runners'if group!='paired-scalar-selects-llvm-reference'else'')+'-20261107-v'+str(version)
    assert json.loads(Path(directory,'summary.json').read_text())['linked_audit_passed']
for kind in ['paired-scalar-selects','compact-visit-work']:
    for name in ['scalar','one-kib']:
        assert json.loads(Path('/private/tmp/mech-related-'+kind+'-regressions-20261107-v1',name+'-comparison.json').read_text())['all_nontiming_fields_equal_IR240']
for config,count in [('latency-config.json',26),('resources-config.json',27)]:
    assert len(json.loads((P/config).read_text())['implementations'])==count
for name in ['comparison-native-paired-select-and-visit-work-20261107-5000-m3.json','comparison-native-paired-select-and-visit-work-resources-20261107-5000-m3.json']:
    assert not (D/name).exists(), 'No overwritten or hidden prior cohorts'
write(P/'source-freeze.json',dict(sources=sources,source_inventory_exact=True,predeclared_rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),no_prior_cohort=True))
quiet('latency')
run(['python3',R/'compare-implementations.py','--config',P/'latency-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','31','--order','randomized','--seed','990278','--deduplicate-outputs','--work',P/'latency','--output',D/'comparison-native-paired-select-and-visit-work-20261107-5000-m3.json'],'latency-run')
latency=json.loads((D/'comparison-native-paired-select-and-visit-work-20261107-5000-m3.json').read_text())
resource=json.loads((P/'resources-config.json').read_text())
instrumented=lambda n: 'rust-standard-instrumented' if n=='rust' else n+'-instrumented'
resource['resource_reference']=instrumented(latency['matched_native_reference']['variant'])
resource['whole_process_resource_reference']=instrumented(latency['whole_process_reference']['variant'])
assert {resource['resource_reference'],resource['whole_process_resource_reference']}<={v['name']for v in resource['implementations']}
write(P/'resources-config.json',resource)
quiet('resources')
run(['python3',R/'compare-implementations.py','--config',P/'resources-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','7','--order','randomized','--seed','9902787','--deduplicate-outputs','--resources','--work',P/'resources','--output',D/'comparison-native-paired-select-and-visit-work-resources-20261107-5000-m3.json'],'resources-run')
for item in sources:
    assert h.source_inventory(Path(item['source']))==json.loads((Path(item['build'])/'source-inventory.json').read_text())['files']
print('both cohorts complete',flush=True)
