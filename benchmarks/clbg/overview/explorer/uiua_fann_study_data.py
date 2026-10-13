"""Add fully qualified composed Uiua fannkuch sources without rewriting cohorts."""
from copy import deepcopy
import json
from pathlib import Path
from isolated_study_data import validate_full_records
COHORT='r-apl-uiua-comparison/20261011/uiua-fann-compact-v5'
MANIFEST=COHORT+'/full-study/manifest.json'
REPORT=COHORT+'/full-study/report.json'
DIGEST='d8b6ad100671b3fef5ae46d90dcb36b044733b25ea8d65e3ba7a4589e5b62513'
CANDIDATE_SHA='41462f55b1c1f6c800d08cca9ba9ba46cb5e3c6efd7f0ff3d21cfde514b70b7c'
LOCS={'uiua-compact':20,'uiua-fast':21,'mech59-aot-control':59}
REFERENCE=dict(bytes=29,sha256='4265a65135c506a68d90d6474003fb9030b7ee244a06c046bd89b3932a28ce20',checksum=3968050,maximum=65)
CONTROL_SHA='b8a2ce6509db36487e42bb3ded0e054b4cde743e05cb05c94f8f3c6e45780ebe'
CHECKS={'sampler_exited_normally','authoritative_native_group_received','native_inherited_common_mutex','native_exited_normally','no_execution_error','no_resource_violation','no_resource_monitor_errors','no_compiler_power_monitor_errors','no_monitor_finish_error','no_execute_cleanup_error','no_remaining_execute_group','no_supervisor_cleanup','no_supervisor_remaining_group','valid_environment','actual_full_workload','exact_command_and_group','complete_output_reference_match','complete_output_bytes_and_hash','pins_unchanged'}

def add_uiua_fann_study(root,data,load,measured,add_file,sha):
    root=Path(root);path=root/REPORT
    if not path.is_file():return
    # Advancing reports contribute no partial endpoint or packaging provenance.
    raw=json.loads(path.read_text())
    if raw.get('status')!='complete':return
    assert sha(root/MANIFEST)==DIGEST,'Reviewed complete-study manifest changed'
    manifest=load(MANIFEST);report=load(REPORT)
    assert Path(report['manifest']).resolve()==(root/MANIFEST).resolve()
    rows,discarded=validate_full_records(manifest,report,DIGEST,benchmark='fannkuchredux',size=12,locs=LOCS,workers=8,reference=REFERENCE,expected_checks=CHECKS)
    assert not discarded and len(report['attempts'])==9 and manifest['canonical_full_workload'] is True and manifest['full_permutations']==479001600
    candidate_path=COHORT+'/manifest.json';assert sha(root/candidate_path)==manifest['candidate_manifest_sha256']==CANDIDATE_SHA
    candidate=load(candidate_path);assert candidate['status']=='prepared-validated' and candidate['full_performance_runs']==0
    programs={row['source_profile']:row for row in candidate['programs']}
    assert set(programs)=={'compact','fast'}
    assert sha(candidate['source_audit'])==candidate['source_audit_sha256']
    source_audit=load(Path(candidate['source_audit']).relative_to(root).as_posix())
    assert source_audit['status']=='passed-source-audit'
    assert source_audit['native_benchmark_kernel'] is False
    assert source_audit['new_host_helpers'] is False
    assert source_audit['permutation_cache'] is False and source_audit['flip_result_cache'] is False
    assert source_audit['input_specific_shortcuts'] is False
    assert source_audit['parent_only_output'] is True
    assert sha(manifest['runner_qualification'])==manifest['runner_qualification_sha256']
    qualification=load(Path(manifest['runner_qualification']).relative_to(root).as_posix())
    assert qualification['status']=='passed'
    for filename,digest in manifest['pins'].items():assert sha(filename)==digest,('Study input differs',filename)
    runtime=candidate['runtime'];assert sha(runtime['path'])==runtime['sha256']
    control=rows['mech59-aot-control'];assert control['source_sha256']==CONTROL_SHA and control['loc']==59 and control['language']=='mech'
    assert manifest['pins'][control['source']]==control['source_sha256'] and manifest['pins'][control['binary']]==control['binary_sha256']
    for role in ('compact','fast'):
        row=rows['uiua-'+role];original=programs[role]
        for field in ('source','source_sha256','command','runtime_environment','loc','maximum_workers'):assert row[field]==original[field],(role,field)
        assert row['language']=='uiua' and row['command'][0]==runtime['path']
        assert original['algorithm']['strict'] is True and original['algorithm']['native_benchmark_kernel'] is False
        assert original['loc_expansions']==[] and original['physical_loc']==row['loc']
        assert sha(original['validation_evidence']['path'])==original['validation_evidence']['sha256']
    candidates=[rows['uiua-compact'],rows['uiua-fast']]
    fast=min(candidates,key=lambda row:(row['median_seconds'],row['loc'],row['key']))
    small=min(candidates,key=lambda row:(row['loc'],row['median_seconds'],row['key']))
    benchmark=next(b for b in data['benchmarks'] if b['key']=='fannkuchredux')
    group=next((g for g in benchmark['groups'] if g['language']=='uiua'),None)
    previous=None
    if group:
        previous=dict(programs=[deepcopy(data['programs'][key]) for key in group['programs']],files={digest:deepcopy(data['files'][digest]) for key in group['programs'] for digest in data['programs'][key]['files']})
    else:
        group=dict(language='uiua',label='Uiua',programs=[]);benchmark['groups'].append(group)
    for row in {r['key']:r for r in (fast,small)}.values():
        original=programs['compact' if row['id']=='uiua-compact' else 'fast']
        measured(row['key'],'fannkuchredux','uiua',row['loc'],row['samples_seconds'],row['source'],row['source_sha256'],REPORT,MANIFEST,
          algorithm=deepcopy(original['algorithm']),required_work_validated=True,loc_metric='statement-normalized',physical_loc=original['physical_loc'],loc_expansions=[],
          source_provenance=original['provenance'],adaptations=original['adaptations'],runtime_policy='Uiua 0.19.1 · official native ARM64 interpreter',runtime=deepcopy(runtime),maximum_workers=8,
          runtime_arguments=row['command'],runtime_environment=row['runtime_environment'],full_output_validation=row['sample_validation'],sample_execution_metadata=row['sample_execution_metadata'],
          study_manifest_sha256=DIGEST,study_report_sha256=sha(path),timing_boundary=manifest['timing_boundary'],
          mech_comparison=dict(scope='paired-control',seconds=control['median_seconds'],control_key=control['key'],control_loc=59,source=control['source'],source_sha256=control['source_sha256'],samples_seconds=control['samples_seconds'],report=REPORT,source_audit=MANIFEST,sample_execution_metadata=control['sample_execution_metadata'],time_x_mech=row['median_seconds']/control['median_seconds'],control_loc_x_mech=row['loc']/59))
    group['programs']=list(dict.fromkeys([fast['key'],small['key']]))
    sources={digest:deepcopy(data['files'][digest]) for row in rows.values() for digest in add_file(row['source'],row['source_sha256'])}
    data['uiua_fann_study_audit']=dict(status='complete',manifest=MANIFEST,manifest_sha256=DIGEST,report=REPORT,report_sha256=sha(path),selected=dict(fast=fast['key'],small=small['key']),profiles=rows,study_sources=sources,previous_endpoints=previous,required_work=dict(n=12,permutations=479001600,actual_prefix_flips=True,global_checksum_parity=True,maximum_compute_workers=8),
      admission_rule='All nine unchanged full fresh-process samples independently reconciled; original suite reports/audits and unmeasured v2 sources remain unchanged.',timing_boundary=manifest['timing_boundary'])
    data['provenance']['uiua_fann_study_data.py']=sha(Path(__file__))
    data['provenance']['isolated_study_data.py']=sha(Path(__file__).with_name('isolated_study_data.py'))
    data['protocol']['uiua_fann_study']='Separate guarded three-round N12 study, native whole-block factoradic gathering and actual prefix reversals in Uiua arrays; standard Spawn/Wait caps compute workers at eight. Every 12! rank and its global checksum sign are evaluated. The freshly timed unchanged 59-LOC Mech AOT control remains named separately. No partial, scaled, pilot or old candidate scores are admitted.'
