"""Mocked ownership/restoration fixtures; no native subject or OS signal."""
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import controller as subject


class CandidateMocks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='matlab-case-mocked-')
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name)
        self.calls = []
        self.original_popen = lambda *args,**kwargs: self.fail('Unexpected real/fake subprocess launch')
        self.original_execute = lambda *args,**kwargs: dict(exit_status=0)
        self.harness = SimpleNamespace(execute=self.original_execute,
            subprocess=SimpleNamespace(Popen=self.original_popen),WORK={'regexredux':(1000,5000000)},
            INPUTS={'regexredux':('original-small','original-full')},validate=lambda *args: 'original-validation',
            kill_group=lambda pid:self.calls.append(('kill',pid)),group_members=lambda pid:[])
        self.row = dict(id='mock-matlab',language='matlab',command=['/mock/native'],benchmark='regexredux')

    def test_observer_restores_exact_original_functions(self):
        restore = subject.bounded_descriptor_observer(self.harness,self.row,64,self.output/'observer')
        self.assertIsNot(self.harness.execute,self.original_execute)
        self.assertIsNot(self.harness.subprocess.Popen,self.original_popen)
        restore()
        self.assertIs(self.harness.execute,self.original_execute)
        self.assertIs(self.harness.subprocess.Popen,self.original_popen)
        self.assertEqual(self.calls,[])

    def test_attempt_failure_preserves_raw_and_restores_overrides(self):
        raw = self.output/'attempts/001-mock-matlab.json'
        raw.parent.mkdir()
        original_bytes = b'{"qualified":true,"seconds":1.25,"exit_status":0}\n'
        guard = SimpleNamespace(OUT=self.output,REPORT=self.output/'report.json')
        guard.save = lambda path,value: Path(path).write_text(json.dumps(value))
        def failed(*args):
            self.assertIsNot(self.harness.execute,self.original_execute)
            raw.write_bytes(original_bytes)
            raise RuntimeError('mock parent exception after raw persistence')
        guard.one_attempt = failed
        previous_work,previous_input,previous_validate = self.harness.WORK['regexredux'],self.harness.INPUTS['regexredux'],self.harness.validate
        report = dict(attempts=[],active={'mock':True})
        with patch.object(subject.fcntl,'fcntl',return_value=64),patch.object(subject.os,'fstat',return_value=SimpleNamespace(st_ino=1)),patch.object(subject.os,'close') as close:
            with self.assertRaises(RuntimeError):
                subject.one(guard,self.harness,self.row,dict(pins={}),report,0,SimpleNamespace(fileno=lambda:3),
                    data=b'>CASE\nA\n',expected=b'mock complete output',case='mock')
            close.assert_called_once_with(64)
        self.assertEqual(raw.read_bytes(),original_bytes)
        wrapper = json.loads(raw.with_suffix('.wrapper.json').read_text())
        self.assertFalse(wrapper['qualified'])
        self.assertTrue(wrapper['wrapper_supervisor_failure'])
        self.assertEqual(len(report['attempts']),1)
        self.assertIsNone(report['active'])
        self.assertEqual(self.harness.WORK['regexredux'],previous_work)
        self.assertEqual(self.harness.INPUTS['regexredux'],previous_input)
        self.assertIs(self.harness.validate,previous_validate)
        self.assertIs(self.harness.execute,self.original_execute)
        self.assertIs(self.harness.subprocess.Popen,self.original_popen)

    def test_owned_observer_timeout_cleans_only_known_mock_groups(self):
        calls = self.calls
        class Native:
            pid = 501001
            def poll(self): return None
        class Observer:
            pid = 501002
            returncode = None
            def communicate(self,timeout):
                calls.append(('communicate',timeout))
                raise subprocess.TimeoutExpired('mock-lsof',timeout,output=b'p501001\n')
            def poll(self): return self.returncode
            def wait(self,timeout):
                calls.append(('wait',timeout)); self.returncode = -9; return -9
        native,observer = Native(),Observer()
        def popen(command,**kwargs):
            calls.append(('popen',command,kwargs))
            return observer if command[0] == '/usr/sbin/lsof' else native
        def execute(*args,**kwargs):
            self.harness.subprocess.Popen(self.row['command'],start_new_session=True)
            return dict(exit_status=-9)
        class Event:
            def __init__(self): self.done = False
            def is_set(self): return self.done
            def set(self): self.done = True
            def wait(self,seconds): pass
        class InlineThread:
            def __init__(self,target,args,daemon): self.target,self.args = target,args
            def start(self): self.target(*self.args)
            def join(self,timeout): calls.append(('join',timeout))
            def is_alive(self): return False
        self.harness.subprocess.Popen,self.harness.execute = popen,execute
        with patch.object(subject,'threading',SimpleNamespace(Event=Event,Thread=InlineThread)):
            restore = subject.bounded_descriptor_observer(self.harness,self.row,64,self.output/'observer')
            try: result = self.harness.execute()
            finally: restore()
        self.assertEqual([call for call in calls if call[0] == 'kill'],[('kill',observer.pid),('kill',native.pid)])
        observer_call = next(call for call in calls if call[0] == 'popen' and call[1][0] == '/usr/sbin/lsof')
        self.assertTrue(observer_call[2]['start_new_session'])
        self.assertEqual(observer_call[2]['pass_fds'],(64,))
        self.assertIn(('wait',5),calls)
        self.assertIn(('join',10),calls)
        self.assertIn('supervisor_error',result)
        self.assertEqual(result['bounded_mutex_fd_evidence']['errors'][0]['error_type'],'TimeoutExpired')
        receipt = json.loads((self.output/'observer/0001-lsof.json').read_text())
        self.assertEqual(receipt['pid'],observer.pid)
        self.assertEqual(receipt['remaining_group_members'],[])
        self.assertEqual((self.output/'observer/0001-lsof.stdout').read_bytes(),b'p501001\n')
