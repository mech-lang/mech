"""Verify completed Swift samples and select smallest/fastest faithful endpoints."""
import json,statistics,math
from pathlib import Path
import run as runner
D=Path(__file__).resolve().parent;sha=runner.sha
def main():
    path=D/'results/timings.json';report=json.loads(path.read_text())
    audit=json.loads((D/'source-audit.json').read_text())
    assert report['status']=='complete' and set(report['completed_benchmarks'])==set(runner.BENCHES)
    assert report['source_pins']['source-audit.json']==sha(D/'source-audit.json')
    for filename,digest in report['source_pins'].items():assert sha(D/filename)==digest,filename
    for filename,digest in audit['ir_pins'].items():assert sha(D/filename)==digest,filename
    for row in report['programs'].values():
        if not row['eligible_for_chart']:continue
        assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
        assert row['small_validation']['reference_match'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds'])==len(row['sample_validation'])==3
        assert all(v['reference_match'] for v in row['sample_validation'])
        assert row['median_seconds']==statistics.median(row['samples_seconds'])
        assert row['sample_stdev_seconds']==statistics.stdev(row['samples_seconds'])
    rows=[];controls={}
    for bench in runner.BENCHES:
        candidates=[r for r in report['programs'].values() if r['benchmark']==bench and r['language']=='swift' and r['eligible_for_chart']]
        fast=min(candidates,key=lambda r:(r['median_seconds'],r['loc']))
        small=min(candidates,key=lambda r:(r['loc'],r['median_seconds']))
        control=report['programs'][bench+':mech-control']
        controls[bench]=dict(chart_key=control['published_chart_key'],source_sha256=control['source_sha256'],
            binary_sha256=control['binary_sha256'],previous_seconds=control['previous_seconds'],
            samples_seconds=control['samples_seconds'],median_seconds=control['median_seconds'],
            median_ratio=control['median_seconds']/control['previous_seconds'])
        def point(row):return {k:row[k] for k in ('key','id','loc','source_sha256','binary_sha256','median_seconds','samples_seconds','sample_stdev_seconds')}
        rows.append(dict(benchmark=bench,candidates=len(candidates),fast=point(fast),small=point(small),
            fastest_seconds_over_mech=fast['median_seconds']/control['median_seconds']))
    result=dict(status='passed',timing_report_sha256=sha(path),source_audit_sha256=sha(D/'source-audit.json'),
        admitted_sources=sum(r['language']=='swift' and r['eligible_for_chart'] for r in report['programs'].values()),
        output_exclusions=[r for r in report['excluded'] if r.get('stage')],excluded=report['excluded'],
        rows=rows,controls=controls,
        fastest_seconds_over_mech_geomean=math.exp(sum(math.log(r['fastest_seconds_over_mech']) for r in rows)/len(rows)),
        protocol=report['protocol'],compiler=report['compiler'],discarded_samples=len(report['discarded']))
    (D/'results/summary.json').write_text(json.dumps(result,indent=2)+'\n')
    print('Swift candidates',result['admitted_sources'],'completed benchmarks',len(rows),'discarded samples',result['discarded_samples'])
    for row in rows:print(row['benchmark'],'fewest',row['small']['loc'],'fastest',row['fast']['loc'],f'{row["fast"]["median_seconds"]:.6f}s',f'{row["fastest_seconds_over_mech"]:.3f}x Mech')
if __name__=='__main__':main()
