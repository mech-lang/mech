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

assert len(cohorts)==2 and sorted(len(d['samples'])for d in cohorts)==[175,744]
assert report['qualification']['CI']['actual_CI_passed']==399
assert report['qualification']['CI']['actual_CI_names_exact']
assert report['qualification']['engine_new_positional_tests']==3
assert report['qualification']['engine']['all_image_tests_passed']
assert report['qualification']['loaders']['full5000_plus_edges_exact']==96
assert report['qualification']['loaders']['finite_denials']==24
assert report['qualification']['loaders']['all_denials_exact_to_prior_and_zero_retention']
assert not report['qualification']['loaders']['LLVM_tools_used']
for kind in ['scalar','one-kib']:assert read('regressions/'+kind+'-comparison.json')['all_nontiming_fields_equal_IR240']
assert report['normalized_native_text_exact_parent']
assert not report['decision']['whole_superiority_passed']and not report['decision']['practical_restore_threshold_passed']
assert report['decision']['resource_restore']['mech-ir266-reuse-4']['instructions']>report['decision']['resource_restore']['mech-ir264-cranelift-runtime-control']['instructions']
oracle=hashlib.sha256(blob('outputs/related_posts_rust.json')).hexdigest()
assert all(r['output_sha256']==oracle for d in cohorts for r in d['samples'])
assert not report['LLVM_required_for_Cranelift']and not report['LLVM_tools_used_for_Cranelift_build_or_execution']
print(json.dumps(dict(files=len(manifest['files']),cohorts=len(cohorts),exact_cohort_outputs=sum(len(d['samples'])for d in cohorts),selected=False,goal_complete=False)))
