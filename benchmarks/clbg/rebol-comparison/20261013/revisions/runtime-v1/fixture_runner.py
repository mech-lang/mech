"""Fresh six-fixture runner, called only while parent holds the named mutex."""
import importlib
import importlib.util
from pathlib import Path
import shutil
import sys


def run(output, pins, sha, verify, save, expected):
    result = None
    inserted = False
    receipt = dict(status='failed',expected_test_ids=expected,actual_test_ids=[],discovered_test_ids=[],
        discovered_tests=0,tests_run=0,execution_completed=False,inherited_common_mutex=True,
        failures=[],errors=[],skips=[],expected_failures=[],unexpected_successes=[],
        python_executable=str(Path(sys.executable).resolve()),python_version=sys.version)
    try:
        assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
        assert 'test_runtime' not in sys.modules and 'contracts' not in sys.modules
        original = Path(importlib.util.find_spec('unittest').origin).resolve().parent
        frozen = output/'python'/'unittest'
        receipt['unittest_snapshot']=str(frozen)
        frozen.mkdir(parents=True)
        for source in sorted(original.rglob('*.py')):
            target=frozen/source.relative_to(original);target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(source,target)
            digest=sha(source)
            assert str(source) not in pins or pins[str(source)]==digest
            pins[str(source)]=pins[str(target)]=digest
            assert sha(target)==pins[str(source)]
        executable=str(Path(sys.executable).resolve());digest=sha(executable)
        assert executable not in pins or pins[executable]==digest
        pins[executable]=digest
        sources=output/'fixture-sources';sources.mkdir()
        for name in ('test_runtime.py','contracts.py'):
            source=Path(__file__).with_name(name).resolve();target=sources/name
            assert sha(source)==pins[str(source)];shutil.copyfile(source,target)
            assert sha(target)==pins[str(source)];pins[str(target)]=sha(target)
        verify(pins)
        sys.path.insert(0,str(frozen.parent));inserted=True
        unittest=importlib.import_module('unittest')
        assert Path(unittest.__file__).resolve()==frozen/'__init__.py'
        def identities():
            for name,loaded in list(sys.modules.items()):
                if name=='unittest' or name.startswith('unittest.'):
                    actual=Path(loaded.__file__).resolve()
                    assert actual.is_relative_to(frozen) and actual.suffix=='.py' and sha(actual)==pins[str(actual)]
            for name in ('test_runtime','contracts'):
                if name in sys.modules:
                    actual=Path(sys.modules[name].__file__).resolve()
                    assert actual==sources/(name+'.py') and sha(actual)==pins[str(actual)]
        class Recorded(unittest.TextTestResult):
            def __init__(self,*args):
                nonlocal result
                super().__init__(*args);result=self
            def startTest(self,test):
                receipt['actual_test_ids'].append(test.id());super().startTest(test)
        sys.path.insert(0,str(sources))
        try:
            loader=unittest.TestLoader();suite=loader.loadTestsFromName('test_runtime')
        finally:
            sys.path.pop(0)
        receipt['loader_errors']=list(loader.errors)
        def collect(part):
            if isinstance(part,unittest.TestSuite):
                for child in part:collect(child)
            else:receipt['discovered_test_ids'].append(part.id())
        collect(suite);receipt['discovered_tests']=suite.countTestCases()
        save(output/'discovery.json',dict(loader_errors=receipt['loader_errors'],
            discovered_test_ids=receipt['discovered_test_ids'],sys_path=sys.path[:]))
        identities();verify(pins)
        assert not loader.errors
        assert len(receipt['discovered_test_ids'])==len(set(receipt['discovered_test_ids']))==len(expected)
        assert set(receipt['discovered_test_ids'])==set(expected)
        with (output/'fixtures.log').open('x') as stream:
            result=unittest.TextTestRunner(stream=stream,verbosity=2,resultclass=Recorded).run(suite)
        receipt['execution_completed']=True
        receipt.update(tests_run=result.testsRun,failures=[(t.id(),why) for t,why in result.failures],
            errors=[(t.id(),why) for t,why in result.errors],skips=[(t.id(),why) for t,why in result.skipped],
            expected_failures=[(t.id(),why) for t,why in result.expectedFailures],
            unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
        assert len(receipt['actual_test_ids'])==len(set(receipt['actual_test_ids']))==len(expected)
        assert set(receipt['actual_test_ids'])==set(expected) and result.testsRun==len(expected)
        assert all(receipt[key]==[] for key in ('failures','errors','skips','expected_failures','unexpected_successes'))
        assert result.wasSuccessful();identities();verify(pins);receipt['status']='passed'
    except BaseException as error:
        receipt.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
        raise
    finally:
        if result is not None:
            receipt.update(tests_run=result.testsRun,failures=[(t.id(),why) for t,why in result.failures],
                errors=[(t.id(),why) for t,why in result.errors],skips=[(t.id(),why) for t,why in result.skipped],
                expected_failures=[(t.id(),why) for t,why in result.expectedFailures],
                unexpected_successes=[t.id() for t in result.unexpectedSuccesses])
        save(output/'fixture-receipt.json',receipt)
        if inserted:sys.path.pop(0)
    return receipt
