"""Reviewed no-gap handoff from mocked preparation to a durable boundary watcher.

This launcher owns no native benchmark process. It may start only the reviewed
waiting-owner maintenance watcher after every fixture and boundary gate passes.
"""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
CONTROLLER_SHA = '12a6467da5251eded5faa0faa760809634ca9af6f8ba3111293eb5c6857eb0be'
EXPECTED_STAGE_PID = 54174
EXPECTED_SUPERVISOR_PID = 31810
EXPECTED_PIN_COUNT = 1253


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, HERE / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class InheritedFirstOpen:
    """Only the first open reuses an already-held common lock description."""
    def __init__(self, descriptor):
        inherited = os.fstat(descriptor)
        target = LOCK.stat()
        assert (inherited.st_dev, inherited.st_ino) == (target.st_dev, target.st_ino), 'Inherited descriptor is not the common mutex inode'
        self.descriptor = descriptor

    def __str__(self):
        return str(LOCK)

    def open(self, mode):
        assert mode == 'a+'
        if self.descriptor is None:
            return LOCK.open(mode)
        descriptor, self.descriptor = self.descriptor, None
        try:
            return os.fdopen(os.dup(descriptor), mode)
        finally:
            os.close(descriptor)

    def close_unused(self):
        if self.descriptor is not None:
            os.close(self.descriptor)
            self.descriptor = None


def reviewed_plan(controller, digest):
    assert controller.sha(controller.__file__) == CONTROLLER_SHA, 'Reviewed controller changed'
    assert controller.sha(controller.PLAN) == digest, 'Reviewed sealed plan changed'
    plan = controller.read(controller.PLAN)
    fixture = controller.read(HERE / 'fixtures.json')
    assert fixture['status'] == 'passed' and fixture['cases'] == 9 and fixture['failures'] == fixture['errors'] == 0
    assert fixture['controller_sha256'] == CONTROLLER_SHA and fixture['actual_signals_sent'] == fixture['native_probes'] == 0
    assert len(plan['pins']) == EXPECTED_PIN_COUNT
    assert plan['stage_owner_identity']['pid'] == EXPECTED_STAGE_PID
    assert plan['coordinator_identity']['pid'] == EXPECTED_SUPERVISOR_PID
    controller.verify(plan['pins'])
    return plan


def child_watch(launcher_digest, plan_digest, lock_fd, go_fd):
    inherited = InheritedFirstOpen(lock_fd)
    controller = load('revision2_durable_controller', 'controller.py')
    try:
        assert controller.sha(__file__) == launcher_digest, 'Reviewed launcher changed'
        # Parent persists launch identity/plan SHA before authorizing the sole
        # waiting-owner signal. EOF or any other byte exits without signalling.
        assert os.read(go_fd, 1) == b'1', 'Parent did not persist and authorize handoff'
        os.close(go_fd)
        go_fd = None
        reviewed_plan(controller, plan_digest)
        controller.save_new(HERE / 'watch-started.json', dict(started_at=controller.now(),
            watcher_identity=controller.identity(os.getpid()), plan_sha256=plan_digest,
            launcher_sha256=launcher_digest, inherited_common_mutex=True,
            inherited_descriptor_matches_lock_inode=True, native_processes_signalled=False))
        controller.LOCK = inherited
        # Its first with-block duplicates the held description, closes lock_fd,
        # and releases it normally. Further iterations open the usual lock.
        controller.watch(plan_digest)
    finally:
        inherited.close_unused()
        if go_fd is not None:
            os.close(go_fd)


def prepare_and_launch(launcher_digest):
    controller = load('revision2_preparation_controller', 'controller.py')
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert controller.sha(__file__) == launcher_digest, 'Exact reviewed launcher SHA required'
        assert controller.sha(controller.__file__) == CONTROLLER_SHA
        assert not any((HERE / name).exists() for name in ('plan2.json', 'fixtures.json', 'launch.json', 'watch-started.json', 'boundary.json', 'transition.json')), 'Do not overwrite prior preparation or launch evidence'
        fixture_module = load('revision2_preparation_fixtures', 'test_controller.py')
        fixture_lock = InheritedFirstOpen(os.dup(lock.fileno()))
        fixture_module.LOCK = fixture_lock
        try:
            fixture_module.main()
        finally:
            fixture_lock.close_unused()
        digest = controller.sha(controller.PLAN)
        plan = reviewed_plan(controller, digest)
        # The inherited description stays held through final outside-lock
        # bookkeeping and fresh boundary verification. No new native trial can
        # start while preparation hands ownership to the durable watcher.
        observed = controller.boundary(plan)
        assert observed is not None, 'Not an inactive qualified child-free Fann WAIT; no watcher launched'
        assert all(controller.nbody_qualified(observed['reports'][language]) for language in ('r', 'uiua'))
        assert not observed['stage_descendants'] and observed['last_stage_log_line'].startswith('WAIT LOCK ')
        controller.verify(plan['pins'])
        go_read, go_write = os.pipe()
        process = None
        try:
            with (HERE / 'watcher.log').open('ab') as log:
                process = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve()), '--watch-child',
                    '--launcher-sha256', launcher_digest, '--plan-sha256', digest,
                    '--inherited-lock-fd', str(lock.fileno()), '--handoff-go-fd', str(go_read)],
                    stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True, pass_fds=(lock.fileno(), go_read))
            os.close(go_read)
            go_read = None
            launch = dict(status='durable-watch-authorized', launched_at=controller.now(), pid=process.pid,
                watcher_identity=controller.identity(process.pid), launcher_sha256=launcher_digest,
                controller_sha256=CONTROLLER_SHA, plan=str(controller.PLAN), plan_sha256=digest,
                fixture_sha256=controller.sha(HERE / 'fixtures.json'), verified_pins=len(plan['pins']),
                common_mutex_handoff='same held open description inherited; parent closes after persisted launch',
                observed_boundary={key: value for key, value in observed.items() if key != 'reports'},
                log=str(HERE / 'watcher.log'), native_processes_signalled=False)
            assert launch['watcher_identity'] is not None, 'Watch child exited before handoff'
            controller.save_new(HERE / 'launch.json', launch)
            assert os.write(go_write, b'1') == 1
            print(json.dumps(launch, indent=2), flush=True)
        finally:
            if go_read is not None:
                os.close(go_read)
            os.close(go_write)
        # This with-block closes only the parent's description reference. The
        # watch child retains the same held lock until its controller releases.


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare-and-launch', action='store_true')
    mode.add_argument('--watch-child', action='store_true')
    parser.add_argument('--launcher-sha256', required=True)
    parser.add_argument('--plan-sha256')
    parser.add_argument('--inherited-lock-fd', type=int)
    parser.add_argument('--handoff-go-fd', type=int)
    args = parser.parse_args()
    if args.prepare_and_launch:
        prepare_and_launch(args.launcher_sha256)
    else:
        assert args.plan_sha256 and args.inherited_lock_fd is not None and args.handoff_go_fd is not None
        child_watch(args.launcher_sha256, args.plan_sha256, args.inherited_lock_fd, args.handoff_go_fd)


if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        controller = load('revision2_launcher_error_evidence', 'controller.py')
        controller.save_new(HERE / ('launcher-error-' + str(os.getpid()) + '.json'),
            dict(finished_at=controller.now(), error=str(error), error_type=type(error).__name__,
                 no_automatic_retry=True, native_processes_owned=False))
        raise
