"""Prepare inspectable Smalltalk applications and immutable native Pharo images.

Compilation/fileIn is outside all benchmark timings. run_command includes cold VM
startup, image loading, application compute and I/O, and VM shutdown. No full
measurements are made by this preparation script.
"""
from pathlib import Path
import argparse
import hashlib
import json
import re
import shutil
import subprocess
import time

D = Path(__file__).resolve().parent
SUPPORT = D / "support/smalltalk"
SOURCES = D / "sources/smalltalk"
UPSTREAM = D / "upstream/smalltalk"
VM = SUPPORT / "vm/Pharo.app/Contents/MacOS/Pharo"
BASE = next((SUPPORT / "base-image").glob("*.image"))
IMAGES = SUPPORT / "images"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def normalize(source):
    # Smalltalk comments are double quoted, strings single quoted, and
    # characters ($.) do not terminate a statement. Count independent packed
    # assignments and cascade actions as separate lines, preserving formatting.
    token = re.compile(r'"[^\"]*"|\x27(?:\x27\x27|[^\x27])*\x27|\$[^\n]', re.S)
    def mask(match):
        text = match[0]
        if text.startswith('"'):
            return '\n' * text.count('\n')
        return 'S' + '\n' * text.count('\n')
    cleaned = token.sub(mask, source)
    physical = sum(bool(line.strip()) for line in cleaned.splitlines())
    expansions = []
    for number, line in enumerate(cleaned.splitlines(), 1):
        if not line.strip():
            continue
        statements = max(1, len(re.findall(r':=', line)), line.count(';') + (1 if ';' in line else 0))
        if statements > 1:
            expansions.append(dict(line=number, extra_lines=statements - 1,
                                   reason='Independent packed assignments or cascade actions.'))
    return dict(physical_loc=physical, normalized_loc=physical + sum(e['extra_lines'] for e in expansions), expansions=expansions)


def published_sources():
    # Upstream candidates are not reduced to opaque wrappers. All classes,
    # accessors, compute methods, and application stream extensions remain here.
    chosen = [
        ('binarytrees', 1), ('fannkuchredux', 1), ('fasta', 1), ('fasta', 2),
        ('knucleotide', 2), ('mandelbrot', 1), ('mandelbrot', 2),
        ('nbody', 1), ('pidigits', 1), ('revcomp', 2), ('spectralnorm', 1),
    ]
    result = []
    for benchmark, version in chosen:
        upstream = UPSTREAM / f'{benchmark}-pharo-{version}.st'
        source = upstream.read_text()
        source = source.replace("category: 'benchmarks game'", "category: 'Benchmarks-Game'")
        source = source.replace('!StdioStream instance methodsFor:', '!StdioStream methodsFor:')
        notes = ['Use the Pharo 14 uppercase package-name and standard instance-side fileIn protocol syntax; all computation remains Smalltalk.']
        if benchmark == 'pidigits':
            source = source.replace('n to: width do:', 'n + 1 to: width do:')
            source = source.replace('output space', 'output nextPut: Character space')
            notes.append('Correct partial-line padding: exactly 10 minus remaining digits spaces, retaining arbitrary precision Pharo integers and the full spigot.')
            source = source.replace('| tmp |\n    numer > accum', '| tmp digit |\n    numer > accum')
            source = source.replace('tmp \\\\ denom >= (denom - numer) ifTrue: [^nil].\n    ^tmp // denom',
                                    'digit := tmp // denom.\n    digit = ((tmp + numer) // denom) ifFalse: [^nil].\n    ^digit')
            notes.append('Replace the upstream remainder shortcut with both exact extract(3) and extract(4) divisions, as explicitly required by the current CLBG description.')
        if benchmark == 'knucleotide':
            source = source.replace('[b key < a key]', '[a key < b key]')
            notes.append('Use ascending lexical ordering for equal frequencies; retain every full string-key growable-map window for all seven k lengths.')
        if benchmark == 'mandelbrot':
            source = source.replace('zr := cr. zi := ci.', 'zr := 0.0d0.\n         zi := 0.0d0.')
            source = source.replace('i := 1.', 'i := 0.')
            source = source.replace('< limit2 and:', '<= limit2 and:')
            notes.append('Compute canonical z0=0, at most 50 iterates, and reject only squared norm >4; retain complete PBM bytes and row padding.')
        if benchmark == 'revcomp':
            # The published #2 program deliberately scans ONE/TWO/THREE. The
            # canonical suite has those records; more general FASTA is prepared
            # as a separate streaming candidate below.
            notes.append('Published buffered Pharo implementation; computes and emits the complete three canonical FASTA records.')
        path = SOURCES / f'{benchmark}-smalltalk-published-{version}.st'
        path.write_text(source)
        result.append(dict(id=path.stem, key=f'{benchmark}:{path.stem}',
                           benchmark=benchmark, language='smalltalk', source=str(path),
                           upstream_source=str(upstream), upstream_sha256=sha(upstream),
                           source_url=f'https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/{benchmark}-pharo-{version}.html',
                           local_adaptation=bool(notes), adaptations=notes,
                           profile_candidates=['fastest', 'smallest']))
    return result


def st_string(text):
    return "'" + text.replace("'", "''").replace('!', '!!') + "'"


def chunks(source):
    pieces, start, i, quote = [], 0, 0, None
    while i < len(source):
        ch = source[i]
        if quote:
            if ch == quote:
                if quote == "'" and i + 1 < len(source) and source[i + 1] == "'":
                    i += 1
                else:
                    quote = None
        elif ch in "'\"":
            quote = ch
        elif ch == '$':
            i += 1
        elif ch == '!':
            pieces.append(source[start:i].strip())
            start = i + 1
        i += 1
    pieces.append(source[start:].strip())
    return pieces


def compile_loader(source):
    # Generic offline Pharo 12/14 fileIn compatibility. The application source
    # stays in normal inspectable class/method chunks, compiled verbatim here.
    operations, active, protocol = [], None, ''
    for chunk in chunks(source):
        if not chunk:
            active = None
            continue
        header = re.fullmatch(r"(\w+)(?:\s+(class|instance))?\s+methodsFor:\s+'((?:''|[^'])*)'", chunk)
        if header:
            name, side, protocol = header.groups()
            active = f'(Smalltalk globals at: #{name})' + (' class' if side == 'class' else '')
        elif active:
            operations.append(f'{active} compile: {st_string(chunk)} classified: {st_string(protocol)}.')
        elif 'subclass:' in chunk and 'instanceVariableNames:' in chunk:
            operations.append(f'OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: {st_string(chunk)}).')
        else:
            operations.append(f'Smalltalk compiler evaluate: {st_string(chunk)}.')
    return '\n'.join(operations) + '\n'


def build(row):
    source = Path(row['source'])
    image = IMAGES / f"{row['id']}.image"
    changes = image.with_suffix('.changes')
    old = image.with_suffix('.build.json')
    loader = SUPPORT / 'build-logs' / f"{row['id']}.loader.st"
    loader.write_text(compile_loader(source.read_text()))
    if old.exists():
        previous = json.loads(old.read_text())
        if previous.get('source_sha256') == sha(source) and image.exists() and previous.get('image_sha256') == sha(image) and previous.get('vm_sha256') == sha(VM) and previous.get('loader_sha256') == sha(loader):
            row.update(previous)
            return row
    shutil.copyfile(BASE, image)
    shutil.copyfile(BASE.with_suffix('.changes'), changes)
    command = [str(VM), '--headless', str(image), 'st', '--no-source', '--save', '--quit', str(loader)]
    process = subprocess.run(command, cwd=SUPPORT, capture_output=True, timeout=120)
    log = SUPPORT / 'build-logs' / f"{row['id']}.log"
    log.write_bytes(process.stdout + process.stderr)
    row.update(source_sha256=sha(source), vm_sha256=sha(VM),
               loader_sha256=sha(loader),
               binary=str(image), build_command=command, build_log=str(log),
               build_status='passed' if process.returncode == 0 else 'failed',
               exit_code=process.returncode)
    row['algorithm'] = dict(strict=True, native_benchmark_kernel=False,
                            application_language='Smalltalk', complete_output=True)
    if process.returncode == 0:
        row.update(image_sha256=sha(image), binary_sha256=sha(image),
                   run_command=[str(VM), '--headless', str(image), 'eval',
                                'BenchmarksGame do: {n}. Stdio stdout flush. Smalltalk exitSuccess'],
                   command=[str(VM), '--headless', str(image), 'eval',
                            'BenchmarksGame do: {n}. Stdio stdout flush. Smalltalk exitSuccess'],
                   cwd=str(SUPPORT), execution='Pharo native ARM64 Cog/Spur JIT, cold image process',
                   loc=normalize(source.read_text()))
        old.write_text(json.dumps(row, indent=2) + '\n')
    else:
        print(process.stderr.decode(errors='replace')[-6500:], flush=True)
    return row


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--published-only', action='store_true')
    parser.add_argument('--only', default='')
    args = parser.parse_args()
    for p in [SOURCES, IMAGES, SUPPORT / 'build-logs']:
        p.mkdir(parents=True, exist_ok=True)
    for source in (SUPPORT / 'base-image').glob('*.sources'):
        link = IMAGES / source.name
        if not link.exists():
            link.symlink_to(source)
    candidates = published_sources()
    custom = SUPPORT / 'custom-candidates.json'
    if not args.published_only and custom.exists():
        candidates.extend(json.loads(custom.read_text()))
    if args.only:
        candidates = [r for r in candidates if any(x in r['id'] for x in args.only.split(','))]
    rows = []
    for row in candidates:
        print('BUILD', row['id'], flush=True)
        rows.append(build(row))
        print(rows[-1]['build_status'], row['id'], flush=True)
    manifest = dict(status='prepared' if all(r['build_status'] == 'passed' for r in rows) else 'partial',
                    language='smalltalk', implementation='Pharo', release_year=1972,
                    image_version='Pharo14.0 SNAPSHOT 3221b560a7, 2026-10-08',
                    vm_version='Pharo v12.0.5-beta commit 7884d2853, 2026-09-22',
                    vm=str(VM), vm_sha256=sha(VM), base_image=str(BASE), base_image_sha256=sha(BASE),
                    image_archive_sha256=sha(SUPPORT / 'Pharo140-image.zip'),
                    vm_archive_sha256=sha(SUPPORT / 'Pharo140-vm.zip'),
                    install_urls=['https://pharo.org/download', 'https://get.pharo.org/64/140+vm',
                                  'https://files.pharo.org/get-files/140/pharoImage-arm64.zip',
                                  'https://files.pharo.org/get-files/140/pharo-vm-Darwin-arm64-stable.zip'],
                    timings_policy='No full timings in preparation. Full run_command includes cold native VM, loading compiled image, all computation/I/O, and shutdown; image fileIn/compile excluded.',
                    source_policy='All application classes, helpers, FFI, compute and application stream methods count in LOC. Generic eval entry expression only dispatches numeric argv to a compiled Smalltalk application and flushes/exits; no host executor.',
                    runtimes={'pharo': {'path': str(VM), 'sha256': sha(VM), 'version': 'v12.0.5-beta 7884d2853, Pharo14.0 SNAPSHOT 3221b560a7'}},
                    libraries={str(p): sha(p) for p in (SUPPORT / 'vm').rglob('*') if p.is_file()},
                    programs=rows)
    manifest['libraries'].update({r['binary']: r['binary_sha256'] for r in rows if r['build_status'] == 'passed'})
    pcre = Path('/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib')
    manifest['libraries'][str(pcre)] = sha(pcre)
    (D / 'smalltalk_manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


if __name__ == '__main__':
    main()
