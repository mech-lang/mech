"""Measure the journal change through final linking with the retained encoder."""

from importlib import import_module
from pathlib import Path
from statistics import median
import json
import subprocess
import sys

measure = import_module('compare-builds')
q = measure.q
out = measure.work / 'apple-m1-parser-journal-pages-current'
out.mkdir()
retained = q / 'cranelift-continuation-registers-word-pages-native/final-candidate'
measure.runtime = json.loads((retained / 'runtime.json').read_text())
measure.reference_pins = json.loads((retained / 'receipt.json').read_text())
binaries = q / 'native-review-pr/baseline-build/compiler/release'
measure.emitter = binaries / 'emit_compact_size'
frontends = {
    'mech-control': q / 'parser-journal-batch-study/control-native-frontend',
    'mech-journal-pages': binaries / 'native_compact_frontend',
}
library, = (v.removeprefix('mech_gpu=') for v in measure.runtime['link'] if v.startswith('mech_gpu='))
measure.mech_link = [library if path.endswith('/libmech_gpu_runtime.rlib') else path for path in measure.mech_link]
actors = ['mech-control', 'mech-journal-pages', 'roc', 'libdeflate']


def run(actor, directory, profile=False):
    if actor in frontends:
        measure.frontend = frontends[actor]
    result = measure.measure('mech' if actor in frontends else actor, directory, profile=profile)
    result['actor'] = actor
    if actor in frontends:
        parsed = json.loads((directory / 'frontend/compile.json').read_text())
        result['seconds']['parse'] = next(row['seconds'] for row in parsed['phases'] if row['phase'] == 'parse')
    (directory / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    return result


warmup = [run(actor, out / 'warmup' / actor) for actor in actors]
orders = ((0, 1, 3, 2), (1, 2, 0, 3), (2, 3, 1, 0), (3, 0, 2, 1))
trials = [run(actors[index], out / f'order-{order}' / actors[index])
          for order, cohort in enumerate(orders, 1) for index in cohort]
profiles = [run(actor, out / 'profile' / actor, profile=True) for actor in actors]
reference = out / 'warmup/mech-control'
compared = ['frontend/program.mecb', 'native/program.o', 'native/metadata.json', 'compressor']
for row in [*warmup, *trials, *profiles]:
    executable = Path(row['executable'])
    if row['actor'] in frontends:
        for file in compared:
            assert (reference / file).read_bytes() == (executable.parent / file).read_bytes(), (row['actor'], file)
    else:
        measure.verify_peer(executable)
for actor in frontends:
    directory = out / 'warmup' / actor
    subprocess.run([sys.executable, '-B', q / 'verify-worker.py', directory / 'compressor',
                    directory / 'native/metadata.json', f'matched-journal-pages-{actor}'], check=True)

sys.path.insert(0, str(q))
macho = import_module('reachability-census')
sizes = {}
for actor in actors:
    directory = out / 'warmup' / actor
    executable = directory / 'compressor'
    if actor == 'roc':
        archive = directory / 'encoder.a'
        members = subprocess.check_output(['/usr/bin/ar', '-t', archive], text=True).splitlines()
        member, = [m for m in members if m.startswith('roc_app_')]
        encoder = directory / 'encoder-code.o'
        encoder.write_bytes(subprocess.check_output(['/usr/bin/ar', '-p', archive, member]))
    else:
        encoder = directory / ('native/program.o' if actor in frontends else 'encoder.o')
    _, sections, _ = macho.load(encoder)
    _, linked, _ = macho.load(executable)
    sizes[actor] = {
        'emitted_text_bytes': sections[macho.TEXT][2], 'encoder_file_bytes': encoder.stat().st_size,
        'linked_text_bytes': linked[macho.TEXT][2], 'executable_file_bytes': executable.stat().st_size,
        'encoder_sha256': measure.sha(encoder),
    }
medians = {}
for actor in actors:
    rows = [row['seconds'] for row in trials if row['actor'] == actor]
    assert len(rows) == 4
    medians[actor] = {key: median(row[key] for row in rows) for key in rows[0]}
record = {
    'machine': 'Apple M1',
    'scope': 'Fresh encoder source through final executable link; compilers, runtimes and adapters prebuilt.',
    'mech_compiler_build': 'Rust opt-level 1, no PGO. Only the parser implementation differs between Mech frontends. Both use the same native emitter and runtime.',
    'source_sha256': measure.sha(measure.source_mech),
    'frontends': {k: {'path': str(v), 'sha256': measure.sha(v)} for k, v in frontends.items()},
    'emitter_sha256': measure.sha(measure.emitter), 'runtime_sha256': measure.sha(Path(library)),
    'roc_compiler_sha256': measure.sha(measure.roc), 'roc_source_sha256': measure.sha(measure.roc_app),
    'c_source_sha256': measure.sha(measure.source_c / 'lib/deflate_compress.c'),
    'orders': [[actors[i] for i in order] for order in orders], 'warmup': warmup, 'trials': trials,
    'diagnostic_profiles_excluded_from_medians': profiles,
    'medians': medians, 'sizes': sizes, 'byte_identical_across_both_frontends_and_all_builds': compared,
    'verified_exact_calls': 324,
    'runtime_score_scope': 'The rebuilt workers passed correctness. The exact identity check of the pinned worker is a separate run.',
}
(out / 'timing.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps({'medians': medians, 'sizes': sizes}, indent=2), flush=True)
