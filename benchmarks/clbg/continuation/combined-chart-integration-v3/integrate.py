"""Crystal20/legacy133 integration over actual Nim baseline; no native/live writes."""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
FMT = CLBG/'loc-normalization/20261013/formatted-loc-v1'
HISTORY = CLBG/'continuation/existing-native-platform-proposal-v1'
C3_BASE = CLBG/'continuation/jobs/c3-current-candidate-v1/candidate-data.json'
C3_BASE_SHA = '4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947'
CLO = CLBG/'continuation/jobs/clojure-current-candidate-v1'
CLO_OWNER = CLBG/'continuation/owned-jobs/clojure-current-candidate-v1'
CLO_REVIEW = CLBG/'continuation/jobs/clojure-current-candidate-closed-review-v1/review.json'
CLO_DATA = CLO/'candidate-data.json'
CLO_SHA = 'b088279074e85ec149466f876bdce49d4d497f7784a0d48be58e023e5b2c6a13'
NIM = CLBG/'continuation/jobs/nim-current-candidate-v1'
NIM_OWNER = CLBG/'continuation/owned-jobs/nim-current-candidate-v1'
NIM_REVIEW = CLBG/'continuation/jobs/nim-current-candidate-closed-review-v1/review.json'
CURRENT = NIM/'candidate-data.json'
CURRENT_SHA = '9d9eba51742dcf1afc76c0a45fa4125524197ed06ae540a1d142769fd859f60a'
NIM_OUTCOME_SHA = '55f8f8eb153968cc1e623238f8c2dedf0c82e87486fa3071473908b7afc6f47c'
NIM_TERMINAL_SHA = 'f051d0a86c8016ea80a7c82fcbd0952ea5b57fb1f48f9c208bee5f1c180b3088'
NIM_REUSED_SHA = '5c3dc71dbba705d53e2ccd6bcaff68a6a799721251154dfbfea33801e8891337'
NIM_WORKER = CLBG/'continuation/qualify_nim_current_candidate_v1.py'
NIM_WORKER_SHA = 'd116ce31b3ab32629be95026d03cff96d3ee63505ee69e2bc8047693027f4106'
CLO_OUTCOME_SHA = '4f915b966b2818ccc0260d7592de9a168b248f609702d47bd314d594d072a5cc'
CLO_TERMINAL_SHA = '01edefc859d80c556dfd879986323992aa806c542630b24761eab51fdcc2e6b2'
CLO_REUSED_SHA = 'b81c9095a76d2d10e8b24d65b6d0037419028b2f1f3e79d3966f75a7095bc123'
CLO_WORKER = CLBG/'continuation/qualify_clojure_current_candidate_v1.py'
CLO_WORKER_SHA = '48e3f61117396d1aef02257273cb6b1e43dd4b3ec23acfbb5702592e7a8afdc7'
CLO_PACKET_SHA = '050d460200926dc7c01c9b721c8f6f34d71c340d8388bfa736f71346eb82e509'
CLO_NOTE_SHA = '48b112d3f3641c9ed543577229e52f109ca2cd190661c6b0bc88f82f16540873'
CLO_METADATA_SHA = '8bf96ed71bd799b80fb6f062ff7b5b76c92d6e992e3e40ef307811f3c2c1b9ff'
FMT_BASE = CLBG/'continuation/jobs/current-chart-session-v2/candidate-data.json'
FMT_BASE_SHA = '27b1c792b253d3d465ca6f853e7f85c9d089cb87e3b161a634feb7b5c1f69c8e'
OLD = CLBG/'continuation/jobs/platform-metadata-qualification-v2/snapshot/4-data.json'
OLD_SHA = '65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f'
FQ = FMT/'jobs/crystal20-format-v1/qualification.json'
FQ_SHA = 'cda8ac6271fee4ae3f37340cc80b17ba3aa98b6582d5c980270923a0abbb382a'
FT = FQ.parent/'terminal.json'
FT_SHA = '0434c52404fa85f863aa0c2167c32981182b166a68dd8e9c1b00a96156237b49'
MODULES = {'loader': (FMT/'load_evidence.py', 'c473d48ee8065983305617c4622a7e21cd31c461889794f5b4b55449b9202a47'),
           'overlay': (FMT/'overlay.py', 'ddc49cb21ad03dda6dee697ce1511617239364b0d5f2c23b9a5f9e637d14d084'),
           'endpoints': (FMT/'endpoints.py', '96724c3ff701fabf7ea262520cacd8951b15948500cfe934efb21a1a129474bf'),
           'adapter': (HISTORY/'adapter.py', '90b6052eaa14f960a6fb2510364335cf87f43d4d62998a015223db42ae516342')}
FORMAT_FIELDS = {'formatted_loc', 'formatted_loc_status', 'formatted_loc_complete', 'formatted_loc_evidence', 'formatted_x'}
METADATA = {'measured_backend', 'prior_runtime_metadata', 'runtime_policy', 'execution_mode'}


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path, digest, pins):
    path = Path(path).resolve(strict=True); actual = sha(path)
    assert actual == digest and (str(path) not in pins or pins[str(path)] == actual), ('Changed pinned input', str(path))
    pins[str(path)] = actual; return json.loads(path.read_bytes())


def verify(pins):
    for path, digest in pins.items(): assert sha(path) == digest, ('Changed input/output pin', path)


def absorb(pins, values):
    for path, digest in values.items():
        assert type(path) is str and Path(path).is_absolute() and '..' not in Path(path).parts and (path not in pins or pins[path] == digest)
        assert sha(path) == digest; pins[path] = digest


def save(path, value):
    with path.open('x') as stream: json.dump(value, stream, indent=2); stream.write('\n')


def fresh(name, path, digest, pins):
    assert name not in sys.modules and sha(path) == digest
    pins[str(path)] = digest; source = path.read_bytes(); assert hashlib.sha256(source).hexdigest() == digest
    spec = importlib.util.spec_from_file_location(name, path); module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module; exec(compile(source, str(path), 'exec'), module.__dict__)
    assert sys.modules[name] is module and Path(module.__file__).resolve() == path; return module


def clojure_baseline(plan, pins):
    # These already completed inputs never consume a mutable live dataset.
    # Plan fields are actual root-captured digests, not future evidence claims.
    review = read(CLO_REVIEW, plan['clojure_review_sha256'], pins)
    assert review['status'] == 'passed-closed-Clojure-candidate-review'
    assert review['candidate_sha256'] == CLO_SHA and review['previous_baseline_sha256'] == C3_BASE_SHA
    assert review['terminal_sha256'] == CLO_TERMINAL_SHA and review['outcome_sha256'] == CLO_OUTCOME_SHA
    assert type(review['verified_pins']) is int and review['verified_pins'] == 5276
    assert review['inventory_sources'] == 20 and review['candidate_samples'] == 60 and review['clojure_endpoints'] == 18
    assert review['original_full_processes'] == 90 and review['prior_records_preserved'] is True
    assert review['benchmark_runs'] == review['fixture_reexecutions'] == 0
    with (CLO_OWNER/'owner.lock').open('r') as lease:
        fcntl.flock(lease, fcntl.LOCK_SH | fcntl.LOCK_NB)
        held, named = os.fstat(lease.fileno()), Path(lease.name).stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        names = ('request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json')
        assert set(plan['clojure_owner_pins']) == set(names) and plan['clojure_owner_pins']['terminal.json'] == CLO_TERMINAL_SHA
        request, launch, worker, start, done = (read(CLO_OWNER/name, plan['clojure_owner_pins'][name], pins) for name in names)
        argv = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
                '-I', '-S', '-B', '-u', str(CLO_WORKER), '--worker-sha256', CLO_WORKER_SHA,
                '--packet-sha256', CLO_PACKET_SHA, '--metadata-sha256', CLO_METADATA_SHA, '--review-sha256', CLO_NOTE_SHA]
        assert request['command'] == launch['command'] == worker['command'] == start['command'] == argv
        assert request['cwd'] == launch['cwd'] == worker['cwd'] == start['cwd'] == str(CLBG.parents[1])
        assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
        assert worker['inherited_private_lease'] is worker['detached_session'] is True
        assert worker['worker_identity'] == done['worker_identity'] and launch['worker_identity']['pid'] == worker['worker_identity']['pid']
        assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid']
        assert start['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
        assert done['status'] == 'command-exited-zero-awaiting-report-review' and type(done['exit_code']) is int and done['exit_code'] == 0
        assert done['actual_command_started'] is True and done['stop_signals'] == [] and done['automatic_retry'] is done['qualification_granted'] is False
        outcome = read(CLO/'outcome.json', CLO_OUTCOME_SHA, pins)
        assert outcome['status'] == 'qualified-current-Clojure-candidate-awaiting-root-stage-review'
        assert outcome['common_mutex_held'] is True and outcome['common_timing_mutex'] == str(LOCK) and outcome['chart_admission'] is False
        assert all(type(outcome[key]) is int and outcome[key] == 0 for key in ('native_commands', 'fixture_reexecutions', 'live_dataset_mutations'))
        assert not any(outcome.get(key) for key in ('error', 'error_type', 'terminal_pin_errors', 'available_bytes_not_fully_pinned', 'error_log_write_failure'))
        assert outcome['pins'][str(C3_BASE)] == C3_BASE_SHA and outcome['available_evidence_pins'][str(CLO_DATA)] == CLO_SHA
        assert outcome['available_evidence_pins'][str(CLO/'input-data.json')] == C3_BASE_SHA
        assert outcome['available_evidence_pins'][str(CLO/'reused-qualification.json')] == CLO_REUSED_SHA
        absorb(pins, outcome['pins']); absorb(pins, outcome['available_evidence_pins'])
        prior = read(C3_BASE, C3_BASE_SHA, pins); current = read(CLO_DATA, CLO_SHA, pins)
        reused = read(CLO/'reused-qualification.json', CLO_REUSED_SHA, pins)
        assert reused['status'] == 'actual-original-four-reused-for-Clojure-without-fixture-reexecution'
        assert reused['published_baseline'] == dict(path=str(C3_BASE), sha256=C3_BASE_SHA, immutable_new_input=str(CLO/'input-data.json'))
        assert reused['original_fixture_methods_passed'] == 4 and reused['new_fixture_methods_executed'] == reused['live_dataset_mutations'] == 0
        assert reused['retained_Clojure_source_inventory'] == 20 and reused['old_programs_files_inventory_baselines_groups_and_summaries_unchanged'] is True
        assert reused['admission']['ready_languages'] == ['clojure'] and reused['admission']['actual_full_native_processes'] == 90
        assert reused['admission']['source_count'] == 20 and reused['admission']['samples_per_source'] == 3
        assert outcome['pins'][str(CLO_WORKER)] == CLO_WORKER_SHA
        proof = fresh('completed_clojure_preservation_proof_v1', CLO_WORKER, CLO_WORKER_SHA, pins)
        proof.preserve(prior, current)  # Existing pure proof; never its run/main.
        inventory = [row for row in current['program_inventory'].values() if row['language'] == 'clojure']
        assert len(inventory) == 20 and sum(len(row['samples']) for row in inventory) == 60 and all(len(row['samples']) == 3 for row in inventory)
        assert sum(row['language'] == 'clojure' for row in current['programs'].values()) == 18
        verify(pins); return current


def nim_baseline(plan, pins):
    prior = clojure_baseline(plan, pins)
    review = read(NIM_REVIEW, plan['nim_review_sha256'], pins)
    assert review['status'] == 'passed-closed-Nim-candidate-review' and review['verified_pins'] == 41944
    assert review['candidate_sha256'] == CURRENT_SHA and review['previous_baseline_sha256'] == CLO_SHA
    assert review['terminal_sha256'] == NIM_TERMINAL_SHA and review['outcome_sha256'] == NIM_OUTCOME_SHA
    assert review['inventory_sources'] == 20 and review['candidate_samples'] == 60 and review['nim_endpoints'] == 20
    assert review['original_full_processes'] == 90 and review['prior_records_preserved'] is True
    assert review['benchmark_runs'] == review['fixture_reexecutions'] == 0
    with (NIM_OWNER/'owner.lock').open('r') as lease:
        fcntl.flock(lease, fcntl.LOCK_SH | fcntl.LOCK_NB)
        held, named = os.fstat(lease.fileno()), Path(lease.name).stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        names = ('request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json')
        assert set(plan['nim_owner_pins']) == set(names) and plan['nim_owner_pins']['terminal.json'] == NIM_TERMINAL_SHA
        request, launch, worker, start, done = (read(NIM_OWNER/name, plan['nim_owner_pins'][name], pins) for name in names)
        argv = ['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12',
            '-I', '-S', '-B', '-u', str(NIM_WORKER), '--worker-sha256', NIM_WORKER_SHA,
            '--packet-sha256', '22670b76b9801c9b7c7b09dc7077584011863ce0aace268e49c6f6e40d056d8a',
            '--metadata-sha256', '98efdbdfcdaacbf22c0644e4225b39e29b68ca94db4d392a3cdd436376ce3722',
            '--review-sha256', 'a2950716e19a0c853b0148618f83713adc76da3403d5c3f60f3c5719103f5d0a',
            '--metadata-review-sha256', '1082d7709da62a12a4da3f5def71c39e647d1fa1c05fccb29885c34b34b4bb68',
            '--baseline-review-sha256', 'd18f446c20bda8b3fe37c966084cb410e27e9bfcea3ea5ebf3d1290aeea94667']
        assert plan['clojure_review_sha256'] == argv[-1]
        assert request['command'] == launch['command'] == worker['command'] == start['command'] == argv
        assert request['cwd'] == launch['cwd'] == worker['cwd'] == start['cwd'] == str(CLBG.parents[1])
        assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
        assert worker['inherited_private_lease'] is worker['detached_session'] is True
        assert worker['worker_identity'] == done['worker_identity'] and launch['worker_identity']['pid'] == worker['worker_identity']['pid'] == 29030
        assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid'] == 29050
        assert start['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
        assert done['status'] == 'command-exited-zero-awaiting-report-review' and type(done['exit_code']) is int and done['exit_code'] == 0
        assert done['actual_command_started'] is True and done['stop_signals'] == [] and done['automatic_retry'] is done['qualification_granted'] is False
        observed = done['command_identity']['listing'].split(None,7)
        assert observed[:2] == ['29050','29030'] and observed[7].split() == argv
        outcome = read(NIM/'outcome.json', NIM_OUTCOME_SHA, pins)
        assert outcome['status'] == 'qualified-current-Nim-candidate-awaiting-root-stage-review'
        assert outcome['common_mutex_held'] is True and outcome['common_timing_mutex'] == str(LOCK) and outcome['chart_admission'] is False
        assert all(type(outcome[key]) is int and outcome[key] == 0 for key in ('native_commands', 'fixture_reexecutions', 'live_dataset_mutations'))
        assert not any(outcome.get(key) for key in ('error', 'error_type', 'terminal_pin_errors', 'available_bytes_not_fully_pinned', 'error_log_write_failure'))
        assert outcome['pins'][str(CLO_DATA)] == CLO_SHA and outcome['available_evidence_pins'][str(CURRENT)] == CURRENT_SHA
        assert outcome['available_evidence_pins'][str(NIM/'input-data.json')] == CLO_SHA
        assert outcome['available_evidence_pins'][str(NIM/'reused-qualification.json')] == NIM_REUSED_SHA
        absorb(pins, outcome['pins']); absorb(pins, outcome['available_evidence_pins'])
        current = read(CURRENT, CURRENT_SHA, pins)
        reused = read(NIM/'reused-qualification.json', NIM_REUSED_SHA, pins)
        assert reused['status'] == 'actual-original-four-reused-for-Nim-without-fixture-reexecution'
        assert reused['published_baseline'] == dict(path=str(CLO_DATA), sha256=CLO_SHA, immutable_new_input=str(NIM/'input-data.json'))
        assert reused['original_fixture_methods_passed'] == 4 and reused['new_fixture_methods_executed'] == reused['live_dataset_mutations'] == 0
        assert reused['retained_Nim_source_inventory'] == 20 and reused['old_programs_files_inventory_baselines_groups_and_summaries_unchanged'] is True
        assert reused['admission']['ready_languages'] == ['nim'] and reused['admission']['actual_full_native_processes'] == 90
        assert reused['admission']['source_count'] == 20 and reused['admission']['samples_per_source'] == 3
        assert outcome['pins'][str(NIM_WORKER)] == NIM_WORKER_SHA
        proof = fresh('completed_nim_preservation_proof_v1', NIM_WORKER, NIM_WORKER_SHA, pins)
        proof.preserve(prior, current)  # Existing pure proof, never run/main.
        inventory = [row for row in current['program_inventory'].values() if row['language'] == 'nim']
        assert len(inventory) == 20 and sum(len(row['samples']) for row in inventory) == 60 and all(len(row['samples']) == 3 for row in inventory)
        assert sum(row['language'] == 'nim' for row in current['programs'].values()) == 20
        verify(pins); return current


def closed_history(plan, pins, lease, adapter):
    section = plan['history']; owner = Path(section['owner']).resolve(strict=True)
    assert owner == CLBG/'continuation/owned-jobs/legacy-platform133-literal-v4-qualification'
    assert owner.parent == CLBG/'continuation/owned-jobs' and lease.name == str(owner/'owner.lock')
    held, named = os.fstat(lease.fileno()), Path(lease.name).stat(); assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
    fcntl.flock(lease, fcntl.LOCK_SH | fcntl.LOCK_NB)
    expected_names = {'request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json'}
    assert set(section['owner_pins']) == expected_names
    assert section['owner_pins']['terminal.json'] == 'cc482ff23ddd295b22613f76b06c6344c5d0f5bc2fac8b25b173e3dc22bd9e1b'
    values = {name: read(owner/name, digest, pins) for name, digest in section['owner_pins'].items()}
    request, launch, worker, start, done = (values[name] for name in ('request.json', 'launch.json', 'worker-started.json', 'command-started.json', 'terminal.json'))
    assert request['command'] == launch['command'] == worker['command'] == start['command'] == section['command']
    assert request['cwd'] == launch['cwd'] == worker['cwd'] == start['cwd'] == str(CLBG.parents[1])
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['worker_identity'] == done['worker_identity'] and launch['worker_identity']['pid'] == worker['worker_identity']['pid']
    assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid']
    assert start['command_identity_observation_missing'] is done['command_identity_observation_missing'] is False
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert done['status'] == 'command-exited-zero-awaiting-report-review' and type(done['exit_code']) is int and done['exit_code'] == 0
    assert done['actual_command_started'] is True and done['stop_signals'] == [] and done['automatic_retry'] is done['qualification_granted'] is False
    qpath, rpath, tpath = (Path(section[name]['path']).resolve(strict=True) for name in ('qualification', 'receipt', 'terminal'))
    assert qpath.name == 'qualified-history.json' and rpath == qpath.parent/'qualification-receipt.json' and tpath == qpath.parent/'terminal.json'
    assert qpath.parent.parent == HISTORY/'jobs'
    assert qpath.parent.name == 'history-literal-routes-v4'
    assert section['qualification']['sha256'] == 'cfea973a22951b7efef504ba48f84b07745397e7dc8df71161b3a01182b026b9'
    assert section['receipt']['sha256'] == '0030b48dffa6e867a69035b2e12c506e947eab72ea58010ac93a2f14460dfca0'
    assert section['terminal']['sha256'] == '9afe129d9563f8ec27809c509259b7190d014c26cd983139ff36126d8d0ca388'
    q, receipt, terminal = (read(section[name]['path'], section[name]['sha256'], pins) for name in ('qualification', 'receipt', 'terminal'))
    assert terminal['status'] == 'completed-history-qualification' and terminal['body_error'] is terminal['integrity_error'] is None
    assert terminal['available_bytes_not_fully_pinned'] is False and terminal['exclusive_common_mutex_held'] is True and terminal['common_timing_mutex'] == str(LOCK)
    assert terminal['pins'][str(qpath)] == section['qualification']['sha256'] and terminal['pins'][str(rpath)] == section['receipt']['sha256']
    assert all(terminal['pins'][path] == digest for path, digest in q['pins'].items())
    absorb(pins, terminal['pins']); absorb(pins, q['pins'])
    assert q['status'] == receipt['status'] and q['status'] in adapter.STATUS
    assert q['profile_count'] == receipt['profile_count'] == 133 and q['baseline'] == dict(path=str(OLD), sha256=OLD_SHA)
    assert receipt['qualified_history'] == str(qpath) and receipt['qualified_history_sha256'] == section['qualification']['sha256']
    assert q['actual_common_mutex_held'] is receipt['actual_common_mutex_held'] is receipt['pins_unchanged_at_end'] is True
    assert all(type(value[name]) is int and value[name] == 0 for value in (terminal, receipt)
               for name in ('actual_native_processes', 'runtime_version_reruns', 'benchmark_replays'))
    assert terminal['chart_admission'] is receipt['chart_admission'] is receipt['dataset_mutated'] is receipt['sources_mutated'] is receipt['timings_mutated'] is False
    review = read(CLBG/'continuation/jobs/legacy-platform133-v4-qualification-closed-review-v1/receipt.json', plan['history_review_sha256'], pins)
    assert review['status'] == 'root-reviewed-qualified-platform-history133' and review['verified_pins'] == 445
    assert review['qualified_history_sha256'] == section['qualification']['sha256'] and review['qualification_receipt_sha256'] == section['receipt']['sha256']
    assert review['body_terminal_sha256'] == section['terminal']['sha256'] and review['durable_terminal_sha256'] == section['owner_pins']['terminal.json']
    assert review['profiles'] == 133 and review['qualified_platform_rows'] == q['qualified_platform_rows'] == 120
    assert review['qualified_execution_rows'] == q['qualified_execution_rows'] == 97
    assert review['qualified_historical_byte_rows'] == q['qualified_historical_byte_rows'] == 114
    assert review['prior115_annotations_preserved'] is review['all133_measurement_identities_preserved'] is True
    assert review['native_processes'] == review['benchmark_runs'] == 0 and review['chart_admission'] is False
    assert {key: fact['missing_evidence'] for key, fact in q['annotations'].items() if fact['verified'] is False} == review['pending']
    assert len(review['pending']) == 13
    fixture = q['fixture']; argv = section['command']
    assert fixture == dict(path=str(HISTORY/'jobs/fixtures-literal-routes-v4/fixture-receipt.json'),
        sha256='30f59901c3843dab14940059b8a2301d6032b9f63f74d3811b0c2fcd17fde31f',
        terminal_sha256='1407ec3f46333b806ba7a965e3b65588c30d731d0aef902bfed99ea7a3877472')
    assert argv == ['/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-u', str(CLBG/'continuation/legacy-platform-history-repair-v4/replay.py'),
        '--worker-sha256', '4915eff8e3c27a86a7708e319c35bd87b5f640be0b80ccf510241e0fa12d413a',
        '--routes-sha256', '8a5abe0438d5e1d8d3bb5599a98e3d58cf5556691b4153762f4b81ff501448d7',
        '--tests-sha256', 'bbb61a92d49901dc4463ff56497b7499f5efb7e432e1e71139550cf78f29bead',
        '--review-sha256', 'e5f09202bdeff50e7bebc916bbf9df9b6e375d64632496d54223ca0908f6adbf',
        '--closed-review-sha256', '6f798421a3d6e0191f9169218ff0f411b2fed13a31e323b03bd42446f50c35f0',
        '--stage', 'qualify', '--revision', 'history-literal-routes-v4', '--fixture-receipt', fixture['path'],
        '--fixture-sha256', fixture['sha256'], '--fixture-terminal-sha256', fixture['terminal_sha256']]
    observed = done['command_identity']['listing'].split(None,7)
    assert observed[:2] == ['28034','28030'] and observed[7].split() == argv
    for backend in q['annotations'].values():
        runtime = backend['evidence']['runtime_receipt']
        if runtime['verified']:
            assert adapter.relative(runtime['report']) and q['provenance'][runtime['report']] == runtime['report_sha256']
            document = read(CLBG/runtime['report'], runtime['report_sha256'], pins)
            value = document
            for token in runtime['field'].split('/')[1:]:
                token = token.replace('~1', '/').replace('~0', '~'); value = value[int(token)] if type(value) is list else value[token]
            assert (value if runtime['value_encoding'] == 'literal-saved-string' else json.dumps(value, sort_keys=True)) == runtime['value']
    return q


def preserve(old, data, overlay, changes):
    full = overlay.catalog(old); added = FORMAT_FIELDS | METADATA
    assert set(data['program_inventory']) == set(old['program_inventory']) and set(old['programs']) <= set(data['programs']) <= set(full)
    assert overlay.exact_value(data['files'], old['files']) and len(old['benchmarks']) == len(data['benchmarks'])
    for pool in ('programs', 'program_inventory'):
        for key, row in data[pool].items():
            prior = old[pool].get(key, full[key])
            assert overlay.exact_value({k:v for k,v in row.items() if k not in added}, {k:v for k,v in prior.items() if k not in added})
            assert all(overlay.exact_value(row.get(field), changes[key][field]) for field in METADATA) if key in changes else all(
                (field in row) == (field in prior) and (field not in row or overlay.exact_value(row[field], prior[field])) for field in METADATA)
            if row['language'] != 'crystal':
                assert row['formatted_loc_status'] == 'pending' and row['formatted_loc'] is row['formatted_x'] is None and row['formatted_loc_complete'] is False and row['formatted_loc_evidence'] == {}
    for before, after in zip(old['benchmarks'], data['benchmarks']):
        assert overlay.exact_value({k:v for k,v in before.items() if k != 'groups'}, {k:v for k,v in after.items() if k not in ('groups', 'baseline_formatted_loc')})
        assert len(before['groups']) == len(after['groups'])
        for a, b in zip(before['groups'], after['groups']):
            assert overlay.exact_value({k:v for k,v in a.items() if k != 'programs'}, {k:v for k,v in b.items() if k not in ('programs', 'formatted_fast', 'formatted_small')})
            assert [key for key in b['programs'] if key in a['programs']] == a['programs']
    assert overlay.exact_value(old['languages'], [{k:v for k,v in row.items() if not k.startswith('formatted_')} for row in data['languages']])
    special = {'programs', 'program_inventory', 'files', 'benchmarks', 'languages', 'provenance', 'formatted_reference_summary'}
    assert overlay.exact_value({k:v for k,v in old.items() if k not in special}, {k:v for k,v in data.items() if k not in special})
    assert all(data['provenance'][path] == digest for path, digest in old['provenance'].items())
    assert not any(row['language'].casefold() in ('v', 'red') for row in overlay.catalog(data).values())


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--plan-sha256', required=True); parser.add_argument('--self-sha256', required=True); args = parser.parse_args()
    with LOCK.open('a+') as common:
        fcntl.flock(common, fcntl.LOCK_EX); held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino) and held.st_ino == 79859134
        pins = {}; plan = read(args.plan, args.plan_sha256, pins)
        assert plan['schema_version'] == 1 and re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', plan['revision']) and sha(__file__) == args.self_sha256
        absorb(pins, {str(Path(__file__).resolve()): args.self_sha256, str(HERE/'REVIEW.md'): plan['review_sha256']})
        output = HERE/'jobs'/plan['revision']; output.parent.mkdir(exist_ok=True); output.mkdir()
        body = None; completed = False; candidate = None
        try:
            sys.dont_write_bytecode = True
            old = nim_baseline(plan, pins); base = read(FMT_BASE, FMT_BASE_SHA, pins); legacy = read(OLD, OLD_SHA, pins)
            fixture = read(FMT/'jobs/fixtures23-v1/fixture-receipt.json', '7567ddfb5a03df5ed4813dd7d87bfd48b8f300ef0a5be66c90504d377fca81b5', pins)
            fixture_terminal = read(FMT/'jobs/fixtures23-v1/terminal.json', '5b3da339f1f8e575d34742f119996b684b40e03e32365cad749e2a23a61b94bd', pins)
            assert fixture['status'] == fixture_terminal['status'] == 'passed-formatted-loc-fixtures' and fixture['tests_run'] == 23
            assert fixture_terminal['body_error'] is None and fixture_terminal['integrity_errors'] == [] and fixture_terminal['available_bytes_not_fully_pinned'] is False
            assert all(len(fixture[name]) == len(set(fixture[name])) == 23 for name in ('expected_test_ids', 'actual_test_ids', 'discovered_test_ids'))
            assert sorted(fixture['expected_test_ids']) == sorted(fixture['actual_test_ids']) == sorted(fixture['discovered_test_ids'])
            assert all(fixture[name] == [] for name in ('failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'))
            assert fixture['inherited_common_mutex'] is fixture_terminal['inherited_common_mutex'] is True
            assert fixture['chart_admission'] is fixture_terminal['chart_admission'] is False
            assert all(type(document[field]) is int and document[field] == 0 for document in (fixture, fixture_terminal)
                       for field in ('actual_native_subjects', 'actual_formatter_processes'))
            absorb(pins, fixture_terminal['pins'])
            python = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12')
            assert Path(sys.executable).resolve() == python.resolve() and sys.version_info[:2] == (3, 12) and sha(python) == fixture_terminal['pins'][str(python)]
            assert all(fixture_terminal['pins'][str(path)] == digest for path, digest in MODULES.values() if path.parent == FMT)
            modules = {name: fresh(name, *spec, pins) for name, spec in MODULES.items()}
            loader, overlay, endpoints, adapter = (modules[name] for name in ('loader', 'overlay', 'endpoints', 'adapter'))
            with Path(plan['history']['owner']).joinpath('owner.lock').open('r') as lease:
                history = closed_history(plan, pins, lease, adapter)
                formatted = loader.load(FQ, FQ_SHA, FT, FT_SHA, common.fileno()); absorb(pins, formatted['pins'])
                review = read(CLBG/'continuation/jobs/formatted-crystal20-actual-review-v1/receipt.json', plan['crystal_review_sha256'], pins)
                assert review['status'] == 'actual-Crystal20-formatter-receipts-reviewed-awaiting-root-admission'
                assert review['qualification'] == dict(path=str(FQ), sha256=FQ_SHA) and review['terminal'] == dict(path=str(FT), sha256=FT_SHA)
                assert review['profiles'] == review['qualified_profiles'] == review['formatted_file_receipts'] == 20
                assert review['all_original_source_pins_preserved'] is review['baseline_unchanged'] is review['pure_stage_accepted'] is True
                assert review['actual_application_processes'] == review['fixture_reexecutions'] == 0
                assert len(formatted['profile_keys']) == 20 and set(formatted['profile_keys']) == {k for k,v in overlay.catalog(old).items() if v['language'] == 'crystal'}
                candidate = deepcopy(old); fmt_result = overlay.apply(candidate, dict(data=base, path=str(FMT_BASE), sha256=FMT_BASE_SHA), formatted)
                assert fmt_result['updated_programs'] == fmt_result['qualified_programs'] == 20
                changes, provenance = adapter.stage(candidate, legacy, history); shadow = deepcopy(candidate)
                for key in changes:
                    if key in shadow['program_inventory']: shadow['programs'][key] = deepcopy(shadow['program_inventory'][key])
                second, second_provenance = adapter.stage(shadow, legacy, history)
                assert overlay.exact_value(changes, second) and overlay.exact_value(provenance, second_provenance)
                for pool in ('programs', 'program_inventory'):
                    for key in changes.keys() & candidate[pool].keys(): candidate[pool][key].update(deepcopy(changes[key]))
                candidate['provenance'] = provenance
                assert len(old['programs']) == 798 and len(old['program_inventory']) == 80
                full_annotated = deepcopy(overlay.catalog(candidate))
                endpoints.finalize_endpoints(candidate); final_catalog = overlay.catalog(candidate)
                candidate['files'] = deepcopy(old['files'])
                for key in old['programs']:
                    assert full_annotated[key]['language'] != 'crystal' or key in final_catalog
                    row = deepcopy(final_catalog[key] if key in final_catalog else full_annotated[key])
                    if row['language'] != 'crystal':
                        row.update(formatted_loc=None, formatted_loc_status='pending', formatted_loc_complete=False,
                                   formatted_loc_evidence={}, formatted_x=None)
                    candidate['programs'][key] = row
                for before, after in zip(old['benchmarks'], candidate['benchmarks']):
                    for a, b in zip(before['groups'], after['groups']): b['programs'] = list(dict.fromkeys([*a['programs'], *b['programs']]))
                # Canonical selectors/coordinates are proved below, so retain
                # original summary rounding/audit and every archived source.
                for field in ('reference_summary', 'files', 'interpreter_integration_audit'):
                    if field in old: candidate[field] = deepcopy(old[field])
                preserve(old, candidate, overlay, changes); absorb(pins, formatted['provenance']); verify(pins)
                path = output/'candidate-data.json'; save(path, candidate); pins[str(path)] = sha(path)
                save(output/'receipt.json', dict(status='qualified-combined-metadata-candidate', candidate=dict(path=str(path), sha256=pins[str(path)]),
                    input=dict(path=str(CURRENT), sha256=CURRENT_SHA), formatted=fmt_result, legacy_programs=len(changes),
                    prior_C3_baseline=dict(path=str(C3_BASE), sha256=C3_BASE_SHA), prior_Clojure_baseline=dict(path=str(CLO_DATA), sha256=CLO_SHA),
                    actual_Clojure_and_Nim_baseline_closures_verified=True,
                    sources_timings_coordinates_baselines_preserved=True, actual_native_processes=0, fixture_reexecutions=0,
                    dataset_mutated=False, live_files_written=False, chart_admission=False, common_mutex_held=True, pins=pins))
                completed = True
        except BaseException as error:
            body = dict(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            with (output/'error.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            failures, actual = [], {}
            for path, digest in pins.items():
                try:
                    actual[path] = sha(path); assert actual[path] == digest
                except BaseException as error: failures.append(dict(path=path, error=str(error) or type(error).__name__))
            for path in output.iterdir():
                if path.is_file():
                    try: actual[str(path)] = sha(path)
                    except BaseException as error: failures.append(dict(path=str(path), error=str(error) or type(error).__name__))
            save(output/'terminal.json', dict(status='failed-integrity' if failures else 'completed' if completed else 'failed-stop',
                body_error=body, integrity_errors=failures, available_bytes_not_fully_pinned=bool(failures), pins=actual,
                common_mutex_held=True, actual_native_processes=0, fixture_reexecutions=0, chart_admission=False))
            if failures: raise RuntimeError('Integration input/output integrity failed; retained candidate is not admitted')


if __name__ == '__main__': main()
