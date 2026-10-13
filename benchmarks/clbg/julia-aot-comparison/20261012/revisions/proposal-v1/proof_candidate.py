"""Source-only observation contract; no native execution or AOT admission."""

INTERPRETER_HOOKS = {
    'jl_fptr_interpret_call', 'jl_interpret_opaque_closure',
    'jl_interpret_toplevel_thunk', 'jl_interpret_toplevel_expr_in',
}
JIT_HOOKS = {'jl_emit_codeinst_to_jit', 'jl_generate_fptr_for_unspecialized',
             'jl_jit_abi_converter_impl', 'JuliaOJIT::addModule'}


class UnqualifiedAot(AssertionError):
    pass


def inspect_observation(record):
    """Shape/rejection oracle; actual pins/address/semantics need root review."""
    assert record['actual_common_mutex_held'] is True
    assert record['actual_native_process_owned'] is True
    assert record['actual_compile_disabled'] is True
    assert record['same_unchanged_scored_binary_or_unscored_canary'] is True
    assert record['no_observer_drop_or_detach'] is True
    assert record['all_thread_events_observed'] is True
    assert record['observer_armed_before_runtime_initialization'] is True
    assert record['observer_retained_through_shutdown'] is True
    assert set(record['hook_evidence']) == INTERPRETER_HOOKS | JIT_HOOKS
    for symbol, evidence in record['hook_evidence'].items():
        assert evidence['symbol'] == symbol and evidence['resolved_address']
        assert evidence['actual_resolved_image_sha256']
        assert evidence['raw_resolution_artifact'] and evidence['raw_events_artifact']
        assert evidence['actual_linkage_name'] and evidence['reviewed_source_route']
        assert type(evidence['entry_count']) is int and evidence['entry_count'] >= 0
        if evidence['entry_count']:
            raise UnqualifiedAot('Observed Julia runtime fallback/code generation: ' + symbol)
    if record['missing_native_code_error'] is True or record['exit_status'] != 0:
        raise UnqualifiedAot('Runtime missing-code/output qualification failed')
    # Empty trace-compile is retained as corroboration, never approval by itself.
    assert type(record['trace_compile_bytes']) is int and record['trace_compile_bytes'] >= 0
    return dict(observation_shape_consistent=True, runtime_aot_proven=False,
                chart_admission=False, requires_actual_image_and_root_semantic_review=True)


def require_actual_negative_canary(record):
    assert record['unscored_negative_canary'] is True
    assert record['actual_target_native_coverage_absent'] is True
    assert record['actual_target_method_identity_artifact']
    assert record['expected_input_not_frozen'] is True
    try:
        inspect_observation(record)
    except UnqualifiedAot as error:
        return dict(status='negative-canary-rejected-by-observation-contract',
                    rejection=str(error), chart_admission=False,
                    actual_canary_proven=False, requires_root_raw_review=True)
    raise AssertionError('Missing-native canary was accepted; observer/coverage qualification must stop')
