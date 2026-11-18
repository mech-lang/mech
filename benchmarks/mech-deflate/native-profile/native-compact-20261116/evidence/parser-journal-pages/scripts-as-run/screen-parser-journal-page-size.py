"""Measure page sizes against the same batched parser and level-12 source."""

from pathlib import Path
from statistics import median
import json
import os
import shutil
import subprocess
import time

q = Path(__file__).resolve().parent
work = q / 'parser-journal-batch-study'
report = json.loads((work / 'parser-comparison.json').read_text())
syntax = q / 'native-review-pr/checkout/src/syntax/src'
drivers = work / 'probe.rs'
candidate = work / 'page-screen'
candidate.mkdir(exist_ok=True)
flags = report['builds']['batched']['command']
deps = q / 'native-review-pr/baseline-build/compiler/release/deps'
artifacts = q / 'fresh-owner-counts/integrated-control/compiler-build.jsonl'
env = os.environ | {'CARGO_PKG_VERSION': '0.3.5', 'CARGO_MANIFEST_DIR': str(q.parent / 'src/syntax')}
fingerprints = deps.parent / '.fingerprint'
frontend_pin, = fingerprints.glob('mech-compression-native-profile-*/bin-native_compact_frontend.json')
selected = {name: ident.to_bytes(8, 'little').hex()
            for _, name, _, ident in json.loads(frontend_pin.read_text())['deps']
            if name in ['serde_json', 'sha2', 'mimalloc']}
extra = {}
for name, expected in selected.items():
    pin, = [p for p in fingerprints.glob(f'*/lib-{name}') if p.read_text().strip() == expected]
    extra[name] = deps / f'lib{name}-{pin.parent.name.rsplit("-", 1)[1]}.rlib'
native_paths = set()
for line in artifacts.open():
    try:
        row = json.loads(line)
    except json.JSONDecodeError:
        continue
    native_paths.update(row.get('linked_paths', []))
assert set(extra) == {'serde_json', 'sha2', 'mimalloc'}
commands = {}


def run(argv, stem):
    start = time.perf_counter()
    with (candidate / f'{stem}.log').open('wb') as log:
        with subprocess.Popen(argv, stdout=log, stderr=subprocess.STDOUT, env=env) as proc:
            _, status, usage = os.wait4(proc.pid, 0)
            proc.returncode = os.waitstatus_to_exitcode(status)
            if proc.returncode:
                raise subprocess.CalledProcessError(proc.returncode, argv)
    return {'command': list(map(str, argv)), 'seconds': time.perf_counter() - start, 'peak_rss_bytes': usage.ru_maxrss}


actors = {'page-4': work / 'batched'}
for size in [8, 16, 32]:
    name = f'page-{size}'
    source = candidate / f'{name}-src'
    shutil.copytree(syntax, source)
    journal = source / 'document/parser/journal.rs'
    s = journal.read_text()
    assert s.count('const PAGE: usize = 4;') == 1
    journal.write_text(s.replace('const PAGE: usize = 4;', f'const PAGE: usize = {size};'))
    lib = candidate / f'libmech_syntax_{size}.rlib'
    cmd = [f'metadata=parser-page-{size}' if a.startswith('metadata=')
           else str(source / 'lib.rs') if a.endswith('/src/syntax/src/lib.rs')
           else str(lib) if a.endswith('/libmech_syntax_batched.rlib')
           else a for a in flags]
    commands[name] = run(cmd, f'{name}-build')
    actor = candidate / name
    link = ['rustc', '+nightly-2026-03-03', '--edition=2024', '-D', 'warnings', '-C', 'opt-level=1',
            '-C', 'codegen-units=16', '-C', 'embed-bitcode=no', '-L', f'dependency={deps}',
            *[part for path in sorted(native_paths) for part in ['-L', path]],
            *[part for crate, path in (extra | {'mech_syntax': lib}).items() for part in ['--extern', f'{crate}={path}']],
            str(drivers), '-o', str(actor)]
    run(link, f'{name}-link')
    actors[name] = actor
    print(name, 'built', flush=True)

program = q / 'native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
trials = []
groups = [(0, 1, 3, 2), (1, 2, 0, 3), (2, 3, 1, 0), (3, 0, 2, 1)]
for i, group in enumerate(groups, 1):
    for index in group:
        key, actor = list(actors.items())[index]
        out = candidate / f'order-{i}-{key}.json'
        elapsed = run([str(actor), str(program), str(out)], out.stem)
        row = json.loads(out.read_text())
        assert row['tree_sha256'] == report['trials'][0]['tree_sha256']
        assert row['parser_steps'] == report['trials'][0]['parser_steps']
        assert row['events'] == report['trials'][0]['events']
        trials.append(row | elapsed | {'actor': key, 'order': i})
        print(key, 'parse', row['parse_seconds'], 'RSS', elapsed['peak_rss_bytes'], flush=True)
medians = {key: {field: median(row[field] for row in trials if row['actor'] == key)
                 for field in ['parse_seconds', 'peak_rss_bytes']} for key in actors}
(candidate / 'report.json').write_text(json.dumps({'scope': 'Parser only; all variants use the same batched code except for PAGE.',
    'trials': trials, 'medians': medians, 'builds': commands}, indent=2) + '\n')
print(json.dumps(medians, indent=2), flush=True)
