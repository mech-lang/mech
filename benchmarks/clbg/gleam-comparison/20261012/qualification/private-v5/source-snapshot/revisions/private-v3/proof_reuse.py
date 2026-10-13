"""Reuse one exact complete prior Gleam app/four cases; preserve its next failure."""
import copy
import importlib.util
import json
import math
from pathlib import Path
import tarfile
import io

HERE=Path(__file__).resolve().parents[2]
REVISION=Path(__file__).resolve().parent
PRIOR=HERE/'qualification/private-v1'
BUILD=HERE/'.build/private-v1'
PRIOR_PROPOSAL=HERE/'proposals/private-v1.json'
PRIOR_SHA='3d87b9cb4b6353794735b1656930042767a855b8c51077aeb78c35d2d6aad51c'
FAILED_SEAL=HERE/'proposal-seals/private-v2'
FAILED_SEAL_TOOL_OUTPUT=HERE.parents[1]/'continuation/gleam-private-v2-seal-tool-output-20261013.json'
REPAIRS={
 'binarytrees/optimized.gleam':[('(index + 1)','{ index + 1 }')],
 'fannkuchredux/optimized.gleam':[('(worker + 1)','{ worker + 1 }')],
 'mandelbrot/optimized.gleam':[('(worker + 1)','{ worker + 1 }')],
 'spectralnorm/compact.gleam':[('(diagonal + 1)','{ diagonal + 1 }')],
 'spectralnorm/optimized.gleam':[('(diagonal + 1)','{ diagonal + 1 }'),('(worker + 1)','{ worker + 1 }')],
}
GATES={
 'sampler_exited_normally','authoritative_native_group_received','native_inherited_common_mutex',
 'native_exited_normally','no_execution_error','no_wrapper_integrity_error','no_resource_violation',
 'no_resource_monitor_errors','no_compiler_power_monitor_errors','no_monitor_finish_error',
 'no_execute_cleanup_error','no_remaining_execute_group','no_supervisor_cleanup',
 'no_supervisor_remaining_group','valid_environment','actual_full_workload','exact_command_and_group',
 'complete_output_reference_match','complete_output_bytes_and_hash','pins_unchanged',
}
OBSERVATIONS=('exit_status','seconds','power_at_start','power_at_end','competing_compilers','power_changes',
 'timeout_seconds','maximum_rss_mib','peak_process_group_rss_kib','valid_environment','pins_unchanged_at_end')
FATAL=('error','resource_violation','monitor_errors','resource_monitor_errors','cleanup_error','supervisor_cleanup_error',
 'remaining_group_members','supervisor_remaining_members','supervisor_pin_failure','source_work_error','source_work_failure',
 'preparation_supervisor_failure','sampler_failure','sampler_error','registration_error','supervisor_error','monitor_finish_error',
 'supervisor_cleanup','execute_group_members_after_cleanup','monitor_finish')


def validate_repairs(preparation):
    old=json.loads((HERE/'profiles.json').read_text())
    new=json.loads((REVISION/'profiles.json').read_text())
    expected=copy.deepcopy(old)
    for row in expected['profiles']:
        relative=row['source'].removeprefix('sources/')
        if relative in REPAIRS:
            before=row['source'];after='revisions/private-v2/sources/'+relative
            row['source']=after
            row['compiled_source_bundle']=[after if p==before else p for p in row['compiled_source_bundle']]
    assert new==expected
    for relative,edits in REPAIRS.items():
        before=(HERE/'sources'/relative).read_text()
        for old,new in edits:
            assert before.count(old)==1
            before=before.replace(old,new)
        assert (REVISION/'sources'/relative).read_text()==before
    assert sum(map(len,REPAIRS.values()))==6
    assert callable(preparation.inputs)
    controller=Path(preparation.__file__).read_text()
    assert '                inputs = []' not in controller and '                compiled_inputs = []' in controller


def observations(record):
    assert type(record['exit_status']) is int and record['exit_status']==0
    assert type(record['seconds']) in (float,int) and math.isfinite(record['seconds']) and record['seconds']>0
    assert record['power_at_start']==record['power_at_end']==dict(source='AC Power',low_power_mode=0)
    assert record['competing_compilers']==record['power_changes']==[]
    assert record['timeout_seconds']==120 and record['maximum_rss_mib']==10240
    assert type(record['peak_process_group_rss_kib']) is int and 0<=record['peak_process_group_rss_kib']<=10240*1024
    assert record['valid_environment'] is True and record['pins_unchanged_at_end'] is True
    assert not any(record.get(key) for key in FATAL)


def historical_fingerprint(row):
    # The frozen producer's support_pins(row) discovers explicit path/sha256
    # source-bundle entries. Its arbitrary row.support map is independently
    # pinned runtime evidence, not that historical fingerprint projection.
    bundle=row['source_bundle']
    assert type(bundle) is list and bundle and len({item['path'] for item in bundle})==len(bundle)
    assert bundle[0]['path']==row['source'] and bundle[0]['sha256']==row['source_sha256']
    support={item['path']:item['sha256'] for item in bundle}
    assert all(type(path) is str and path and type(value) is str and value for path,value in support.items())
    return dict(binary_sha256=row['binary_sha256'],
        command=[part.replace(row['source'],'{source}') for part in row['command']],cwd=row['cwd'],
        environment=row['runtime_environment'],source_sha256=row['source_sha256'],support=support)


def validate_failed_tool_output(value):
    assert value['status']=='preserved-original-tool-output' and type(value['session_id']) is int and value['session_id']==2438
    assert value['source_measurements_mutated'] is False and value['chart_admission'] is False
    record=value['original_rollout_record']
    assert record['timestamp']=='2026-10-13T00:46:35.216Z' and record['type']=='response_item'
    assert record['payload']['type']=='custom_tool_call_output'
    assert record['metadata']['client_authored'] is False
    outputs=[]
    for block in record['payload']['output']:
        assert block['type']=='input_text' and type(block['text']) is str
        if block['text'].startswith('{'):
            item=json.loads(block['text'])
            if item.get('value',{}).get('chunk_id')=='952a6c': outputs.append(item)
    assert len(outputs)==1
    item=outputs[0]
    assert item['i']==4 and item['status']=='fulfilled' and type(item['value']['exit_code']) is int and item['value']['exit_code']==1
    trace=item['value']['output']
    assert type(trace) is str and trace.startswith('Traceback (most recent call last):\n') and trace.endswith('AssertionError\n')
    assert str(HERE/'revisions/private-v2/guarded_prepare.py') in trace
    assert str(HERE/'revisions/private-v2/proof_reuse.py') in trace
    assert 'line 89, in validate_records' in trace
    assert "assert json.loads(attempt['execution_fingerprint'])==fingerprint" in trace
    return copy.deepcopy(item['value'])


def preserve_failed_seal(preparation,service,guard):
    assert not (HERE/'proposals/private-v2.json').exists()
    assert not (HERE/'qualification/private-v2').exists() and not (HERE/'.build/private-v2').exists()
    archive=json.loads(FAILED_SEAL_TOOL_OUTPUT.read_text())
    relevant=validate_failed_tool_output(archive)
    import fixture_runner
    local_ids=fixture_runner.INVENTORY[:-1]
    assert len(local_ids)==8
    generic=json.loads((FAILED_SEAL/'generic-ownership-inventory.json').read_text())
    local=json.loads((FAILED_SEAL/'gleam-bundle-inventory.json').read_text())
    for value,ids in ((local,local_ids),(generic,generic['expected_test_ids'])):
        assert value['status']=='passed' and value['successful'] is True
        assert type(value['tests_run']) is type(value['discovered_tests']) is int
        assert value['tests_run']==value['discovered_tests']==len(ids)
        assert value['expected_test_ids']==ids and len(value['actual_test_ids'])==len(set(value['actual_test_ids']))==len(ids)
        assert set(value['actual_test_ids'])==set(ids)
        assert all(value[key]==[] for key in ('skipped','errors','failures','expected_failures','unexpected_successes'))
    assert len(generic['expected_test_ids'])==11
    generic_spec=importlib.util.spec_from_file_location('gleam_retained_generic_fixture_inventory',preparation.SERVICE_FIXTURES)
    generic_module=importlib.util.module_from_spec(generic_spec);generic_spec.loader.exec_module(generic_module)
    generic_module.validate_receipt(generic,'ownership')
    commands={
        'generic-ownership-fixtures':['-B',str(preparation.SERVICE_FIXTURES),'--set','ownership','--receipt',str(FAILED_SEAL/'generic-ownership-inventory.json')],
        'gleam-bundle-fixtures':['-B',str(HERE/'revisions/private-v2/fixture_runner.py'),'--receipt',str(FAILED_SEAL/'gleam-bundle-inventory.json')],
    }
    for basename,tail in commands.items():
        value=json.loads((FAILED_SEAL/(basename+'.json')).read_text())
        assert value['command'][1:]==tail and Path(value['command'][0]).is_file()
        assert type(value['exit_status']) is type(value['final_exit_status']) is int and value['exit_status']==value['final_exit_status']==0
        assert value['inherited_common_mutex'] is True
        assert not any(value.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error','remaining_group_members'))
        assert (FAILED_SEAL/(basename+'.stdout')).is_file() and (FAILED_SEAL/(basename+'.stderr')).is_file()
    pins=service.merge(service.pin_tree(FAILED_SEAL),{str(FAILED_SEAL_TOOL_OUTPUT):preparation.sha(FAILED_SEAL_TOOL_OUTPUT)})
    guard.verify(pins)
    return dict(status='preserved-actual-private-v2-metadata-seal-failure',session_id=2438,
        original_tool_output=str(FAILED_SEAL_TOOL_OUTPUT),original_tool_output_sha256=pins[str(FAILED_SEAL_TOOL_OUTPUT)],
        original_rollout_timestamp=archive['original_rollout_record']['timestamp'],relevant_original_chunk=relevant,
        passed_fixture_methods=19,failed_stage='historical-fingerprint-reconstruction',
        native_benchmark_processes=0,proposal_created=False,
        recorded_in_completed_preparation_pins=False,newly_captured_original_tool_output=True,pins=pins)


def validate_records(report):
    assert report['status']=='stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256']==PRIOR_SHA and report['full_performance_runs']==0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    assert report['error_type']=='AssertionError' and report['error']=="('Owned command failed', 'compile', 1)"
    assert len(report['profiles'])==1 and len(report['attempts'])==4
    row=report['profiles'][0]
    assert row['id']=='gleam-binarytrees-compact' and row['benchmark']=='binarytrees' and row['variant']=='compact'
    fingerprint=historical_fingerprint(row)
    cases=['n-0','n-4','n-7','n-9']
    for number,(attempt,case) in enumerate(zip(report['attempts'],cases),1):
        assert attempt['id']==row['id'] and attempt['attempt']==number and attempt['case']==case
        assert attempt['benchmark']=='binarytrees' and attempt['key']=='binarytrees:'+row['id']
        assert attempt['language']=='gleam' and attempt['workload_n']==int(case[2:])
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        assert json.loads(attempt['execution_fingerprint'])==fingerprint
        assert attempt['source_sha256']==row['source_sha256'] and attempt['cwd']==row['cwd']
        assert attempt['runtime_environment']==row['runtime_environment']
        assert attempt['command']==[str(part).replace('{n}',case[2:]) for part in row['command']]
        checks=attempt['qualification_checks']
        assert type(checks) is dict and set(checks)==GATES and all(type(value) is bool and value is True for value in checks.values())
        assert attempt['qualified'] is True and attempt['sampler_wait_status']==0
        observations(attempt)
        native=attempt['supervised_native_group']
        assert native['type']=='native-group-started' and type(native['pid']) is int and native['pid']>0 and native['pgid']==native['pid']
        assert native['command']==attempt['command'] and native['inherited_common_mutex'] is True
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes']==attempt['stdout_bytes'] and attempt['validation']['sha256']==attempt['stdout_sha256']
    return copy.deepcopy(report['attempts'])


def archive_correspondence(preparation,payload,directory):
    pins={}
    with tarfile.open(fileobj=io.BytesIO(payload),mode='r:*') as archive:
        for member in archive:
            target=directory/member.name
            assert target.resolve().is_relative_to(directory.resolve()) and (member.isdir() or member.isfile())
            if member.isfile():
                import hashlib
                digest=hashlib.file_digest(archive.extractfile(member),'sha256').hexdigest()
                assert preparation.sha(target)==digest
                pins[str(target)]=digest
    assert pins
    return pins


def reconstruct(preparation,service,guard):
    validate_repairs(preparation)
    assert preparation.sha(PRIOR_PROPOSAL)==PRIOR_SHA
    proposal=json.loads(PRIOR_PROPOSAL.read_text());guard.verify(proposal['pins'])
    failed_seal=preserve_failed_seal(preparation,service,guard)
    report_path=PRIOR/'report.json';report=json.loads(report_path.read_text());records=validate_records(report)
    row=report['profiles'][0]
    # Independently bind the reconstruction to the unchanged pinned producer.
    harness=guard.load_harness()
    assert json.loads(harness.execution_fingerprint(row))==historical_fingerprint(row)
    assert row['compiler_version']=='1.19.0' and row['runtime_version']=='29.1.1'
    assert row['measured_architecture']=='arm64' and row['backend']=='BEAM load-time JIT'
    compiler_candidates=[p for p in (BUILD/'compiler').rglob('gleam') if p.is_file()]
    assert len(compiler_candidates)==1
    compiler=compiler_candidates[0];libraries=BUILD/'libraries';application=BUILD/'applications'/row['id']
    pins=service.merge(proposal['pins'],failed_seal['pins'],{str(PRIOR_PROPOSAL):PRIOR_SHA,str(report_path):preparation.sha(report_path)},
        service.pin_tree(PRIOR),service.pin_tree(BUILD/'compiler'),service.pin_tree(BUILD/'stdlib'),
        service.pin_tree(libraries),service.pin_tree(application),service.pin_tree(BUILD/'packages'/row['id']))
    gleam_archive,stdlib_archive=BUILD/'gleam-arm64.tar.gz',BUILD/'gleam_stdlib.tar'
    assert preparation.sha(gleam_archive)==proposal['compiler_archive_sha256']==preparation.GLEAM_SHA
    assert preparation.sha(stdlib_archive)==proposal['stdlib_archive_sha256']
    pins=service.merge(pins,{str(gleam_archive):preparation.sha(gleam_archive),str(stdlib_archive):preparation.sha(stdlib_archive)},
        archive_correspondence(preparation,gleam_archive.read_bytes(),BUILD/'compiler'),
        archive_correspondence(preparation,preparation.hex_contents(stdlib_archive.read_bytes()),BUILD/'stdlib'))
    version=json.loads((PRIOR/'bootstrap/gleam-version.json').read_text())
    assert version['command']==[str(compiler),'--version']
    assert version['exit_status']==version['final_exit_status']==0 and version['inherited_common_mutex'] is True
    assert not any(version.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error'))
    assert (PRIOR/'bootstrap/gleam-version.stdout').read_text().strip()=='gleam 1.19.0'
    metadata=json.loads((PRIOR/'bootstrap/otp-version.stdout').read_text())
    assert metadata['otp']=='29' and metadata['erts'].startswith('17.1') and metadata['jit']=='jit' and metadata['schedulers']==8
    assert 'aarch64' in metadata['arch']
    otp=json.loads((PRIOR/'bootstrap/otp-version.json').read_text())
    assert otp['exit_status']==otp['final_exit_status']==0 and otp['inherited_common_mutex'] is True
    assert not any(otp.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error'))
    for name,image in (('gleam',compiler),('beam',preparation.ENGINE),('erlexec',preparation.ERLEXEC)):
        directory=PRIOR/'bootstrap'/name
        assert (directory/'architecture.stdout').read_text().split()==['arm64']
        assert 'ARM64' in (directory/'mach-header.stdout').read_text()
        commands={'architecture':['/usr/bin/lipo','-archs',str(image)],
            'mach-header':['/usr/bin/otool','-hv',str(image)],'linked-images':['/usr/bin/otool','-L',str(image)]}
        for basename,command in commands.items():
            value=json.loads((directory/(basename+'.json')).read_text())
            assert value['command']==command
            assert value['exit_status']==value['final_exit_status']==0 and value['inherited_common_mutex'] is True
            assert not any(value.get(k) for k in ('error','cleanup_error','cleanup_errors','output_drain_error'))
        review=json.loads((directory/'dependency-review.json').read_text())
        assert set(review)=={'binary','architectures','dependencies'} and review['binary']==str(image) and review['architectures']==['arm64']
        linked=[line.strip().split(' (',1)[0] for line in (directory/'linked-images.stdout').read_text().splitlines()[1:]]
        assert [dep['install_name'] for dep in review['dependencies']]==linked
        for dependency in review['dependencies']:
            install=dependency['install_name']
            if install.startswith(('/usr/lib/','/System/Library/')):
                assert dependency==dict(install_name=install,scope='OS-image-not-hashed')
            else:
                assert set(dependency)=={'install_name','lookup_path','path','sha256'}
                lookup=Path(dependency['lookup_path']);target=lookup.resolve(strict=True)
                assert target==Path(dependency['path']) and target.is_relative_to(Path('/opt/homebrew/Cellar'))
                assert preparation.sha(lookup)==preparation.sha(target)==dependency['sha256']
                pins=service.merge(pins,{str(lookup):dependency['sha256'],str(target):dependency['sha256']})
    declaration=next(r for r in proposal['profiles'] if r['id']==row['id'])
    assert row['compiled_source_bundle']==declaration['compiled_source_bundle']
    inventory_path=Path(row['compiled_input_inventory'])
    assert preparation.sha(inventory_path)==row['compiled_input_inventory_sha256']
    inventory=json.loads(inventory_path.read_text())
    assert inventory['status']=='compiled-exact-authored-input-inventory' and inventory['profile']==row['id']
    assert inventory['source_bundle']==row['source_bundle'] and inventory['compiled_artifacts']==row['compiled_artifacts']
    assert inventory['compiler']==str(compiler) and inventory['compiler_sha256']==preparation.sha(compiler)
    assert inventory['runtime_classpath']==row['runtime_classpath']==[str(application/'ebin'),str(libraries/'gleam_stdlib/ebin')]
    expected_command=preparation.compile_command(compiler,BUILD/'packages'/row['id'],application,libraries)
    assert inventory['build_command']==expected_command
    compile_receipt=json.loads((PRIOR/'builds'/row['id']/'compile.json').read_text())
    assert compile_receipt['command']==expected_command
    assert compile_receipt['exit_status']==compile_receipt['final_exit_status']==0 and compile_receipt['inherited_common_mutex'] is True
    assert not any(compile_receipt.get(k) for k in ('error','cleanup_error','cleanup_errors','output_drain_error'))
    assert row['command']==preparation.runtime_command(application,libraries)
    assert preparation.sha(row['command'][0])==row['binary_sha256']
    assert len(row['source_bundle'])==len(declaration['compiled_source_bundle'])
    for item,original in zip(row['source_bundle'],declaration['compiled_source_bundle']):
        assert preparation.sha(item['path'])==item['sha256']==preparation.sha(HERE/original)
        assert item['origin']==str(PRIOR/'source-snapshot'/original)
        assert preparation.sha(item['origin'])==item['sha256']
    guard.verify(row['support']);guard.verify(row['compiled_artifacts'])
    import references
    prior_records=[]
    for attempt in records:
        directory=PRIOR/row['id']/attempt['case'];raw=Path(attempt['result_path']);supervisor=Path(attempt['supervisor_result_path'])
        assert raw==PRIOR/'attempts'/f"{attempt['attempt']:03d}-{row['id']}.json" and supervisor==raw.with_suffix('.supervisor.json')
        actual=(directory/'actual.stdout').read_bytes()
        assert actual==(directory/'expected.stdout').read_bytes()==references.binarytrees(attempt['workload_n'])
        assert preparation.sha(directory/'actual.stdout')==attempt['stdout_sha256']
        sampled,enriched=json.loads(raw.read_text()),json.loads(supervisor.read_text())
        observations(sampled);observations(enriched)
        for key in OBSERVATIONS+('command','source_sha256','execution_fingerprint','runtime_environment','stdout_bytes','stdout_sha256'):
            assert sampled[key]==enriched[key]==attempt[key]
        for key in ('qualified','qualification_checks','supervised_native_group','sampler_wait_status'):
            assert enriched[key]==attempt[key]
        prior_records.append(dict(case=attempt['case'],prior_attempt=attempt['attempt'],raw_result=str(raw),supervisor_result=str(supervisor),
            stdout_sha256=attempt['stdout_sha256'],origin='retained-private-v1'))
    failed=PRIOR/'builds/gleam-binarytrees-optimized'
    receipt=json.loads((failed/'compile.json').read_text())
    assert receipt['exit_status']==receipt['final_exit_status']==1 and receipt['inherited_common_mutex'] is True
    assert receipt['error_type']=='AssertionError' and receipt['error']=="('Owned command failed', 'compile', 1)"
    assert not any(receipt.get(k) for k in ('cleanup_error','cleanup_errors','output_drain_error'))
    assert 'To group expressions in Gleam' in (failed/'compile.stderr').read_text() and '(index + 1)' in (failed/'compile.stderr').read_text()
    guard.verify(pins)
    return dict(status='verified-exact-old-Gleam-app-four-cases',profile=copy.deepcopy(row),prior_records=prior_records,
        reused_bounded_cases=4,old_cases_relabelled_as_new_attempts=False,native_cases_reexecuted=0,
        prior_report=str(report_path),prior_report_sha256=pins[str(report_path)],prior_proposal_sha256=PRIOR_SHA,
        failed_compiler_receipt=str(failed/'compile.json'),failed_compiler_stderr=str(failed/'compile.stderr'),
        toolchain=dict(compiler=str(compiler),libraries=str(libraries)),actual_toolchain=metadata,pins=pins,
        prior_failed_metadata_seal=failed_seal,
        full_performance_runs=0,never_chart_eligible=True)


def validate_packet(packet,preparation,service,guard):
    guard.verify(packet['pins'])
    assert packet==reconstruct(preparation,service,guard)
    return packet


def mock_report():
    row=dict(id='gleam-binarytrees-compact',benchmark='binarytrees',variant='compact',source_sha256='source',
        source='/old/src/benchmark.gleam',binary_sha256='launcher',command=['/old/erl','-extra','{n}'],
        cwd='/old',runtime_environment={},
        source_bundle=[dict(path='/old/src/benchmark.gleam',sha256='source',role='application'),
            dict(path='/old/src/clbg_io.gleam',sha256='helper',role='application-helper')],
        support={'/old/src/clbg_io.gleam':'helper','/old/ebin/benchmark.beam':'compiled-application',
            '/old/runtime/beam.smp':'runtime-image'})
    report=dict(status='stopped-unqualified',active=None,proposal_sha256=PRIOR_SHA,full_performance_runs=0,
        chart_admission=False,no_automatic_retry=True,error_type='AssertionError',error="('Owned command failed', 'compile', 1)",profiles=[row],attempts=[])
    fingerprint=json.dumps(historical_fingerprint(row))
    for number,n in enumerate((0,4,7,9),1):
        command=['/old/erl','-extra',str(n)]
        report['attempts'].append(dict(id=row['id'],key='binarytrees:'+row['id'],attempt=number,case='n-'+str(n),
            benchmark='binarytrees',language='gleam',workload_n=n,full_workload=False,full_performance_run=False,
            preparation_only=True,never_chart_eligible=True,execution_fingerprint=fingerprint,source_sha256='source',cwd='/old',
            runtime_environment={},command=command,qualification_checks={key:True for key in GATES},qualified=True,sampler_wait_status=0,
            exit_status=0,seconds=0.25,power_at_start=dict(source='AC Power',low_power_mode=0),power_at_end=dict(source='AC Power',low_power_mode=0),
            competing_compilers=[],power_changes=[],timeout_seconds=120,maximum_rss_mib=10240,peak_process_group_rss_kib=0,
            valid_environment=True,pins_unchanged_at_end=True,supervised_native_group=dict(type='native-group-started',pid=1234,pgid=1234,command=command,inherited_common_mutex=True),
            stdout_bytes=7,stdout_sha256='output',validation=dict(reference_match=True,bytes=7,sha256='output')))
    return report
