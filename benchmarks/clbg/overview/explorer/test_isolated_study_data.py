"""Focused malformed-ledger fixtures; no benchmarking or runtime execution."""
from copy import deepcopy
import json
import statistics
import tempfile
import unittest
from pathlib import Path
from isolated_study_data import validate_full_records, POWER
from mandelbrot_study_data import _fingerprint
from uiua_fann_study_data import add_uiua_fann_study, DIGEST, LOCS, REFERENCE, REPORT, CHECKS

def fixture():
    rows=[]
    for identity,loc in LOCS.items():
        row=dict(id=identity,key='fannkuchredux:'+identity,benchmark='fannkuchredux',language='mech' if identity.startswith('mech') else 'uiua',source='/source/'+identity,source_sha256='a'*64,command=['/runtime/'+identity,'/source/'+identity,'{n}'],cwd='/work',runtime_environment={},loc=loc,maximum_workers=8)
        rows.append(row)
    orders=[dict(round=i,profiles=list(LOCS)) for i in range(1,4)]
    manifest=dict(samples=3,size=12,maximum_compute_workers=8,required_power_profile=deepcopy(POWER),full_output_reference=deepcopy(REFERENCE),profiles=rows,orders=orders,maximum_rss_mib=8192,timeout_seconds=14400,environment_retry_limit=3)
    attempts=[];profiles={r['id']:dict(loc=r['loc'],samples_seconds=[]) for r in rows}
    for ordered in orders:
        for index,row in enumerate(rows):
            command=[p.replace('{n}','12') for p in row['command']];seconds=float(ordered['round']+index)
            a=dict(attempt=len(attempts)+1,round=ordered['round'],**{k:row[k] for k in ('id','key','benchmark','language','source_sha256','cwd')},command=command,runtime_environment={},execution_fingerprint=json.dumps(_fingerprint(row)),sampler_wait_status=0,exit_status=0,maximum_rss_mib=8192,peak_process_group_rss_kib=1024,timeout_seconds=14400,full_workload=True,workload_n=12,pins_unchanged_at_end=True,qualified=True,valid_environment=True,power_at_start=deepcopy(POWER),power_at_end=deepcopy(POWER),validation=dict(bytes=REFERENCE['bytes'],sha256=REFERENCE['sha256'],reference_match=True),stdout_bytes=REFERENCE['bytes'],stdout_sha256=REFERENCE['sha256'],supervised_native_group=dict(pid=100+len(attempts),pgid=100+len(attempts),command=command,inherited_common_mutex=True),seconds=seconds,qualification_checks={key:True for key in CHECKS})
            for field in ('remaining_group_members','competing_compilers','power_changes','monitor_errors','resource_monitor_errors','supervisor_remaining_members'):a[field]=[]
            attempts.append(a);profiles[row['id']]['samples_seconds'].append(seconds)
    for p in profiles.values():p['median_seconds']=statistics.median(p['samples_seconds']);p['sample_stdev_seconds']=statistics.stdev(p['samples_seconds'])
    report=dict(status='complete',active=None,manifest_sha256=DIGEST,pins_unchanged_at_end=True,orders=orders,attempts=attempts,profiles=profiles)
    return manifest,report

def validate(m,r,**kwargs):return validate_full_records(m,r,DIGEST,benchmark='fannkuchredux',size=12,locs=LOCS,workers=8,reference=REFERENCE,expected_checks=CHECKS,**kwargs)

class LedgerTests(unittest.TestCase):
    def reject(self,edit):
        m,r=fixture();edit(m,r)
        with self.assertRaises(AssertionError):validate(m,r)
    def test_complete_nine_records(self):
        m,r=fixture();rows,discarded=validate(m,r);self.assertEqual(set(rows),set(LOCS));self.assertEqual(discarded,[])
        self.assertEqual(rows['uiua-compact']['samples_seconds'],[1.,2.,3.])
    def test_running_report(self):self.reject(lambda m,r:r.update(status='running'))
    def test_false_end_pin(self):self.reject(lambda m,r:r['attempts'][0].update(pins_unchanged_at_end=False))
    def test_wrong_manifest_digest(self):self.reject(lambda m,r:r.update(manifest_sha256='b'*64))
    def test_wrong_source(self):self.reject(lambda m,r:r['attempts'][0].update(source_sha256='b'*64))
    def test_shortened_workload(self):self.reject(lambda m,r:r['attempts'][0].update(workload_n=8))
    def test_nonfull_workload(self):self.reject(lambda m,r:r['attempts'][0].update(full_workload=False))
    def test_wrong_output(self):self.reject(lambda m,r:r['attempts'][0]['validation'].update(sha256='b'*64))
    def test_wrong_raw_output(self):self.reject(lambda m,r:r['attempts'][0].update(stdout_bytes=0))
    def test_missing_owned_group(self):self.reject(lambda m,r:r['attempts'][0]['supervised_native_group'].update(inherited_common_mutex=False))
    def test_wrong_native_command(self):self.reject(lambda m,r:r['attempts'][0]['supervised_native_group'].update(command=['other']))
    def test_leftover_group(self):self.reject(lambda m,r:r['attempts'][0].update(supervisor_remaining_members=[{'pid':9}]))
    def test_wrong_source_loc(self):self.reject(lambda m,r:m['profiles'][0].update(loc=19))
    def test_stats_do_not_match_samples(self):self.reject(lambda m,r:r['profiles']['uiua-compact'].update(median_seconds=.1))
    def test_samples_do_not_match_records(self):self.reject(lambda m,r:r['profiles']['uiua-compact'].update(samples_seconds=[.1,.1,.1]))
    def test_missing_actual_record(self):self.reject(lambda m,r:r['attempts'].pop())
    def test_extra_actual_record(self):self.reject(lambda m,r:r['attempts'].append(deepcopy(r['attempts'][0])))
    def test_swapped_attempt_order(self):
        def edit(m,r):r['attempts'][0],r['attempts'][1]=r['attempts'][1],r['attempts'][0]
        self.reject(edit)
    def test_wrong_runtime_environment(self):self.reject(lambda m,r:r['attempts'][0].update(runtime_environment={'JIT':'off'}))
    def test_wrong_power(self):self.reject(lambda m,r:r['attempts'][0].update(power_at_start={'source':'Battery Power','low_power_mode':0}))
    def test_resource_ceiling(self):self.reject(lambda m,r:r['attempts'][0].update(peak_process_group_rss_kib=8192*1024+1))
    def test_false_qualification_flag(self):self.reject(lambda m,r:r['attempts'][0].update(qualified=False,retryable_environment_only=True))
    def test_missing_captured_check(self):self.reject(lambda m,r:r['attempts'][0]['qualification_checks'].pop('native_inherited_common_mutex'))
    def test_false_captured_check(self):self.reject(lambda m,r:r['attempts'][0]['qualification_checks'].update(native_inherited_common_mutex=False))
    def test_probe_marked_never_eligible(self):self.reject(lambda m,r:r['attempts'][0].update(never_chart_eligible=True))
    def test_no_admission_for_missing_report(self):
        with tempfile.TemporaryDirectory() as temp:
            data={'untouched':True};add_uiua_fann_study(Path(temp),data,None,None,None,None);self.assertEqual(data,{'untouched':True})
    def test_no_admission_for_running_report(self):
        with tempfile.TemporaryDirectory() as temp:
            p=Path(temp)/REPORT;p.parent.mkdir(parents=True);p.write_text('{"status":"running"}')
            data={'untouched':True};add_uiua_fann_study(Path(temp),data,None,None,None,None);self.assertEqual(data,{'untouched':True})
if __name__=='__main__':unittest.main()
