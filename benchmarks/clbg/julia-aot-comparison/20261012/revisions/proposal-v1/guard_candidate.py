"""Declarative Julia AOT qualification candidate; starts no process or audit.

Callers must bind actual pinned raw evidence under the common mutex. This is not
a completed preparation runner and cannot publish, seal or measure anything.
"""
BENCHMARKS = {'nbody', 'mandelbrot', 'knucleotide', 'spectralnorm', 'revcomp',
              'fannkuchredux', 'fasta', 'binarytrees', 'regexredux', 'pidigits'}
JULIA_LANGUAGE_GATES = (
    'actual_compile_disabled', 'actual_image_native_method_coverage',
    'missing_native_code_negative_control_rejected',
    'no_julia_jit_on_scored_paths', 'no_julia_interpreter_on_scored_paths',
    'no_runtime_source_include_or_eval', 'no_runtime_package_compilation',
    'direct_compiled_entry_without_launcher_eval',
    'final_image_ir_and_module_policies_reviewed',
    'runtime_args_stdin_not_image_constants', 'fresh_runtime_mutable_state',
    'complete_runtime_required_work', 'complete_output_reference_match',
    'compiled_source_bundle_exact', 'native_image_launcher_runtime_pins_exact',
    'ordinary_gc_enabled', 'observed_compute_worker_cap',
    'inherited_common_mutex', 'exact_native_command_and_group',
    'valid_environment', 'power_profile_unchanged', 'full_cleanup', 'pins_unchanged',
)


def require_candidate_record_shape(record):
    assert record['benchmark'] in BENCHMARKS
    assert record['variant'] in {'compact', 'optimized'}
    assert record['language'] == 'julia-aot' and record['execution_mode'] == 'aot'
    checks = record['qualification_checks']
    assert set(checks) == set(JULIA_LANGUAGE_GATES)
    assert all(checks[gate] is True for gate in JULIA_LANGUAGE_GATES)
    evidence = record['raw_evidence']
    for kind in ('compiled_input_inventory', 'image_build', 'runtime_options',
                 'native_method_coverage', 'missing_code_negative_control',
                 'julia_jit_and_interpreter_observation', 'runtime_state_review',
                 'direct_launcher_and_exported_entry_review',
                 'final_image_ir_and_module_policy_review',
                 'output_validation', 'native_process_ownership'):
        artifact = evidence[kind]
        assert artifact['path'] and artifact['sha256'] and artifact['actual_record'] is True
    assert record['source_adaptations_reviewed'] is True
    assert record['library_policy']['domain_kernel'] is False
    # Standard PCRE2 compilation/JIT is not a Julia-language compiler event.
    assert record['library_policy']['generic_vendor_compilation_disclosed'] is True
    return dict(schema_complete=True, runtime_aot_proven=False, chart_admission=False)


def require_full_candidate_shape(records):
    assert len(records) == 20
    assert {(row['benchmark'], row['variant']) for row in records} == {
        (benchmark, variant) for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
    for row in records: require_candidate_record_shape(row)
    return dict(status='candidate-contract-satisfied-awaiting-root-raw-review',
                chart_admission=False, timing_launch_authorized=False)
