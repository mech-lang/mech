"""Import complete, audited local Haskell/OCaml measurements."""
from pathlib import Path
COHORT='functional-comparison/20261010'
def add_functional_languages(root,data,load,measured,sha):
    audit=load(COHORT+'/source-audit.json');built=load(COHORT+'/results/builds.json')
    report_name=COHORT+'/results/timings.json';report=load(report_name);summary=load(COHORT+'/results/summary.json')
    loc_report=COHORT+'/results/final-loc-audit.json';final_loc=load(loc_report)
    assert audit['status']=='reviewed' and report['status']=='complete' and summary['status']=='passed'
    assert report['sample_count']==3 and report['power']==dict(source='AC Power',low_power_mode=0)
    assert set(report['completed_benchmarks'])=={b['key'] for b in data['benchmarks']}
    assert audit['builds_sha256']==sha(root/COHORT/'results/builds.json')
    assert summary['timing_report_sha256']==sha(root/report_name)
    assert summary['source_audit_sha256']==sha(root/COHORT/'source-audit.json')
    assert final_loc['status']=='passed' and final_loc['timing_report_sha256']==sha(root/report_name)
    assert final_loc['source_audit_sha256']==sha(root/COHORT/'source-audit.json')
    assert final_loc['audit_script_sha256']==sha(root/COHORT/'audit_loc_final.py')
    assert summary['final_loc_audit_sha256']==sha(root/loc_report)
    for relative,expected in report['source_pins'].items():assert sha(root/COHORT/relative)==expected
    for evidence in audit['allocation_evidence']:assert sha(root/COHORT/evidence['file'])==evidence['sha256']
    review={r['key']:r for r in audit['rows']}
    configs={'haskell':('Haskell','#5E5086','GHC '+built['compilers']['haskell']['version']+' · LLVM '+built['llvm']['version']+' · -O2 -fllvm'),
             'ocaml':('OCaml','#EF7A08','OCaml '+built['compilers']['ocaml']['version']+' · Flambda · -O3 -unsafe -inline 100')}
    assert 'flambda: true' in built['ocaml_config']
    for key,(label,color,policy) in configs.items():data['languages'].append(dict(key=key,label=label,color=color,execution='AOT',line_style='solid'))
    for row in report['programs'].values():
        lang=row['language']
        if lang not in configs or not row['eligible_for_chart']:continue
        assert review[row['key']]['admitted'] and row['algorithm']['strict'] and row['algorithm']['native_benchmark_kernel'] is False
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds'])==len(row['sample_validation'])==3 and all(v['reference_match'] for v in row['sample_validation'])
        assert sha(row['binary'])==row['binary_sha256'] and row['loc']==row['loc_audit']['normalized_loc']
        metric=final_loc['rows'][row['key']]
        assert metric['source_sha256']==row['source_sha256'] and metric['initial_normalized_loc']==row['loc']
        adapted=row.get('local_adaptation',False);compiler=built['compilers'][lang]
        support='Ordinary array, map, arbitrary-precision arithmetic and regex libraries as needed. Application control flow remains in the displayed source; no native benchmark executor.'
        if row['id']=='binarytrees-ghc-4-faithful':
            support+=' This source uses the standard GHC.Compact library for the long-lived tree; temporary trees use ordinary default GC.'
        measured(row['key'],row['benchmark'],lang,metric['normalized_loc'],row['samples_seconds'],row['source'],row['source_sha256'],report_name,COHORT+'/source-audit.json',
            algorithm=row['algorithm'],required_work_validated=True,binary_sha256=row['binary_sha256'],compiler_sha256=compiler.get('frontend_sha256',compiler['sha256']),
            physical_loc=metric['physical_loc'],loc_expansions=metric['expansions'],loc_metric='statement-normalized',loc_report=loc_report,source_url=row['source_url'],local_adaptation=adapted,
            adaptations=row['adaptations'],upstream_id=row.get('upstream_id',row['id']),source_provenance=('Locally corrected published '+configs[lang][0]+' source: '+' '.join(row['adaptations']) if adapted else 'Published Benchmarks Game '+configs[lang][0]+' source, locally compiled and timed.'),
            runtime_policy='AOT · '+configs[lang][2],runtime_support=support,
            runtime_arguments=row['command'][1:],
            timing_boundary='Fresh-process elapsed time: startup, complete input, computation, full stdout to a regular file, and teardown. Compilation and validation excluded.')
        benchmark=next(b for b in data['benchmarks'] if b['key']==row['benchmark'])
        group=next((g for g in benchmark['groups'] if g['language']==lang),None)
        if group is None:
            group=dict(language=lang,label=configs[lang][0],programs=[]);benchmark['groups'].append(group)
        group['programs'].append(row['key'])
    for row in report['excluded']:data['exclusions'].append(dict(key=row['benchmark']+':'+row['id'],**row))
    data['functional_audit']=dict(published_sources=60,corrected_sources=len(load(COHORT+'/corrections.json')['programs']),admitted_sources=summary['admitted_sources'],
        sample_count=3,compilers=built['compilers'],llvm=built['llvm'],timing_report=report_name,source_audit=COHORT+'/source-audit.json',final_loc_audit=loc_report,controls=summary['controls'],
        allocation_evidence=audit['allocation_evidence'],output_exclusions=summary['output_exclusions'],
        scheduling='Eight logical CPUs. Published parallel Haskell variants use eight RTS capabilities; serial variants retain defaults. Published OCaml process/task partitioning is retained. No GC heap-tuning flags for trees.')
    data['provenance']['functional_data.py']=sha(Path(__file__))
    data['protocol']['functional_languages']='Haskell and OCaml cover all ten full workloads with three shuffled local samples per admitted source. All 60 published candidates were considered; required-work and API corrections are separately labelled and count toward LOC. GHC 9.10.3 uses LLVM and -O2; OCaml 5.4.1 uses Flambda and -O3. Each tree allocates its complete interior and leaf nodes with default GC; compiler evidence verifies dynamic leaf allocation, separate child construction, and long-lived-root retention. Nucleotide maps grow from small initial capacities and include every window. All random FASTA samples use binary64 division and unscaled cumulative probabilities. Eight logical CPUs; original task partitioning retained. General GMP, PCRE2, hash-map and array libraries provide ordinary operations; no native benchmark executor. Unchanged Mech AOT controls were remeasured alongside the new sources; earlier cohort samples remain unchanged.'
