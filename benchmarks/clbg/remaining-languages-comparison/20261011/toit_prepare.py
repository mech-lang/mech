"""Preserve/port CLBG Toit sources and compile native standalone VMs offline."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys

D = Path(__file__).resolve().parent
SUPPORT = D / 'support/toit'
SDK = SUPPORT / 'toit'
TOIT = SDK / 'bin/toit'
PROJECT = SUPPORT / 'project'
SOURCES = D / 'sources/toit'
BIN = SUPPORT / 'bin'
LOGS = SUPPORT / 'build-logs'
for directory in (SOURCES, BIN, LOGS):
    directory.mkdir(parents=True, exist_ok=True)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def published():
    result = []
    for row in json.loads((D / 'upstream/toit/source-index.json').read_text()):
        original = D / row['source']
        benchmark, _, number = original.stem.split('-')
        if benchmark == 'revcomp':
            continue  # Its block scanner confuses multiple records in one read.
        source = original.read_text()
        adaptations = ['Compile before timing using Toit alpha.199 -O2 into a native arm64 standalone VM executable.']
        if 'import host.pipe' in source:
            source = source.replace('import host.pipe', 'import io.stdio').replace('pipe.stdout', 'stdio.stdout').replace('pipe.stdin.in', 'stdio.stdin').replace('pipe.stdin', 'stdio.stdin')
            source = re.sub(r'^\s*out\.close\s*$', '', source, flags=re.M)
            adaptations.append('Use SDK io.stdio for the same native standard streams; the obsolete host.pipe dependency is not required, and SDK streams are not closeable.')
        if benchmark == 'fasta':
            source = source.replace('return max * seed / M', 'return seed.to-float / 139968.0')
            adaptations.append('Explicit binary64 division by 139968.0 after every LCG advance; no scaled integer selection thresholds.')
            if number == '8':
                source = source.replace('  len ::= seq.size\n  i := 0; j := 0', '  len ::= seq.size\n  total := 0.0\n  len.repeat:\n    total += probability[it]\n    probability[it] = total\n  i := 0; j := 0')
                source = source.replace('      v -= probability[j]\n      if v<0: break', '      if v < probability[j]: break')
                adaptations.append('Replace subtracting individual probabilities with the required cumulative binary64 probability thresholds, retaining sequential symbol generation.')
        if benchmark == 'knucleotide':
            source = source.replace('    lines.add line', '    if not line.starts-with ";": lines.add line')
            source = source.replace('a[1] < b[1] ? 1 : -1', 'a[1] == b[1] ? (a[0].compare-to b[0]) : (a[1] < b[1] ? 1 : -1)')
            adaptations.append('Skip FASTA comment lines and enforce deterministic descending frequency/ascending nucleotide tie ordering; all seven growing maps and every window remain.')
        candidate = f'{benchmark}-toit-published-{number}'
        path = SOURCES / f'{candidate}.toit'
        path.write_text(source)
        result.append(dict(id=candidate, benchmark=benchmark, source=str(path),
                           source_url=row['url'], upstream_source=str(original),
                           upstream_source_sha256=sha(original), adaptations=adaptations,
                           local_adaptation=True))
    return result


def main():
    # Package resolution is attached to the application's source directory.
    for name in ('package.yaml', 'package.lock'):
        (SOURCES/name).write_bytes((PROJECT/name).read_bytes())
    dependency_link = SOURCES / '.packages'
    if not dependency_link.exists():
        dependency_link.symlink_to(PROJECT/'.packages', target_is_directory=True)
    programs = published()
    programs += json.loads((SUPPORT / 'custom-candidates.json').read_text())
    for row in programs:
        source = Path(row['source'])
        binary = BIN / row['id']
        log = LOGS / (row['id']+'.log')
        state = LOGS / (row['id']+'.json')
        command = [str(TOIT), 'compile', '-O2', '--project-root', str(SOURCES), '-o', str(binary), str(source)]
        cache = dict(source_sha256=sha(source), sdk_sha256=sha(TOIT), package_lock_sha256=sha(PROJECT / 'package.lock'))
        if not (state.exists() and binary.exists() and json.loads(state.read_text()).get('cache') == cache):
            proc = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            log.write_bytes(proc.stdout)
            if proc.returncode:
                print(proc.stdout.decode(errors='replace'), file=sys.stderr)
                raise RuntimeError('Compile failed: '+row['id'])
            state.write_text(json.dumps(dict(cache=cache, command=command, binary_sha256=sha(binary)), indent=2)+'\n')
        print('PREPARED', row['id'], flush=True)
        row.update(key=row['benchmark']+':'+row['id'], language='toit',
                   source_sha256=sha(source), binary=str(binary), binary_sha256=sha(binary),
                   command=[str(binary), '{n}'], compile_command=command,
                   algorithm=dict(strict=True, native_benchmark_kernel=False,
                                  application_language='Toit', complete_output=True),
                   execution='Optimized Toit bytecode compiled before timing, executed by an embedded native arm64 VM; cold process startup included.')
    libs = {}
    for prefix in (SDK / 'lib/toit/lib', PROJECT / '.packages'):
        for path in prefix.rglob('*'):
            if path.is_file():
                libs[str(path.resolve())] = sha(path)
    for path in (PROJECT / 'package.yaml', PROJECT / 'package.lock', SOURCES/'package.yaml', SOURCES/'package.lock'):
        libs[str(path)] = sha(path)
    runtime_paths = [TOIT, SDK / 'lib/toit/bin/toit.compile', SDK / 'lib/toit/bin/toit.run']
    version = subprocess.check_output([str(TOIT), 'version'], text=True).strip()
    manifest = dict(status='prepared-awaiting-validation', language='toit',
        runtimes={p.name:dict(path=str(p), sha256=sha(p), version=version) for p in runtime_paths},
        libraries=libs, programs=programs,
        provenance=dict(sdk_url='https://github.com/toitlang/toit/releases/download/v2.0.0-alpha.199/toit-macos.tar.gz',
                        sdk_archive=str(SUPPORT/'toit-macos-alpha199.tar.gz'),
                        sdk_archive_sha256=sha(SUPPORT/'toit-macos-alpha199.tar.gz'),
                        upstream_index=str(D/'upstream/toit/source-index.json'),
                        regexp_repository='https://github.com/close2/toit-dartino-regexp',
                        regexp_version='0.4.1', regexp_commit='768fed0da7102530356c5f94f2d373546892697a'))
    (D/'toit_manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')


if __name__ == '__main__':
    main()
