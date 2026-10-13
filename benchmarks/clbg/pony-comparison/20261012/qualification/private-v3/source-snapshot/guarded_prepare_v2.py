"""Additive Pony package-alias bootstrap with exact prior-binary proof reuse.

Root must review the sealed proposal SHA before --run. Full timings and full
benchmark manifests are deliberately absent; work/LOC review remains a gate.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tarfile
import time
import urllib.request

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
GUARD = SUITE / 'uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
GUARD_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PROPOSAL = HERE / 'proposal.json'
PCRE = Path('/opt/homebrew/Cellar/pcre2/10.49')
GMP = Path('/opt/homebrew/Cellar/gmp/6.3.0')
LIBRARY_PATH = str(PCRE / 'lib') + ':' + str(GMP / 'lib')
PONY_URL = 'https://github.com/ponylang/ponyc/releases/download/0.74.0/ponyc-arm64-apple-darwin.tar.gz'
PONY_SHA = '02a20dce1c283da67966a57c58093e87981eb896e0cddefdcafe949ff17870ad'
REGEX_URL = 'https://codeload.github.com/ponylang/regex/tar.gz/589eb008db250aab17d4c073a41c5562ac5b8dc7'
GMP_WRAPPER_URL = 'https://codeload.github.com/tadashi9e/gmp4pony/tar.gz/b14db4ba347f6a2b840f135e1602e19f85625c39'
NUMERIC = {
    'nbody': (0, 1, 2, 10000),
    'mandelbrot': (1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 65, 129, 200),
    'spectralnorm': (1, 2, 3, 5, 9, 100),
    'binarytrees': (0, 4, 7, 9),
    'fannkuchredux': (1, 2, 3, 4, 5, 6, 7, 8),
    'pidigits': (0, 1, 9, 10, 11, 20, 21, 27, 97),
    'fasta': (0, 1, 2, 29, 30, 31, 60, 61, 287, 574, 20000),
}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def load_guard():
    assert sha(GUARD) == GUARD_SHA, 'Sealed common native guard changed'
    spec = importlib.util.spec_from_file_location('pony_private_native_guard', GUARD)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def merge(*groups):
    pins = {}
    for group in groups:
        for filename, digest in group.items():
            assert filename not in pins or pins[filename] == digest, ('Conflicting pin', filename)
            pins[filename] = digest
    return pins


def static_paths():
    return sorted(set(HERE.glob('*.py')) | set(HERE.glob('*.md')) | {HERE / 'profiles.json'} |
                  set((HERE / 'sources').glob('*/*/*.pony')) | set((HERE / 'support').glob('*/*.pony')) |
                  set((HERE / 'support-v2').glob('*/*.pony')))


def proposal_path(value):
    path = Path(value).resolve()
    assert path.is_relative_to(HERE) and path.suffix == '.json', 'Proposal must be an additive cohort JSON file'
    return path


def seal(proposal_file):
    print('WAIT LOCK: seal Pony source/preparation proposal', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not proposal_file.exists(), 'Never replace an existing reviewed proposal'
        guard = load_guard()
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        seal_output = HERE / 'proposal-seals' / proposal_file.stem
        owned_command(guard, harness, [sys.executable, '-B', '-m', 'unittest', '-v', 'test_guarded_prepare_v2', 'test_proof_reuse_v2'],
                      dict(os.environ), seal_output, lock.fileno(), 30, 'ownership-fixtures')
        sdk_result = owned_command(guard, harness, ['/usr/bin/xcrun', '--show-sdk-path'],
                                   dict(os.environ), seal_output, lock.fileno(), 30, 'sdk-path')
        sdk = Path(sdk_result['stdout'].decode().strip()).resolve(strict=True)
        assert sdk.name == 'MacOSX14.4.sdk' and sdk.is_relative_to(Path('/Library/Developer/CommandLineTools/SDKs'))
        required = [Path('/usr/bin/xcrun'), Path('/usr/bin/otool'), Path('/usr/bin/lipo'),
                    PCRE / 'include/pcre2.h', PCRE / 'lib/libpcre2-8.0.dylib',
                    GMP / 'include/gmp.h', GMP / 'lib/libgmp.10.dylib',
                    Path('/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'),
                    Path('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib'),
                    GUARD, SUITE / 'run.py', SUITE / 'prepare.py', sdk / 'SDKSettings.json',
                    sdk / 'usr/include/stdio.h', sdk / 'usr/include/pthread.h',
                    sdk / 'usr/lib/libSystem.tbd']
        existing = json.loads((GUARD.parent / 'remaining-owner/manifest.json').read_text())['pins']
        pins = merge(existing, {str(path.resolve()): sha(path) for path in static_paths() + required},
                     {str(path): sha(path) for path in required},
                     pin_tree(sdk / 'usr/include'), pin_tree(sdk / 'usr/lib'))
        profiles = json.loads((HERE / 'profiles.json').read_text())['profiles']
        assert len(profiles) == 20 and len({row['id'] for row in profiles}) == 20
        assert {row['benchmark'] for row in profiles} == set(harness.WORK)
        assert all((HERE / row['source']).is_file() and 1 <= row['maximum_compute_actors'] <= 8 for row in profiles)
        import proof_reuse_v2
        reuse = proof_reuse_v2.reconstruct(sys.modules[__name__], guard)
        reuse_path = seal_output / 'prior-proof-reuse.json'
        new_json(reuse_path, reuse)
        pins = merge(pins, reuse['pins'], {str(reuse_path): sha(reuse_path)})
        proposal = dict(prior_proof_reuse=str(reuse_path), prior_proof_reuse_sha256=sha(reuse_path),
                        reused_profile_ids=sorted(reuse['profiles']), reused_bounded_cases=20,
                        fresh_profile_builds_required=16,
                        compatibility_package=str(HERE / 'support-v2/ponytest'),
                        compatibility_scope='four ordinary current PonyTest type aliases; no workload code',
                        snapshot_paths=[str(path) for path in static_paths()],
                        status='sealed-awaiting-root-approval', created_at=guard.now(), pins=pins,
                        profiles=profiles,
                        sdk=str(sdk), compiler_archive_url=PONY_URL, compiler_archive_sha256=PONY_SHA,
                        compiler_source_commit='95ad18015d04f4414d0fd4f469ed6b2e9b9b9e73',
                        regex_source_commit='589eb008db250aab17d4c073a41c5562ac5b8dc7',
                        gmp_wrapper_source_commit='b14db4ba347f6a2b840f135e1602e19f85625c39',
                        shared_timing_lock=str(LOCK), guard_sha256=GUARD_SHA,
                        full_processes_authorized=0, full_timings=False, chart_admission=False,
                        required_pending_gates=['actual source-work and actor GC/finalizer traces', 'actual scheduler/native worker gate', 'canonical LOC audit',
                                                'independent per-benchmark manifest review'])
        new_json(proposal_file, proposal)
        print(json.dumps(dict(proposal=str(proposal_file), sha256=sha(proposal_file)), indent=2), flush=True)


def owned_command(guard, harness, command, environment, output, lock_fd, timeout, name):
    """Own only this Popen's new PGID, starting before identity/journal setup."""
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / (name + '.json')
    assert not receipt.exists(), 'Never repeat an attempt or overwrite its journal'
    detail = dict(command=command, started_at=guard.now(), inherited_common_mutex=True,
                  timeout_seconds=timeout, preparation_only=True, never_chart_eligible=True)
    process = None
    stdout = b''
    stderr = b''
    cleanup_error = None
    try:
        process = subprocess.Popen(command, cwd=HERE, env=environment, stdin=subprocess.DEVNULL,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   start_new_session=True, pass_fds=(lock_fd,))
        detail['pid'] = process.pid
        detail['identity'] = guard.identity(process.pid)
        if detail['identity'] is None and process.poll() is None:
            raise RuntimeError('Owned child identity missing while still live')
        guard.save(output / (name + '.active.json'), detail)
        stdout, stderr = process.communicate(timeout=timeout)
        detail.update(exit_status=process.returncode, finished_at=guard.now())
        assert process.returncode == 0, ('Owned command failed', name, process.returncode)
        assert not harness.group_members(process.pid), 'Owned leader exited with a live compiler child'
    except BaseException as error:
        detail.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        if isinstance(error, subprocess.TimeoutExpired):
            stdout = error.output or b''
            stderr = error.stderr or b''
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
                    complete_stdout, complete_stderr = process.communicate(timeout=5)
                    stdout = complete_stdout
                    stderr = complete_stderr
                except BaseException as error:
                    cleanup_error = error
                    detail.update(output_drain_error=str(error), output_drain_error_type=type(error).__name__)
                finally:
                    if process.stdout is not None:
                        process.stdout.close()
                    if process.stderr is not None:
                        process.stderr.close()
        (output / (name + '.stdout')).write_bytes(stdout)
        (output / (name + '.stderr')).write_bytes(stderr)
        new_json(receipt, detail)
        if cleanup_error is not None:
            raise RuntimeError('Owned command cleanup failed; diagnostics retained') from cleanup_error
    return dict(stdout=stdout, stderr=stderr, detail=detail)


def download(url, destination):
    assert not destination.exists(), 'Never silently reuse an unreviewed partial download'
    request = urllib.request.Request(url, headers={'User-Agent': 'mech-private-pony-source-audit'})
    with urllib.request.urlopen(request, timeout=60) as response, destination.open('xb') as stream:
        while chunk := response.read(1024 * 1024):
            stream.write(chunk)


def extract(archive, destination):
    destination.mkdir()
    with tarfile.open(archive) as bundle:
        bundle.extractall(destination, filter='data')


def pin_tree(directory):
    return {str(path.resolve()): sha(path) for path in directory.rglob('*') if path.is_file()}


def build_command(compiler, sdk, regex, gmp_wrapper, source, destination, name):
    # Pony 0.74 ships LLVM/LLD. Mach-O linking discovers the SDK via xcrun;
    # place the exact reviewed SDK/library roots ahead of automatic fallbacks.
    snapshot = next((parent for parent in source.parents if parent.name == 'source-snapshot'), None)
    assert snapshot is not None, 'Builds must use the reviewed immutable source snapshot'
    alias = snapshot / 'support-v2'
    assert (alias / 'ponytest/aliases.pony').is_file(), 'Pinned private PonyTest alias package is required'
    return [str(compiler), '--strip', '--triple', 'aarch64-apple-macosx14.4.0',
            '--sysroot', str(sdk), '--lib-path', str(sdk / 'usr/lib'),
            '--lib-path', str(PCRE / 'lib'), '--lib-path', str(GMP / 'lib'),
            '--path', str(regex), '--path', str(gmp_wrapper), '--path', str(alias),
            '--output', str(destination), '--bin-name', name, str(source.parent)]


def native_images(guard, harness, binary, environment, output, lock_fd, allowed_roots):
    architecture = owned_command(guard, harness, ['/usr/bin/lipo', '-archs', str(binary)],
                                 environment, output, lock_fd, 30, 'architecture')['stdout'].decode().split()
    assert 'arm64' in architecture, 'Compiler/executable lacks a native arm64 slice'
    payload = owned_command(guard, harness, ['/usr/bin/otool', '-L', str(binary)],
                            environment, output, lock_fd, 30, 'linked-images')['stdout'].decode()
    dependencies = []
    pins = {}
    for line in payload.splitlines()[1:]:
        install = line.strip().split(' (', 1)[0]
        if install.startswith(('/usr/lib/', '/System/Library/')):
            dependencies.append(dict(install_name=install, scope='OS-image-not-hashed'))
            continue
        if install.startswith(('@loader_path/', '@executable_path/')):
            lookup = binary.parent / install.split('/', 1)[1]
        elif install.startswith('@rpath/'):
            suffix = install[len('@rpath/'):]
            candidates = [path for root in allowed_roots for path in root.rglob(suffix) if path.is_file()]
            assert len({path.resolve() for path in candidates}) == 1, ('Ambiguous/unresolved runtime image', install)
            lookup = candidates[0]
        else:
            lookup = Path(install)
        target = lookup.resolve(strict=True)
        assert any(target.is_relative_to(root.resolve()) for root in allowed_roots), ('Unreviewed runtime image', install)
        digest = sha(target)
        dependencies.append(dict(install_name=install, lookup_path=str(lookup), path=str(target), sha256=digest))
        pins[str(lookup)] = digest
        pins[str(target)] = digest
    new_json(output / 'dependency-review.json', dict(binary=str(binary), architectures=architecture, dependencies=dependencies))
    return pins


def inputs(benchmark, references):
    if benchmark == 'knucleotide':
        return references.dna_fixtures() + (('header-eof', b'>THREE'), ('empty', b''),
                ('exact-token-cr', b'>THREEBAD\rAAAA\r;>THREE\r>THREE\tselected\rGGTATTTTAATTTATAGT'),
                ('exact-token-controls', b'>THREE\vwrong\nAAAA\n>THREE\fwrong\nAAAA\n>THREE selected\nACGT'),
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


def probe(guard, harness, row, n, data, expected, pins, output, lock_fd, report, case, trace=None):
    benchmark = row['benchmark']
    previous = harness.WORK[benchmark]
    old_input = harness.INPUTS.get(benchmark)
    input_was_present = benchmark in harness.INPUTS
    old_validate = harness.validate
    directory = output / row['id'] / case
    directory.mkdir(parents=True)
    reference = dict(bytes=len(expected), sha256=hashlib.sha256(expected).hexdigest())

    def validate(actual_benchmark, full, stream):
        assert actual_benchmark == benchmark and full is False
        stream.seek(0)
        actual = stream.read()
        if benchmark == 'nbody':
            harness.compact.common.validate(actual.decode(), [float(value) for value in expected.splitlines()])
        else:
            assert actual == expected, ('Complete bounded output differs', row['id'], case, actual[:160])
        (directory / 'actual.stdout').write_bytes(actual)
        return dict(reference_match=True, bytes=len(actual), sha256=hashlib.sha256(actual).hexdigest())

    try:
        (directory / 'expected.stdout').write_bytes(expected)
        pins[str(directory / 'expected.stdout')] = sha(directory / 'expected.stdout')
        if data is not None:
            filename = directory / 'stdin.fasta'
            filename.write_bytes(data)
            assert old_input is not None, 'Only declared stdin tasks accept input overrides'
            harness.INPUTS[benchmark] = (filename, old_input[1])
            pins[str(filename)] = sha(filename)
        harness.WORK[benchmark] = (n, previous[1])
        harness.validate = validate
        manifest = dict(size=n, canonical_full_workload=False, timeout_seconds=120,
                        maximum_rss_mib=10240, pins=pins,
                        full_output_reference=None if benchmark == 'nbody' else reference)
        try:
            attempt = guard.one_attempt(harness, row, manifest, report, 0, lock_fd)
        except BaseException as error:
            # Keep actual sampler bytes intact even if a parent journal/pin/event
            # operation raises after execution. Never invent a duration or exit.
            index = len(report['attempts']) + 1
            raw_path = guard.OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
            enriched_path = raw_path.with_suffix('.supervisor.json')
            actual_path = enriched_path if enriched_path.exists() else raw_path
            attempt = json.loads(actual_path.read_text()) if actual_path.exists() else {}
            if attempt.get('error'):
                attempt['execution_failure_before_preparation_error'] = dict(
                    error=attempt['error'], error_type=attempt.get('error_type'))
            attempt.update(qualified=False, preparation_supervisor_failure=True,
                           preparation_error=str(error) or type(error).__name__,
                           preparation_error_type=type(error).__name__,
                           case=case, id=row['id'], benchmark=benchmark,
                           preparation_only=True, never_chart_eligible=True,
                           full_performance_run=False)
            if actual_path.exists():
                attempt['actual_execution_result_path'] = str(actual_path)
                attempt['actual_execution_result_sha256'] = sha(actual_path)
            wrapper_path = raw_path.with_suffix('.preparation.json')
            attempt['preparation_result_path'] = str(wrapper_path)
            new_json(wrapper_path, attempt)
            report['attempts'].append(attempt)
            report['active'] = None
            guard.save(guard.REPORT, report)
            raise
        attempt.update(case=case, preparation_only=True, never_chart_eligible=True, full_performance_run=False)
        if trace is not None and attempt['qualified']:
            try:
                attempt['source_work_proof'] = trace(directory)
            except BaseException as error:
                attempt.update(output_qualified_before_source_work=True, qualified=False,
                               source_work_error=str(error), source_work_error_type=type(error).__name__)
        report['attempts'].append(attempt)
        report['active'] = None
        guard.save(guard.REPORT, report)
        assert attempt['qualified'], ('Unqualified attempt retained; no automatic retry', row['id'], case)
        return attempt
    finally:
        harness.WORK[benchmark] = previous
        harness.validate = old_validate
        if input_was_present:
            harness.INPUTS[benchmark] = old_input
        else:
            harness.INPUTS.pop(benchmark, None)


def run(digest, revision, proposal_file):
    assert revision and revision.replace('-', '').isalnum(), 'Use an explicit fresh revision name'
    print('WAIT LOCK: guarded private Pony preparation', revision, flush=True)
    with (HERE / 'preparation-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert sha(proposal_file) == digest, 'Exact root-reviewed proposal changed'
            proposal = json.loads(proposal_file.read_text())
            guard = load_guard()
            guard.verify(proposal['pins'])
            proposal['pins'] = merge(proposal['pins'], {str(proposal_file): digest})
            import proof_reuse_v2
            reuse = json.loads(Path(proposal['prior_proof_reuse']).read_text())
            assert sha(proposal['prior_proof_reuse']) == proposal['prior_proof_reuse_sha256']
            assert set(reuse['profiles']) == set(proposal['reused_profile_ids']) == proof_reuse_v2.PROFILES
            guard.verify(reuse['pins'])
            proof_reuse_v2.validate_records(json.loads(Path(reuse['prior_report']).read_text()))
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            output = HERE / 'qualification' / revision
            assert not output.exists(), 'Failed/completed revisions are immutable; never implicitly resume'
            output.mkdir(parents=True)
            guard.OUT = output
            guard.REPORT = output / 'report.json'
            report = dict(status='preparing', started_at=guard.now(), proposal_sha256=digest,
                          attempts=[], active=None, full_performance_runs=0, chart_admission=False,
                          never_chart_eligible=True, no_automatic_retry=True,
                          prior_bounded_proof_reuse=proposal['prior_proof_reuse'],
                          prior_bounded_proof_reuse_sha256=proposal['prior_proof_reuse_sha256'],
                          reused_bounded_cases=20, reused_cases_relabelled_as_attempts=False)
            guard.save(guard.REPORT, report)
            try:
                snapshot = output / 'source-snapshot'
                snapshot.mkdir()
                for filename in proposal['snapshot_paths']:
                    source = Path(filename)
                    if source.is_relative_to(HERE) and source != proposal_file:
                        destination = snapshot / source.relative_to(HERE)
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(source, destination)
                        assert sha(destination) == proposal['pins'][filename]
                build = HERE / '.build' / revision
                assert not build.exists(), 'Never overwrite any prior build revision'
                build.mkdir(parents=True)
                archive = build / 'ponyc-arm64-apple-darwin.tar.gz'
                download(PONY_URL, archive)
                assert sha(archive) == PONY_SHA, 'Official native compiler archive digest differs'
                extract(archive, build / 'release')
                compilers = [path for path in (build / 'release').rglob('bin/ponyc') if path.is_file()]
                assert len(compilers) == 1, 'Release compiler is ambiguous'
                compiler = compilers[0]
                regex_archive = build / 'regex-source.tar.gz'
                download(REGEX_URL, regex_archive)
                extract(regex_archive, build / 'regex')
                regex_packages = list((build / 'regex').glob('*/regex/regex.pony'))
                assert len(regex_packages) == 1
                regex = regex_packages[0].parents[1]
                gmp_archive = build / 'gmp4pony-source.tar.gz'
                download(GMP_WRAPPER_URL, gmp_archive)
                extract(gmp_archive, build / 'gmp4pony')
                gmp_packages = list((build / 'gmp4pony').glob('*/gmp/mpz.pony'))
                assert len(gmp_packages) == 1
                gmp_wrapper = gmp_packages[0].parents[1]
                sdk = Path(proposal['sdk'])
                private_libraries = sorted({path.parent.resolve() for path in (build / 'release').rglob('*.dylib')})
                runtime_environment = dict(LC_ALL='C',
                    DYLD_LIBRARY_PATH=os.pathsep.join([str(PCRE / 'lib'), str(GMP / 'lib')] + [str(path) for path in private_libraries]),
                    OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
                environment = dict(os.environ, SDKROOT=str(sdk), PATH='/usr/bin:/bin', **runtime_environment)
                for key in ('PONY_PATH', 'PONYPATH', 'PONYC_OPTIONS', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS',
                            'DYLD_INSERT_LIBRARIES', 'DYLD_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH'):
                    environment.pop(key, None)
                version = owned_command(guard, harness, [str(compiler), '--version'], environment,
                                        output / 'bootstrap', lock.fileno(), 30, 'ponyc-version')['stdout'].decode()
                assert '0.74.0' in version, version
                pins = merge(proposal['pins'], pin_tree(build / 'release'), pin_tree(build / 'regex'),
                             pin_tree(build / 'gmp4pony'), pin_tree(snapshot),
                             {str(archive): sha(archive), str(regex_archive): sha(regex_archive), str(gmp_archive): sha(gmp_archive)},
                             native_images(guard, harness, compiler, environment, output / 'bootstrap/images',
                                           lock.fileno(), [build / 'release', GMP, PCRE]))
                import references
                import source_proofs
                binaries = build / 'bin'
                binaries.mkdir()
                rows = []
                fresh_builds = 0
                for declared in proposal['profiles']:
                    if declared['id'] in reuse['profiles']:
                        prior = reuse['profiles'][declared['id']]
                        assert prior['declaration'] == declared
                        rows.append(proof_reuse_v2.profile_row(prior))
                        continue
                    fresh_builds += 1
                    source = snapshot / declared['source']
                    destination = binaries / declared['id']
                    destination.mkdir()
                    binary = destination / declared['id']
                    command = build_command(compiler, sdk, regex, gmp_wrapper, source, destination, declared['id'])
                    owned_command(guard, harness, command, environment, output / 'builds' / declared['id'],
                                  lock.fileno(), 600, 'build')
                    pins = merge(pins, pin_tree(destination), native_images(guard, harness, binary, environment,
                        output / 'builds' / declared['id'] / 'images', lock.fileno(), [build / 'release', GMP, PCRE]))
                    guard.verify(pins)
                    row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='pony',
                               source=str(source), source_sha256=sha(source), binary_sha256=sha(binary),
                               command=[str(binary), '{n}', '--ponymaxthreads', '8'], cwd=str(HERE),
                               runtime_environment=dict(runtime_environment))
                    rows.append(row)
                    benchmark = row['benchmark']
                    for n in NUMERIC.get(benchmark, ()):
                        expected = getattr(references, benchmark)(n)
                        probe(guard, harness, row, n, None, expected, pins, output, lock.fileno(), report, 'n-' + str(n))
                    if benchmark not in NUMERIC:
                        for case, data in inputs(benchmark, references):
                            probe(guard, harness, row, harness.WORK[benchmark][0], data,
                                  getattr(references, benchmark)(data), pins, output, lock.fileno(), report, case)

                assert fresh_builds == proposal['fresh_profile_builds_required'] == 16
                proofs = source_proofs.run(sys.modules[__name__], guard, harness, proposal, snapshot, build, compiler,
                                          regex, gmp_wrapper, sdk, environment, runtime_environment, pins, lock.fileno(), report)
                guard.verify(pins)
                report.update(status='bounded-output-primitives-passed-awaiting-work-worker-and-LOC-review',
                              finished_at=guard.now(), proofs=proofs, profiles=rows, pins=pins,
                              fresh_profile_builds=fresh_builds,
                              remaining_gates=proposal['required_pending_gates'], full_manifests_sealed=0)
                guard.save(guard.REPORT, report)
            except BaseException as error:
                report.update(status='stopped-unqualified', finished_at=guard.now(),
                              error=str(error) or type(error).__name__, error_type=type(error).__name__)
                guard.save(guard.REPORT, report)
                raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Pony preparation interrupted by signal ' + str(signum))

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
        assert args.proposal_sha256, 'Root-approved exact proposal SHA required'
        run(args.proposal_sha256, args.revision, args.proposal_file)


if __name__ == '__main__':
    main()
