"""Unexecuted candidate: raw unchanged scored-tree ARM and source-matched IR.

Call only from the root-reviewed native work runner while holding common_lock.
No boolean allocation/retention conclusion is synthesized from the artifacts.
"""
import json
from pathlib import Path


def emit(preparation, guard, harness, qualification_path, output, lock_fd):
    qpath = Path(qualification_path)
    q = json.loads(qpath.read_text())
    assert q['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review'
    guard.verify(q['pins'])
    assert not output.exists(), 'Never overwrite an earlier allocator evidence attempt'
    output.mkdir()
    profiles = {}
    pins = dict(q['pins'])
    for row in q['profiles']:
        if row['benchmark'] != 'binarytrees': continue
        source = Path(row['source']); binary = Path(row['command'][0])
        assert preparation.sha(source) == row['source_sha256']
        assert preparation.sha(binary) == row['binary_sha256']
        build_receipt = Path(row['production_build_receipt'])
        original = json.loads(build_receipt.read_text())
        assert original['inherited_common_mutex'] is True and original['exit_status'] == 0
        assert original['final_exit_status'] == 0 and not original.get('cleanup_error')
        command = original['command']
        compiler = Path(command[0]); compiler_sha = preparation.sha(compiler)
        assert q['pins'][str(compiler)] == compiler_sha
        directory = output / row['id']; directory.mkdir()
        environment = dict(__import__('os').environ, PATH='/usr/bin:/bin', **row['runtime_environment'])
        for key in ('PONY_PATH', 'PONYPATH', 'PONYC_OPTIONS', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS',
                    'DYLD_INSERT_LIBRARIES', 'DYLD_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH'):
            environment.pop(key, None)
        sdk = command[command.index('--sysroot') + 1]
        environment['SDKROOT'] = sdk
        # ARM comes from the existing scored binary, never the diagnostic copy.
        preparation.owned_command(guard, harness, ['/usr/bin/otool', '-tvV', str(binary)],
            environment, directory / 'arm', lock_fd, 120, 'scored-arm')
        ir_directory = directory / 'ir'; ir_directory.mkdir()
        ir_command = list(command)
        ir_command[ir_command.index('--output') + 1] = str(ir_directory)
        ir_command[-1:-1] = ['--pass', 'ir']
        preparation.owned_command(guard, harness, ir_command, environment,
            directory / 'ir-command', lock_fd, 600, 'scored-source-ir')
        ir_files = list(ir_directory.glob('*.ll'))
        assert len(ir_files) == 1, 'Exact source IR output must be unambiguous'
        # owned_command persists raw stdout as <name>.stdout (see native guard).
        arm = directory / 'arm' / 'scored-arm.stdout'
        assert arm.is_file() and arm.stat().st_size > 0
        llvm = ir_files[0]
        profiles[row['id']] = dict(binary=str(binary), binary_sha256=row['binary_sha256'],
            source=str(source), source_sha256=row['source_sha256'], compiler=str(compiler), compiler_sha256=compiler_sha,
            actual_scored_binary=True, diagnostic_binary_substituted=False,
            production_build_receipt=dict(path=str(build_receipt), sha256=preparation.sha(build_receipt)),
            arm_disassembly=dict(path=str(arm), sha256=preparation.sha(arm), binary_sha256=row['binary_sha256'],
                source_sha256=row['source_sha256'], compiler_sha256=compiler_sha, production_compile_command=command,
                reviewed_symbols=[], annotated_leaf_control_flow=[]),
            llvm_ir=dict(path=str(llvm), sha256=preparation.sha(llvm), source_sha256=row['source_sha256'],
                compiler_sha256=compiler_sha, production_compile_command=command, actual_ir_command=ir_command,
                reviewed_symbols=[], annotated_leaf_control_flow=[]),
            leaf_path_allocates_fresh_node=None, interior_path_allocates_fresh_node=None,
            no_static_or_shared_tree_graph=None, long_root_storage_retained_through_worker_joins=None)
        pins = preparation.merge(pins, preparation.pin_tree(directory), {str(build_receipt): preparation.sha(build_receipt)})
    assert len(profiles) == 2
    guard.verify(pins)
    receipt = output / 'raw-evidence.json'
    preparation.new_json(receipt, dict(status='scored-tree-raw-evidence-awaiting-root-semantic-review',
        common_mutex_held=True, qualification=str(qpath), qualification_sha256=preparation.sha(qpath),
        profiles=profiles, root_semantic_review_required=True, score_eligible=False,
        no_automatic_retry=True))
    pins = preparation.merge(pins, {str(receipt): preparation.sha(receipt)})
    return dict(path=str(receipt), sha256=preparation.sha(receipt), root_semantic_review_required=True), pins
