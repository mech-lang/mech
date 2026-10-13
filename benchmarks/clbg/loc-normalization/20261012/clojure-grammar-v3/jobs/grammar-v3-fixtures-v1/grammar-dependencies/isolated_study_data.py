"""Independent full-record reconciliation for additive guarded source studies."""
from copy import deepcopy
import json
import math
import statistics
from mandelbrot_study_data import _fingerprint
POWER=dict(source='AC Power',low_power_mode=0)
EMPTY=('remaining_group_members','competing_compilers','power_changes','monitor_errors','resource_monitor_errors','supervisor_remaining_members')
ERRORS=('error','resource_violation','cleanup_error','monitor_finish_error','supervisor_cleanup','sampler_failure','supervisor_registration_error')

def validate_full_records(manifest,report,digest,*,benchmark,size,locs,workers,reference,environment_retries=False,expected_checks=None):
    """Completion flags cannot replace source/command/output/process evidence."""
    assert report['status']=='complete' and report.get('active') is None
    assert report['manifest_sha256']==digest and report['pins_unchanged_at_end'] is True
    if 'canonical_full_workload' in report:assert report['canonical_full_workload'] is True
    if 'never_chart_eligible' in report:assert report['never_chart_eligible'] is False
    assert manifest['samples']==3 and manifest['size']==size
    assert manifest['required_power_profile']==POWER
    assert manifest['maximum_compute_workers']==workers
    assert all(manifest['full_output_reference'][key]==value for key,value in reference.items())
    expected_validation={key:reference[key] for key in ('bytes','sha256')}|{'reference_match':True}
    rows={row['id']:row for row in manifest['profiles']}
    assert len(rows)==len(manifest['profiles'])==len(locs) and set(rows)==set(locs)
    assert set(report['profiles'])==set(rows) and report['orders']==manifest['orders']
    assert len(manifest['orders'])==3
    expected_order=[]
    for round_index,ordered in enumerate(manifest['orders'],1):
        assert ordered['round']==round_index and len(ordered['profiles'])==len(rows) and set(ordered['profiles'])==set(rows)
        expected_order.extend((round_index,identity) for identity in ordered['profiles'])
    attempts={identity:[] for identity in rows};discarded=[];position=0;retries=0
    for number,attempt in enumerate(report['attempts'],1):
        assert position<len(expected_order),'Extra actual full record'
        round_index,identity=expected_order[position];row=rows[identity]
        assert row['loc']==locs[identity] and row['benchmark']==benchmark
        assert row['maximum_workers']<=workers and row['maximum_workers']>=1
        assert attempt['attempt']==number and attempt['round']==round_index
        for field in ('id','key','benchmark','language','source_sha256','cwd'):assert attempt[field]==row[field],(identity,field)
        command=[part.replace('{n}',str(size)) for part in row['command']]
        assert attempt['command']==command and attempt['runtime_environment']==row.get('runtime_environment',{})
        assert json.loads(attempt['execution_fingerprint'])==_fingerprint(row)
        assert attempt['sampler_wait_status']==attempt['exit_status']==0
        assert attempt['maximum_rss_mib']==manifest['maximum_rss_mib']
        assert type(attempt['peak_process_group_rss_kib']) is int and 0<=attempt['peak_process_group_rss_kib']<=manifest['maximum_rss_mib']*1024
        assert attempt['timeout_seconds']==manifest['timeout_seconds']
        assert attempt['full_workload'] is True and attempt['workload_n']==size
        if 'canonical_full_workload' in attempt:assert attempt['canonical_full_workload'] is True
        if 'never_chart_eligible' in attempt:assert attempt['never_chart_eligible'] is False
        assert attempt['pins_unchanged_at_end'] is True
        assert all(attempt[field]==[] for field in ('remaining_group_members','monitor_errors','resource_monitor_errors','supervisor_remaining_members'))
        assert not any(attempt.get(field) for field in ERRORS)
        assert attempt['validation']==expected_validation and attempt['stdout_bytes']==reference['bytes'] and attempt['stdout_sha256']==reference['sha256']
        native=attempt['supervised_native_group']
        assert type(native['pid']) is int and native['pid']>0 and native['pid']==native['pgid'] and native['command']==command
        assert native['inherited_common_mutex'] is True
        seconds=attempt['seconds'];assert type(seconds) in (int,float) and math.isfinite(seconds) and seconds>0
        if attempt['qualified'] is not True:
            assert environment_retries and attempt['retryable_environment_only'] is True
            assert attempt['valid_environment'] is False and not attempt.get('fatal_qualification_fields')
            assert (attempt['competing_compilers'] or attempt['power_changes'] or attempt['power_at_start']!=POWER or attempt['power_at_end']!=POWER),'No actual environmental exclusion evidence'
            retries+=1;assert retries<manifest['environment_retry_limit'],'Retry allowance exhausted'
            assert attempt['environment_attempt']==retries
            discarded.append(deepcopy(attempt));continue
        assert attempt['valid_environment'] is True and all(attempt[field]==[] for field in EMPTY)
        assert attempt['power_at_start']==attempt['power_at_end']==POWER
        if environment_retries:assert attempt['environment_attempt']==retries+1
        if expected_checks is not None:
            assert set(attempt.get('qualification_checks',{}))==set(expected_checks)
            assert all(value is True for value in attempt['qualification_checks'].values())
        elif 'qualification_checks' in attempt:assert attempt['qualification_checks'] and all(value is True for value in attempt['qualification_checks'].values())
        else:assert attempt['fatal_qualification_fields']==[]
        attempts[identity].append(attempt);position+=1;retries=0
    assert position==len(expected_order) and retries==0,'Required full records are missing'
    result={}
    for identity,row in rows.items():
        recorded=report['profiles'][identity];samples=[a['seconds'] for a in attempts[identity]]
        assert len(samples)==3 and recorded['loc']==row['loc'] and recorded['samples_seconds']==samples
        assert recorded['median_seconds']==statistics.median(samples) and recorded['sample_stdev_seconds']==statistics.stdev(samples)
        result[identity]=dict(deepcopy(row),samples_seconds=samples,median_seconds=statistics.median(samples),sample_stdev_seconds=statistics.stdev(samples),sample_validation=[deepcopy(a['validation']) for a in attempts[identity]],sample_execution_metadata=deepcopy(attempts[identity]))
    return result,discarded
