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
assert h.source_inventory(S)==json.loads((O/'source-inventory.json').read_text())['files']
assert json.loads(Path('/private/tmp/mech-related-positional-metadata-tests-20261107-v1/summary.json').read_text())['actual_CI_passed']==399
assert json.loads(Path('/private/tmp/mech-related-positional-metadata-engine-tests-20261107-v1/summary.json').read_text())['all_image_tests_passed']
loader=json.loads(Path('/private/tmp/mech-related-positional-metadata-loader-tests-20261107-v1/summary.json').read_text())
assert loader['full5000_plus_edges_exact']==96 and loader['finite_denials']==24 and loader['all_denials_exact_to_prior_and_zero_retention']
assert not loader['LLVM_tools_used']
assert json.loads(Path('/private/tmp/mech-related-positional-metadata-runners-20261107-v1/summary.json').read_text())['linked_audit_passed']
for config,count in [('latency-config.json',24),('resources-config.json',25)]:
    assert len(json.loads((P/config).read_text())['implementations'])==count
for name in ['comparison-native-positional-metadata-20261107-5000-m3.json','comparison-native-positional-metadata-resources-20261107-5000-m3.json']:
    assert not (D/name).exists(), 'No overwritten or hidden prior cohorts'
write(P/'source-freeze.json',dict(commit=subprocess.check_output(['git','-c','gc.auto=0','rev-parse','HEAD'],cwd=S,text=True).strip(),source_inventory_exact=True,predeclared_rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),no_prior_cohort=True))
for kind in ['scalar','one-kib']:
    assert json.loads(Path('/private/tmp/mech-related-positional-metadata-regressions-20261107-v1',kind+'-comparison.json').read_text())['all_nontiming_fields_equal_IR240']
quiet('latency')
run(['python3',R/'compare-implementations.py','--config',P/'latency-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','31','--order','randomized','--seed','990276','--deduplicate-outputs','--work',P/'latency','--output',D/'comparison-native-positional-metadata-20261107-5000-m3.json'],'latency-run')
latency=json.loads((D/'comparison-native-positional-metadata-20261107-5000-m3.json').read_text())
resource=json.loads((P/'resources-config.json').read_text())
instrumented=lambda n: 'rust-standard-instrumented' if n=='rust' else n+'-instrumented'
resource['resource_reference']=instrumented(latency['matched_native_reference']['variant'])
resource['whole_process_resource_reference']=instrumented(latency['whole_process_reference']['variant'])
assert {resource['resource_reference'],resource['whole_process_resource_reference']}<={v['name']for v in resource['implementations']}
write(P/'resources-config.json',resource)
quiet('resources')
run(['python3',R/'compare-implementations.py','--config',P/'resources-config.json','--input','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json','--samples','7','--order','randomized','--seed','9902767','--deduplicate-outputs','--resources','--work',P/'resources','--output',D/'comparison-native-positional-metadata-resources-20261107-5000-m3.json'],'resources-run')
assert h.source_inventory(S)==json.loads((O/'source-inventory.json').read_text())['files']
print('both cohorts complete',flush=True)
