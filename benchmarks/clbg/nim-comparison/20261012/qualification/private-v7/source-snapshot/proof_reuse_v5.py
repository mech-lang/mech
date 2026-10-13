"""Reuse 154 exact production cases; retain the failed signed primitive separately."""
import copy
import json
from pathlib import Path

import proof_reuse_v4 as old
import library_compat_v4 as compatibility

HERE = Path(__file__).resolve().parent
PRIOR = HERE / 'qualification/private-v4'
PRIOR_PROPOSAL = HERE / 'proposals/private-v4.json'
PRIOR_SHA = '70a6d7d2f6237307e9c7af0d649cc003ccd15b8d94c9bc56502b2ad32e5428e0'
NEW_PROFILES = {f'nim-{benchmark}-{variant}' for benchmark in
                ('knucleotide', 'binarytrees', 'nbody', 'fannkuchredux') for variant in ('compact', 'optimized')}
PROFILES = old.PROFILES | NEW_PROFILES
PENDING_REUSED_CASES = {}
profile_row = old.profile_row


def validate_records(report, preparation):
    assert report['status'] == 'stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256'] == PRIOR_SHA
    assert report['error_type'] == 'AssertionError' and 'pure-bigint-abi' in report['error']
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert report['reused_bounded_cases'] == 106 and report['reused_cases_relabelled_as_attempts'] is False
    assert len(report['attempts']) == 49, 'Only 48 production successes plus the retained primitive failure exist'
    failed = report['attempts'][-1]
    assert failed['id'] == 'pure-bigint-abi' and failed['case'] == 'contract' and failed['attempt'] == 49
    assert failed['qualified'] is False and failed['exit_status'] == 1
    assert 'initBigInt(-17) div b == initBigInt(-3)' in failed['stderr']
    groups = {}
    inventory = preparation.preflight_inputs(__import__('references'))
    for number, attempt in enumerate(report['attempts'][:-1], 1):
        assert attempt['attempt'] == number
        old.validate_observations(attempt)
        identifier = attempt['id']
        assert identifier in NEW_PROFILES
        benchmark = attempt['benchmark']
        assert identifier.startswith('nim-' + benchmark + '-')
        expected_cases = set(inventory[benchmark])
        if identifier == 'nim-knucleotide-compact':
            expected_cases = set(old.PENDING_REUSED_CASES[identifier])
        assert attempt['case'] in expected_cases
        if benchmark in preparation.NUMERIC:
            assert attempt['workload_n'] == int(attempt['case'][2:])
        assert set(attempt['qualification_checks']) == old.EXPECTED_GATES
        assert all(value is True for value in attempt['qualification_checks'].values())
        assert attempt['qualified'] is True and attempt['sampler_wait_status'] == 0
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        assert attempt['validation']['reference_match'] is True
        assert attempt['validation']['bytes'] == attempt['stdout_bytes']
        assert attempt['validation']['sha256'] == attempt['stdout_sha256']
        fingerprint = json.loads(attempt['execution_fingerprint'])
        assert fingerprint['command'][1:] == ['{n}']
        assert attempt['command'] == [fingerprint['command'][0], str(attempt['workload_n'])]
        native = attempt['supervised_native_group']
        assert native['command'] == attempt['command'] and native['inherited_common_mutex'] is True
        assert type(native['pid']) is int and native['pid'] > 0 and native['pid'] == native['pgid']
        assert fingerprint['source_sha256'] == attempt['source_sha256']
        assert fingerprint['environment'] == attempt['runtime_environment'] and fingerprint['cwd'] == attempt['cwd']
        group = groups.setdefault(identifier, dict(benchmark=benchmark, cases={}, fingerprint=fingerprint))
        assert group['fingerprint'] == fingerprint and attempt['case'] not in group['cases']
        group['cases'][attempt['case']] = attempt
    assert set(groups) == NEW_PROFILES
    for identifier, group in groups.items():
        expected = set(old.PENDING_REUSED_CASES[identifier]) if identifier == 'nim-knucleotide-compact' else set(inventory[group['benchmark']])
        assert set(group['cases']) == expected
    return groups


def compatibility_records(preparation):
    complete = json.loads((PRIOR / 'bootstrap/compatibility-complete.json').read_text())
    plan = json.loads((PRIOR / 'bootstrap/compatibility-plan.json').read_text())
    assert complete['status'] == 'exact-two-union-pragma-moves' and plan['status'] == 'planned-two-union-pragma-moves'
    assert complete['files'] == plan['files'] and len(complete['files']) == 2
    assert complete['production_source_changed'] is False and complete['general_library_compatibility_only'] is True
    assert complete['public_package_commit'] == '4e9a6a05bfc274abd02a241f2d7c9162b2300191'
    records = {}
    for record in complete['files']:
        target = Path(record['path']).resolve()
        assert target.is_relative_to(HERE / '.build/private-v4/nim-gmp')
        filename = next(name for name in compatibility.UNIONS if str(target).endswith('/' + name))
        assert (record['old_declaration'], record['new_declaration']) == compatibility.UNIONS[filename]
        assert record['arithmetic_changed'] is False and record['ffi_symbols_types_argument_order_changed'] is False
        retained = Path(record['original_copy'])
        assert retained.is_relative_to(PRIOR / 'bootstrap') and preparation.sha(retained) == record['original_sha256']
        assert preparation.sha(target) == record['changed_sha256'] and str(target) not in records
        records[str(target)] = record
    return records


def reconstruct(preparation, guard, harness, output, lock_fd):
    assert preparation.sha(PRIOR_PROPOSAL) == PRIOR_SHA
    proposal = json.loads(PRIOR_PROPOSAL.read_text())
    guard.verify(proposal['pins'])
    prior_reuse_path = Path(proposal['prior_proof_reuse'])
    assert preparation.sha(prior_reuse_path) == proposal['prior_proof_reuse_sha256']
    prior = json.loads(prior_reuse_path.read_text())
    old.validate_prior_reports(prior)
    guard.verify(prior['pins'])
    assert set(prior['profiles']) == old.PROFILES and sum(len(p['prior_records']) for p in prior['profiles'].values()) == 106
    profiles = copy.deepcopy(prior['profiles'])
    report_path = PRIOR / 'report.json'
    report = json.loads(report_path.read_text())
    groups = validate_records(report, preparation)
    old_build = HERE / '.build/private-v4'
    pins = preparation.merge(proposal['pins'], prior['pins'], {str(PRIOR_PROPOSAL): PRIOR_SHA,
        str(report_path): preparation.sha(report_path)}, preparation.pin_tree(PRIOR / 'source-snapshot'),
        preparation.pin_tree(PRIOR / 'builds'), preparation.pin_tree(PRIOR / 'bootstrap'))
    for name, directory in (('nim-2.2.12-macosx_arm64.tar.xz', 'nim'),
                            ('bigints-source.tar.gz', 'bigints'), ('nim-gmp-source.tar.gz', 'nim-gmp')):
        archive = old_build / name
        if directory == 'nim':
            assert preparation.sha(archive) == preparation.NIM_SHA
        pins[str(archive)] = preparation.sha(archive)
        pins = preparation.merge(pins, old.archive_correspondence(preparation, archive, old_build / directory,
            compatibility_records(preparation) if directory == 'nim-gmp' else None))
    compiler = old_build / 'nim/nim-2.2.12/bin/nim'
    toolchain = compiler.parent.parent
    bigint = old_build / 'bigints/bigints-3b5a106b88ec1baa904a059a1430c8335d885075'
    gmp_wrapper = old_build / 'nim-gmp/nim-gmp-4e9a6a05bfc274abd02a241f2d7c9162b2300191'
    version = json.loads((PRIOR / 'bootstrap/nim-version.json').read_text())
    assert version['command'] == [str(compiler), '--version']
    assert version['exit_status'] == version['final_exit_status'] == 0 and version['inherited_common_mutex'] is True
    assert not version.get('error')
    assert (PRIOR / 'bootstrap/nim-version.stdout').read_text().startswith('Nim Compiler Version 2.2.12')
    roots = [old_build / 'nim', preparation.GMP, preparation.PCRE, preparation.CLANG.parent.parent]
    pins = preparation.merge(pins, preparation.native_images(guard, harness, compiler,
        dict(LC_ALL='C', DYLD_LIBRARY_PATH=preparation.LIBRARY_PATH), output / 'compiler-images', lock_fd, roots))
    import references
    for declared in proposal['profiles']:
        identifier = declared['id']
        if identifier not in groups:
            continue
        group = groups[identifier]
        fingerprint = group['fingerprint']
        if identifier in profiles:
            reused = profiles[identifier]
            assert identifier == 'nim-knucleotide-compact'
            assert json.loads(reused['execution_fingerprint']) == fingerprint
            source, binary = Path(reused['source']), Path(reused['binary'])
        else:
            source = PRIOR / 'source-snapshot' / declared['source']
            binary = old_build / 'bin' / identifier
            receipt = json.loads((PRIOR / 'builds' / identifier / 'build.json').read_text())
            assert receipt['exit_status'] == receipt['final_exit_status'] == 0 and receipt['inherited_common_mutex'] is True
            assert not receipt.get('error')
            expected_command = preparation.build_command(compiler, toolchain, bigint, gmp_wrapper,
                Path(proposal['sdk']), source, binary, old_build / 'cache' / identifier)
            assert receipt['command'] == expected_command
            profiles[identifier] = dict(declaration=declared, source=str(source), binary=str(binary),
                source_sha256=fingerprint['source_sha256'], binary_sha256=fingerprint['binary_sha256'],
                execution_fingerprint=json.dumps(fingerprint, sort_keys=True), command=fingerprint['command'],
                cwd=fingerprint['cwd'], runtime_environment=fingerprint['environment'],
                original_build_command=receipt['command'], prior_records=[],
                qualification_origin='exact-unchanged-v4-binary-and-records', new_native_cases=0)
        assert fingerprint['command'][0] == str(binary)
        old.require_identity(fingerprint, preparation.sha(source), preparation.sha(binary))
        old.require_identity(fingerprint, preparation.sha(HERE / declared['source']), preparation.sha(binary))
        pins[str(binary)] = fingerprint['binary_sha256']
        if binary.is_relative_to(old_build):
            pins = preparation.merge(pins, preparation.pin_tree(old_build / 'cache' / identifier))
        pins = preparation.merge(pins, preparation.native_images(guard, harness, binary,
            fingerprint['environment'], output / identifier / 'images', lock_fd, roots))
        for case, attempt in sorted(group['cases'].items()):
            directory = PRIOR / identifier / case
            payload = None if declared['benchmark'] in preparation.NUMERIC else dict(preparation.inputs(declared['benchmark'], references))[case]
            expected = getattr(references, declared['benchmark'])(int(case[2:]) if payload is None else payload)
            actual = (directory / 'actual.stdout').read_bytes()
            assert (directory / 'expected.stdout').read_bytes() == expected
            if declared['benchmark'] == 'nbody':
                harness.compact.common.validate(actual.decode(), [float(v) for v in expected.splitlines()])
            else:
                assert actual == expected
            if payload is not None:
                assert (directory / 'stdin.fasta').read_bytes() == payload
            assert preparation.sha(directory / 'actual.stdout') == attempt['stdout_sha256']
            raw, supervisor = Path(attempt['result_path']), Path(attempt['supervisor_result_path'])
            assert raw.is_relative_to(PRIOR / 'attempts') and supervisor.is_relative_to(PRIOR / 'attempts')
            original, enriched = json.loads(raw.read_text()), json.loads(supervisor.read_text())
            old.validate_observations(original)
            old.validate_observations(enriched)
            assert all(original[key] == enriched[key] == attempt[key] for key in old.OBSERVATION_FIELDS)
            for key in ('command', 'source_sha256', 'runtime_environment', 'execution_fingerprint', 'stdout_sha256'):
                assert original[key] == enriched[key] == attempt[key]
            assert enriched['qualified'] is True and enriched['qualification_checks'] == attempt['qualification_checks']
            assert enriched['supervised_native_group'] == attempt['supervised_native_group']
            assert enriched['sampler_wait_status'] == attempt['sampler_wait_status'] == 0
            for path in list(directory.iterdir()) + [raw, supervisor]:
                if path.is_file():
                    pins[str(path)] = preparation.sha(path)
            profiles[identifier]['prior_records'].append(dict(case=case, prior_attempt=attempt['attempt'],
                raw_result=str(raw), supervisor_result=str(supervisor), stdout_sha256=attempt['stdout_sha256']))
    assert set(profiles) == PROFILES and sum(len(p['prior_records']) for p in profiles.values()) == 154
    for identifier, profile in profiles.items():
        assert json.loads(harness.execution_fingerprint(profile_row(profile))) == json.loads(profile['execution_fingerprint'])
        assert {r['case'] for r in profile['prior_records']} == set(preparation.preflight_inputs(references)[profile['declaration']['benchmark']])
    pins = preparation.merge(pins, preparation.pin_tree(output), preparation.pin_tree(PRIOR / 'attempts'),
        preparation.pin_tree(PRIOR / 'source-proofs'))
    guard.verify(pins)
    return dict(status='verified-exact-old-bounded-proof', profiles=profiles, pins=pins,
        prior_reports=prior['prior_reports'] + [str(report_path)], lineage=prior['lineage'] + [dict(
            prior_proposal=str(PRIOR_PROPOSAL), prior_proposal_sha256=PRIOR_SHA,
            prior_report=str(report_path), prior_report_sha256=pins[str(report_path)])],
        reused_bounded_cases=154, pending_reused_profile_cases={}, native_cases_reexecuted=0,
        old_cases_relabelled_as_new_attempts=False, failed_primitive_retained=str(Path(report['attempts'][-1]['result_path'])),
        full_performance_runs=0, never_chart_eligible=True)


def validate_prior_reports(reuse):
    assert len(reuse['prior_reports']) == 3
    old.validate_prior_reports(dict(prior_reports=reuse['prior_reports'][:2]))
    import guarded_prepare_v5 as preparation
    validate_records(json.loads(Path(reuse['prior_reports'][2]).read_text()), preparation)
