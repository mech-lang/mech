"""Four new synthetic archive fixtures; original runtime six are not repeated."""
import argparse
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

PACKET = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
FILES = ('extract_source.py','test_extract_source.py','bootstrap_runtime_directory_v1.py','qualify_extract.py','REVIEW.md')
IDS = tuple('test_extract_source.DirectoryContracts.'+name for name in (
    'test_repeated_exact_directory_identity_allows_only_unapplied_timestamp_difference',
    'test_regular_link_and_cross_type_duplicates_never_overwrite',
    'test_alias_escape_metadata_and_unselected_member_types_reject',
    'test_existing_destination_and_dangling_link_keep_original_policy'))


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--revision',required=True)
    args = parser.parse_args()
    assert args.revision and all(value.isalnum() or value in '-_' for value in args.revision)
    print('WAIT common mutex: four new directory extraction fixtures',flush=True)
    with LOCK.open('a+') as common:
        fcntl.flock(common,fcntl.LOCK_EX)
        held,named = os.fstat(common.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
        output = PACKET/'jobs'/args.revision; output.mkdir(parents=True)
        pins = {}; body_error = None; result = None
        receipt = dict(status='failed-directory-extraction-fixtures',expected_ids=list(IDS),
            discovered_ids=[],actual_ids=[],tests_run=0,loader_errors=[],failures=[],errors=[],skips=[],
            expected_failures=[],unexpected_successes=[],execution_completed=False,
            inherited_common_mutex=True,actual_native_subjects=0,original_runtime_fixture_reexecutions=0)
        try:
            assert sys.dont_write_bytecode and not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
            assert not any(name in sys.modules for name in ('extract_source','test_extract_source'))
            snapshot = output/'source'; snapshot.mkdir()
            for name in FILES:
                original,target = PACKET/name,snapshot/name
                raw = original.read_bytes(); target.write_bytes(raw); target.chmod(0o444)
                pins[str(original)] = pins[str(target)] = hashlib.sha256(raw).hexdigest()
            python = str(Path(sys.executable).resolve()); pins[python] = sha(python)
            stdlib = Path(importlib.util.find_spec('unittest').origin).resolve().parent
            frozen = output/'python/unittest'; frozen.mkdir(parents=True)
            for original in sorted(stdlib.rglob('*.py')):
                target = frozen/original.relative_to(stdlib); target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(original,target); target.chmod(0o444)
                pins[str(original)] = pins[str(target)] = sha(original)
            for path,digest in pins.items(): assert sha(path) == digest
            sys.path[:0] = [str(snapshot),str(frozen.parent)]
            unittest = importlib.import_module('unittest')
            assert Path(unittest.__file__).resolve() == frozen/'__init__.py'
            class Observed(unittest.TextTestResult):
                def startTest(self,test):
                    receipt['actual_ids'].append(test.id()); super().startTest(test)
            loader = unittest.TestLoader(); suite = loader.loadTestsFromName('test_extract_source')
            def collect(part):
                if isinstance(part,unittest.TestSuite):
                    for child in part: collect(child)
                else: receipt['discovered_ids'].append(part.id())
            collect(suite); receipt['loader_errors'] = list(loader.errors)
            assert not loader.errors and len(receipt['discovered_ids']) == len(set(receipt['discovered_ids'])) == 4
            assert set(receipt['discovered_ids']) == set(IDS)
            for name in ('extract_source','test_extract_source'):
                assert Path(sys.modules[name].__file__).resolve() == snapshot/(name+'.py')
            with (output/'fixtures.log').open('x') as stream:
                result = unittest.TextTestRunner(stream=stream,verbosity=2,resultclass=Observed).run(suite)
            receipt.update(tests_run=result.testsRun,failures=[(test.id(),why) for test,why in result.failures],
                errors=[(test.id(),why) for test,why in result.errors],skips=[(test.id(),why) for test,why in result.skipped],
                expected_failures=[(test.id(),why) for test,why in result.expectedFailures],
                unexpected_successes=[test.id() for test in result.unexpectedSuccesses],execution_completed=True)
            assert result.testsRun == len(receipt['actual_ids']) == len(set(receipt['actual_ids'])) == 4
            assert set(receipt['actual_ids']) == set(IDS) and result.wasSuccessful()
            assert all(receipt[key] == [] for key in ('failures','errors','skips','expected_failures','unexpected_successes'))
            for name,module in list(sys.modules.items()):
                if name == 'unittest' or name.startswith('unittest.'):
                    path = Path(module.__file__).resolve()
                    assert path.is_relative_to(frozen) and pins[str(path)] == sha(path)
            for path,digest in pins.items(): assert sha(path) == digest
            receipt['status'] = 'passed-directory-extraction-fixtures'
        except BaseException as failure:
            body_error = dict(error=str(failure) or type(failure).__name__,error_type=type(failure).__name__)
            with (output/'error.log').open('x') as stream: traceback.print_exc(file=stream)
            raise
        finally:
            integrity_errors = []
            for path,digest in pins.items():
                try: assert sha(path) == digest
                except BaseException as failure: integrity_errors.append(dict(path=path,error=str(failure) or type(failure).__name__))
            if integrity_errors: receipt['status'] = 'failed-directory-extraction-integrity'
            available = {}
            for path in output.rglob('*'):
                if path.is_file():
                    try: available[str(path.resolve())] = sha(path)
                    except BaseException as failure:
                        integrity_errors.append(dict(path=str(path),error=str(failure) or type(failure).__name__))
            if integrity_errors: receipt['status'] = 'failed-directory-extraction-integrity'
            receipt['source_pins'] = pins
            receipt['available_evidence_pins'] = available
            save(output/'fixture-receipt.json',receipt)
            save(output/'terminal.json',dict(status=receipt['status'],body_error=body_error,
                integrity_errors=integrity_errors,source_pins=pins,available_evidence_pins=available,receipt=str(output/'fixture-receipt.json'),
                receipt_sha256=sha(output/'fixture-receipt.json'),inherited_common_mutex=True,
                actual_native_subjects=0,original_runtime_fixture_reexecutions=0))
            if integrity_errors: raise RuntimeError('Directory fixture source integrity failed')
        print(json.dumps(dict(status=receipt['status'],receipt=str(output/'fixture-receipt.json'),
            receipt_sha256=sha(output/'fixture-receipt.json'),terminal_sha256=sha(output/'terminal.json'),
            tests_run=4,actual_native_subjects=0)),flush=True)


if __name__ == '__main__': main()
