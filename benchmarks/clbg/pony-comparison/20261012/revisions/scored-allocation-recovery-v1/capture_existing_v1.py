"""Capture failed Pony work evidence and complete IR ownership proposals only.

No native, benchmark, fixture, compiler, disassembler, parser service or signal
is started. This additive source candidate must be independently reviewed and
frozen before an exact-digest command can run after the publication window.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
COHORT = HERE.parents[1]
CLBG = COHORT.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
FAILED = COHORT / 'work-qualification/private-v8-work-v5'
DEPENDENCY = CLBG / 'continuation/owned-jobs/pony-two-tree-reviewed-next-stage-v2'
QUALIFICATION = COHORT / 'qualification/private-v8/report.json'
QUALIFICATION_SHA = '4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159'
COMPACT = 'pony-binarytrees-compact-precedence-v4'
IR_ROOT = FAILED / 'scored-tree-allocation' / COMPACT
OTOOL = '/usr/bin/otool'
LLVM_DIS = Path('/opt/homebrew/Cellar/llvm/23.1.3_1/bin/llvm-dis')
HOST_LINKAGE = CLBG / 'continuation/jobs/python-framework-host-linkage-v1/host-linkage.json'
HOST_LINKAGE_SHA = '9a1c0fa4fc330cf0c7c176349c220640ad4714c5e8e081eaead84c00b89729af'


def require(value, message):
    if not value: raise ValueError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value,stream,indent=2);stream.write('\n')


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','source-sha256','review-sha256','failed-report-sha256','manifest-sha256'):
        parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    require(re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision), 'Fresh exclusive revision required')
    require(all(re.fullmatch('[0-9a-f]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256')), 'Exact reviewed digest required')
    print('WAIT original failed Pony work owner lease',flush=True)
    with (DEPENDENCY / 'owner.lock').open('r+') as dependency:
        fcntl.flock(dependency,fcntl.LOCK_EX)
        held,named = os.fstat(dependency.fileno()),(DEPENDENCY / 'owner.lock').stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different original durable lease')
        print('WAIT common mutex: capture original Pony trace and IR proposals only',flush=True)
        with LOCK.open('r+') as common:
            fcntl.flock(common,fcntl.LOCK_EX)
            held,named = os.fstat(common.fileno()),LOCK.stat()
            require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino) and held.st_ino == 79859134,
                'Different named common mutex')
            output = HERE / 'jobs' / args.revision
            output.mkdir(parents=True,exist_ok=False)
            pins = {}
            outcome = dict(status='failed-original-evidence-capture',chart_admission=False,
                actual_native_commands=0,actual_benchmark_runs=0,actual_fixture_reexecutions=0)
            def bind(path, expected=None):
                path = Path(path).resolve(strict=True);value = sha(path)
                require(expected is None or value == expected, 'Changed original evidence: '+str(path))
                require(str(path) not in pins or pins[str(path)] == value, 'Conflicting original pin')
                pins[str(path)] = value;return value
            def read(path, expected=None):
                bind(path,expected);return json.loads(Path(path).read_text())
            try:
                for path in (Path(__file__),HERE / 'REVIEW.md'):
                    require(not path.stat().st_mode & 0o222, 'Released source capture must be frozen')
                bind(__file__,args.source_sha256);bind(HERE / 'REVIEW.md',args.review_sha256)
                terminal = read(DEPENDENCY / 'terminal.json')
                request,started = read(DEPENDENCY / 'request.json'),read(DEPENDENCY / 'command-started.json')
                worker_started = read(DEPENDENCY / 'worker-started.json')
                read(HOST_LINKAGE,HOST_LINKAGE_SHA)
                original_command = ['/opt/homebrew/opt/python@3.14/bin/python3.14','-B','-u',
                    str(CLBG / 'continuation/enter_reviewed_followup_v2.py'),'--stage','pony-work',
                    '--host-linkage',str(HOST_LINKAGE),'--host-linkage-sha256',HOST_LINKAGE_SHA]
                require(terminal['status'] == 'command-failed' and type(terminal['exit_code']) is int and terminal['exit_code'] == 1 and
                    terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and terminal['stop_signals'] == [] and
                    not any(terminal.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure')), 'Unexpected original failed durable closure')
                require(request['command'] == started['command'] == worker_started['command'] == original_command and
                    request['cwd'] == started['cwd'] == worker_started['cwd'] == str(CLBG.parent.parent) and
                    request['common_mutex_owned_by_target_command'] is True and request['automatic_retry'] is False and
                    started['command_identity_observation_missing'] is False and started['command_pid'] == terminal['command_pid'] and
                    started['command_identity'] == terminal['command_identity'] and
                    worker_started['worker_identity'] == terminal['worker_identity'] and worker_started['inherited_private_lease'] is True and
                    worker_started['detached_session'] is True and worker_started['common_mutex_owned_by_target_command'] is True,
                    'Changed original failed command identity')
                report = read(FAILED / 'report.json',args.failed_report_sha256)
                manifest = read(FAILED / 'manifest.json',args.manifest_sha256)
                qualification = read(QUALIFICATION,QUALIFICATION_SHA)
                require(report['status'] == 'stopped' and report['error_type'] == 'AssertionError' and
                    report['error'] == 'Exact source IR output must be unambiguous' and not report.get('terminal_integrity_error'), 'Original work failure has another cause')
                require(report['manifest'] == str(FAILED / 'manifest.json') and report['manifest_sha256'] == args.manifest_sha256 and
                    report['fresh_actual_work_traces'] == 2 and report['reused_actual_work_traces'] == 4 and
                    report['old_attempts_relabelled_as_new'] is False and report['full_performance_runs'] == 0 and
                    len(report['attempts']) == len({row['id'] for row in report['attempts']}) == 6, 'Incomplete original six trace records')
                require(manifest['qualification'] == str(QUALIFICATION) and manifest['qualification_sha256'] == QUALIFICATION_SHA and
                    manifest['source_work_profiles'] == 20 and manifest['reused_fixture_tests'] == 14 and manifest['fresh_fixture_tests'] == 0 and
                    manifest['production_operations_preserved'] is True and manifest['workload_code_delegated_to_native'] is False, 'Original frozen work manifest differs')
                for values in (qualification['pins'],manifest['pins'],report['pins']):
                    for path,value in values.items():bind(path,value)
                originals = {row['id']:row for row in qualification['profiles']}
                require(len(originals) == 20 and qualification['status'] == 'bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review', 'Original qualification differs')
                trace_rows = {row['id']:row for row in manifest['rows']}
                require(len(trace_rows) == len(manifest['rows']) == 6 and set(trace_rows) == {attempt['id'] for attempt in report['attempts']},
                    'Retained trace inventory differs from original manifest')
                tree_attempts = []
                for attempt in report['attempts']:
                    require(attempt['qualified'] is True and attempt['complete_output_match'] is True and
                        attempt['source'] == trace_rows[attempt['id']]['source'] == originals[attempt['id']]['source'] and
                        attempt['source_sha256'] == trace_rows[attempt['id']]['source_sha256'] == originals[attempt['id']]['source_sha256'],
                        'Unqualified retained actual trace')
                    bind(attempt['source'],attempt['source_sha256']);bind(attempt['trace'],attempt['trace_sha256'])
                    bind(attempt['trace_source'],attempt['trace_source_sha256'])
                    if attempt['benchmark'] == 'binarytrees':
                        require(attempt['input_n'] == 6 and attempt['proof']['actual_record_mutation_rejections'] ==
                            ['missing-work-record','too-many-scheduler-identities'], 'Changed actual tree mutation checks')
                        tree_attempts.append(attempt)
                require(len(tree_attempts) == 2, 'Both actual completed tree observations required')
                row = originals[COMPACT]
                bind(row['command'][0],row['binary_sha256'])
                production = read(row['production_build_receipt'],qualification['pins'][row['production_build_receipt']])
                ir_path,arm_path = IR_ROOT / 'ir-command/scored-source-ir.json',IR_ROOT / 'arm/scored-arm.json'
                ir_receipt = read(ir_path,report['pins'][str(ir_path)])
                arm_receipt = read(arm_path,report['pins'][str(arm_path)])
                for receipt in (production,ir_receipt,arm_receipt):
                    require(receipt['inherited_common_mutex'] is True and type(receipt['exit_status']) is type(receipt['final_exit_status']) is int and
                        receipt['exit_status'] == receipt['final_exit_status'] == 0 and
                        not any(receipt.get(key) for key in ('error','error_type','cleanup_error','cleanup_errors','remaining_group_members')), 'Original owned evidence command not normal')
                toolchain = manifest['toolchain'];command = production['command']
                require(command[0] == toolchain['compiler'] and command[-1] == str(Path(row['source']).parent) and
                    command[command.index('--output')+1] == str(Path(row['command'][0]).parent) and
                    command[command.index('--bin-name')+1] == row['id'] and
                    command[command.index('--sysroot')+1] == toolchain['sdk'] and '--pass' not in command,
                    'Original pinned production compile ownership differs')
                bind(command[0],toolchain['compiler_sha256'])
                expected = list(command);expected[expected.index('--output')+1] = str(IR_ROOT / 'ir')
                expected[-1:-1] = ['--pass','ir']
                require(ir_receipt['command'] == expected and arm_receipt['command'] == [OTOOL,'-tvV',row['command'][0]],
                    'Original exact source/binary/compile route differs')
                for receipt in (ir_receipt,arm_receipt):
                    require(receipt['bounded_local_service'] is True and receipt['leader_reaped'] is True and receipt['cleanup_complete'] is True,
                        'Original evidence lifecycle did not close normally')
                stderr_path = IR_ROOT / 'ir-command/scored-source-ir.stderr';bind(stderr_path,report['pins'][str(stderr_path)])
                stderr = stderr_path.read_text()
                require('Building '+str(Path(row['source']).parent)+' -> '+str(Path(row['source']).parent)+'\n' in stderr, 'Actual original authored package not printed')
                writes = [line[len('Writing '):] for line in stderr.splitlines() if line.startswith('Writing ')]
                ir_files = sorted((IR_ROOT / 'ir').glob('*.ll'))
                require(len(ir_files) == 16 and set(writes) == {str(path) for path in ir_files} and
                    len(writes) == 17 and writes.count(str(IR_ROOT / 'ir' / (COMPACT+'.main.ll'))) == 2, 'Original complete duplicate-write inventory differs')
                modules = {}
                for path in ir_files:
                    require(not path.is_symlink(), 'Original emitted module unexpectedly aliases another file')
                    digest = bind(path,report['pins'][str(path)])
                    lines = path.read_text().splitlines()
                    modules[str(path)] = dict(sha256=digest,module_headers=lines[:5],
                        defined_function_headers=[line for line in lines if line.startswith('define ')],
                        actual_recorded_write_occurrences=writes.count(str(path)),owner_selection_admitted=False)
                main = IR_ROOT / 'ir' / (COMPACT+'.main.ll')
                require(modules[str(main)]['module_headers'][:2] == ["; ModuleID = 'compact.builtin'",'source_filename = "compact.builtin"'], 'Original overwritten main witness differs')
                disassembler_sha = bind(LLVM_DIS)
                interpreter_sha = bind(sys.executable)
                proposal = dict(status='original-six-traces-and-partial-IR-retained-awaiting-independent-owner-review',
                    capture_interpreter=dict(path=str(Path(sys.executable).resolve()),sha256=interpreter_sha,current_observation_only=True),
                    failed_report=dict(path=str(FAILED / 'report.json'),sha256=args.failed_report_sha256,status_unchanged='stopped'),
                    original_trace_records=report['attempts'],original_tree_observations=tree_attempts,
                    original_compact_source=row['source'],original_compact_source_sha256=row['source_sha256'],
                    original_production_compile_command=production['command'],original_successful_IR_command=ir_receipt,
                    original_successful_scored_ARM_command=arm_receipt,exact_original_write_sequence=writes,complete_original_IR_modules=modules,
                    surviving_main_header_is_builtin=True,automatic_IR_selection_performed=False,
                    proposed_recovery_pass=['--pass','bitcode'],proposed_disassembler=dict(path=str(LLVM_DIS),sha256=disassembler_sha,
                        status='current-reader-image-pinned-awaiting-owned-version-and-loaded-image-qualification',
                        original_compiler_backend_version='23.1.2',retrospective_reader_pin_claimed=False),
                    compiler_sources=['https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genobj.c',
                        'https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/gensplit.cc',
                        'https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genexe.cc',
                        'https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/pass/pass.c'],
                    root_semantic_review_required=True,source_or_timing_mutations=0,tree_observation_reexecutions=0,
                    actual_native_commands=0,actual_fixture_reexecutions=0,chart_admission=False,pins=dict(pins))
                save(output / 'ownership-proposals.json',proposal)
                outcome['status'] = 'captured-original-evidence-proposals-awaiting-root-owner-review'
            except BaseException as error:
                outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                try:(output / 'error.log').write_text(traceback.format_exc())
                except BaseException as journal_error:outcome['error_log_write_failure'] = str(journal_error)
                raise
            finally:
                errors,available = [],{}
                for path,value in list(pins.items()):
                    try:bind(path,value)
                    except BaseException as error:errors.append(dict(path=path,error=str(error)))
                for path in output.rglob('*'):
                    if path.is_file():
                        try:available[str(path.resolve())] = sha(path)
                        except BaseException as error:errors.append(dict(path=str(path),error=str(error)))
                if errors:outcome.update(status='failed-terminal-integrity',integrity_errors=errors,available_bytes_not_fully_pinned=True)
                outcome.update(pins=pins,available_evidence_pins=available,common_mutex_held=True,common_timing_mutex=str(LOCK))
                save(output / 'outcome.json',outcome)
                if errors:raise RuntimeError('Original evidence capture terminal pin failure')


if __name__ == '__main__':main()
