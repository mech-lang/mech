"""Unexecuted private SWI DMG/bootstrap candidate, entirely under common mutex.

No global install/upgrade. Existing PATH candidates are observed, not selected.
Publisher archive and original fat application are immutable; a separate private
copy receives an exact ARM64 launcher slice. Qualification requires root review.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import sys

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
DMG_URL = 'https://www.swi-prolog.org/download/stable/bin/swipl-10.0.2-1.fat.dmg'
DMG_SHA = 'bf775f0b8d7880f4908dee513316013ef42a73793be392814fde2a0a8e9ddc5d'
DMG_BYTES = 34301763
SOURCE_URL = 'https://www.swi-prolog.org/download/stable/src/swipl-10.0.2.tar.gz'
SOURCE_SHA = 'e42cc098f7b8a6051c4f79a99b55162d467098aba60f69649bdc7583f0734b57'
SOURCE_BYTES = 12825701
MACH_MAGICS = {b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xce\xfa\xed\xfe',
               b'\xfe\xed\xfa\xce', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca',
               b'\xca\xfe\xba\xbf', b'\xbf\xba\xfe\xca'}


def safe_tree(root):
    root = root.resolve()
    for path in root.rglob('*'):
        if path.is_symlink():
            assert path.resolve(strict=True).is_relative_to(root), ('External bundle symlink', path)
        elif not path.is_file() and not path.is_dir():
            raise AssertionError(('Unsupported bundle entry', path))


def extract_source(archive, destination):
    import tarfile
    destination.mkdir()
    with tarfile.open(archive, 'r:gz') as source:
        members = source.getmembers()
        names = set()
        links = []
        for member in members:
            target = (destination / member.name).resolve()
            assert target.is_relative_to(destination.resolve()) and str(target) not in names
            names.add(str(target))
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                with source.extractfile(member) as incoming, target.open('xb') as outgoing:
                    shutil.copyfileobj(incoming, outgoing)
            elif member.issym():
                assert not Path(member.linkname).is_absolute()
                assert (target.parent / member.linkname).resolve().is_relative_to(destination.resolve())
                links.append((target, member.linkname))
            else:
                raise AssertionError(('Unselected archive member', member.name, member.type))
        for target, link in links:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(link)
    safe_tree(destination)


def actual_receipt(preparation, path, command):
    value = json.loads(path.read_text())
    assert value['command'] == command and value['inherited_common_mutex'] is True
    assert type(value['exit_status']) is int and value['exit_status'] == 0
    assert type(value['final_exit_status']) is int and value['final_exit_status'] == 0
    assert not value.get('error') and not value.get('cleanup_errors')
    return dict(status='passed', receipt=str(path), receipt_sha256=preparation.sha(path),
                command=command, stdout=str(path.with_suffix('.stdout')),
                stdout_sha256=preparation.sha(path.with_suffix('.stdout')))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bootstrap', action='store_true', required=True)
    parser.add_argument('--proposal-sha256', required=True)
    args = parser.parse_args()
    print('WAIT common mutex: private SWI10.0.2 runtime bootstrap candidate', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        import guarded_prepare as preparation
        proposal = HERE / 'proposals/private-v1.json'
        assert preparation.sha(proposal) == args.proposal_sha256
        sealed = json.loads(proposal.read_text())
        assert sealed['status'] == 'sealed-awaiting-root-review'
        preparation.verify(sealed['pins'])
        service, references, guard, harness, reused = preparation.reused_service(sealed['service_proposal_sha256'])
        pins = preparation.merge(sealed['pins'], reused, {str(proposal): args.proposal_sha256})
        output = HERE / '.build/swi-runtime-private-v1'
        output.mkdir(parents=True)
        guard.OUT = output
        guard.REPORT = output / 'report.json'
        report = dict(status='running', chart_admission=False, full_processes_authorized=0,
            inherited_common_mutex=True, global_installation_mutated=False, proposal=str(proposal),
            proposal_sha256=args.proposal_sha256, publisher_dmg_url=DMG_URL, publisher_dmg_sha256=DMG_SHA,
            default_gc_review=dict(status='pending'), source_work_qualified=False)
        error = None
        mount = output / 'owned-readonly-mount'
        detach_error = None
        try:
            harness.install_interrupt_handlers()
            harness.compact.wait_for_environment(harness.REQUIRED)
            environment = dict(harness.compact.ENV)
            existing = shutil.which('swipl')
            report['existing_PATH_candidate'] = dict(path=existing, selected=False, verified=False)
            if existing:
                candidate = Path(existing).resolve(strict=True)
                report['existing_PATH_candidate'].update(resolved_path=str(candidate), sha256=preparation.sha(candidate))
                command = [str(candidate), '--version']
                service.owned_command(guard, harness, command, environment, output/'existing-observation',
                                      lock.fileno(), 30, 'version')
                report['existing_PATH_candidate']['version_observation'] = actual_receipt(
                    preparation, output/'existing-observation/version.json', command)
            archive = output / 'swipl-10.0.2-1.fat.dmg'
            source_archive = output / 'swipl-10.0.2.tar.gz'
            print('DOWNLOAD exact official DMG/source archives under common mutex', flush=True)
            service.download(DMG_URL, archive)
            assert archive.stat().st_size == DMG_BYTES and preparation.sha(archive) == DMG_SHA
            service.download(SOURCE_URL, source_archive)
            assert source_archive.stat().st_size == SOURCE_BYTES and preparation.sha(source_archive) == SOURCE_SHA
            pins = preparation.merge(pins, {str(archive): DMG_SHA, str(source_archive): SOURCE_SHA})
            extract_source(source_archive, output/'official-source')
            mount.mkdir()
            command = ['/usr/bin/hdiutil','attach','-readonly','-nobrowse','-noautoopen',
                       '-mountpoint',str(mount),'-plist',str(archive)]
            attached = service.owned_command(guard, harness, command, environment, output/'diskimage',
                                             lock.fileno(), 120, 'attach')
            import plistlib
            listing = plistlib.loads(attached['stdout'])
            entities = [entity for entity in listing['system-entities'] if 'mount-point' in entity]
            assert len(entities) == 1 and Path(entities[0]['mount-point']).resolve() == mount.resolve()
            assert os.path.ismount(mount), 'No actual owned read-only mount'
            report['owned_mount_entity'] = entities[0]
            apps = [path for path in mount.glob('*.app') if path.is_dir() and not path.is_symlink()]
            assert len(apps) == 1, 'Review actual bundle layout rather than guessing an app name'
            safe_tree(apps[0])
            original = output / 'original-app' / apps[0].name
            original.parent.mkdir()
            shutil.copytree(apps[0], original, symlinks=True)
            safe_tree(original)
            report['original_fat_app_pins'] = service.pin_tree(original)
            native = output / 'arm64-app' / apps[0].name
            native.parent.mkdir()
            shutil.copytree(original, native, symlinks=True)
            safe_tree(native)
            candidates = set()
            for candidate in native.rglob('swipl'):
                if candidate.is_file():
                    with candidate.open('rb') as stream:
                        if stream.read(4) in MACH_MAGICS:
                            candidates.add(candidate.resolve())
            assert len(candidates) == 1, ('Review actual commandline Mach-O layout', sorted(map(str,candidates)))
            binary = candidates.pop()
            relative = binary.relative_to(native.resolve())
            fat_binary = (original / relative).resolve(strict=True)
            original_mode = binary.stat().st_mode
            binary.unlink()
            command = ['/usr/bin/lipo',str(fat_binary),'-thin','arm64','-output',str(binary)]
            service.owned_command(guard, harness, command, environment, output/'slice',lock.fileno(),30,'arm64')
            binary.chmod(original_mode)
            # No hard-coded source/home/PlugIns layout: the actual relocated
            # runtime reports it. Entire private app corpus is pinned afterward.
            home = native / 'Contents/swipl'
            assert home.is_dir(), 'Actual bundle home differs; retain and review this layout'
            environment['SWI_HOME_DIR'] = str(home)
            command = [str(binary), '--version']
            version = service.owned_command(guard, harness, command, environment, output/'runtime',
                                            lock.fileno(),30,'version')
            assert '10.0.2' in version['stdout'].decode('utf8')
            probe_path = output / 'runtime/feature-probe.json'
            command = preparation.prefix(binary) + ['-l',str(PACKET/'tools/runtime_probe.pl'),
                '-g','clbg_runtime_probe:main','--',str(probe_path)]
            service.owned_command(guard, harness, command, environment, output/'runtime',lock.fileno(),120,'feature-probe')
            actual = json.loads(probe_path.read_text())
            contracts = preparation.module(PACKET/'contracts.py','contracts',pins)
            contracts.runtime_flags(actual['runtime'],home)
            large = (1 << 220) + 12345
            assert type(actual['large']) is int and actual['large'] == large
            assert type(actual['quotient']) is int and actual['quotient'] == large // 97
            assert type(actual['remainder']) is int and actual['remainder'] == large % 97
            assert actual['negative_floor'] == -3 and actual['negative_modulo'] == 2
            assert actual['square_root'] == 2.0
            assert (actual['hash_value'],actual['hash_size']) == (5,4) and actual['hash_capacity'] >= 8
            assert actual['regex_matches'] == 2 and actual['regex_replacement'] == 'xbx'
            assert actual['regex_jit_requested'] is False and type(actual['pcre_version']) is str
            pending = [binary]
            for foreign in actual['foreign_libraries']:
                path = Path(foreign['resolved_path']).resolve(strict=True)
                assert path.is_relative_to(native.resolve()), ('Foreign package outside private app', path)
                pending.append(path)
            visited = set()
            while pending:
                image = pending.pop().resolve(strict=True)
                if image in visited:
                    continue
                visited.add(image)
                image_pins = service.native_images(guard,harness,image,environment,
                    output/'images'/f'{len(visited):03d}',lock.fileno(),[native],scored=image == binary)
                pins = preparation.merge(pins,image_pins,{str(image):preparation.sha(image)})
                pending.extend(Path(path) for path in image_pins)
            pins = preparation.merge(pins, service.pin_tree(native), service.pin_tree(original),
                                     service.pin_tree(output/'official-source'))
            for filename in actual['loaded_sources']:
                path = Path(filename).resolve(strict=True)
                assert path.is_relative_to(native.resolve()) or path.is_relative_to(PACKET/'tools')
                pins = preparation.merge(pins,{str(path):preparation.sha(path)})
            report.update(status='runtime-discovered-awaiting-root-source-library-review',
                binary=str(binary),binary_sha256=preparation.sha(binary),home=str(home),architecture='arm64',
                version_components=[10,0,2],runtime_backend='swi-native-bytecode-vm',
                allowed_library_roots=[str(native)],environment={'SWI_HOME_DIR':str(home)},
                version_receipt=actual_receipt(preparation,output/'runtime/version.json',[str(binary),'--version']),
                archive_or_existing_installation_receipt=dict(status='passed',archive=str(archive),sha256=DMG_SHA,
                    official_source_archive=str(source_archive),official_source_archive_sha256=SOURCE_SHA,
                    original_fat_binary=str(fat_binary),arm64_slice_receipt=str(output/'slice/arm64.json')),
                actual_feature_probe=str(probe_path),actual_feature_probe_sha256=preparation.sha(probe_path),
                loaded_native_images=sorted(map(str,visited)),default_gc_observed=actual['runtime']['gc'],
                bounded_observed=actual['runtime']['bounded'],pcre_version=actual['pcre_version'],
                source_library_review_pending=True,runtime_configuration_admitted=False)
        except BaseException as failure:
            error = failure
        finally:
            if os.path.ismount(mount):
                try:
                    command = ['/usr/bin/hdiutil','detach',str(mount)]
                    service.owned_command(guard,harness,command,environment,output/'diskimage',
                                          lock.fileno(),120,'detach')
                    assert not os.path.ismount(mount), 'Owned mount remains after exact detach'
                except BaseException as failure:
                    detach_error = failure
                    report['owned_mount_cleanup_error'] = str(failure) or type(failure).__name__
                    report['owned_mount_requires_root_review'] = True
            if detach_error is not None:
                if error is not None:
                    report['preceding_error'] = str(error) or type(error).__name__
                error = detach_error
            preparation.terminal(output,report,pins,error)
        if error is not None:
            raise error
        print(json.dumps(dict(report=str(output/'report.json'),sha256=preparation.sha(output/'report.json'),
            status=report['status'],configuration_admitted=False)),flush=True)


if __name__ == '__main__':
    main()
