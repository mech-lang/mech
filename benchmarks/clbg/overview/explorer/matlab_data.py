"""Import an immutable export of fully measured, work-audited MATLAB pairs."""
from pathlib import Path
COHORT='matlab-comparison/20261010'
def add_matlab_language(root,data,load,measured,add_file,sha):
    audit=load(COHORT+'/source-audit.json');built=load(COHORT+'/results/builds.json')
    pointer=load(COHORT+'/chart-export.json');export=COHORT+'/'+pointer['directory']
    report_name=export+'/timings.json';report=load(report_name);summary=load(export+'/summary.json')
    loc_name=export+'/loc.json';loc=load(loc_name);snapshot_name=export+'/measurement-snapshot.json';load(snapshot_name)
    assert pointer['report_sha256']==sha(root/report_name) and pointer['summary_sha256']==sha(root/export/'summary.json')
    assert pointer['loc_sha256']==sha(root/loc_name) and pointer['measurement_snapshot_sha256']==sha(root/snapshot_name)
    assert pointer['exporter_sha256']==sha(root/COHORT/'export_chart.py')
    assert audit['status']=='reviewed' and report['status'] in ('partial','complete') and summary['status']=='passed'
    assert report['sample_count']==3 and report['power']==dict(source='AC Power',low_power_mode=0)
    completed=report['completed_benchmarks'];assert set(completed)<={b['key'] for b in data['benchmarks']}
    assert completed and set(completed)==set(pointer['completed_benchmarks'])==set(summary['completed_benchmarks'])
    assert report['status']==('complete' if len(completed)==10 else 'partial')
    assert audit['builds_sha256']==sha(root/COHORT/'results/builds.json')
    assert summary['timing_report_sha256']==sha(root/report_name) and summary['source_audit_sha256']==sha(root/COHORT/'source-audit.json')
    assert loc['status']=='passed' and loc['timing_report_sha256']==sha(root/report_name)
    assert loc['source_audit_sha256']==sha(root/COHORT/'source-audit.json') and loc['audit_script_sha256']==sha(root/COHORT/'audit_loc_final.py')
    assert summary['final_loc_audit_sha256']==sha(root/loc_name) and summary['admitted_sources']==2*len(completed)
    for f,h in report['source_pins'].items():assert sha(root/COHORT/f)==h,f
    for f,h in audit['evidence'].items():assert sha(root/COHORT/f)==h,f
    for runtime in report['runtimes'].values():assert sha(runtime['path'])==runtime['sha256']
    assert audit['allocation_evidence']['status']=='passed'
    branches_name=COHORT+'/results/final-allocation-branches.json'
    branches=load(branches_name)
    assert branches['status']=='passed' and branches['timed'] is False
    assert branches['depths']==[1,3,7,11]
    for file,digest in branches['sha256'].items():assert sha(root/COHORT/file)==digest,file
    for profile in ('compact','optimized'):
        for index,depth in enumerate(branches['depths']):
            nodes=2**(depth+1)-1
            assert branches['evidence'][profile][index]==[nodes,nodes,nodes,2**depth,1]
    data['languages'].append(dict(key='matlab',label='MATLAB',color='#e16737',execution='JIT',line_style='solid',
        coverage_note=f'{len(completed)}/10 complete three-sample benchmark pairs; remaining measurements are in progress.' if len(completed)<10 else 'All ten complete three-sample benchmark pairs.'))
    rows=[r for r in report['programs'].values() if r['language']=='matlab'];assert len(rows)==2*len(completed)
    for row in rows:
        assert row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds'])==len(row['sample_validation'])==3 and all(v['reference_match'] for v in row['sample_validation'])
        metric=loc['rows'][row['key']];assert metric['source_sha256']==row['source_sha256'] and metric['normalized_loc']==row['loc']
        measured(row['key'],row['benchmark'],'matlab',metric['normalized_loc'],row['samples_seconds'],row['source'],row['source_sha256'],report_name,COHORT+'/source-audit.json',
            algorithm=row['algorithm'],required_work_validated=True,runtime_sha256=built['runtimes']['engine']['sha256'],
            physical_loc=metric['physical_loc'],loc_expansions=metric['expansions'],loc_metric='statement-normalized',loc_report=loc_name,
            source_url=row['source_url'],source_link_label='Task specification',local_adaptation=True,source_provenance=row['source_provenance'],
            runtime_policy='MATLAB R2026b · standard JIT · native Apple Silicon · base MATLAB · at most eight automatic computation threads',
            execution_mode='jit',timing_boundary=built['timing_boundary'],timing_summary=built['timing_boundary'],
            runtime_support='Ordinary MATLAB arrays, dictionaries, regex and binary I/O. Pi uses standard Java BigInteger from the pinned task-local Temurin JRE 25. No MEX benchmark executor; the native cell-identity diagnostic is untimed audit evidence only.',
            runtime_arguments=row['command'],full_output_validation=row['sample_validation'])
        if row['benchmark'] in ('knucleotide','revcomp','regexredux'):
            launcher=built['runtimes']['input_routing'];ids=add_file(launcher['path'],launcher['sha256'])
            data['files'][ids[0]]['role']='runtime-support'
            data['files'][ids[0]]['loc_note']='Excluded from application LOC: stream routing only. Duplicates stdin to descriptor 3 before MATLAB disconnects descriptor 0; no input parsing or computation.'
            data['programs'][row['key']]['files']+=ids
        b=next(b for b in data['benchmarks'] if b['key']==row['benchmark'])
        group=next((g for g in b['groups'] if g['language']=='matlab'),None)
        if group is None:group=dict(language='matlab',label='MATLAB',programs=[]);b['groups'].append(group)
        group['programs'].append(row['key'])
    data['matlab_audit']=dict(admitted_sources=len(rows),attempted_sources=20,completed_benchmarks=completed,coverage_status=report['status'],
        sample_count=3,timing_report=report_name,source_audit=COHORT+'/source-audit.json',final_loc_audit=loc_name,runtimes=built['runtimes'],
        controls=summary['controls'],allocation_evidence=audit['allocation_evidence'],
        supplemental_allocation_evidence=dict(branches,report=branches_name,report_sha256=sha(root/branches_name)),
        scheduling='Base MATLAB standard JIT and automatic computation threads, capped by the host’s eight logical CPUs. No Parallel Computing Toolbox or Coder is installed.',
        source_provenance='Twenty local MATLAB source ports of the official required algorithms. Published MATLAB source rankings or timings are not claimed or imported.')
    data['provenance']['matlab_data.py']=sha(Path(__file__))
    data['protocol']['matlab']=f'MATLAB R2026b: {len(completed)}/10 complete CPU workloads measured; twenty local compact/optimized candidates implement the full suite. Only benchmark pairs with three shuffled fresh-process samples per source and full reference-output checks enter this immutable chart snapshot. Unfinished tests contribute no endpoint or ratio. The chart selects the actual fastest median and fewest statement-normalized LOC. Base MATLAB standard JIT, arrays, growing dictionaries and regex; generic Java BigInteger supplies unbounded arithmetic. Startup, cached online-license checks, first-use JIT, complete input, computation, all output and teardown are timed. The pinned stream-only launcher preserves stdin and is inspectable separately from application LOC. Untimed identity probes verify independent two-slot cell allocations for every tree node. No native benchmark executor, precomputed matrix, histogram substitute or LCG cycle shortcut. Unchanged Mech AOT controls were remeasured alongside MATLAB; earlier cohort samples remain unchanged.'
