"""Continue only missing runtime discovery from the closed ARM64 slice.

Source candidate. No downloads, extraction, mounts, app copies, slicing, fixture
or benchmark execution. Original failed outputs stay immutable.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys

PACKET=Path(__file__).resolve().parent
HERE=PACKET.parents[1]
CLBG=HERE.parents[1]
LOCK=Path('/tmp/mech-clbg-timing.lock')
PRIOR=HERE/'.build/swi-runtime-directory-v1'
DURABLE=CLBG/'continuation/owned-jobs/prolog-private-runtime-directory-v1'
REVIEW=CLBG/'continuation/jobs/prolog-postslice-layout-review-v3/review.json'
REVIEW_TERMINAL=REVIEW.with_name('terminal.json')
OLD_WORKER=HERE/'revisions/runtime-directory-v1/bootstrap_runtime_directory_v1.py'
OLD_WORKER_SHA='a7f97dc59580154db6d790498778f3ab883047f368e03733b9fa65b6b7add783'
FAILED_SHA='3acb7a2c6dc10618e9eead402575bdc0b60015c6dc1feadca926f871289ed69c'
FAILED_TERMINAL_SHA='9d99b5a07bc771979b4b0ed52e7e76d73918e9a5c41126c00664a3e96f01c64e'
REVIEW_SHA='da4b6493af38e5af06cbd080346577292eb4113e51632bfd15394cf509fc3f4d'
REVIEW_TERMINAL_SHA='02439e159cc1d5665ee5ededc7e241130ca72cec308f63484e715120b5ecda95'
BINARY_REL=Path('Contents/MacOS/swipl')
HOME_REL=Path('Contents/Resources/swipl')
BINARY_SHA='554814b1da7c0c30595adafb758145e42a6134c696b47c8870d359008246443e'
FAT_BINARY_SHA='e47fc404978145ade0ddde49a19542ca3f92515b4eea7051d33d5ff368896e4b'
SLICE_SHA='d6bc7b7920a451de54c6160450adc135324e9d7027b0497ad353c014f7bf0f50'
PROPOSAL_SHA='5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88'
RUNTIME_SHA='c72f7ab0d41b195acafff131e5f2cd7c61577a9fa14898af15e4c543b8daa322'


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')


def verify(pins):
    for path,digest in pins.items():assert sha(path)==digest,('Changed input bytes',path)


def merge(*groups):
    result={}
    for group in groups:
        for path,digest in group.items():
            assert path not in result or result[path]==digest
            result[path]=digest
    return result


def fresh(path,name,pins):
    path=Path(path).resolve(strict=True)
    assert name not in sys.modules and sha(path)==pins[str(path)]
    source=path.read_bytes();assert hashlib.sha256(source).hexdigest()==pins[str(path)]
    module=importlib.util.module_from_spec(importlib.util.spec_from_file_location(name,path))
    sys.modules[name]=module;exec(compile(source,str(path),'exec'),module.__dict__)
    assert module is sys.modules[name] and Path(module.__file__).resolve()==path
    assert sha(path)==pins[str(path)]
    return module


def original_inputs(args,pins):
    expected_digests=(
        (args.failed_report_sha256,FAILED_SHA),(args.failed_terminal_sha256,FAILED_TERMINAL_SHA),
        (args.layout_review_sha256,REVIEW_SHA),(args.layout_terminal_sha256,REVIEW_TERMINAL_SHA),
        (args.proposal_sha256,PROPOSAL_SHA),(args.runtime_proposal_sha256,RUNTIME_SHA))
    assert all(value==expected for value,expected in expected_digests)
    def read(path,digest=None):
        actual=sha(path);assert digest is None or actual==digest
        assert str(path) not in pins or pins[str(path)]==actual
        pins[str(path)]=actual;return json.loads(path.read_text())
    review=read(REVIEW,args.layout_review_sha256)
    terminal=read(REVIEW_TERMINAL,args.layout_terminal_sha256)
    assert review['status']==terminal['status']=='closed-postslice-layout-and-pin-inspection-passed'
    assert terminal['body_error'] is None and terminal['integrity_errors']==[]
    assert terminal['review']==str(REVIEW) and terminal['review_sha256']==args.layout_review_sha256
    assert review['common_mutex_inode']==terminal['common_mutex_inode']==79859134
    assert review['private_lease_closed'] is review['exact_owned_mount_detached'] is True
    assert review['current_requested_mount_is_mount'] is False
    for field in ('actual_native_commands','actual_fixture_reexecutions','version_commands_reexecuted'):
        assert type(review[field]) is int and review[field]==0
    assert review['failed_report']==str(PRIOR/'report.json') and review['failed_report_sha256']==args.failed_report_sha256
    assert review['failed_terminal']==str(PRIOR/'terminal-pin.json') and review['failed_terminal_sha256']==args.failed_terminal_sha256
    pins.update(merge(pins,review['pins']));verify(pins)
    failed=read(PRIOR/'report.json',args.failed_report_sha256)
    failed_terminal=read(PRIOR/'terminal-pin.json',args.failed_terminal_sha256)
    assert failed['status']==failed_terminal['status']=='stopped-unqualified'
    assert failed['error_type']=='AssertionError' and failed['error']=='Actual bundle home differs; retain and review this layout'
    assert failed['terminal_evidence_complete'] is True and not failed.get('terminal_artifact_errors')
    assert failed_terminal['report']==str(PRIOR/'report.json') and failed_terminal['report_sha256']==args.failed_report_sha256
    assert failed['pins']==failed_terminal['partial_evidence_pins'] and failed_terminal['inherited_common_mutex'] is True
    assert failed['runtime_configuration_admitted'] is failed['source_work_qualified'] is False and failed['full_processes_authorized']==0
    assert failed['proposal_sha256']==args.proposal_sha256 and failed['runtime_proposal_sha256']==args.runtime_proposal_sha256
    assert failed['requested_mount_detached'] is True and failed['attach_device_closure']=='exact-requested-mount-detached'
    assert not any(failed.get(key) for key in ('owned_mount_requires_root_review','owned_mount_cleanup_error'))
    mount=PRIOR/'owned-readonly-mount';assert not os.path.ismount(mount)
    inventory=failed['attach_inventory']
    assert inventory['requested_mount']==str(mount) and inventory['all_observed_mounts_are_requested'] is True
    assert len(inventory['actual_mount_entities'])==len(inventory['exact_requested_mount_entities'])==1
    pins.update(merge(pins,failed['pins']));verify(pins)
    request=read(DURABLE/'request.json');launch=read(DURABLE/'launch.json')
    worker=read(DURABLE/'worker-started.json');started=read(DURABLE/'command-started.json');ended=read(DURABLE/'terminal.json')
    expected=['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12','-I','-S','-B','-u',str(OLD_WORKER),
        '--bootstrap','--proposal-sha256',PROPOSAL_SHA,'--runtime-proposal-sha256',RUNTIME_SHA,
        '--worker-sha256',OLD_WORKER_SHA,'--extractor-sha256','3d0c6778454b26cdfaca3cd945927183795b820a1c1904e9e6f4ee4dce11ee5e',
        '--fixture-receipt',str(HERE/'revisions/runtime-directory-v1/jobs/fixtures-v1/fixture-receipt.json'),
        '--fixture-sha256','6bd2f38bef28c017c45395e05a307cb1005d03e4540ca92b3832754bc6977ad3',
        '--fixture-terminal-sha256','d4273be3af3161410f39a9125bba453beb25cd7461f5df691bc5652d254ccd3f']
    assert request['command']==launch['command']==worker['command']==started['command']==expected
    assert request['cwd']==launch['cwd']==worker['cwd']==started['cwd']==str(CLBG.parent.parent)
    assert request['schema_version']==1 and request['automatic_retry'] is launch['automatic_retry'] is ended['automatic_retry'] is False
    assert request['common_mutex_owned_by_target_command'] is worker['common_mutex_owned_by_target_command'] is True
    assert worker['inherited_private_lease'] is worker['detached_session'] is True
    assert ended['status']=='command-failed' and type(ended['exit_code']) is int and ended['exit_code']==1
    assert ended['actual_command_started'] is True and ended['stop_signals']==[]
    assert not any(ended.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
    assert worker['worker_identity']==ended['worker_identity']
    assert started['command_pid']==ended['command_pid'] and started['command_identity']==ended['command_identity']
    assert started['command_identity_observation_missing'] is ended['command_identity_observation_missing'] is False
    assert started['command_identity']['listing'].split()[7:]==expected
    for name in ('command.stdout','command.stderr','worker.log'):
        path=DURABLE/name;assert pins[str(path)]==sha(path)
    fixture_path=HERE/'revisions/runtime-directory-v1/jobs/fixtures-v1/fixture-receipt.json'
    fixture=read(fixture_path,'6bd2f38bef28c017c45395e05a307cb1005d03e4540ca92b3832754bc6977ad3')
    fixture_terminal=read(fixture_path.with_name('terminal.json'),'d4273be3af3161410f39a9125bba453beb25cd7461f5df691bc5652d254ccd3f')
    fixture_ids=tuple('test_extract_source.DirectoryContracts.'+name for name in (
        'test_repeated_exact_directory_identity_allows_only_unapplied_timestamp_difference',
        'test_regular_link_and_cross_type_duplicates_never_overwrite',
        'test_alias_escape_metadata_and_unselected_member_types_reject',
        'test_existing_destination_and_dangling_link_keep_original_policy'))
    assert fixture['status']==fixture_terminal['status']=='passed-directory-extraction-fixtures'
    assert fixture['execution_completed'] is True and type(fixture['tests_run']) is int and fixture['tests_run']==4
    assert fixture_terminal['body_error'] is None and fixture_terminal['integrity_errors']==[]
    assert fixture_terminal['receipt']==str(fixture_path) and fixture_terminal['receipt_sha256']==sha(fixture_path)
    assert fixture['source_pins']==fixture_terminal['source_pins'] and fixture['available_evidence_pins']==fixture_terminal['available_evidence_pins']
    assert fixture['inherited_common_mutex'] is fixture_terminal['inherited_common_mutex'] is True
    for field in ('expected_ids','discovered_ids','actual_ids'):
        assert len(fixture[field])==len(set(fixture[field]))==4 and set(fixture[field])==set(fixture_ids)
    assert all(fixture[field]==[] for field in ('loader_errors','failures','errors','skips','expected_failures','unexpected_successes'))
    for record in (fixture,fixture_terminal):
        for field in ('actual_native_subjects','original_runtime_fixture_reexecutions'):
            assert type(record[field]) is int and record[field]==0
    assert fixture['source_pins'][str(OLD_WORKER)]==OLD_WORKER_SHA
    pins.update(merge(pins,fixture['source_pins'],fixture['available_evidence_pins']));verify(pins)
    original=PRIOR/'original-app/SWI-Prolog.app';native=PRIOR/'arm64-app/SWI-Prolog.app'
    assert review['original_app']==str(original) and review['native_app']==str(native)
    assert original.is_dir() and native.is_dir() and not original.is_symlink() and not native.is_symlink()
    binary=(native/BINARY_REL).resolve(strict=True);fat_binary=(original/BINARY_REL).resolve(strict=True)
    home=native/HOME_REL
    assert review['binary']==str(binary) and review['binary_sha256']==BINARY_SHA
    assert review['fat_binary']==str(fat_binary) and review['fat_binary_sha256']==FAT_BINARY_SHA
    assert review['home']==str(home) and review['unique_boot']==str(home/'boot.prc') and review['home_library']==str(home/'library')
    assert home.is_dir() and (home/'boot.prc').is_file() and (home/'library').is_dir()
    assert not (native/'Contents/swipl').exists()
    assert len([path for path in native.rglob('boot.prc') if path.is_file()])==1
    assert sha(binary)==pins[str(binary)]==BINARY_SHA and sha(fat_binary)==pins[str(fat_binary)]==FAT_BINARY_SHA
    with binary.open('rb') as stream:header=stream.read(32)
    assert header.hex()==review['native_header_hex'] and header[:4]==bytes.fromhex('cffaedfe')
    assert int.from_bytes(header[4:8],'little')==0x0100000c
    layout_path=Path(review['native_layout']);assert layout_path==REVIEW.parent/'native-layout.json'
    layout=read(layout_path,review['native_layout_sha256'])
    assert len(layout)==2977 and len({row['relative'] for row in layout})==2977
    actual_paths={str(path.relative_to(native)) for path in native.rglob('*')}
    assert actual_paths=={row['relative'] for row in layout}
    for row in layout:
        path=native/row['relative']
        assert path.is_dir() is row['directory'] and path.is_file() is row['file'] and path.is_symlink() is row['symlink']
        if path.is_file():assert path.stat().st_size==row['bytes'] and sha(path)==row['sha256']==pins[str(path)]
        if path.is_symlink():
            assert os.readlink(path)==row['link'] and path.resolve(strict=True).is_relative_to(native)
            assert str(path.resolve(strict=True).relative_to(native))==row['resolved']
    commands={
        'diskimage/attach':['/usr/bin/hdiutil','attach','-readonly','-nobrowse','-noautoopen','-mountpoint',str(mount),'-plist',str(PRIOR/'swipl-10.0.2-1.fat.dmg')],
        'diskimage/detach':['/usr/bin/hdiutil','detach',str(mount)],
        'slice/arm64':['/usr/bin/lipo',str(fat_binary),'-thin','arm64','-output',str(binary)]}
    for relative,command in commands.items():
        receipt_path=PRIOR/(relative+'.json');receipt=read(receipt_path,failed['pins'][str(receipt_path)])
        assert receipt['command']==command and receipt['inherited_common_mutex'] is receipt['preparation_only'] is receipt['never_chart_eligible'] is True
        assert type(receipt['exit_status']) is type(receipt['final_exit_status']) is int and receipt['exit_status']==receipt['final_exit_status']==0
        assert not any(receipt.get(key) for key in ('error','error_type','cleanup_error','cleanup_errors','remaining_group_members'))
    assert review['slice_receipt_sha256']==sha(PRIOR/'slice/arm64.json')==SLICE_SHA
    assert failed['source_extraction']['repeated_directory_count']==40
    assert failed['source_extraction']['timestamps_applied'] is False and failed['source_extraction']['file_or_link_duplicates_accepted']==0
    assert not (PRIOR/'runtime').exists()
    assert pins[str(OLD_WORKER)]==sha(OLD_WORKER)==OLD_WORKER_SHA
    verify(pins)
    return pins,binary,fat_binary,home,native,original,dict(
        failed_report=str(PRIOR/'report.json'),failed_report_sha256=args.failed_report_sha256,
        failed_terminal=str(PRIOR/'terminal-pin.json'),failed_terminal_sha256=args.failed_terminal_sha256,
        layout_review=str(REVIEW),layout_review_sha256=args.layout_review_sha256,
        original_slice_receipt=str(PRIOR/'slice/arm64.json'),original_slice_receipt_sha256=SLICE_SHA,
        exact_prior_mount_detached=True,prior_output_mutated=False,archives_reused=True,
        original_source_extraction_reused=True,original_app_copies_reused=True,original_slice_reused=True,
        downloads=0,extractions=0,mounts=0,app_copies=0,slices=0,fixture_reexecutions=0,version_reexecutions=0)


def main():
    parser=argparse.ArgumentParser()
    for name in ('proposal-sha256','runtime-proposal-sha256','worker-sha256','failed-report-sha256','failed-terminal-sha256','layout-review-sha256','layout-terminal-sha256'):
        parser.add_argument('--'+name,required=True)
    args=parser.parse_args()
    for digest in vars(args).values():assert len(digest)==64 and all(char in '0123456789abcdef' for char in digest)
    print('WAIT common mutex: missing private SWI runtime discovery from closed ARM64 slice',flush=True)
    with LOCK.open('r+') as common:
        fcntl.flock(common,fcntl.LOCK_EX);held,named=os.fstat(common.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino) and held.st_ino==79859134
        with (DURABLE/'owner.lock').open('r+') as prior_owner:
            fcntl.flock(prior_owner,fcntl.LOCK_EX | fcntl.LOCK_NB)
            output=HERE/'.build/swi-runtime-postslice-v1';output.mkdir()
            pins={};preparation=service=guard=harness=legacy=None;error=None
            report=dict(status='running',chart_admission=False,full_processes_authorized=0,
                inherited_common_mutex=True,global_installation_mutated=False,runtime_configuration_admitted=False,
                source_work_qualified=False,source_library_review_pending=True,automatic_retry=False,
                downloads=0,extractions=0,mounts=0,app_copies=0,slices=0,fixture_reexecutions=0,version_reexecutions=0,actual_benchmark_runs=0,
                proposal=str(HERE/'proposals/private-v1.json'),proposal_sha256=args.proposal_sha256,
                runtime_proposal_sha256=args.runtime_proposal_sha256)
            try:
                assert sys.dont_write_bytecode
                source=Path(__file__).resolve();pins[str(source)]=sha(source)
                assert pins[str(source)]==args.worker_sha256
                for path in PACKET.iterdir():
                    if path.is_file():
                        assert not path.stat().st_mode & 0o222
                        pins[str(path.resolve())]=sha(path)
                pins,binary,fat_binary,home,native,original,reused=original_inputs(args,pins)
                report['reused_closed_bootstrap']=reused
                legacy=fresh(OLD_WORKER,'prolog_postslice_original_bootstrap',pins)
                runtime=HERE/'proposals/runtime-v2.json'
                assert sha(runtime)==args.runtime_proposal_sha256
                sealed_runtime=json.loads(runtime.read_text())
                assert sealed_runtime['status']=='sealed-runtime-bootstrap-awaiting-root-review'
                assert sealed_runtime['prolog_proposal_sha256']==args.proposal_sha256
                legacy.runtime_fixtures(sealed_runtime['local_fixture_receipt'])
                pins=merge(pins,sealed_runtime['pins'],{str(runtime):args.runtime_proposal_sha256})
                preparation,sealed,pins=legacy.load_preparation(args,pins)
                service,references,guard,harness,pins=legacy.reused_service(preparation,sealed,pins)
                images=legacy.fresh_load(legacy.PACKET/'image_closure_v2.py','prolog_postslice_runtime_images',pins)
                guard.OUT,guard.REPORT=output,output/'report.json'
                harness.install_interrupt_handlers();harness.compact.wait_for_environment(harness.REQUIRED)
                environment=dict(harness.compact.ENV);environment['SWI_HOME_DIR']=str(home)
                verify(pins)
                command=[str(binary),'--version']
                version=service.owned_command(guard,harness,command,environment,output/'runtime',common.fileno(),30,'version')
                assert '10.0.2' in version['stdout'].decode('utf8')
                probe_path=output/'runtime/feature-probe.json'
                command=preparation.prefix(binary)+['-l',str(legacy.ORIGINAL_PACKET/'tools/runtime_probe.pl'),'-g','clbg_runtime_probe:main','--',str(probe_path)]
                service.owned_command(guard,harness,command,environment,output/'runtime',common.fileno(),120,'feature-probe')
                actual=json.loads(probe_path.read_text())
                contracts=preparation.module(legacy.ORIGINAL_PACKET/'contracts.py','contracts',pins)
                contracts.runtime_flags(actual['runtime'],home)
                large=(1<<220)+12345
                assert type(actual['large']) is int and actual['large']==large
                assert type(actual['quotient']) is int and actual['quotient']==large//97
                assert type(actual['remainder']) is int and actual['remainder']==large%97
                assert actual['negative_floor']==-3 and actual['negative_modulo']==2 and actual['square_root']==2.0
                assert (actual['hash_value'],actual['hash_size'])==(5,4) and actual['hash_capacity']>=8
                assert actual['regex_matches']==2 and actual['regex_replacement']=='xbx'
                assert actual['regex_jit_requested'] is False and type(actual['pcre_version']) is str
                foreign_paths=[]
                for foreign in actual['foreign_libraries']:
                    path=Path(foreign['resolved_path']).resolve(strict=True)
                    assert path.is_relative_to(native.resolve()),('Foreign package outside preserved app',path)
                    foreign_paths.append(path)
                image_pins,visited=images.inspect_closure(preparation,service,guard,harness,binary,foreign_paths,
                    environment,output/'images',common.fileno(),native)
                layout=json.loads((REVIEW.parent/'native-layout.json').read_text())
                assert {str(path.relative_to(native)) for path in native.rglob('*')}=={row['relative'] for row in layout}
                pins=merge(pins,image_pins,service.pin_tree(native),service.pin_tree(original),service.pin_tree(PRIOR/'official-source'))
                for filename in actual['loaded_sources']:
                    path=Path(filename).resolve(strict=True)
                    assert path.is_relative_to(native.resolve()) or path.is_relative_to(legacy.ORIGINAL_PACKET/'tools')
                    pins=merge(pins,{str(path):sha(path)})
                report.update(status='runtime-discovered-awaiting-root-source-library-review',
                    binary=str(binary),binary_sha256=sha(binary),home=str(home),architecture='arm64',version_components=[10,0,2],
                    runtime_backend='swi-native-bytecode-vm',allowed_library_roots=[str(native)],environment={'SWI_HOME_DIR':str(home)},
                    version_receipt=legacy.actual_receipt(preparation,output/'runtime/version.json',[str(binary),'--version']),
                    archive_or_existing_installation_receipt=dict(status='passed',archive=str(PRIOR/'swipl-10.0.2-1.fat.dmg'),sha256=legacy.DMG_SHA,
                        official_source_archive=str(PRIOR/'swipl-10.0.2.tar.gz'),official_source_archive_sha256=legacy.SOURCE_SHA,
                        original_fat_binary=str(fat_binary),arm64_slice_receipt=str(PRIOR/'slice/arm64.json')),
                    actual_feature_probe=str(probe_path),actual_feature_probe_sha256=sha(probe_path),loaded_native_images=visited,
                    default_gc_observed=actual['runtime']['gc'],bounded_observed=actual['runtime']['bounded'],pcre_version=actual['pcre_version'])
            except BaseException as failure:error=failure
            finally:
                if legacy is not None:
                    legacy.terminal(output,report,pins,error)
                else:
                    failures=[];available=dict(pins)
                    for path in output.rglob('*'):
                        if path.is_file():
                            try:available[str(path.resolve())]=sha(path)
                            except BaseException as failure:failures.append(dict(path=str(path),error=str(failure) or type(failure).__name__))
                    for path,digest in pins.items():
                        try:assert sha(path)==digest
                        except BaseException as failure:failures.append(dict(path=path,error=str(failure) or type(failure).__name__))
                    report.update(status='failed-terminal-integrity' if failures else 'stopped-unqualified',pins=available,
                        terminal_evidence_complete=not failures,terminal_artifact_errors=failures)
                    if error:report.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                    save(output/'report.json',report)
                    save(output/'terminal-pin.json',dict(status=report['status'],report=str(output/'report.json'),report_sha256=sha(output/'report.json'),
                        partial_evidence_pins=available,inherited_common_mutex=True,chart_admission=False,full_processes_authorized=0))
                    if failures:raise RuntimeError('Post-slice early terminal integrity failed')
            if error is not None:raise error
            print(json.dumps(dict(status=report['status'],report=str(output/'report.json'),report_sha256=sha(output/'report.json'),
                runtime_configuration_admitted=False,source_work_qualified=False,full_processes_authorized=0)),flush=True)


if __name__=='__main__':main()
