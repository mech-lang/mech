"""Four transactional platform-overlay fixtures under the common timing lock.

Qualifies the unhooked adapter, not historical runtime metadata or publication.
Actual backend data still requires its separately reviewed qualification receipt.
"""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import shutil
import sys
import sysconfig
import traceback

HERE = Path(__file__).resolve().parent
EXPLORER = HERE.parent / 'overview/explorer'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PREREQUISITE = HERE / 'jobs/publication-fixtures-v2/fixture-receipt.json'
PRIOR_ADAPTER_RECEIPT = HERE / 'jobs/platform-metadata-data-v1/fixture-receipt.json'
PRIOR_ADAPTER_IDS = tuple('test_platform_metadata_data_v1.PlatformMetadataTransaction.' + name for name in (
    'test_complete_actual_overlay_preserves_records_and_object_identities',
    'test_unresolved_java_remains_false_and_does_not_repair_the_entry',
    'test_late_changed_source_report_receipt_or_provenance_leaves_input_unchanged',
    'test_partial_typed_or_foreign_descriptor_evidence_is_transactional'))
METADATA = HERE / 'jobs/platform-metadata-qualification-v2/measured-backends.json'
METADATA_SHA = 'd8920942ab078dc79228d6d77af64391a818354ddc6b5af9cf383ade46dbe8d8'
METADATA_RECEIPT = METADATA.parent / 'qualification-receipt.json'
METADATA_RECEIPT_SHA = 'c49db52205aba82db5e19be6607a03d3934776254bee31506a146800889ff427'
UNRESOLVED = {'knucleotide:knucleotide-graalvmaot-1', 'knucleotide:knucleotide-graalvmaot-8'}
EXPECTED = tuple('test_platform_metadata_data_v2.PlatformMetadataTransaction.' + name for name in (
    'test_complete_actual_overlay_preserves_records_and_object_identities',
    'test_mixed_java_verification_clears_legacy_modes_and_preserves_provenance',
    'test_late_changed_source_report_receipt_or_provenance_leaves_input_unchanged',
    'test_partial_typed_or_foreign_descriptor_evidence_is_transactional'))


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def now(): return datetime.now(timezone.utc).isoformat()


def new_json(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


def run(output):
    print('WAIT common mutex: four platform metadata transaction fixtures', flush=True)
    with (HERE / 'platform-metadata-data-v2-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            named, opened = LOCK.stat(), os.fstat(lock.fileno())
            assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
            assert output.is_relative_to(HERE / 'jobs') and not output.exists()
            output.mkdir(parents=True)
            receipt = dict(status='failed', started_at=now(), required_fixture_ids=list(EXPECTED),
                discovered_fixture_ids=[], actual_started_fixture_ids=[], tests_run=0,
                expected_fixture_count=4, actual_native_processes=0,
                chart_admission=False, dataset_mutated=False, sources_mutated=False,
                timings_mutated=False, exclusive_common_timing_mutex_held=True,
                common_timing_mutex=str(LOCK), actual_mutex_fd=lock.fileno(),
                automatic_retry=False, prerequisite=str(PREREQUISITE))
            pins = {}
            try:
                assert sys.version_info[:2] == (3, 14) and sys.flags.optimize == 0
                previous = json.loads(PREREQUISITE.read_text())
                assert previous['status'] == 'passed' and previous['tests_run'] == previous['expected_fixture_count'] == 16
                assert previous['exclusive_common_timing_mutex_held'] is True
                assert all(previous[field] == 0 for field in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
                assert len(previous['actual_started_fixture_ids']) == len(set(previous['actual_started_fixture_ids'])) == 16
                assert set(previous['actual_started_fixture_ids']) == set(previous['required_fixture_ids']) == set(previous['discovered_fixture_ids'])
                pins.update(previous['source_pins'])
                for path, digest in pins.items(): assert sha(path) == digest, ('Qualified dependency changed', path)
                pins[str(PREREQUISITE)] = sha(PREREQUISITE)
                pins[str(Path(__file__).resolve())] = sha(__file__)
                prior_adapter = json.loads(PRIOR_ADAPTER_RECEIPT.read_text())
                assert prior_adapter['status'] == 'passed' and prior_adapter['tests_run'] == prior_adapter['expected_fixture_count'] == 4
                assert prior_adapter['exclusive_common_timing_mutex_held'] is True and prior_adapter['pins_unchanged_at_end'] is True
                for field in ('required_fixture_ids', 'discovered_fixture_ids', 'actual_started_fixture_ids'):
                    assert len(prior_adapter[field]) == len(set(prior_adapter[field])) == 4 and set(prior_adapter[field]) == set(PRIOR_ADAPTER_IDS)
                assert all(prior_adapter[field] == [] for field in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
                for path, digest in prior_adapter['source_pins'].items():
                    assert path not in pins or pins[path] == digest
                    assert sha(path) == digest
                    pins[path] = digest
                prior_log = prior_adapter['raw_log']
                assert sha(prior_log['path']) == prior_log['sha256']
                pins[prior_log['path']], pins[str(PRIOR_ADAPTER_RECEIPT)] = prior_log['sha256'], sha(PRIOR_ADAPTER_RECEIPT)
                receipt['prior_adapter_fixture_receipt'] = dict(path=str(PRIOR_ADAPTER_RECEIPT), sha256=pins[str(PRIOR_ADAPTER_RECEIPT)])
                assert sha(METADATA) == METADATA_SHA and sha(METADATA_RECEIPT) == METADATA_RECEIPT_SHA
                metadata, metadata_receipt = json.loads(METADATA.read_text()), json.loads(METADATA_RECEIPT.read_text())
                for artifact in (metadata, metadata_receipt):
                    assert artifact['status'] == 'reconciled-with-unresolved-platform-metadata'
                    assert artifact['expected_program_count'] == artifact['actual_program_count'] == 100
                    assert artifact['qualified_program_count'] == 98
                    assert {row['key'] for row in artifact['unresolved_programs']} == UNRESOLVED
                assert metadata_receipt['metadata_output'] == dict(path=str(METADATA), sha256=METADATA_SHA)
                assert metadata_receipt['expected_fixture_count'] == metadata_receipt['tests_run'] == 6
                assert metadata_receipt['exclusive_common_timing_mutex_held'] is True and metadata_receipt['pins_unchanged_at_end'] is True
                assert all(metadata_receipt[field] == [] for field in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
                for mapping in (metadata['pins'], metadata_receipt['source_pins']):
                    for path, digest in mapping.items():
                        assert path not in pins or pins[path] == digest
                        assert sha(path) == digest
                        pins[path] = digest
                raw_metadata = metadata_receipt['raw_fixture_log']
                assert sha(raw_metadata['path']) == raw_metadata['sha256']
                pins[raw_metadata['path']] = raw_metadata['sha256']
                pins[str(METADATA)], pins[str(METADATA_RECEIPT)] = METADATA_SHA, METADATA_RECEIPT_SHA
                receipt['historical_metadata_prerequisite'] = dict(path=str(METADATA), sha256=METADATA_SHA,
                    receipt_path=str(METADATA_RECEIPT), receipt_sha256=METADATA_RECEIPT_SHA,
                    qualified_program_count=98, unresolved_programs=sorted(UNRESOLVED))
                python = Path(sys.executable).resolve(strict=True)
                pins[str(python)] = sha(python)
                snapshot = output / 'snapshot'; snapshot.mkdir()
                names = ('platform_metadata_data_v2', 'test_platform_metadata_data_v2')
                for name in names:
                    source, target = EXPLORER / (name + '.py'), snapshot / (name + '.py')
                    shutil.copyfile(source, target)
                    assert sha(source) == sha(target)
                    pins[str(source)] = pins[str(target)] = sha(source)
                    target.chmod(0o444)
                snapshot.chmod(0o555)
                frontend_snapshot = output / 'frontend-candidate'; frontend_snapshot.mkdir()
                for name in ('platform.mjs', 'traits.mjs', 'test_platform.mjs', 'REVIEW.md'):
                    source = HERE / 'platform-filter-proposal-v2' / name
                    target = frontend_snapshot / name
                    shutil.copyfile(source, target)
                    assert sha(source) == sha(target)
                    pins[str(source)] = pins[str(target)] = sha(source)
                    target.chmod(0o444)
                frontend_snapshot.chmod(0o555)
                receipt.update(frontend_source_snapshot=str(frontend_snapshot), frontend_fixture_executions=0,
                               frontend_status='unhooked-source-only-separately-unqualified')
                assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
                unittest_root = Path(sysconfig.get_path('stdlib')).resolve(strict=True) / 'unittest'
                unittest_snapshot = output / 'python-dependencies/unittest'
                for source in sorted(unittest_root.rglob('*.py')):
                    target = unittest_snapshot / source.relative_to(unittest_root)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, target)
                    assert sha(source) == sha(target)
                    pins[str(source)] = pins[str(target)] = sha(source)
                    target.chmod(0o444)
                assert (unittest_snapshot / '__init__.py').is_file() and not list(unittest_snapshot.rglob('*.pyc'))
                sys.dont_write_bytecode = True
                sys.path.insert(0, str(unittest_snapshot.parent))
                unittest = importlib.import_module('unittest')
                def unittest_identities():
                    loaded = {}
                    for name, module in sys.modules.items():
                        if name == 'unittest' or name.startswith('unittest.'):
                            path = Path(module.__file__).resolve(strict=True)
                            assert path.is_relative_to(unittest_snapshot) and str(path) in pins
                            loaded[name] = str(path)
                    assert loaded
                    return loaded
                unittest_identities()
                assert all(name not in sys.modules for name in names)
                sys.path.insert(0, str(snapshot))
                tests = importlib.import_module(names[1])
                for name in names:
                    assert Path(sys.modules[name].__file__).resolve() == snapshot / (name + '.py')
                tests.adapter.validate_receipt(metadata, metadata_receipt)
                tests.MUTEX_FD = lock.fileno()
                class RecordedResult(unittest.TextTestResult):
                    def __init__(self, *args, **kwargs):
                        super().__init__(*args, **kwargs); self.started_ids = []
                    def startTest(self, test):
                        self.started_ids.append(test.id()); super().startTest(test)
                suite = unittest.defaultTestLoader.loadTestsFromModule(tests)
                def flatten(value):
                    for test in value:
                        if isinstance(test, unittest.TestSuite): yield from flatten(test)
                        else: yield test.id()
                discovered = list(flatten(suite))
                assert suite.countTestCases() == len(discovered) == len(set(discovered)) == 4
                assert set(discovered) == set(EXPECTED)
                raw = output / 'fixtures.log'
                with raw.open('x') as stream:
                    result = unittest.TextTestRunner(stream=stream, verbosity=2, resultclass=RecordedResult).run(suite)
                receipt.update(discovered_fixture_ids=discovered, actual_started_fixture_ids=result.started_ids,
                    tests_run=result.testsRun, failures=[(test.id(), detail) for test, detail in result.failures],
                    errors=[(test.id(), detail) for test, detail in result.errors],
                    skips=[(test.id(), reason) for test, reason in result.skipped],
                    expected_failures=[(test.id(), detail) for test, detail in result.expectedFailures],
                    unexpected_successes=[test.id() for test in result.unexpectedSuccesses],
                    raw_log=dict(path=str(raw), sha256=sha(raw)), unittest_module_identities=unittest_identities())
                pins[str(raw)] = sha(raw)
                assert result.wasSuccessful() and result.testsRun == len(result.started_ids) == len(set(result.started_ids)) == 4
                assert set(result.started_ids) == set(EXPECTED)
                assert not any(receipt[field] for field in ('failures', 'errors', 'skips', 'expected_failures', 'unexpected_successes'))
                for path, digest in pins.items(): assert sha(path) == digest, ('Fixture evidence changed', path)
                receipt.update(status='passed', source_pins=pins, pins_unchanged_at_end=True)
            except BaseException as error:
                receipt.update(status='failed', error=str(error) or type(error).__name__, error_type=type(error).__name__, source_pins=pins)
                with (output / 'failure.log').open('x') as stream: stream.write(traceback.format_exc())
                raise
            finally:
                receipt['completed_at'] = now()
                path = output / 'fixture-receipt.json'
                new_json(path, receipt)
                available = {str(item): sha(item) for item in output.rglob('*') if item.is_file()}
                new_json(output / 'terminal-pins.json', dict(pins=available, receipt=str(path),
                    receipt_sha256=sha(path), terminal_status=receipt['status'],
                    exclusive_common_timing_mutex_held=True, chart_admission=False, automatic_retry=False))
            print(json.dumps(dict(status=receipt['status'], tests_run=receipt['tests_run'], receipt=str(path))), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.output.resolve())
