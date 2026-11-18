"""Build the benchmark source pipeline with catalog work overlapping parsing."""

from difflib import unified_diff
import json
import os
from pathlib import Path
import subprocess
import sys

from build import BINARY as TRAVERSAL_COMPILER, OUT as TRAVERSAL_BUILD
from stage import BASE, BRANCH, CHECKOUT, HERE, INCUMBENT, PACKET, PLAN, SOURCE, TARGET
from stage import git, read, sha, write

OUT = HERE / 'catalog'
PROFILE = PACKET.parent / 'compile_profile.rs'
MODE = 'MECH_COMPILE_OVERLAP_SOURCE_CATALOG'


def modify(text, point, replacement):
    assert text.count(point) == 1
    return text.replace(point, replacement, 1)


def source():
    original = git('show', f'{BASE}:{PROFILE}').decode()
    assert (CHECKOUT / PROFILE).read_text() == original
    content = modify(original, '    let (artifact, catalog) = {\n', '''    let load_catalog = || {
        let start = Instant::now();
        let catalog = mech_stdlib::source_catalog();
        (catalog, start.elapsed().as_secs_f64())
    };
    let preload = std::env::var_os("MECH_COMPILE_OVERLAP_SOURCE_CATALOG").and_then(|_| {
        std::thread::Builder::new()
            .name("source-catalog-preload".into())
            .spawn(load_catalog)
            .ok()
    });
    let overlapped_catalog = preload.is_some();
    let (artifact, catalog, catalog_work_seconds) = {
''')
    content = modify(content, '        let catalog = mech_stdlib::source_catalog();\n        stamp("catalog", start);',
        '''        let (catalog, catalog_work_seconds) = match preload {
            Some(worker) => worker
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload)),
            None => load_catalog(),
        };
        stamp("catalog", start);''')
    content = modify(content, '        (artifact, catalog)\n', '        (artifact, catalog, catalog_work_seconds)\n')
    content = modify(content, '        "phases": phases,\n',
        '''        "phases": phases,
        "catalog_work_seconds": catalog_work_seconds,
        "catalog_overlapped_parse": overlapped_catalog,
''')
    path = OUT / 'source' / PROFILE
    write(path, content)
    subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024',
                    '--config', 'skip_children=true', path], check=True)
    write(OUT / 'compiler.patch', ''.join(unified_diff(original.splitlines(keepends=True),
          path.read_text().splitlines(keepends=True), f'a/{PROFILE}', f'b/{PROFILE}')))
    return path


def library_hashes(path):
    needed = {'mech_syntax', 'mech_engine', 'mech_gpu', 'mech_stdlib'}
    libraries = {}
    for line in path.read_text().splitlines():
        if not line.startswith('{'):
            continue
        item = json.loads(line)
        if item.get('reason') == 'compiler-artifact' and item['target']['name'] in needed:
            (path,) = [Path(p) for p in item['filenames'] if p.endswith('.rlib')]
            libraries[item['target']['name']] = {'path': str(path), 'sha256': sha(path), 'fresh': item['fresh']}
    assert libraries.keys() == needed
    return libraries


def artifacts(folder):
    return {role: sha(folder / filename) for role, filename in
            (('bytecode', 'program.mecb'), ('encoder', 'program.o'), ('metadata', 'metadata.json'))}


def main():
    assert git('rev-parse', BRANCH).decode().strip() == BASE
    build = read(TRAVERSAL_BUILD / 'result.json')
    assert build['compiler_sha256'] == sha(TRAVERSAL_COMPILER)
    pinned = read(PLAN)['settings'][INCUMBENT]
    assert build['artifacts'] == pinned['artifact_sha256']
    OUT.mkdir()
    compiler_source = source()
    attempt = OUT / 'serial'
    argv = [sys.executable, '-B', CHECKOUT / PACKET / 'rebuild.py', attempt,
            '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
            '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    base_env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    overlay = {SOURCE: (HERE / 'source' / SOURCE).read_bytes(), PROFILE: compiler_source.read_bytes()}
    old = {file: ((CHECKOUT / file).stat(), (CHECKOUT / file).read_bytes()) for file in overlay}
    assert all(content == git('show', f'{BASE}:{file}') for file, (_, content) in old.items())
    assert sha(HERE / 'source' / SOURCE) == build['source']['candidate_sha256']
    previous_libraries = library_hashes(TRAVERSAL_BUILD / 'rebuilt/compiler-build.jsonl')
    syntax_suffix = Path(previous_libraries['mech_syntax']['path']).stem.removeprefix('libmech_syntax-')
    stamp = TARGET / f'release/.fingerprint/mech-syntax-{syntax_suffix}/invoked.timestamp'
    cache_time = stamp.stat().st_mtime_ns - 1_000_000_000
    binary = OUT / 'compile_profile'
    try:
        for file, code in overlay.items():
            (CHECKOUT / file).write_bytes(code)
        syntax = CHECKOUT / SOURCE
        # Cargo has already built these exact syntax inputs. Reuse them while
        # rebuilding the top-level entry, then restore the review checkout.
        os.utime(syntax, ns=(old[SOURCE][0].st_atime_ns, cache_time))
        with (OUT / 'build.log').open('w') as log:
            subprocess.run(argv, env=base_env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        compiler = TARGET / 'release/compile_profile'
        assert os.access(compiler, os.X_OK) and compiler.stat().st_size > 10_000_000
        subprocess.run(['/bin/cp', '-c', compiler, binary], check=True)
    finally:
        for file, (times, content) in old.items():
            local = CHECKOUT / file
            assert local.read_bytes() in (content, overlay[file])
            local.write_bytes(content)
            os.utime(local, ns=(times.st_atime_ns, times.st_mtime_ns))

    current_libraries = library_hashes(attempt / 'compiler-build.jsonl')
    for name, observed in current_libraries.items():
        assert observed['fresh'] and observed['sha256'] == previous_libraries[name]['sha256'], name
    serial = read(attempt / 'rebuild.json')
    assert serial['full_integrated_source_identity']
    assert sha(attempt / 'frontend/program.mecb') == pinned['artifact_sha256']['bytecode']
    assert not read(attempt / 'frontend/compile.json')['catalog_overlapped_parse']
    stage = OUT / 'overlap'
    stage.mkdir()
    enabled = [binary, Path(pinned['compile'][1]), stage / 'frontend', 'compact']
    with (stage / 'source.log').open('w') as log:
        subprocess.run(enabled, env=base_env | pinned['flags'] | {MODE: '1'}, check=True,
                       stdout=log, stderr=subprocess.STDOUT)
    assert artifacts(stage / 'frontend') == pinned['artifact_sha256']
    receipt = read(stage / 'frontend/compile.json')
    assert receipt['catalog_overlapped_parse'] and 0 < receipt['catalog_work_seconds'] < 1
    bad = OUT / 'invalid.mec'
    write(bad, 'x :=\n')
    for role, flags in (('serial', {}), ('overlap', {MODE: '1'})):
        failed = OUT / f'invalid-{role}'
        with (OUT / f'invalid-{role}.log').open('w') as log:
            result = subprocess.run([binary, bad, failed, 'frontend'],
                                    env=base_env | flags, stdout=log, stderr=subprocess.STDOUT)
        assert result.returncode != 0 and not (failed / 'program.mecb').exists()
        assert 'parse' in (OUT / f'invalid-{role}.log').read_text()
    stats = {'base': BASE, 'original_traversal_compiler_sha256': build['compiler_sha256'],
        'compiler_sha256': sha(binary), 'build_command': list(map(str, argv)),
        'overlap_command': list(map(str, enabled)), 'profile_source': str(PROFILE),
        'profile_source_sha256': sha(compiler_source), 'traversal_source_sha256': build['source']['candidate_sha256'],
        'compiler_patch_sha256': sha(OUT / 'compiler.patch'),
        'artifacts': pinned['artifact_sha256'], 'libraries': current_libraries,
        'probe_catalog_time_seconds': receipt['catalog_work_seconds'],
        'normal_recipe_sha256': sha(attempt / 'rebuild.json'), 'invalid_source_checks': 2}
    write(OUT / 'result.json', stats)
    print('Serial and overlapping entries reproduce the maintained artifacts.', stats['probe_catalog_time_seconds'], flush=True)


if __name__ == '__main__':
    main()
