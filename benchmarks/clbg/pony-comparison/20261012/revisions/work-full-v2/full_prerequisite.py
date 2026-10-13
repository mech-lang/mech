"""Unexecuted full-sealer prerequisite candidate; call inside the common lock.

The root must first inspect actual unchanged-production ARM/LLVM evidence and
approve its exact receipt digest. True-valued claims without raw evidence and
the root-approved immutable pin cannot admit a full timing suite.
"""


def require_tree_proof(qualification, work, allocation, approved_allocation_sha256, sha, load_json):
    assert qualification['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    assert work['status'] == 'native-work-v2-passed-awaiting-scored-binary-allocation-review'
    assert allocation['status'] == 'root-reviewed-unchanged-scored-binary-allocation'
    assert sha(allocation['receipt_path']) == approved_allocation_sha256
    assert allocation['common_mutex_held'] is True
    rows = {row['id']: row for row in qualification['profiles'] if row['benchmark'] == 'binarytrees'}
    assert len(rows) == 2
    work_rows = {row['id']: row for row in work['attempts'] if row['benchmark'] == 'binarytrees'}
    assert set(work_rows) == set(rows) == set(allocation['profiles'])
    for key, row in rows.items():
        proof = work_rows[key]['proof']
        for gate in ('actual_graph_identities', 'concurrent_graphs_disjoint',
                     'long_graph_retained_across_all_worker_joins', 'long_root_retained'):
            assert proof.get(gate) is True, ('Missing actual v2 tree proof', key, gate)
        review = allocation['profiles'][key]
        binary = row['command'][0]
        assert review['binary'] == binary and review['binary_sha256'] == row['binary_sha256']
        assert sha(binary) == row['binary_sha256']
        assert review['source_sha256'] == row['source_sha256']
        assert sha(row['source']) == row['source_sha256']
        build = review['production_build_receipt']
        assert sha(build['path']) == build['sha256']
        original = load_json(build['path'])
        assert original['inherited_common_mutex'] is True and original['exit_status'] == 0
        assert original['final_exit_status'] == 0 and not original.get('cleanup_error')
        compile_command = original['command']
        compiler_sha256 = qualification['pins'][compile_command[0]]
        assert sha(compile_command[0]) == compiler_sha256 == review['compiler_sha256']
        assert review['actual_scored_binary'] is True and review['diagnostic_binary_substituted'] is False
        for gate in ('leaf_path_allocates_fresh_node', 'interior_path_allocates_fresh_node',
                     'no_static_or_shared_tree_graph', 'long_root_storage_retained_through_worker_joins'):
            assert review.get(gate) is True
        # ARM is from the exact unchanged scored binary; LLVM is additional
        # source/flag-matched evidence and never a substitute binary identity.
        for kind in ('arm_disassembly', 'llvm_ir'):
            artifact = review[kind]
            assert artifact['path'] and artifact['sha256'] == sha(artifact['path'])
            assert artifact['source_sha256'] == row['source_sha256']
            assert artifact['compiler_sha256'] == compiler_sha256
            assert artifact['production_compile_command'] == compile_command
            assert artifact['reviewed_symbols'] and artifact['annotated_leaf_control_flow']
        assert review['arm_disassembly']['binary_sha256'] == row['binary_sha256']
    return dict(tree_work_revision='work-v2', allocation_review_sha256=approved_allocation_sha256,
                unchanged_scored_binary_allocation_verified=True)
