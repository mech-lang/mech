"""Verify complete MATLAB samples and select actual fastest/fewest-LOC endpoints."""
import json,math,statistics
from pathlib import Path
from prepare import BENCHES,sha
D=Path(__file__).resolve().parent
def main():
    path=D/'results/timings.json';report=json.loads(path.read_text());audit=json.loads((D/'source-audit.json').read_text());built=json.loads((D/'results/builds.json').read_text())
    assert report['status']=='complete' and set(report['completed_benchmarks'])==set(BENCHES)
    assert report['sample_count']==3 and report['power']==dict(source='AC Power',low_power_mode=0)
    loc_path=D/'results/final-loc-audit.json';loc=json.loads(loc_path.read_text())
    assert loc['status']=='passed' and loc['timing_report_sha256']==sha(path)
    assert loc['source_audit_sha256']==sha(D/'source-audit.json') and loc['audit_script_sha256']==sha(D/'audit_loc_final.py')
    assert audit['builds_sha256']==sha(D/'results/builds.json')
    for f,h in report['source_pins'].items():assert sha(D/f)==h,f
    for f,h in audit['evidence'].items():assert sha(D/f)==h,f
    for r in report['runtimes'].values():assert sha(r['path'])==r['sha256']
    for r in report['programs'].values():
        assert sha(r['source'])==r['source_sha256'] and r['full_validation']['reference_match']
        assert len(r['samples_seconds'])==len(r['sample_validation'])==3 and all(v['reference_match'] for v in r['sample_validation'])
        assert r['median_seconds']==statistics.median(r['samples_seconds']) and r['sample_stdev_seconds']==statistics.stdev(r['samples_seconds'])
        if r['language']=='matlab':
            assert r['small_validation']['reference_match'] and r['algorithm']['strict'] and r['algorithm']['native_benchmark_kernel'] is False
            assert loc['rows'][r['key']]['source_sha256']==r['source_sha256'] and loc['rows'][r['key']]['normalized_loc']==r['loc']
        else:assert sha(r['binary'])==r['binary_sha256']
    rows=[];controls={}
    def point(r):return {k:r[k] for k in ('key','id','loc','source_sha256','median_seconds','samples_seconds','sample_stdev_seconds')}
    for bench in BENCHES:
        control=report['programs'][bench+':mech-control']
        controls[bench]=dict(chart_key=control['published_chart_key'],source_sha256=control['source_sha256'],binary_sha256=control['binary_sha256'],previous_seconds=control['previous_seconds'],samples_seconds=control['samples_seconds'],median_seconds=control['median_seconds'],median_ratio=control['median_seconds']/control['previous_seconds'])
        candidates=[r for r in report['programs'].values() if r['benchmark']==bench and r['language']=='matlab'];assert len(candidates)==2
        fast=min(candidates,key=lambda r:(r['median_seconds'],r['loc']));small=min(candidates,key=lambda r:(r['loc'],r['median_seconds']))
        rows.append(dict(benchmark=bench,language='matlab',candidates=2,fast=point(fast),small=point(small),fastest_seconds_over_mech=fast['median_seconds']/control['median_seconds']))
    result=dict(status='passed',timing_report_sha256=sha(path),source_audit_sha256=sha(D/'source-audit.json'),final_loc_audit_sha256=sha(loc_path),admitted_sources=20,rows=rows,controls=controls,runtimes=built['runtimes'],protocol=report['protocol'],discarded_samples=len(report['discarded']),
        fastest_seconds_over_mech_geomean=math.exp(sum(math.log(r['fastest_seconds_over_mech']) for r in rows)/10))
    (D/'results/summary.json').write_text(json.dumps(result,indent=2)+'\n')
    for row in rows:print(row['benchmark'],'fewest',row['small']['loc'],'fastest',row['fast']['loc'],f'{row["fast"]["median_seconds"]:.6f}s',f'{row["fastest_seconds_over_mech"]:.3f}x Mech')
if __name__=='__main__':main()
