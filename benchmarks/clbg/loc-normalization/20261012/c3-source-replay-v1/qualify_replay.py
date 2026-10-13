"""Source-only candidate: replay qualified C3 grammar on exact compiler inputs.

No compiler, benchmark, fixture subprocess, signal or admission is issued.
All imports, hashes, source normalization and terminal evidence share one mutex.
"""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import re
import shutil
import sys
import traceback

HERE = Path(__file__).resolve().parent
LOC = HERE.parent
CLBG = LOC.parents[1]
C3 = CLBG / 'c3-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
QUALIFICATION = C3 / 'qualification/private-v3/report.json'
QUALIFICATION_SHA = '7cb4a25819935f3e15b3c270595efc9df3dbc056982625892abc3b16bef2f20c'
PROPOSAL = C3 / 'proposals/private-v3.json'
PROPOSAL_SHA = 'e82a50ede553c98c801c4ddf8d13ca8d0a0c1c06c85b26fec03a407a8988a564'
PROFILES = C3 / 'revisions/private-v3/profiles.json'
REUSE = C3 / 'proposal-seals/private-v3/verified-native-reuse.json'
REUSE_SHA = '2ef6acbe5a66157c26b7b9e37f8b377e477b596a5722aca9590da5ca5d6583ec'
FIXTURE_JOB = LOC / 'c3-proposal-v4/jobs/fixtures-v4'
FIXTURE = FIXTURE_JOB / 'fixture-receipt.json'
FIXTURE_SHA = 'a3e1197e6371c7e2b77c160763e44e1b4e62c397b1704dfd3f345052753647da'
RAW_FIXTURE_SHA = '56ad496dbd588b7e947e3d6ca3c1ab548d97b6be332775196d741ce56bfa71b7'
NEUTRAL_FIXTURE = LOC / 'jobs/boundary-audit-18/fixture-receipt.json'
NEUTRAL_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
SNAPSHOT_FILES = ('foreign_c3.py', 'c3_mask.py', 'foreign_core.py', 'canonical.py', 'test_foreign_c3.py', 'REVIEW.md')
PACKET_FILES = ('qualify_replay.py', 'REVIEW.md')
BENCHMARKS = ('binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
    'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')
FRESH = {'c3-' + name + '-' + form for name in ('nbody', 'fannkuchredux') for form in ('compact', 'optimized')} | {'c3-binarytrees-optimized'}
METHODS = (
    'test_module_import_and_separate_packed_statements', 'test_c3_function_formals_are_structural',
    'test_extern_abi_signature_and_attribute_are_one_header', 'test_single_external_variable_has_proved_typed_owner',
    'test_function_pointer_and_generic_aliases', 'test_struct_pointer_array_slice_fields_and_independent_declarators',
    'test_nested_struct_has_its_own_real_close', 'test_initialized_uninitialized_and_chained_bindings',
    'test_for_foreach_headers_do_not_become_body_assignments', 'test_inline_control_branch_and_body_count',
    'test_defer_call_and_defer_block_owners', 'test_pure_arrow_one_definition_and_mutating_arrow_is_gated',
    'test_named_call_arguments_and_data_assignments_are_structural',
    'test_initializer_and_index_owners_cannot_hide_executable_effects',
    'test_scalar_switch_arm_headers_preserve_each_real_body_statement',
    'test_nested_comments_literals_raw_ticks_and_exact_offsets',
    'test_contracts_and_macro_compile_time_forms_are_not_silent_comments',
    'test_unterminated_and_unbalanced_input_remains_gated',
    'test_optional_unwrapping_unknown_attribute_and_switch_are_gated',
)
EXPECTED_IDS = sorted('test_foreign_c3.C3Fixtures.' + name for name in METHODS)
FATAL_BUILD = ('error', 'cleanup_error', 'cleanup_errors', 'output_drain_error',
    'remaining_group_members', 'supervisor_remaining_members')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def merge(*groups):
    pins = {}
    for group in groups:
        for name, digest in group.items():
            assert name not in pins or pins[name] == digest, ('Conflicting pin', name)
            pins[name] = digest
    return pins


def verify(pins):
    for name, digest in pins.items():
        assert sha(name) == digest, ('Pinned evidence changed', name)


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def now():
    return datetime.now(timezone.utc).isoformat()


def qualified_grammar():
    assert sha(FIXTURE) == FIXTURE_SHA and sha(NEUTRAL_FIXTURE) == NEUTRAL_SHA
    receipt = json.loads(FIXTURE.read_text())
    assert receipt['status'] == 'passed-awaiting-source-review'
    assert type(receipt['tests_run']) is int and receipt['tests_run'] == 19
    for key in ('expected_ids', 'discovered_ids', 'started_ids'):
        assert sorted(receipt[key]) == EXPECTED_IDS and len(receipt[key]) == len(set(receipt[key])) == 19
    for key in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes',
            'benchmark_processes_started', 'subprocesses_started_by_fixtures'):
        assert type(receipt[key]) is int and receipt[key] == 0
    assert receipt['exact_test_execution'] is receipt['pin_reconciliation_passed'] is True
    assert receipt['exclusive_mutex_held'] is True and receipt['common_timing_mutex'] == str(LOCK)
    assert receipt['source_counting_started'] is receipt['chart_admitted'] is False
    assert receipt['pins_before'] == receipt['pins_after']
    assert receipt['compiler_version'] == '0.8.5' and receipt['compiler_source_commit'] == '640091d3303a393f9651d644063b837fc19d05f1'
    raw = FIXTURE_JOB / 'fixtures.log'
    assert receipt['raw_fixture_log'] == dict(path=str(raw), sha256=RAW_FIXTURE_SHA)
    assert sha(raw) == RAW_FIXTURE_SHA
    terminal_path = FIXTURE_JOB / 'fixture-receipt-pin.json'
    terminal = json.loads(terminal_path.read_text())
    assert terminal['common_mutex_held'] is True
    assert terminal['terminal_worker_status'] == terminal['saved_receipt_status'] == receipt['status']
    assert terminal['receipt'] == str(FIXTURE) and terminal['receipt_sha256'] == FIXTURE_SHA
    assert terminal['terminal_artifact_pins'][str(FIXTURE)] == FIXTURE_SHA
    assert terminal['terminal_artifact_pins'][str(raw)] == RAW_FIXTURE_SHA
    neutral = json.loads(NEUTRAL_FIXTURE.read_text())
    assert neutral['status'] == 'passed' and type(neutral['tests_run']) is int and neutral['tests_run'] == 672
    assert neutral['failures'] == neutral['errors'] == 0 and neutral['exclusive_mutex_held'] is True
    assert neutral['common_timing_mutex'] == str(LOCK)
    assert receipt['python_version'].split()[0] == neutral['python_version'].split()[0] == sys.version.split()[0]
    assert sys.version_info[:2] == (3, 14) and receipt['pygments_version'] == neutral['pygments_version'] == '2.19.2'
    pins = merge(receipt['pins_before'], neutral['frozen_dependency_pins'], terminal['terminal_artifact_pins'],
        {str(FIXTURE): FIXTURE_SHA, str(NEUTRAL_FIXTURE): NEUTRAL_SHA, str(terminal_path): sha(terminal_path)})
    verify(pins)
    return receipt, pins


def printed_inputs(stdout, authored, stdlib, pins):
    lines = stdout.splitlines()
    assert lines.count('# input-files-begin') == lines.count('# input-files-end') == 1
    first, last = lines.index('# input-files-begin'), lines.index('# input-files-end')
    assert first < last
    names = lines[first + 1:last]
    assert names and len(names) == len(set(names))
    assert names[:len(authored)] == authored
    result = []
    for index, name in enumerate(names):
        path = Path(name)
        assert path.is_absolute() and str(path) == name and path.suffix == '.c3'
        assert path.is_file() and name in pins and sha(path) == pins[name]
        if index < len(authored):
            role = 'application' if index == 0 else 'application-helper'
        else:
            assert path.resolve(strict=True).is_relative_to(stdlib.resolve(strict=True)), ('Unknown compiler input', name)
            role = 'vendor-standard-library'
        result.append(dict(path=name, sha256=pins[name], role=role, counted=role != 'vendor-standard-library'))
    assert len([item for item in result if item['counted']]) == len(authored)
    return result


def compiler_inventory():
    assert sha(QUALIFICATION) == QUALIFICATION_SHA and sha(PROPOSAL) == PROPOSAL_SHA and sha(REUSE) == REUSE_SHA
    report = json.loads(QUALIFICATION.read_text())
    proposal = json.loads(PROPOSAL.read_text())
    reuse = json.loads(REUSE.read_text())
    assert report['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review' and report['active'] is None
    assert report['proposal_sha256'] == PROPOSAL_SHA
    assert report['prior_native_proof_reuse'] == str(REUSE) and report['prior_native_proof_reuse_sha256'] == REUSE_SHA
    assert report['reused_bounded_cases'] == 126 and report['fresh_profile_builds'] == 5 and len(report['attempts']) == 28
    assert report['reused_cases_relabelled_as_attempts'] is False
    assert report['full_performance_runs'] == 0 and report['chart_admission'] is False
    assert reuse['status'] == 'verified-fifteen-exact-C3-v2-profiles-126-cases'
    assert reuse['reused_profile_count'] == 15 and reuse['reused_bounded_cases'] == 126
    assert reuse['native_cases_reexecuted'] == 0 and reuse['old_cases_relabelled_as_new_attempts'] is False
    declared = json.loads(PROFILES.read_text())
    expected_ids = {'c3-' + benchmark + '-' + form for benchmark in BENCHMARKS for form in ('compact', 'optimized')}
    rows = {row['id']: row for row in declared['profiles']}
    actual = {row['id']: row for row in report['profiles']}
    assert len(rows) == len(declared['profiles']) == len(actual) == len(report['profiles']) == 20
    assert set(rows) == set(actual) == expected_ids and set(reuse['profiles']) == expected_ids - FRESH
    assert {item['id'] for item in report['attempts']} == FRESH
    pins = merge(report['pins'], proposal['pins'], reuse['pins'],
        {str(QUALIFICATION): QUALIFICATION_SHA, str(PROPOSAL): PROPOSAL_SHA, str(REUSE): REUSE_SHA})
    verify(pins)
    assert sha(PROFILES) == pins[str(PROFILES)]
    compiler, stdlib = Path(report['compiler']), Path(report['stdlib'])
    assert str(compiler) == proposal['toolchain_binding']['compiler']
    assert str(stdlib) == proposal['toolchain_binding']['stdlib']
    assert proposal['toolchain_binding']['actual_toolchain']['compiler_sha256'] == pins[str(compiler)]
    evidence, authors, captured = {}, set(), set()
    for identifier, row in rows.items():
        native = actual[identifier]
        origin = 'private-v3' if identifier in FRESH else 'private-v2'
        if identifier not in FRESH:
            retained = reuse['profiles'][identifier]
            for key in ('source', 'source_sha256', 'source_bundle', 'compiled_source_bundle',
                    'build_command', 'command', 'binary_sha256', 'cache'):
                assert native[key] == retained[key]
            assert native['qualification_origin'] == 'retained-private-v2-exact-source-binary-and-records'
        assert native['compiled_source_bundle'] == row['compiled_source_bundle']
        assert row['source'] == row['compiled_source_bundle'][0] and row['loc'] is None
        copies = [str(C3 / 'qualification' / origin / 'source-snapshot' / relative) for relative in row['compiled_source_bundle']]
        assert [item['path'] for item in native['source_bundle']] == copies
        assert len(copies) == len(set(copies)) and native['source'] == copies[0]
        assert native['source_sha256'] == native['source_bundle'][0]['sha256']
        binary = C3 / '.build' / origin / 'bin' / identifier
        cache = C3 / '.build' / origin / 'cache' / identifier
        assert native['command'] == [str(binary), '{n}'] and native['cache'] == str(cache)
        assert sha(binary) == native['binary_sha256'] == pins[str(binary)]
        directory = C3 / 'qualification' / origin / 'builds' / identifier
        build_path = directory / 'build.json'
        stdout_path, stderr_path = directory / 'build.stdout', directory / 'build.stderr'
        assert all(str(path) in pins for path in (build_path, stdout_path, stderr_path))
        build = json.loads(build_path.read_text())
        command = native['build_command']
        assert build['command'] == command and command[:2] == [str(compiler), 'compile']
        assert command[2:2 + len(copies)] == copies and command[2 + len(copies)] == '-O3'
        assert command.count('--print-input') == command.count('--print-linking') == 1
        assert command[command.index('--stdlib') + 1] == str(stdlib)
        assert command[command.index('--build-dir') + 1] == str(cache)
        assert command[command.index('-o') + 1] == str(binary)
        assert type(build['exit_status']) is type(build['final_exit_status']) is int and build['exit_status'] == build['final_exit_status'] == 0
        assert build['inherited_common_mutex'] is True and not any(build.get(name) for name in FATAL_BUILD)
        inputs = printed_inputs(stdout_path.read_text(), copies, stdlib, pins)
        bound = []
        for index, (relative, item) in enumerate(zip(row['compiled_source_bundle'], native['source_bundle'])):
            author, copy = C3 / relative, Path(item['path'])
            assert not Path(relative).is_absolute() and '..' not in Path(relative).parts and author.suffix == '.c3'
            assert sha(author) == sha(copy) == item['sha256'] == pins[str(author)] == pins[str(copy)]
            assert author.read_bytes() == copy.read_bytes()
            role = 'application' if index == 0 else 'application-helper'
            assert index == 0 or relative in {'support/buffer.c3', 'support/threads.c3', 'support/gmp.c3', 'support/pcre2.c3'}
            bound.append(dict(path=str(author), sha256=item['sha256'], role=role,
                compiler_input_copy=str(copy), compiler_input_sha256=item['sha256']))
            authors.add(author); captured.add(copy)
        evidence[identifier] = dict(id=identifier, benchmark=row['benchmark'], variant=row['variant'],
            status='compiler-input-proposals-awaiting-source-role-review', source=native['source'],
            source_sha256=native['source_sha256'], binary=str(binary), binary_sha256=native['binary_sha256'],
            original_authored_source=str(C3 / row['source']), producer_loc=None, files=bound,
            source_bundle=native['source_bundle'], actual_original_compile_command=command,
            production_build_receipt=str(build_path), production_build_receipt_sha256=pins[str(build_path)],
            compiler_input_stdout=str(stdout_path), compiler_input_stdout_sha256=pins[str(stdout_path)],
            compiler_input_stderr=str(stderr_path), compiler_input_stderr_sha256=pins[str(stderr_path)],
            actual_printed_compiler_inputs=inputs, qualification_origin=origin)
    assert len(authors) == 24 and len(captured) == 25
    assert {str(path.relative_to(C3)) for path in authors if path.is_relative_to(C3 / 'support')} == {
        'support/buffer.c3', 'support/threads.c3', 'support/gmp.c3', 'support/pcre2.c3'}
    verify(pins)
    return evidence, sorted(authors), pins


@contextmanager
def terminal_evidence(output, lock):
    fcntl.flock(lock, fcntl.LOCK_EX)
    assert not output.exists(), 'Exclusive additive output; no frozen evidence overwrite'
    output.mkdir(parents=True)
    failed = False
    try:
        yield
    except BaseException as error:
        failed = True
        raw = output / 'terminal-error.log'
        with raw.open('x') as stream:
            stream.write(traceback.format_exc())
        new_json(output / 'terminal-error.json', dict(status='failed-preflight-or-replay',
            error=str(error) or type(error).__name__, error_type=type(error).__name__, error_log=str(raw),
            exclusive_mutex_held=True, actual_native_subjects=0, actual_signals_sent=0,
            canonical_LOC_admitted=False, chart_admission=False))
        raise
    finally:
        pins = {}
        try:
            for path in sorted(output.rglob('*')):
                if path.is_file(): pins[str(path)] = sha(path)
        except BaseException as error:
            new_json(output / 'terminal-pins.json', dict(status='failed-terminal-integrity',
                pins=pins, terminal_error=str(error) or type(error).__name__,
                available_bytes_not_fully_pinned=True, exclusive_mutex_held=True,
                common_timing_mutex=str(LOCK), canonical_LOC_admitted=False, chart_admission=False))
            raise
        new_json(output / 'terminal-pins.json', dict(status='failed-terminal-evidence-retained' if failed else 'completed-source-proposal-evidence-retained',
            pins=pins, exclusive_mutex_held=True, common_timing_mutex=str(LOCK),
            canonical_LOC_admitted=False, chart_admission=False))


def run(revision):
    print('WAIT common mutex: exact C3 compiler-input/source replay; no native or fixtures', flush=True)
    output = HERE / 'jobs' / revision
    with LOCK.open('a+') as lock, terminal_evidence(output, lock):
        descriptor, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (descriptor.st_dev, descriptor.st_ino) == (named.st_dev, named.st_ino)
        fixture, grammar_pins = qualified_grammar()
        inputs, authors, compiler_pins = compiler_inventory()
        pins = merge(grammar_pins, compiler_pins)
        python = Path(sys.executable).resolve(strict=True)
        assert str(python) in fixture['pins_before'] and sha(python) == fixture['pins_before'][str(python)]
        pins[str(python)] = sha(python)
        snapshot, packet = output / 'snapshot', output / 'packet'
        snapshot.mkdir(); packet.mkdir()
        for name in SNAPSHOT_FILES:
            source, target = FIXTURE_JOB / 'snapshot' / name, snapshot / name
            assert sha(source) == fixture['pins_before'][str(source)]
            shutil.copyfile(source, target)
            assert sha(target) == fixture['pins_before'][str(source)]
            pins[str(target)] = sha(target); target.chmod(0o444)
        for name in PACKET_FILES:
            source, target = HERE / name, packet / name
            shutil.copyfile(source, target)
            assert sha(source) == sha(target)
            pins[str(source)] = pins[str(target)] = sha(source); target.chmod(0o444)
        new_json(output / 'compiler-input-proposals.json', dict(status='compiler-input-proposals-awaiting-source-role-review',
            profiles=inputs, profile_count=20, unique_authored_source_count=24, unique_compiler_copy_count=25,
            qualification=dict(path=str(QUALIFICATION), sha256=QUALIFICATION_SHA), pins=pins,
            complete_printed_application_inputs=True, unknown_compiler_inputs=[], source_role_review_passed=False,
            canonical_LOC_admitted=False, chart_admission=False))
        inventory_path = output / 'compiler-input-proposals.json'
        pins[str(inventory_path)] = sha(inventory_path)
        assert not any(name in sys.modules for name in ('foreign_c3', 'c3_mask', 'foreign_core', 'canonical', 'pygments'))
        assert not any(name.startswith('pygments.') for name in sys.modules)
        sys.dont_write_bytecode = True
        pygments = Path(fixture['pygments_module']).resolve(strict=True)
        sys.path.insert(0, str(pygments.parent.parent)); sys.path.insert(0, str(snapshot))
        normalizer = importlib.import_module('foreign_c3')
        for name in ('foreign_c3', 'c3_mask', 'foreign_core', 'canonical'):
            assert Path(sys.modules[name].__file__).resolve(strict=True) == snapshot / (name + '.py')
        assert Path(sys.modules['pygments'].__file__).resolve(strict=True) == pygments
        assert sys.modules['pygments'].__version__ == '2.19.2' and normalizer.VERSION == 'c3-source-owners-0.8.5-private-v4'
        def module_identities():
            result = {}
            for name, module in sys.modules.items():
                if name in ('foreign_c3', 'c3_mask', 'foreign_core', 'canonical') or name == 'pygments' or name.startswith('pygments.'):
                    path = Path(module.__file__).resolve(strict=True)
                    assert str(path) in pins and sha(path) == pins[str(path)]
                    result[name] = str(path)
            return result
        module_identities(); verify(pins)
        counts = {}
        for author in authors:
            text = author.read_text()
            metric = normalizer.normalize(text, 'c3')
            assert metric['proposal_only'] is True and metric['eligible_for_chart'] is False
            counts[str(author)] = dict(source_sha256=pins[str(author)], source_character_length=len(text), metric=metric)
        profiles = {}
        for identifier, row in inputs.items():
            files = []
            for item in row['files']:
                counted = counts[item['path']]
                assert counted['source_sha256'] == item['sha256']
                files.append(dict(item, physical_loc=counted['metric']['physical_loc'],
                    normalized_loc=counted['metric']['normalized_loc'], diagnostics=counted['metric']['diagnostics']))
            profiles[identifier] = dict(id=identifier, benchmark=row['benchmark'], variant=row['variant'],
                source=row['source'], source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                source_bundle=files, producer_loc=None, physical_loc=sum(item['physical_loc'] for item in files),
                normalized_loc=sum(item['normalized_loc'] for item in files))
        unresolved = [dict(source=name, diagnostic=issue) for name, value in counts.items() for issue in value['metric']['diagnostics']]
        identities = module_identities(); verify(pins)
        replay = output / 'source-count-proposals.json'
        new_json(replay, dict(status='source-proposals-needing-grammar-review' if unresolved else 'source-proposals-awaiting-semantic-review',
            qualification=dict(path=str(QUALIFICATION), sha256=QUALIFICATION_SHA),
            compiler_bound_input_inventory=dict(path=str(inventory_path), sha256=pins[str(inventory_path)]),
            fixture_receipt=dict(path=str(FIXTURE), sha256=FIXTURE_SHA, status='passed'), actual_fixture_methods_reused=19,
            qualified_neutral_fixture=dict(path=str(NEUTRAL_FIXTURE), sha256=NEUTRAL_SHA, actual_tests=672),
            normalization_policy=dict(rule_version='canonical-source-units-20261012-approved-rule-v2',
                grammar=normalizer.VERSION, evidence_path=str(snapshot / 'foreign_core.py'), evidence_sha256=pins[str(snapshot / 'foreign_core.py')]),
            source_counts=counts, profiles=profiles, profile_count=20, unique_authored_source_count=24,
            unique_compiler_copy_count=25, unresolved=unresolved, module_identities=identities,
            grammar_snapshot_pins={name: digest for name, digest in pins.items() if Path(name).is_relative_to(snapshot) or Path(name).is_relative_to(pygments.parent)},
            pins=pins, actual_common_mutex_held=True, actual_native_subjects=0, actual_signals_sent=0,
            canonical_LOC_admitted=False, chart_admission=False, full_performance_runs=0,
            producer_LOC_mutated=False, count_override=False, parser_pass_is_not_semantic_certification=True))
        print(json.dumps(dict(inventory=str(inventory_path), inventory_sha256=pins[str(inventory_path)],
            replay=str(replay), replay_sha256=sha(replay), actual_profiles=20, actual_unique_authored_inputs=24,
            actual_captured_input_copies=25, fixture_methods_reused=19, unresolved=len(unresolved), canonical_LOC_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    run(args.revision)


if __name__ == '__main__':
    main()
