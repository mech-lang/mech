import json,sys
from pathlib import Path
census = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('/private/tmp/mech-buffer-liveness-round56-20261031/sites-after/census.json')
sites = json.loads(census.read_text())['sites']
def has(s, needle):
    return any(needle in f for f in s['frames'])
def at(s, offset):
    return any('mech_canonical_program_entry_v1' in f and f.endswith('+ ' + str(offset)) for f in s['frames'])
checks = {
    'unsealed_bound_call_clone': lambda s: has(s, 'native_semantics9bind_call'),
    'owned_call_reclone_in_refinement': lambda s: has(s, 'apply_observed_turn_demand') and has(s, 'CallMemoryPlan') and has(s, 'clone'),
    'temporary_call_clone_in_base_planner': lambda s: has(s, 'plan_turn_memory') and has(s, 'CallMemoryPlan') and has(s, 'clone'),
    'temporary_one_call_program_wrapper': lambda s: has(s, 'plan_current') and not has(s, 'plan_turn_memory'),
    'discarded_scope_allocation_copy': lambda s: any('scope_turn_call' in f and f.endswith('+ 348') for f in s['frames']),
    'operation_descriptor_validation_clones': lambda s: has(s, 'ResolvedOperationDescriptor8validate'),
    'current_composite_constructor_rebuild': lambda s: has(s, 'current_constructor'),
}
result = {'reviewed_head': '46617b61f3ccf0e4a6ddf27fe37f9f4003016ac4', 'census': str(census), 'families': {}}
for name, check in checks.items():
    a = [s for s in sites if check(s)]
    result['families'][name] = {
        'requests': sum(s['count'] for s in a),
        'requested_bytes': sum(s['requested_bytes'] for s in a),
        'seed_record_requests': sum(s['count'] for s in a if at(s, 92004)),
        'seed_matrix_requests': sum(s['count'] for s in a if at(s, 92212)),
        'outside_seed_requests': sum(s['count'] for s in a if not at(s, 92004) and not at(s, 92212)),
    }
result['overlaps'] = {}
for i, (name, check) in enumerate(checks.items()):
    for other, other_check in list(checks.items())[i+1:]:
        requests = sum(s['count'] for s in sites if check(s) and other_check(s))
        if requests:
            result['overlaps'][name + ' / ' + other] = requests
print(json.dumps(result, indent=2))
