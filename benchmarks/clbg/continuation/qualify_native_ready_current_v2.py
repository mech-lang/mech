"""Locked exact-four fixtures, optionally stage reviewed ready data in a job.

No native execution, benchmark replay, frontend regeneration or live publication.
The optional candidate is an exclusive review artifact, never dist/data.json.
"""
import argparse
from contextlib import contextmanager
from copy import deepcopy
import fcntl
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import traceback

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
EXPLORER = ROOT/'overview/explorer'
LOCK = Path('/tmp/mech-clbg-timing.lock')
FILES = (
    'native_languages_ready_v1.py', 'test_native_languages_ready_v1.py',
    'native_languages_data_v2.py', 'native_platform_metadata_v2.py',
    'native_language_groups_v2.py', 'publication_endpoints_v1.py',
    'test_native_languages_data_v1.py', 'native_languages_data_v1.py')
IDS = tuple('test_native_languages_ready_v1.NativeReadyAdmission.'+name for name in (
    'test_crystal_only_keeps_unchanged_twenty_source_ninety_process_validation',
    'test_partial_unknown_eighty_nine_runtime_and_unqualified_loc_reject',
    'test_pure_subset_platform_and_groups_preserve_prior_cohort_and_controls',
    'test_guarded_late_failures_are_atomic_and_matching_admission_is_idempotent'))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        if sha(path) != digest: raise ValueError('Changed ready qualification input: '+path)


@contextmanager
def common_mutex(adopted_fd):
    """Borrow a duplicate of the coordinator's already held open description."""
    if adopted_fd is None:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            yield lock
        return
    if type(adopted_fd) is not int or adopted_fd < 64:
        raise ValueError('Already held common descriptor must be an integer >=64')
    held, named = os.fstat(adopted_fd), LOCK.stat()
    if (held.st_dev, held.st_ino) != (named.st_dev, named.st_ino):
        raise ValueError('Adopted descriptor is not the named common mutex')
    duplicate = fcntl.fcntl(adopted_fd, fcntl.F_DUPFD, 64)
    try:
        held, named = os.fstat(duplicate), LOCK.stat()
        if (held.st_dev, held.st_ino) != (named.st_dev, named.st_ino):
            raise ValueError('Common mutex identity changed during adoption')
        fcntl.flock(duplicate, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with os.fdopen(duplicate, 'a+') as lock:
            duplicate = None
            yield lock
    finally:
        if duplicate is not None: os.close(duplicate)
    # Never LOCK_UN: this is the coordinator's shared open description.


def retain_program_inventory(candidate, selected):
    """Retain all admitted source forms before chart endpoint pruning."""
    selected = set(selected)
    inventory = deepcopy(candidate.get('program_inventory', {}))
    if not isinstance(inventory, dict):
        raise ValueError('Existing program inventory must be a map')
    benchmarks = {item['key']: item for item in candidate['benchmarks']}
    incoming = {key: deepcopy(row) for key, row in candidate['programs'].items()
                if row['language'] in selected}
    if len(incoming) != 20 * len(selected):
        raise ValueError('Twenty actual source forms per selected language required')
    for key, row in incoming.items():
        benchmark = benchmarks[row['benchmark']]
        row['x'] = row['loc'] / benchmark['baseline_loc']
        row['y'] = row['seconds'] / benchmark['baseline_seconds']
        if key in inventory and inventory[key] != row:
            raise ValueError('Existing source inventory conflicts with actual admission: ' + key)
        inventory[key] = row
    if any(row['language'] in {'v', 'red'} for row in inventory.values()):
        raise ValueError('Excluded language cannot enter source inventory')
    candidate['program_inventory'] = inventory


def run(args):
    output = args.output.resolve()
    if not output.is_relative_to(HERE/'jobs') or output.exists():
        raise ValueError('Exclusive continuation job output required')
    print('WAIT common mutex: four ready-cohort fixtures and optional candidate', flush=True)
    with (HERE/'native-ready-current-v2-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        held, named = os.fstat(owner.fileno()), (HERE/'native-ready-current-v2-owner.lock').stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        with common_mutex(getattr(args, 'common_timing_mutex_fd', None)) as lock:
            held, named = os.fstat(lock.fileno()), LOCK.stat()
            assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
            output.mkdir(parents=True)
            inherited = fcntl.fcntl(lock.fileno(), fcntl.F_DUPFD, 64); os.set_inheritable(inherited, True)
            old_environment = os.environ.get('MECH_CLBG_TIMING_LOCK_FD')
            os.environ['MECH_CLBG_TIMING_LOCK_FD'] = str(inherited)
            pins, result, suite, loader = {}, None, None, None
            receipt = dict(status='failed', expected_test_ids=list(IDS), discovered_test_ids=[], actual_test_ids=[],
                tests_run=0, execution_completed=False, actual_native_processes=0, chart_admission=False,
                source_or_timing_mutations=0, inherited_common_mutex=True, automatic_retry=False)
            error, path_added = None, 0
            try:
                pins[str(Path(__file__).resolve())] = sha(__file__)
                pins[str(Path(sys.executable).resolve())] = sha(sys.executable)
                names = {Path(name).stem for name in FILES}
                assert not any(name in names or name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
                snapshot = output/'snapshot'; snapshot.mkdir()
                for name in FILES:
                    source, target = EXPLORER/name, snapshot/name
                    digest = sha(source); pins[str(source)] = digest
                    shutil.copyfile(source, target); assert sha(target) == digest
                    pins[str(target)] = digest; target.chmod(0o444)
                original_unittest = Path(importlib.util.find_spec('unittest').origin).resolve().parent
                frozen_unittest = output/'python/unittest'; frozen_unittest.mkdir(parents=True)
                for source in sorted(original_unittest.rglob('*.py')):
                    target = frozen_unittest/source.relative_to(original_unittest)
                    target.parent.mkdir(parents=True, exist_ok=True); digest = sha(source)
                    pins[str(source)] = digest; shutil.copyfile(source, target)
                    assert sha(target) == digest; pins[str(target)] = digest; target.chmod(0o444)
                sys.dont_write_bytecode = True
                sys.path.insert(0, str(frozen_unittest.parent)); sys.path.insert(0, str(snapshot)); path_added = 2
                unittest = importlib.import_module('unittest')
                def identities():
                    for name, loaded in list(sys.modules.items()):
                        if name == 'unittest' or name.startswith('unittest.'):
                            path = Path(loaded.__file__).resolve()
                            assert path.is_relative_to(frozen_unittest) and path.suffix == '.py' and sha(path) == pins[str(path)]
                        elif name in names:
                            path = Path(loaded.__file__).resolve()
                            assert path == snapshot/(name+'.py') and sha(path) == pins[str(path)]
                class Recorded(unittest.TextTestResult):
                    def __init__(self, *values, **kwargs):
                        nonlocal result
                        super().__init__(*values, **kwargs); result = self
                    def startTest(self, test):
                        receipt['actual_test_ids'].append(test.id()); super().startTest(test)
                module = importlib.import_module('test_native_languages_ready_v1')
                loader = unittest.TestLoader(); suite = loader.loadTestsFromModule(module)
                def collect(part):
                    if isinstance(part, unittest.TestSuite):
                        for child in part: collect(child)
                    else: receipt['discovered_test_ids'].append(part.id())
                collect(suite)
                receipt['loader_errors'] = list(loader.errors)
                save(output/'discovery.json', dict(discovered_test_ids=receipt['discovered_test_ids'], loader_errors=loader.errors))
                identities(); verify(pins)
                assert not loader.errors and len(receipt['discovered_test_ids']) == len(set(receipt['discovered_test_ids'])) == 4
                assert set(receipt['discovered_test_ids']) == set(IDS)
                with (output/'fixtures.log').open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=Recorded).run(suite)
                receipt['execution_completed'] = True
                assert result.wasSuccessful() and result.testsRun == 4 and len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 4
                assert set(receipt['actual_test_ids']) == set(IDS)
                assert not any((result.failures, result.errors, result.skipped, result.expectedFailures, result.unexpectedSuccesses))
                identities(); verify(pins); receipt['status'] = 'passed'
                if args.packet:
                    assert args.packet_sha256 and args.input_data
                    packet = args.packet.resolve(strict=True); data_path = args.input_data.resolve(strict=True)
                    assert packet.is_relative_to(HERE) and data_path.is_relative_to(ROOT)
                    assert sha(packet) == args.packet_sha256
                    pins[str(packet)] = args.packet_sha256; pins[str(data_path)] = sha(data_path)
                    candidate = json.loads(data_path.read_bytes())
                    ready = sys.modules['native_languages_ready_v1']
                    admission = ready.add_ready_native_languages(ROOT, candidate, packet_path=packet,
                        packet_sha256=args.packet_sha256, common_timing_mutex_fd=inherited, finalize=False)
                    retain_program_inventory(candidate, admission['ready_languages'])
                    endpoint_admission = ready.finalize_ready_endpoints(candidate, admission['ready_languages'])
                    identities(); verify(pins)
                    save(output/'candidate-data.json', candidate)
                    receipt.update(candidate_admission=admission, endpoint_admission=endpoint_admission,
                        candidate_data=str(output/'candidate-data.json'), live_dataset_mutations=0)
            except BaseException as failure:
                error = failure; receipt.update(status='failed', error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
                try:
                    with (output/'exception.log').open('x') as stream: traceback.print_exc(file=stream)
                except BaseException as journal_error: receipt['exception_journal_error'] = str(journal_error)
                raise
            finally:
                if result is not None:
                    receipt.update(tests_run=result.testsRun, failures=[(t.id(), why) for t, why in result.failures],
                        errors=[(t.id(), why) for t, why in result.errors], skips=[(t.id(), why) for t, why in result.skipped],
                        expected_failures=[(t.id(), why) for t, why in result.expectedFailures],
                        unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
                partial, failures = dict(pins), []
                for path in sorted(output.rglob('*')):
                    if path.is_file():
                        try: partial[str(path.resolve())] = sha(path)
                        except BaseException as failure: failures.append(dict(path=str(path), error=str(failure)))
                for path, digest in pins.items():
                    try: assert sha(path) == digest
                    except BaseException as failure: failures.append(dict(path=path, error=str(failure)))
                if failures: receipt.update(status='failed-terminal-integrity', terminal_pin_errors=failures)
                receipt.update(source_pins=pins, partial_evidence_pins=partial, terminal_evidence_complete=not failures)
                try:
                    save(output/'fixture-receipt.json', receipt)
                    save(output/'terminal-pin.json', dict(status=receipt['status'], receipt=str(output/'fixture-receipt.json'),
                        receipt_sha256=sha(output/'fixture-receipt.json'), partial_evidence_pins=partial,
                        inherited_common_mutex=True, chart_admission=False, actual_native_processes=0))
                finally:
                    for _ in range(path_added): sys.path.pop(0)
                    if old_environment is None: os.environ.pop('MECH_CLBG_TIMING_LOCK_FD', None)
                    else: os.environ['MECH_CLBG_TIMING_LOCK_FD'] = old_environment
                    os.close(inherited)
                if failures and error is None: raise RuntimeError('Ready qualification terminal integrity failed')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--packet', type=Path); parser.add_argument('--packet-sha256'); parser.add_argument('--input-data', type=Path)
    parser.add_argument('--common-timing-mutex-fd', type=int)
    args = parser.parse_args()
    if args.common_timing_mutex_fd is not None and args.common_timing_mutex_fd < 64:
        parser.error('Adopted common timing mutex FD must be >=64')
    if bool(args.packet) != bool(args.packet_sha256) or bool(args.packet) != bool(args.input_data):
        parser.error('Packet, reviewed SHA and input-data must be supplied together')
    run(args)


if __name__ == '__main__': main()
