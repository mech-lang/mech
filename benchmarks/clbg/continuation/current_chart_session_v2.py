"""Hold one common-lock description through qualification, staging, QA and packaging.

Source candidate only. No benchmark, runtime preparation, Git or credential is
handled here. The supported Sites helper owns source commit/push/archive work.
"""
import fcntl
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site')
NODE = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'
HELPER = '/Users/cim310/.codex/plugins/cache/openai-curated-remote/sites/1.0.1/scripts/site-workflow.mjs'
PROJECT = 'appgprj_6ac8f49664688191961eac14d8a9465b'
BASELINE_SHA = '65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f'
EXPECTED = {
    'status': 'reviewed-completed-native-language-subset',
    'ready_languages': ['crystal'], 'future_requested_languages': ['elixir', 'kotlin'], 'unresolved': [],
    'languages': {'crystal': {
        'suite': {'path': str(CLBG/'crystal-comparison/20261012/reconciliation/full-v1-suite-v2.json'),
            'sha256': '386356315c9fc5c680e99077fa70fe52db130c369fe74f7e0aa301e5f7ea6490'},
        'canonical_loc': {'path': str(CLBG/'crystal-comparison/20261012/source-counts/corrected-qualified-overlay-v2/qualified-loc-overlay.json'),
            'sha256': 'e22140622a5154a1e3b9512580eb469c8c078beda4a2ae9812bf429e51e089e2'},
        'metadata': {'path': str(HERE/'publication-crystal-metadata-v2/qualified/actual-metadata-v2/metadata.json'),
            'sha256': '63b93da6f941ae4ea276d6e595d29b438ba64e315b29f461874a0edbedfa1a1f'}}}}


def run():
    with (HERE/'current-chart-session-v2-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        held, named = os.fstat(owner.fileno()), (HERE/'current-chart-session-v2-owner.lock').stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        print('WAIT common mutex: current chart qualification, browser QA and packaging', flush=True)
        with LOCK.open('a+') as common:
            fcntl.flock(common, fcntl.LOCK_EX)
            held, named = os.fstat(common.fileno()), LOCK.stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
            retained = fcntl.fcntl(common.fileno(), fcntl.F_DUPFD, 64)
            os.set_inheritable(retained, True)
            # All source imports, hashing and JSON parsing begin after the grant.
            from copy import deepcopy
            import hashlib
            import http.server
            import json
            import signal
            import stat
            import subprocess
            import threading
            import types

            def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
            def save(path, value):
                with Path(path).open('x') as stream:
                    json.dump(value, stream, indent=2); stream.write('\n')
            def alarm(_number, _frame): raise TimeoutError('Current chart session exceeded 1200 seconds')
            journal = HERE/'jobs/current-chart-session-v2'
            output = HERE/'jobs/native-ready-v1'
            durable_terminal_path = HERE/'owned-jobs/crystal-current-chart4-fixture-and-candidate-v1/terminal.json'
            server, thread = None, None
            previous_handler = signal.getsignal(signal.SIGALRM)
            created = False
            try:
                assert not journal.exists(), 'Exclusive new session; no retry or overwrite'
                journal.mkdir(parents=True); created = True
                signal.signal(signal.SIGALRM, alarm); signal.alarm(1200)
                source = Path(__file__).resolve()
                worker_path = HERE/'qualify_native_ready_current_v2.py'
                packet = HERE/'crystal-ready-publication-v1.json'
                for path in (source, worker_path, packet):
                    assert path.is_file() and not path.stat().st_mode & 0o222, 'Reviewed frozen source/packet required'
                source_pins = {str(path): sha(path) for path in (source, worker_path, packet)}
                raw_packet = packet.read_bytes()
                assert json.loads(raw_packet) == EXPECTED, 'Reviewed Crystal descriptors changed'
                assert hashlib.sha256(raw_packet).hexdigest() == source_pins[str(packet)]
                root_data, site_data = CLBG/'overview/explorer/dist/data.json', SITE/'dist/data.json'
                baseline = root_data.read_bytes()
                assert hashlib.sha256(baseline).hexdigest() == BASELINE_SHA, 'Repository baseline changed; new review required'
                assert site_data.read_bytes() == baseline and sha(site_data) == BASELINE_SHA, 'Opened Site baseline differs'
                before = json.loads(baseline)
                # Pin an immutable exact input snapshot, so staging dist/data
                # does not invalidate the worker's closed source-pin receipt.
                input_data = journal/'input-data.json'
                with input_data.open('xb') as stream: stream.write(baseline)
                input_data.chmod(0o444); assert sha(input_data) == BASELINE_SHA
                save(journal/'started.json', dict(status='entered-single-common-grant',
                    common_mutex_fd=retained, common_mutex_inode=named.st_ino,
                    source_pins=source_pins, baseline_sha256=BASELINE_SHA,
                    immutable_worker_input=str(input_data), native_benchmarks_started=0,
                    secret_input_received=False, automatic_retry=False))
                name = 'current_chart_ready_worker_v2'
                assert name not in sys.modules
                raw_worker = worker_path.read_bytes()
                assert hashlib.sha256(raw_worker).hexdigest() == source_pins[str(worker_path)]
                sys.dont_write_bytecode = True
                worker = types.ModuleType(name); worker.__file__ = str(worker_path)
                sys.modules[name] = worker
                # Compile the verified source bytes directly; never load .pyc.
                exec(compile(raw_worker, str(worker_path), 'exec'), worker.__dict__)
                assert Path(worker.__file__).resolve() == worker_path.resolve()
                assert sha(worker_path) == source_pins[str(worker_path)]
                # Reuse the completed exact-four qualification. No test module
                # or worker.run is executed again.
                receipt_path, terminal_path = output/'fixture-receipt.json', output/'terminal-pin.json'
                receipt = json.loads(receipt_path.read_bytes()); terminal = json.loads(terminal_path.read_bytes())
                durable = json.loads(durable_terminal_path.read_bytes())
                assert durable['status'] == 'command-exited-zero-awaiting-report-review'
                assert durable['actual_command_started'] is True and type(durable['exit_code']) is int and durable['exit_code'] == 0
                assert durable['stop_signals'] == [] and durable['command_identity_observation_missing'] is False
                assert durable['command_identity'] and durable['worker_identity']
                assert durable['started_at'].startswith('2026-10-13T15:49:')
                assert durable['finished_at'].startswith('2026-10-13T16:43:')
                assert not any(key in durable for key in ('error', 'error_type', 'cleanup_error', 'log_error', 'failure_drain_error'))
                assert receipt['status'] == terminal['status'] == 'passed'
                assert receipt['tests_run'] == 4 and receipt['execution_completed'] is True
                assert len(receipt['discovered_test_ids']) == len(receipt['actual_test_ids']) == 4
                assert set(receipt['expected_test_ids']) == set(receipt['discovered_test_ids']) == set(receipt['actual_test_ids']) == set(worker.IDS)
                assert receipt['loader_errors'] == [] and receipt['terminal_evidence_complete'] is True
                for key in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'):
                    assert receipt[key] == []
                assert not any(key in receipt for key in ('error', 'error_type', 'exception_journal_error', 'terminal_pin_errors'))
                assert receipt['actual_native_processes'] == terminal['actual_native_processes'] == 0
                assert terminal['receipt'] == str(receipt_path) and terminal['receipt_sha256'] == sha(receipt_path)
                assert terminal['partial_evidence_pins'] == receipt['partial_evidence_pins']
                historical_pins = {}
                for pins in (receipt['source_pins'], receipt['partial_evidence_pins']):
                    for path, digest in pins.items():
                        assert path not in historical_pins or historical_pins[path] == digest
                        historical_pins[path] = digest
                assert historical_pins[str(root_data.resolve())] == BASELINE_SHA
                historical_baseline = str(root_data.resolve())
                def verify_historical_pins(after_stage=False):
                    for path, digest in historical_pins.items():
                        if after_stage and str(Path(path).resolve()) == historical_baseline:
                            assert digest == BASELINE_SHA and sha(input_data) == digest
                            assert sha(root_data) == candidate_sha
                        else: assert sha(path) == digest, 'Original qualification pin changed: '+path
                verify_historical_pins()
                source_pins.update({str(path): sha(path) for path in (receipt_path, terminal_path, durable_terminal_path)})
                old_candidate_path = output/'candidate-data.json'
                assert receipt['candidate_data'] == str(old_candidate_path)
                assert receipt['candidate_admission']['ready_languages'] == ['crystal']
                assert receipt['candidate_admission']['actual_full_native_processes'] == 90
                assert receipt['candidate_admission']['source_count'] == 20
                assert historical_pins[str(old_candidate_path.resolve())] == sha(old_candidate_path)
                old_candidate = json.loads(old_candidate_path.read_bytes())
                snapshot = output/'snapshot'
                adapter_names = ('native_languages_data_v1', 'native_languages_data_v2',
                    'native_platform_metadata_v2', 'native_language_groups_v2',
                    'publication_endpoints_v1', 'native_languages_ready_v1')
                for adapter_name in adapter_names:
                    assert adapter_name not in sys.modules
                    adapter_path = snapshot/(adapter_name+'.py')
                    raw = adapter_path.read_bytes()
                    assert hashlib.sha256(raw).hexdigest() == historical_pins[str(adapter_path.resolve())]
                    module = types.ModuleType(adapter_name); module.__file__ = str(adapter_path)
                    sys.modules[adapter_name] = module
                    exec(compile(raw, str(adapter_path), 'exec'), module.__dict__)
                    assert Path(module.__file__).resolve() == adapter_path.resolve()
                ready = sys.modules['native_languages_ready_v1']
                candidate = deepcopy(before)
                admission = ready.add_ready_native_languages(CLBG, candidate, packet_path=packet,
                    packet_sha256=source_pins[str(packet)], common_timing_mutex_fd=retained, finalize=False)
                worker.retain_program_inventory(candidate, admission['ready_languages'])
                endpoint_admission = ready.finalize_ready_endpoints(candidate, admission['ready_languages'])
                comparison = deepcopy(candidate)
                if 'program_inventory' in old_candidate:
                    comparison['program_inventory'] = deepcopy(old_candidate['program_inventory'])
                else: comparison.pop('program_inventory', None)
                assert comparison == old_candidate, 'Rebuilt qualified candidate differs beyond added inventory'
                assert admission == receipt['candidate_admission']
                assert endpoint_admission == receipt['endpoint_admission']
                verify_historical_pins()
                for path, digest in source_pins.items(): assert sha(path) == digest
                candidate_path = journal/'candidate-data.json'
                save(candidate_path, candidate)
                candidate_bytes = candidate_path.read_bytes()
                save(journal/'reused-qualification.json', dict(status='actual-original-four-reused-without-fixture-reexecution',
                    original_fixture_receipt=dict(path=str(receipt_path), sha256=source_pins[str(receipt_path)]),
                    original_fixture_terminal=dict(path=str(terminal_path), sha256=source_pins[str(terminal_path)]),
                    original_durable_terminal=dict(path=str(durable_terminal_path), sha256=source_pins[str(durable_terminal_path)]),
                    new_fixture_methods_executed=0, original_fixture_methods_passed=4,
                    candidate_admission=admission, endpoint_admission=endpoint_admission,
                    candidate_equal_to_original_except_inventory=True,
                    historical_mutable_input=dict(path=historical_baseline, original_sha256=BASELINE_SHA,
                        immutable_snapshot=str(input_data)), source_pins=source_pins))
                current_languages = {row['key']: row for row in candidate['languages']}
                for row in before['languages']: assert current_languages[row['key']] == row
                for key, row in before['programs'].items(): assert candidate['programs'][key] == row
                for key, row in before['files'].items(): assert candidate['files'][key] == row
                for key, row in before.get('program_inventory', {}).items(): assert candidate['program_inventory'][key] == row
                for row in before.get('reference_summary', []):
                    assert next(item for item in candidate['reference_summary'] if item['lang'] == row['lang']) == row
                for old in before['benchmarks']:
                    current = next(item for item in candidate['benchmarks'] if item['key'] == old['key'])
                    assert {key: value for key, value in current.items() if key != 'groups'} == {key: value for key, value in old.items() if key != 'groups'}
                    for group in old['groups']:
                        assert next(item for item in current['groups'] if item['language'] == group['language']) == group
                inventory = candidate['program_inventory']
                crystal_rows = {key: row for key, row in inventory.items() if row['language'] == 'crystal'}
                assert len(crystal_rows) == 20 and len({row['id'] for row in crystal_rows.values()}) == 20
                for key, row in crystal_rows.items():
                    benchmark = next(item for item in before['benchmarks'] if item['key'] == row['benchmark'])
                    assert key == row['key'] and len(row['samples']) == 3
                    assert row['x'] == row['loc']/benchmark['baseline_loc'] and row['y'] == row['seconds']/benchmark['baseline_seconds']
                    assert row['files'] and all(identity in candidate['files'] for identity in row['files'])
                    if key in candidate['programs']: assert candidate['programs'][key] == row
                assert not any(row['key'] in {'v', 'red'} for row in candidate['languages'])
                assert not any(row['language'] in {'v', 'red'} for row in [*candidate['programs'].values(), *inventory.values()])
                assert not any(group['language'] in {'v', 'red'} for item in candidate['benchmarks'] for group in item['groups'])
                assert root_data.read_bytes() == site_data.read_bytes() == baseline, 'Baseline changed before staging'
                candidate_sha = hashlib.sha256(candidate_bytes).hexdigest()
                staged_paths = []
                for target in (root_data, site_data):
                    temporary = target.with_name('.data.current-chart-session-v2.tmp')
                    mode = stat.S_IMODE(target.stat().st_mode)
                    with temporary.open('xb') as stream:
                        stream.write(candidate_bytes); stream.flush(); os.fsync(stream.fileno())
                    temporary.chmod(mode); os.replace(temporary, target)
                    assert sha(target) == candidate_sha; staged_paths.append(str(target))
                    save(journal/('stage-'+str(len(staged_paths))+'.json'), dict(path=str(target), sha256=candidate_sha))
                save(journal/'qualified-stage.json', dict(status='qualified-and-staged-awaiting-browser-QA',
                    candidate=str(candidate_path), candidate_sha256=candidate_sha,
                    fixture_receipt=dict(path=str(receipt_path), sha256=sha(receipt_path)),
                    fixture_terminal=dict(path=str(terminal_path), sha256=sha(terminal_path)),
                    retained_twenty_source_inventory=True, old_records_and_fixed_baselines_unchanged=True,
                    staged_paths=staged_paths, native_benchmarks_started=0, audience_changes=0))
                print('QUALIFIED DATA STAGED. After root copies reviewed UI, send preview on stdin.', flush=True)
                assert sys.stdin.readline().strip() == 'preview', 'Expected reviewed UI handoff before preview'
                assert sha(root_data) == sha(site_data) == candidate_sha, 'Qualified data must remain exact before preview'
                assets = {str(path.resolve()): sha(path) for path in sorted((SITE/'dist').rglob('*')) if path.is_file()}
                class Handler(http.server.SimpleHTTPRequestHandler):
                    def __init__(self, *values, **options):
                        super().__init__(*values, directory=str(SITE/'dist'), **options)
                    def log_message(self, _format, *_values): pass
                server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
                thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
                url = 'http://127.0.0.1:'+str(server.server_address[1])+'/'
                save(journal/'preview.json', dict(url=url, source_asset_pins=assets, common_mutex_fd=retained))
                print('READY '+url, flush=True)
                print('After browser QA, send package on stdin. Do not send credentials until the Sites helper asks.', flush=True)
                while True:
                    command = sys.stdin.readline().strip()
                    if command == 'package': break
                    assert command == 'refresh-ui', 'Expected package or refresh-ui after browser QA'
                    assert sha(root_data) == sha(site_data) == candidate_sha, 'Qualified data must remain exact'
                    assets = {str(path.resolve()): sha(path) for path in sorted((SITE/'dist').rglob('*')) if path.is_file()}
                    revision = len(list(journal.glob('ui-refresh-*.json'))) + 1
                    save(journal/('ui-refresh-'+str(revision)+'.json'), dict(status='root-reviewed-UI-refreshed-awaiting-repeat-browser-QA', source_asset_pins=assets, candidate_sha256=candidate_sha))
                    print('UI REFRESHED. Repeat affected syntax/browser checks before package.', flush=True)
                verify_historical_pins(after_stage=True)
                for pins in (source_pins, assets):
                    for path, digest in pins.items(): assert sha(path) == digest, 'Reviewed asset/evidence changed before packaging: '+path
                assert sha(root_data) == sha(site_data) == candidate_sha
                server.shutdown(); server.server_close(); server = None
                thread.join(timeout=10); assert not thread.is_alive(); thread = None
                save(journal/'packaging-handoff.json', dict(status='entering-supported-Sites-helper',
                    common_mutex_fd=retained, candidate_sha256=candidate_sha, credentials_received=False,
                    node=NODE, helper=HELPER, project_id=PROJECT, audience_changes=0))
                held, named = os.fstat(retained), LOCK.stat()
                assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
                fcntl.flock(retained, fcntl.LOCK_EX | fcntl.LOCK_NB)
                os.set_inheritable(retained, True)
                signal.alarm(0); signal.signal(signal.SIGALRM, previous_handler)
                # Child inherits terminal directly. Python never reads or
                # captures the supported helper's credential JSON.
                helper = subprocess.Popen([NODE, HELPER, '--project-id', PROJECT],
                    cwd=str(SITE), close_fds=True, pass_fds=(retained,))
                save(journal/'helper-started.json', dict(pid=helper.pid, common_mutex_fd=retained,
                    credentials_read_by_coordinator=False, supported_helper_owns_site_git=True))
                helper_exit = helper.wait()
                save(journal/'helper-completed.json', dict(exit_code=helper_exit,
                    common_mutex_retained=True, credentials_read_by_coordinator=False))
                assert type(helper_exit) is int and helper_exit == 0, 'Supported Sites helper failed'
                print('SITE WORKFLOW COMPLETE. Publish and save the authorized checkpoint, then send release.', flush=True)
                assert sys.stdin.readline().strip() == 'release', 'Expected nonsecret release after publication/checkpoint'
                verify_historical_pins(after_stage=True)
                for path, digest in source_pins.items(): assert sha(path) == digest
                assert sha(root_data) == sha(site_data) == candidate_sha
                save(journal/'released.json', dict(status='root-authorized-publication-and-checkpoint-complete',
                    helper_exit_code=helper_exit, candidate_sha256=candidate_sha,
                    native_benchmarks_started=0, audience_changes=0, credentials_read_by_coordinator=False))
            except BaseException as error:
                if created:
                    save(journal/'failure.json', dict(status='failed-with-preserved-session-evidence',
                        error=str(error) or type(error).__name__, error_type=type(error).__name__,
                        native_benchmarks_started=0, automatic_retry=False))
                raise
            finally:
                signal.alarm(0); signal.signal(signal.SIGALRM, previous_handler)
                try:
                    if server is not None:
                        try:
                            if thread is not None and thread.is_alive(): server.shutdown()
                        finally: server.server_close()
                    if thread is not None and thread.ident is not None: thread.join(timeout=10)
                finally: os.close(retained)


if __name__ == '__main__': run()
