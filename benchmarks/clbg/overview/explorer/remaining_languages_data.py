"""Import complete, source-pinned remaining Benchmark Game language measurements."""
from pathlib import Path

COHORT='remaining-languages-comparison/20261011'
LANGUAGES={
    'csharp':dict(label='C# (JIT)',color='#7355dd',execution='JIT',policy='.NET 10.0.12 · C# · fresh optimizing JIT process'),
    'csharp-aot':dict(label='C# (AOT)',color='#7355dd',execution='AOT',policy='.NET 10.0.12 · C# NativeAOT · Release · native ARM64'),
    'fsharp':dict(label='F# (JIT)',color='#b845fc',execution='JIT',policy='.NET 10.0.12 · F# · fresh optimizing JIT process'),
    'fsharp-aot':dict(label='F# (AOT)',color='#b845fc',execution='AOT',policy='.NET 10.0.12 · F# NativeAOT · Release · native ARM64'),
    'ada':dict(label='Ada',color='#02f88c',execution='AOT',policy='GNAT FSF 16.1.0-1 · optimized native ARM64'),
    'pascal':dict(label='Pascal',color='#E3F171',execution='AOT',policy='Free Pascal 3.2.2 · optimized native ARM64'),
    'fortran':dict(label='Fortran',color='#4d41b1',execution='AOT',policy='GNU Fortran 16.2 · -O3 · native CPU · OpenMP · binary64'),
    'dart':dict(label='Dart (AOT)',color='#00B4AB',execution='AOT',policy='Dart 3.13.5 · native ARM64 AOT'),
    'dart-jit':dict(label='Dart (JIT)',color='#00B4AB',execution='JIT',policy='Dart 3.13.5 · fresh VM process · optimizing JIT'),
    'smalltalk':dict(label='Smalltalk (Pharo)',color='#596706',execution='JIT',policy='Pharo 14 snapshot · native ARM64 Cog/Spur JIT · fresh headless image process'),
}

def add_remaining_languages(root,data,load,measured,add_file,sha):
    audit_name=COHORT+'/source-audit-v2.json';summary_name=COHORT+'/results/summary.json'
    audit=load(audit_name);summary=load(summary_name)
    assert audit['status']=='reviewed' and summary['status']=='passed'
    assert summary['source_audit_sha256']==sha(root/audit_name)
    assert summary['summary_script_sha256']==sha(root/COHORT/'summarize.py')
    assert summary['loc_tool_sha256']==sha(root/COHORT/'metrics_final.py')
    evidence=load(COHORT+'/evidence/supplemental-audit.json')
    assert summary['supplemental_audit_sha256']==sha(root/COHORT/'evidence/supplemental-audit.json')
    assert evidence['status']=='passed' and all(u['included_in_counted_source'] for u in evidence['compiled_units'])
    for filename,record in evidence['fortran_dart_evidence'].items():assert sha(filename)==record['sha256']
    audits={}
    for relative,digest in summary['source_audits'].items():
        assert sha(root/COHORT/relative)==digest
        audits[relative]=load(COHORT+'/'+relative)
    reports={}
    for relative,digest in summary['report_pins'].items():
        report=load(COHORT+'/'+relative);assert sha(root/COHORT/relative)==digest
        assert report['status']=='complete' and report['sample_count']==3
        assert report['power']==dict(source='AC Power',low_power_mode=0)
        report_audit_name=Path(report['source_audit']).name
        assert report['source_audit_sha256']==summary['source_audits'][report_audit_name]
        expected=audits[report_audit_name].get('expected_benchmarks',[b['key'] for b in data['benchmarks']])
        assert set(report['completed_benchmarks'])==set(expected)
        reports[relative]=report
    pins={}
    for check in audits.values():
        for filename,digest in (check['pin_files']|check['libraries']).items():
            assert filename not in pins or pins[filename]==digest
            pins[filename]=digest
    for filename,digest in pins.items():assert sha(filename)==digest,filename
    for runtime in summary['runtimes'].values():assert sha(runtime['path'])==runtime['sha256']
    colors=load(COHORT+'/support/language-colors.json')
    for key,meta in LANGUAGES.items():
        color_key=key.removesuffix('-aot').removesuffix('-jit')
        assert colors['languages'][color_key]['color']==meta['color']
        data['languages'].append(dict(key=key,label=meta['label'],color=meta['color'],execution=meta['execution'],
            line_style='dashed' if key in ('csharp','fsharp','dart-jit') else 'solid',runtime_policy=meta['policy']))
    review={r['key']:r for r in audit['programs']}
    added=[]
    for relative,report in reports.items():
        report_audit_name=Path(report['source_audit']).name
        report_audit=audits[report_audit_name]
        review={r['key']:r for r in report_audit['programs']}
        measured_audit_name=COHORT+'/'+report_audit_name
        for row in report['programs'].values():
            lang=row['language']
            if lang not in LANGUAGES or row.get('full_failure'):continue
            assert len(row['samples_seconds'])==len(row['sample_validation'])==3
            assert row['small_validation']['reference_match'] and all(v['reference_match'] for v in row['sample_validation'])
            check=review[row['key']];assert check['eligible_for_chart']
            assert row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
            for f in ('source','binary'):
                assert row[f+'_sha256']==check[f+'_sha256'] and sha(row[f])==row[f+'_sha256']
            loc=summary['final_loc'][row['key']]
            assert row['loc']==loc['timed_loc'] and row['physical_loc']==loc['physical_loc']
            final_count=loc['normalized_loc']
            upstream=row.get('upstream_source',row.get('original_source'))
            upstream_sha=row.get('upstream_sha256',row.get('upstream_source_sha256',row.get('original_source_sha256')))
            if upstream:assert sha(upstream)==upstream_sha
            control=summary['controls'][relative.split('/')[-1]+':'+row['benchmark']]
            assert control['seconds'] is not None
            meta=LANGUAGES[lang]
            runtime=report_audit['runtimes']['pharo' if lang=='smalltalk' else lang]
            source_url=row.get('source_url',row.get('url')) or 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+row['benchmark']+'.html'
            adaptations=row.get('adaptations',[])
            provenance=('Published Benchmark Game source, locally corrected and compiled. ' if upstream and row.get('local_adaptation')
                else 'Published Benchmark Game source, locally compiled and measured. ' if upstream
                else 'Locally authored '+meta['label']+' application. ')+' '.join(adaptations)
            support='Standard arrays, growable maps, threads and process I/O; general GMP and PCRE2 where used. Complete application orchestration, helpers and FFI declarations count toward LOC. No specialized native benchmark executor.'
            if lang=='fortran':support+=' Published Intel ifx source algorithms are compiled with native GNU Fortran here; published timings are not reused.'
            if lang=='smalltalk':support+=' The algorithm is loaded from the displayed Smalltalk source into a pinned image before timing; the measured fresh VM includes image loading and JIT work.'
            measured(row['key'],row['benchmark'],lang,final_count,row['samples_seconds'],row['source'],row['source_sha256'],
                COHORT+'/'+relative,measured_audit_name,algorithm=row['algorithm'],required_work_validated=True,
                full_output_validation=row['sample_validation'],binary_sha256=row['binary_sha256'],runtime_sha256=runtime['sha256'],
                compiler_sha256=runtime['sha256'],physical_loc=row['physical_loc'],loc_expansions=loc['expansions'],
                loc_metric='statement-normalized',loc_report=summary_name,source_url=source_url,
                source_link_label='Published source' if upstream else 'Benchmark requirements',upstream_source_sha256=upstream_sha,
                local_adaptation=bool(row.get('local_adaptation')),adaptations=adaptations,source_provenance=provenance,
                execution_mode=meta['execution'].lower(),runtime_policy=meta['policy'],runtime_arguments=row['command'][1:],runtime_support=support,
                mech_comparison=dict(seconds=control['seconds'],fastest_loc=control['fastest_loc'],smallest_loc=control['smallest_loc'],
                    time_x_mech=row['median_seconds']/control['seconds'],fastest_loc_x_mech=final_count/control['fastest_loc'],
                    smallest_loc_x_mech=final_count/control['smallest_loc']),timing_boundary=report_audit['timing_boundary'],timing_summary=report_audit['timing_boundary'])
            bench=next(b for b in data['benchmarks'] if b['key']==row['benchmark'])
            group=next((g for g in bench['groups'] if g['language']==lang),None)
            if group is None:
                group=dict(language=lang,label=meta['label'],programs=[]);bench['groups'].append(group)
            group['programs'].append(row['key']);added.append(row['key'])
    assert len(added)==summary['admitted_sources']
    downloads={}
    for ext in ('csv','md'):
        target=Path(__file__).parent/'dist'/('remaining-languages-mech.'+ext)
        target.write_bytes((root/COHORT/'results'/('summary.'+ext)).read_bytes())
        data['provenance'][target.name]=sha(target);downloads[ext]='./'+target.name
    for failure in summary['full_failures']:
        r=next(r for check in audits.values() for r in check['programs'] if r['key']==failure['key'])
        data['exclusions'].append(dict(**failure,source_sha256=r['source_sha256'],reason='Failed complete full-workload validation or execution.'))
    data['remaining_languages_audit']=dict(admitted_sources=len(added),sample_count=3,
        completed_samples=summary['completed_samples'],expected_samples=summary['expected_samples'],
        full_failures=summary['full_failures'],unresolved_full_failures=summary['unresolved_full_failures'],
        repairs=summary['repairs'],planned_samples=summary['planned_samples'],
        timing_reports=list(reports),source_audit=audit_name,summary=summary_name,
        comparisons_to_mech=summary['comparisons'],controls=summary['controls'],comparison_downloads=downloads,
        ratio_policy=summary['ratio_policy'],endpoint_policy=summary['endpoint_policy'],
        runtimes={k:{f:v for f,v in r.items() if f in ('version','sha256','execution','runtime_options')} for k,r in summary['runtimes'].items()},
        source_audits=summary['source_audits'],
        color_source=colors['source_url'],loc_corrections=summary['loc_corrections'],supplemental_evidence=evidence,
        unmeasured_alternates=[r for r in audit['unmeasured_alternates']
            if not any(name in r['id'].lower() for name in ('toit','micropython'))])
    data['provenance']['remaining_languages_data.py']=sha(Path(__file__))
    data['protocol']['remaining_languages']=(audit['timing_boundary']+' C# and F# include both JIT and NativeAOT; Dart includes AOT and JIT. '
        'Pascal uses Free Pascal, not a separate Delphi entry. Smalltalk uses headless Pharo. Fortran uses GNU Fortran on ARM64 rather than the published Intel compiler. '
        'Application algorithms remain in the named language. Source work corrections and all helpers/FFI are counted. Existing explorer sources and absolute measurements are preserved.')
