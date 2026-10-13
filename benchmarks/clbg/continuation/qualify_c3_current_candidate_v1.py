"""Reuse actual unchanged ready fixtures; add local C3 candidate and inventory.

No fixture, benchmark, preparation, build, native probe, signal, Git, credential,
server, Site operation or live dataset write occurs. Root separately reviews
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
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site')
ROOT_DATA = CLBG / 'overview/explorer/dist/data.json'
SITE_DATA = SITE / 'dist/data.json'
PUBLISHED = HERE / 'jobs/current-chart-session-v2/candidate-data.json'
BASELINE_SHA = '27b1c792b253d3d465ca6f853e7f85c9d089cb87e3b161a634feb7b5c1f69c8e'
OLD_INPUT = HERE / 'jobs/current-chart-session-v2/input-data.json'
OLD_INPUT_SHA = '65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f'
FIXTURE_JOB = HERE / 'owned-jobs/crystal-current-chart4-fixture-and-candidate-v1'
METADATA_JOB = HERE / 'owned-jobs/c3-reviewed-metadata-v1'
FIXTURES = HERE / 'jobs/native-ready-v1'
RECEIPT_SHA = 'bce66a4cb1fc02cc827c6906f9ddc5be4b3abddb8851e2028415600972ec29b8'
TERMINAL_SHA = '0b3a1cce3a5c2a41d27b88e2f3af8145d014c9ed3c1a2b5959c5c1613afeba59'
PACKET = HERE / 'c3-ready-publication-v1.json'
INVENTORY_WORKER = HERE / 'qualify_native_ready_current_v2.py'
OUT = HERE / 'jobs/c3-current-candidate-v1'
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
EXPECTED = dict(status='reviewed-completed-native-language-subset', ready_languages=['c3'],
    future_requested_languages=['elixir','kotlin'], unresolved=[], languages={'c3': {
        'suite': dict(path=str(CLBG / 'c3-comparison/20261012/reconciliation/full-v1-suite-v2.json'),
            sha256='333ae71d39ccc25644893ef060aa9293d08e81441769c4249fdc961bacb1245e'),
        'canonical_loc': dict(path=str(CLBG / 'c3-comparison/20261012/source-counts/canonical-loc-overlay-v2/qualified-loc-overlay.json'),
            sha256='4465bcc760401a7fd40aa516390a3681cd20edbb91538d9f5607a4060f1a5263'),
        'metadata': dict(path=str(HERE / 'publication-c3-metadata-v1/qualified/actual-metadata-v1/metadata.json'),
            sha256='c9c33e008e52280f5ec0a4051b2d57c0a9f052f3e89614ca2310c87e67250e07')}})


def require(value, why):
    if not value: raise ValueError(why)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def closed_durable(job, read):
    terminal = read(job / 'terminal.json')
    require(terminal['status'] == 'command-exited-zero-awaiting-report-review' and
        type(terminal['exit_code']) is int and terminal['exit_code'] == 0 and
        terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and
        terminal['stop_signals'] == [] and not any(terminal.get(k) for k in
        ('error','error_type','exception_cleanup','error_log_write_failure')), 'Original durable command not normal')
    request, started = read(job / 'request.json'), read(job / 'command-started.json')
    require(request['command'] == started['command'] and request['cwd'] == started['cwd'] == str(CLBG.parent.parent) and
        started['command_pid'] == terminal['command_pid'] and started['command_identity'] == terminal['command_identity'] and
        started['command_identity_observation_missing'] is False and request['common_mutex_owned_by_target_command'] is True,
        'Changed original durable command identity')
    return terminal


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
            set(new['languages'][len(old['languages']):]) <= {'c3'}, 'Changed prior trait memberships/order')
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
    print('WAIT closed C3 metadata lease', flush=True)
    with (METADATA_JOB / 'owner.lock').open('a+') as dependency:
        fcntl.flock(dependency, fcntl.LOCK_EX)
        held,named = os.fstat(dependency.fileno()), (METADATA_JOB / 'owner.lock').stat()
        require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different metadata lease')
        with (FIXTURE_JOB / 'owner.lock').open('a+') as old_owner:
            fcntl.flock(old_owner, fcntl.LOCK_EX)
            held,named = os.fstat(old_owner.fileno()), (FIXTURE_JOB / 'owner.lock').stat()
            require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different original fixture lease')
            print('WAIT common mutex: source-only C3 current candidate', flush=True)
            with LOCK.open('a+') as common:
                fcntl.flock(common, fcntl.LOCK_EX)
                held,named = os.fstat(common.fileno()), LOCK.stat()
                require((held.st_dev,held.st_ino) == (named.st_dev,named.st_ino), 'Different common mutex')
                OUT.mkdir(parents=True, exist_ok=False)
                pins, historical, retained = {}, {}, None
                outcome = dict(status='failed-current-C3-candidate',chart_admission=False,native_commands=0,fixture_reexecutions=0)
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
                    for path in (Path(__file__).resolve(),PACKET,INVENTORY_WORKER):
                        require(not path.stat().st_mode & 0o222, 'Reviewed frozen source/packet required')
                    bind(__file__, args.worker_sha256); bind(PACKET, args.packet_sha256)
                    require(read(PACKET) == EXPECTED, 'Changed reviewed C3 cohort descriptors')
                    bind(Path(sys.executable).resolve())
                    metadata_terminal = closed_durable(METADATA_JOB,read)
                    closed_durable(FIXTURE_JOB,read)
                    require(metadata_terminal['finished_at'].startswith('2026-10-13T18:37:'), 'Wrong actual C3 metadata closure')
                    metadata_output = HERE / 'publication-c3-metadata-v1/qualified/actual-metadata-v1'
                    metadata_outcome = read(metadata_output / 'outcome.json')
                    require(metadata_outcome['status'] == 'metadata-qualified-awaiting-root-whole-six-review' and
                        metadata_outcome['chart_admission'] is False and metadata_outcome['native_subjects'] == 0 and
                        not any(metadata_outcome.get(k) for k in ('error','error_type','terminal_error','terminal_error_type','available_bytes_not_fully_pinned')),
                        'Original metadata qualifier outcome not normal')
                    for values in (metadata_outcome['prerequisite_pins'],metadata_outcome['available_evidence_pins']):
                        for path,digest in values.items(): bind(path,digest)
                    published = read(PUBLISHED,BASELINE_SHA)
                    require(ROOT_DATA.read_bytes() == SITE_DATA.read_bytes() == PUBLISHED.read_bytes(), 'Current published baselines differ')
                    require(sha(ROOT_DATA) == sha(SITE_DATA) == BASELINE_SHA, 'Changed live baseline observation')
                    bind(OLD_INPUT,OLD_INPUT_SHA)
                    live_observations = [dict(path=str(path),observed_sha256=BASELINE_SHA,
                        immutable_input=str(PUBLISHED),frozen_source_pin=False) for path in (ROOT_DATA,SITE_DATA)]
                    before = deepcopy(published)
                    require(not any(row['key'] == 'c3' for row in before['languages']) and
                        not any(row['language'] == 'c3' for row in [*before['programs'].values(),*before.get('program_inventory',{}).values()]),
                        'C3 is already present in the reviewed baseline')
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
                    inventory_digest = bind(INVENTORY_WORKER)
                    sys.dont_write_bytecode = True
                    worker = cold_load('c3_current_inventory_worker_v2',INVENTORY_WORKER,inventory_digest)
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
                    require(admission['ready_languages'] == ['c3'] and admission['actual_full_native_processes'] == 90 and
                        admission['source_count'] == 20 and admission['samples_per_source'] == 3 and admission['source_or_timing_mutations'] == 0,
                        'Incomplete actual C3 ready admission')
                    worker.retain_program_inventory(candidate,admission['ready_languages'])
                    endpoint = ready.finalize_ready_endpoints(candidate,admission['ready_languages'])
                    preserve(before,candidate)
                    inventory = {key:row for key,row in candidate['program_inventory'].items() if row['language'] == 'c3'}
                    require(len(inventory) == len({row['id'] for row in inventory.values()}) == 20, 'Incomplete20 C3 source inventory')
                    for key,row in inventory.items():
                        benchmark = next(item for item in before['benchmarks'] if item['key'] == row['benchmark'])
                        require(key == row['key'] and len(row['samples']) == 3 and row['x'] == row['loc']/benchmark['baseline_loc'] and
                            row['y'] == row['seconds']/benchmark['baseline_seconds'] and row['files'] and all(identity in candidate['files'] for identity in row['files']),
                            'C3 source inventory identity/baseline/samples/source mismatch')
                        if key in candidate['programs']: require(candidate['programs'][key] == row, 'C3 endpoint/inventory mismatch')
                    require(not any(row['key'] in {'v','red'} for row in candidate['languages']) and
                        not any(row['language'] in {'v','red'} for row in [*candidate['programs'].values(),*candidate['program_inventory'].values()]) and
                        not any(group['language'] in {'v','red'} for benchmark in candidate['benchmarks'] for group in benchmark['groups']), 'Excluded language in candidate')
                    for path,digest in list(pins.items()): bind(path,digest)
                    require(ROOT_DATA.read_bytes() == SITE_DATA.read_bytes() == PUBLISHED.read_bytes(), 'Live baseline changed during qualification')
                    save(OUT / 'candidate-data.json',candidate)
                    save(OUT / 'reused-qualification.json',dict(status='actual-original-four-reused-for-C3-without-fixture-reexecution',
                        original_fixture_receipt=dict(path=str(FIXTURES / 'fixture-receipt.json'),sha256=RECEIPT_SHA),
                        original_fixture_terminal=dict(path=str(FIXTURES / 'terminal-pin.json'),sha256=TERMINAL_SHA),
                        original_fixture_methods_passed=4,new_fixture_methods_executed=0,unchanged_eight_source_files=list(FILES),
                        original_mutable_input=dict(path=mutable_input,original_sha256=OLD_INPUT_SHA,immutable_snapshot=str(OLD_INPUT)),
                        published_baseline=dict(path=str(PUBLISHED),sha256=BASELINE_SHA,immutable_new_input=str(immutable)),
                        live_baseline_observations=live_observations,
                        admission=admission,endpoint_admission=endpoint,retained_C3_source_inventory=20,
                        old_programs_files_inventory_baselines_groups_and_summaries_unchanged=True,live_dataset_mutations=0))
                    outcome['status'] = 'qualified-current-C3-candidate-awaiting-root-stage-review'
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
                        if failures: raise RuntimeError('C3 candidate terminal integrity failure: ' + str(failures))
                    finally:
                        if retained is not None: os.close(retained)
                print(json.dumps(dict(status=outcome['status'],candidate=str(OUT / 'candidate-data.json'),sha256=sha(OUT / 'candidate-data.json'),
                    actual_C3_sources=20,actual_full_processes=90,fixture_reexecutions=0,live_dataset_mutations=0)),flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--worker-sha256',required=True); parser.add_argument('--packet-sha256',required=True)
    args = parser.parse_args()
    require(all(len(digest) == 64 and all(c in '0123456789abcdef' for c in digest) for digest in vars(args).values()), 'Invalid reviewed source/packet SHA')
    run(args)


if __name__ == '__main__': main()
