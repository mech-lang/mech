"""Reuse actual unchanged ready fixtures; add local Nim candidate/inventory.

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
PUBLISHED = HERE / 'jobs/clojure-current-candidate-v1/candidate-data.json'
BASELINE_SHA = 'b088279074e85ec149466f876bdce49d4d497f7784a0d48be58e023e5b2c6a13'
OLD_INPUT = HERE / 'jobs/current-chart-session-v2/input-data.json'
OLD_INPUT_SHA = '65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f'
FIXTURE_JOB = HERE / 'owned-jobs/crystal-current-chart4-fixture-and-candidate-v1'
METADATA_JOB = HERE / 'owned-jobs/nim-publication-metadata-v2'
METADATA_SHA = '98efdbdfcdaacbf22c0644e4225b39e29b68ca94db4d392a3cdd436376ce3722'
METADATA_OUTCOME_SHA = 'a02dedb0aa44367b484841adcd2c3dc061364691ca41d93a71d4279416570cb4'
METADATA_TERMINAL_SHA = 'b6359c02526c8b1e7c1ba633f62a38c40fa96def3db6e61a463a519ccb55c49c'
RUNTIME_SHA = '1c6d5a872768b9bb1c47d321dbc909eb702899ecaaca3f5b1117d7746e770352'
BASELINE_JOB = HERE / 'owned-jobs/clojure-current-candidate-v1'
BASELINE_REVIEW = HERE / 'jobs/clojure-current-candidate-closed-review-v1/review.json'
BASELINE_TERMINAL_SHA = '01edefc859d80c556dfd879986323992aa806c542630b24761eab51fdcc2e6b2'
BASELINE_OUTCOME_SHA = '4f915b966b2818ccc0260d7592de9a168b248f609702d47bd314d594d072a5cc'
BASELINE_REUSED_SHA = 'b81c9095a76d2d10e8b24d65b6d0037419028b2f1f3e79d3966f75a7095bc123'
C3_BASELINE_SHA = '4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947'
FIXTURES = HERE / 'jobs/native-ready-v1'
RECEIPT_SHA = 'bce66a4cb1fc02cc827c6906f9ddc5be4b3abddb8851e2028415600972ec29b8'
TERMINAL_SHA = '0b3a1cce3a5c2a41d27b88e2f3af8145d014c9ed3c1a2b5959c5c1613afeba59'
PACKET = HERE / 'nim-ready-publication-v1.json'
INVENTORY_WORKER = HERE / 'qualify_native_ready_current_v2.py'
INVENTORY_WORKER_SHA = '4118db4115c2e37af1533495e24f45423e66194c6fc7fa87d04d60e6e821a7b8'
OUT = HERE / 'jobs/nim-current-candidate-v1'
NOTES = HERE / 'NIM_CURRENT_CANDIDATE_V1_REVIEW.md'
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
    return dict(status='reviewed-completed-native-language-subset', ready_languages=['nim'],
        future_requested_languages=['elixir','kotlin'], unresolved=[], languages={'nim': {
            'suite': dict(path=str(CLBG / 'nim-comparison/20261012/reconciliation/full-v6-suite-v2.json'),
                sha256='c4dff4ea00db0f380257708cfb0aea50b3a3eaecd1c5ce0f11f6cc780561f3f7'),
            'canonical_loc': dict(path=str(CLBG / 'nim-comparison/20261012/source-counts/helper-owners-v2/jobs/exact-source20-loc-v2/qualified-loc-overlay.json'),
                sha256='55ef7197d8caa62a5087cf93380e524d8a0fac11d2d82a731e44675e3d288398'),
            'metadata': dict(path=str(HERE / 'publication-nim-metadata-v2/qualified/actual-metadata-v2/metadata.json'),
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
    require(review['status'] == 'passed-closed-Clojure-candidate-review' and review['verified_pins'] == 5276 and
        review['candidate_sha256'] == BASELINE_SHA and review['previous_baseline_sha256'] == C3_BASELINE_SHA and
        review['terminal_sha256'] == BASELINE_TERMINAL_SHA and review['outcome_sha256'] == BASELINE_OUTCOME_SHA and
        review['inventory_sources'] == 20 and review['candidate_samples'] == 60 and review['clojure_endpoints'] == 18 and
        review['original_full_processes'] == 90 and review['prior_records_preserved'] is True and
        review['benchmark_runs'] == review['fixture_reexecutions'] == 0, 'Published Clojure baseline closure differs')
    with (BASELINE_JOB / 'owner.lock').open('r') as lease:
        fcntl.flock(lease,fcntl.LOCK_SH | fcntl.LOCK_NB)
        held,named = os.fstat(lease.fileno()),Path(lease.name).stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different closed baseline lease')
        terminal = closed_durable(BASELINE_JOB,read,BASELINE_TERMINAL_SHA)
        command = [PYTHON,'-I','-S','-B','-u',str(HERE / 'qualify_clojure_current_candidate_v1.py'),
            '--worker-sha256','48e3f61117396d1aef02257273cb6b1e43dd4b3ec23acfbb5702592e7a8afdc7',
            '--packet-sha256','050d460200926dc7c01c9b721c8f6f34d71c340d8388bfa736f71346eb82e509',
            '--metadata-sha256','8bf96ed71bd799b80fb6f062ff7b5b76c92d6e992e3e40ef307811f3c2c1b9ff',
            '--review-sha256','48b112d3f3641c9ed543577229e52f109ca2cd190661c6b0bc88f82f16540873']
        require(read(BASELINE_JOB / 'request.json')['command'] == command and
            terminal['qualification_granted'] is terminal['automatic_retry'] is False, 'Changed completed baseline command')
        outcome = read(PUBLISHED.parent / 'outcome.json',BASELINE_OUTCOME_SHA)
        require(outcome['status'] == 'qualified-current-Clojure-candidate-awaiting-root-stage-review' and
            outcome['common_mutex_held'] is True and outcome['common_timing_mutex'] == str(LOCK) and outcome['chart_admission'] is False and
            all(type(outcome[key]) is int and outcome[key] == 0 for key in ('native_commands','fixture_reexecutions','live_dataset_mutations')) and
            not any(outcome.get(key) for key in ('error','error_type','terminal_pin_errors','available_bytes_not_fully_pinned','error_log_write_failure')),
            'Completed baseline outcome differs')
        require(outcome['available_evidence_pins'][str(PUBLISHED)] == BASELINE_SHA and
            outcome['available_evidence_pins'][str(PUBLISHED.parent / 'input-data.json')] == C3_BASELINE_SHA and
            outcome['available_evidence_pins'][str(PUBLISHED.parent / 'reused-qualification.json')] == BASELINE_REUSED_SHA,
            'Completed baseline artifact identities differ')
        for values in (outcome['pins'],outcome['available_evidence_pins']):
            for path,digest in values.items(): bind(path,digest)
        prior = read(PUBLISHED.parent / 'reused-qualification.json',BASELINE_REUSED_SHA)
        require(prior['status'] == 'actual-original-four-reused-for-Clojure-without-fixture-reexecution' and
            prior['retained_Clojure_source_inventory'] == 20 and prior['old_programs_files_inventory_baselines_groups_and_summaries_unchanged'] is True and
            prior['new_fixture_methods_executed'] == prior['live_dataset_mutations'] == 0, 'Completed baseline preservation proof differs')
        data = read(PUBLISHED,BASELINE_SHA)
        inventory = [row for row in data['program_inventory'].values() if row['language'] == 'clojure']
        require(len(inventory) == 20 and sum(len(row['samples']) for row in inventory) == 60 and
            all(len(row['samples']) == 3 for row in inventory) and
            sum(row['language'] == 'clojure' for row in data['programs'].values()) == 18, 'Published baseline inventory changed')
        return data


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
            set(new['languages'][len(old['languages']):]) <= {'nim'}, 'Changed prior trait memberships/order')
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
    print('WAIT closed Nim metadata lease', flush=True)
    with (METADATA_JOB / 'owner.lock').open('r+') as dependency:
        fcntl.flock(dependency, fcntl.LOCK_EX)
        held,named = os.fstat(dependency.fileno()), (METADATA_JOB / 'owner.lock').stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different metadata lease')
        with (FIXTURE_JOB / 'owner.lock').open('r+') as old_owner:
            fcntl.flock(old_owner, fcntl.LOCK_EX)
            held,named = os.fstat(old_owner.fileno()), (FIXTURE_JOB / 'owner.lock').stat()
            require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different original fixture lease')
            print('WAIT common mutex: source-only Nim current candidate', flush=True)
            with LOCK.open('r+') as common:
                fcntl.flock(common, fcntl.LOCK_EX)
                held,named = os.fstat(common.fileno()), LOCK.stat()
                require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino) and held.st_ino == 79859134, 'Different common mutex')
                OUT.mkdir(parents=True, exist_ok=False)
                pins, historical, retained = {}, {}, None
                outcome = dict(status='failed-current-Nim-candidate',chart_admission=False,native_commands=0,fixture_reexecutions=0)
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
                    for path in (Path(__file__).resolve(),PACKET,INVENTORY_WORKER,NOTES):
                        require(not path.stat().st_mode & 0o222, 'Reviewed frozen source/packet required')
                    bind(__file__, args.worker_sha256); bind(PACKET, args.packet_sha256);bind(NOTES,args.review_sha256)
                    require(args.metadata_sha256 == METADATA_SHA and read(PACKET) == expected_packet(METADATA_SHA), 'Changed reviewed actual Nim cohort descriptors')
                    bind(Path(sys.executable).resolve())
                    closed_durable(METADATA_JOB,read,METADATA_TERMINAL_SHA)
                    closed_durable(FIXTURE_JOB,read)
                    metadata_output = HERE / 'publication-nim-metadata-v2/qualified/actual-metadata-v2'
                    metadata_outcome = read(metadata_output / 'outcome.json',METADATA_OUTCOME_SHA)
                    bind(metadata_output / 'metadata.json',METADATA_SHA)
                    require(metadata_outcome['status'] == 'metadata-qualified-awaiting-root-whole-six-review' and
                        metadata_outcome['chart_admission'] is False and metadata_outcome['native_subjects'] == 0 and
                        not any(metadata_outcome.get(k) for k in ('error','error_type','terminal_error','terminal_error_type','integrity_errors','available_bytes_not_fully_pinned')),
                        'Original metadata qualifier outcome not normal')
                    for values in (metadata_outcome['prerequisite_pins'],metadata_outcome['available_evidence_pins']):
                        for path,digest in values.items(): bind(path,digest)
                    metadata_source = HERE / 'publication-nim-metadata-v2'
                    metadata_request = read(METADATA_JOB / 'request.json')
                    expected_metadata_command = ['/opt/homebrew/opt/python@3.14/bin/python3.14','-B','-u',str(metadata_source / 'qualify_descriptor.py'),
                        '--revision','actual-metadata-v2','--qualifier-sha256','925834eba9781dd4805860127dfa391a1ed3fcd841729e1f20c82ac33a6a5edc',
                        '--descriptor-sha256','7c5b269d641c685f33e731cff0bc7ecc1625e7bbf756295c78c2bc3978e2a460',
                        '--suite',expected_packet(METADATA_SHA)['languages']['nim']['suite']['path'],
                        '--suite-sha256','c4dff4ea00db0f380257708cfb0aea50b3a3eaecd1c5ce0f11f6cc780561f3f7',
                        '--canonical-loc',expected_packet(METADATA_SHA)['languages']['nim']['canonical_loc']['path'],
                        '--canonical-loc-sha256','55ef7197d8caa62a5087cf93380e524d8a0fac11d2d82a731e44675e3d288398',
                        '--full-index',str(CLBG / 'nim-comparison/20261012/studies/full-v6/index.json'),
                        '--full-index-sha256','462bfa79e8fdd4900b6c4b575b7645c25675ac81f0161ad229d1941241eb9923',
                        '--importer-sha256','27779d09a6cfea98d0377c58d4cdd459fc7f59aac46efdb8f61144070a871dc7']
                    require(metadata_request['command'] == expected_metadata_command, 'Different actual metadata command/source arguments')
                    metadata_terminal = read(METADATA_JOB / 'terminal.json',METADATA_TERMINAL_SHA)
                    observed = metadata_terminal['command_identity']['listing'].split(None,7)
                    require(observed[:2] == ['26623','26619'] and observed[7].split() == expected_metadata_command,
                        'Different actual metadata process image/arguments')
                    require(pins[str(metadata_source / 'qualify_descriptor.py')] == expected_metadata_command[7] and
                        pins[str(metadata_source / 'descriptor.json')] == expected_metadata_command[9], 'Changed actual qualifier/descriptor bodies')
                    metadata_review = read(HERE / 'jobs/nim-metadata-v2-closed-review-v1/receipt.json',args.metadata_review_sha256)
                    require(metadata_review['status'] == 'root-reviewed-qualified-Nim20-runtime-metadata' and metadata_review['verified_pins'] == 38829 and
                        metadata_review['metadata_sha256'] == METADATA_SHA and metadata_review['outcome_sha256'] == METADATA_OUTCOME_SHA and
                        metadata_review['durable_terminal_sha256'] == METADATA_TERMINAL_SHA and metadata_review['runtime_receipt_sha256'] == RUNTIME_SHA and
                        metadata_review['profile_count'] == 20 and metadata_review['original_compiler_roots'] == 3 and
                        metadata_review['reused_private_v2_image_profiles'] == ['nim-pidigits-compact','nim-regexredux-compact',
                            'nim-regexredux-optimized','nim-revcomp-compact','nim-revcomp-optimized'] and
                        metadata_review['memory_strategies'] == ['rc','gc'] and metadata_review['architecture'] == 'arm64' and
                        metadata_review['source_counts_unchanged'] is True and metadata_review['full_samples_reused'] == 90 and
                        metadata_review['native_processes'] == metadata_review['benchmark_runs'] == 0 and metadata_review['chart_admission'] is False,
                        'Actual complete20 metadata root closure missing')
                    bind(metadata_output / 'toolchain-runtime-receipt.json',RUNTIME_SHA)
                    published = closed_baseline(args,read,bind)
                    require(sha(PUBLISHED) == BASELINE_SHA, 'Changed immutable published baseline')
                    require(ROOT_DATA.read_bytes() == PUBLISHED.read_bytes(), 'Current published baseline differs')
                    require(sha(ROOT_DATA) == BASELINE_SHA, 'Changed live baseline observation')
                    bind(OLD_INPUT,OLD_INPUT_SHA)
                    live_observations = [dict(path=str(path),observed_sha256=BASELINE_SHA,
                        immutable_input=str(PUBLISHED),frozen_source_pin=False) for path in (ROOT_DATA,)]
                    before = deepcopy(published)
                    require(not any(row['key'] == 'nim' for row in before['languages']) and
                        not any(row['language'] == 'nim' for row in [*before['programs'].values(),*before.get('program_inventory',{}).values()]),
                        'Nim is already present in the reviewed baseline')
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
                    worker = cold_load('nim_current_inventory_worker_v2',INVENTORY_WORKER,inventory_digest)
                    require(worker.FILES == FILES and worker.IDS == IDS, 'Changed accepted inventory worker fixture declarations')
                    for name in ('native_languages_data_v1','native_languages_data_v2','native_platform_metadata_v2',
                        'native_language_groups_v2','publication_endpoints_v1','native_languages_ready_v1'):
                        path = snapshot / (name + '.py'); cold_load(name,path,historical[str(path.resolve())])
                    immutable = OUT / 'input-data.json'
                    with immutable.open('xb') as stream: stream.write(PUBLISHED.read_bytes())
                    immutable.chmod(0o444); bind(immutable,BASELINE_SHA)
                    retained = fcntl.fcntl(common.fileno(),fcntl.F_DUPFD,64); os.set_inheritable(retained,True)
                    ready = sys.modules['native_languages_ready_v1']; candidate = deepcopy(before)
                    admission = ready.add_ready_native_languages(CLBG,candidate,packet_path=PACKET,
                        packet_sha256=args.packet_sha256,common_timing_mutex_fd=retained,finalize=False)
                    require(admission['ready_languages'] == ['nim'] and admission['actual_full_native_processes'] == 90 and
                        admission['source_count'] == 20 and admission['samples_per_source'] == 3 and admission['source_or_timing_mutations'] == 0,
                        'Incomplete actual Nim ready admission')
                    worker.retain_program_inventory(candidate,admission['ready_languages'])
                    endpoint = ready.finalize_ready_endpoints(candidate,admission['ready_languages'])
                    preserve(before,candidate)
                    inventory = {key:row for key,row in candidate['program_inventory'].items() if row['language'] == 'nim'}
                    require(len(inventory) == len({row['id'] for row in inventory.values()}) == 20, 'Incomplete20 Nim source inventory')
                    for key,row in inventory.items():
                        benchmark = next(item for item in before['benchmarks'] if item['key'] == row['benchmark'])
                        require(key == row['key'] and len(row['samples']) == 3 and row['x'] == row['loc']/benchmark['baseline_loc'] and
                            row['y'] == row['seconds']/benchmark['baseline_seconds'] and row['files'] and all(identity in candidate['files'] for identity in row['files']),
                            'Nim source inventory identity/baseline/samples/source mismatch')
                        if key in candidate['programs']: require(candidate['programs'][key] == row, 'Nim endpoint/inventory mismatch')
                    require(not any(row['key'] in {'v','red'} for row in candidate['languages']) and
                        not any(row['language'] in {'v','red'} for row in [*candidate['programs'].values(),*candidate['program_inventory'].values()]) and
                        not any(group['language'] in {'v','red'} for benchmark in candidate['benchmarks'] for group in benchmark['groups']), 'Excluded language in candidate')
                    for path,digest in list(pins.items()): bind(path,digest)
                    require(ROOT_DATA.read_bytes() == PUBLISHED.read_bytes(), 'Live baseline changed during qualification')
                    save(OUT / 'candidate-data.json',candidate)
                    save(OUT / 'reused-qualification.json',dict(status='actual-original-four-reused-for-Nim-without-fixture-reexecution',
                        original_fixture_receipt=dict(path=str(FIXTURES / 'fixture-receipt.json'),sha256=RECEIPT_SHA),
                        original_fixture_terminal=dict(path=str(FIXTURES / 'terminal-pin.json'),sha256=TERMINAL_SHA),
                        original_fixture_methods_passed=4,new_fixture_methods_executed=0,unchanged_eight_source_files=list(FILES),
                        original_mutable_input=dict(path=mutable_input,original_sha256=OLD_INPUT_SHA,immutable_snapshot=str(OLD_INPUT)),
                        published_baseline=dict(path=str(PUBLISHED),sha256=BASELINE_SHA,immutable_new_input=str(immutable)),
                        live_baseline_observations=live_observations,
                        admission=admission,endpoint_admission=endpoint,retained_Nim_source_inventory=20,
                        old_programs_files_inventory_baselines_groups_and_summaries_unchanged=True,live_dataset_mutations=0))
                    outcome['status'] = 'qualified-current-Nim-candidate-awaiting-root-stage-review'
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
                        if failures: raise RuntimeError('Nim candidate terminal integrity failure: ' + str(failures))
                    finally:
                        if retained is not None: os.close(retained)
                print(json.dumps(dict(status=outcome['status'],candidate=str(OUT / 'candidate-data.json'),sha256=sha(OUT / 'candidate-data.json'),
                    actual_Nim_sources=20,actual_full_processes=90,fixture_reexecutions=0,live_dataset_mutations=0)),flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker-sha256',required=True); parser.add_argument('--packet-sha256',required=True)
    parser.add_argument('--metadata-sha256',required=True);parser.add_argument('--review-sha256',required=True)
    parser.add_argument('--metadata-review-sha256',required=True);parser.add_argument('--baseline-review-sha256',required=True)
    args = parser.parse_args()
    require(all(len(digest) == 64 and all(c in '0123456789abcdef' for c in digest) for digest in vars(args).values()), 'Invalid reviewed source/packet SHA')
    run(args)


if __name__ == '__main__': main()
