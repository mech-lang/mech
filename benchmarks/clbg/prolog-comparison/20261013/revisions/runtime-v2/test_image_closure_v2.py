"""Synthetic parser/import/attach fixtures; never benchmark observations."""
from pathlib import Path
import plistlib
import sys
import tempfile
import types
import unittest

import bootstrap_runtime_v2 as bootstrap
import image_closure_v2 as images


def header(path, kind='DYLIB', commands=2):
    return (str(path) + ':\nMach header\n'
        'magic cputype cpusubtype caps filetype ncmds sizeofcmds flags\n'
        f'MH_MAGIC_64 ARM64 ALL 0x00 {kind} {commands} 160 NOUNDEFS\n')


def load(path, entries):
    text = str(path) + ':\n'
    for index, (command, field, value) in enumerate(entries):
        text += f'Load command {index}\n cmd {command}\n cmdsize 64\n {field} {value} (offset 24)\n'
    return text


def linked(path, names):
    return str(path) + ':\n' + ''.join(
        f'\t{name} (compatibility version 1.0.0, current version 1.0.0)\n' for name in names)


class RuntimeClosure(unittest.TestCase):
    def test_scoped_arm64_header_and_load_command_inventory(self):
        image = Path('/private/candidate/lib/libswipl.dylib')
        self.assertEqual(images.mach_header(header(image), image)['filetype'], 'DYLIB')
        for bad in (header(image).replace('ARM64', 'X86_64'),
                header(image).replace(str(image), '/other/image'),
                header(image).replace('DYLIB', 'OBJECT')):
            with self.assertRaises(AssertionError):
                images.mach_header(bad, image)
        actual = load(image, [('LC_ID_DYLIB', 'name', '@rpath/libswipl.dylib'),
            ('LC_LOAD_DYLIB', 'name', '/usr/lib/libSystem.B.dylib')])
        self.assertEqual(len(images.load_commands(actual, image, 2)['dependencies']), 1)
        for bad in (actual.replace('Load command 1', 'Load command 2'),
                actual.replace('LC_LOAD_DYLIB', 'LC_UNKNOWN_DYLIB')):
            with self.assertRaises(AssertionError):
                images.load_commands(bad, image, 2)
        with self.assertRaises(AssertionError):
            images.load_commands(actual, image, 3)

    def test_dylib_install_id_is_not_a_dependency(self):
        image = Path('/private/candidate/lib/libswipl.dylib')
        own = '@rpath/libswipl.dylib'
        dependency = '@executable_path/../lib/libgmp.dylib'
        commands = images.load_commands(load(image, [('LC_ID_DYLIB', 'name', own),
            ('LC_LOAD_DYLIB', 'name', dependency)]), image, 2)
        value = images.reconcile_listings(linked(image, [own, dependency]),
            str(image) + ':\n' + own + '\n', image,
            images.mach_header(header(image), image), commands)
        self.assertEqual(value['install_id'], own)
        self.assertEqual([entry['install_name'] for entry in value['dependencies']], [dependency])
        for names, identifier in (([own], own), ([own, dependency], dependency),
                ([dependency], own)):
            with self.assertRaises(AssertionError):
                images.reconcile_listings(linked(image, names), str(image) + ':\n' + identifier + '\n',
                    image, images.mach_header(header(image), image), commands)
        executable = Path('/private/candidate/bin/swipl')
        empty = images.load_commands(load(executable, [
            ('LC_LOAD_DYLIB', 'name', '/usr/lib/libSystem.B.dylib')]), executable, 1)
        images.reconcile_listings(linked(executable, ['/usr/lib/libSystem.B.dylib']),
            str(executable) + ':\n', executable,
            images.mach_header(header(executable, 'EXECUTE', 1), executable), empty)

    def test_executable_and_loader_anchors_differ_for_a_library(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            executable = root / 'bin/swipl'
            library = root / 'PlugIns/package.dylib'
            for path in (executable, library, root/'bin/libgmp.dylib', root/'PlugIns/libgmp.dylib'):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture-only')
            executable_target = images.resolve_install('@executable_path/libgmp.dylib',
                library, executable, root, tuple())
            loader_target = images.resolve_install('@loader_path/libgmp.dylib',
                library, executable, root, tuple())
            self.assertEqual(executable_target['path'], str(root/'bin/libgmp.dylib'))
            self.assertEqual(loader_target['path'], str(root/'PlugIns/libgmp.dylib'))
            self.assertNotEqual(executable_target['path'], loader_target['path'])
            self.assertEqual(images.expand_rpath('@executable_path/../lib', library,
                executable, root), root/'lib')
            self.assertEqual(images.expand_rpath('@loader_path/../lib', library,
                executable, root), root/'lib')

    def test_rpaths_require_actual_unique_private_images(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory).resolve()
            root = base / 'app'
            executable, library = root/'bin/swipl', root/'PlugIns/package.dylib'
            for path in (executable, library, root/'lib/libgmp.dylib'):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture-only')
            chain = images.rpath_chain(['@loader_path/../lib'], library, executable,
                root, (root/'lib',))
            self.assertEqual(chain, (root/'lib',))
            self.assertEqual(images.resolve_install('@rpath/libgmp.dylib', library,
                executable, root, chain)['path'], str(root/'lib/libgmp.dylib'))
            for name in ('@rpath/missing.dylib', 'libgmp.dylib', '@unknown/libgmp.dylib',
                    '/usr/lib/../../tmp/private.dylib', '/System/Library/../../tmp/private.dylib'):
                with self.assertRaises(AssertionError):
                    images.resolve_install(name, library, executable, root, chain)
            self.assertFalse(images.os_install('/usr/lib/../../tmp/private.dylib'))
            self.assertFalse(images.os_install('/System/Library/../../tmp/private.dylib'))
            second = root/'other/libgmp.dylib'
            second.parent.mkdir()
            second.write_bytes(b'different-fixture-only')
            with self.assertRaises(AssertionError):
                images.resolve_install('@rpath/libgmp.dylib', library, executable,
                    root, chain + (second.parent,))
            outside = base/'outside.dylib'
            outside.write_bytes(b'outside-fixture-only')
            (root/'lib/escape.dylib').symlink_to(outside)
            with self.assertRaises(AssertionError):
                images.resolve_install('@rpath/escape.dylib', library, executable, root, chain)
            for rpath in ('@rpath/recursive', 'relative/lib', str(base/'foreign')):
                with self.assertRaises(AssertionError):
                    images.expand_rpath(rpath, library, executable, root)
            with self.assertRaises(AssertionError):
                images.resolve_install('@rpath/libgmp.dylib', library, executable,
                    root, chain + (Path('/usr/lib'),))
            self.assertEqual(images.resolve_install('/usr/lib/libSystem.B.dylib',
                library, executable, root, chain)['scope'], 'OS-image-not-hashed')

    def test_fresh_import_rejects_cache_or_changed_source_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root/'dependency.py'
            marker = root/'executed'
            source.write_text(f'from pathlib import Path\nPath({str(marker)!r}).write_text("actual")\nvalue = 17\n')
            pins = {str(source): bootstrap.sha(source)}
            name = 'prolog_runtime_v2_fixture_dependency'
            sys.modules[name] = types.ModuleType(name)
            try:
                with self.assertRaises(AssertionError):
                    bootstrap.fresh_load(source, name, pins)
                self.assertFalse(marker.exists())
            finally:
                del sys.modules[name]
            source.write_text(source.read_text().replace('value = 17', 'value = 19'))
            with self.assertRaises(AssertionError):
                bootstrap.fresh_load(source, name, pins)
            self.assertFalse(marker.exists())
            pins = {str(source): bootstrap.sha(source)}
            # Invalid sibling bytecode cannot replace freshly verified source.
            cache = root/'__pycache__'
            cache.mkdir()
            (cache/'dependency.cpython-fixture.pyc').write_bytes(b'unreviewed-bytecode')
            try:
                actual = bootstrap.fresh_load(source, name, pins)
                self.assertEqual(actual.value, 19)
                self.assertEqual(marker.read_text(), 'actual')
                self.assertEqual(Path(actual.__file__).resolve(), source)
            finally:
                sys.modules.pop(name, None)

    def test_partial_attach_preserves_actual_device_entities_without_detach_authority(self):
        mount = Path('/private/candidate/owned-readonly-mount')
        device = {'dev-entry': '/dev/fixture-only-disk', 'content-hint': 'GUID_partition_scheme'}
        incomplete = bootstrap.attach_inventory(plistlib.dumps({'system-entities': [device]}), mount)
        self.assertEqual(incomplete['system_entities'], [device])
        self.assertEqual(incomplete['exact_requested_mount_entities'], [])
        self.assertIs(incomplete['unknown_device_detach_authorized'], False)
        proper = dict(device, **{'mount-point': str(mount)})
        value = bootstrap.attach_inventory(plistlib.dumps({'system-entities': [device, proper]}), mount)
        self.assertEqual(value['exact_requested_mount_entities'], [proper])
        self.assertIs(value['all_observed_mounts_are_requested'], True)
        foreign = {'mount-point': '/private/different-mount', 'dev-entry': '/dev/other-fixture'}
        value = bootstrap.attach_inventory(plistlib.dumps({'system-entities': [proper, foreign]}), mount)
        self.assertIs(value['all_observed_mounts_are_requested'], False)
        self.assertIs(value['unknown_device_detach_authorized'], False)
        for raw in (b'not-plist', plistlib.dumps({'system-entities': 'wrong'})):
            with self.assertRaises(Exception):
                bootstrap.attach_inventory(raw, mount)
