"""Conservative original-executable-aware Mach-O closure candidate.

Only a caller holding the common mutex may inspect real images or run tools.
The parsers consume scoped ARM64 otool output; no basename search substitutes
for an actual LC_RPATH. Ambiguous, relative or unknown install names diagnose.
"""
from pathlib import Path
import posixpath
import re

OS_PREFIXES = ('/usr/lib/', '/System/Library/')
DEPENDENCY_COMMANDS = {'LC_LOAD_DYLIB', 'LC_LOAD_WEAK_DYLIB',
    'LC_REEXPORT_DYLIB', 'LC_LOAD_UPWARD_DYLIB', 'LC_LAZY_LOAD_DYLIB'}


def scoped_lines(text, image):
    lines = text.splitlines()
    assert lines and lines[0] in (str(image) + ':', str(image) + ' (architecture arm64):'), \
        ('Not one scoped original image', image, lines[:1])
    return lines[1:]


def mach_header(text, image):
    lines = scoped_lines(text, image)
    headers = [index for index, line in enumerate(lines) if line.strip().startswith('magic ')]
    assert len(headers) == 1
    index = headers[0]
    names, values = lines[index].split(), lines[index + 1].split()
    assert len(values) >= len(names)
    record = dict(zip(names, values))
    assert record['magic'] == 'MH_MAGIC_64' and record['cputype'] == 'ARM64'
    assert record['filetype'] in ('EXECUTE', 'DYLIB', 'BUNDLE')
    count = int(record['ncmds'])
    assert count > 0
    return dict(architecture='arm64', filetype=record['filetype'], load_command_count=count)


def load_commands(text, image, expected_count):
    lines = scoped_lines(text, image)
    starts = []
    for index, line in enumerate(lines):
        match = re.fullmatch(r'Load command (\d+)', line.strip())
        if match:
            starts.append((index, int(match[1])))
    assert [ordinal for _, ordinal in starts] == list(range(expected_count))
    result = dict(install_id=None, dependencies=[], rpaths=[])
    for ordinal, (begin, _) in enumerate(starts):
        end = starts[ordinal + 1][0] if ordinal + 1 < len(starts) else len(lines)
        block = [line.strip() for line in lines[begin + 1:end]]
        command_lines = [line for line in block if line.startswith('cmd ')]
        assert len(command_lines) == 1
        command = command_lines[0].split()[1]
        if command in DEPENDENCY_COMMANDS or command == 'LC_ID_DYLIB':
            names = [line for line in block if line.startswith('name ')]
            assert len(names) == 1
            match = re.fullmatch(r'name (.+) \(offset \d+\)', names[0])
            assert match and match[1]
            if command == 'LC_ID_DYLIB':
                assert result['install_id'] is None, 'Duplicate dylib install ID'
                result['install_id'] = match[1]
            else:
                result['dependencies'].append(dict(command=command, install_name=match[1]))
        elif command == 'LC_RPATH':
            paths = [line for line in block if line.startswith('path ')]
            assert len(paths) == 1
            match = re.fullmatch(r'path (.+) \(offset \d+\)', paths[0])
            assert match and match[1]
            result['rpaths'].append(match[1])
        else:
            assert 'DYLIB' not in command and 'RPATH' not in command, ('Unknown linkage command', command)
    return result


def reconcile_listings(linked_text, id_text, image, header, commands):
    linked = []
    for line in scoped_lines(linked_text, image):
        if line.strip():
            match = re.fullmatch(r'(.+) \((?:compatibility version|current version) .+\)', line.strip())
            assert match, ('Unrecognized otool -L entry', line)
            linked.append(match[1])
    identifiers = [line.strip() for line in scoped_lines(id_text, image) if line.strip()]
    install_id = commands['install_id']
    if header['filetype'] == 'DYLIB':
        assert install_id is not None and identifiers == [install_id]
    else:
        assert install_id is None and identifiers == []
    expected = ([install_id] if install_id is not None else []) + \
        [entry['install_name'] for entry in commands['dependencies']]
    assert linked == expected, ('-L / -D / load-command mismatch', linked, expected)
    return commands


def os_install(path):
    value = str(path)
    # OS shared-cache install names need not be on-disk files. Normalize
    # lexically before the exemption so a parent traversal cannot borrow it.
    return value.startswith('/') and posixpath.normpath(value).startswith(OS_PREFIXES)


def expand_rpath(value, image, executable, root):
    if value.startswith('@loader_path/'):
        candidate = image.parent / value.split('/', 1)[1]
    elif value == '@loader_path':
        candidate = image.parent
    elif value.startswith('@executable_path/'):
        candidate = executable.parent / value.split('/', 1)[1]
    elif value == '@executable_path':
        candidate = executable.parent
    else:
        assert Path(value).is_absolute() and not value.startswith('@'), ('Unselected LC_RPATH', value)
        candidate = Path(value)
    candidate = candidate.resolve()
    assert candidate.is_relative_to(root.resolve()) or os_install(candidate), ('External LC_RPATH', value, candidate)
    return candidate


def rpath_chain(values, image, executable, root, inherited):
    result = []
    for value in [expand_rpath(v, image, executable, root) for v in values] + list(inherited):
        assert value.is_relative_to(root.resolve()) or os_install(value)
        if value not in result:
            result.append(value)
    return tuple(result)


def resolve_install(install, image, executable, root, rpaths):
    if os_install(install):
        return dict(install_name=install, scope='OS-image-not-hashed')
    if install.startswith('@loader_path/'):
        lookups = [image.parent / install.split('/', 1)[1]]
    elif install.startswith('@executable_path/'):
        lookups = [executable.parent / install.split('/', 1)[1]]
    elif install.startswith('@rpath/'):
        suffix = install[len('@rpath/'):]
        assert suffix and not Path(suffix).is_absolute()
        lookups = [base / suffix for base in rpaths]
        # Do not claim that an OS shared-cache candidate is absent merely
        # because it is not represented by a regular on-disk file.
        assert not any(os_install(path.resolve()) for path in lookups), ('Unresolved shared-cache rpath', install)
    else:
        assert Path(install).is_absolute() and not install.startswith('@'), ('Unselected install name', install)
        lookups = [Path(install)]
    targets = {}
    for lookup in lookups:
        if lookup.is_file():
            target = lookup.resolve(strict=True)
            assert target.is_relative_to(root.resolve()), ('Runtime image outside private app', install, target)
            targets.setdefault(target, []).append(lookup)
    assert len(targets) == 1, ('Unresolved/ambiguous actual install name', install, list(map(str, targets)))
    target, aliases = next(iter(targets.items()))
    return dict(install_name=install, scope='private-app-image', path=str(target),
                lookup_paths=list(map(str, aliases)))


def inspect_closure(preparation, service, guard, harness, executable, foreign, environment,
                    output, lock_fd, root):
    """Run only original owned image tools, retaining every actual raw receipt."""
    import json
    import os
    assert os.fstat(lock_fd).st_ino == os.stat('/tmp/mech-clbg-timing.lock').st_ino
    assert os.fstat(lock_fd).st_dev == os.stat('/tmp/mech-clbg-timing.lock').st_dev
    executable = Path(executable).resolve(strict=True)
    root = Path(root).resolve(strict=True)
    assert executable.is_relative_to(root)
    output.mkdir(parents=True)
    pending = [(executable, tuple(), None)]
    for image in foreign:
        path = Path(image).resolve(strict=True)
        assert path.is_relative_to(root)
        pending.append((path, None, 'feature-probe foreign-library'))
    seen = set()
    selected = {}
    entry_chain = None
    bindings = {}
    pins = {str(Path(tool).resolve(strict=True)): preparation.sha(Path(tool).resolve(strict=True))
            for tool in ('/usr/bin/lipo', '/usr/bin/otool')}
    records = []
    # The entry is inspected first so explicitly loaded foreign packages can
    # inherit its actual LC_RPATH context, never another package's directory.
    while pending:
        image, inherited, parent = pending.pop(0)
        if inherited is None:
            assert entry_chain is not None
            inherited = entry_chain
        key = (image, inherited)
        if key in seen:
            continue
        assert len(seen) < 1000, 'Unbounded/unknown dynamic linkage context'
        seen.add(key)
        directory = output / f'{len(records) + 1:03d}'
        directory.mkdir()
        image_digest = preparation.sha(image)
        pins = preparation.merge(pins, {str(image): image_digest})
        preparation.verify(pins)
        commands_run = {}
        payloads = {}
        for name, command in (
            ('architecture', ['/usr/bin/lipo', '-archs', str(image)]),
            ('mach-header', ['/usr/bin/otool', '-arch', 'arm64', '-hv', str(image)]),
            ('linked-images', ['/usr/bin/otool', '-arch', 'arm64', '-L', str(image)]),
            ('install-id', ['/usr/bin/otool', '-arch', 'arm64', '-D', str(image)]),
            ('load-commands', ['/usr/bin/otool', '-arch', 'arm64', '-l', str(image)])):
            result = service.owned_command(guard, harness, command, environment,
                directory, lock_fd, 30, name)
            receipt = json.loads((directory / (name + '.json')).read_text())
            assert receipt['command'] == command and receipt['inherited_common_mutex'] is True
            assert type(receipt['exit_status']) is int and receipt['exit_status'] == 0
            assert type(receipt['final_exit_status']) is int and receipt['final_exit_status'] == 0
            assert not receipt.get('cleanup_errors') and not receipt.get('error')
            payloads[name] = result['stdout'].decode('utf8')
            commands_run[name] = dict(command=command, receipt=str(directory / (name + '.json')))
        architectures = payloads['architecture'].split()
        assert len(architectures) == len(set(architectures)) and 'arm64' in architectures
        assert set(architectures) <= {'arm64', 'x86_64'}
        if image == executable:
            assert architectures == ['arm64']
        header = mach_header(payloads['mach-header'], image)
        if image == executable:
            assert header['filetype'] == 'EXECUTE'
        else:
            assert header['filetype'] in ('DYLIB', 'BUNDLE')
        loads = load_commands(payloads['load-commands'], image, header['load_command_count'])
        reconcile_listings(payloads['linked-images'], payloads['install-id'], image, header, loads)
        chain = rpath_chain(loads['rpaths'], image, executable, root, inherited)
        if image == executable:
            entry_chain = chain
        dependencies = []
        for entry in loads['dependencies']:
            resolved = resolve_install(entry['install_name'], image, executable, root, chain)
            resolved['load_command'] = entry['command']
            if resolved['scope'] == 'private-app-image':
                target = Path(resolved['path'])
                previous = bindings.setdefault((image, entry['install_name']), target)
                assert previous == target, 'Different inherited contexts select different native bytes'
                digest = preparation.sha(target)
                pins = preparation.merge(pins, {str(target): digest},
                    {alias: digest for alias in resolved['lookup_paths']})
                resolved['sha256'] = digest
                pending.append((target, chain, str(image)))
            dependencies.append(resolved)
        assert preparation.sha(image) == image_digest, 'Image changed during owned inspection'
        preparation.verify(pins)
        selected[str(image)] = image_digest
        record = dict(image=str(image), executable_anchor=str(executable), parent=parent,
            architectures=architectures, selected_architecture='arm64', mach_header=header,
            install_id=loads['install_id'], declared_rpaths=loads['rpaths'],
            actual_rpath_context=list(map(str, chain)), dependencies=dependencies,
            commands=commands_run, chart_admission=False)
        preparation.save_new(directory / 'dependency-review.json', record)
        records.append(record)
    preparation.save_new(output / 'closure.json', dict(status='passed-native-image-closure',
        executable=str(executable), selected_architecture='arm64',
        image_pins=selected, contexts=records, chart_admission=False,
        inherited_common_mutex=True, shared_timing_lock='/tmp/mech-clbg-timing.lock'))
    return pins, sorted(selected)
