"""Locked fixtures then separate legacy133 history qualification; no subprocess."""
import argparse
from datetime import datetime,timezone
import fcntl
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import sys
import sysconfig
import traceback

HERE=Path(__file__).resolve().parent
CLBG=HERE.parents[1]
LOCK=Path('/tmp/mech-clbg-timing.lock')
BASELINE=CLBG/'continuation/jobs/platform-metadata-qualification-v2/snapshot/4-data.json'
BASELINE_SHA='65bfa4d833f61fff69b16eb3c917e4fcd1eb21d5788bbbeed7e09e9f2382d32f'
PRIOR=CLBG/'continuation/jobs/platform-metadata-qualification-v2/qualification-receipt.json'
PRIOR_SHA='c49db52205aba82db5e19be6607a03d3934776254bee31506a146800889ff427'
READER=PRIOR.parent/'snapshot/qualify_metadata.py'
FILES=('guarded_qualify.py','qualify_history.py','adapter.py','test_history.py','test_adapter.py',
       'expected-program-keys.json','routes.json','REVIEW.md','extract_existing_evidence.jq')
IDS=sorted(['test_history.HistoryEvidence.'+name for name in (
    'test_runtime_observer_must_use_the_preserved_sample_symbol',
    'test_resident_selector_requires_the_actual_after_phase',
    'test_three_five_seven_samples_are_exact_not_converted',
    'test_byte_identity_cannot_be_borrowed_from_equal_basename_or_version',
    'test_native_link_requires_exact_output_and_cargo_artifact_pin',
    'test_duplicate_or_wrong_language_record_is_never_a_sample_join')]+
    ['test_adapter.LegacyPlatformAdapterFixtures.'+name for name in (
    'test_independent_platform_with_unknown_mode_or_bytes',
    'test_three_and_seven_samples_and_resident_boundary_are_preserved',
    'test_exact_old_current_source_sample_key_and_provenance_identity',
    'test_late_invalid_row_causes_zero_mutation')])


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def verify(pins):
    for path,digest in pins.items():assert sha(path)==digest,('Changed preserved/qualified bytes',path)


def merge(pins,values):
    for path,digest in values.items():
        assert path not in pins or pins[path]==digest,('Conflicting pin',path)
        pins[path]=digest


def new(path,value):
    with Path(path).open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')


def read(path):return json.loads(Path(path).read_text())


def now():return datetime.now(timezone.utc).isoformat()


def fresh_module(name,path,pins):
    assert name not in sys.modules and sha(path)==pins[str(path)]
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec)
    sys.modules[name]=module;spec.loader.exec_module(module)
    assert sys.modules[name] is module and Path(module.__file__).resolve()==path.resolve()
    verify(pins);return module


def snapshot(output,pins):
    target=output/'snapshot';target.mkdir()
    for name in FILES:
        original,copy=HERE/name,target/name
        digest=sha(original);shutil.copyfile(original,copy);assert sha(copy)==digest
        copy.chmod(0o444);merge(pins,{str(original):digest,str(copy):digest})
    python=Path(sys.executable).resolve(strict=True);pins[str(python)]=sha(python)
    assert sys.version_info[:3]==(3,14,8)
    return target


def unittest_snapshot(output,pins):
    assert not any(name=='unittest' or name.startswith('unittest.') for name in sys.modules)
    original=Path(sysconfig.get_path('stdlib')).resolve(strict=True)/'unittest'
    copy=output/'python-dependencies/unittest'
    for source in sorted(original.rglob('*.py')):
        target=copy/source.relative_to(original);target.parent.mkdir(parents=True,exist_ok=True)
        digest=sha(source);shutil.copyfile(source,target);assert sha(target)==digest
        target.chmod(0o444);merge(pins,{str(source):digest,str(target):digest})
    assert not list(copy.rglob('*.pyc'));verify(pins)
    sys.path.insert(0,str(copy.parent));unit=importlib.import_module('unittest')
    assert Path(unit.__file__).resolve()==copy/'__init__.py'
    def identities():
        result={}
        for name,module in tuple(sys.modules.items()):
            if name=='unittest' or name.startswith('unittest.'):
                path=Path(module.__file__).resolve(strict=True)
                assert path.is_relative_to(copy) and sha(path)==pins[str(path)]
                result[name]=str(path)
        assert result;return result
    identities();return unit,identities


def fixture_stage(output,directory,pins,lock):
    receipt=dict(status='failed-fixtures',expected_ids=IDS,discovered_ids=[],started_ids=[],tests_run=0,
        failures=[],errors=[],skipped=[],expected_failures=[],unexpected_successes=[],execution_completed=False,
        successful=False,exclusive_common_mutex_held=True,common_timing_mutex=str(LOCK),common_mutex_fd=lock.fileno(),
        actual_native_processes=0,runtime_version_reruns=0,benchmark_replays=0,chart_admission=False,
        python_version=sys.version,python_executable=str(Path(sys.executable).resolve()),pins=pins,started_at=now())
    results=[]
    try:
        unit,identities=unittest_snapshot(output,pins)
        modules=[fresh_module(name,directory/(name+'.py'),pins) for name in ('test_history','test_adapter')]
        suite=unit.TestSuite(unit.defaultTestLoader.loadTestsFromModule(module) for module in modules)
        def ids(value):
            for item in value:
                if isinstance(item,unit.TestSuite):yield from ids(item)
                else:yield item.id()
        receipt['discovered_ids']=list(ids(suite))
        assert sorted(receipt['discovered_ids'])==IDS and len(set(receipt['discovered_ids']))==10
        identities();verify(pins)
        class ActualResult(unit.TextTestResult):
            def __init__(self,*values):super().__init__(*values);results.append(self)
            def startTest(self,test):receipt['started_ids'].append(test.id());super().startTest(test)
        with (output/'fixtures.log').open('x') as stream:
            result=unit.TextTestRunner(stream=stream,verbosity=2,resultclass=ActualResult).run(suite)
        receipt['execution_completed']=True
        identities();verify(pins)
        receipt['successful']=result.wasSuccessful()
        assert result.wasSuccessful() and result.testsRun==10 and sorted(receipt['started_ids'])==IDS and len(set(receipt['started_ids']))==10
        assert not any((result.failures,result.errors,result.skipped,result.expectedFailures,result.unexpectedSuccesses))
        receipt['status']='passed-fixtures-awaiting-root-history-review'
        receipt['unittest_module_identities']=identities()
    except BaseException as error:
        receipt.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
        raise
    finally:
        result=results[0] if results else None
        if result:
            receipt.update(tests_run=result.testsRun,failures=[(test.id(),detail) for test,detail in result.failures],
                errors=[(test.id(),detail) for test,detail in result.errors],skipped=[(test.id(),detail) for test,detail in result.skipped],
                expected_failures=[(test.id(),detail) for test,detail in result.expectedFailures],unexpected_successes=[test.id() for test in result.unexpectedSuccesses])
        receipt['finished_at']=now();new(output/'fixture-receipt.json',receipt)


def prior_fixture(args,pins):
    path=args.fixture_receipt.resolve(strict=True);assert path.parent.parent==HERE/'jobs'
    assert sha(path)==args.fixture_sha256
    receipt=read(path);terminal_path=path.parent/'terminal.json'
    assert sha(terminal_path)==args.fixture_terminal_sha256
    terminal=read(terminal_path)
    assert terminal['status']=='completed-fixtures' and terminal['body_error'] is terminal['integrity_error'] is None
    assert terminal['exclusive_common_mutex_held'] is True and terminal['common_timing_mutex']==str(LOCK) and terminal['available_bytes_not_fully_pinned'] is False
    assert receipt['status']=='passed-fixtures-awaiting-root-history-review' and receipt['successful'] is receipt['execution_completed'] is True
    assert type(receipt['tests_run']) is int and receipt['tests_run']==10
    for name in ('expected_ids','discovered_ids','started_ids'):
        assert sorted(receipt[name])==IDS and len(receipt[name])==len(set(receipt[name]))==10
    assert all(receipt[name]==[] for name in ('failures','errors','skipped','expected_failures','unexpected_successes'))
    assert receipt['exclusive_common_mutex_held'] is True and receipt['common_timing_mutex']==str(LOCK)
    for artifact in (receipt,terminal):
        assert all(type(artifact[name]) is int and artifact[name]==0 for name in ('actual_native_processes','runtime_version_reruns','benchmark_replays'))
        assert artifact['chart_admission'] is False
    assert terminal['pins'][str(path)]==args.fixture_sha256
    verify(receipt['pins']);verify(terminal['pins'])
    for name in FILES:assert pins[str(HERE/name)]==receipt['pins'][str(HERE/name)]
    merge(pins,receipt['pins']);merge(pins,terminal['pins']);merge(pins,{str(path):args.fixture_sha256,str(terminal_path):args.fixture_terminal_sha256})


def qualification_stage(args,output,directory,pins,history,adapter):
    assert sha(BASELINE)==BASELINE_SHA and sha(PRIOR)==PRIOR_SHA
    prior=read(PRIOR)
    assert prior['status']=='reconciled-with-unresolved-platform-metadata' and prior['pins_unchanged_at_end'] is True
    assert prior['source_pins'][str(BASELINE)]==BASELINE_SHA and str(READER) in prior['source_pins']
    merge(pins,{str(BASELINE):BASELINE_SHA,str(PRIOR):PRIOR_SHA,str(READER):prior['source_pins'][str(READER)]})
    reader_module=fresh_module('legacy133_actual_pinned_reader',READER,pins)
    reads=reader_module.PinnedReads(CLBG,pins)
    data=read(BASELINE);expected=read(directory/'expected-program-keys.json')
    history.inventory(data,expected)
    result=history.qualify(CLBG,data,expected,reads,output)
    # Independently replay every saved runtime JSON pointer/value encoding.
    for backend in result['annotations'].values():
        runtime=backend['evidence']['runtime_receipt']
        if runtime['verified'] is True:
            original=reads.report(runtime['report'],runtime['report_sha256'])
            value=history.pointer(original,runtime['field'])
            encoded=value if runtime['value_encoding']=='literal-saved-string' else json.dumps(value,sort_keys=True)
            assert encoded==runtime['value']
    changes,provenance=adapter.stage(data,data,result)
    assert len(changes)==133 and provenance
    verify(pins)
    result.update(pins=dict(pins),baseline=dict(path=str(BASELINE),sha256=BASELINE_SHA),
        fixture=dict(path=str(args.fixture_receipt.resolve()),sha256=args.fixture_sha256,terminal_sha256=args.fixture_terminal_sha256),
        created_at=now(),actual_common_mutex_held=True)
    new(output/'qualified-history.json',result)
    new(output/'qualification-receipt.json',dict(status=result['status'],profile_count=133,
        qualified_history=str(output/'qualified-history.json'),qualified_history_sha256=sha(output/'qualified-history.json'),
        qualified_platform_rows=result['qualified_platform_rows'],qualified_execution_rows=result['qualified_execution_rows'],
        qualified_historical_byte_rows=result['qualified_historical_byte_rows'],actual_native_processes=0,runtime_version_reruns=0,
        benchmark_replays=0,chart_admission=False,dataset_mutated=False,sources_mutated=False,timings_mutated=False,
        actual_common_mutex_held=True,pins_unchanged_at_end=True))


def run(args):
    print('WAIT common mutex: legacy133 fixtures/history metadata only',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        named,held=LOCK.stat(),os.fstat(lock.fileno());assert (named.st_dev,named.st_ino)==(held.st_dev,held.st_ino)
        output=HERE/'jobs'/args.revision;output.parent.mkdir(parents=True,exist_ok=True);output.mkdir()
        pins={};body_error=None;integrity_error=None;completed='failed-stop'
        try:
            sys.dont_write_bytecode=True
            directory=snapshot(output,pins)
            if args.stage=='qualify':prior_fixture(args,pins)
            assert not any(name in sys.modules for name in ('qualify_history','adapter','test_history','test_adapter'))
            sys.path.insert(0,str(directory))
            history=fresh_module('qualify_history',directory/'qualify_history.py',pins)
            adapter=fresh_module('adapter',directory/'adapter.py',pins)
            if args.stage=='fixtures':fixture_stage(output,directory,pins,lock);completed='completed-fixtures'
            else:qualification_stage(args,output,directory,pins,history,adapter);completed='completed-history-qualification'
        except BaseException as error:
            body_error=dict(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            with (output/'terminal-error.log').open('x') as stream:stream.write(traceback.format_exc())
            raise
        finally:
            actual={};candidates=dict(pins)
            candidates.update({str(path):None for path in output.rglob('*') if path.is_file()})
            for path,expected in candidates.items():
                try:
                    digest=sha(path);assert expected is None or digest==expected,('Terminal changed pin',path)
                    actual[path]=digest
                except BaseException as error:
                    if integrity_error is None:integrity_error=dict(path=path,error=str(error) or type(error).__name__,error_type=type(error).__name__)
            terminal=dict(status='failed-terminal-integrity' if integrity_error else completed,pins=actual,
                body_error=body_error,integrity_error=integrity_error,available_bytes_not_fully_pinned=integrity_error is not None,
                exclusive_common_mutex_held=True,common_timing_mutex=str(LOCK),actual_native_processes=0,runtime_version_reruns=0,
                benchmark_replays=0,chart_admission=False,finished_at=now())
            new(output/'terminal.json',terminal)
            if integrity_error:raise RuntimeError('Terminal integrity failure retained: '+str(integrity_error))
        print(json.dumps(dict(status=completed,output=str(output),terminal_sha256=sha(output/'terminal.json'),
            receipt_sha256=sha(output/('fixture-receipt.json' if args.stage=='fixtures' else 'qualification-receipt.json')))),flush=True)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--stage',choices=('fixtures','qualify'),required=True)
    parser.add_argument('--revision',required=True);parser.add_argument('--fixture-receipt',type=Path)
    parser.add_argument('--fixture-sha256');parser.add_argument('--fixture-terminal-sha256')
    args=parser.parse_args();assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision)
    if args.stage=='qualify':assert all((args.fixture_receipt,args.fixture_sha256,args.fixture_terminal_sha256))
    run(args)


if __name__=='__main__':main()
