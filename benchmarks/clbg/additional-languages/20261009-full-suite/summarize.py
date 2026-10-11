"""Select admitted measured endpoints and compare each with current Mech."""
import hashlib,json,math,statistics
from pathlib import Path
D=Path(__file__).resolve().parent
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def gm(xs):return math.exp(statistics.mean(math.log(x) for x in xs))
def main():
    report=json.loads((D/'results/timings.json').read_text())
    audit=json.loads((D/'source-audit.json').read_text())
    profiles=json.loads((D/'profiles.json').read_text())
    assert report['status']=='complete'
    assert report['source_audit_sha256']==sha(D/'source-audit.json')
    programs=list(report['programs'].values())
    assert len(programs)==len(audit['rows'])+10
    assert all(len(r['samples_seconds'])==3 for r in programs)
    assert all(r['small_validation']['reference_match'] and r['full_validation']['reference_match'] and all(v['reference_match'] for v in r['sample_validation']) for r in programs)
    for file,digest in report['library_pins'].items():assert sha(D/file)==digest
    for file,digest in report['review_pins'].items():assert sha(D/file)==digest
    for file,digest in report['external_library_pins'].items():assert sha(Path(file))==digest
    previous=json.loads((D.parent/'20261009/results/summary.json').read_text())
    old={r['benchmark']:r for r in previous['benchmarks']}
    rows=[]
    for benchmark in report['completed_benchmarks']:
        group=[r for r in programs if r['benchmark']==benchmark]
        control=next(r for r in group if r['language']=='mech')
        row=dict(benchmark=benchmark,mech_seconds=control['median_seconds'])
        for language in ('mojo','futhark'):
            candidates=[r for r in group if r['language']==language]
            pair=profiles['pairs'][language][benchmark]
            if not candidates:
                assert pair.get('status')=='unsupported-allocation-model'
                row[language]=dict(status='excluded',reason=pair['reason'])
                continue
            assert all(r['eligible_for_chart'] and r['algorithm']['strict'] and not r['algorithm']['native_benchmark_kernel'] for r in candidates)
            fast=min(candidates,key=lambda r:(r['median_seconds'],r['loc'],r['key']))
            fewest=min(candidates,key=lambda r:(r['loc'],r['median_seconds'],r['key']))
            covered={Path(r['source']).name for r in candidates}
            assert {pair['compact'],pair['performance']}<=covered
            previous_seconds=old[benchmark][language]['fast_seconds'] if benchmark!='binarytrees' else None
            row[language]=dict(status='admitted',fast_key=fast['key'],fast_seconds=fast['median_seconds'],fast_loc=fast['loc'],fewest_key=fewest['key'],fewest_seconds=fewest['median_seconds'],fewest_loc=fewest['loc'],time_ratio_to_mech=fast['median_seconds']/control['median_seconds'],fewest_time_ratio_to_mech=fewest['median_seconds']/control['median_seconds'],previous_fast_seconds=previous_seconds,speedup_from_previous=previous_seconds/fast['median_seconds'] if previous_seconds else None,candidate_count=len(candidates),deliberate_pair=pair,candidates=[dict(key=r['key'],loc=r['loc'],median_seconds=r['median_seconds'],samples_seconds=r['samples_seconds'],candidate_role=r['candidate_role'],selected_fast=r['key']==fast['key'],selected_fewest=r['key']==fewest['key']) for r in candidates])
        rows.append(row)
    coverage={l:[r['benchmark'] for r in rows if r[l]['status']=='admitted'] for l in ('mojo','futhark')}
    assert len(coverage['mojo'])==10 and len(coverage['futhark'])==9
    common=[r for r in rows if all(r[l]['status']=='admitted' for l in ('mojo','futhark'))]
    summary=dict(status='passed-with-documented-allocation-gap',timing_report_sha256=sha(D/'results/timings.json'),source_audit_sha256=sha(D/'source-audit.json'),profile_plan_sha256=sha(D/'profiles.json'),benchmarks=rows,coverage=coverage,geometric_mean_time_ratio_to_mech={l:gm([r[l]['time_ratio_to_mech'] for r in rows if r[l]['status']=='admitted']) for l in ('mojo','futhark')},geometric_mean_fewest_time_ratio_to_mech={l:gm([r[l]['fewest_time_ratio_to_mech'] for r in rows if r[l]['status']=='admitted']) for l in ('mojo','futhark')},common_benchmark_comparison=dict(benchmarks=[r['benchmark'] for r in common],geometric_mean_time_ratio_to_mech={l:gm([r[l]['time_ratio_to_mech'] for r in common]) for l in ('mojo','futhark')}),samples=sum(len(r['samples_seconds']) for r in programs),candidate_sources=len(audit['rows']),distinct_compact_performance_pairs=19,attempted_compact_performance_pairs=20,unsupported=audit['unsupported'],cpu_only=True,max_workers=8,work_audited=True,rejected_tree_result_excluded=True,discarded_samples=len(report.get('discarded',[])))
    (D/'results/summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print('Benchmark       Mojo fast/short LOC    Futhark fast/short LOC   Time / Mech (Mojo, Futhark)')
    for r in rows:
        f=r['futhark'];fl=f"{f['fast_loc']:3}/{f['fewest_loc']:<3}" if f['status']=='admitted' else 'excluded'
        ft=f"{f['time_ratio_to_mech']:.2f}×" if f['status']=='admitted' else 'allocation gap'
        print(f"{r['benchmark']:15} {r['mojo']['fast_loc']:3}/{r['mojo']['fewest_loc']:<3}                {fl:10}              {r['mojo']['time_ratio_to_mech']:.2f}×, {ft}")
    print('Geometric means:',summary['geometric_mean_time_ratio_to_mech'])
    print(summary['samples'],'fresh samples;',summary['candidate_sources'],'admitted candidate sources; nineteen compact/performance pairs; one documented allocation gap')
if __name__=='__main__':main()
