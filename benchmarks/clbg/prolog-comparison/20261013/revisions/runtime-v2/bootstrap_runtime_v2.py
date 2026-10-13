"""Unexecuted private SWI DMG/bootstrap candidate, entirely under common mutex.

No global install/upgrade. Existing PATH candidates are observed, not selected.
Publisher archive and original fat application are immutable; a separate private
copy receives an exact ARM64 launcher slice. Qualification requires root review.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parents[1]
ORIGINAL_PACKET = HERE / 'drafts/private-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
DMG_URL = 'https://www.swi-prolog.org/download/stable/bin/swipl-10.0.2-1.fat.dmg'
DMG_SHA = 'bf775f0b8d7880f4908dee513316013ef42a73793be392814fde2a0a8e9ddc5d'
DMG_BYTES = 34301763
SOURCE_URL = 'https://www.swi-prolog.org/download/stable/src/swipl-10.0.2.tar.gz'
SOURCE_SHA = 'e42cc098f7b8a6051c4f79a99b55162d467098aba60f69649bdc7583f0734b57'
SOURCE_BYTES = 12825701
RUNTIME_IDS = [
    'test_image_closure_v2.RuntimeClosure.test_scoped_arm64_header_and_load_command_inventory',
    'test_image_closure_v2.RuntimeClosure.test_dylib_install_id_is_not_a_dependency',
    'test_image_closure_v2.RuntimeClosure.test_executable_and_loader_anchors_differ_for_a_library',
    'test_image_closure_v2.RuntimeClosure.test_rpaths_require_actual_unique_private_images',
    'test_image_closure_v2.RuntimeClosure.test_fresh_import_rejects_cache_or_changed_source_before_execution',
    'test_image_closure_v2.RuntimeClosure.test_partial_attach_preserves_actual_device_entities_without_detach_authority']
MACH_MAGICS = {b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xce\xfa\xed\xfe',
               b'\xfe\xed\xfa\xce', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca',
               b'\xca\xfe\xba\xbf', b'\xbf\xba\xfe\xca'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Pinned bytes changed', path)


def merge(*groups):
    result = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in result or result[path] == digest
            result[path] = digest
    return result


def fresh_load(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules, ('Cached bootstrap dependency', name)
    assert sha(path) == pins[str(path)]
    source = path.read_bytes()
    assert hashlib.sha256(source).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    sys.modules[name] = loaded
    exec(compile(source, str(path), 'exec'), loaded.__dict__)
    assert loaded is sys.modules[name] and Path(loaded.__file__).resolve() == path
    assert sha(path) == pins[str(path)]
    return loaded


def load_preparation(args, pins):
    proposal = HERE / 'proposals/private-v1.json'
    assert sha(proposal) == args.proposal_sha256
    sealed = json.loads(proposal.read_text())
    assert sealed['status'] == 'sealed-awaiting-root-review'
    retained = sealed['local_fixture_receipt']
    assert retained['status'] == 'passed' and retained['execution_completed'] is True
    assert type(retained['tests_run']) is int and retained['tests_run'] == 6
    assert type(retained['discovered_tests']) is int and retained['discovered_tests'] == 6
    expected = [
        'test_contracts.PreparationContracts.test_twenty_profiles_and_explicit_same_source_aliases',
        'test_contracts.PreparationContracts.test_compiler_inventory_requires_each_helper_and_no_foreign_authored_root',
        'test_contracts.PreparationContracts.test_actual_runtime_flags_reject_bounded_or_x86_or_disabled_gc',
        'test_contracts.PreparationContracts.test_native_counter_and_workload_boolean_substitution_is_rejected',
        'test_contracts.PreparationContracts.test_isolation_requires_exact_twenty_boolean_gates_and_clean_ownership',
        'test_contracts.PreparationContracts.test_full_admission_requires_actual_scored_allocation_not_source_assertion']
    assert retained['expected_test_ids'] == expected
    for key in ('discovered_test_ids', 'actual_test_ids'):
        assert len(retained[key]) == len(set(retained[key])) == 6
        assert set(retained[key]) == set(expected)
    assert all(retained[key] == [] for key in ('failures', 'errors', 'skips',
        'expected_failures', 'unexpected_successes'))
    pins = merge(pins, sealed['pins'], {str(proposal): args.proposal_sha256})
    verify(pins)
    source = ORIGINAL_PACKET / 'guarded_prepare.py'
    assert str(source) in sealed['pins']
    preparation = fresh_load(source, 'prolog_runtime_v2_preparation', pins)
    assert preparation.PACKET == ORIGINAL_PACKET and preparation.HERE == HERE
    return preparation, sealed, pins


def reused_service(preparation, sealed, pins):
    # V1's loader permits only its own packet as a cached benchmark module.
    # This additive entry has a different path, so repeat that check with the
    # exact verified source pins rather than falsifying any module.__file__.
    for loaded in list(sys.modules.values()):
        filename = getattr(loaded, '__file__', None)
        if filename:
            path = Path(filename).resolve()
            if path.is_relative_to(preparation.CLBG):
                assert str(path) in pins and sha(path) == pins[str(path)], ('Foreign cached source', path)
    digest = sealed['service_proposal_sha256']
    assert preparation.sha(preparation.SERVICE_PROPOSAL) == digest
    original = json.loads(preparation.SERVICE_PROPOSAL.read_text())
    assert original['status'] == 'sealed-awaiting-root-approval'
    receipt = Path(original['actual_mock_fixture_receipt'])
    assert original['actual_mock_fixture_receipt_sha256'] == original['pins'][str(receipt)]
    saved = {path: value for path, value in original['pins'].items() if Path(path).suffix == '.py'}
    for path in (preparation.SERVICE, preparation.REFERENCES, preparation.GENERIC_FIXTURE_RUNNER, receipt):
        saved[str(path)] = original['pins'][str(path)]
    saved.update({path: value for path, value in original['pins'].items()
        if Path(path).parent == receipt.parent and Path(path).name.startswith('ownership-fixtures')})
    saved[str(preparation.SERVICE_PROPOSAL)] = digest
    verify(saved)
    actual = json.loads(receipt.read_text())
    assert actual['status'] == 'passed' and actual['inventory'] == 'ownership'
    assert actual['expected_test_ids'] == preparation.OWNERSHIP_IDS
    assert type(actual['tests_run']) is int and actual['tests_run'] == 11
    assert type(actual['discovered_tests']) is int and actual['discovered_tests'] == 11
    assert len(actual['actual_test_ids']) == len(set(actual['actual_test_ids'])) == 11
    assert set(actual['actual_test_ids']) == set(preparation.OWNERSHIP_IDS)
    assert actual['successful'] is True
    assert all(actual[key] == [] for key in ('skipped', 'failures', 'errors',
        'expected_failures', 'unexpected_successes'))
    service = preparation.module(preparation.SERVICE, 'prolog_reviewed_generic_service', saved)
    references = preparation.module(preparation.REFERENCES, 'prolog_reviewed_bounded_references', saved)
    guard = service.load_guard()
    harness = guard.load_harness()
    for loaded in (guard, harness, harness.compact, harness.compact.common):
        filename = str(Path(loaded.__file__).resolve())
        assert filename in saved and sha(filename) == saved[filename]
    verify(saved)
    return service, references, guard, harness, merge(pins, saved)


def terminal(output, report, pins, error):
    partial, failures = dict(pins), []
    for path in sorted(output.rglob('*')):
        if path.is_file() and path.name not in ('report.json', 'terminal-pin.json'):
            try:
                partial = merge(partial, {str(path): sha(path)})
            except BaseException as failure:
                failures.append(dict(path=str(path), error=str(failure) or type(failure).__name__))
    if error is not None:
        report.update(status='stopped-unqualified', error=str(error) or type(error).__name__,
            error_type=type(error).__name__)
    for path, digest in partial.items():
        try:
            assert sha(path) == digest, ('Pinned bytes changed', path)
        except BaseException as failure:
            failures.append(dict(path=path, error=str(failure) or type(failure).__name__))
    if failures:
        report.update(status='failed-terminal-integrity', terminal_artifact_errors=failures)
    report.update(pins=partial, terminal_evidence_complete=not failures)
    save(output / 'report.json', report)
    save(output / 'terminal-pin.json', dict(status=report['status'], report=str(output/'report.json'),
        report_sha256=sha(output/'report.json'), partial_evidence_pins=partial,
        inherited_common_mutex=True, chart_admission=False, full_processes_authorized=0))
    if failures:
        raise RuntimeError('Runtime terminal integrity failed; readable partial evidence preserved')


def attach_inventory(payload, mount):
    """Record only actual attach plist entities; never derive guessed devices."""
    import plistlib
    value = plistlib.loads(payload)
    assert isinstance(value, dict) and isinstance(value.get('system-entities'), list)
    entities = value['system-entities']
    assert all(isinstance(entity, dict) for entity in entities)
    mounts = [entity for entity in entities if 'mount-point' in entity]
    exact = [entity for entity in mounts if Path(entity['mount-point']).resolve() == mount.resolve()]
    return dict(system_entities=entities, actual_mount_entities=mounts,
        requested_mount=str(mount), exact_requested_mount_entities=exact,
        all_observed_mounts_are_requested=len(mounts) == len(exact),
        unknown_device_detach_authorized=False)


def runtime_fixtures(value):
    assert value['status'] == 'passed' and value['execution_completed'] is True
    assert value['inherited_common_mutex'] is True and value['expected_test_ids'] == RUNTIME_IDS
    for key in ('discovered_tests', 'tests_run'):
        assert type(value[key]) is int and value[key] == 6
    for key in ('discovered_test_ids', 'actual_test_ids'):
        assert len(value[key]) == len(set(value[key])) == 6
        assert set(value[key]) == set(RUNTIME_IDS)
    assert all(value[key] == [] for key in ('failures', 'errors', 'skips',
        'expected_failures', 'unexpected_successes'))


def metadata_seal(args):
    output = HERE / 'proposal-seals/runtime-v2'
    output.mkdir(parents=True)
    pins = {}
    report = dict(status='sealing', inherited_common_mutex=True, chart_admission=False,
        runtime_configuration_admitted=False, source_work_qualified=False, full_processes_authorized=0)
    error = None
    try:
        for path in sorted(PACKET.glob('*.py')) + sorted(PACKET.glob('*.md')):
            pins[str(path.resolve())] = sha(path)
        preparation, sealed, pins = load_preparation(args, pins)
        service, references, guard, harness, pins = reused_service(preparation, sealed, pins)
        runner = fresh_load(PACKET/'fixture_runner_v2.py', 'prolog_runtime_v2_fixtures', pins)
        actual = runner.run(output, pins, sha, verify, save)
        runtime_fixtures(actual)
        verify(pins)
        report.update(status='sealed-runtime-bootstrap-awaiting-root-review',
            prolog_proposal=str(HERE/'proposals/private-v1.json'),
            prolog_proposal_sha256=args.proposal_sha256, local_fixture_receipt=actual,
            retained_prolog_fixture_count=6, retained_generic_fixture_count=11,
            publisher_dmg_url=DMG_URL, publisher_dmg_sha256=DMG_SHA,
            original_bootstrap=str(ORIGINAL_PACKET/'bootstrap_runtime.py'),
            correction='verified-source import; actual executable anchor/install IDs/rpaths; partial attach evidence',
            native_processes_authorized=0)
    except BaseException as failure:
        error = failure
    finally:
        terminal(output, report, pins, error)
    if error is not None:
        raise error
    filename = HERE / 'proposals/runtime-v2.json'
    save(filename, report)
    print(json.dumps(dict(proposal=str(filename), sha256=sha(filename), status=report['status'])), flush=True)


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
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--bootstrap', action='store_true')
    parser.add_argument('--proposal-sha256', required=True)
    parser.add_argument('--runtime-proposal-sha256')
    args = parser.parse_args()
    print('WAIT common mutex: private SWI10.0.2 runtime bootstrap candidate', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino), 'Named common mutex replaced while waiting'
        if args.seal:
            assert args.runtime_proposal_sha256 is None
            metadata_seal(args)
            return
        assert args.runtime_proposal_sha256
        output = HERE / '.build/swi-runtime-private-v2'
        output.mkdir(parents=True)
        pins = {}
        preparation = service = guard = harness = None
        environment = None
        report = dict(status='running', chart_admission=False, full_processes_authorized=0,
            inherited_common_mutex=True, global_installation_mutated=False,
            proposal=str(HERE/'proposals/private-v1.json'),
            proposal_sha256=args.proposal_sha256, publisher_dmg_url=DMG_URL, publisher_dmg_sha256=DMG_SHA,
            runtime_proposal_sha256=args.runtime_proposal_sha256,
            default_gc_review=dict(status='pending'), source_work_qualified=False,
            runtime_configuration_admitted=False)
        error = None
        mount = output / 'owned-readonly-mount'
        detach_error = None
        attach_attempted = False
        try:
            runtime_proposal = HERE / 'proposals/runtime-v2.json'
            assert sha(runtime_proposal) == args.runtime_proposal_sha256
            runtime_seal = json.loads(runtime_proposal.read_text())
            assert runtime_seal['status'] == 'sealed-runtime-bootstrap-awaiting-root-review'
            assert runtime_seal['prolog_proposal_sha256'] == args.proposal_sha256
            runtime_fixtures(runtime_seal['local_fixture_receipt'])
            pins = merge(runtime_seal['pins'], {str(runtime_proposal): args.runtime_proposal_sha256})
            verify(pins)
            assert sha(Path(__file__).resolve()) == pins[str(Path(__file__).resolve())]
            preparation, sealed, pins = load_preparation(args, pins)
            service, references, guard, harness, pins = reused_service(preparation, sealed, pins)
            images = fresh_load(PACKET/'image_closure_v2.py', 'prolog_runtime_images_v2', pins)
            guard.OUT, guard.REPORT = output, output/'report.json'
            harness.install_interrupt_handlers()
            harness.compact.wait_for_environment(harness.REQUIRED)
            environment = dict(harness.compact.ENV)
            for tool in ('/usr/bin/hdiutil', '/usr/bin/lipo', '/usr/bin/otool'):
                resolved = Path(tool).resolve(strict=True)
                pins = merge(pins, {str(resolved): sha(resolved)})
            verify(pins)
            existing = shutil.which('swipl')
            report['existing_PATH_candidate'] = dict(path=existing, selected=False, verified=False)
            if existing:
                candidate = Path(existing).resolve(strict=True)
                report['existing_PATH_candidate'].update(resolved_path=str(candidate), sha256=preparation.sha(candidate))
                pins = merge(pins, {str(candidate): preparation.sha(candidate)})
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
            attach_attempted = True
            report['attach_attempted'] = True
            attached = service.owned_command(guard, harness, command, environment, output/'diskimage',
                                             lock.fileno(), 120, 'attach')
            inventory = attach_inventory(attached['stdout'], mount)
            report['attach_inventory'] = inventory
            entities = inventory['actual_mount_entities']
            assert inventory['all_observed_mounts_are_requested'] is True
            assert len(inventory['exact_requested_mount_entities']) == len(entities) == 1
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
            pins = merge(pins, report['original_fat_app_pins'])
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
            pins = merge(pins, service.pin_tree(native))
            verify(pins)
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
            command = preparation.prefix(binary) + ['-l',str(ORIGINAL_PACKET/'tools/runtime_probe.pl'),
                '-g','clbg_runtime_probe:main','--',str(probe_path)]
            service.owned_command(guard, harness, command, environment, output/'runtime',lock.fileno(),120,'feature-probe')
            actual = json.loads(probe_path.read_text())
            contracts = preparation.module(ORIGINAL_PACKET/'contracts.py','contracts',pins)
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
            foreign_paths = []
            for foreign in actual['foreign_libraries']:
                path = Path(foreign['resolved_path']).resolve(strict=True)
                assert path.is_relative_to(native.resolve()), ('Foreign package outside private app', path)
                foreign_paths.append(path)
            image_pins, visited = images.inspect_closure(preparation, service, guard, harness,
                binary, foreign_paths, environment, output/'images', lock.fileno(), native)
            pins = preparation.merge(pins, image_pins)
            pins = preparation.merge(pins, service.pin_tree(native), service.pin_tree(original),
                                     service.pin_tree(output/'official-source'))
            for filename in actual['loaded_sources']:
                path = Path(filename).resolve(strict=True)
                assert path.is_relative_to(native.resolve()) or path.is_relative_to(ORIGINAL_PACKET/'tools')
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
                loaded_native_images=visited,default_gc_observed=actual['runtime']['gc'],
                bounded_observed=actual['runtime']['bounded'],pcre_version=actual['pcre_version'],
                source_library_review_pending=True,runtime_configuration_admitted=False)
        except BaseException as failure:
            error = failure
        finally:
            # Failed attach may leave only device entities or malformed/partial
            # stdout. Preserve actual bytes and mark its closure unresolved.
            # No listed/guessed device is independently authorized for detach.
            if attach_attempted and 'attach_inventory' not in report:
                raw_attach = output/'diskimage/attach.stdout'
                if raw_attach.is_file():
                    try:
                        report['attach_inventory'] = attach_inventory(raw_attach.read_bytes(), mount)
                    except BaseException as failure:
                        report['attach_inventory_error'] = str(failure) or type(failure).__name__
                report.update(owned_mount_requires_root_review=True,
                    attach_device_closure='unresolved-after-failed-attach')
            if os.path.ismount(mount):
                try:
                    command = ['/usr/bin/hdiutil','detach',str(mount)]
                    service.owned_command(guard,harness,command,environment,output/'diskimage',
                                          lock.fileno(),120,'detach')
                    assert not os.path.ismount(mount), 'Owned mount remains after exact detach'
                    report['requested_mount_detached'] = True
                    inventory = report.get('attach_inventory', {})
                    if inventory.get('all_observed_mounts_are_requested') is True and \
                            len(inventory.get('exact_requested_mount_entities', [])) == 1:
                        report['attach_device_closure'] = 'exact-requested-mount-detached'
                    else:
                        report['owned_mount_requires_root_review'] = True
                except BaseException as failure:
                    detach_error = failure
                    report['owned_mount_cleanup_error'] = str(failure) or type(failure).__name__
                    report['owned_mount_requires_root_review'] = True
            elif attach_attempted and not report.get('requested_mount_detached'):
                report.update(owned_mount_requires_root_review=True,
                    attach_device_closure='unresolved-no-requested-mount-observable')
            if detach_error is not None:
                if error is not None:
                    report['preceding_error'] = str(error) or type(error).__name__
                error = detach_error
            terminal(output,report,pins,error)
        if error is not None:
            raise error
        print(json.dumps(dict(report=str(output/'report.json'),sha256=preparation.sha(output/'report.json'),
            status=report['status'],configuration_admitted=False)),flush=True)


if __name__ == '__main__':
    main()
