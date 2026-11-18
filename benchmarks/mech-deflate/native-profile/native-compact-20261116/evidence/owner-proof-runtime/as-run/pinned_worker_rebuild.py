"""Relink the benchmark harness when its original Cargo intermediates are missing."""
from hashlib import file_digest
from pathlib import Path
import argparse
import json
import subprocess
import sys

q = Path(__file__).resolve().parent
reference = q / 'vector-facts-root-proofs-stack-early'
rebuilt = q / 'pinned-host-link/rebuild'


def digest(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def rebuild_link(argv):
    artifacts = {}
    for line in (rebuilt / 'artifacts.jsonl').read_text().splitlines():
        row = json.loads(line)
        if row['reason'] != 'compiler-artifact':
            continue
        for item in map(Path, row['filenames']):
            if item.suffix == '.rlib' and item.parent.name == 'deps':
                artifacts[row['target']['name']] = item
            elif item.name == 'libmech_engine.rlib':
                artifacts['mech_engine'] = item
    dependencies = ('mech_engine', 'miniz_oxide', 'serde', 'serde_json')
    assert all(artifacts[name].is_file() for name in dependencies)
    deps = artifacts['serde'].parent
    command = []
    args = iter(argv)
    for arg in args:
        if arg == '--extern':
            external = next(args)
            name = external.split('=', 1)[0]
            if name == 'sha2':
                continue
            command.extend((arg, f'{name}={artifacts[name]}' if name in dependencies else external))
        elif arg == '-L':
            directory = next(args)
            if directory.startswith('dependency=') and not Path(directory[11:]).is_dir():
                directory = f'dependency={deps}'
            command.extend((arg, directory))
        else:
            command.append(arg)
    provenance = {
        'cargo_lock_sha256': digest(rebuilt / 'Cargo.lock'),
        'source_names_sha256': digest(rebuilt / 'source_names.rs'),
        'rebuilt_dependencies': {name: {'path': str(artifacts[name]), 'sha256': digest(artifacts[name])}
                                 for name in dependencies},
    }
    return command, provenance


def reference_link(out, worker='mech-worker.rs', executable='mech-worker'):
    argv = json.loads((reference / 'runtime.json').read_text())['link']
    argv = [arg.replace(str(reference / 'program.o'), str(out / 'program.o'))
            .replace(str(reference / 'mech-worker'), str(out / executable))
            for arg in argv]
    if worker != 'mech-worker.rs':
        original = q / 'pinned-host-link/mech-worker.rs'
        replacement = original.with_name(worker)
        assert argv.count(str(original)) == 1 and replacement.is_file()
        argv[argv.index(str(original))] = str(replacement)
    return argv


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('folder', type=Path)
    parser.add_argument('--worker', default='mech-worker.rs')
    parser.add_argument('--executable', default='mech-worker')
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    output = args.folder.resolve(strict=True)
    binary = output / args.executable
    argv, receipt = rebuild_link(reference_link(output, args.worker, args.executable))
    subprocess.run(argv, check=True)
    receipt.update(link=argv, worker_sha256=digest(binary),
                   object_sha256=digest(output / 'program.o'))
    (output / f'{args.executable}-link-receipt.json').write_text(
        json.dumps(receipt, indent=2) + '\n')
    if args.verify:
        subprocess.run([sys.executable, '-B', q / 'verify-worker.py', binary,
                        output / 'metadata.json', output.name], check=True)
