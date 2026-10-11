"""Independent failure/admission canaries; no benchmark timings are admitted."""
import copy,sys,tempfile,unittest,os,signal,subprocess
from pathlib import Path
from unittest.mock import patch
import prepare,run,summarize

class Monitor:
 def __init__(self,power):self.observations=[];self.power_changes=[];self.errors=[]
 def start(self):pass
 def finish(self):pass

class HarnessTests(unittest.TestCase):
 def row(self,body):
  return {'key':'nbody:canary','id':'canary','benchmark':'nbody','language':'fixture','source':'fixture-only','source_sha256':'fixture-only','command':[sys.executable,'-c',body]}
 def execute(self,body,timeout=5):
  with patch.object(run.compact,'power_profile',return_value=run.REQUIRED),patch.object(run.compact,'CompilerMonitor',Monitor):return run.execute(self.row(body),False,timeout,10240)
 def test_full_output_canary(self):
  value=self.execute("print('-0.169075164\\n-0.169087605')")
  self.assertTrue(value['validation']['reference_match']);self.assertTrue(value['valid_environment']);self.assertFalse(value['full_workload'])
 def test_wrong_output_not_admitted(self):
  value=self.execute("print('0\\n0')")
  self.assertEqual(value['error_type'],'OutputValidationError')
 def test_failure_archived(self):
  value=self.execute("import sys;print('retained');sys.stderr.write('diagnostic');sys.exit(3)")
  self.assertEqual(value['exit_status'],3);self.assertEqual(value['stderr'],'diagnostic');self.assertIn('retained',value['stdout_prefix'])
 def test_timeout_kills_and_retains_attempt(self):
  value=self.execute('import time;time.sleep(10)',timeout=0.03)
  self.assertEqual(value['error_type'],'TimeoutError');self.assertNotEqual(value['exit_status'],0)
 def test_success_with_orphan_worker_rejected_and_cleaned(self):
  value=self.execute("import os,time;child=os.fork();time.sleep(10) if child==0 else print('-0.169075164\\n-0.169087605')")
  self.assertIn('benchmark process group',value['error']);self.assertTrue(value['remaining_group_members'])
  self.assertFalse(run.group_members(value['remaining_group_members'][0]['pgid']))
 def test_signal_handlers_override_background_ignore(self):
  probe="import signal,run;signal.signal(signal.SIGINT,signal.SIG_IGN);run.install_interrupt_handlers();assert all(signal.getsignal(s)==run.interrupted for s in [signal.SIGINT,signal.SIGTERM,signal.SIGHUP]);print('READY',flush=True);signal.pause()"
  p=subprocess.Popen([sys.executable,'-c',probe],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  self.assertEqual(p.stdout.readline().strip(),'READY');p.send_signal(signal.SIGTERM);out,err=p.communicate(timeout=5)
  self.assertNotEqual(p.returncode,0);self.assertIn('KeyboardInterrupt',err)
 def test_monitor_failure_invalidates_without_losing_output(self):
  class FailingMonitor(Monitor):
   def finish(self):self.errors.append('canary');raise RuntimeError('canary')
  with patch.object(run.compact,'power_profile',return_value=run.REQUIRED),patch.object(run.compact,'CompilerMonitor',FailingMonitor):value=run.execute(self.row("print('-0.169075164\\n-0.169087605')"),False,5,10240)
  self.assertTrue(value['validation']['reference_match']);self.assertFalse(value['valid_environment']);self.assertEqual(value['monitor_finish_error'],'canary')
 def test_empty_interrupt_message_remains_interruption(self):
  class WaitInterrupted:
   def __init__(self,*args,**kwargs):self.pid=99999999;self.returncode=-9
   def wait(self):
    if not hasattr(self,'already'):self.already=True;raise KeyboardInterrupt()
    return self.returncode
  with patch.object(run.compact,'power_profile',return_value=run.REQUIRED),patch.object(run.compact,'CompilerMonitor',Monitor),patch.object(run.subprocess,'Popen',WaitInterrupted),patch.object(run,'kill_group'):
   value=run.execute(self.row('unused'),False,5,10240)
  self.assertEqual(value['error_type'],'KeyboardInterrupt');self.assertEqual(value['error'],'KeyboardInterrupt');self.assertNotIn('validation',value)
 def test_pin_changed_content(self):
  with tempfile.TemporaryDirectory() as directory:
   path=Path(directory)/'source';path.write_text('before');guard=run.PinGuard({str(path):prepare.sha(path)});guard.verify();path.write_text('after')
   with self.assertRaises(AssertionError):guard.verify()
 def test_alias_requires_identical_execution(self):
  a={'id':'one','key':'nbody:one','benchmark':'nbody','language':'fixture','source':'/source/one','source_sha256':'identical','command':['runtime','/source/one'],'runtime_environment':{}}
  b=copy.deepcopy(a);b.update(id='two',key='nbody:two',source='/source/two',command=['runtime','/source/two'],timing_alias_of='one')
  self.assertEqual(prepare.resolve_aliases([a,b])[1]['timing_alias_of'],'nbody:one')
  b['runtime_environment']={'workers':'8'}
  with self.assertRaises(AssertionError):prepare.resolve_aliases([a,b])
 def test_admission_needs_three_valid_full_evidence_rows(self):
  row={'key':'fixture','source_sha256':'fixture','samples_seconds':[1,2],'sample_validation':[{'reference_match':True}]*2};report={'sample_count':3,'attempts':[{'key':'fixture','source_sha256':'fixture','seconds':n,'full_workload':True,'valid_environment':True,'validation':{'reference_match':True}} for n in [1,2,3]]}
  self.assertFalse(summarize.admitted(row,report))
  row['samples_seconds'].append(3);row['sample_validation'].append({'reference_match':True});self.assertTrue(summarize.admitted(row,report))
  row['sample_validation'][0]={'reference_match':False};self.assertFalse(summarize.admitted(row,report))
if __name__=='__main__':unittest.main()
