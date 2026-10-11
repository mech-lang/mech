"""Freeze only MATLAB benchmarks with three complete, validated samples per source.

Never modifies the runner's report. An immutable export makes an interim chart
reproducible while further measurements continue. Unfinished tests contribute no
timing or LOC endpoint, and coverage is explicit.
"""
import hashlib,json,math,statistics
from datetime import datetime,timezone
from pathlib import Path
from prepare import sha
from audit_loc_final import metrics
D=Path(__file__).resolve().parent
def write(path,value):path.write_text(json.dumps(value,indent=2)+'\n')
def main():
    raw=(D/'results/timings.json').read_bytes();original=json.loads(raw)
    audit=json.loads((D/'source-audit.json').read_text());built=json.loads((D/'results/builds.json').read_text())
    assert original['sample_count']==3 and original['power']==dict(source='AC Power',low_power_mode=0)
    assert audit['status']=='reviewed' and audit['builds_sha256']==sha(D/'results/builds.json')
    for file,digest in original['source_pins'].items():assert sha(D/file)==digest,file
    for file,digest in audit['evidence'].items():assert sha(D/file)==digest,file
    for runtime in original['runtimes'].values():assert sha(runtime['path'])==runtime['sha256']
    completed=original['completed_benchmarks'];assert completed
    rows={key:row for key,row in original['programs'].items() if row['benchmark'] in completed}
    for row in rows.values():
        assert sha(row['source'])==row['source_sha256'] and row['full_validation']['reference_match']
        assert len(row['samples_seconds'])==len(row['sample_validation'])==3
        assert all(sample['reference_match'] for sample in row['sample_validation'])
        assert row['median_seconds']==statistics.median(row['samples_seconds'])
        assert row['sample_stdev_seconds']==statistics.stdev(row['samples_seconds'])
        if row['language']=='matlab':
            assert row['small_validation']['reference_match'] and row['algorithm']['strict']
            assert row['algorithm']['native_benchmark_kernel'] is False
        else:assert sha(row['binary'])==row['binary_sha256']
    assert len(rows)==3*len(completed)
    stamp=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ');out=D/'exports'/stamp;out.mkdir(parents=True)
    (out/'measurement-snapshot.json').write_bytes(raw)
    status='complete' if len(completed)==10 else 'partial'
    report=dict(original,status=status,measurement_status=original['status'],programs=rows,
                measurement_snapshot_sha256=hashlib.sha256(raw).hexdigest(),exported_at=stamp)
    write(out/'timings.json',report)
    loc_rows={}
    for row in built['programs']:
        assert sha(row['source'])==row['source_sha256']
        metric=metrics(Path(row['source']).read_text())
        assert metric['normalized_loc']==row['loc_audit']['normalized_loc'] and metric['physical_loc']==row['loc_audit']['physical_loc']
        loc_rows[row['key']]=dict(source_sha256=row['source_sha256'],**metric)
    assert len(loc_rows)==20
    loc=dict(status='passed',timing_report_sha256=sha(out/'timings.json'),source_audit_sha256=sha(D/'source-audit.json'),
             audit_script_sha256=sha(D/'audit_loc_final.py'),rows=loc_rows,
             measured_benchmarks=completed,attempted_sources=20,measured_sources=2*len(completed))
    write(out/'loc.json',loc)
    selected=[];controls={}
    def point(row):return {key:row[key] for key in ('key','id','loc','source_sha256','median_seconds','samples_seconds','sample_stdev_seconds')}
    for bench in completed:
        candidates=[row for row in rows.values() if row['benchmark']==bench and row['language']=='matlab'];assert len(candidates)==2
        fast=min(candidates,key=lambda row:(row['median_seconds'],row['loc']))
        small=min(candidates,key=lambda row:(row['loc'],row['median_seconds']))
        control=rows[bench+':mech-control']
        controls[bench]=dict(chart_key=control['published_chart_key'],source_sha256=control['source_sha256'],binary_sha256=control['binary_sha256'],
                            previous_seconds=control['previous_seconds'],samples_seconds=control['samples_seconds'],median_seconds=control['median_seconds'],
                            median_ratio=control['median_seconds']/control['previous_seconds'])
        selected.append(dict(benchmark=bench,language='matlab',candidates=2,fast=point(fast),small=point(small),
                             fastest_seconds_over_mech=fast['median_seconds']/control['median_seconds']))
    summary=dict(status='passed',coverage_status=status,timing_report_sha256=sha(out/'timings.json'),source_audit_sha256=sha(D/'source-audit.json'),
                 final_loc_audit_sha256=sha(out/'loc.json'),admitted_sources=2*len(completed),attempted_sources=20,completed_benchmarks=completed,
                 rows=selected,controls=controls,runtimes=built['runtimes'],protocol=original['protocol'],discarded_samples=len(original['discarded']),
                 fastest_seconds_over_mech_geomean=math.exp(sum(math.log(row['fastest_seconds_over_mech']) for row in selected)/len(completed)),
                 mean_benchmark_count=len(completed))
    write(out/'summary.json',summary)
    pointer=dict(directory=str(out.relative_to(D)),status=status,completed_benchmarks=completed,
                 report_sha256=sha(out/'timings.json'),loc_sha256=sha(out/'loc.json'),summary_sha256=sha(out/'summary.json'),
                 exporter_sha256=sha(Path(__file__)),measurement_snapshot_sha256=sha(out/'measurement-snapshot.json'))
    temporary=D/'chart-export.json.tmp';write(temporary,pointer);temporary.replace(D/'chart-export.json')
    print(json.dumps(dict(status=status,measured_benchmarks=len(completed),measured_sources=len(completed)*2,directory=str(out))))
if __name__=='__main__':main()
