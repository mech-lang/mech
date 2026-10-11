"""Import complete source-pinned native D measurements without replacing older cohorts."""
from pathlib import Path

COHORT='d-comparison/20261011'

def add_d_language(root,data,load,measured,add_file,sha):
    audit_name=COHORT+'/source-audit.json';report_name=COHORT+'/results/timings.json'
    audit=load(audit_name);report=load(report_name);summary=load(COHORT+'/results/summary.json')
    assert audit['status']=='reviewed' and report['status']=='complete' and summary['status']=='passed'
    assert summary['source_audit_sha256']==report['source_audit_sha256']==sha(root/audit_name)
    assert summary['timing_report_sha256']==sha(root/report_name)
    assert summary['loc_tool_sha256']==sha(root/COHORT/'loc.py')
    assert summary['summary_script_sha256']==sha(root/COHORT/'summarize.py')
    assert report['sample_count']==3 and report['power']==dict(source='AC Power',low_power_mode=0)
    assert set(report['completed_benchmarks'])=={b['key'] for b in data['benchmarks']}
    assert not summary['failures'] and not summary['pending']
    for file,digest in (audit['pins']|audit['libraries']).items():assert sha(file)==digest,file
    assert sha(audit['runtime']['path'])==audit['runtime']['sha256']
    data['languages'].append(dict(key='d',label='D',color='#ba595e',execution='AOT',line_style='solid',
        runtime_policy='LDC 1.43.0 · LLVM 23.1.2 · optimized native ARM64 · Apple M3'))
    admitted=[]
    for row in report['programs'].values():
        if row['language']!='d':continue
        assert row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
        assert row['eligible_for_chart'] and row['small_validation']['reference_match']
        assert len(row['samples_seconds'])==len(row['sample_validation'])==3
        assert all(v['reference_match'] for v in row['sample_validation'])
        for field in ('source','binary'):assert sha(row[field])==row[field+'_sha256']
        metric=summary['loc'][row['key']];assert metric['normalized_loc']==row['loc']
        adaptations=row.get('adaptations',[])
        measured(row['key'],row['benchmark'],'d',row['loc'],row['samples_seconds'],row['source'],row['source_sha256'],report_name,audit_name,
            source_url=row.get('source_url'),source_provenance=row.get('source_provenance','D application source adapted from archived Benchmarks Game or published D implementations; documented corrections retain the common workload.'),
            adaptations=adaptations,local_adaptation=bool(adaptations),physical_loc=row['physical_loc'],loc_expansions=row['loc_expansions'],loc_metric='statement-normalized',
            algorithm=row['algorithm'],required_work_validated=True,full_output_validation=row['sample_validation'],binary_sha256=row['binary_sha256'],
            compiler_sha256=audit['runtime']['sha256'],execution_mode='aot',runtime_policy='LDC 1.43.0 native ARM64 AOT',runtime_arguments=row['command'][1:],
            build_command=row.get('build_command'),timing_boundary=audit['timing_boundary'],timing_summary=audit['timing_boundary'],
            runtime_support='Ordinary D arrays, growable associative arrays, threads, Phobos BigInt/regex/Region allocator, general GMP arithmetic, and native SIMD intrinsics where used. Application algorithms, FFI and helpers count toward D LOC; no native benchmark executor.')
        if row.get('upstream_source'):
            add_file(row['upstream_source'],row['upstream_sha256'])
        bench=next(b for b in data['benchmarks'] if b['key']==row['benchmark'])
        group=next((g for g in bench['groups'] if g['language']=='d'),None)
        if group is None:group=dict(language='d',label='D',programs=[]);bench['groups'].append(group)
        group['programs'].append(row['key']);admitted.append(row['key'])
    assert len(admitted)==summary['admitted_sources']
    for row in audit.get('excluded',[]):
        if isinstance(row,dict):data['exclusions'].append(dict(language='d',**row))
    downloads={}
    for name in ('summary.json','summary.csv','summary.md'):
        file=root/COHORT/'results'/name
        downloads[name]=add_file(file,sha(file))
    data['d_audit']=dict(admitted_sources=len(admitted),sample_count=3,samples_completed=summary['samples_completed'],
        source_audit=audit_name,timing_report=report_name,compiler=audit['runtime']['version'],
        comparisons_to_mech=summary['comparisons'],controls=summary['controls'],ratio_policy=summary['ratio_policy'],
        geometric_means=summary['geometric_means'],comparison_downloads=downloads,discarded_samples=summary['discarded_samples'],
        color_source='https://github.com/github-linguist/linguist/blob/master/lib/linguist/languages.yml')
    data['protocol']['d']=audit['timing_boundary']+' D covers all ten tests with locally compiled compact and optimized candidates. Historical D sources and newer published D programs were considered; platform and common-work corrections are recorded. Complete output and allocation/algorithm work are checked before admission. Endpoints are measured winners from this candidate pool. Fresh unchanged Mech controls support the × Mech report; existing explorer source counts and absolute timings are preserved.'
    data['provenance']['d_data.py']=sha(Path(__file__))
