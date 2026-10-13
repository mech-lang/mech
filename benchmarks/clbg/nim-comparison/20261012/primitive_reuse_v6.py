"""Explicitly retain three passed v5 primitives and its failed ORC build bytes."""
import copy
import json
from pathlib import Path

import proof_reuse_v5 as production
import source_proofs_v5 as original
import source_proofs_v6 as corrected

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-v5'
BUILD = HERE / '.build/private-v5'
PRIOR_PROPOSAL = HERE / 'proposals/private-v5.json'
PRIOR_SHA = '90cb7f87e43ae7f54d68501eab1daebdede78d856ce5564a8b9077a0c5ac57aa'
IDS = ['pure-bigint-abi', 'gmp-abi', 'pcre-abi']
CONTRACTS = {
    'pure-bigint-abi': (original.BIGINT_ABI, b'pure-bigint-contract-ok\n'),
    'gmp-abi': (original.GMP_ABI, b'gmp-lp64-alias-sign-contract-ok\n'),
    'pcre-abi': (original.PCRE_ABI, b'pcre-8.45-dynamic-layout-regex-contract-ok\n'),
}


def diagnostic_correction():
    assert corrected.ORC_OWNERSHIP == original.ORC_OWNERSHIP.replace('distinct', 'threadIds')
    assert original.ORC_OWNERSHIP.count('distinct') == 3
    assert corrected.BIGINT_ABI == original.BIGINT_ABI
    assert corrected.GMP_ABI == original.GMP_ABI and corrected.PCRE_ABI == original.PCRE_ABI


def validate_records(report):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256'] == PRIOR_SHA
    assert report['error_type'] == 'AssertionError' and report['error'] == "('Owned command failed', 'build', 1)"
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert report['never_chart_eligible'] is True and report['no_automatic_retry'] is True
    assert report['reused_bounded_cases'] == 154 and report['reused_cases_relabelled_as_attempts'] is False
    assert len(report['attempts']) == 3
    for number, (identifier, attempt) in enumerate(zip(IDS, report['attempts']), 1):
        assert attempt['id'] == identifier and attempt['attempt'] == number and attempt['case'] == 'contract'
        assert attempt['benchmark'] == 'pidigits' and attempt['key'] == 'pidigits:' + identifier
        assert attempt['language'] == 'nim' and attempt['workload_n'] == 0
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        production.old.validate_observations(attempt)
        checks = attempt['qualification_checks']
        assert type(checks) is dict and set(checks) == production.old.EXPECTED_GATES
        assert all(type(value) is bool and value is True for value in checks.values())
        assert attempt['qualified'] is True and attempt['sampler_wait_status'] == 0
        fingerprint = json.loads(attempt['execution_fingerprint'])
        assert set(fingerprint) == {'source_sha256','binary_sha256','command','cwd','environment','support'}
        assert fingerprint['command'] == [str(BUILD / 'bin' / identifier), '{n}'] and fingerprint['support'] == {}
        assert fingerprint['source_sha256'] == attempt['source_sha256']
        assert fingerprint['cwd'] == attempt['cwd'] == str(HERE)
        assert fingerprint['environment'] == attempt['runtime_environment']
        assert attempt['command'] == [fingerprint['command'][0], '0']
        native = attempt['supervised_native_group']
        assert native['type'] == 'native-group-started' and native['command'] == attempt['command']
        assert native['inherited_common_mutex'] is True
        assert type(native['pid']) is int and native['pid'] > 0 and native['pid'] == native['pgid']
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes']
        assert attempt['validation']['sha256'] == attempt['stdout_sha256']
    return copy.deepcopy(report['attempts'])


def toolchain(preparation):
    compiler = BUILD / 'nim/nim-2.2.12/bin/nim'
    return dict(compiler=str(compiler),toolchain=str(compiler.parent.parent),
        bigint=str(BUILD / 'bigints/bigints-3b5a106b88ec1baa904a059a1430c8335d885075'),
        gmp_wrapper=str(BUILD / 'nim-gmp/nim-gmp-4e9a6a05bfc274abd02a241f2d7c9162b2300191'))


def compatibility_records(preparation):
    complete = json.loads((PRIOR / 'bootstrap/compatibility-complete.json').read_text())
    plan = json.loads((PRIOR / 'bootstrap/compatibility-plan.json').read_text())
    assert complete['status'] == 'exact-two-union-pragma-moves' and plan['status'] == 'planned-two-union-pragma-moves'
    assert complete['files'] == plan['files'] and len(complete['files']) == 2
    assert complete['public_package_commit'] == '4e9a6a05bfc274abd02a241f2d7c9162b2300191'
    assert complete['production_source_changed'] is False and complete['general_library_compatibility_only'] is True
    records = {}
    for record in complete['files']:
        target = Path(record['path']).resolve()
        assert target.is_relative_to(BUILD / 'nim-gmp')
        name = next(name for name in production.compatibility.UNIONS if str(target).endswith('/' + name))
        assert (record['old_declaration'],record['new_declaration']) == production.compatibility.UNIONS[name]
        assert record['arithmetic_changed'] is False and record['ffi_symbols_types_argument_order_changed'] is False
        retained = Path(record['original_copy'])
        assert retained.is_relative_to(PRIOR / 'bootstrap') and preparation.sha(retained) == record['original_sha256']
        assert preparation.sha(target) == record['changed_sha256'] and str(target) not in records
        records[str(target)] = record
    return records


def reconstruct(preparation, guard, harness):
    diagnostic_correction()
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    proposal = json.loads(PRIOR_PROPOSAL.read_text())
    guard.verify(proposal['pins'])
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    records = validate_records(report)
    bindings = toolchain(preparation)
    compiler, library = Path(bindings['compiler']), Path(bindings['toolchain'])
    bigint, wrapper = Path(bindings['bigint']), Path(bindings['gmp_wrapper'])
    sdk = Path(proposal['sdk'])
    pins = preparation.merge(proposal['pins'], {str(PRIOR_PROPOSAL):PRIOR_SHA,str(report_path):preparation.sha(report_path)},
        preparation.pin_tree(PRIOR / 'bootstrap'),preparation.pin_tree(PRIOR / 'source-snapshot'),
        preparation.pin_tree(PRIOR / 'attempts'),preparation.pin_tree(PRIOR / 'source-proofs'))
    for filename, directory in (('nim-2.2.12-macosx_arm64.tar.xz','nim'),('bigints-source.tar.gz','bigints'),('nim-gmp-source.tar.gz','nim-gmp')):
        archive = BUILD / filename
        if directory == 'nim': assert preparation.sha(archive) == preparation.NIM_SHA
        pins = preparation.merge(pins,{str(archive):preparation.sha(archive)},
            production.old.archive_correspondence(preparation,archive,BUILD / directory,
                compatibility_records(preparation) if directory=='nim-gmp' else None))
    version = json.loads((PRIOR / 'bootstrap/nim-version.json').read_text())
    assert version['command'] == [str(compiler),'--version']
    assert version['exit_status'] == version['final_exit_status'] == 0 and version['inherited_common_mutex'] is True
    assert not any(version.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error'))
    assert (PRIOR / 'bootstrap/nim-version.stdout').read_text().startswith('Nim Compiler Version 2.2.12')
    proofs = []
    for attempt in records:
        identifier = attempt['id']
        source = PRIOR / 'source-snapshot' / (identifier.replace('-','_') + '.nim')
        text, expected = CONTRACTS[identifier]
        assert source.read_text() == text and preparation.sha(source) == attempt['source_sha256']
        fingerprint = json.loads(attempt['execution_fingerprint'])
        binary = Path(fingerprint['command'][0])
        assert preparation.sha(binary) == fingerprint['binary_sha256']
        receipt = json.loads((PRIOR / 'source-proofs' / identifier / 'build.json').read_text())
        assert receipt['command'] == preparation.build_command(compiler,library,bigint,wrapper,sdk,source,binary,BUILD / 'cache' / identifier)
        assert receipt['exit_status'] == receipt['final_exit_status'] == 0 and receipt['inherited_common_mutex'] is True
        assert not any(receipt.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error'))
        directory = PRIOR / 'source-proofs' / identifier / 'contract'
        actual = (directory / 'actual.stdout').read_bytes()
        assert actual == (directory / 'expected.stdout').read_bytes() == expected
        assert preparation.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
        raw, supervisor = Path(attempt['result_path']), Path(attempt['supervisor_result_path'])
        assert raw == PRIOR / 'attempts' / f"{attempt['attempt']:03d}-{identifier}.json"
        assert supervisor == raw.with_suffix('.supervisor.json')
        sampled, enriched = json.loads(raw.read_text()),json.loads(supervisor.read_text())
        production.old.validate_observations(sampled);production.old.validate_observations(enriched)
        for key in production.old.OBSERVATION_FIELDS + ('command','source_sha256','runtime_environment','execution_fingerprint','stdout_sha256','stdout_bytes'):
            assert sampled[key] == enriched[key] == attempt[key]
        for key in ('qualified','qualification_checks','supervised_native_group','sampler_wait_status'):
            assert enriched[key] == attempt[key]
        pins = preparation.merge(pins,{str(binary):preparation.sha(binary)},preparation.pin_tree(BUILD / 'cache' / identifier))
        proofs.append(dict(proof=identifier,status='passed',result=str(supervisor),asan=False,
            application_full_workload=False,origin='retained-private-v5',prior_attempt=attempt['attempt'],
            prior_report=str(report_path),prior_report_sha256=pins[str(report_path)],
            raw_result=str(raw),source=str(source),source_sha256=attempt['source_sha256'],
            binary=str(binary),binary_sha256=fingerprint['binary_sha256'],execution_fingerprint=attempt['execution_fingerprint']))
    failed = PRIOR / 'source-proofs/orc-join-parent-destruction'
    receipt = json.loads((failed / 'build.json').read_text())
    assert receipt['exit_status'] == receipt['final_exit_status'] == 1 and receipt['inherited_common_mutex'] is True
    assert receipt['error_type'] == 'AssertionError' and receipt['error'] == "('Owned command failed', 'build', 1)"
    assert not any(receipt.get(key) for key in ('cleanup_error','cleanup_errors','output_drain_error'))
    failed_source = PRIOR / 'source-snapshot/orc_join_parent_destruction.nim'
    assert failed_source.read_text() == original.ORC_OWNERSHIP
    assert "identifier expected, but got 'keyword distinct'" in (failed / 'build.stderr').read_text()
    expected_command = preparation.build_command(compiler,library,bigint,wrapper,sdk,failed_source,
        BUILD / 'bin/orc-join-parent-destruction',BUILD / 'cache/orc-join-parent-destruction')
    expected_command[-1:-1] = ['--passC:-fsanitize=address','--passL:-fsanitize=address','--passC:-fno-omit-frame-pointer']
    assert receipt['command'] == expected_command
    guard.verify(pins)
    return dict(status='verified-three-exact-passed-v5-primitives',prior_report=str(report_path),
        prior_report_sha256=pins[str(report_path)],prior_proposal=str(PRIOR_PROPOSAL),prior_proposal_sha256=PRIOR_SHA,
        proofs=proofs,reused_primitive_cases=3,old_cases_relabelled_as_new_attempts=False,native_cases_reexecuted=0,
        failed_diagnostic=dict(build_receipt=str(failed / 'build.json'),stderr=str(failed / 'build.stderr'),source=str(failed_source)),
        toolchain=bindings,pins=pins,full_performance_runs=0,never_chart_eligible=True)


def validate_packet(packet, preparation, guard, harness=None):
    guard.verify(packet['pins'])
    expected = reconstruct(preparation,guard,harness)
    assert packet == expected, 'Reuse identities/receipts/proofs must reconstruct exactly; no fabricated prior success'
    return packet
