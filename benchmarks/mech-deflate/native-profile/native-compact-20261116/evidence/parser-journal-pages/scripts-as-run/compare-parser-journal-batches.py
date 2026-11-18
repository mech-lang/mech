"""Compare identical parser builds using the retained compiler dependency set."""

from pathlib import Path
from statistics import median
import hashlib
import json
import os
import subprocess
import sys
import time

q = Path(__file__).resolve().parent
root = q.parent
area = q / 'native-review-pr'
current = area / 'checkout'
work = q / 'parser-journal-batch-study'
if '--selected' in sys.argv:
    work = work / 'selected'
    work.mkdir(exist_ok=True)
control = work / 'control-source'
control.mkdir(exist_ok=True)
parent = 'f11628e21630bd3cf9e3a662417b717709f45453'
source_root = 'src/syntax/src'
tree = subprocess.check_output(['git', 'archive', parent, source_root], cwd=root)
subprocess.run(['/usr/bin/tar', '-xf', '-', '-C', str(control)], input=tree, check=True)

profile = area / 'baseline-build/compiler/release'
deps = profile / 'deps'
fingerprints = profile / '.fingerprint'
pin, = fingerprints.glob('mech-syntax-*/lib-mech_syntax.json')
frontend_pin, = fingerprints.glob('mech-compression-native-profile-*/bin-native_compact_frontend.json')
syntax = json.loads(pin.read_text())
frontend = json.loads(frontend_pin.read_text())
builder = q / 'fresh-owner-counts/integrated-control/compiler-build.jsonl'
linked = set()
for line in builder.read_text().splitlines():
    try:
        item = json.loads(line)
    except json.JSONDecodeError:
        continue
    linked.update(item.get('linked_paths', []))


def library(name, ident):
    expected = ident.to_bytes(8, 'little').hex()
    path, = [p for p in fingerprints.glob(f'*/lib-{name}') if p.read_text().strip() == expected]
    suffix = path.parent.name.rsplit('-', 1)[1]
    found, = [p for ext in ['rlib', 'dylib'] if (p := deps / f'lib{name}-{suffix}.{ext}').is_file()]
    return found


def extern_arguments(values):
    return [part for name, path in values.items() for part in ['--extern', f'{name}={path}']]


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def dump(path, obj):
    path.write_text(json.dumps(obj, indent=2) + '\n')


def run(command, name, env=None):
    start = time.perf_counter()
    with (work / f'{name}.log').open('wb') as log:
        with subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, env=env) as proc:
            _, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = os.waitstatus_to_exitcode(status)
            if proc.returncode:
                raise subprocess.CalledProcessError(proc.returncode, command)
    return {'command': command, 'seconds': time.perf_counter() - start,
            'peak_rss_bytes': usage.ru_maxrss}


common = ['rustc', '+nightly-2026-03-03', '--edition=2024', '-C', 'opt-level=1',
          '-C', 'codegen-units=16', '-C', 'embed-bitcode=no', '-L', f'dependency={deps}', *syntax['rustflags']]
rust_flags = [part for f in json.loads(syntax['features']) for part in ['--cfg', f'feature="{f}"']]
syntax_extern = extern_arguments({name: library(name, ident) for _, name, _, ident in syntax['deps']})
link_paths = [part for path in sorted(linked) for part in ['-L', path]]
env = os.environ | {'CARGO_PKG_VERSION': '0.3.5', 'CARGO_MANIFEST_DIR': str(root / 'src/syntax')}
probe = work / 'probe.rs'
original_probe = (q / 'parser-optimization-profile/main.rs').read_text()
original_probe = original_probe.replace('fn main(){', '#[path = "' + str(current / 'src/compiler_allocator.rs') + '"]\nmod compiler_allocator;\nfn main(){')
probe.write_text(original_probe)
probe_libs = {name: library(name, ident) for _, name, _, ident in frontend['deps']
              if name in ['sha2', 'serde_json', 'mimalloc']}
assert set(probe_libs) == {'sha2', 'serde_json', 'mimalloc'}
builds = {}
hashes = {}
for name, checkout in [('control', control), ('batched', current)]:
    library_path = work / f'libmech_syntax_{name}.rlib'
    command = [*common, '--crate-name', 'mech_syntax', '--crate-type', 'rlib',
               '-C', f'metadata=parser-journal-batching-{name}', *rust_flags, *syntax_extern,
               str(checkout / source_root / 'lib.rs'), '-o', str(library_path)]
    build = '--measure-only' not in sys.argv
    builds[name] = run(command, f'{name}-build', env) if build else {'command': command}
    hashes[name] = sha(library_path)
    if build:
        run([*common, *link_paths, *extern_arguments(probe_libs | {'mech_syntax': library_path}),
             str(probe), '-o', str(work / name)], f'{name}-link', env)
        print(name, 'parser compiled', builds[name]['seconds'], flush=True)

unit = [*common, '--crate-name', 'mech_syntax', '--test', *rust_flags, *syntax_extern,
        str(current / source_root / 'lib.rs'), '-o', str(work / 'syntax-tests')]
test_deps = q / 'parser-test-dependencies/target/release/deps'
proptests = list(test_deps.glob('libproptest-*.rlib'))
if proptests:
    unit += ['-L', f'dependency={test_deps}', '--extern', f'proptest={proptests[0]}']
test_build = run(unit, 'unit-build', env) if '--measure-only' not in sys.argv else {'command': unit}
test_command = [str(work / 'syntax-tests')]
test = run(test_command, 'unit-tests', env) if '--measure-only' not in sys.argv else {'command': test_command}
assert '531 passed; 0 failed; 0 ignored;' in (work / 'unit-tests.log').read_text()
print((work / 'unit-tests.log').read_text().splitlines()[-2], flush=True)

source = current / 'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
trials = []
for i, order in enumerate([('control', 'batched'), ('batched', 'control')] * 3):
    for actor in order:
        output = work / f'order-{i + 1}-{actor}.json'
        receipt = run([str(work / actor), str(source), str(output)], output.stem)
        sample = json.loads(output.read_text()) | {'actor': actor, 'order': i + 1, **receipt}
        trials.append(sample)
        print(actor, sample['parse_seconds'], sample['tree_sha256'], flush=True)
assert len({(x['tree_sha256'], x['source_sha256'], x['parser_steps'], x['events']) for x in trials}) == 1
report = {'parent': parent, 'trials': trials, 'sha256_parser_libraries': hashes,
          'extra_flags': syntax['rustflags'], 'builds': builds, 'unit_build': test_build, 'unit_test': test,
          'scope': 'Parser-only comparison using the same compiler options, dependency builds, and mimalloc allocator. The probe hashes the full tree after stopping the timer.',
          'medians': {name: median(x['parse_seconds'] for x in trials if x['actor'] == name)
                      for name in builds}}
dump(work / 'parser-comparison.json', report)
print(json.dumps(report['medians'], indent=2), flush=True)
