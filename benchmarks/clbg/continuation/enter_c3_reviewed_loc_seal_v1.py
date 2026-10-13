"""Guard actual C3 meaning/fixture closure, then exec its unchanged sealer.

Bundled Python3.12 orchestration only. The original qualified Python3.14 image
runs the sealer and revalidates its unchanged compiler/grammar services. No
fixture, parser, compiler, benchmark, native probe or signal is started here.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
C3 = CLBG / 'c3-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
REPLAY_ROOT = CLBG / 'loc-normalization/20261012/c3-source-replay-v2/jobs/grammar5-source24-v1'
REPLAY = REPLAY_ROOT / 'source-count-proposals.json'
REPLAY_SHA = '877e58726c90511caea86ad29b5ca56654bfd0ffbc83908229a56d388db5f16e'
REPLAY_TERMINAL_SHA = 'a59779dacdfff83200fea474843f4d17a64262063a7128d948314a53575fe5dc'
MEANING = C3 / 'source-counts/reviews/grammar5-source24-v1/meaning-review.json'
MEANING_SHA = '9802bfeeb0cdfefd5b557213fa91bee727bd4020f6e288fb57f826ea31ee1b56'
CAPTURE = C3 / 'source-counts/reviews/meaning-capture-v1'
SEALER = C3 / 'source-counts/seal_overlay_v2.py'
SEALER_SHA = '25e686b3bfe0feb24c5023a42a6d9466ad2271bc599bbe7a54fdccc9ad8c84a8'
DEPENDENCY = HERE / 'owned-jobs/c3-grammar5-source24-reviewed-next-stage-v1'
OUT = HERE / 'jobs/c3-reviewed-loc-seal-entry-v1'
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
 'test_multiline_string_array_commas_have_complete_data_owner',
 'test_multiline_probability_and_body_record_rows_preserve_wrapping',
 'test_multiline_data_owner_keeps_callback_write_and_mutation_gates',
 'test_comma_region_after_initializer_close_is_not_data')
IDS = sorted('test_foreign_c3.C3Fixtures.' + method for method in METHODS)


def require(value, why):
    if not value: raise ValueError(why)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def main():
    print('WAIT closed dependency lease: actual C3 v5 source replay', flush=True)
    with (DEPENDENCY / 'owner.lock').open('a+') as lease:
        fcntl.flock(lease, fcntl.LOCK_EX)
        held, named = os.fstat(lease.fileno()), (DEPENDENCY / 'owner.lock').stat()
        require((held.st_dev, held.st_ino) == (named.st_dev, named.st_ino), 'Different dependency lease')
        require((DEPENDENCY / 'terminal.json').is_file(), 'Replay ended without durable terminal')
        print('WAIT common mutex: exact C3 reviewed LOC entry', flush=True)
        with LOCK.open('a+') as common:
            fcntl.flock(common, fcntl.LOCK_EX)
            held, named = os.fstat(common.fileno()), LOCK.stat()
            require((held.st_dev, held.st_ino) == (named.st_dev, named.st_ino), 'Different common mutex')
            OUT.mkdir(parents=True, exist_ok=False)
            pins, argv = {}, None
            def bind(path, expected=None):
                path = Path(path).resolve(strict=True); digest = sha(path)
                require(expected is None or digest == expected, 'Changed pinned input: ' + str(path))
                require(str(path) not in pins or pins[str(path)] == digest, 'Conflicting pin')
                pins[str(path)] = digest; return path, digest
            def read(path, expected=None):
                actual, digest = bind(path, expected)
                return json.loads(actual.read_text()), digest
            try:
                terminal, _ = read(DEPENDENCY / 'terminal.json')
                require(terminal['status'] == 'command-exited-zero-awaiting-report-review' and
                    type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
                    terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and
                    terminal['stop_signals'] == [] and not any(terminal.get(k) for k in
                    ('error','error_type','exception_cleanup','error_log_write_failure')), 'Replay durable outcome not normal')
                request, _ = read(DEPENDENCY / 'request.json')
                started, _ = read(DEPENDENCY / 'command-started.json')
                require(request['command'] == started['command'] and request['cwd'] == started['cwd'] and
                    started['command_pid'] == terminal['command_pid'] and
                    started['command_identity'] == terminal['command_identity'] and
                    started['command_identity_observation_missing'] is False and
                    request['common_mutex_owned_by_target_command'] is True,
                    'Changed original durable replay command identity')
                replay, _ = read(REPLAY, REPLAY_SHA)
                final, _ = read(REPLAY_ROOT / 'terminal-pins.json', REPLAY_TERMINAL_SHA)
                require(final['status'] == 'completed-source-proposal-evidence-retained' and
                    final['exclusive_mutex_held'] is True and final['common_timing_mutex'] == str(LOCK) and
                    final['canonical_LOC_admitted'] is final['chart_admission'] is False and
                    not final.get('available_bytes_not_fully_pinned', False), 'Replay terminal integrity not qualified')
                require(final['pins'][str(REPLAY)] == REPLAY_SHA, 'Terminal/replay mismatch')
                for path, digest in final['pins'].items(): bind(path, digest)
                for path, digest in replay['pins'].items(): bind(path, digest)
                require(replay['status'] == 'source-proposals-awaiting-semantic-review' and replay['unresolved'] == [] and
                    (replay['profile_count'], replay['unique_authored_source_count'], replay['unique_compiler_copy_count']) == (20,24,25) and
                    replay['actual_fixture_methods_reused'] == 23 and replay['actual_common_mutex_held'] is True and
                    replay['canonical_LOC_admitted'] is replay['chart_admission'] is replay['count_override'] is False,
                    'Unqualified actual replay')
                inventory, _ = read(replay['compiler_bound_input_inventory']['path'], replay['compiler_bound_input_inventory']['sha256'])
                require((inventory['profile_count'], inventory['unique_authored_source_count'], inventory['unique_compiler_copy_count']) == (20,24,25) and
                    inventory['unknown_compiler_inputs'] == [] and inventory['complete_printed_application_inputs'] is True,
                    'Incomplete actual compiler inputs')
                prior = replay['preserved_prior_replay']
                old, _ = read(prior['path'], prior['sha256'])
                old_terminal, _ = read(Path(prior['path']).parent / 'terminal-pins.json', prior['terminal_sha256'])
                require(old['status'] == 'source-proposals-needing-grammar-review' and len(old['unresolved']) == 46 and
                    prior['original_diagnostics'] == 46 and prior['source_or_report_mutations'] == 0 and prior['prior_units_not_overwritten'] is True and
                    old_terminal['status'] == 'completed-source-proposal-evidence-retained', 'Original46 evidence lost')
                for path, digest in old_terminal['pins'].items(): bind(path, digest)
                fixture, _ = read(replay['fixture_receipt']['path'], replay['fixture_receipt']['sha256'])
                require(fixture['status'] == 'passed-awaiting-source-review' and type(fixture['tests_run']) is int and
                    fixture['tests_run'] == 23 and fixture['pins_before'] == fixture['pins_after'] and
                    fixture['exact_test_execution'] is fixture['pin_reconciliation_passed'] is True, 'Actual23 fixture closure missing')
                for key in ('expected_ids','discovered_ids','started_ids'):
                    require(sorted(fixture[key]) == IDS and len(fixture[key]) == len(set(fixture[key])) == 23, 'Wrong actual23 identities')
                for key in ('failures','errors','skipped','expected_failures','unexpected_successes','benchmark_processes_started','subprocesses_started_by_fixtures'):
                    require(type(fixture[key]) is int and fixture[key] == 0, 'Unsuccessful/native fixture outcome')
                for path, digest in fixture['pins_before'].items(): bind(path, digest)
                bind(fixture['raw_fixture_log']['path'], fixture['raw_fixture_log']['sha256'])
                fixture_terminal = Path(replay['fixture_receipt']['path']).parent / 'fixture-receipt-pin.json'
                fixture_final, _ = read(fixture_terminal, replay['pins'][str(fixture_terminal)])
                require(fixture_final['receipt_sha256'] == replay['fixture_receipt']['sha256'] and fixture_final['common_mutex_held'] is True,
                    'Fixture terminal binding changed')
                review, review_sha = read(MEANING, MEANING_SHA)
                require(review['status'] == 'exact24-source-owner-meaning-reviewed-awaiting-guarded-seal' and review['language'] == 'c3' and
                    review['actual_source_replay'] == dict(path=str(REPLAY),sha256=REPLAY_SHA) and review['actual_qualified_preparation'] == replay['qualification'] and
                    review['unresolved'] == [] and review['peer_review_pending'] is False and review['counts_transcribed_from_actual_replay'] is True and
                    review['source_only_peer_review']['status'] == 'accepted-all24-source-owners-all20-bundles-and-effective-totals', 'Meaning review not accepted')
                require(set(review['sources']) == set(replay['source_counts']) == set(old['source_counts']), 'Wrong24 meaning inventory')
                capture, _ = read(CAPTURE / 'capture.json')
                bind(CAPTURE / 'source-count-proposals.json', REPLAY_SHA)
                bind(CAPTURE / 'terminal-pins.json', REPLAY_TERMINAL_SHA)
                for source, counted in replay['source_counts'].items():
                    item, metric = review['sources'][source], counted['metric']
                    bind(source, counted['source_sha256']); bind(capture['sources'][source]['snapshot'], counted['source_sha256'])
                    require(item['source'] == source and item['source_sha256'] == counted['source_sha256'] and
                        item['physical_loc'] == metric['physical_loc'] and item['normalized_loc'] == metric['normalized_loc'] and
                        item['unresolved'] == [] and metric['diagnostics'] == [] and item['canonical_owner_events_reviewed'] is True and
                        item['owner_review'] and item['source_role_review'], 'Incomplete actual source meaning')
                    for key, actual in (('canonical_events_reviewed','canonical_events'),('expansions_reviewed','expansions'),
                        ('complete_data_fragment_certificates_reviewed','complete_data_fragment_certificates'),('scope_evidence_reviewed','scope_evidence')):
                        require(item[key] == metric[actual], 'Changed exact reviewed source projection')
                    require(all(metric[k] == old['source_counts'][source]['metric'][k] for k in
                        ('physical_loc','normalized_loc','canonical_events','expansions')), 'Unexpected unit change')
                require(set(review['profiles']) == set(replay['profiles']) == set(inventory['profiles']), 'Wrong20 bundle review')
                preparation, _ = read(replay['qualification']['path'], replay['qualification']['sha256'])
                rows = {row['id']: row for row in preparation['profiles']}
                for identity, counted in replay['profiles'].items():
                    item = review['profiles'][identity]
                    require(item['complete_authored_source_bundle'] == rows[identity]['compiled_source_bundle'] and
                        item['effective_physical_loc'] == counted['physical_loc'] and item['effective_normalized_loc'] == counted['normalized_loc'] and
                        item['benchmark'] == counted['benchmark'] and item['variant'] == counted['variant'] and item['unresolved'] == [], 'Changed20 effective bundle')
                for path in (MEANING, MEANING.parent / 'REVIEW.md', SEALER, SEALER.parent / 'OVERLAY_V2_REVIEW.md', Path(__file__).resolve()):
                    require(not path.stat().st_mode & 0o222, 'Reviewed source or completed packet not frozen')
                    bind(path)
                bind(SEALER, SEALER_SHA)
                python = Path(fixture['python_executable']); real_python = python.resolve(strict=True)
                require(str(real_python) in fixture['pins_before'] and fixture['python_version'].startswith('3.14.'), 'Original qualified3.14 image required')
                bind(real_python, fixture['pins_before'][str(real_python)])
                require(not (C3 / 'source-counts/canonical-loc-overlay-v2').exists(), 'Never retry or overwrite an overlay')
                argv = [str(python), '-B', '-u', str(SEALER), '--revision','canonical-loc-overlay-v2',
                    '--replay-sha256',REPLAY_SHA,'--review-file',str(MEANING),'--review-sha256',review_sha]
                save(OUT / 'handoff.json',dict(status='accepted-actual24-review-entering-unchanged-sealer',command=argv,
                    meaning_review_sha256=review_sha,sealer_sha256=pins[str(SEALER)],pins=pins,common_mutex_held=True,
                    native_commands=0,fixture_reexecutions=0,source_or_timing_mutations=0,chart_admission=False,automatic_retry=False))
            except BaseException as error:
                (OUT / 'error.log').write_text(traceback.format_exc())
                save(OUT / 'failed-handoff.json',dict(status='failed-actual-review-precondition',error=str(error) or type(error).__name__,
                    error_type=type(error).__name__,available_input_pins=pins,common_mutex_held=True,automatic_retry=False,chart_admission=False))
                raise
    # Original sealer reacquires and rechecks all pins; the durable private lease
    # and command PID span this explicit handoff. No second benchmark owner.
    os.execv(str(python), argv)


if __name__ == '__main__': main()
