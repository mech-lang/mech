from pathlib import Path
import gzip,hashlib,importlib.util,json,re,statistics,sys
sys.dont_write_bytecode=True
P=Path(__file__).parent
def blob(name):
    p=P/name
    if not p.exists():p=Path(str(p)+'.gz')
    b=p.read_bytes();return gzip.decompress(b)if p.suffix=='.gz'else b
def read(name):return json.loads(blob(name))
manifest=read('manifest.json')
for n,pin in manifest['files'].items():
    b=(P/n).read_bytes();assert len(b)==pin['bytes']and hashlib.sha256(b).hexdigest()==pin['sha256'],n
    b=gzip.decompress(b)if pin['gzip']else b
    assert len(b)==pin['decoded_bytes']and hashlib.sha256(b).hexdigest()==pin['decoded_sha256'],n
report=read('report.json');selection=read('source-and-selection.json')
assert not report['selected']and not report['goal_complete']and selection['selected_IR242_unchanged']and not selection['LLVM_mandatory']
cohorts=[]
for p in sorted((P/'cohorts').glob('comparison*.json*')):
    d=read(str(p.relative_to(P)).removesuffix('.gz'));cohorts.append(d)
    assert all(r['exit']==0 and r['exact_rust_output']for r in d['samples'])
    assert len(d['samples'])==len(d['implementations'])*len(d['execution_order']['rounds'])
    for name,s in d['summary'].items():
        rows=[r for r in d['samples']if r['variant']==name]
        assert s['median_whole_process_seconds']==statistics.median(r['wall_seconds']for r in rows)
        if s['calculation_boundary']=='post_json_including_setup':
            v=next(v for v in d['implementations']if v['name']==name)
            field='post_json_seconds'if v['kind']=='mech'else'seconds'
            assert s['median_post_json_seconds']==statistics.median(r[field]for r in rows)
            assert abs(s['post_json_over_fastest_matched_native']-s['median_post_json_seconds']/d['matched_native_reference']['median_seconds'])<1e-12
    external=[v for v in d['implementations']if v['kind']!='mech']
    eligible=[v['name']for v in external if v['calculation_boundary']=='post_json_including_setup']
    assert min(eligible,key=lambda n:d['summary'][n]['median_post_json_seconds'])==d['matched_native_reference']['variant']
    assert min([v['name']for v in external],key=lambda n:d['summary'][n]['median_whole_process_seconds'])==d['whole_process_reference']['variant']
if report['schema']=='mech.related-post-gen.optional-LLVM-native-image.v1':
    assert len(cohorts)==3 and sorted(len(d['samples'])for d in cohorts)==[182,775,775]
    tests=read('tests/summary.json');assert tests['CI399_passed']==399 and tests['LLVM_native_tests_passed']
    native=read('tests/llvm-native-tests.json');assert native['exit']==0 and '4 passed; 0 failed'in native['stdout']
    observations=[json.loads(line)for line in native['stdout'].splitlines()if line.startswith('{')]
    assert observations[-1]['comparisons_so_far']==240 and observations[-1]['loader_checks_so_far']==132
    apps=read('generated/summary.json');assert apps['actual_minimal_runtime_only_Cargo_build_passed']and not apps['Cargo_project_not_built']
    spec=importlib.util.spec_from_file_location('machine_reader',P/'machine-reader/machine_facts.py');machine=importlib.util.module_from_spec(spec);spec.loader.exec_module(machine)
    for name,count in [('after-4',55),('reuse-4',53)]:
        names=read('machine/'+name+'/definitions.json');assert len(names)==count
        derived=machine.extract(blob('machine/'+name+'/native.o'),blob('machine/'+name+'/remarks.yaml').decode(),names)
        actual=read('machine/'+name+'/machine-facts.json')
        for field in ['functions','code_bytes','text_bytes','machine_constant_bytes']:assert derived[field]==actual[field],(name,field)
        metadata=read('objects/'+name+'/metadata.json')
        assert metadata['native_descriptor_version']==10 and metadata['code_bytes']==actual['code_bytes']
    assert report['default_backend_changed']is False and report['LLVM_required_for_Cranelift']is False
else:
    assert len(cohorts)==2 and sorted(len(d['samples'])for d in cohorts)==[175,744]
    tests=read('tests/summary.json');assert tests['actual_CI_passed']==401 and tests['actual_CI_names_exact']
    assert report['decision']['paired']['mech-ir264-cranelift-runtime-control']['post_json_seconds']['wins']==0
    for kind in ['scalar','one-kib']:assert read('regressions/'+kind+'-comparison.json')['all_nontiming_fields_equal_IR240']
print(json.dumps(dict(files=len(manifest['files']),cohorts=len(cohorts),exact_cohort_outputs=sum(len(d['samples'])for d in cohorts),selected=False,goal_complete=False)))
