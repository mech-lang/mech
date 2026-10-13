"""Verify one completed qualification and stage its data in the opened Site.

The existing Site must equal the exact qualification baseline. No benchmark,
formatter, native execution or synthetic observation is produced here.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import statistics

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
JOB = HERE/'owned-jobs/crystal-current-chart4-fixture-and-candidate-v1'
QUALIFIED = HERE/'jobs/native-ready-v1'
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site')
LOCK = Path('/tmp/mech-clbg-timing.lock')
OUTPUT = HERE/'jobs/current-crystal-chart-stage-v1'

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def load(path):
    return json.loads(Path(path).read_bytes())

print('WAIT qualification owner lease: current Crystal chart', flush=True)
with (JOB/'owner.lock').open('a+') as lease:
    fcntl.flock(lease, fcntl.LOCK_EX)
    held, named = os.fstat(lease.fileno()), (JOB/'owner.lock').stat()
    assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
    terminal = load(JOB/'terminal.json')
    assert terminal['status'] == 'command-exited-zero-awaiting-report-review'
    assert terminal['actual_command_started'] is True and type(terminal['exit_code']) is int and terminal['exit_code'] == 0
    assert terminal['stop_signals'] == [] and terminal['command_identity'] and terminal['command_identity_observation_missing'] is False
    assert not any(key in terminal for key in ('error', 'error_type', 'exception_cleanup', 'error_log_write_failure'))
print('WAIT common mutex: verify and stage current Crystal chart', flush=True)
with LOCK.open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    held, named = os.fstat(lock.fileno()), LOCK.stat()
    assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
    assert not OUTPUT.exists()
    OUTPUT.mkdir(parents=True)
    receipt_path = QUALIFIED/'fixture-receipt.json'
    raw_receipt = receipt_path.read_bytes()
    receipt = json.loads(raw_receipt)
    pin = load(QUALIFIED/'terminal-pin.json')
    assert receipt['status'] == pin['status'] == 'passed'
    assert receipt['execution_completed'] is True and receipt['tests_run'] == 4
    assert receipt['actual_native_processes'] == 0 and receipt['live_dataset_mutations'] == 0
    assert receipt['terminal_evidence_complete'] is True and not receipt.get('terminal_pin_errors')
    assert all(receipt[field] == [] for field in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes', 'loader_errors'))
    assert len(set(receipt['expected_test_ids'])) == 4
    assert set(receipt['expected_test_ids']) == set(receipt['discovered_test_ids']) == set(receipt['actual_test_ids'])
    assert len(receipt['discovered_test_ids']) == len(receipt['actual_test_ids']) == 4
    assert pin['receipt'] == str(receipt_path) and pin['receipt_sha256'] == sha(raw_receipt)
    candidate_path = QUALIFIED/'candidate-data.json'
    candidate_raw = candidate_path.read_bytes()
    assert pin['partial_evidence_pins'][str(candidate_path)] == sha(candidate_raw)
    assert pin['partial_evidence_pins'][str(QUALIFIED/'fixtures.log')] == sha((QUALIFIED/'fixtures.log').read_bytes())
    for path, digest in receipt['source_pins'].items():
        assert sha(Path(path).read_bytes()) == digest, 'Changed qualification prerequisite: '+path
    repo_data = CLBG/'overview/explorer/dist/data.json'
    site_data = SITE/'dist/data.json'
    old_raw = repo_data.read_bytes()
    baseline_sha = receipt['source_pins'][str(repo_data)]
    assert sha(old_raw) == sha(site_data.read_bytes()) == baseline_sha, 'Opened Site and exact qualification baseline differ'
    assert load(SITE/'.openai/hosting.json')['project_id'] == 'appgprj_6ac8f49664688191961eac14d8a9465b'
    old, candidate = json.loads(old_raw), json.loads(candidate_raw)
    assert len(candidate['languages']) == len(old['languages'])+1
    assert {row['key'] for row in candidate['languages']} - {row['key'] for row in old['languages']} == {'crystal'}
    for row in old['languages']:
        assert row == next(value for value in candidate['languages'] if value['key'] == row['key'])
    for key, row in old['programs'].items(): assert candidate['programs'][key] == row
    for key, row in old['files'].items(): assert candidate['files'][key] == row
    for row in old['reference_summary']:
        assert row == next(value for value in candidate['reference_summary'] if value['lang'] == row['lang'])
    assert len(candidate['benchmarks']) == len(old['benchmarks']) == 10
    for before, after in zip(old['benchmarks'], candidate['benchmarks']):
        assert {key:value for key,value in before.items() if key != 'groups'} == {key:value for key,value in after.items() if key != 'groups'}
        for group in before['groups']:
            assert group == next(value for value in after['groups'] if value['language'] == group['language'])
        new_groups = [group for group in after['groups'] if group['language'] == 'crystal']
        assert len(new_groups) == 1 and 1 <= len(new_groups[0]['programs']) <= 2
        assert all(candidate['programs'][key]['language'] == 'crystal' for key in new_groups[0]['programs'])
    audit = candidate['native_language_audit']['languages']['crystal']
    assert audit['source_count'] == 20 and audit['actual_full_native_processes'] == 90 and audit['sample_count'] == 3
    forbidden = {'v', 'red'}
    assert not forbidden & {row['key'] for row in candidate['languages']}
    assert not any(row['language'] in forbidden for row in candidate['programs'].values())
    assert not any(group['language'] in forbidden for benchmark in candidate['benchmarks'] for group in benchmark['groups'])
    new_rows = {key:row for key,row in candidate['programs'].items() if key not in old['programs']}
    assert 10 <= len(new_rows) <= 20 and {row['language'] for row in new_rows.values()} == {'crystal'}
    for row in new_rows.values():
        assert len(row['samples']) == 3 and statistics.median(row['samples']) == row['seconds']
        assert row['required_work_validated'] is True and row['loc'] > 0
        assert row['files'] and all(key in candidate['files'] for key in row['files'])
    (OUTPUT/'baseline-data.json').write_bytes(old_raw)
    (OUTPUT/'candidate-data.json').write_bytes(candidate_raw)
    # Both locations receive exactly the already-qualified candidate bytes.
    for path in (repo_data, site_data):
        temporary = path.with_name(path.name+'.current-crystal-stage-v1')
        with temporary.open('xb') as stream:
            stream.write(candidate_raw); stream.flush(); os.fsync(stream.fileno())
        os.replace(temporary, path)
        assert sha(path.read_bytes()) == sha(candidate_raw)
    report = dict(status='verified-qualified-crystal-staged-awaiting-browser-and-publication',
        baseline_sha256=baseline_sha, candidate_sha256=sha(candidate_raw),
        fixture_receipt_sha256=sha(raw_receipt), fixture_tests=4,
        old_programs_preserved=len(old['programs']), new_crystal_endpoints=len(new_rows),
        original_crystal_sources=20, full_process_timing_samples=90,
        existing_baselines_unchanged=True, V_visualization_entries=0, Red_visualization_entries=0,
        common_mutex_held=True, benchmark_runs=0, published=False)
    (OUTPUT/'receipt.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report), flush=True)
