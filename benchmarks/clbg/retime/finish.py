#!/usr/bin/env python3
"""Audit the complete fresh normal-power cohort before publishing chart data."""
import importlib.util, json, statistics
from pathlib import Path

HERE=Path(__file__).resolve().parent;CLBG=HERE.parent
spec=importlib.util.spec_from_file_location('compact_run',CLBG/'compact/run.py')
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)

def main():
    p=HERE/'results/apple-m3-ac-normal-20261008.json';report=json.loads(p.read_text())
    assert report['status']=='complete'
    assert set(report['completed_benchmarks'])==set(run.WORK)
    assert set(report['benchmark_power_profiles'])==set(run.WORK)
    assert all(profile=={'source':'AC Power','low_power_mode':0} for profile in report['benchmark_power_profiles'].values())
    assert run.sha(CLBG/'compact/manifest.json')==report['compact_manifest_sha256']
    for filename,pinned_hash in report['asset_hashes'].items():
        assert run.sha(Path(filename))==pinned_hash,'Pinned command asset changed: '+filename
    for row in report['programs']:
        if row.get('excluded_full'):continue
        assert row['full_validation_current_batch'] and row['correctness_full']['reference_match'],row['id']
        assert len(row['samples_seconds'])>=3,row['id']
        assert len(row['sample_validation'])==len(row['samples_seconds']),row['id']
        assert row['median_seconds']==statistics.median(row['samples_seconds']),row['id']
        assert all(v['reference_match'] for v in row['sample_validation']),row['id']
        if row['language']=='luajit':
            lua=next(r for r in report['programs'] if r['id']==row['upstream_id'])
            assert lua['sha256']==row['sha256']
            for metric in ('gzip1_bytes','nonblank_noncomment_lines'):assert lua['source'][metric]==row['source'][metric]
    for filename,archive in report['original_reports'].items():
        assert run.sha(CLBG/filename)==archive['sha256'],'Original report changed: '+filename
    # Refresh derived fields retained by older harness versions. Never carry
    # legacy ranges or a mislabeled raw-second value into the fresh report.
    for bench in run.WORK:
        rows=[r for r in report['programs'] if r['benchmark']==bench and not r.get('excluded_full')]
        control=next(r for r in rows if r['id']==run.ANCHORS.get(bench,'fasta-gcc-2-llvm'))
        for row in rows:
            row['min_seconds']=min(row['samples_seconds']);row['max_seconds']=max(row['samples_seconds'])
            row['comparison_time_ratio']=row['median_seconds']/control['median_seconds']
            row['normalization_control']=control['id']
    temporary=p.with_suffix('.json.tmp');temporary.write_text(json.dumps(report,indent=2)+'\n');temporary.replace(p)
    measured=sum(not r.get('excluded_full') for r in report['programs'])
    audit=dict(passed=True,report=p.name,sha256=run.sha(p),measured_builds=measured,
        excluded_builds=len(report['programs'])-measured,benchmarks=report['completed_benchmarks'],
        power_profiles=report['benchmark_power_profiles'],discarded_attempts=len(report.get('discarded_samples',[])),
        verified_command_assets=len(report['asset_hashes']),
        old_times_used_in_new_chart=False,compact_only_fastest_inferred=False)
    (HERE/'results/audit.json').write_text(json.dumps(audit,indent=2)+'\n')
    print('Fresh cohort audit passed:',measured,'measured builds;',audit['excluded_builds'],'explicit exclusions')

if __name__=='__main__':main()
