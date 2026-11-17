"""Count exact no-op constant-phi sweeps using the archived compiler trace."""
from collections import defaultdict
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess
import sys

here = Path(__file__).resolve().parent
archive = Path('/private/tmp/mech-native-exclusive.Hc4E2q')
source = archive / 'source'
benchmark = source / 'benchmarks/mech-deflate/native-profile/compile_profile.rs'
target = archive / 'target/release/deps'
old = archive / 'run-shared'
record_path = archive / 'build-shared-1/cargo.log'
log_prefix = 'phi-constants'
pass_name = 'Remove constant phi-nodes'
pattern = ('Remove unreachable blocks', 'Verify Cranelift IR', pass_name, 'Verify Cranelift IR')
canonical_artifacts = ('source.mec', 'program.mecb', 'program.o', 'metadata.json')


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def cargo_receipts():
    for line in record_path.read_text().splitlines():
        if line.startswith('{'):
            yield json.loads(line)


def build():
    src = here / 'compile_profile.rs'
    binary = here / 'phi-census'
    assert not src.exists() and not binary.exists()
    text = benchmark.read_text()
    for relative in ('../../../src/compiler_allocator.rs',
                     '../../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs'):
        assert text.count(relative) == 1
        text = text.replace(relative, str((benchmark.parent / relative).resolve(strict=True)))
    marker = 'fn main() {\n'
    assert text.count(marker) == 1
    text = text.replace(marker, '''struct PhiLogger;

impl log::Log for PhiLogger {
    fn enabled(&self, meta: &log::Metadata<'_>) -> bool {
        meta.target() == "cranelift_codegen::remove_constant_phis"
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            let thread = std::thread::current();
            eprintln!("phi-constants\\t{}\\t{}", thread.name().unwrap_or("unnamed"), record.args());
        }
    }

    fn flush(&self) {}
}

fn main() {
    log::set_logger(&PhiLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
''')
    src.write_text(text)
    library_names = {'log', 'mech_engine', 'mech_gpu', 'mech_stdlib', 'mech_syntax', 'serde_json', 'sha2', 'mimalloc'}
    libraries = {}
    native_paths = set()
    for row in cargo_receipts():
        if row['reason'] == 'compiler-artifact' and row['target']['name'] in library_names:
            values = [Path(arg) for arg in row['filenames'] if arg.endswith('.rlib')]
            if values and not (row['target']['name'] == 'sha2' and 'std' not in row['features']):
                name = row['target']['name']
                assert name not in libraries, name
                libraries[name], = values
        if row['reason'] == 'build-script-executed':
            native_paths.update(row['linked_paths'])
    assert library_names == libraries.keys()
    args = ['rustc', '+nightly-2026-03-03', '--edition=2024', '--crate-name', 'compile_profile',
            '-C', 'opt-level=3', '-C', 'codegen-units=16', '-C', 'embed-bitcode=no', '-D', 'warnings',
            '-L', f'dependency={target}']
    for flag in sorted(native_paths):
        args.extend(('-L', flag))
    for name, path in sorted(libraries.items()):
        assert path.is_file(), path
        args.extend(('--extern', f'{name}={path}'))
    args.extend((str(src), '-o', str(binary)))
    with (here / 'build.log').open('wb') as log:
        subprocess.run(args, stdout=log, stderr=subprocess.STDOUT, check=True)
    write(here / 'build.json', {'source_sha256': digest(src), 'compiler_sha256': digest(binary),
                               'argv': args, 'libraries_sha256': {name: digest(path) for name, path in libraries.items()}})


def acquire():
    out = here / 'observed'
    out.mkdir()
    binary = here / 'phi-census'
    assert digest(binary) == json.loads((here / 'build.json').read_bytes())['compiler_sha256']
    original = json.loads((old / 'capture-inputs.json').read_bytes())
    args = [str(binary), original['compile'][1], str(out), 'compact']
    flags = original['flags'] | {'MECH_NATIVE_EXCLUSIVE_TRACE': str(out / 'native-exclusive.json')}
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    with (out / 'compile.log').open('wb') as log:
        subprocess.run(args, env=env | flags | {'PATH': '/usr/bin:/bin'}, stderr=log, stdout=log, check=True)
    pins = {name: digest(old / name) for name in canonical_artifacts}
    assert pins == {name: digest(out / name) for name in canonical_artifacts}
    write(out / 'plan.json', {'argv': args, 'flags': flags, 'artifacts': pins})


def native_scope(events, row):
    return None if row['parent'] is None else events[row['parent']]['detail']


def summary():
    out = here / 'observed'
    observed = json.loads((out / 'native-exclusive.json').read_bytes())
    archived = json.loads((old / 'native-exclusive.json').read_bytes())
    counts = defaultdict(list)
    for line in (out / 'compile.log').read_text().splitlines():
        if not line.startswith(log_prefix):
            continue
        _, name, msg = line.split('\t', 2)
        thread = 'coordinator' if name == 'compile-profile' else 'worker-' + name.removeprefix('mech-compile-')
        match = re.fullmatch(r'do_remove_constant_phis: done, (\d+) iters\.   (\d+) formals, of which (\d+) const\.', msg)
        assert match, msg
        counts[thread].append(tuple(map(int, match.groups())))
    assert len(counts) == len(observed), counts.keys()
    by_thread = {}
    for record in observed:
        thread = record['thread']
        body = record['events']
        phis = [row for row in body if row['detail'] == pass_name]
        assert len(phis) == len(counts[thread]), thread
        by_thread[thread] = [{'scope': native_scope(body, row), 'iterations': iters,
                              'formals': slots, 'constant_formals': fixed}
                             for row, (iters, slots, fixed) in zip(phis, counts[thread], strict=True)]
    coordinator = next(row['events'] for row in archived if row['thread'] == 'coordinator')
    indexed = [(i, native_scope(coordinator, row)) for i, row in enumerate(coordinator) if row['detail'] == pass_name]
    proof = by_thread['coordinator']
    assert [scope for _, scope in indexed] == [row['scope'] for row in proof]
    outer = defaultdict(list)
    for index, event in enumerate(coordinator):
        outer[event['parent']].append((index, event))
    candidates = {}
    for parent, rows in outer.items():
        for i in range(3, len(rows)):
            window = rows[i-3:i+1]
            if tuple(row['detail'] for _, row in window) != pattern:
                continue
            _, _, (phi_index, phi), (verify_index, verifier) = window
            assert phi['end_ns'] <= verifier['start_ns']
            candidates[phi_index] = {'verifier_event': verify_index,
                                     'archived_verifier_elapsed_ns': verifier['end_ns'] - verifier['start_ns']}
    report = defaultdict(lambda: {'adjacent_pairs': 0, 'noop_phis': 0, 'verification_ns': 0,
                                  'verification_after_noop_ns': 0})
    for (index, scope), counts in zip(indexed, proof, strict=True):
        if index not in candidates:
            continue
        candidate = candidates[index]
        entry = report[scope]
        elapsed = candidate['archived_verifier_elapsed_ns']
        entry['adjacent_pairs'] += 1
        entry['verification_ns'] += elapsed
        counts.update(archived_event=index, **candidate)
        if counts['constant_formals'] == 0:
            entry['noop_phis'] += 1
            entry['verification_after_noop_ns'] += elapsed
    data = {'counts': by_thread, 'archived_coordinator_receipt': digest(old / 'native-exclusive.json'),
            'exact_artifacts': json.loads((out / 'plan.json').read_bytes())['artifacts'],
            'candidate_pair_totals': report}
    write(here / 'report.json', data)
    print(report, flush=True)


if __name__ == '__main__':
    actions = {'build': build, 'acquire': acquire, 'summary': summary}
    for task in sys.argv[1:]:
        actions[task]()
