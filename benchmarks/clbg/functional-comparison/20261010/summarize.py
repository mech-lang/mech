"""Verify all completed functional cohorts and retain fastest/fewest-LOC endpoints."""
import json,math,statistics
from pathlib import Path
import run as runner
D=Path(__file__).resolve().parent;sha=runner.sha
def main():
    path=D/'results/timings.json';report=json.loads(path.read_text());audit=json.loads((D/'source-audit.json').read_text())
    assert report['status']=='complete' and set(report['completed_benchmarks'])==set(runner.BENCHES)
    final_loc=json.loads((D/'results/final-loc-audit.json').read_text())
    assert final_loc['status']=='passed' and final_loc['timing_report_sha256']==sha(path)
    assert final_loc['source_audit_sha256']==sha(D/'source-audit.json')
    assert final_loc['audit_script_sha256']==sha(D/'audit_loc_final.py')
    assert report['source_pins']['source-audit.json']==sha(D/'source-audit.json')
    for f,h in report['source_pins'].items():assert sha(D/f)==h,f
    for r in audit['allocation_evidence']:assert sha(D/r['file'])==r['sha256']
    for r in report['programs'].values():
        if not r['eligible_for_chart']:continue
        assert sha(r['source'])==r['source_sha256'] and sha(r['binary'])==r['binary_sha256']
        assert r['small_validation']['reference_match'] and r['full_validation']['reference_match']
        assert len(r['samples_seconds'])==len(r['sample_validation'])==3
        assert all(v['reference_match'] for v in r['sample_validation'])
        assert r['median_seconds']==statistics.median(r['samples_seconds'])
        assert r['sample_stdev_seconds']==statistics.stdev(r['samples_seconds'])
    rows=[];controls={}
    def point(r):return {k:r[k] for k in ('key','id','loc','source_sha256','binary_sha256','median_seconds','samples_seconds','sample_stdev_seconds')}
    for bench in runner.BENCHES:
        control=report['programs'][bench+':mech-control']
        controls[bench]=dict(chart_key=control['published_chart_key'],source_sha256=control['source_sha256'],binary_sha256=control['binary_sha256'],previous_seconds=control['previous_seconds'],samples_seconds=control['samples_seconds'],median_seconds=control['median_seconds'],median_ratio=control['median_seconds']/control['previous_seconds'])
        for lang in ('haskell','ocaml'):
            candidates=[dict(r,loc=final_loc['rows'][r['key']]['normalized_loc']) for r in report['programs'].values() if r['benchmark']==bench and r['language']==lang and r['eligible_for_chart']]
            assert candidates,(bench,lang)
            fast=min(candidates,key=lambda r:(r['median_seconds'],r['loc']));small=min(candidates,key=lambda r:(r['loc'],r['median_seconds']))
            rows.append(dict(benchmark=bench,language=lang,candidates=len(candidates),fast=point(fast),small=point(small),fastest_seconds_over_mech=fast['median_seconds']/control['median_seconds']))
    result=dict(status='passed',timing_report_sha256=sha(path),source_audit_sha256=sha(D/'source-audit.json'),final_loc_audit_sha256=sha(D/'results/final-loc-audit.json'),admitted_sources=sum(r['language'] in ('haskell','ocaml') and r['eligible_for_chart'] for r in report['programs'].values()),
        rows=rows,controls=controls,output_exclusions=[r for r in report['excluded'] if r.get('stage')],excluded=report['excluded'],compilers=report['compilers'],protocol=report['protocol'],discarded_samples=len(report['discarded']),
        fastest_seconds_over_mech_geomeans={lang:math.exp(sum(math.log(r['fastest_seconds_over_mech']) for r in rows if r['language']==lang)/10) for lang in ('haskell','ocaml')})
    (D/'results/summary.json').write_text(json.dumps(result,indent=2)+'\n')
    for row in rows:print(row['language'],row['benchmark'],'fewest',row['small']['loc'],'fastest',row['fast']['loc'],f'{row["fast"]["median_seconds"]:.6f}s',f'{row["fastest_seconds_over_mech"]:.3f}x Mech')
if __name__=='__main__':main()
