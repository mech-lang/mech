"""Unexecuted compiler-receipt inventory; call only under the shared mutex.

Loaded package records are independent of any LOC/source-bundle proposal.
The actual caller provides verified frozen qualification/reuse pins. This module
never runs a compiler, guesses a singleton bundle or qualifies a source role.
"""
from pathlib import Path
import re

PACKAGE = re.compile(r'^Building (.+) -> (/.*)$')


def package_records(stdout, stderr):
    records = []
    for stream, payload in (('stdout', stdout), ('stderr', stderr)):
        for line_number, line in enumerate(payload.splitlines(), 1):
            if not line.startswith('Building '):
                continue
            match = PACKAGE.fullmatch(line)
            assert match, ('Unproved compiler package record', stream, line_number, line)
            records.append(dict(label=match[1], path=match[2], stream=stream, line=line_number,
                                raw_line=line))
    assert records, 'Missing actual compiler loaded-package evidence'
    assert len(records) == len({item['path'] for item in records}), 'Duplicate package identity'
    return records


def command_option(command, name):
    indices = [index for index, value in enumerate(command) if value == name]
    assert len(indices) == 1 and indices[0] + 1 < len(command)
    return command[indices[0] + 1]


def command_search_paths(command):
    return [Path(command[index + 1]).resolve(strict=True)
            for index, value in enumerate(command[:-1]) if value == '--path']


def inventory_one(row, prior, build, stdout, stderr, pins, verify_file):
    """Bind every loaded package to the retained original compilation.

    verify_file checks a path against actual frozen report/reuse pins and returns
    its SHA. It must run under the caller's held common mutex, never here at import.
    """
    assert row['id'] == prior['declaration']['id']
    assert row['source'] == prior['source'] and row['source_sha256'] == prior['source_sha256']
    assert row['binary_sha256'] == prior['binary_sha256']
    command = build['command']
    assert command == prior['original_build_command']
    assert build['exit_status'] == build['final_exit_status'] == 0
    assert build['inherited_common_mutex'] is True and not build.get('error')
    assert not any(build.get(field) for field in (
        'cleanup_error', 'cleanup_errors', 'remaining_group_members', 'remaining_group',
        'supervisor_cleanup_error', 'supervisor_remaining_members'))
    assert command_option(command, '--bin-name') == row['id']
    source = Path(row['source']).resolve(strict=True)
    binary = Path(row['command'][0]).resolve(strict=True)
    assert binary == Path(prior['binary']).resolve(strict=True)
    assert binary.parent == Path(command_option(command, '--output')).resolve(strict=True)
    assert binary.name == row['id']
    linking = [line for payload in (stdout, stderr) for line in payload.splitlines() if line.startswith('Linking ')]
    assert linking == ['Linking ' + str(binary)], 'Actual compiler link target must match the reused binary'
    assert Path(command[-1]).resolve(strict=True) == source.parent
    assert verify_file(source, pins) == row['source_sha256']
    assert verify_file(binary, pins) == row['binary_sha256']
    compiler = Path(command[0]).resolve(strict=True)
    compiler_sha = verify_file(compiler, pins)
    snapshot = next(parent for parent in source.parents if parent.name == 'source-snapshot')
    # The original compiler's own standard packages and the actual two generic
    # library search roots are excluded, with every loaded input still inventoried.
    # A --path root under source-snapshot is authored, not a vendor exception.
    original_build_root = compiler.parents[3]
    build_receipt = Path(row['production_build_receipt'])
    assert compiler.name == 'ponyc' and compiler.parent.name == 'bin'
    assert compiler.parent.parent.name == '0.74.0-95ad180' and compiler.parents[2].name == 'release'
    assert original_build_root.parent.name == '.build'
    assert snapshot.parent.name == original_build_root.name == build_receipt.parents[2].name
    assert build_receipt.parents[1].name == 'builds' and build_receipt.parents[3].name == 'qualification'
    expected_vendors = {
        original_build_root / 'regex/regex-589eb008db250aab17d4c073a41c5562ac5b8dc7',
        original_build_root / 'gmp4pony/gmp4pony-b14db4ba347f6a2b840f135e1602e19f85625c39',
    }
    search = command_search_paths(command)
    assert {path for path in search if not path.is_relative_to(snapshot)} == expected_vendors
    vendor_roots = [compiler.parent.parent / 'packages', *sorted(expected_vendors)]
    assert all(path.exists() for path in vendor_roots)
    packages = package_records(stdout, stderr)
    bundle, excluded, unresolved = [], [], []
    saw_main = False
    for package in packages:
        directory = Path(package['path']).resolve(strict=True)
        assert directory.is_dir()
        if directory == source.parent:
            scope = 'application'
            assert not saw_main; saw_main = True
            assert package['label'] == command[-1]
        elif directory.is_relative_to(snapshot):
            scope = 'application-helper'
        elif any(directory.is_relative_to(root) for root in vendor_roots):
            scope = 'generic-vendor-source'
        else:
            raise AssertionError(('Loaded package has no qualified source role', str(directory)))
        # Tagged compiler package.c parses all direct, nonhidden .pony files in
        # deterministic filename order; it also handles direct .c shim inputs.
        # Nested package directories require their own actual Building record.
        files = sorted(path for path in directory.iterdir()
                       if not path.name.startswith('.') and path.is_file() and path.suffix in {'.pony', '.c'})
        assert any(path.suffix == '.pony' for path in files), ('Loaded package has no Pony source', directory)
        package_files = []
        for path in files:
            digest = verify_file(path, pins)
            item = dict(path=str(path), sha256=digest, role=scope, package=str(directory),
                        compiler_package_record=package, source_language='pony' if path.suffix == '.pony' else 'c')
            package_files.append(dict(path=str(path), sha256=digest, source_language=item['source_language']))
            if scope == 'generic-vendor-source':
                excluded.append(item)
            else:
                if path.suffix != '.pony':
                    unresolved.append(dict(kind='authored-native-shim-requires-own-grammar', source=str(path)))
                bundle.append(item)
        package.update(resolved_path=str(directory), proposed_scope=scope, files=package_files)
    assert saw_main, 'The original main package is absent from actual compiler log'
    assert len(bundle) == len({item['path'] for item in bundle})
    bundle.sort(key=lambda item: (item['path'] != str(source), item['role'] != 'application', item['path']))
    assert bundle[0]['path'] == str(source) and bundle[0]['sha256'] == row['source_sha256']
    return dict(id=row['id'], benchmark=row['benchmark'], variant=row['variant'],
        producer_loc=row.get('producer_loc', row.get('loc')),
        source=str(source), source_sha256=row['source_sha256'], binary=str(binary),
        binary_sha256=row['binary_sha256'], actual_original_compiler=str(compiler),
        actual_original_compiler_sha256=compiler_sha, actual_original_compile_command=command,
        production_build_receipt=row['production_build_receipt'], packages=packages,
        files=bundle, excluded_generic_vendor_files=excluded, unresolved=unresolved,
        status='compiler-input-proposal-awaiting-independent-source-role-review',
        source_role_reviewed=False, counts_started=False, canonical_LOC_admitted=False,
        chart_admission=False, native_cases_reexecuted=0)
