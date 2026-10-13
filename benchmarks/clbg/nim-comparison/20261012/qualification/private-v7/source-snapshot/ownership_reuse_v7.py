"""Retain the passed v6 ORC contract and preserve its later map-trace failure."""
import copy
import json
from pathlib import Path

import primitive_reuse_v6 as primitive
import proof_reuse_v5 as production
import source_proofs_v6 as previous
import source_proofs_v7 as corrected
import references

HERE=Path(__file__).resolve().parent
PRIOR=HERE/'qualification/private-v6'
BUILD=HERE/'.build/private-v6'
PRIOR_PROPOSAL=HERE/'proposals/private-v6.json'
PRIOR_SHA='3fd82d63e3549349c4cda792412ee3493a0b0e5615bd940eabd81f3caf4814ff'
IDS=('orc-join-parent-destruction','actual-seven-map-compact')
OBSERVATIONS=('exit_status','seconds','power_at_start','power_at_end','competing_compilers','power_changes',
    'timeout_seconds','maximum_rss_mib','peak_process_group_rss_kib','valid_environment','pins_unchanged_at_end')


def source_correction():
    for name in ('BIGINT_ABI','GMP_ABI','PCRE_ABI','ORC_OWNERSHIP'):
        assert getattr(previous,name)==getattr(corrected,name)
    before=Path(previous.__file__).read_text()
    after=Path(corrected.__file__).read_text()
    before=before[before.index('def reconcile('):before.index('\ndef run(')]
    after=after[after.index('def reconcile('):after.index('\ndef run(')]
    wrong="    import json\n    primitive = json.loads(Path(proposal['prior_primitive_reuse']).read_text())\n    summaries = [dict(p) for p in primitive['proofs']]"
    assert before.count(wrong)==1 and after==before.replace(wrong,'    summaries = []')


def validate_records(report):
    assert report['status']=='stopped-unqualified' and report['active'] is None and report['proposal_sha256']==PRIOR_SHA
    assert report['error_type']=='AssertionError' and report['error']=="('Unqualified attempt retained; no automatic retry', 'actual-seven-map-compact', 'empty')"
    assert report['full_performance_runs']==0 and report['chart_admission'] is False
    assert report['never_chart_eligible'] is True and report['no_automatic_retry'] is True
    assert report['reused_bounded_cases']==154 and report['reused_cases_relabelled_as_attempts'] is False
    assert report['reused_primitive_cases']==3 and report['reused_primitive_cases_relabelled_as_attempts'] is False
    assert len(report['attempts'])==2
    for number,(identifier,attempt) in enumerate(zip(IDS,report['attempts']),1):
        assert attempt['id']==identifier and attempt['attempt']==number
        assert attempt['language']=='nim' and attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        if number==1:
            production.old.validate_observations(attempt)
        else:
            # The exact Python trace error is a post-native failure. Validate
            # native observations on a copy without promoting the failed row.
            assert attempt['source_work_error_type']=='NameError' and attempt['source_work_error']=="name 'proposal' is not defined"
            native_observations=dict(attempt)
            native_observations.pop('source_work_error')
            production.old.validate_observations(native_observations)
        checks=attempt['qualification_checks']
        assert type(checks) is dict and set(checks)==production.old.EXPECTED_GATES
        assert all(type(value) is bool and value is True for value in checks.values())
        assert attempt['sampler_wait_status']==0
        fingerprint=json.loads(attempt['execution_fingerprint'])
        assert set(fingerprint)=={'source_sha256','binary_sha256','command','cwd','environment','support'}
        assert fingerprint['command']==[str(BUILD/'bin'/identifier),'{n}'] and fingerprint['support']=={}
        assert fingerprint['source_sha256']==attempt['source_sha256'] and fingerprint['cwd']==attempt['cwd']==str(HERE)
        assert fingerprint['environment']==attempt['runtime_environment']
        native=attempt['supervised_native_group']
        assert native['type']=='native-group-started' and native['command']==attempt['command'] and native['inherited_common_mutex'] is True
        assert type(native['pid']) is int and native['pid']>0 and native['pid']==native['pgid']
        assert attempt['validation']['reference_match'] is True
        assert (attempt['validation']['bytes'],attempt['validation']['sha256'])==(attempt['stdout_bytes'],attempt['stdout_sha256'])
        if number==1:
            assert attempt['qualified'] is True and attempt['case']=='contract' and attempt['workload_n']==0
            assert attempt['benchmark']=='pidigits' and attempt['key']=='pidigits:'+identifier
            assert attempt['command']==[str(BUILD/'bin'/identifier),'0']
            assert not attempt.get('source_work_error') and not attempt.get('source_work_proof')
        else:
            assert attempt['qualified'] is False and attempt['case']=='empty' and attempt['workload_n']==1000
            assert attempt['benchmark']=='knucleotide' and attempt['key']=='knucleotide:'+identifier
            assert attempt['command']==[str(BUILD/'bin'/identifier),'1000']
            assert attempt['output_qualified_before_source_work'] is True
            assert attempt['source_work_error_type']=='NameError' and attempt['source_work_error']=="name 'proposal' is not defined"
            assert 'source_work_proof' not in attempt
            assert fingerprint['environment']['MECH_NIM_WORK_TRACE']==str(PRIOR/'source-proofs'/identifier/'empty/actual-maps.trace')
    return copy.deepcopy(report['attempts'][0]),copy.deepcopy(report['attempts'][1])


def reconstruct(preparation,guard,harness):
    source_correction()
    assert preparation.sha(PRIOR_PROPOSAL)==PRIOR_SHA
    proposal=json.loads(PRIOR_PROPOSAL.read_text());guard.verify(proposal['pins'])
    prior_primitive_path=Path(proposal['prior_primitive_reuse'])
    assert preparation.sha(prior_primitive_path)==proposal['prior_primitive_reuse_sha256']
    retained_primitive=json.loads(prior_primitive_path.read_text())
    primitive.validate_packet(retained_primitive,preparation,guard,harness)
    report_path=PRIOR/'report.json'
    report=json.loads(report_path.read_text());passed,failed=validate_records(report)
    pins=preparation.merge(proposal['pins'],retained_primitive['pins'],
        {str(PRIOR_PROPOSAL):PRIOR_SHA,str(report_path):preparation.sha(report_path)},
        preparation.pin_tree(PRIOR),preparation.pin_tree(BUILD))
    environment={'LC_ALL':'C','DYLD_LIBRARY_PATH':preparation.LIBRARY_PATH}
    binding=proposal['toolchain_binding']
    assert binding==retained_primitive['toolchain']
    if harness is None: harness=guard.load_harness()
    for attempt in (passed,failed):
        identifier=attempt['id']
        source=PRIOR/'source-snapshot'/(identifier.replace('-','_')+'.nim')
        binary=BUILD/'bin'/identifier
        expected_text=previous.ORC_OWNERSHIP if identifier==IDS[0] else previous.map_copy(
            (PRIOR/'source-snapshot/sources/knucleotide/compact.nim').read_text(),'compact')
        assert source.read_text()==expected_text and preparation.sha(source)==attempt['source_sha256']
        fingerprint=json.loads(attempt['execution_fingerprint'])
        assert preparation.sha(binary)==fingerprint['binary_sha256']
        row=dict(id=identifier,source=str(source),source_sha256=attempt['source_sha256'],
            binary_sha256=fingerprint['binary_sha256'],command=[str(binary),'{n}'],cwd=str(HERE),
            runtime_environment=attempt['runtime_environment'])
        assert json.loads(harness.execution_fingerprint(row))==fingerprint
        directory=PRIOR/'source-proofs'/identifier
        receipt=json.loads((directory/'build.json').read_text())
        command=preparation.build_command(Path(binding['compiler']),Path(binding['toolchain']),Path(binding['bigint']),
            Path(binding['gmp_wrapper']),Path(proposal['sdk']),source,binary,BUILD/'cache'/identifier)
        command[-1:-1]=['--passC:-fsanitize=address','--passL:-fsanitize=address','--passC:-fno-omit-frame-pointer']
        assert receipt['command']==command and receipt['exit_status']==receipt['final_exit_status']==0
        assert receipt['inherited_common_mutex'] is True
        assert not any(receipt.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error','remaining_group_members'))
        image=directory/'images'
        assert (image/'architecture.stdout').read_text().split()==['arm64']
        for basename,command in (('architecture',['/usr/bin/lipo','-archs',str(binary)]),
                ('linked-images',['/usr/bin/otool','-L',str(binary)])):
            metadata=json.loads((image/(basename+'.json')).read_text())
            assert metadata['command']==command and metadata['exit_status']==metadata['final_exit_status']==0
            assert metadata['inherited_common_mutex'] is True
            assert not any(metadata.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error'))
        dependency=json.loads((image/'dependency-review.json').read_text())
        assert dependency['binary']==str(binary) and dependency['architectures']==['arm64']
        linked=[line.strip().split(' (',1)[0] for line in (image/'linked-images.stdout').read_text().splitlines()[1:]]
        assert [value['install_name'] for value in dependency['dependencies']]==linked
        for value in dependency['dependencies']:
            if value['install_name'].startswith(('/usr/lib/','/System/Library/')):
                assert value==dict(install_name=value['install_name'],scope='OS-image-not-hashed')
            else:
                lookup=Path(value['lookup_path']);target=lookup.resolve(strict=True)
                assert str(target)==value['path']
                assert any(target.is_relative_to(root.resolve()) for root in (
                    Path(binding['toolchain']),preparation.GMP,preparation.PCRE,preparation.CLANG.parent.parent))
                assert preparation.sha(lookup)==preparation.sha(target)==value['sha256']
                pins=preparation.merge(pins,{str(lookup):value['sha256'],str(target):value['sha256']})
        raw=PRIOR/'attempts'/('%03d-%s.json'%(attempt['attempt'],identifier))
        supervisor=raw.with_suffix('.supervisor.json')
        assert attempt['result_path']==str(raw) and attempt['supervisor_result_path']==str(supervisor)
        sampled,enriched=json.loads(raw.read_text()),json.loads(supervisor.read_text())
        for record in (sampled,enriched): production.old.validate_observations(record)
        for key in OBSERVATIONS+('command','cwd','runtime_environment','source_sha256','execution_fingerprint','stdout_bytes','stdout_sha256','validation'):
            assert sampled[key]==enriched[key]==attempt[key]
        assert enriched['qualified'] is True
        assert enriched['qualification_checks']==attempt['qualification_checks']
        assert enriched['supervised_native_group']==attempt['supervised_native_group'] and enriched['sampler_wait_status']==0
        case=directory/attempt['case']
        expected=b'orc-seven-worker-join-parent-destruction-ok\n' if identifier==IDS[0] else references.knucleotide(b'>THREE')
        assert (case/'actual.stdout').read_bytes()==(case/'expected.stdout').read_bytes()==expected
        assert preparation.sha(case/'actual.stdout')==attempt['stdout_sha256']
    failed_directory=PRIOR/'source-proofs'/IDS[1]/'empty'
    assert (failed_directory/'stdin.fasta').read_bytes()==b'>THREE'
    # Regression replay validates only the parser repair. The original failed
    # wrapper record remains failed and supplies no retained passed proof.
    diagnostic=corrected.reconcile(failed_directory/'actual-maps.trace',b'>THREE','compact')
    assert len(diagnostic['tables'])==7 and all(row['keys']==row['windows']==0 for row in diagnostic['tables'])
    guard.verify(pins)
    return dict(status='verified-one-exact-passed-v6-ORC-contract',
        prior_report=str(report_path),prior_report_sha256=pins[str(report_path)],prior_proposal=str(PRIOR_PROPOSAL),prior_proposal_sha256=PRIOR_SHA,
        proofs=[dict(proof=IDS[0],status='passed',result=passed['supervisor_result_path'],asan=True,
            application_full_workload=False,origin='retained-private-v6')],
        reused_ownership_cases=1,native_cases_reexecuted=0,old_cases_relabelled_as_new_attempts=False,
        preserved_failed_map_trace=dict(original_wrapper_record=failed,original_report=str(report_path),
            original_raw_result=failed['result_path'],original_supervisor_result=failed['supervisor_result_path'],
            original_trace=str(failed_directory/'actual-maps.trace'),original_trace_sha256=pins[str(failed_directory/'actual-maps.trace')],
            corrected_parser_regression_only=diagnostic,failed_record_promoted_to_passed=False,retained_as_passed_source_proof=False),
        recorded_in_completed_preparation_pins=False,newly_captured_preserved_native_evidence=True,
        pins=pins,full_performance_runs=0,chart_admission=False)


def validate_packet(packet,preparation,guard):
    guard.verify(packet['pins'])
    assert packet==reconstruct(preparation,guard,None)
    return packet
