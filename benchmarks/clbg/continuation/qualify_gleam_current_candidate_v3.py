"""Reuse actual unchanged ready fixtures; add local Gleam candidate/inventory.

No fixture, benchmark, preparation, build, native probe, signal, Git, credential,
server, publication operation or live dataset write occurs. Root separately reviews
this complete exclusive candidate before any staging/publication.
"""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import traceback
import types

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
ROOT_DATA = CLBG / 'overview/explorer/dist/data.json'
PUBLISHED = HERE / 'combined-chart-integration-v3/jobs/current-nim-format-platform-v1/candidate-data.json'
BASELINE_SHA = '8b85d35f9ff8c267171c9d91f3467ba765bf019ac438c64ed2071898a7656378'
OLD_INPUT = HERE / 'jobs/current-chart-session-v2/input-data.json'
OLD_INPUT_SHA = '65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f'
FIXTURE_JOB = HERE / 'owned-jobs/crystal-current-chart4-fixture-and-candidate-v1'
METADATA_JOB = HERE / 'owned-jobs/gleam-publication-metadata-v1'
METADATA_SHA = '24c2fc111a79e9093fd2e92625bf552c1539d68ef0d53523b084eebf565b2625'
METADATA_OUTCOME_SHA = '452e3437a24a5fb6b9f1252d32af5872cbdd879bd2c05f240dda3ed6bb69687b'
METADATA_TERMINAL_SHA = '87098c03510b7830b4e7ba7dfddedf60b447e913dd51653a8465c7abd483511d'
RUNTIME_SHA = '7dc62848e329047e47f46b9df9278e90b4f59f6ac99212f85ba3ee8999371da1'
BASELINE_JOB = HERE / 'owned-jobs/combined-nim-format-platform-v3'
BASELINE_REVIEW = HERE / 'jobs/combined-integration-v3-closed-review-v1/receipt.json'
BASELINE_TERMINAL_SHA = 'e9dbd62906260ebf0cab416a9196b0d5e8c05b26a5bc159103d8be57b4a8af07'
BASELINE_BODY_TERMINAL_SHA = '30ca3d505a43f058ff61674333e379d82ea0219099ab9ad8dd21d6a87ceb723d'
BASELINE_RECEIPT_SHA = '085517e32d7547c36e99abf8320a096cbd3ba8b3b01a59d597c95f58272e9ce5'
PRIOR_NIM_BASELINE_SHA = '9d9eba51742dcf1afc76c0a45fa4125524197ed06ae540a1d142769fd859f60a'
FIXTURES = HERE / 'jobs/native-ready-v1'
RECEIPT_SHA = 'bce66a4cb1fc02cc827c6906f9ddc5be4b3abddb8851e2028415600972ec29b8'
TERMINAL_SHA = '0b3a1cce3a5c2a41d27b88e2f3af8145d014c9ed3c1a2b5959c5c1613afeba59'
PACKET = HERE / 'gleam-ready-publication-v1.json'
INVENTORY_WORKER = HERE / 'qualify_native_ready_current_v2.py'
INVENTORY_WORKER_SHA = '4118db4115c2e37af1533495e24f45423e66194c6fc7fa87d04d60e6e821a7b8'
OUT = HERE / 'jobs/gleam-current-candidate-v3'
NOTES = HERE / 'GLEAM_CURRENT_CANDIDATE_V3_REVIEW.md'
RECORD_ADAPTER = HERE / 'gleam_loc_record_adapter_v1.py'
MEANING = CLBG / 'gleam-comparison/20261012/source-counts/semantic-exact20-v2/source-meaning-resolution.json'
MEANING_SHA = 'c4b65664696b26539d4ce1f32e7494c80c2e706786ffff482df18116aca72505'
FAILED_JOB = HERE / 'owned-jobs/gleam-current-candidate-v2'
FAILED_REVIEW = HERE / 'jobs/gleam-current-candidate-v2-closed-review/receipt.json'
FAILED_OUTCOME_SHA = '8f64f4ad1df17adca809cc5cc5b15af3f17e8ed53141807657196f373831e4e4'
FAILED_TERMINAL_SHA = '273daef2538bb7bea4ccd3bdd104068d6f3d9ea3b4a190980fa8ab37299f5d08'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
FILES = (
    'native_languages_ready_v1.py', 'test_native_languages_ready_v1.py',
    'native_languages_data_v2.py', 'native_platform_metadata_v2.py',
    'native_language_groups_v2.py', 'publication_endpoints_v1.py',
    'test_native_languages_data_v1.py', 'native_languages_data_v1.py')
IDS = tuple('test_native_languages_ready_v1.NativeReadyAdmission.' + name for name in (
    'test_crystal_only_keeps_unchanged_twenty_source_ninety_process_validation',
    'test_partial_unknown_eighty_nine_runtime_and_unqualified_loc_reject',
    'test_pure_subset_platform_and_groups_preserve_prior_cohort_and_controls',
    'test_guarded_late_failures_are_atomic_and_matching_admission_is_idempotent'))
def expected_packet(metadata_sha256):
    return dict(status='reviewed-completed-native-language-subset', ready_languages=['gleam'],
        future_requested_languages=['elixir','kotlin'], unresolved=[], languages={'gleam': {
            'suite': dict(path=str(CLBG / 'gleam-comparison/20261012/reconciliation/full-v4-suite-v2.json'),
                sha256='e2b4a772153c9997d4ab5f749dfa950cbcf4ebaad825201b3c6338454f4e2594'),
            'canonical_loc': dict(path=str(CLBG / 'gleam-comparison/20261012/source-counts/semantic-exact20-v2/qualified-loc-overlay.json'),
                sha256='dd51c9a33d0eade6b5f4f94266ad1561b0f36770ac03a9c1691d2f19cbc04fce'),
            'metadata': dict(path=str(HERE / 'publication-gleam-metadata-v1/qualified/actual-metadata-v1/metadata.json'),
                sha256=metadata_sha256)}})


def require(value, why):
    if not value: raise ValueError(why)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def closed_durable(job, read, terminal_sha256=None):
    terminal = read(job / 'terminal.json',terminal_sha256)
    require(terminal['status'] == 'command-exited-zero-awaiting-report-review' and
        type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
        terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and
        terminal['stop_signals'] == [] and not any(terminal.get(k) for k in
        ('error','error_type','exception_cleanup','error_log_write_failure')), 'Original durable command not normal')
    request, started, worker = (read(job / name) for name in ('request.json','command-started.json','worker-started.json'))
    require(request['command'] == started['command'] == worker['command'] and
        request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parent.parent) and
        started['command_pid'] == terminal['command_pid'] and started['command_identity'] == terminal['command_identity'] and
        started['command_identity_observation_missing'] is False and request['common_mutex_owned_by_target_command'] is True and
        request['automatic_retry'] is False and worker['worker_identity'] == terminal['worker_identity'],
        'Changed original durable command identity')
    return terminal


def closed_baseline(args, read, bind):
    review = read(BASELINE_REVIEW,args.baseline_review_sha256)
    require(review['status'] == 'passed-closed-combined-format-platform-candidate-review' and review['verified_pins'] == 46991 and
        review['candidate'] == dict(path=str(PUBLISHED),sha256=BASELINE_SHA) and
        review['receipt_sha256'] == BASELINE_RECEIPT_SHA and review['body_terminal_sha256'] == BASELINE_BODY_TERMINAL_SHA and
        review['durable_terminal_sha256'] == BASELINE_TERMINAL_SHA and review['programs'] == 798 and review['inventory'] == 80 and
        review['qualified_formatted_crystal_programs'] == 20 and review['unqualified_formatted_coordinates_blank'] is True and
        review['prior_programs_sources_timings_and_canonical_coordinates_preserved'] is True and
        review['canonical_baselines_preserved'] is True and review['legacy_annotation_rows'] == 133 and
        review['native_processes'] == review['fixture_reexecutions'] == 0 and review['chart_admission'] is False,
        'Actual combined formatted/platform baseline closure differs')
    with (BASELINE_JOB / 'owner.lock').open('r') as lease:
        fcntl.flock(lease,fcntl.LOCK_SH | fcntl.LOCK_NB)
        held,named = os.fstat(lease.fileno()),Path(lease.name).stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different closed combined baseline lease')
        terminal = closed_durable(BASELINE_JOB,read,BASELINE_TERMINAL_SHA)
        command = [PYTHON,'-I','-S','-B','-u',str(HERE / 'combined-chart-integration-v3/integrate.py'),
            '--plan',str(HERE / 'combined-chart-integration-v3/plan-v1.json'),
            '--plan-sha256','46b5665e811a57766476385a73f2bdf1fa0e602bd70c34cd8c30d62b8caeda0c',
            '--self-sha256','890edeb5035ef1ff7473728ddf684651f019e1514edbdc054c93b6eb7f828923']
        observed = terminal['command_identity']['listing'].split(None,7)
        require(read(BASELINE_JOB / 'request.json')['command'] == command and
            observed[:2] == ['31954','31948'] and observed[7].split() == command and
            terminal['qualification_granted'] is terminal['automatic_retry'] is False, 'Different completed combined baseline command')
        body = read(PUBLISHED.parent / 'terminal.json',BASELINE_BODY_TERMINAL_SHA)
        require(body['status'] == 'completed' and body['body_error'] is None and body['integrity_errors'] == [] and
            body['available_bytes_not_fully_pinned'] is False and body['common_mutex_held'] is True and
            body['actual_native_processes'] == body['fixture_reexecutions'] == 0 and body['chart_admission'] is False,
            'Combined baseline body terminal not clean')
        require(body['pins'][str(PUBLISHED)] == BASELINE_SHA and
            body['pins'][str(PUBLISHED.parent / 'receipt.json')] == BASELINE_RECEIPT_SHA,
            'Combined baseline actual artifact pins differ')
        for path,digest in body['pins'].items(): bind(path,digest)
        receipt = read(PUBLISHED.parent / 'receipt.json',BASELINE_RECEIPT_SHA)
        require(receipt['status'] == 'qualified-combined-metadata-candidate' and
            receipt['candidate'] == dict(path=str(PUBLISHED),sha256=BASELINE_SHA) and
            receipt['input'] == dict(path=str(HERE / 'jobs/nim-current-candidate-v1/candidate-data.json'),sha256=PRIOR_NIM_BASELINE_SHA) and
            receipt['formatted'] == dict(updated_programs=20,qualified_programs=20) and receipt['legacy_programs'] == 133 and
            receipt['actual_Clojure_and_Nim_baseline_closures_verified'] is True and receipt['sources_timings_coordinates_baselines_preserved'] is True and
            receipt['actual_native_processes'] == receipt['fixture_reexecutions'] == 0 and receipt['dataset_mutated'] is False and
            receipt['live_files_written'] is False and receipt['chart_admission'] is False and receipt['common_mutex_held'] is True,
            'Actual combined baseline receipt differs')
        data = read(PUBLISHED,BASELINE_SHA)
        require(len(data['programs']) == 798 and len(data['program_inventory']) == 80,
            'Actual combined inventory dimensions changed')
        crystal = [row for row in data['program_inventory'].values() if row['language'] == 'crystal']
        require(len(crystal) == 20 and all(row['formatted_loc_status'] == 'qualified' and row['formatted_loc_complete'] is True and
            type(row['formatted_loc']) is int and row['formatted_loc'] > 0 and row['formatted_x'] > 0 for row in crystal),
            'Actual complete Crystal formatted inventory differs')
        return data



def retained_schema_failure(args, read, bind):
    review = read(FAILED_REVIEW,args.failure_review_sha256)
    require(review['status'] == 'closed-failed-candidate-at-Gleam-LOC-schema-mapping' and
        review['verified_pins'] == 53538 and review['outcome_sha256'] == FAILED_OUTCOME_SHA and
        review['durable_terminal_sha256'] == FAILED_TERMINAL_SHA and
        review['native_processes'] == review['fixture_reexecutions'] == review['live_dataset_mutations'] == 0 and
        review['candidate_written'] is False and review['error'] == "KeyError: 'expansions' in original ready program_record",
        'Different retained failed Gleam schema closure')
    with (FAILED_JOB / 'owner.lock').open('r') as lease:
        fcntl.flock(lease,fcntl.LOCK_SH | fcntl.LOCK_NB)
        held,named = os.fstat(lease.fileno()),Path(lease.name).stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different closed failed candidate lease')
        terminal = read(FAILED_JOB / 'terminal.json',FAILED_TERMINAL_SHA)
        request,started,worker = (read(FAILED_JOB / leaf) for leaf in ('request.json','command-started.json','worker-started.json'))
        command = [PYTHON,'-I','-S','-B','-u',str(HERE / 'qualify_gleam_current_candidate_v2.py'),
            '--worker-sha256','1ea421879d7088889832a5d8d41afb74eabe65ee0a287f53f938ddbe87d1fe22',
            '--packet-sha256','5cfcbdb90ce977089a225be4861edaf2b6aa6c52532ec23525970e7416bf25bd',
            '--metadata-sha256',METADATA_SHA,'--review-sha256','b2d54d0b1d0555b7eb5ee2dbd097061d5673b208f110e6e5fe1909345a79bc87',
            '--metadata-review-sha256','406ac95e0f93b99efc26d7bb62d0fe593568860c8d6cc3b8bfa39918ca1f9cdc',
            '--baseline-review-sha256','bb369ef1b1dab6579e788c220c148082aba3a185c89468572e0056e2711d647f']
        observed = terminal['command_identity']['listing'].split(None,7)
        require(terminal['status'] == 'command-failed' and type(terminal['exit_code']) is int and terminal['exit_code'] == 1 and
            terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and
            terminal['stop_signals'] == [] and terminal['qualification_granted'] is terminal['automatic_retry'] is False and
            request['command'] == started['command'] == worker['command'] == command and
            request['cwd'] == started['cwd'] == worker['cwd'] == str(CLBG.parent.parent) and
            request['common_mutex_owned_by_target_command'] is True and request['automatic_retry'] is False and
            started['command_pid'] == terminal['command_pid'] == 42355 and
            started['command_identity'] == terminal['command_identity'] and started['command_identity_observation_missing'] is False and
            worker['worker_identity'] == terminal['worker_identity'] and observed[:2] == ['42355','42351'] and
            observed[7].split() == command, 'Changed failed candidate process/command closure')
        outcome = read(HERE / 'jobs/gleam-current-candidate-v2/outcome.json',FAILED_OUTCOME_SHA)
        require(outcome['status'] == 'failed-current-Gleam-candidate' and outcome['error'] == "'expansions'" and
            outcome['error_type'] == 'KeyError' and outcome['chart_admission'] is False and
            outcome['native_commands'] == outcome['fixture_reexecutions'] == outcome['live_dataset_mutations'] == 0 and
            not any(outcome.get(k) for k in ('terminal_pin_errors','available_bytes_not_fully_pinned','error_log_write_failure')) and
            not (HERE / 'jobs/gleam-current-candidate-v2/candidate-data.json').exists(), 'Failed candidate history changed')
        for path,digest in outcome['available_evidence_pins'].items(): bind(path,digest)
        bind(HERE / 'qualify_gleam_current_candidate_v2.py',command[7])
        bind(HERE / 'GLEAM_CURRENT_CANDIDATE_V2_REVIEW.md',command[13])


def preserve(before, after):
    allowed = {'programs','files','provenance','languages','benchmarks','language_groups','protocol',
        'native_language_audit','native_ready_publication_audit','native_language_filter_facts','reference_summary','program_inventory'}
    for key, value in before.items():
        if key not in allowed: require(after[key] == value, 'Changed unrelated dataset field: ' + key)
    for field in ('programs','files','provenance','program_inventory'):
        for key, value in before.get(field, {}).items():
            require(after[field][key] == value, 'Changed prior ' + field + ': ' + key)
    languages = {row['key']: row for row in after['languages']}
    for row in before['languages']: require(languages[row['key']] == row, 'Changed prior language')
    for row in before.get('reference_summary', []):
        require(next(item for item in after['reference_summary'] if item['lang'] == row['lang']) == row, 'Changed prior summary')
    for old in before['benchmarks']:
        new = next(item for item in after['benchmarks'] if item['key'] == old['key'])
        require({key:value for key,value in old.items() if key != 'groups'} ==
            {key:value for key,value in new.items() if key != 'groups'}, 'Changed fixed benchmark baseline/fields')
        for group in old['groups']:
            require(next(item for item in new['groups'] if item['language'] == group['language']) == group, 'Changed prior endpoint group')
    for old in before['language_groups']:
        new = next(item for item in after['language_groups'] if item['key'] == old['key'])
        require({key:value for key,value in old.items() if key != 'languages'} ==
            {key:value for key,value in new.items() if key != 'languages'}, 'Changed prior trait group metadata')
        require(new['languages'][:len(old['languages'])] == old['languages'] and
            set(new['languages'][len(old['languages']):]) <= {'gleam'}, 'Changed prior trait memberships/order')
    for key, value in before['protocol'].items():
        if key != 'native_ready_languages': require(after['protocol'][key] == value, 'Changed prior protocol')
    for field in ('native_language_audit','native_ready_publication_audit'):
        for key, value in before.get(field, {}).items():
            if key in {'languages','packets'}:
                for identity, item in value.items(): require(after[field][key][identity] == item, 'Changed prior native audit')
            else: require(after[field][key] == value, 'Changed prior native audit metadata')
    for key, value in before.get('native_language_filter_facts', {}).items():
        require(after['native_language_filter_facts'][key] == value, 'Changed prior source trait fact')


def run(args):
    print('WAIT closed Gleam metadata lease', flush=True)
    with (METADATA_JOB / 'owner.lock').open('r+') as dependency:
        fcntl.flock(dependency, fcntl.LOCK_EX)
        held,named = os.fstat(dependency.fileno()), (METADATA_JOB / 'owner.lock').stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different metadata lease')
        with (FIXTURE_JOB / 'owner.lock').open('r+') as old_owner:
            fcntl.flock(old_owner, fcntl.LOCK_EX)
            held,named = os.fstat(old_owner.fileno()), (FIXTURE_JOB / 'owner.lock').stat()
            require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different original fixture lease')
            print('WAIT common mutex: source-only Gleam current candidate', flush=True)
            with LOCK.open('r+') as common:
                fcntl.flock(common, fcntl.LOCK_EX)
                held,named = os.fstat(common.fileno()), LOCK.stat()
                require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino) and held.st_ino == 79859134, 'Different common mutex')
                OUT.mkdir(parents=True, exist_ok=False)
                pins, historical, retained = {}, {}, None
                outcome = dict(status='failed-current-Gleam-candidate',chart_admission=False,native_commands=0,fixture_reexecutions=0)
                def bind(path, expected=None):
                    path = Path(path).resolve(strict=True); digest = sha(path)
                    require(expected is None or digest == expected, 'Changed actual input: ' + str(path))
                    require(str(path) not in pins or pins[str(path)] == digest, 'Conflicting actual pin')
                    pins[str(path)] = digest; return digest
                def read(path, expected=None):
                    bind(path, expected); return json.loads(Path(path).read_text())
                def cold_load(name, path, digest):
                    require(name not in sys.modules, 'Existing adapter/module identity: ' + name)
                    bind(path, digest); raw = Path(path).read_bytes()
                    require(hashlib.sha256(raw).hexdigest() == digest, 'Changed cold source bytes')
                    module = types.ModuleType(name); module.__file__ = str(path)
                    sys.modules[name] = module
                    exec(compile(raw, str(path), 'exec'), module.__dict__)
                    require(Path(module.__file__).resolve() == Path(path).resolve(), 'Wrong adapter path')
                    bind(path, digest); return module
                try:
                    for path in (Path(__file__).resolve(),PACKET,INVENTORY_WORKER,NOTES,RECORD_ADAPTER):
                        require(not path.stat().st_mode & 0o222, 'Reviewed frozen source/packet required')
                    bind(__file__, args.worker_sha256); bind(PACKET, args.packet_sha256);bind(NOTES,args.review_sha256)
                    bind(RECORD_ADAPTER,args.adapter_sha256)
                    retained_schema_failure(args,read,bind)
                    require(args.metadata_sha256 == METADATA_SHA and read(PACKET) == expected_packet(METADATA_SHA), 'Changed reviewed actual Gleam cohort descriptors')
                    bind(Path(sys.executable).resolve())
                    closed_durable(METADATA_JOB,read,METADATA_TERMINAL_SHA)
                    closed_durable(FIXTURE_JOB,read)
                    metadata_output = HERE / 'publication-gleam-metadata-v1/qualified/actual-metadata-v1'
                    metadata_outcome = read(metadata_output / 'outcome.json',METADATA_OUTCOME_SHA)
                    bind(metadata_output / 'metadata.json',METADATA_SHA)
                    require(metadata_outcome['status'] == 'metadata-qualified-awaiting-root-current-candidate-review' and
                        metadata_outcome['chart_admission'] is False and metadata_outcome['native_subjects'] == metadata_outcome['fixture_reexecutions'] == 0 and
                        not any(metadata_outcome.get(k) for k in ('error','error_type','terminal_error','terminal_error_type','integrity_errors','available_bytes_not_fully_pinned')),
                        'Original Gleam metadata qualifier outcome not normal')
                    for values in (metadata_outcome['prerequisite_pins'],metadata_outcome['available_evidence_pins']):
                        for path,digest in values.items(): bind(path,digest)
                    metadata_source = HERE / 'publication-gleam-metadata-v1'
                    expected_metadata_command = [PYTHON,'-I','-S','-B','-u',str(metadata_source / 'qualify_descriptor.py'),
                        '--revision','actual-metadata-v1','--qualifier-sha256','1503e6a1e6bb87732eae780a83403fb145f0dc59b5a614f8b9d18c3e3ab73f3c',
                        '--descriptor-sha256','ac19c1d896e556a683ece26c9ff8384dfe48c43c501ecd315b09365484ecb112',
                        '--review-sha256','e58445350df8dc342520105372dbf2710c6e9fd5d91c6ddb439a196b5ef30031',
                        '--root-review-sha256','db2e7999b951932089070f223d77649b0cdc9c1a579fdb5562102a206c303dec']
                    require(read(METADATA_JOB / 'request.json')['command'] == expected_metadata_command,
                        'Different actual Gleam metadata command/source arguments')
                    metadata_terminal = read(METADATA_JOB / 'terminal.json',METADATA_TERMINAL_SHA)
                    observed = metadata_terminal['command_identity']['listing'].split(None,7)
                    require(observed[:2] == ['34603','34599'] and observed[7].split() == expected_metadata_command,
                        'Different actual Gleam metadata target image/arguments')
                    require(pins[str(metadata_source / 'qualify_descriptor.py')] == expected_metadata_command[9] and
                        pins[str(metadata_source / 'descriptor.json')] == expected_metadata_command[11] and
                        pins[str(metadata_source / 'REVIEW.md')] == expected_metadata_command[13], 'Changed actual qualifier/descriptor/review bodies')
                    metadata_review = read(HERE / 'jobs/gleam-metadata-closed-review-v1/receipt.json',args.metadata_review_sha256)
                    require(metadata_review['status'] == 'closed-qualified-Gleam-exact20-publication-metadata' and metadata_review['verified_pins'] == 6907 and
                        metadata_review['metadata_sha256'] == METADATA_SHA and metadata_review['outcome_sha256'] == METADATA_OUTCOME_SHA and
                        metadata_review['durable_terminal_sha256'] == METADATA_TERMINAL_SHA and metadata_review['runtime_sha256'] == RUNTIME_SHA and
                        metadata_review['source_profiles'] == 20 and metadata_review['original_full_samples'] == 90 and
                        metadata_review['native_processes'] == metadata_review['fixture_reexecutions'] == metadata_review['benchmark_runs'] == 0 and
                        metadata_review['chart_admission'] is False, 'Actual complete20 Gleam metadata root closure missing')
                    bind(metadata_output / 'toolchain-runtime-receipt.json',RUNTIME_SHA)
                    published = closed_baseline(args,read,bind)
                    require(sha(PUBLISHED) == BASELINE_SHA, 'Changed immutable published baseline')
                    require(ROOT_DATA.read_bytes() == PUBLISHED.read_bytes(), 'Current published baseline differs')
                    require(sha(ROOT_DATA) == BASELINE_SHA, 'Changed live baseline observation')
                    bind(OLD_INPUT,OLD_INPUT_SHA)
                    live_observations = [dict(path=str(path),observed_sha256=BASELINE_SHA,
                        immutable_input=str(PUBLISHED),frozen_source_pin=False) for path in (ROOT_DATA,)]
                    before = deepcopy(published)
                    require(not any(row['key'] == 'gleam' for row in before['languages']) and
                        not any(row['language'] == 'gleam' for row in [*before['programs'].values(),*before.get('program_inventory',{}).values()]),
                        'Gleam is already present in the reviewed baseline')
                    receipt = read(FIXTURES / 'fixture-receipt.json',RECEIPT_SHA)
                    terminal = read(FIXTURES / 'terminal-pin.json',TERMINAL_SHA)
                    require(receipt['status'] == terminal['status'] == 'passed' and receipt['tests_run'] == 4 and
                        receipt['execution_completed'] is receipt['terminal_evidence_complete'] is True and receipt['loader_errors'] == [], 'Actual4 fixture closure missing')
                    for key in ('expected_test_ids','discovered_test_ids','actual_test_ids'):
                        require(set(receipt[key]) == set(IDS) and len(receipt[key]) == len(set(receipt[key])) == 4, 'Changed actual4 fixture identities')
                    require(all(receipt[k] == [] for k in ('failures','errors','skips','expected_failures','unexpected_successes')) and
                        not any(k in receipt for k in ('error','error_type','exception_journal_error','terminal_pin_errors')) and
                        receipt['actual_native_processes'] == terminal['actual_native_processes'] == 0 and
                        terminal['receipt'] == str(FIXTURES / 'fixture-receipt.json') and terminal['receipt_sha256'] == RECEIPT_SHA and
                        terminal['partial_evidence_pins'] == receipt['partial_evidence_pins'], 'Unsuccessful or incomplete original fixture outcome')
                    for values in (receipt['source_pins'],receipt['partial_evidence_pins']):
                        for path,digest in values.items():
                            require(path not in historical or historical[path] == digest, 'Conflicting original fixture pin')
                            historical[path] = digest
                    mutable_input = str(ROOT_DATA.resolve())
                    require(historical[mutable_input] == OLD_INPUT_SHA, 'Unexpected old mutable input identity')
                    for path,digest in historical.items():
                        if path == mutable_input:
                            bind(OLD_INPUT,digest)
                            require(sha(ROOT_DATA) == BASELINE_SHA, 'Changed current root baseline observation')
                        else: bind(path,digest)
                    snapshot = FIXTURES / 'snapshot'
                    for name in FILES:
                        digest = historical[str((snapshot / name).resolve())]
                        require(historical[str((CLBG / 'overview/explorer' / name).resolve())] == digest, 'Fixture source/snapshot mismatch')
                        bind(snapshot / name,digest); bind(CLBG / 'overview/explorer' / name,digest)
                    inventory_digest = bind(INVENTORY_WORKER,INVENTORY_WORKER_SHA)
                    sys.dont_write_bytecode = True
                    worker = cold_load('gleam_current_inventory_worker_v2',INVENTORY_WORKER,inventory_digest)
                    require(worker.FILES == FILES and worker.IDS == IDS, 'Changed accepted inventory worker fixture declarations')
                    for name in ('native_languages_data_v1','native_languages_data_v2','native_platform_metadata_v2',
                        'native_language_groups_v2','publication_endpoints_v1','native_languages_ready_v1'):
                        path = snapshot / (name + '.py'); cold_load(name,path,historical[str(path.resolve())])
                    immutable = OUT / 'input-data.json'
                    with immutable.open('xb') as stream: stream.write(PUBLISHED.read_bytes())
                    immutable.chmod(0o444); bind(immutable,BASELINE_SHA)
                    retained = fcntl.fcntl(common.fileno(),fcntl.F_DUPFD,64); os.set_inheritable(retained,True)
                    ready = sys.modules['native_languages_ready_v1']; candidate = deepcopy(before)
                    meaning = read(MEANING,MEANING_SHA)
                    adapter = cold_load('gleam_exact_owner_record_adapter_v1',RECORD_ADAPTER,args.adapter_sha256)
                    owner_ref = dict(path=str(MEANING),sha256=MEANING_SHA)
                    admit, complete_owners = adapter.admission_service(ready,meaning,owner_ref)
                    admission = admit(CLBG,candidate,packet_path=PACKET,
                        packet_sha256=args.packet_sha256,common_timing_mutex_fd=retained,finalize=False)
                    require(admission['ready_languages'] == ['gleam'] and admission['actual_full_native_processes'] == 90 and
                        admission['source_count'] == 20 and admission['samples_per_source'] == 3 and admission['source_or_timing_mutations'] == 0,
                        'Incomplete actual Gleam ready admission')
                    owner_projection = complete_owners()
                    candidate['provenance'][RECORD_ADAPTER.relative_to(CLBG).as_posix()] = args.adapter_sha256
                    worker.retain_program_inventory(candidate,admission['ready_languages'])
                    endpoint = ready.finalize_ready_endpoints(candidate,admission['ready_languages'])
                    preserve(before,candidate)
                    inventory = {key:row for key,row in candidate['program_inventory'].items() if row['language'] == 'gleam'}
                    require(len(inventory) == len({row['id'] for row in inventory.values()}) == 20, 'Incomplete20 Gleam source inventory')
                    require(len(candidate['program_inventory']) == len(before['program_inventory']) + 20 == 100,
                        'Unexpected added or removed source inventory')
                    for key,row in inventory.items():
                        benchmark = next(item for item in before['benchmarks'] if item['key'] == row['benchmark'])
                        require(key == row['key'] and len(row['samples']) == 3 and row['x'] == row['loc']/benchmark['baseline_loc'] and
                            row['y'] == row['seconds']/benchmark['baseline_seconds'] and row['files'] and all(identity in candidate['files'] for identity in row['files']),
                            'Gleam source inventory identity/baseline/samples/source mismatch')
                        if key in candidate['programs']: require(candidate['programs'][key] == row, 'Gleam endpoint/inventory mismatch')
                    require(sum(len(row['samples']) for row in inventory.values()) == 60 and
                        all(row.get('formatted_loc') is None and row.get('formatted_x') is None and
                            row.get('formatted_loc_status') != 'qualified' and row.get('formatted_loc_complete') is not True
                            for row in inventory.values()), 'Gleam samples or unqualified formatted coordinates differ')
                    require(not any(row['key'] in {'v','red'} for row in candidate['languages']) and
                        not any(row['language'] in {'v','red'} for row in [*candidate['programs'].values(),*candidate['program_inventory'].values()]) and
                        not any(group['language'] in {'v','red'} for benchmark in candidate['benchmarks'] for group in benchmark['groups']), 'Excluded language in candidate')
                    for path,digest in list(pins.items()): bind(path,digest)
                    require(ROOT_DATA.read_bytes() == PUBLISHED.read_bytes(), 'Live baseline changed during qualification')
                    save(OUT / 'candidate-data.json',candidate)
                    save(OUT / 'reused-qualification.json',dict(status='actual-original-four-reused-for-Gleam-without-fixture-reexecution',
                        original_fixture_receipt=dict(path=str(FIXTURES / 'fixture-receipt.json'),sha256=RECEIPT_SHA),
                        original_fixture_terminal=dict(path=str(FIXTURES / 'terminal-pin.json'),sha256=TERMINAL_SHA),
                        original_fixture_methods_passed=4,new_fixture_methods_executed=0,unchanged_eight_source_files=list(FILES),
                        original_mutable_input=dict(path=mutable_input,original_sha256=OLD_INPUT_SHA,immutable_snapshot=str(OLD_INPUT)),
                        published_baseline=dict(path=str(PUBLISHED),sha256=BASELINE_SHA,immutable_new_input=str(immutable)),
                        live_baseline_observations=live_observations,
                        admission=admission,endpoint_admission=endpoint,retained_Gleam_source_inventory=20,
                        exact_owner_projection=owner_projection,
                        retained_failed_schema_outcome=dict(path=str(HERE / 'jobs/gleam-current-candidate-v2/outcome.json'),sha256=FAILED_OUTCOME_SHA),
                        old_programs_files_inventory_baselines_groups_and_summaries_unchanged=True,live_dataset_mutations=0))
                    outcome['status'] = 'qualified-current-Gleam-candidate-awaiting-root-stage-review'
                except BaseException as error:
                    outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                    try: (OUT / 'error.log').write_text(traceback.format_exc())
                    except BaseException as journal_error:
                        outcome['error_log_write_failure'] = dict(error=str(journal_error) or type(journal_error).__name__,
                            error_type=type(journal_error).__name__)
                    raise
                finally:
                    try:
                        available, failures = {}, []
                        for path,digest in list(pins.items()):
                            try: bind(path,digest)
                            except BaseException as failure:
                                failures.append(dict(path=path,error=str(failure) or type(failure).__name__,error_type=type(failure).__name__))
                        for path in OUT.rglob('*'):
                            if path.is_file():
                                try: available[str(path.resolve())] = sha(path)
                                except BaseException as failure:
                                    failures.append(dict(path=str(path),error=str(failure) or type(failure).__name__,error_type=type(failure).__name__))
                        if failures: outcome.update(status='failed-terminal-integrity',terminal_pin_errors=failures,available_bytes_not_fully_pinned=True)
                        outcome.update(pins=pins,available_evidence_pins=available,common_mutex_held=True,common_timing_mutex=str(LOCK),live_dataset_mutations=0)
                        save(OUT / 'outcome.json',outcome)
                        if failures: raise RuntimeError('Gleam candidate terminal integrity failure: ' + str(failures))
                    finally:
                        if retained is not None: os.close(retained)
                print(json.dumps(dict(status=outcome['status'],candidate=str(OUT / 'candidate-data.json'),sha256=sha(OUT / 'candidate-data.json'),
                    actual_Gleam_sources=20,actual_full_processes=90,fixture_reexecutions=0,live_dataset_mutations=0)),flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker-sha256',required=True); parser.add_argument('--packet-sha256',required=True)
    parser.add_argument('--metadata-sha256',required=True);parser.add_argument('--review-sha256',required=True)
    parser.add_argument('--metadata-review-sha256',required=True);parser.add_argument('--baseline-review-sha256',required=True)
    parser.add_argument('--adapter-sha256',required=True);parser.add_argument('--failure-review-sha256',required=True)
    args = parser.parse_args()
    require(all(len(digest) == 64 and all(c in '0123456789abcdef' for c in digest) for digest in vars(args).values()), 'Invalid reviewed source/packet SHA')
    run(args)


if __name__ == '__main__': main()
