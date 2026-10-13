"""Additive Crystal archive-link-v4 metadata repair with exact bounded-proof reuse.

No full timings, scored manifests, installer mutation or chart write. Root must
review the exact sealed proposal before running a fresh explicit revision.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tarfile
import urllib.request

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
V = CLBG / 'v-report-only/20261012'
GUARD = SUITE / 'uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
GUARD_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PROPOSAL = HERE / 'proposals/private-input-archive-v4.json'
RETAINED_BUILD = HERE / '.build/private-layout-v2'
SDK = Path('/Library/Developer/CommandLineTools/SDKs/MacOSX14.4.sdk')
CLANG = Path('/opt/homebrew/opt/llvm@22/bin/clang-22').resolve()
GMP = Path('/opt/homebrew/Cellar/gmp/6.3.0')
PCRE2 = Path('/opt/homebrew/Cellar/pcre2/10.49')
PKGCONF = Path('/opt/homebrew/Cellar/pkgconf/3.0.7/bin/pkgconf')
ARCHIVE_URL = 'https://github.com/crystal-lang/crystal/releases/download/1.21.1/crystal-1.21.1-1-darwin-universal.tar.gz'
ARCHIVE_SHA = 'ba34fa7fb8bc8b951f12d17e12f69c9d7ccce38900caa4b8a17f1b4f29d37605'
SOURCE_COMMIT = '73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef'
NUMERIC = {
    'nbody': (0, 1, 2, 10000),
    'mandelbrot': (0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 65, 129, 200),
    'spectralnorm': (1, 2, 3, 5, 9, 100),
    'binarytrees': (0, 4, 7, 9),
    'fannkuchredux': (1, 2, 3, 4, 5, 6, 7, 8),
    'pidigits': (0, 1, 9, 10, 11, 20, 21, 27, 30, 97),
    'fasta': (0, 1, 2, 29, 30, 31, 60, 61, 287, 574, 20000),
}
BENCHMARKS = set(NUMERIC) | {'knucleotide', 'revcomp', 'regexredux'}
PREREQUISITES = {'preparation-fixtures.json': 12, 'work-trace-fixtures.json': 12,
                 'layout-v2-fixtures.json': 8, 'input-v3-fixtures.json': 10, 'archive-v4-fixtures.json': 6}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_guard():
    assert sha(GUARD) == GUARD_SHA, 'Frozen native ownership guard changed'
    return load_module('crystal_private_native_guard', GUARD)


def merge(*groups):
    pins = {}
    for group in groups:
        for path, digest in group.items():
            assert path not in pins or pins[path] == digest, ('Conflicting pin', path)
            pins[path] = digest
    return pins


def prerequisite(path, count):
    receipt = json.loads(Path(path).read_text())
    assert receipt['status'] == 'passed' and receipt['tests_run'] == count
    assert receipt['failures'] == receipt['errors'] == 0
    assert receipt['native_probes'] == receipt['actual_signals_sent'] == 0
    for path, digest in receipt['pins'].items():
        assert sha(path) == digest, ('Fixture or prerequisite source changed', path)
    return receipt


def static_paths():
    selected = set(HERE.glob('*.py')) | set(HERE.glob('*.md')) | {HERE / 'profiles.json'}
    selected |= set((HERE / 'sources').glob('*/*.cr')) | set((HERE / 'support').glob('*.cr'))
    selected |= {p for p in (HERE / 'research').rglob('*') if p.is_file()}
    return sorted(selected)


def metadata_paths():
    return [GUARD, SUITE / 'run.py', SUITE / 'prepare.py', CLANG, PKGCONF,
            GMP / 'include/gmp.h', GMP / 'lib/libgmp.10.dylib',
            GMP / 'lib/pkgconfig/gmp.pc', PCRE2 / 'include/pcre2.h',
            PCRE2 / 'lib/libpcre2-8.0.dylib', PCRE2 / 'lib/pkgconfig/libpcre2-8.pc',
            SDK / 'SDKSettings.json', SDK / 'usr/include/stdio.h',
            SDK / 'usr/include/pthread.h', SDK / 'usr/lib/libSystem.tbd',
            V / 'references.py', V / 'test_references.py', V / 'oracle-fixtures.json',
            V / 'oracle-fixtures.log', V / 'source_trace.py', V / 'dna_oracles.py',
            V / 'test_dna_oracles.py', V / 'dna-oracle-fixtures.json', V / 'dna-oracle-fixtures-001.log',
            HERE / 'metadata-failures/input-v3-seal-001/failure.json']


def owned_command(guard, harness, command, environment, directory, lock_fd,
                  timeout=180, name='command'):
    """Cleanup is established before Popen and covers identity/journal failure."""
    directory.mkdir(parents=True, exist_ok=True)
    receipt = directory / (name + '.json')
    assert not receipt.exists(), 'No overwrite or implicit command retry'
    detail = dict(command=list(command), started_at=guard.now(),
                  inherited_common_mutex=True, preparation_only=True,
                  never_chart_eligible=True, timeout_seconds=timeout)
    process = None
    stdout, stderr = b'', b''
    cleanup_error = None
    try:
        process = subprocess.Popen(command, cwd=HERE, env=environment,
                                   stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, start_new_session=True,
                                   pass_fds=(lock_fd,))
        detail['pid'] = process.pid
        detail['identity'] = guard.identity(process.pid)
        if detail['identity'] is None and process.poll() is None:
            raise RuntimeError('Owned live child has no process identity')
        guard.save(directory / (name + '.active.json'), detail)
        stdout, stderr = process.communicate(timeout=timeout)
        detail['exit_status'] = process.returncode
        assert process.returncode == 0, ('Owned command failed', command, process.returncode)
        assert not harness.group_members(process.pid), 'Owned command left a native child'
    except BaseException as error:
        detail.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        if isinstance(error, subprocess.TimeoutExpired):
            stdout, stderr = error.output or b'', error.stderr or b''
        raise
    finally:
        if process is not None:
            try:
                harness.kill_group(process.pid)
            except BaseException as error:
                cleanup_error = error
                detail.update(cleanup_error=str(error), cleanup_error_type=type(error).__name__)
            finally:
                process.wait()
                detail['final_exit_status'] = process.returncode
                try:
                    stdout, stderr = process.communicate(timeout=5)
                except BaseException as error:
                    cleanup_error = error
                    detail.update(output_drain_error=str(error), output_drain_error_type=type(error).__name__)
                finally:
                    if process.stdout is not None:
                        process.stdout.close()
                    if process.stderr is not None:
                        process.stderr.close()
        detail['finished_at'] = guard.now()
        (directory / (name + '.stdout')).write_bytes(stdout)
        (directory / (name + '.stderr')).write_bytes(stderr)
        new_json(receipt, detail)
        if cleanup_error is not None:
            raise RuntimeError('Owned command cleanup failed; diagnostics retained') from cleanup_error
    return dict(stdout=stdout, stderr=stderr, detail=detail)


def proposal_path(value):
    path = Path(value).resolve()
    assert path.is_relative_to(HERE) and path.suffix == '.json', 'Use an additive cohort JSON proposal path'
    return path


def seal(proposal_file):
    print('WAIT LOCK: seal additive Crystal archive-link/reuse proposal', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not proposal_file.exists(), 'Never replace an earlier reviewed proposal'
        references = load_module('crystal_seal_reference_v3', V / 'references.py')
        preflight_inputs(references)
        fixtures = {name: prerequisite(HERE / name, count) for name, count in PREREQUISITES.items()}
        # The fourteen reference fixtures are reused with their exact receipt,
        # source and log pins. The V producer uses its own receipt shape.
        oracle = json.loads((V / 'oracle-fixtures.json').read_text())
        assert oracle['status'] == 'passed' and oracle['tests_run'] == 14
        dna = json.loads((V / 'dna-oracle-fixtures.json').read_text())
        assert dna['status'] == 'passed' and dna['tests_run'] == 8
        guard = load_guard()
        harness = guard.load_harness()
        sdk_result = owned_command(guard, harness, ['/usr/bin/xcrun', '--show-sdk-path'],
                                  dict(os.environ), HERE / 'proposal-seals' / proposal_file.stem, lock.fileno(), 30, 'sdk-path')
        assert Path(sdk_result['stdout'].decode().strip()).resolve(strict=True) == SDK.resolve(strict=True)
        rows = json.loads((HERE / 'profiles.json').read_text())['profiles']
        assert len(rows) == 20 and len({r['id'] for r in rows}) == 20
        assert {(r['benchmark'], r['variant']) for r in rows} == {(b, v) for b in BENCHMARKS for v in ('compact', 'optimized')}
        assert all(r['chart_eligible'] is False and r['loc'] is None and r['maximum_workers'] <= 8 for r in rows)
        paths = static_paths() + metadata_paths() + [HERE / name for name in PREREQUISITES]
        # Keep lookup aliases as pins as well as real targets: a changed dylib
        # symlink must not silently select another image with its old target
        # still present and correctly hashed.
        pins = merge({str(p): sha(p) for p in paths}, {str(p.resolve()): sha(p) for p in paths},
                     *(r['pins'] for r in fixtures.values()), oracle['pins'], dna['pins'])
        guard.verify(pins)
        import proof_reuse_v4
        reuse = proof_reuse_v4.reconstruct(sys.modules[__name__], guard, harness,
            HERE / 'proposal-seals' / proposal_file.stem / 'prior-proof-reuse', lock.fileno())
        reuse_path = HERE / 'proposal-seals' / proposal_file.stem / 'prior-proof-reuse.json'
        new_json(reuse_path, reuse)
        pins = merge(pins, reuse['pins'], {str(reuse_path): sha(reuse_path)})
        guard.verify(pins)
        proposal = dict(prior_proof_reuse=str(reuse_path), prior_proof_reuse_sha256=sha(reuse_path),
                        reused_profile_ids=sorted(reuse['profiles']), reused_bounded_cases=112,
                        fresh_profile_builds_required=5, fresh_bounded_output_cases_required=44,
                        snapshot_paths=[str(p) for p in static_paths()],
                        status='sealed-awaiting-root-approval', created_at=guard.now(),
                        pins=pins, profiles=rows, fixtures=fixtures,
                        compiler_archive_url=ARCHIVE_URL, compiler_archive_sha256=ARCHIVE_SHA,
                        compiler_source_commit=SOURCE_COMMIT, sdk=str(SDK.resolve()),
                        shared_timing_lock=str(LOCK), guard_sha256=GUARD_SHA,
                        full_processes_authorized=0, full_timings=False, chart_admission=False,
                        no_automatic_retry=True, native_dependency_installation=False,
                        SDK_pin_scope='explicit sentinel files, not whole SDK/operating system',
                        source_revision='input-label-archive-v4',
                        preserved_prior_proposal='proposals/private-layout-v2.json',
                        preserved_prior_proposal_sha256='a3c202e1d21c052263f0bf7d1ed1aa2143ce4a3f9e1a77b3a1e75301aec57c93',
                        prior_failure='Input-case label collision after112 qualified probes; v3 metadata seal stopped resolving an official foreign-target circular symlink.',
                        preserved_metadata_failure='metadata-failures/input-v3-seal-001/failure.json',
                        pending_gates=['remaining required-work review', 'canonical normalized LOC audit',
                                       'independent per-benchmark full manifest review',
                                       'three actual canonical fulls and fresh matched Mech controls'])
        new_json(proposal_file, proposal)
        print(json.dumps(dict(proposal=str(proposal_file), sha256=sha(proposal_file)), indent=2), flush=True)


def package_layout(release):
    """Exact official archive layout; never treat its shell launcher as Mach-O."""
    package = release / 'crystal-1.21.1-1'
    assert package.is_dir() and sorted(p.name for p in release.iterdir()) == ['crystal-1.21.1-1'], 'Unexpected official release root'
    wrapper = package / 'bin/crystal'
    compiler = package / 'embedded/bin/crystal'
    prelude = package / 'src/prelude.cr'
    gc = package / 'embedded/lib/libgc.a'
    assert wrapper.is_file() and wrapper.read_bytes().startswith(b'#!'), 'Expected official shell launcher'
    assert compiler.is_file(), 'Missing exact embedded native compiler; no fallback'
    with compiler.open('rb') as stream:
        magic = stream.read(4)
    assert magic in (bytes.fromhex('cafebabe'), bytes.fromhex('bebafeca'),
                     bytes.fromhex('cafebabf'), bytes.fromhex('bfbafeca'),
                     bytes.fromhex('cffaedfe'), bytes.fromhex('feedfacf')), 'Embedded compiler is not native Mach-O'
    assert prelude.is_file() and gc.is_file(), 'Missing exact standard library or bundled Boehm GC'
    # External recorded GMP/PCRE2 must win before the bundled GC directory,
    # which also contains a different static PCRE2 archive.
    libraries = [GMP / 'lib', PCRE2 / 'lib', gc.parent]
    assert len({p.resolve() for p in libraries}) == 3
    return compiler, prelude.parent, libraries, wrapper


def pin_tree(root):
    pins = {}
    root = root.resolve()
    for path in sorted(root.rglob('*')):
        if path.is_file():
            assert path.resolve().is_relative_to(root), ('Package symlink escapes private package', path)
            pins[str(path)] = sha(path)
            pins[str(path.resolve())] = sha(path)
    return pins


def download(url, destination):
    assert not destination.exists(), 'Never reuse an incomplete archive'
    request = urllib.request.Request(url, headers={'User-Agent': 'mech-private-crystal-source-audit'})
    with urllib.request.urlopen(request, timeout=60) as response, destination.open('xb') as stream:
        while data := response.read(1024 * 1024):
            stream.write(data)


def extract(archive, destination):
    destination.mkdir()
    with tarfile.open(archive) as bundle:
        bundle.extractall(destination, filter='data')


def build_command(compiler, sdk, library_paths, source, binary):
    flags = ['-isysroot', str(sdk), '-mmacosx-version-min=14.4']
    flags += [item for root in library_paths for item in ('-L', str(root))]
    return [str(compiler), 'build', '--release', '--no-debug', '--no-color',
            '--error-trace', '--mcpu', 'native', '--threads', '1',
            '--link-flags', shlex.join(flags), '-o', str(binary), str(source)]


def inputs(benchmark, references):
    if benchmark == 'knucleotide':
        return references.dna_fixtures() + (('header-eof', b'>THREE'), ('empty-input', b''),
            ('exact-token-vtab', b'>THREE\vwrong\nAAAA\n>THREE\tselected\nGGTATTTTAATTTATAGT'),
            ('tail', b';before\r\n>THREE\tselected\r\nggtattttaatttatagtacgt'))
    if benchmark == 'revcomp':
        return (('iupac', b'>one\nACGTUMRWSYKVHDBNacgtumrwsykvhdbn\n>empty\n>two\nAACCGGTT'),
                ('comment-crlf-eof', b'>one\r\naCgTu\r\n;>ignored\r\nMrWsYkVhDbN'),
                ('tail', b'>one\n' + b'ACGTUMRWSYKVHDBN' * 13), ('empty', b''), ('header-eof', b'>one'))
    assert benchmark == 'regexredux'
    return (('all-replacements', b'>one\nagggtaaatttaccct\nAGGGTAAA\ntHaNtaNDcaNHaDWaSaNaSaNBY<x>|hello|\n'),
            ('records', b'>one\nagggtaaa\n>two\ntttaccct\n'),
            ('header-eof', b'>one\nagggtaaa\n>unterminated'),
            ('replacement-boundaries', b'>one\ntHaNtHatHaTtaNDcaNHaDHaSWaSaNaSaNBY<<x>>|a||b|\n'), ('empty', b''))


def output_validation(harness, benchmark, expected, stream, destination):
    stream.seek(0)
    actual = stream.read()
    destination.write_bytes(actual)
    if benchmark == 'nbody':
        harness.compact.common.validate(actual.decode(), [float(x) for x in expected.splitlines()])
    else:
        assert actual == expected, ('Complete bounded output differs', benchmark, actual[:160], expected[:160])
    return dict(reference_match=True, bytes=len(actual), sha256=hashlib.sha256(actual).hexdigest())


def probe(guard, harness, row, n, data, expected, pins, output, lock_fd, report, case, trace=None):
    benchmark = row['benchmark']
    old_work = harness.WORK[benchmark]
    existed = benchmark in harness.INPUTS
    old_input = harness.INPUTS.get(benchmark)
    old_validate = harness.validate
    directory = output / row['id'] / case
    try:
        # All writes and temporary harness changes are inside this restoration
        # scope, including input/oracle preparation errors before native launch.
        directory.mkdir(parents=True)
        (directory / 'expected.stdout').write_bytes(expected)
        if data is not None:
            filename = directory / 'stdin.fasta'
            filename.write_bytes(data)
            harness.INPUTS[benchmark] = (filename, old_input[1] if existed else filename)
        harness.WORK[benchmark] = (n, old_work[1])

        def validate(actual_benchmark, full, stream):
            assert actual_benchmark == benchmark and full is False
            return output_validation(harness, benchmark, expected, stream, directory / 'actual.stdout')

        harness.validate = validate
        all_pins = merge(pins, {str(path): sha(path) for path in directory.iterdir() if path.is_file()})
        manifest = dict(size=n, canonical_full_workload=False, timeout_seconds=300,
                        maximum_rss_mib=10240, pins=all_pins,
                        full_output_reference=None if benchmark == 'nbody' else
                            dict(bytes=len(expected), sha256=hashlib.sha256(expected).hexdigest()))
        try:
            attempt = guard.one_attempt(harness, row, manifest, report, 0, lock_fd)
        except BaseException as error:
            index = len(report['attempts']) + 1
            raw = guard.OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
            enriched = raw.with_suffix('.supervisor.json')
            actual = enriched if enriched.exists() else raw
            attempt = json.loads(actual.read_text()) if actual.exists() else {}
            if attempt.get('error'):
                attempt['execution_failure_before_preparation_error'] = dict(
                    error=attempt['error'], error_type=attempt.get('error_type'))
            attempt.update(qualified=False, retryable_environment_only=False,
                preparation_supervisor_failure=True, preparation_error=str(error) or type(error).__name__,
                preparation_error_type=type(error).__name__, case=case, id=row['id'], benchmark=benchmark,
                preparation_only=True, never_chart_eligible=True, full_performance_run=False)
            if actual.exists():
                attempt.update(actual_execution_result_path=str(actual), actual_execution_result_sha256=sha(actual))
            wrapped = raw.with_suffix('.preparation.json')
            attempt['preparation_result_path'] = str(wrapped)
            new_json(wrapped, attempt)
            report['attempts'].append(attempt)
            report['active'] = None
            guard.save(guard.REPORT, report)
            raise
        attempt.update(case=case, preparation_only=True, never_chart_eligible=True,
                       full_performance_run=False, measurement_kind=row.get('measurement_kind', 'bounded-output'))
        if trace is not None and attempt['qualified']:
            try:
                attempt['source_work_proof'] = trace(directory)
            except BaseException as error:
                attempt.update(output_qualified_before_source_work=True, qualified=False,
                               source_work_failure=True, source_work_error=str(error) or type(error).__name__,
                               source_work_error_type=type(error).__name__)
                new_json(directory / 'failed-work-review.json', attempt)
        report['attempts'].append(attempt)
        report['active'] = None
        guard.save(guard.REPORT, report)
        assert attempt['qualified'], ('Unqualified attempt retained; no retry', row['id'], case)
        assert attempt['full_workload'] is False
        return attempt, directory
    finally:
        harness.WORK[benchmark] = old_work
        harness.validate = old_validate
        if existed:
            harness.INPUTS[benchmark] = old_input
        else:
            harness.INPUTS.pop(benchmark, None)


def native_images(guard, harness, binary, environment, output, lock_fd, allowed_roots):
    """Inspect actual linked images; reject unresolved non-system dependencies."""
    arch = owned_command(guard, harness, ['/usr/bin/lipo', '-archs', str(binary)],
                         environment, output, lock_fd, 30, 'architecture')['stdout'].decode().split()
    assert 'arm64' in arch, ('Missing native ARM64 image', binary, arch)
    result = owned_command(guard, harness, ['/usr/bin/otool', '-L', str(binary)],
                           environment, output, lock_fd, 30, 'linked-images')['stdout'].decode()
    dependencies = []
    for line in result.splitlines()[1:]:
        install = line.strip().split(' (', 1)[0]
        if install.startswith(('/usr/lib/', '/System/Library/')):
            dependencies.append(dict(install_name=install, scope='OS-system-image-not-hashed'))
            continue
        if install.startswith('@executable_path/'):
            path = binary.parent / install[len('@executable_path/'):]
        elif install.startswith('@loader_path/'):
            path = binary.parent / install[len('@loader_path/'):]
        elif install.startswith('@rpath/'):
            suffix = install[len('@rpath/'):]
            candidates = [root / suffix for root in allowed_roots if (root / suffix).is_file()]
            assert len({p.resolve() for p in candidates}) == 1, ('Unresolved/ambiguous runtime library', install)
            path = candidates[0]
        else:
            path = Path(install)
        lookup = path
        path = lookup.resolve(strict=True)
        assert any(path.is_relative_to(root.resolve()) for root in allowed_roots), ('Unreviewed linked dependency', path)
        dependencies.append(dict(install_name=install, path=str(path), lookup_path=str(lookup), sha256=sha(path)))
    new_json(output / 'dependency-review.json', dict(binary=str(binary), architectures=arch, dependencies=dependencies))
    return {path: row['sha256'] for row in dependencies if 'path' in row
            for path in (row['path'], row['lookup_path'])}


def runtime_environments(stdlib, libraries):
    runtime_environment = dict(LC_ALL='C', CRYSTAL_WORKERS='8',
        DYLD_LIBRARY_PATH=os.pathsep.join(str(p.resolve()) for p in libraries),
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
    environment = dict(os.environ, LC_ALL='C', SDKROOT=str(SDK), CC=str(CLANG),
        PATH=str(PKGCONF.parent) + ':/usr/bin:/bin', CRYSTAL_PATH=str(stdlib),
        CRYSTAL_LIBRARY_PATH=os.pathsep.join(str(p) for p in libraries),
        PKG_CONFIG_LIBDIR=os.pathsep.join(str(p / 'pkgconfig') for p in libraries),
        PKG_CONFIG_PATH='', **{k: v for k, v in runtime_environment.items() if k != 'LC_ALL'})
    return runtime_environment, environment


def retained_release():
    return RETAINED_BUILD / 'release'


def preflight_inputs(references):
    """Validate every case namespace before any compile or native process."""
    inventories = {benchmark: tuple(inputs(benchmark, references)) for benchmark in
                   ('knucleotide', 'revcomp', 'regexredux')}
    assert set(NUMERIC) | set(inventories) == BENCHMARKS
    for benchmark, sizes in NUMERIC.items():
        assert sizes and all(type(n) is int and n >= 0 for n in sizes)
        names = ['n-' + str(n) for n in sizes]
        assert len(set(names)) == len(names), ('Duplicate numeric bounded case label', benchmark)
    for benchmark, cases in inventories.items():
        names = [name for name, data in cases]
        assert names and len(set(names)) == len(names), ('Duplicate bounded input-case label', benchmark, names)
        assert all(re.fullmatch(r'[a-z0-9][a-z0-9-]*', name) for name in names)
        assert all(isinstance(data, bytes) for name, data in cases)
    assert dict(inventories['knucleotide'])['empty'] == b'>THREE\n>FOUR\nGGG\n'
    assert dict(inventories['knucleotide'])['empty-input'] == b''
    return inventories


def run(digest, revision, proposal_file):
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', revision or ''), 'Explicit fresh revision required'
    print('WAIT LOCK: guarded Crystal additive archive-link revision', revision, flush=True)
    with (HERE / 'preparation-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert sha(proposal_file) == digest, 'Exact root-reviewed proposal changed'
            proposal = json.loads(proposal_file.read_text())
            guard = load_guard()
            guard.verify(proposal['pins'])
            assert proposal['source_revision'] == 'input-label-archive-v4'
            assert proposal['reused_bounded_cases'] == 112 and proposal['fresh_profile_builds_required'] == 5
            reuse_path = Path(proposal['prior_proof_reuse'])
            assert sha(reuse_path) == proposal['prior_proof_reuse_sha256']
            reuse = json.loads(reuse_path.read_text())
            guard.verify(reuse['pins'])
            proposal['pins'] = merge(proposal['pins'], {str(proposal_file): digest})
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            references = load_module('crystal_independent_references_v3', V / 'references.py')
            dna = load_module('crystal_independent_exact_dna_v3', V / 'dna_oracles.py')
            references.knucleotide = dna.knucleotide
            inventories = preflight_inputs(references)
            import proof_reuse_v4
            assert set(reuse['profiles']) == proof_reuse_v4.PROFILES
            # Link metadata is checked independently of regular-file hashes:
            # foreign-platform self links cannot be read as file payloads.
            archive = RETAINED_BUILD / 'crystal-1.21.1-1-darwin-universal.tar.gz'
            guard.verify(proof_reuse_v4.archive_correspondence(sys.modules[__name__], archive, retained_release()))
            output = HERE / 'qualification' / revision
            assert not output.exists(), 'No overwrite or implicit failed-revision recovery'
            output.mkdir(parents=True)
            guard.OUT, guard.REPORT = output, output / 'report.json'
            report = dict(status='preparing', started_at=guard.now(), proposal_sha256=digest,
                attempts=[], active=None, full_performance_runs=0, chart_admission=False,
                never_chart_eligible=True, no_automatic_retry=True,
                exact_prior_proof=str(reuse_path), exact_prior_proof_sha256=sha(reuse_path),
                reused_native_cases=112, prior_cases_reexecuted=0,
                prior_records_relabelled_as_new_attempts=False)
            guard.save(guard.REPORT, report)
            try:
                snapshot = output / 'source-snapshot'
                snapshot.mkdir()
                for filename in proposal['snapshot_paths']:
                    source = Path(filename)
                    assert source.is_relative_to(HERE) and filename in proposal['pins']
                    destination = snapshot / source.relative_to(HERE)
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, destination)
                    assert sha(destination) == proposal['pins'][filename]
                build = HERE / '.build' / revision
                build.mkdir(parents=True)
                compiler, stdlib, libraries, wrapper = package_layout(retained_release())
                assert str(compiler) == reuse['compiler'] and str(stdlib) == reuse['source_root']
                assert [str(p) for p in libraries] == reuse['library_paths']
                runtime_environment, environment = runtime_environments(stdlib, libraries)
                assert runtime_environment == reuse['runtime_environment']
                new_json(output / 'package-layout.json', dict(status='exact-retained-layout-v2-toolchain',
                    compiler=str(compiler), source_root=str(stdlib), shell_wrapper=str(wrapper),
                    compiler_archive_sha256=ARCHIVE_SHA, old_toolchain_mutated=False,
                    old_archive_reextracted=False, full_processes_authorized=0))
                pins = merge(proposal['pins'], reuse['pins'], pin_tree(snapshot))
                guard.verify(pins)
                source_proofs = load_module('crystal_preparation_work_proofs_v3', snapshot / 'source_proofs_v3.py')
                binaries = build / 'bin'
                binaries.mkdir()
                rows = []
                for declared in proposal['profiles']:
                    completed = set()
                    if declared['id'] in reuse['profiles']:
                        prior = reuse['profiles'][declared['id']]
                        assert prior['declaration'] == declared
                        row = proof_reuse_v4.row_from_evidence(prior)
                        proof_reuse_v4.require_identity(json.loads(prior['execution_fingerprint']),
                            sha(row['source']), sha(row['binary']))
                        assert sha(snapshot / declared['source']) == row['source_sha256']
                        assert json.loads(harness.execution_fingerprint(row)) == json.loads(prior['execution_fingerprint'])
                        completed = {r['case'] for r in prior['prior_records']}
                    else:
                        source = snapshot / declared['source']
                        binary = binaries / declared['id']
                        cache = build / 'cache' / declared['id']
                        cache.mkdir(parents=True)
                        this_environment = dict(environment, CRYSTAL_CACHE_DIR=str(cache))
                        command = build_command(compiler, SDK, libraries, source, binary)
                        owned_command(guard, harness, command, this_environment,
                            output / 'builds' / declared['id'], lock.fileno(), 600, 'build')
                        pins = merge(pins, pin_tree(cache), {str(binary): sha(binary)},
                            native_images(guard, harness, binary, environment,
                                output / 'builds' / declared['id'] / 'images', lock.fileno(),
                                [retained_release(), GMP, PCRE2]))
                        guard.verify(pins)
                        row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='crystal',
                            source=str(source), source_sha256=sha(source), binary=str(binary), binary_sha256=sha(binary),
                            command=[str(binary), '{n}'], cwd=str(HERE), runtime_environment=runtime_environment,
                            qualification_origin='fresh-input-label-archive-v4-build')
                    rows.append(row)
                    benchmark = row['benchmark']
                    fresh = 0
                    for n in NUMERIC.get(benchmark, ()):
                        case = 'n-' + str(n)
                        if case in completed:
                            continue
                        expected = getattr(references, benchmark)(n)
                        attempt, directory = probe(guard, harness, row, n, None, expected, pins, output,
                            lock.fileno(), report, case)
                        pins = merge(pins, {str(p): sha(p) for p in directory.iterdir() if p.is_file()})
                        fresh += 1
                    if benchmark not in NUMERIC:
                        for case, data in inventories[benchmark]:
                            if case in completed:
                                continue
                            expected = getattr(references, benchmark)(data)
                            attempt, directory = probe(guard, harness, row, harness.WORK[benchmark][0], data, expected,
                                pins, output, lock.fileno(), report, case)
                            pins = merge(pins, {str(p): sha(p) for p in directory.iterdir() if p.is_file()})
                            fresh += 1
                    row['new_native_cases'] = fresh
                assert len(rows) == 20 and len(report['attempts']) == 44
                work = source_proofs.run(sys.modules[__name__], guard, harness, proposal, snapshot,
                    build, compiler, libraries, environment, rows, pins, lock.fileno(), report, references)
                pins = merge(pins, work['pins'])
                guard.verify(pins)
                guard.verify(proof_reuse_v4.archive_correspondence(sys.modules[__name__], archive, retained_release()))
                report.update(status='bounded-output-ownership-work-passed-awaiting-LOC-and-full-review',
                    finished_at=guard.now(), proofs=work, profiles=rows, pins=pins,
                    remaining_gates=proposal['pending_gates'], pins_unchanged_at_end=True,
                    full_manifests_sealed=0, fresh_profile_builds=5, fresh_bounded_output_cases=44)
                guard.save(guard.REPORT, report)
            except BaseException as error:
                report.update(status='stopped-unqualified', finished_at=guard.now(), active=None,
                    error=str(error) or type(error).__name__, error_type=type(error).__name__)
                guard.save(guard.REPORT, report)
                raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Crystal preparation interrupted by signal ' + str(signum))

    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--run', action='store_true')
    parser.add_argument('--proposal-sha256')
    parser.add_argument('--proposal-file', type=proposal_path, default=PROPOSAL)
    parser.add_argument('--revision')
    args = parser.parse_args()
    if args.seal:
        seal(args.proposal_file)
    else:
        assert args.proposal_sha256, 'Exact root-approved proposal SHA required'
        run(args.proposal_sha256, args.revision, args.proposal_file)


if __name__ == '__main__':
    main()
