"""Metadata-only inspection of the closed private Prolog post-slice failure."""
from pathlib import Path
import fcntl
import hashlib
import json
import os
import plistlib
from datetime import datetime, timezone

ROOT=Path('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech')
CONT=ROOT/'benchmarks/clbg/continuation'
COHORT=ROOT/'benchmarks/clbg/prolog-comparison/20261013'
FAILED=COHORT/'.build/swi-runtime-directory-v1'
JOB=CONT/'owned-jobs/prolog-private-runtime-directory-v1'
OUT=CONT/'jobs/prolog-postslice-layout-review-v3'
LOCK=Path('/tmp/mech-clbg-timing.lock')

def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()

def save(path,value):
    with path.open('x') as stream: json.dump(value,stream,indent=2);stream.write('\n')

print('WAIT common mutex: closed Prolog post-slice layout and pin inspection',flush=True)
with LOCK.open('r+') as common:
    fcntl.flock(common,fcntl.LOCK_EX)
    held,named=os.fstat(common.fileno()),LOCK.stat()
    assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino) and held.st_ino==79859134
    with (JOB/'owner.lock').open('r+') as private:
        fcntl.flock(private,fcntl.LOCK_EX | fcntl.LOCK_NB)
        OUT.mkdir(parents=True)
        pins={str(Path(__file__).resolve()):sha(Path(__file__).resolve())}
        def read(path):
            path=Path(path);pins[str(path)]=sha(path);return json.loads(path.read_text())
        request,launch,worker,started,terminal=[read(JOB/name) for name in (
            'request.json','launch.json','worker-started.json','command-started.json','terminal.json')]
        assert terminal['status']=='command-failed' and type(terminal['exit_code']) is int and terminal['exit_code']==1
        assert terminal['actual_command_started'] is True and terminal['stop_signals']==[]
        assert not any(terminal.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
        assert request['command']==launch['command']==worker['command']==started['command']
        assert request['cwd']==launch['cwd']==worker['cwd']==started['cwd']==str(ROOT)
        assert request['command'][0]=='/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
        assert request['command'][1:5]==['-I','-S','-B','-u']
        assert request['command'][5]==str(COHORT/'revisions/runtime-directory-v1/bootstrap_runtime_directory_v1.py')
        assert worker['worker_identity']==terminal['worker_identity']
        assert started['command_pid']==terminal['command_pid'] and started['command_identity']==terminal['command_identity']
        assert started['command_identity_observation_missing'] is terminal['command_identity_observation_missing'] is False
        assert started['command_identity']['listing'].split()[7:]==request['command']
        report,evidence=read(FAILED/'report.json'),read(FAILED/'terminal-pin.json')
        assert report['status']==evidence['status']=='stopped-unqualified'
        assert report['error_type']=='AssertionError' and report['error']=='Actual bundle home differs; retain and review this layout'
        assert report['terminal_evidence_complete'] is True and not report.get('terminal_artifact_errors')
        assert evidence['report']==str(FAILED/'report.json') and evidence['report_sha256']==sha(FAILED/'report.json')
        assert report['pins']==evidence['partial_evidence_pins'] and evidence['inherited_common_mutex'] is True
        assert report['runtime_configuration_admitted'] is report['source_work_qualified'] is False and report['full_processes_authorized']==0
        assert report['requested_mount_detached'] is True and report['attach_device_closure']=='exact-requested-mount-detached'
        assert not report.get('owned_mount_requires_root_review') and not report.get('owned_mount_cleanup_error')
        mount=FAILED/'owned-readonly-mount'
        assert not os.path.ismount(mount)
        inventory=report['attach_inventory']
        assert inventory['requested_mount']==str(mount) and inventory['all_observed_mounts_are_requested'] is True
        assert len(inventory['actual_mount_entities'])==len(inventory['exact_requested_mount_entities'])==1
        for path,digest in report['pins'].items():
            assert sha(path)==digest,('Changed original pinned bytes',path)
            assert path not in pins or pins[path]==digest
            pins[path]=digest
        receipts={}
        for relative,command in (
            ('diskimage/attach',['/usr/bin/hdiutil','attach','-readonly','-nobrowse','-noautoopen','-mountpoint',str(mount),'-plist',str(FAILED/'swipl-10.0.2-1.fat.dmg')]),
            ('diskimage/detach',['/usr/bin/hdiutil','detach',str(mount)])):
            value=read(FAILED/(relative+'.json'))
            assert value['command']==command and value['inherited_common_mutex'] is True
            assert type(value['exit_status']) is type(value['final_exit_status']) is int and value['exit_status']==value['final_exit_status']==0
            assert value['preparation_only'] is value['never_chart_eligible'] is True
            assert not any(value.get(key) for key in ('remaining_group_members','cleanup_error','cleanup_errors','error','error_type'))
            receipts[relative]=value
        attach=plistlib.loads((FAILED/'diskimage/attach.stdout').read_bytes())
        assert attach['system-entities']==inventory['system_entities']
        original_roots=list((FAILED/'original-app').glob('*.app'))
        native_roots=list((FAILED/'arm64-app').glob('*.app'))
        assert len(original_roots)==len(native_roots)==1 and original_roots[0].name==native_roots[0].name
        original,native=original_roots[0],native_roots[0]
        assert original.is_dir() and native.is_dir() and not original.is_symlink() and not native.is_symlink()
        layout=[];boot=[]
        for path in sorted(native.rglob('*')):
            relative=str(path.relative_to(native))
            row=dict(relative=relative,directory=path.is_dir(),file=path.is_file(),symlink=path.is_symlink())
            if path.is_symlink():
                row.update(link=os.readlink(path),resolved=str(path.resolve(strict=True).relative_to(native)))
            if path.is_file():
                assert report['pins'][str(path)]==sha(path)
                row.update(bytes=path.stat().st_size,sha256=report['pins'][str(path)])
            else: assert path.is_dir()
            layout.append(row)
            if path.name=='boot.prc' and path.is_file():boot.append(path)
        assert len(boot)==1
        home=boot[0].parent
        assert home.is_dir() and (home/'library').is_dir()
        assert not (native/'Contents/swipl').exists()
        slice_receipt=read(FAILED/'slice/arm64.json')
        command=slice_receipt['command'];assert command[0]=='/usr/bin/lipo' and command[2:5]==['-thin','arm64','-output']
        fat_binary,binary=Path(command[1]),Path(command[5])
        assert fat_binary.is_relative_to(original) and binary.is_relative_to(native)
        assert fat_binary.relative_to(original)==binary.relative_to(native)
        assert binary.is_file() and fat_binary.is_file()
        assert type(slice_receipt['exit_status']) is type(slice_receipt['final_exit_status']) is int and slice_receipt['exit_status']==slice_receipt['final_exit_status']==0
        assert slice_receipt['inherited_common_mutex'] is slice_receipt['preparation_only'] is slice_receipt['never_chart_eligible'] is True
        assert not any(slice_receipt.get(key) for key in ('remaining_group_members','cleanup_error','cleanup_errors','error','error_type'))
        with binary.open('rb') as stream: header=stream.read(32)
        assert header[:4]==bytes.fromhex('cffaedfe') and int.from_bytes(header[4:8],'little')==0x0100000c
        assert not (FAILED/'runtime').exists()
        for name in ('command.stdout','command.stderr','worker.log'):pins[str(JOB/name)]=sha(JOB/name)
        info=(native/'Contents/Info.plist').read_text()
        for path,digest in pins.items():assert sha(path)==digest
        save(OUT/'native-layout.json',layout)
        review=dict(status='closed-postslice-layout-and-pin-inspection-passed',completed_at=datetime.now(timezone.utc).isoformat(),
            durable_job=str(JOB),durable_finished_at=terminal['finished_at'],durable_exit_code=1,stop_signals=[],private_lease_closed=True,
            failed_report=str(FAILED/'report.json'),failed_report_sha256=sha(FAILED/'report.json'),
            failed_terminal=str(FAILED/'terminal-pin.json'),failed_terminal_sha256=sha(FAILED/'terminal-pin.json'),
            source_extraction=report['source_extraction'],exact_owned_mount_detached=True,current_requested_mount_is_mount=False,
            original_app=str(original),native_app=str(native),native_layout=str(OUT/'native-layout.json'),native_layout_sha256=sha(OUT/'native-layout.json'),
            unique_boot=str(boot[0]),home=str(home),home_library=str(home/'library'),
            fat_binary=str(fat_binary),fat_binary_sha256=sha(fat_binary),binary=str(binary),binary_sha256=sha(binary),
            slice_receipt=str(FAILED/'slice/arm64.json'),slice_receipt_sha256=sha(FAILED/'slice/arm64.json'),
            native_header_hex=header.hex(),publisher_info_plist_text=info,publisher_info_plist_parsed=False,
            version_commands_reexecuted=0,actual_native_commands=0,actual_fixture_reexecutions=0,
            common_mutex_inode=held.st_ino,pins=pins)
        save(OUT/'review.json',review)
        for path,digest in pins.items():assert sha(path)==digest
        save(OUT/'terminal.json',dict(status=review['status'],body_error=None,integrity_errors=[],review=str(OUT/'review.json'),
            review_sha256=sha(OUT/'review.json'),common_mutex_inode=held.st_ino,actual_native_commands=0))
        print(json.dumps({key:review[key] for key in ('status','home','binary','binary_sha256','fat_binary_sha256','failed_report_sha256','failed_terminal_sha256','slice_receipt_sha256','exact_owned_mount_detached')}),flush=True)
        print(json.dumps(dict(review=str(OUT/'review.json'),review_sha256=sha(OUT/'review.json'),terminal_sha256=sha(OUT/'terminal.json'),
            layout_rows=len(layout),verified_pins=len(pins))),flush=True)
