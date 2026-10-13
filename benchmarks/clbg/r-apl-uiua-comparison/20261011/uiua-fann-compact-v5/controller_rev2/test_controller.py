"""Nine mocked revision2 maintenance safety fixtures; no native probe or real signal."""
from copy import deepcopy
from contextlib import redirect_stdout
import fcntl
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch, call

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')


def load():
    spec = importlib.util.spec_from_file_location('isolated_boundary_fixture', HERE / 'controller.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Safety(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.m = load()
        self.m.HERE = self.root
        self.m.STATE = self.root / 'state'
        self.m.STATE.mkdir()
        self.m.PLAN = self.root / 'plan2.json'
        self.m.LOCK = self.root / 'mutex'
        self.clock = 0.0
        self.identities = {1001: dict(pid=1001, listing='recorded waiting Python owner'), 1000: dict(pid=1000, listing='recorded supervisor')}
        self.m.identity = lambda pid: deepcopy(self.identities.get(pid))
        self.m.descendants = lambda pid: []
        self.m.time = SimpleNamespace(monotonic=lambda: self.clock, sleep=self.sleep)
        self.reports = {}
        for language in ('r', 'uiua', 'apl'):
            row = dict(key='nbody-' + language, benchmark='nbody', samples_seconds=[1, 2, 3], sample_validation=[dict(reference_match=True)] * 3)
            attempts = [dict(key=row['key'], full_workload=True, workload_n=50000000, exit_status=0, valid_environment=True, validation=dict(reference_match=True)) for _ in range(3)]
            self.reports[language] = dict(source_audit_sha256=language, active=None,
                programs={row['key']: row, 'old-fann': dict(id='old-fann', key='old-fann', benchmark='fannkuchredux', language=language, samples_seconds=[])}, attempts=attempts, marker='before-final-save')
        self.log = self.root / 'stage.log'
        self.log.write_text('WAIT LOCK 1 old-fann\n')
        self.plan = dict(stage='long-r-uiua', stage_owner_identity=self.identities[1001], coordinator_identity=self.identities[1000],
            stage_log=str(self.log), audit_inputs={language: dict(sha256=language) for language in self.reports},
            report_inputs={language: str(self.root / (language + '.json')) for language in self.reports},
            maximum_boundary_settle_seconds=3, maximum_watch_seconds=.2, maximum_exit_wait_seconds=3,
            pins={}, original_leases=[str(self.root / 'owner.lock')])
        self.write_reports()
        self.write_status('running')
        self.write_plan()

    def sleep(self, seconds):
        self.clock += max(seconds, .1)

    def write_reports(self):
        for language, path in self.plan['report_inputs'].items():
            Path(path).write_text(json.dumps(self.reports[language]))

    def write_status(self, status):
        row = dict(name='long-r-uiua', pid=1001)
        if status != 'running':
            row['exit_status'] = 130
        (self.m.STATE / 'status.json').write_text(json.dumps(dict(status=status, stages=[row])))

    def write_plan(self):
        self.m.PLAN.write_text(json.dumps(self.plan))
        self.digest = self.m.sha(self.m.PLAN)

    def assert_no_signal(self):
        with patch.object(self.m.os, 'kill') as kill, patch.object(self.m.os, 'killpg') as killpg:
            with self.assertRaises(AssertionError):
                self.m.watch(self.digest)
            kill.assert_not_called()
            killpg.assert_not_called()

    def test_active_never_signals(self):
        self.reports['uiua']['active'] = dict(key='nbody-uiua')
        self.write_reports()
        self.assert_no_signal()

    def test_native_child_never_signals(self):
        self.m.descendants = lambda pid: [dict(pid=1002, command='mock native benchmark')]
        self.assert_no_signal()

    def test_incomplete_nbody_never_signals(self):
        self.reports['r']['programs']['nbody-r']['samples_seconds'].pop()
        self.write_reports()
        self.assert_no_signal()

    def test_identity_change_is_fatal(self):
        self.identities[1001] = dict(pid=1001, listing='different process')
        self.assert_no_signal()

    def test_settles_final_save_then_rereads_fresh_bytes(self):
        self.log.write_text('SAMPLE FULL 3 nbody-uiua 1s\n')
        def final_save(seconds):
            self.sleep(seconds)
            for row in self.reports.values():
                row['marker'] = 'after-final-save'
            self.write_reports()
            self.log.write_text('WAIT LOCK 1 old-fann\n')
        self.m.time.sleep = final_save
        with patch.object(self.m.os, 'kill') as kill:
            result = self.m.boundary(self.plan)
            self.assertTrue(all(row['marker'] == 'after-final-save' for row in result['reports'].values()))
            self.assertEqual(result['report_sha256'], {language: self.m.sha(path) for language, path in self.plan['report_inputs'].items()})
            kill.assert_not_called()

    def test_success_targets_recorded_python_pid_only(self):
        def stop(pid, signum):
            self.assertEqual((pid, signum), (1001, signal.SIGINT))
            self.identities.clear()
            self.write_status('stage-error')
        self.m.revision = lambda plan, receipt: dict(revision_directory='mock additive revision')
        with patch.object(self.m.os, 'kill', side_effect=stop) as kill, patch.object(self.m.os, 'killpg') as killpg:
            self.m.watch(self.digest)
            self.assertEqual(kill.call_args_list, [call(1001, signal.SIGINT)])
            killpg.assert_not_called()
            self.assertTrue((self.root / 'transition.json').exists())

    def test_old_uiua_fann_attempt_is_retained_at_boundary(self):
        attempt = dict(key='old-fann', benchmark='fannkuchredux', language='uiua', full_workload=True, seconds=2400, validation=dict(reference_match=True))
        self.reports['uiua']['attempts'].append(attempt)
        self.write_reports()
        with patch.object(self.m.os, 'kill') as kill:
            result = self.m.boundary(self.plan)
            self.assertEqual(result['reports']['uiua']['attempts'][-1], attempt)
            kill.assert_not_called()

    def test_revision_preserves_all_legacy_fann_lineage(self):
        old = self.reports['uiua']
        row = old['programs']['old-fann']
        row.update(samples_seconds=[2400], sample_validation=[dict(reference_match=True)], full_failure=dict(error='recorded own timeout', attempt_number=5))
        old['orders'] = dict(fannkuchredux=[['old-fann']])
        old['discarded'] = [4]
        old['attempts'].extend([dict(key='old-fann', benchmark='fannkuchredux', language='uiua', full_workload=True, seconds=2400),
            dict(key='old-fann', benchmark='fannkuchredux', language='uiua', full_workload=True, error='environment discarded'),
            dict(key='old-fann', benchmark='fannkuchredux', language='uiua', full_workload=True, error='recorded own timeout')])
        self.write_reports()
        self.m.N = self.root
        self.m.CANDIDATE = self.root / 'candidate'
        (self.root / 'uiua-manifest.json').write_text('{}')
        self.plan['revision_directory'] = str(self.root / 'revision')
        self.plan['qualification'] = str(self.root / 'qualification.json')
        self.plan['full_study_manifest'] = str(self.root / 'study.json')
        self.plan['qualification_sha256'] = 'qualification-pin'
        self.plan['full_study_manifest_sha256'] = 'study-pin'
        self.plan['remaining_stage_plan'] = [dict(language='r', benchmark='fannkuchredux')]
        for language, value in self.plan['audit_inputs'].items():
            value['path'] = str(self.root / ('audit-' + language + '.json'))
            Path(value['path']).write_text('{}')
        Path(self.plan['qualification']).write_text(json.dumps(dict(programs=[dict(id='new-v5', key='new-v5', source='v5.ua', source_sha256='v5', loc=20, maximum_workers=8)])))
        Path(self.plan['full_study_manifest']).write_text(json.dumps(dict(profiles=[dict(id='mech59-aot-control')])))
        before = {language: Path(path).read_bytes() for language, path in self.plan['report_inputs'].items()}
        receipt = self.m.boundary(self.plan)
        result = self.m.revision(self.plan, receipt)
        audit = self.m.read(result['endpoint_revision'])
        self.assertEqual(audit['retained_legacy_v2_rows']['old-fann'], row)
        self.assertEqual(audit['retained_legacy_fann_attempts'], old['attempts'][3:])
        self.assertEqual(audit['retained_legacy_fann_orders'], old['orders']['fannkuchredux'])
        self.assertEqual(audit['retained_legacy_discarded_attempts'], [4])
        self.assertEqual(audit['imported_performance_samples'], 0)
        self.assertFalse(audit['chart_admission'])
        mapping = self.m.read(result['mapping'])
        self.assertEqual(set(mapping['continuing_reports']), {'r', 'apl'})
        for language, path in self.plan['report_inputs'].items():
            self.assertEqual(Path(path).read_bytes(), before[language])
            snapshot = mapping['lineage'][path]['snapshot']
            self.assertEqual(Path(snapshot).read_bytes(), before[language])

    def test_nbody_monitor_failure_is_rejected(self):
        self.reports['r']['attempts'][-1]['monitor_errors'] = ['mock fatal instrumentation failure']
        self.assertFalse(self.m.nbody_qualified(self.reports['r']))
        self.write_reports()
        self.assert_no_signal()


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    print('Waiting for common mutex for mocked fixtures then controller plan sealing.', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not (HERE / 'fixtures.json').exists() and not (HERE / 'plan2.json').exists(), 'Do not rerun or overwrite sealed evidence'
        log = io.StringIO()
        with redirect_stdout(log):
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Safety))
        (HERE / 'fixtures.log').write_text(log.getvalue())
        report = dict(status='passed' if result.wasSuccessful() else 'failed', cases=result.testsRun,
                      failures=len(result.failures), errors=len(result.errors), actual_signals_sent=0, native_probes=0,
                      controller_sha256=sha(HERE / 'controller.py'), fixture_source_sha256=sha(__file__),
                      shared_mutex=str(LOCK), isolated_tempfiles=True, mocked_process_identities=True,
                      log=str(HERE / 'fixtures.log'), log_sha256=sha(HERE / 'fixtures.log'))
        (HERE / 'fixtures.json').write_text(json.dumps(report, indent=2) + '\n')
        print(log.getvalue(), flush=True)
        assert result.wasSuccessful() and result.testsRun == 9, 'Safety fixtures failed; no plan seal'
        module = load()
        class InheritedCommonLock:
            def open(self, mode):
                # Duplicate this already-held open description so prepare's
                # normal flock is reentrant and no race separates validation
                # from sealing. No native process is launched by preparation.
                return os.fdopen(os.dup(lock.fileno()), mode)
        module.LOCK = InheritedCommonLock()
        module.prepare()
        print('Fixture receipt SHA', sha(HERE / 'fixtures.json'), flush=True)


if __name__ == '__main__':
    main()
