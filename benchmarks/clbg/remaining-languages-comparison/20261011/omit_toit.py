"""Narrow the cohort once, retaining every completed non-Toit measurement."""
import copy
import json
from collections import Counter
from datetime import datetime,timezone
from dotnet_fetch import D,sha
from probe import save

old_audit=D/'source-audit.json'
old_report=D/'results/native-timings.json'
new_audit=D/'source-audit-v2.json'
new_report=D/'results/native-timings-v2.json'
assert not new_audit.exists() and not new_report.exists()
audit=json.loads(old_audit.read_text());report=json.loads(old_report.read_text())
assert not report.get('active')
retained={key:row for key,row in report['programs'].items() if row['language']!='toit'}
transition=dict(reason='User removed Toit from the comparison.',
    at=datetime.now(timezone.utc).isoformat(),
    original_audit=str(old_audit),original_audit_sha256=sha(old_audit),
    original_report=str(old_report),original_report_sha256=sha(old_report),
    original_runner=str(D/'evidence/run-v1.py'),original_runner_sha256=sha(D/'evidence/run-v1.py'),
    retained_samples=sum(len(row['samples_seconds']) for row in retained.values()),
    excluded_samples=sum(len(row['samples_seconds']) for row in report['programs'].values() if row['language']=='toit'),
    sample_policy='Retain complete validated samples verbatim. Preserve randomized round orders after removing Toit; execute only missing samples.')
new=copy.deepcopy(audit)
new['programs']=[row for row in audit['programs'] if row['language']!='toit']
new['runtimes']={key:row for key,row in audit['runtimes'].items() if not key.startswith('toit')}
new['libraries']={key:value for key,value in audit['libraries'].items() if '/support/toit/' not in key}
new['pin_files']={key:value for key,value in audit['pin_files'].items() if 'toit' not in key.lower()}
new['pin_files'][str(D/'run.py')]=sha(D/'run.py')
for path in (D/'evidence/run-v1.py',D/'omit_toit.py',D/'metrics_final.py',old_audit,old_report):
    new['pin_files'][str(path)]=sha(path)
new['coverage']=dict(Counter(row['language'] for row in new['programs']))
new['unmeasured_alternates']=[row for row in new['unmeasured_alternates'] if 'toit' not in row['id']]
new['scope_transition']=transition
assert len(new['programs'])==224 and len(retained)==214
save(new_audit,new)
continuation=copy.deepcopy(report)
continuation['programs']=retained
continuation['orders']={bench:[[key for key in order if key in retained] for order in orders]
    for bench,orders in report['orders'].items()}
continuation['source_audit']=str(new_audit);continuation['source_audit_sha256']=sha(new_audit)
continuation['runtimes']=new['runtimes'];continuation['scope_transition']=transition
save(new_report,continuation)
assert all(continuation['programs'][key]==row for key,row in retained.items())
print(json.dumps(dict(native_candidates=len(retained)-len(new['controls']),
    retained_samples=transition['retained_samples'],excluded_samples=transition['excluded_samples'],
    total_expected_samples=3*(len(new['programs'])+2*len(new['controls'])))))
