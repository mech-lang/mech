"""Reuse actual Nim20 formatting with required-receipt JSON loading only."""
import argparse
from contextlib import ExitStack
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
C = CLBG / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
FMT = CLBG / 'loc-normalization/20261013/formatted-loc-v1'
FQ = FMT / 'jobs/nim-native20-v1/qualification.json'
FQ_SHA = '4e6dd5fe9cc4eb33f8ff6205d8812f196220987406f3e01013f76c99af8ebaaf'
FT_SHA = 'c96d429a6afec57a335ba6f359b1045d9c349209dce4a981691a53bad34dc609'
FO_SHA = 'a41fde25a20e42e147c762b12d8cf90c63d767e408bce0dbc9aa10681c4dd0b2'
FORMAT_OWNER = C / 'owned-jobs/nim-native-formatted20-v1'
FORMAT_REVIEW = C / 'jobs/nim-native-formatted20-closed-review-v1/receipt.json'
FORMAT_REVIEW_SHA = 'f5797f675386522de8c66828d435c48ec919495145f483e0886be37a571bbfe6'
FORMAT_PLAN = C / 'formatted-nim-plan-v1/jobs/actual-plans-v1/cohort-plan.json'
FORMAT_PLAN_SHA = 'dddc2ee8b22bac0081351633ce89e23c0d88a404b2cd01b81ee6d778cf8b281b'
BASE = C / 'combined-chart-integration-v3/jobs/current-nim-format-platform-v1/candidate-data.json'
BASE_SHA = '8b85d35f9ff8c267171c9d91f3467ba765bf019ac438c64ed2071898a7656378'
GLEAM = C / 'jobs/gleam-current-candidate-v3'
CURRENT = GLEAM / 'candidate-data.json'
CURRENT_SHA = '76ab76e215082279eff8f1b5c132b4d6c875da6a8be6b4288a2207afd9ba3f9d'
OUTCOME_SHA = '18b37de334530c5dbe7a74d9c0c1032647dfcb24362841e68ad6525c3a5d6311'
GLEAM_OWNER = C / 'owned-jobs/gleam-current-candidate-v3'
GO_SHA = 'bdac370fbcfb72b174da5c91aa8dbb32daa420734e019abb8bdacb0a6103e36d'
GR_SHA = '1387e06aef15ccb5e5f5bad81682904e5139e809e0c845ab6c1cc9235b9f10c8'
REUSED_SHA = '5d32a51b57b899987dd6e9f2bab9f8e8be6b22a94984a38ed8ced8977b9adb6e'
GLEAM_REVIEW = C / 'jobs/gleam-current-candidate-v3-closed-review-v1/receipt.json'
GLEAM_REVIEW_SHA = 'b15f1db2189ccba429146bc6ff5867c9c9efc03144dcdaccdf001b54b2da90a6'
GLEAM_WORKER = C / 'qualify_gleam_current_candidate_v3.py'
GLEAM_WORKER_SHA = '4b8329ba554aa491ba5e895ed8c525fa7e8a99ee5b2c67f74830aebd9d2d27a2'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
HOST_LINKAGE = C / 'jobs/python-framework-host-linkage-v1/host-linkage.json'
HOST_LINKAGE_SHA = '9a1c0fa4fc330cf0c7c176349c220640ad4714c5e8e081eaead84c00b89729af'
LAUNCHER = '/opt/homebrew/opt/python@3.14/bin/python3.14'
LAUNCHER_IMAGE = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/bin/python3.14'
FRAMEWORK = '/opt/homebrew/Cellar/python@3.14/3.14.8/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python'
OLD = C / 'nim-formatted-chart-integration-v1'
OLD_OUTPUT = OLD / 'jobs/gleam-nim-native-formatted-v1'
FAILED_OWNER = C / 'owned-jobs/gleam-nim-native-formatted-chart-v1'
FAILED_REVIEW = C / 'jobs/nim-formatted-integration-v1-closed-review/receipt.json'
FAILED_REVIEW_SHA = '8b5d82ede825b5e2835800c5ca7c46963c7f3109630e0ffbc04b5f5b6d3b06d4'
FAILED_BODY_SHA = 'aee7563e900617560e1884f137a6c573119e3d6eda5c6f2d6b316d7fcf18007f'
FAILED_OWNER_SHA = 'd5e6ea2fffe2018220897dcfc146973cc24a9d35bdb8e47beb27ee0cd0a102b6'
FAILED_REQUEST_SHA = '8f4d64feb338f2055f8f8ad527d98f200f15db2cbba8bf6bcd0e569c7721e9ec'
OLD_WORKER_SHA = 'db25e78012a030a7b176cddb09bbbf974dcff1ccf0d615a1df8c606e10e7728b'
OLD_REVIEW_SHA = '35f7aee42b8dd16a65cbb5f6f5f8d3393d910eb8b5a947adc80f55d61ed0b7d6'
MODULES = {'overlay': (FMT / 'overlay.py', 'ddc49cb21ad03dda6dee697ce1511617239364b0d5f2c23b9a5f9e637d14d084'),
    'endpoints': (FMT / 'endpoints.py', '96724c3ff701fabf7ea262520cacd8951b15948500cfe934efb21a1a129474bf')}
FIELDS = ('formatted_loc', 'formatted_loc_status', 'formatted_loc_complete', 'formatted_loc_evidence', 'formatted_x')
DEFAULTS = (None, 'pending', False, {}, None)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert re.fullmatch('[a-f0-9]{64}', digest) and sha(path) == digest, str(path)
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def absorb(values, pins):
    for path, digest in values.items():
        assert Path(path).is_absolute() and '..' not in Path(path).parts
        bind(path, digest, pins)


def new(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def cold(name, path, digest, pins):
    assert name not in sys.modules
    raw = bind(path, digest, pins).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == digest
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path.resolve() and sys.modules[name] is module
    return module


def closed(owner, terminal_sha, command, pids, pins, request_sha=None, expected_exit=0):
    ended = read(owner / 'terminal.json', terminal_sha, pins)
    request = read(owner / 'request.json', request_sha or sha(owner / 'request.json'), pins)
    launch, worker, started = [read(owner / name, sha(owner / name), pins) for name in
        ('launch.json', 'worker-started.json', 'command-started.json')]
    assert all(item['command'] == command and item['cwd'] == str(CLBG.parents[1]) for item in (request, launch, worker, started))
    assert type(expected_exit) is int and expected_exit in (0, 1)
    assert ended['status'] == ('command-exited-zero-awaiting-report-review' if expected_exit == 0 else 'command-failed')
    assert type(ended['exit_code']) is int and ended['exit_code'] == expected_exit
    assert ended['actual_command_started'] is True and ended['stop_signals'] == [] and not ended.get('error')
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert ended['automatic_retry'] is ended['qualification_granted'] is request['automatic_retry'] is request['qualification_granted'] is False
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert worker['worker_identity'] == ended['worker_identity'] and started['command_identity'] == ended['command_identity']
    assert ended['command_identity_observation_missing'] is started['command_identity_observation_missing'] is False
    worker_pid, command_pid = pids
    assert worker['worker_identity']['pid'] == launch['worker_identity']['pid'] == worker_pid
    initial = launch['worker_identity']['listing'].split(None, 7)
    final = worker['worker_identity']['listing'].split(None, 7)
    assert initial[0] == final[0] == str(worker_pid) and initial[2:7] == final[2:7]
    initial_argv, final_argv = initial[7].split(), final[7].split()
    remaining = ['-B', '-u', str(C / 'launch_durable_v1.py'), '--worker', str(owner), '--lease-fd', '3', '--go-fd', '4']
    assert initial_argv[1:] == final_argv[1:] == remaining
    # Only these two actual saved representations are admitted. The startup
    # launcher and Framework images have distinct independently recorded bytes.
    assert (initial_argv[0], final_argv[0]) == ((LAUNCHER, FRAMEWORK) if owner == GLEAM_OWNER else (FRAMEWORK, FRAMEWORK))
    linkage = read(HOST_LINKAGE, HOST_LINKAGE_SHA, pins)
    assert linkage['status'] == 'root-reviewed-exact-python-framework-linkage' and linkage['python_version'] == '3.14.8'
    assert linkage['same_image_bytes_claimed'] is linkage['benchmark_runtime_image_changed'] is False
    assert linkage['official_startup_source'] == 'https://raw.githubusercontent.com/python/cpython/v3.14.8/Mac/Tools/pythonw.c'
    assert linkage['launcher'] == dict(path=LAUNCHER, resolved_path=LAUNCHER_IMAGE, sha256='e401ac7a95cda75dee34a0cfe1e4faf0a6df1a501cf67887f177fb78cd4eb60a')
    assert linkage['framework'] == dict(path=FRAMEWORK, sha256='b3e2d3162eb789101425bbe76810fc5eb0d412d7cd5867d27568dd180ccf5a22')
    assert Path(LAUNCHER).resolve(strict=True) == Path(LAUNCHER_IMAGE) and not Path(FRAMEWORK).is_symlink()
    bind(LAUNCHER, linkage['launcher']['sha256'], pins); bind(LAUNCHER_IMAGE, linkage['launcher']['sha256'], pins)
    bind(FRAMEWORK, linkage['framework']['sha256'], pins)
    assert started['command_pid'] == ended['command_pid'] == command_pid
    listing = ended['command_identity']['listing'].split(None, 7)
    assert listing[:2] == [str(command_pid), str(worker_pid)] and listing[7].split() == command


def previous_failure(pins):
    review = read(FAILED_REVIEW, FAILED_REVIEW_SHA, pins)
    assert review['status'] == 'closed-failed-Nim-formatted-integration-at-untyped-json-pin-loader' and review['verified_pins'] == 54112
    assert (review['body_terminal_sha256'], review['durable_terminal_sha256'], review['request_sha256']) == (FAILED_BODY_SHA, FAILED_OWNER_SHA, FAILED_REQUEST_SHA)
    assert review['worker_pid'] == 53887 and review['command_pid'] == 53893
    assert review['original_formatter_qualification_sha256'] == FQ_SHA
    assert review['candidate_written'] is review['chart_admission'] is False and review['common_mutex_held'] is True
    assert all(type(review[field]) is int and review[field] == 0 for field in
        ('native_processes', 'fixture_reexecutions', 'formatter_reexecutions', 'benchmark_runs', 'live_dataset_mutations'))
    command = [PYTHON, '-I', '-S', '-B', '-u', str(OLD / 'integrate.py'),
        '--worker-sha256', OLD_WORKER_SHA, '--review-sha256', OLD_REVIEW_SHA,
        '--formatter-review-sha256', FORMAT_REVIEW_SHA]
    closed(FAILED_OWNER, FAILED_OWNER_SHA, command, (53887, 53893), pins, FAILED_REQUEST_SHA, expected_exit=1)
    ended = read(OLD_OUTPUT / 'terminal.json', FAILED_BODY_SHA, pins)
    assert ended['status'] == 'failed-Nim-formatted-integration'
    assert ended['body_error'] == dict(error='Expecting value: line 1 column 1 (char 0)', error_type='JSONDecodeError')
    assert ended['integrity_errors'] == [] and ended['common_mutex_held'] is True and ended['chart_admission'] is False
    assert all(type(ended[field]) is int and ended[field] == 0 for field in
        ('actual_native_processes', 'fixture_reexecutions', 'live_dataset_mutations'))
    assert not (OLD_OUTPUT / 'candidate-data.json').exists() and not (OLD_OUTPUT / 'receipt.json').exists()
    assert ended['pins'][str(OLD / 'integrate.py')] == OLD_WORKER_SHA and ended['pins'][str(OLD / 'REVIEW.md')] == OLD_REVIEW_SHA
    assert ended['pins'][str(FMT / 'load_evidence.py')] == 'c473d48ee8065983305617c4622a7e21cd31c461889794f5b4b55449b9202a47'
    absorb(ended['pins'], pins); absorb(ended['available_evidence_pins'], pins)
    error_path = OLD_OUTPUT / 'error.log'
    assert str(error_path) in ended['available_evidence_pins']
    error = error_path.read_text()
    assert 'load_evidence.py", line 53, in load' in error and 'JSONDecodeError: Expecting value: line 1 column 1 (char 0)' in error
    expected = [dict(path=str(CLBG / ('nim-comparison/20261012/.build/private-v' + revision + '/nim/nim-2.2.12/tests/testdata/' + name)),
        sha256=digest, error=message) for revision in ('2', '3', '4', '5') for name, digest, message in
        (('jsontest.json', '580fa042233efbc847c2cd5624fa728b422c526299f9e91101e6de97cffe8c24', 'Expecting value: line 1 column 1 (char 0)'),
         ('jsonwithextradata.json', '83bb007ad26363626b81565322ab4ee1840c4753e9477c15b6bc340f4f75e557', 'Extra data: line 6 column 1 (char 36)'))]
    assert review['unparseable_json_named_pins'] == expected
    return expected


def baseline(pins):
    review = read(GLEAM_REVIEW, GLEAM_REVIEW_SHA, pins)
    assert review['status'] == 'closed-qualified-Gleam-current-candidate-with-exact-owner-evidence' and review['verified_pins'] == 53558
    assert review['candidate'] == dict(path=str(CURRENT), sha256=CURRENT_SHA)
    assert (review['outcome_sha256'], review['durable_terminal_sha256'], review['request_sha256'], review['reuse_sha256']) == (OUTCOME_SHA, GO_SHA, GR_SHA, REUSED_SHA)
    assert review['worker_pid'] == 51512 and review['command_pid'] == 51516 and review['programs'] == 815 and review['inventory'] == 100
    assert review['Gleam_source_profiles'] == 20 and review['Gleam_counted_compiler_inputs'] == 104
    assert review['Gleam_full_processes_reused'] == 90 and review['Gleam_recorded_samples'] == 60
    assert review['original_programs_sources_and_inventory_unchanged'] is review['source_owner_slices_verified'] is review['common_mutex_held'] is True
    assert review['native_processes'] == review['fixture_reexecutions'] == review['benchmark_runs'] == 0 and review['chart_admission'] is False
    command = [PYTHON, '-I', '-S', '-B', '-u', str(GLEAM_WORKER), '--worker-sha256', GLEAM_WORKER_SHA,
        '--packet-sha256', '5cfcbdb90ce977089a225be4861edaf2b6aa6c52532ec23525970e7416bf25bd',
        '--metadata-sha256', '24c2fc111a79e9093fd2e92625bf552c1539d68ef0d53523b084eebf565b2625',
        '--review-sha256', '86ede86c969496cc8d315a06b191b9358398199eb08530516c79a8db85ca27db',
        '--metadata-review-sha256', '406ac95e0f93b99efc26d7bb62d0fe593568860c8d6cc3b8bfa39918ca1f9cdc',
        '--baseline-review-sha256', 'bb369ef1b1dab6579e788c220c148082aba3a185c89468572e0056e2711d647f',
        '--adapter-sha256', '8adfe2c8f3ddfc2f6db45bd0d3d5c592372015a4d56a3acccaaad55c9d4a127b',
        '--failure-review-sha256', '82cdd22cb2cee1d2f399cac1b4c57a41d8d37972cf6a2d4536c266a4e3ca03cf']
    closed(GLEAM_OWNER, GO_SHA, command, (51512, 51516), pins, GR_SHA)
    outcome = read(GLEAM / 'outcome.json', OUTCOME_SHA, pins)
    assert outcome['status'] == 'qualified-current-Gleam-candidate-awaiting-root-stage-review'
    assert outcome['common_mutex_held'] is True and outcome['common_timing_mutex'] == str(LOCK) and outcome['chart_admission'] is False
    assert all(type(outcome[key]) is int and outcome[key] == 0 for key in ('native_commands', 'fixture_reexecutions', 'live_dataset_mutations'))
    assert not any(outcome.get(key) for key in ('error', 'error_type', 'terminal_pin_errors', 'available_bytes_not_fully_pinned', 'error_log_write_failure'))
    assert outcome['pins'][str(BASE)] == BASE_SHA and outcome['available_evidence_pins'][str(CURRENT)] == CURRENT_SHA
    assert outcome['available_evidence_pins'][str(GLEAM / 'input-data.json')] == BASE_SHA
    assert outcome['available_evidence_pins'][str(GLEAM / 'reused-qualification.json')] == REUSED_SHA
    absorb(outcome['pins'], pins); absorb(outcome['available_evidence_pins'], pins)
    reused = read(GLEAM / 'reused-qualification.json', REUSED_SHA, pins)
    assert reused['status'] == 'actual-original-four-reused-for-Gleam-without-fixture-reexecution'
    assert reused['published_baseline'] == dict(path=str(BASE), sha256=BASE_SHA, immutable_new_input=str(GLEAM / 'input-data.json'))
    assert reused['retained_Gleam_source_inventory'] == 20 and reused['old_programs_files_inventory_baselines_groups_and_summaries_unchanged'] is True
    assert reused['original_fixture_methods_passed'] == 4 and reused['new_fixture_methods_executed'] == reused['live_dataset_mutations'] == 0
    assert reused['admission']['ready_languages'] == ['gleam'] and reused['admission']['actual_full_native_processes'] == 90
    before, current = read(BASE, BASE_SHA, pins), read(CURRENT, CURRENT_SHA, pins)
    assert len(current['programs']) == 815 and len(current['program_inventory']) == 100
    assert len([row for row in current['program_inventory'].values() if row['language'] == 'gleam']) == 20
    # Reuse only this already accepted pure preservation function, never run/main.
    cold('accepted_gleam_v3_pure_preservation', GLEAM_WORKER, GLEAM_WORKER_SHA, pins).preserve(before, current)
    return before, current


def formatting(args, loader, common_fd, pins):
    assert args.formatter_review_sha256 == FORMAT_REVIEW_SHA
    root = read(FORMAT_REVIEW, args.formatter_review_sha256, pins)
    assert root['status'] == 'closed-qualified-complete-Nim20-native-formatted-LOC' and root['verified_pins'] == 39561
    assert (root['qualification_sha256'], root['body_terminal_sha256'], root['durable_terminal_sha256']) == (FQ_SHA, FT_SHA, FO_SHA)
    assert root['profiles'] == 20 and root['authored_inputs'] == root['qualified_formatted_inputs'] == 21
    assert root['native_formatter'] == 'nimpretty0.2' and root['helper_formatted_loc'] == 18
    assert root['source_bytes_and_timings_preserved'] is True and root['actual_application_processes'] == root['fixture_reexecutions'] == 0 and root['chart_admission'] is False
    closed(FORMAT_OWNER, FO_SHA, [PYTHON, '-I', '-S', '-B', '-u', str(FMT / 'measure.py'), '--plan', str(FORMAT_PLAN),
        '--plan-sha256', FORMAT_PLAN_SHA, '--revision', 'nim-native20-v1'], (45348, 45354), pins)
    q = loader.load(FQ, FQ_SHA, FQ.parent / 'terminal.json', FT_SHA, common_fd)
    assert q['source_dataset_path'] == str(BASE) and q['source_dataset_sha256'] == BASE_SHA
    assert len(q['profile_keys']) == len(set(q['profile_keys'])) == 20
    assert all(row['language'] == 'nim' and row['status'] == 'qualified' and row['complete'] is True for row in q['rows'].values())
    assert sum(len(row['files']) for row in q['rows'].values()) == 21
    assert all(item['status'] == 'qualified' for row in q['rows'].values() for item in row['files'])
    assert root['formatted_counts'] == {key: row['formatted_loc'] for key, row in q['rows'].items()}
    assert q['rows']['pidigits:nim-pidigits-optimized']['formatted_loc'] == 79
    helpers = [item for row in q['rows'].values() for item in row['files'] if item['role'] == 'application-helper']
    assert len(helpers) == 1 and helpers[0]['path'].endswith('/support/gmp_ops.nim') and helpers[0]['physical_loc'] == 18
    assert helpers[0]['sha256'] == 'adad6363f378372c1f54b8382193a70c8e142565589bb2511ba6567838f8b3e4'
    absorb(q['pins'], pins); absorb(q['provenance'], pins)
    return q


def refresh(old, candidate, overlay, endpoints):
    # Retain the complete annotated pool BEFORE the selector prunes it. Only
    # its five overlay fields may be copied back onto the original full maps.
    annotated = deepcopy(overlay.catalog(candidate))
    selection = endpoints.finalize_endpoints(candidate)
    authoritative = deepcopy(overlay.catalog(candidate))
    for pool in ('programs', 'program_inventory'):
        restored = {}
        for key, prior in old[pool].items():
            source = authoritative.get(key, annotated[key])
            assert annotated[key].get('formatted_loc_status') != 'qualified' or key in authoritative
            row = deepcopy(prior)
            row.update({field: deepcopy(source.get(field, default)) for field, default in zip(FIELDS, DEFAULTS)})
            restored[key] = row
        candidate[pool] = restored
    candidate['files'] = deepcopy(old['files'])
    for before, after in zip(old['benchmarks'], candidate['benchmarks']):
        groups = []
        for original, chosen in zip(before['groups'], after['groups']):
            assert original['language'] == chosen['language']
            group = deepcopy(original)
            for field in ('formatted_fast', 'formatted_small'):
                key = chosen[field]
                assert key is None or key in original['programs'], 'Formatted endpoint would require an unreviewed canonical pool change'
                group[field] = key
            groups.append(group)
        base = after['baseline_formatted_loc']
        after.clear(); after.update(deepcopy(before)); after.update(groups=groups, baseline_formatted_loc=base)
    for before, after in zip(old['languages'], candidate['languages']):
        formatted = {field: deepcopy(after[field]) for field in ('formatted_timed_benchmarks', 'formatted_source_benchmarks')}
        after.clear(); after.update(deepcopy(before)); after.update(formatted)
    for field in ('reference_summary', 'interpreter_integration_audit'):
        if field in old: candidate[field] = deepcopy(old[field])
    return selection


def preserve(old, candidate, overlay, qualified):
    assert set(candidate['programs']) == set(old['programs']) and set(candidate['program_inventory']) == set(old['program_inventory'])
    assert overlay.exact_value(candidate['files'], old['files'])
    for pool in ('programs', 'program_inventory'):
        for key, row in candidate[pool].items():
            prior = old[pool][key]
            assert overlay.exact_value({k:v for k,v in row.items() if k not in FIELDS}, {k:v for k,v in prior.items() if k not in FIELDS})
            if key not in qualified:
                for field, default in zip(FIELDS[:4], DEFAULTS[:4]):
                    assert overlay.exact_value(row[field], prior.get(field, default))
            if row['formatted_loc_status'] != 'qualified':
                assert row['formatted_loc'] is row['formatted_x'] is None and row['formatted_loc_complete'] is False
    for before, after in zip(old['benchmarks'], candidate['benchmarks']):
        assert overlay.exact_value({k:v for k,v in before.items() if k not in ('groups', 'baseline_formatted_loc')},
            {k:v for k,v in after.items() if k not in ('groups', 'baseline_formatted_loc')})
        assert len(before['groups']) == len(after['groups'])
        for a, b in zip(before['groups'], after['groups']):
            assert overlay.exact_value({k:v for k,v in a.items() if k not in ('formatted_fast', 'formatted_small')},
                {k:v for k,v in b.items() if k not in ('formatted_fast', 'formatted_small')})
        pool = [row for row in overlay.catalog(candidate).values() if row['benchmark'] == before['key'] and row['formatted_loc_status'] == 'qualified']
        assert after['baseline_formatted_loc'] == min(row['formatted_loc'] for row in pool)
        assert all(row['formatted_x'] == row['formatted_loc'] / after['baseline_formatted_loc'] for row in pool)
    assert len(old['languages']) == len(candidate['languages'])
    for a, b in zip(old['languages'], candidate['languages']):
        assert overlay.exact_value({k:v for k,v in a.items() if k not in ('formatted_timed_benchmarks', 'formatted_source_benchmarks')},
            {k:v for k,v in b.items() if k not in ('formatted_timed_benchmarks', 'formatted_source_benchmarks')})
    special = {'programs', 'program_inventory', 'files', 'benchmarks', 'languages', 'provenance', 'formatted_reference_summary'}
    assert overlay.exact_value({k:v for k,v in old.items() if k not in special}, {k:v for k,v in candidate.items() if k not in special})
    assert all(candidate['provenance'][path] == digest for path, digest in old['provenance'].items())
    overlay.validate_exclusions(candidate)


def main():
    parser = argparse.ArgumentParser()
    for name in ('worker', 'review', 'loader', 'formatter-review'): parser.add_argument('--' + name + '-sha256', required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert all(re.fullmatch('[a-f0-9]{64}', value) for value in vars(args).values())
    def interrupted(signum, frame): raise KeyboardInterrupt('Nim formatted metadata interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP): signal.signal(signum, interrupted)
    print('WAIT common mutex: add existing Nim formatter evidence; no native execution', flush=True)
    with LOCK.open('r+') as common, ExitStack() as leases:
        fcntl.flock(common, fcntl.LOCK_EX)
        opened, named = os.fstat(common.fileno()), LOCK.stat()
        assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino) and opened.st_ino == 79859134
        for owner in (GLEAM_OWNER, FORMAT_OWNER, FAILED_OWNER):
            lease = leases.enter_context((owner / 'owner.lock').open('rb'))
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            opened, named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
            assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        output = HERE / 'jobs/gleam-nim-native-formatted-v2'
        assert not output.exists() and not any(path.is_symlink() for path in (output, output.parent, HERE))
        output.mkdir(parents=True, exist_ok=False)
        pins, body = {}, None
        try:
            for path, digest in ((HERE / 'integrate.py', args.worker_sha256), (HERE / 'REVIEW.md', args.review_sha256),
                                 (HERE / 'load_required_evidence.py', args.loader_sha256)):
                assert not path.stat().st_mode & 0o222
                bind(path, digest, pins)
            failed_inputs = previous_failure(pins)
            base, old = baseline(pins)
            loader = cold('required_formatter_receipts_v2', HERE / 'load_required_evidence.py', args.loader_sha256, pins)
            overlay, endpoints = (cold(name, *MODULES[name], pins) for name in ('overlay', 'endpoints'))
            q = formatting(args, loader, common.fileno(), pins)
            required_receipts = loader.receipt_paths(q)
            assert set(required_receipts) == set(q['receipts'])
            assert all(q['pins'].get(item['path']) == item['sha256'] and item['path'] not in required_receipts for item in failed_inputs)
            nim = {key for key, row in overlay.catalog(old).items() if row['language'] == 'nim'}
            assert set(q['profile_keys']) == nim and len(nim) == 20 and nim <= set(old['programs'])
            candidate = deepcopy(old)
            applied = overlay.apply(candidate, dict(data=base, path=str(BASE), sha256=BASE_SHA), q)
            assert applied['updated_programs'] == applied['qualified_programs'] == 20
            selected = refresh(old, candidate, overlay, endpoints)
            for path in (HERE / 'integrate.py', HERE / 'REVIEW.md', HERE / 'load_required_evidence.py', GLEAM_REVIEW, FORMAT_REVIEW, FAILED_REVIEW):
                assert str(path) not in candidate['provenance'] or candidate['provenance'][str(path)] == pins[str(path)]
                candidate['provenance'][str(path)] = pins[str(path)]
            preserve(old, candidate, overlay, nim)
            assert len(candidate['programs']) == 815 and len(candidate['program_inventory']) == 100
            assert sum(row['formatted_loc_status'] == 'qualified' for row in candidate['program_inventory'].values()) == 40
            for path, digest in pins.items(): assert sha(path) == digest
            path = output / 'candidate-data.json'; new(path, candidate); bind(path, sha(path), pins)
            new(output / 'receipt.json', dict(status='qualified-additive-Nim20-native-formatted-candidate',
                candidate=dict(path=str(path), sha256=pins[str(path)]), input=dict(path=str(CURRENT), sha256=CURRENT_SHA),
                immutable_formatter_baseline=dict(path=str(BASE), sha256=BASE_SHA), formatted=applied, selection=selected,
                original_failed_integration=dict(path=str(OLD_OUTPUT / 'terminal.json'), sha256=FAILED_BODY_SHA),
                required_receipt_pins=required_receipts, upstream_nonreceipt_json_pins_preserved=failed_inputs,
                programs=815, inventory=100, Nim_formatted_programs=20, Crystal_formatted_programs=20,
                unchanged_canonical_groups_source_timings_coordinates_and_runtime_fields=True,
                full_catalog_preserved=True, actual_native_processes=0, fixture_reexecutions=0,
                live_dataset_mutations=0, chart_admission=False, common_mutex_held=True, pins=dict(pins)))
        except BaseException as error:
            body = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            try:
                with (output / 'error.log').open('x') as stream: stream.write(traceback.format_exc())
            except BaseException as secondary: body['error_log_write_failure'] = str(secondary)
            raise
        finally:
            available, errors = {}, []
            candidates = dict(pins)
            candidates.update({str(path): None for path in output.rglob('*') if path.is_file() and str(path) not in candidates})
            for path, expected in candidates.items():
                try:
                    available[path] = sha(path); assert expected is None or available[path] == expected
                except BaseException as error: errors.append(dict(path=path, error=str(error) or type(error).__name__))
            new(output / 'terminal.json', dict(status='failed-terminal-integrity' if errors else 'failed-Nim-formatted-integration' if body else 'completed-Nim-formatted-integration',
                body_error=body, integrity_errors=errors, pins=pins, available_evidence_pins=available,
                common_mutex_held=True, actual_native_processes=0, fixture_reexecutions=0,
                live_dataset_mutations=0, chart_admission=False))
            if errors: raise RuntimeError('Nim formatted metadata integrity failed; partial evidence retained')


if __name__ == '__main__': main()
