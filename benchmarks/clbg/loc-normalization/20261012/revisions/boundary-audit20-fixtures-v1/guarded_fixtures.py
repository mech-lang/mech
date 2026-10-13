"""Run only exact982 preserved fixtures after one test-helper repair, under mutex."""
import argparse
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import re
import sys
import traceback
import unittest

PACKET=Path(__file__).resolve().parent
COHORT=PACKET.parents[1]
PRIOR=COHORT/'jobs/boundary-audit-19'
LOCK=Path('/tmp/mech-clbg-timing.lock')
CHANGED='test_cpp_direct_counter_adapter.py'
FAILED_ID='test_cpp_direct_counter_adapter.CppDirectCounterAdapterFixtures.test_structural_for_header_is_not_a_declaration_statement'
EXPECTED_COUNT=982


def now():return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def new_json(path,value):
    with path.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')


def repair(original):
    edits=[
        ('    def prepare(self, source, marker, *, environment_mutator=None):',
         '    def prepare(self, source, marker, *, environment_mutator=None,\n                intentionally_structural_boundary=False):'),
        ('        else:\n            line_start = cleaned.rfind',
         '''        elif intentionally_structural_boundary:
            # This negative deliberately supplies a control-header candidate;
            # the real adapter must reject structural context before boundary.
            self.assertEqual(cleaned[previous], '(')
            boundary = dict(kind='fixture-intentionally-structural-header-opener',
                            offset=previous)
        else:
            line_start = cleaned.rfind'''),
        ("        self.assert_gated(self.analyze(source, 'int a(0);'),\n                          'cpp-direct-structural-statement-owner-review')",
         "        arguments = self.prepare(source, 'int a(0);',\n            intentionally_structural_boundary=True)\n        segment = arguments.pop('segment')\n        self.assert_gated(analyze_cpp_direct_counter_segment(segment, **arguments),\n                          'cpp-direct-structural-statement-owner-review')"),
    ]
    for old,new in edits:
        assert original.count(old)==1,('Only exact unique fixture helper/test edits',old)
        original=original.replace(old,new,1)
    return original


def flatten(suite):
    for test in suite:
        if isinstance(test,unittest.TestSuite):yield from flatten(test)
        else:yield test.id()


class RecordedResult(unittest.TextTestResult):
    def __init__(self,*args,**kwargs):super().__init__(*args,**kwargs);self.started_ids=[]
    def startTest(self,test):self.started_ids.append(test.id());super().startTest(test)


def run(output):
    print('WAIT common timing mutex: additive audit20 exact982 fixtures only',flush=True)
    with (PACKET/'owner.lock').open('a+') as owner:
        fcntl.flock(owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock,fcntl.LOCK_EX)
            assert output.is_relative_to(COHORT/'jobs') and not output.exists()
            output.mkdir(parents=True)
            report=dict(status='running-fixtures-only',started_at=now(),pid=os.getpid(),active=True,
                exclusive_mutex_held=True,shared_timing_lock=str(LOCK),benchmark_processes_started=0,
                native_probes=0,compiler_processes=0,source_pool_audit_started=False,chart_admitted=False,
                automatic_retry=False,expected_fixture_count=EXPECTED_COUNT)
            try:
                prior=json.loads((PRIOR/'fixture-receipt.json').read_text())
                state=json.loads((PRIOR/'state.json').read_text())
                launch=json.loads((PRIOR/'launch.json').read_text())
                assert prior['status']==state['status']=='failed-fixtures'
                assert prior['tests_run']==state['tests_run']==EXPECTED_COUNT and prior['failures']==state['failures']==1
                assert prior['errors']==state['errors']==0
                assert prior['exclusive_mutex_held'] is True and prior['source_pool_audit_started'] is False
                assert prior['chart_admitted'] is False and prior['automatic_retry'] is False
                assert state['audit_started'] is False and state['automatic_retry'] is False
                log=(PRIOR/'fixtures.log').read_text()
                actual_old=re.findall(r'^test_[A-Za-z0-9_]+ \((test_[A-Za-z0-9_.]+)\) \.\.\. (?:ok|FAIL|ERROR)$',log,re.M)
                failures=re.findall(r'^FAIL: test_[A-Za-z0-9_]+ \((test_[A-Za-z0-9_.]+)\)$',log,re.M)
                assert failures==[FAILED_ID] and 'Fixture must supply a separately owned prior boundary.' in log
                assert len(actual_old)==len(set(actual_old))==EXPECTED_COUNT
                expected=json.loads((PACKET/'expected-test-ids.json').read_text())
                assert expected==actual_old
                pins={str(p):sha(p) for p in (PRIOR/'fixture-receipt.json',PRIOR/'state.json',PRIOR/'launch.json',PRIOR/'fixtures.log',
                    Path(__file__).resolve(),PACKET/'expected-test-ids.json',PACKET/'REVIEW.md')}
                original_names={Path(p).name for p in prior['frozen_dependency_pins']}
                assert len(original_names)==108
                for directory in ('prerequisite-snapshot','snapshot'):
                    assert {p.name for p in (PACKET/directory).iterdir()}==original_names
                for old,digest in prior['frozen_dependency_pins'].items():
                    old=Path(old);assert old.parent==PRIOR/'snapshot' and sha(old)==digest
                    prerequisite=PACKET/'prerequisite-snapshot'/old.name
                    candidate=PACKET/'snapshot'/old.name
                    assert prerequisite.read_bytes()==old.read_bytes() and sha(prerequisite)==digest
                    if old.name==CHANGED:assert candidate.read_text()==repair(old.read_text())
                    else:assert candidate.read_bytes()==old.read_bytes() and sha(candidate)==digest
                    pins.update({str(old):digest,str(prerequisite):sha(prerequisite),str(candidate):sha(candidate)})
                assert sys.version==prior['python_version'] and Path(sys.executable).resolve()==Path(prior['python_executable']).resolve()
                pins[str(Path(sys.executable))]=sha(sys.executable)
                pins[str(Path(sys.executable).resolve())]=sha(Path(sys.executable).resolve())
                dependency=Path(launch['dependency_root'])
                sys.path.insert(0,str(dependency))
                pygments=importlib.import_module('pygments')
                assert pygments.__version__==prior['pygments_version']==launch['expected_pygments_version']
                assert Path(pygments.__file__).resolve().parent==dependency/'pygments'
                assert sha(pygments.__file__)==prior['pygments_module_sha256']
                for p in sorted((dependency/'pygments').rglob('*')):
                    if p.is_file():pins[str(p)]=sha(p)
                os.environ['MECH_LOC_CLBG_ROOT']=launch['root']
                sys.path.insert(0,str(PACKET/'snapshot'))
                modules=[importlib.import_module(name) for name in prior['suites']]
                assert len(modules)==38 and all(Path(m.__file__).resolve().parent==PACKET/'snapshot' for m in modules)
                next(m for m in modules if m.__name__=='test_legacy_loc_data').MUTEX_FD=lock.fileno()
                suite=unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromModule(m) for m in modules)
                discovered=list(flatten(suite))
                assert suite.countTestCases()==EXPECTED_COUNT and discovered==expected
                for p,d in pins.items():assert sha(p)==d,('Pinned prerequisite/candidate changed before run',p)
                raw_log=output/'fixtures.log'
                with raw_log.open('x') as stream:
                    result=unittest.TextTestRunner(stream=stream,verbosity=2,resultclass=RecordedResult).run(suite)
                receipt=dict(status='passed' if result.wasSuccessful() else 'failed-fixtures',
                    tests_run=result.testsRun,expected_test_ids=expected,discovered_test_ids=discovered,
                    actual_test_ids=result.started_ids,failures=[(t.id(),detail) for t,detail in result.failures],
                    errors=[(t.id(),detail) for t,detail in result.errors],skipped=[(t.id(),why) for t,why in result.skipped],
                    expected_failures=[(t.id(),detail) for t,detail in result.expectedFailures],
                    unexpected_successes=[t.id() for t in result.unexpectedSuccesses],
                    successful=result.wasSuccessful(),exclusive_mutex_held=True,common_timing_mutex=str(LOCK),
                    python_executable=sys.executable,python_version=sys.version,pygments_version=pygments.__version__,
                    pygments_module=str(Path(pygments.__file__).resolve()),pygments_module_sha256=sha(pygments.__file__),
                    frozen_dependency_pins={p:d for p,d in pins.items() if Path(p).parent==PACKET/'snapshot'},
                    suites=prior['suites'],benchmark_processes_started=0,source_pool_audit_started=False,
                    chart_admitted=False,automatic_retry=False,raw_log=str(raw_log),raw_log_sha256=sha(raw_log),completed_at=now())
                path=output/'fixture-receipt.json';new_json(path,receipt)
                pins.update({str(raw_log):sha(raw_log),str(path):sha(path)})
                assert receipt['status']=='passed' and receipt['tests_run']==len(receipt['actual_test_ids'])==len(set(receipt['actual_test_ids']))==EXPECTED_COUNT
                assert receipt['actual_test_ids']==expected and not any(receipt[k] for k in ('failures','errors','skipped','expected_failures','unexpected_successes'))
                for p,d in pins.items():assert sha(p)==d,('Pinned fixture bytes changed at end',p)
                report.update(status='passed-fixtures-only-awaiting-source-count-review',active=False,finished_at=now(),pins=pins,
                    fixture_receipt=str(path),fixture_receipt_sha256=pins[str(path)],actual_fixture_count=EXPECTED_COUNT,
                    zero_fixture_skips=True,pins_unchanged_at_end=True,prior_failed_receipt=str(PRIOR/'fixture-receipt.json'),
                    unchanged_production_adapter=True,only_repaired_test=CHANGED)
            except BaseException as error:
                report.update(status='stopped-unqualified',active=False,finished_at=now(),error=str(error) or type(error).__name__,error_type=type(error).__name__)
                traceback.print_exc()
                raise
            finally:new_json(output/'report.json',report)
            print(json.dumps(dict(report=str(output/'report.json'),status=report['status'],actual_fixture_count=EXPECTED_COUNT)),flush=True)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--run',action='store_true',required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();run((COHORT/args.output).resolve())


if __name__=='__main__':main()
