"""Thirteen focused argv/lineage/descriptor checks; no benchmark or signal."""
import copy
import fcntl
import os
import subprocess
import tempfile
from pathlib import Path
from types import SimpleNamespace
import unittest
import continuation as c
from descriptor_guard import HighDescriptorLock, reconcile


def fixture():
    row = dict(key=c.KEY, id=c.KEY.split(':')[1], benchmark='fasta', language='apl', source='/source',
        source_sha256='source-sha', command=[str(c.N / 'run-apl.sh'), '--cc', '8', '-f', '/source'],
        samples_seconds=[], sample_validation=[], attempts=[58], measurement_status='failed',
        full_failure=dict(error_type='OutputValidationError', stdout_bytes=0, exit_status=0, error='empty output'))
    attempt = dict(attempt=58, key=c.KEY, qualified=False, command=copy.deepcopy(row['command']))
    report = dict(programs={c.KEY:row}, attempts=[attempt], orders={'fasta':[[c.KEY]]},
                  completed_benchmarks=['mandelbrot'], status='partial')
    audit = dict(programs=[copy.deepcopy(row)], controls=[])
    return report, audit


def contract_fixture():
    report = dict(programs={})
    pins = {}
    for _, benchmark in c.STAGES:
        for language in ('apl', 'mech'):
            key = benchmark + ':' + language
            source = '/' + key + '.source'
            command = [str(c.N / 'run-apl.sh'), '--cc', '8', '-f', source] if language == 'apl' else ['/' + key + '.bin']
            if benchmark != 'knucleotide':
                command += ['--', '{n}'] if language == 'apl' else ['{n}']
            row = dict(key=key, id=key, source=source, source_sha256='source-' + key, command=command,
                       benchmark=benchmark, language=language, binary_sha256='binary-' + key)
            report['programs'][key] = row
            pins[source] = row['source_sha256']
            if language == 'mech':
                pins[command[0]] = row['binary_sha256']
    files = (Path('/small-input'), Path('/full-input'))
    pins.update({str(path):str(path) + '-sha' for path in files})
    harness = SimpleNamespace(INPUTS={'knucleotide':files}, WORK={name:(10,1000) for _,name in c.STAGES})
    return report, harness, pins


class ArgvChecks(unittest.TestCase):
    def test_high_descriptor_is_same_description_without_a_low_keeper(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'isolated.lock'
            held = HighDescriptorLock(path).open('a+')
            self.assertGreaterEqual(held.fileno(), 64)
            fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
            state = os.fstat(held.fileno())
            self.assertEqual((state.st_dev, state.st_ino), (path.stat().st_dev, path.stat().st_ino))
            duplicate = os.dup(held.fileno())
            try:
                held.close()
                with path.open('a+') as independent:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(independent, fcntl.LOCK_EX | fcntl.LOCK_NB)
                os.close(duplicate)
                duplicate = None
                with path.open('a+') as independent:
                    fcntl.flock(independent, fcntl.LOCK_EX | fcntl.LOCK_NB)
            finally:
                held.close()
                if duplicate is not None:
                    os.close(duplicate)

    def test_actual_descriptor_fields_require_exact_pid_fd_device_inode_and_path(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'isolated.lock'
            path.touch()
            state = path.stat()
            fields = ['p987', 'f64', 'D' + hex(state.st_dev), 'i' + str(state.st_ino), 'n' + str(path)]
            self.assertEqual(reconcile(('\n'.join(fields) + '\n').encode(), 987, 64, state, path)['status'], 'passed')
            for index, replacement in ((0, 'p988'), (1, 'f3'), (2, 'D' + hex(state.st_dev + 1)),
                                       (3, 'i' + str(state.st_ino + 1)), (4, 'n' + str(path.parent / 'different'))):
                changed = list(fields)
                changed[index] = replacement
                with self.assertRaises(AssertionError):
                    reconcile(('\n'.join(changed) + '\n').encode(), 987, 64, state, path)
            for extra in ('p988', 'f3'):
                with self.assertRaises(AssertionError):
                    reconcile(('\n'.join(fields + [extra]) + '\n').encode(), 987, 64, state, path)

    def test_only_argv_changes_and_all_failed_lineage_remains(self):
        report, audit = fixture()
        original, old_audit = copy.deepcopy(report), copy.deepcopy(audit)
        revised, overlay = c.corrected(report, audit)
        self.assertEqual(report, original)
        self.assertEqual(audit, old_audit)
        self.assertEqual(revised['attempts'], original['attempts'])
        self.assertEqual(revised['orders'], original['orders'])
        self.assertEqual(revised['programs'][c.KEY]['command'], original['programs'][c.KEY]['command'] + ['--','{n}'])
        self.assertEqual(overlay['programs'][0]['command'], revised['programs'][c.KEY]['command'])
        self.assertEqual(revised['programs'][c.KEY]['command_metadata_failure_lineage'], original['programs'][c.KEY]['full_failure'])

    def test_a_qualified_sample_cannot_be_reset(self):
        report, audit = fixture()
        report['programs'][c.KEY]['samples_seconds'] = [1.0]
        with self.assertRaises(AssertionError):
            c.corrected(report, audit)

    def test_other_failures_or_changed_commands_cannot_be_silently_repaired(self):
        for field, value in (('error_type','TimeoutError'),('stdout_bytes',1),('exit_status',1)):
            report,audit=fixture()
            report['programs'][c.KEY]['full_failure'][field]=value
            with self.assertRaises(AssertionError):c.corrected(report,audit)
        report,audit=fixture()
        report['programs'][c.KEY]['command'] += ['--','{n}']
        with self.assertRaises(AssertionError):c.corrected(report,audit)

    def test_audit_and_last_failed_attempt_must_match(self):
        report,audit=fixture()
        audit['programs'][0]['command']=['changed']
        with self.assertRaises(AssertionError):c.corrected(report,audit)
        report,audit=fixture()
        report['attempts'][0]['qualified']=True
        with self.assertRaises(AssertionError):c.corrected(report,audit)

    def test_all_five_remaining_numeric_and_stdin_contracts_are_checked(self):
        report,harness,pins=contract_fixture()
        contracts=c.preflight(report,harness,pins)
        self.assertEqual(len(contracts),10)
        self.assertEqual(contracts['knucleotide:apl']['mode'],'stdin')
        self.assertEqual(contracts['fasta:apl']['full'],1000)
        self.assertEqual(contracts['knucleotide:mech']['full_sha256'],'/full-input-sha')

    def test_missing_numeric_input_is_rejected_before_native_work(self):
        report,harness,pins=contract_fixture()
        report['programs']['fasta:apl']['command']=report['programs']['fasta:apl']['command'][:-2]
        with self.assertRaises(AssertionError):c.preflight(report,harness,pins)
        report,harness,pins=contract_fixture()
        report['programs']['nbody:mech']['command'].append('{n}')
        with self.assertRaises(AssertionError):c.preflight(report,harness,pins)

    def test_stdin_requires_pinned_small_full_and_no_numeric_argument(self):
        report,harness,pins=contract_fixture()
        pins.pop('/full-input')
        with self.assertRaises(AssertionError):c.preflight(report,harness,pins)
        report,harness,pins=contract_fixture()
        report['programs']['knucleotide:apl']['command']+=['--','{n}']
        with self.assertRaises(AssertionError):c.preflight(report,harness,pins)

    def test_worker_source_binary_and_placeholder_contracts_cannot_change(self):
        for mutation in (
            lambda r:r['programs']['fasta:apl']['command'].__setitem__(2,'9'),
            lambda r:r['programs']['binarytrees:apl']['command'].__setitem__(4,'different-source'),
            lambda r:r['programs']['nbody:mech'].__setitem__('binary_sha256','changed'),
            lambda r:r['programs']['fasta:apl']['command'].__setitem__(4,'{source}'),
        ):
            report,harness,pins=contract_fixture();mutation(report)
            with self.assertRaises(AssertionError):c.preflight(report,harness,pins)

    def test_remaining_stages_exclude_completed_r_mandel_and_uiua(self):
        self.assertEqual(c.STAGES,[('apl',b) for b in ('fasta','knucleotide','binarytrees','nbody','fannkuchredux')])
        report,harness,pins=contract_fixture()
        report['programs']['fasta:apl']['language']='uiua'
        with self.assertRaises(AssertionError):c.preflight(report,harness,pins)

    def test_frozen_guard_alias_order_and_sample_resume_are_reused(self):
        g=c.guard()
        g.STAGES=c.STAGES
        row=dict(id='source',key='source',benchmark='fasta',samples_seconds=[333.0],sample_validation=[{}],attempts=[56])
        report=dict(programs={'source':row,'alias':dict(row,key='alias',timing_alias_of='source')},orders={})
        rows=g.ordered_candidates(report,'apl','fasta',0)
        self.assertEqual([r['key'] for r in rows],['source'])
        self.assertEqual(row['samples_seconds'],[333.0])
        self.assertTrue(len(row['samples_seconds'])>0)
        with self.assertRaises(AssertionError):g.ordered_candidates(report,'r','fasta',0)

    def test_failed_dispatch_cleans_only_known_waiter_with_bounded_waits(self):
        class Process:
            pid = 999
            def __init__(self, first_error=None, terminate_interrupted=False):
                self.returncode = None
                self.first_error = first_error
                self.calls = []
                self.terminate_interrupted = terminate_interrupted
            def poll(self): return self.returncode
            def terminate(self):
                self.calls.append('term')
                if self.terminate_interrupted: raise KeyboardInterrupt('terminate interrupted')
            def kill(self): self.calls.append('kill'); self.returncode = -9
            def wait(self, timeout):
                self.calls.append(('wait', timeout))
                if self.first_error is not None:
                    error, self.first_error = self.first_error, None
                    raise error
                if self.returncode is None: self.returncode = -15
                return self.returncode
        harness = SimpleNamespace(group_members=lambda pgid: [])
        for error in (None, subprocess.TimeoutExpired(['owned-waiter'], 8), KeyboardInterrupt('parent interrupted')):
            process = Process(error)
            detail = {}
            if isinstance(error, KeyboardInterrupt):
                with self.assertRaises(KeyboardInterrupt): c.cleanup_waiter(process, harness, detail)
            else:
                c.cleanup_waiter(process, harness, detail)
                self.assertEqual(detail['remaining_owned_group'], [])
            self.assertIsNotNone(process.returncode)
            self.assertIn(('wait', 8), process.calls)
            if error is not None:
                self.assertIn('kill', process.calls)
                self.assertIn(('wait', 5), process.calls)
        process = Process(terminate_interrupted=True)
        with self.assertRaises(KeyboardInterrupt): c.cleanup_waiter(process, harness, {})
        self.assertIsNotNone(process.returncode)
        self.assertIn('kill', process.calls)
        self.assertIn(('wait', 5), process.calls)


if __name__=='__main__':
    descriptor=int(os.environ['MECH_TIMING_LOCK_FD'])
    actual=os.fstat(descriptor); expected=c.LOCK.stat()
    assert (actual.st_dev,actual.st_ino)==(expected.st_dev,expected.st_ino)
    fcntl.flock(descriptor,fcntl.LOCK_EX | fcntl.LOCK_NB)
    unittest.main(verbosity=2)
