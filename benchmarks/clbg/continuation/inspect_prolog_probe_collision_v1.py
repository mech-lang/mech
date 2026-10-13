"""Metadata-only review of closed post-slice version/probe failure."""
from pathlib import Path
import fcntl,hashlib,json,os
ROOT=Path('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech')
C=ROOT/'benchmarks/clbg/continuation';H=ROOT/'benchmarks/clbg/prolog-comparison/20261013'
P=H/'.build/swi-runtime-postslice-v1';J=C/'owned-jobs/prolog-runtime-postslice-discovery-v1';O=C/'jobs/prolog-probe-collision-review-v1'

def sha(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def save(path,value):
    with path.open('x') as f:json.dump(value,f,indent=2);f.write('\n')

print('WAIT common mutex: closed Prolog version/probe collision review',flush=True)
with open('/tmp/mech-clbg-timing.lock','r+') as common:
    fcntl.flock(common,fcntl.LOCK_EX);h=os.fstat(common.fileno());n=os.stat('/tmp/mech-clbg-timing.lock');assert(h.st_dev,h.st_ino)==(n.st_dev,n.st_ino)
    with (J/'owner.lock').open('r+') as lease:
        fcntl.flock(lease,fcntl.LOCK_EX|fcntl.LOCK_NB);O.mkdir(parents=True)
        pins={str(Path(__file__).resolve()):sha(Path(__file__).resolve())}
        def read(path):pins[str(path)]=sha(path);return json.loads(path.read_text())
        terminal=read(J/'terminal.json');request=read(J/'request.json');started=read(J/'command-started.json');worker=read(J/'worker-started.json')
        assert terminal['status']=='command-failed' and terminal['exit_code']==1 and terminal['stop_signals']==[]
        assert worker['worker_identity']==terminal['worker_identity'] and started['command_identity']==terminal['command_identity']
        assert request['command']==started['command']==worker['command'] and request['cwd']==started['cwd']==worker['cwd']==str(ROOT)
        report=read(P/'report.json');t=read(P/'terminal-pin.json')
        assert report['status']==t['status']=='stopped-unqualified' and report['error_type']=='FileExistsError'
        assert report['terminal_evidence_complete'] is True and report['pins']==t['partial_evidence_pins']
        assert t['report']==str(P/'report.json') and t['report_sha256']==sha(P/'report.json')
        for path,digest in report['pins'].items():assert sha(path)==digest;pins[path]=digest
        version=read(P/'runtime/version.json');active=read(P/'runtime/feature-probe.active.json');raw=read(P/'runtime/feature-probe.json')
        assert version['command'][1:]==['--version'] and version['exit_status']==version['final_exit_status']==0
        assert version['inherited_common_mutex'] is True and not any(version.get(k) for k in ('error','cleanup_error','cleanup_errors'))
        assert (P/'runtime/version.stdout').read_text().strip()=='SWI-Prolog version 10.0.2 for arm64-darwin'
        assert active['command'][-1]==str(P/'runtime/feature-probe.json')
        assert 'command' not in raw and raw['runtime']['version_components']==[10,0,2]
        assert raw['runtime']['home']==str(H/'.build/swi-runtime-directory-v1/arm64-app/SWI-Prolog.app/Contents/Resources/swipl')
        assert raw['runtime']['bounded'] is False and raw['runtime']['gc'] is True
        assert raw['large']==(1<<220)+12345 and raw['quotient']==raw['large']//97 and raw['remainder']==raw['large']%97
        assert (raw['hash_value'],raw['hash_size'],raw['regex_matches'],raw['regex_replacement'])==(5,4,2,'xbx')
        assert (P/'runtime/feature-probe.stderr').read_bytes()==b''
        stderr=(J/'command.stderr').read_text();assert "('Owned command failed', 'feature-probe', 1)" in stderr and 'FileExistsError' in stderr
        for name in ('command.stdout','command.stderr','worker.log'):pins[str(J/name)]=sha(J/name)
        assert not (P/'images').exists()
        probe=H/'drafts/private-v1/tools/runtime_probe.pl';prep=H/'drafts/private-v1/tools/prepare_qlf.pl';source=(probe.read_text(),prep.read_text())
        assert 'halt(0).' in source[0] and ':- initialization(main, main).' in source[1]
        home=Path(raw['runtime']['home']);boot=home/'boot/init.pl';pins[str(boot)]=sha(boot)
        lines=boot.read_text().splitlines();excerpts=[]
        for index,line in enumerate(lines):
            if 'initialization' in line and ('main' in line or '$run' in line):excerpts.append(dict(line=index+1,text='\n'.join(lines[max(0,index-3):index+6])))
        for path,digest in pins.items():assert sha(path)==digest
        value=dict(status='closed-version-passed-probe-result-retained-owned-receipt-collision',
            failed_report=str(P/'report.json'),failed_report_sha256=sha(P/'report.json'),failed_terminal=str(P/'terminal-pin.json'),failed_terminal_sha256=sha(P/'terminal-pin.json'),
            durable_terminal=str(J/'terminal.json'),durable_finished_at=terminal['finished_at'],private_lease_closed=True,
            version_receipt=str(P/'runtime/version.json'),version_receipt_sha256=sha(P/'runtime/version.json'),version_exit_status=0,
            actual_probe_command=active['command'],raw_probe_result=str(P/'runtime/feature-probe.json'),raw_probe_result_sha256=sha(P/'runtime/feature-probe.json'),
            raw_probe_observations=raw,owned_probe_exit_one_witness=str(J/'command.stderr'),
            probe_result_is_not_normal_owned_receipt=True,no_image_closure_started=True,
            probe_checked_already_halts_zero=True,preparation_registers_main_initialization=True,
            published_boot_source=str(boot),published_boot_source_sha256=sha(boot),published_boot_excerpts=excerpts,
            actual_native_commands=0,fixture_reexecutions=0,pins=pins)
        save(O/'review.json',value);save(O/'terminal.json',dict(status=value['status'],body_error=None,integrity_errors=[],review=str(O/'review.json'),review_sha256=sha(O/'review.json'),actual_native_commands=0))
        print(json.dumps({key:value[key] for key in ('status','failed_report_sha256','failed_terminal_sha256','version_receipt_sha256','raw_probe_result_sha256','actual_probe_command','published_boot_excerpts')}),flush=True)
        print(json.dumps(dict(review=str(O/'review.json'),review_sha256=sha(O/'review.json'),terminal_sha256=sha(O/'terminal.json'),verified_pins=len(pins))),flush=True)
