"""Independent metadata-only review of the completed four-extraction fixture job."""
from pathlib import Path
import fcntl
import hashlib
import json
import os
from datetime import datetime, timezone

ROOT = Path('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech')
CONT = ROOT/'benchmarks/clbg/continuation'
PACKET = ROOT/'benchmarks/clbg/prolog-comparison/20261013/revisions/runtime-directory-v1'
JOB = CONT/'owned-jobs/prolog-directory4-fixtures-v1'
OUTPUT = CONT/'jobs/prolog-directory4-actual-review-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
IDS = tuple('test_extract_source.DirectoryContracts.' + value for value in (
    'test_repeated_exact_directory_identity_allows_only_unapplied_timestamp_difference',
    'test_regular_link_and_cross_type_duplicates_never_overwrite',
    'test_alias_escape_metadata_and_unselected_member_types_reject',
    'test_existing_destination_and_dangling_link_keep_original_policy'))
EXPECTED = [
    '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
    '-I','-S','-B','-u',str(PACKET/'qualify_extract.py'),'--revision','fixtures-v1']

def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()

def save(path,value):
    with path.open('x') as stream:
        json.dump(value,stream,indent=2); stream.write('\n')

print('WAIT common mutex: independent directory4 actual receipt/pin review',flush=True)
with LOCK.open('a+') as common:
    fcntl.flock(common,fcntl.LOCK_EX)
    held,named = os.fstat(common.fileno()),LOCK.stat()
    assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
    with (JOB/'owner.lock').open('r+') as private:
        fcntl.flock(private,fcntl.LOCK_EX | fcntl.LOCK_NB)
        OUTPUT.mkdir(parents=True)
        pins = {str(Path(__file__).resolve()):sha(Path(__file__).resolve())}
        def read(path):
            path = Path(path).resolve(strict=True)
            pins[str(path)] = sha(path)
            return json.loads(path.read_text())
        request,launch,started,command_started,durable = [read(JOB/name) for name in (
            'request.json','launch.json','worker-started.json','command-started.json','terminal.json')]
        assert request['schema_version'] == 1
        for value in (request,launch,started,command_started):
            assert value['command'] == EXPECTED and value['cwd'] == str(ROOT)
        assert request['automatic_retry'] is launch['automatic_retry'] is durable['automatic_retry'] is False
        assert request['common_mutex_owned_by_target_command'] is started['common_mutex_owned_by_target_command'] is True
        assert launch['detached_session'] is started['detached_session'] is True
        assert started['inherited_private_lease'] is True
        assert durable['status'] == 'command-exited-zero-awaiting-report-review'
        assert type(durable['exit_code']) is int and durable['exit_code'] == 0
        assert durable['actual_command_started'] is True and durable['stop_signals'] == []
        assert not any(durable.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
        assert started['worker_identity'] == durable['worker_identity']
        assert launch['worker_identity']['pid'] == started['worker_identity']['pid']
        initial_tokens = launch['worker_identity']['listing'].split()
        started_tokens = started['worker_identity']['listing'].split()
        assert initial_tokens[0] == started_tokens[0] and initial_tokens[2:] == started_tokens[2:]
        assert command_started['command_pid'] == durable['command_pid']
        assert command_started['command_identity'] == durable['command_identity']
        assert command_started['command_identity_observation_missing'] is durable['command_identity_observation_missing'] is False
        assert command_started['command_identity']['pid'] == durable['command_pid']
        assert command_started['command_identity']['listing'].split()[7:] == EXPECTED
        for name in ('command.stdout','command.stderr','worker.log'):
            pins[str(JOB/name)] = sha(JOB/name)
        assert (JOB/'command.stderr').read_bytes() == b'' and (JOB/'worker.log').read_bytes() == b''
        stdout = (JOB/'command.stdout').read_text().splitlines()
        assert len(stdout) == 2 and stdout[0] == 'WAIT common mutex: four new directory extraction fixtures'
        summary = json.loads(stdout[1])
        receipt_path = PACKET/'jobs/fixtures-v1/fixture-receipt.json'
        terminal_path = receipt_path.with_name('terminal.json')
        assert summary['receipt'] == str(receipt_path)
        assert summary['receipt_sha256'] == sha(receipt_path) == '6bd2f38bef28c017c45395e05a307cb1005d03e4540ca92b3832754bc6977ad3'
        assert summary['terminal_sha256'] == sha(terminal_path) == 'd4273be3af3161410f39a9125bba453beb25cd7461f5df691bc5652d254ccd3f'
        receipt,terminal = read(receipt_path),read(terminal_path)
        assert summary['status'] == receipt['status'] == terminal['status'] == 'passed-directory-extraction-fixtures'
        assert type(summary['tests_run']) is int and summary['tests_run'] == 4
        assert type(receipt['tests_run']) is int and receipt['tests_run'] == 4
        assert receipt['execution_completed'] is True
        for field in ('expected_ids','discovered_ids','actual_ids'):
            assert len(receipt[field]) == len(set(receipt[field])) == 4 and set(receipt[field]) == set(IDS)
        assert all(receipt[key] == [] for key in ('loader_errors','failures','errors','skips','expected_failures','unexpected_successes'))
        assert terminal['body_error'] is None and terminal['integrity_errors'] == []
        assert terminal['receipt'] == str(receipt_path) and terminal['receipt_sha256'] == summary['receipt_sha256']
        assert receipt['source_pins'] == terminal['source_pins']
        assert receipt['available_evidence_pins'] == terminal['available_evidence_pins']
        assert receipt['inherited_common_mutex'] is terminal['inherited_common_mutex'] is True
        for record in (receipt,terminal):
            for field in ('actual_native_subjects','original_runtime_fixture_reexecutions'):
                assert type(record[field]) is int and record[field] == 0
        freeze = read(CONT/'jobs/prolog-directory-reviewed-freeze-v1/receipt.json')
        assert freeze['status'] == 'root-and-independent-peer-accepted-directory-recovery-source-frozen'
        assert len(freeze['pins']) == 5
        for field in ('source_pins','available_evidence_pins'):
            for path,digest in receipt[field].items():
                assert path not in pins or pins[path] == digest
                assert sha(path) == digest
                pins[path] = digest
        for path,digest in freeze['pins'].items():
            source = Path(path)
            assert source.parent == PACKET and sha(source) == digest
            assert receipt['source_pins'][path] == digest
            snapshot = receipt_path.parent/'source'/source.name
            assert source.read_bytes() == snapshot.read_bytes()
            assert receipt['source_pins'][str(snapshot)] == digest
            assert source.stat().st_mode & 0o222 == snapshot.stat().st_mode & 0o222 == 0
        log = receipt_path.parent/'fixtures.log'
        assert receipt['available_evidence_pins'][str(log)] == sha(log)
        rawlog = log.read_text()
        assert 'Ran 4 tests' in rawlog and rawlog.rstrip().endswith('OK')
        for name in IDS:
            assert name.split('.')[-1] in rawlog
        for path,digest in pins.items(): assert sha(path) == digest
        review = dict(status='passed-independent-directory4-actual-receipt-and-pin-review',
            completed_at=datetime.now(timezone.utc).isoformat(),common_mutex_inode=held.st_ino,
            inherited_common_mutex=True,private_lease_closed=True,durable_terminal=str(JOB/'terminal.json'),
            durable_finished_at=durable['finished_at'],durable_exit_code=0,stop_signals=[],
            fixture_receipt=str(receipt_path),fixture_receipt_sha256=summary['receipt_sha256'],
            fixture_terminal=str(terminal_path),fixture_terminal_sha256=summary['terminal_sha256'],
            exact_actual_ids=receipt['actual_ids'],tests_run=4,actual_native_subjects=0,
            original_runtime_fixture_reexecutions=0,benchmark_reexecutions=0,fixture_reexecutions_in_review=0,
            worker_sha256=freeze['pins'][str(PACKET/'bootstrap_runtime_directory_v1.py')],
            extractor_sha256=freeze['pins'][str(PACKET/'extract_source.py')],
            source_and_evidence_pins=pins,chart_admission=False,native_bootstrap_launched=False)
        save(OUTPUT/'review.json',review)
        for path,digest in pins.items(): assert sha(path) == digest
        save(OUTPUT/'terminal.json',dict(status=review['status'],body_error=None,integrity_errors=[],
            review=str(OUTPUT/'review.json'),review_sha256=sha(OUTPUT/'review.json'),
            common_mutex_inode=held.st_ino,source_and_evidence_pins=pins))
        print(json.dumps(dict(status=review['status'],review=str(OUTPUT/'review.json'),
            review_sha256=sha(OUTPUT/'review.json'),terminal_sha256=sha(OUTPUT/'terminal.json'),
            tests_run=4,pins=len(pins),native_bootstrap_launched=False)),flush=True)
