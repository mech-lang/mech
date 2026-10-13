"""Enter the source-reviewed C3 metadata qualifier with exact actual inputs.

The frozen plan is written only after actual reviewed LOC closure. This entry
starts no fixture, parser, compiler, benchmark, native probe or signal, and never
edits Site data. The unchanged metadata qualifier owns its own second guarded
phase after same-PID exec, under the continuous private durable owner lease.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
ROOT = CLBG.parent.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
PACKET = HERE / 'publication-c3-metadata-v1'
PLAN = PACKET / 'actual-plan-v1.json'
QUALIFIER = PACKET / 'qualify_descriptor.py'
DESCRIPTOR = PACKET / 'descriptor.json'
IMPORTER = CLBG / 'overview/explorer/native_languages_data_v1.py'
C3 = CLBG / 'c3-comparison/20261012'
SUITE = C3 / 'reconciliation/full-v1-suite-v2.json'
INDEX = C3 / 'studies/full-v1/index.json'
OVERLAY = C3 / 'source-counts/canonical-loc-overlay-v2/qualified-loc-overlay.json'
SEAL_OUTCOME = C3 / 'source-counts/overlay-seal-outcomes/canonical-loc-overlay-v2/outcome.json'
DEPENDENCY = HERE / 'owned-jobs/c3-reviewed-loc-seal-v2'
OUT = HERE / 'jobs/c3-reviewed-metadata-entry-v1'
PYTHON = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12')


def require(value, why):
    if not value: raise ValueError(why)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--plan-sha256', required=True)
    args = parser.parse_args()
    require(len(args.plan_sha256) == 64 and all(c in '0123456789abcdef' for c in args.plan_sha256), 'Invalid exact plan SHA')
    print('WAIT closed dependency lease: C3 exact reviewed LOC seal', flush=True)
    with (DEPENDENCY / 'owner.lock').open('a+') as dependency:
        fcntl.flock(dependency, fcntl.LOCK_EX)
        held, named = os.fstat(dependency.fileno()), (DEPENDENCY / 'owner.lock').stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different dependency lease')
        require((DEPENDENCY / 'terminal.json').is_file(), 'Sealer ended without durable terminal')
        print('WAIT common mutex: exact C3 metadata handoff', flush=True)
        with LOCK.open('a+') as common:
            fcntl.flock(common, fcntl.LOCK_EX)
            held, named = os.fstat(common.fileno()), LOCK.stat()
            require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different common mutex')
            OUT.mkdir(parents=True, exist_ok=False)
            pins, argv = {}, None
            def bind(path, expected=None):
                path = Path(path).resolve(strict=True)
                digest = sha(path)
                require(expected is None or digest == expected, 'Changed pinned actual input: ' + str(path))
                require(str(path) not in pins or pins[str(path)] == digest, 'Conflicting actual input pin')
                pins[str(path)] = digest
                return path, digest
            def read(path, expected=None):
                actual, digest = bind(path, expected)
                return json.loads(actual.read_text()), digest
            try:
                plan, _ = read(PLAN, args.plan_sha256)
                require(plan['status'] == 'source-reviewed-metadata-plan-with-actual-reviewed-LOC' and
                    plan['language'] == 'c3' and plan['chart_admission'] is False and
                    plan['native_commands'] == plan['fixture_reexecutions'] == 0 and plan['unresolved'] == [], 'Unqualified actual plan')
                for path, digest in plan['pins'].items(): bind(path, digest)
                terminal, _ = read(DEPENDENCY / 'terminal.json', plan['pins'][str(DEPENDENCY / 'terminal.json')])
                require(terminal['status'] == 'command-exited-zero-awaiting-report-review' and
                    type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
                    terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and
                    terminal['stop_signals'] == [] and not any(terminal.get(k) for k in
                    ('error','error_type','exception_cleanup','error_log_write_failure')), 'Sealer durable outcome not normal')
                request, _ = read(DEPENDENCY / 'request.json', plan['pins'][str(DEPENDENCY / 'request.json')])
                started, _ = read(DEPENDENCY / 'command-started.json', plan['pins'][str(DEPENDENCY / 'command-started.json')])
                require(request['command'] == started['command'] and request['cwd'] == started['cwd'] == str(ROOT) and
                    started['command_pid'] == terminal['command_pid'] and started['command_identity'] == terminal['command_identity'] and
                    started['command_identity_observation_missing'] is False and request['common_mutex_owned_by_target_command'] is True,
                    'Changed original sealer command identity')
                final, _ = read(SEAL_OUTCOME, plan['pins'][str(SEAL_OUTCOME)])
                require(final['status'] == 'completed-overlay-stage' and final['body_error'] is final['integrity_error'] is None and
                    final['available_bytes_not_fully_pinned'] is False and final['actual_common_mutex_held'] is True and
                    final['common_timing_mutex'] == str(LOCK) and final['actual_native_subjects'] == final['actual_signals_sent'] == 0 and
                    final['original_sources_or_timings_mutated'] is final['chart_admission'] is False, 'Unqualified actual LOC seal closure')
                for path, digest in final['pins'].items(): bind(path, digest)
                overlay, _ = read(OVERLAY, plan['pins'][str(OVERLAY)])
                require(final['pins'][str(OVERLAY)] == plan['pins'][str(OVERLAY)] and overlay['status'] == 'qualified-exact-source-LOC' and
                    overlay['language'] == 'c3' and type(overlay['profile_count']) is int and overlay['profile_count'] == 20 and
                    overlay['complete_authored_source_bundles'] is overlay['canonical_LOC_admitted'] is True and overlay['unresolved'] == [] and
                    overlay['chart_admission'] is overlay['original_sources_or_timings_mutated'] is False, 'Actual qualified20 overlay missing')
                for path in (PLAN, QUALIFIER, DESCRIPTOR, PACKET / 'REVIEW.md', Path(__file__).resolve()):
                    require(not path.stat().st_mode & 0o222, 'Actual plan or reviewed metadata source not frozen')
                    bind(path)
                for path in (QUALIFIER, DESCRIPTOR, IMPORTER, SUITE, INDEX, PYTHON.resolve(strict=True)):
                    bind(path, plan['pins'][str(path)])
                require(not (PACKET / 'qualified/actual-metadata-v1').exists(), 'Never retry or overwrite metadata output')
                argv = [str(PYTHON),'-E','-S','-B','-u',str(QUALIFIER),'--revision','actual-metadata-v1',
                    '--qualifier-sha256',pins[str(QUALIFIER)],'--descriptor-sha256',pins[str(DESCRIPTOR)],
                    '--suite',str(SUITE),'--suite-sha256',pins[str(SUITE)],'--canonical-loc',str(OVERLAY),
                    '--canonical-loc-sha256',pins[str(OVERLAY)],'--full-index',str(INDEX),'--full-index-sha256',pins[str(INDEX)],
                    '--importer-sha256',pins[str(IMPORTER)]]
                require(argv == plan['command'], 'Changed exact metadata qualification arguments')
                save(OUT / 'handoff.json',dict(status='reviewed-actual-LOC-entering-unchanged-metadata-qualifier',
                    command=argv,plan_sha256=args.plan_sha256,pins=pins,common_mutex_held=True,native_commands=0,
                    fixture_reexecutions=0,source_or_timing_mutations=0,chart_admission=False,automatic_retry=False))
            except BaseException as error:
                (OUT / 'error.log').write_text(traceback.format_exc())
                save(OUT / 'failed-handoff.json',dict(status='failed-exact-metadata-precondition',error=str(error) or type(error).__name__,
                    error_type=type(error).__name__,available_input_pins=pins,common_mutex_held=True,automatic_retry=False,chart_admission=False))
                raise
    os.execv(str(PYTHON), argv)


if __name__ == '__main__': main()
