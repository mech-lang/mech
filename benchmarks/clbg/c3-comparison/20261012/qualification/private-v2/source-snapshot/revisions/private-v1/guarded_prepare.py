"""Candidate additive C3 bootstrap and bounded output/ownership qualification.

All execution, fixtures and pinning happen under the common timing mutex.
This proposal authorizes no canonical full performance run.
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
import shutil
import signal
import subprocess
import sys
import time
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parents[2]
REVISION_SOURCE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
GUARD = SUITE / 'uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
GUARD_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
LOCK = Path('/tmp/mech-clbg-timing.lock')
CLANG = Path('/opt/homebrew/opt/llvm@22/bin/clang-22').resolve()
PCRE = Path('/opt/homebrew/Cellar/pcre2/10.49')
GMP = Path('/opt/homebrew/Cellar/gmp/6.3.0')
LIBRARY_PATH = str(PCRE / 'lib') + ':' + str(GMP / 'lib')
C3_URL = 'https://github.com/c3lang/c3c/releases/download/v0.8.5/c3-macos.zip'
C3_SHA = 'f1f1f9359aba666f06f9a4f3eca339f515336911d278000a64fd91b2ebeee20f'
C3_COMMIT = '640091d3303a393f9651d644063b837fc19d05f1'
C3_TAG = 'v0.8.5'
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
    spec = importlib.util.spec_from_file_location('c3_private_native_guard', GUARD)
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
    return sorted(set(HERE.glob('*.md')) | {HERE / 'profiles.json'} |
                  set((HERE / 'sources').glob('*/*.c3')) | set((HERE / 'support').glob('*.c3')) |
                  set(REVISION_SOURCE.glob('*.py')) | set(REVISION_SOURCE.glob('*.md')))


def proposal_path(value):
    path = Path(value).resolve()
    assert path.is_relative_to(HERE) and path.suffix == '.json'
    return path


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
            cleanup_errors = []
            def record_cleanup(stage, error):
                nonlocal cleanup_error
                cleanup_error = error
                cleanup_errors.append(dict(stage=stage, error=str(error) or type(error).__name__,
                                           error_type=type(error).__name__))
            try:
                harness.kill_group(process.pid)
            except BaseException as error:
                record_cleanup('owned-new-PGID-cleanup', error)
                # A live unreaped Popen leader still owns this new PGID. Never
                # fall back to signalling an inferred, foreign or reaped PID.
                if process.poll() is None:
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    except BaseException as fallback_error:
                        record_cleanup('live-owned-PGID-fallback', fallback_error)
                    if process.poll() is None:
                        try:
                            process.kill()
                        except ProcessLookupError:
                            pass
                        except BaseException as fallback_error:
                            record_cleanup('live-owned-leader-fallback', fallback_error)
            try:
                process.wait(timeout=5)
            except BaseException as error:
                record_cleanup('bounded-owned-leader-reap', error)
                if process.poll() is None:
                    try:
                        process.kill()
                        process.wait(timeout=5)
                    except BaseException as fallback_error:
                        record_cleanup('bounded-owned-leader-final-reap', fallback_error)
            detail['final_exit_status'] = process.returncode
            try:
                complete_stdout, complete_stderr = process.communicate(timeout=5)
                stdout = complete_stdout
                stderr = complete_stderr
            except BaseException as error:
                record_cleanup('bounded-output-drain', error)
                # Keep earlier actual bytes and separately archive any later
                # partial drain bytes; an error cannot replace prior output.
                if isinstance(error, subprocess.TimeoutExpired):
                    (output / (name + '.drain.stdout')).write_bytes(error.output or b'')
                    (output / (name + '.drain.stderr')).write_bytes(error.stderr or b'')
            finally:
                if process.stdout is not None:
                    process.stdout.close()
                if process.stderr is not None:
                    process.stderr.close()
            if cleanup_errors:
                detail['cleanup_errors'] = cleanup_errors
                detail['cleanup_error'] = cleanup_errors[0]['error']
                detail['cleanup_error_type'] = cleanup_errors[0]['error_type']
        (output / (name + '.stdout')).write_bytes(stdout)
        (output / (name + '.stderr')).write_bytes(stderr)
        new_json(receipt, detail)
        if cleanup_error is not None:
            raise RuntimeError('Owned command cleanup failed; diagnostics retained') from cleanup_error
    return dict(stdout=stdout, stderr=stderr, detail=detail)


def download(url, destination):
    assert not destination.exists(), 'Never silently reuse an unreviewed partial download'
    request = urllib.request.Request(url, headers={'User-Agent': 'mech-private-c3-source-audit'})
    with urllib.request.urlopen(request, timeout=60) as response, destination.open('xb') as stream:
        while chunk := response.read(1024 * 1024):
            stream.write(chunk)


def extract(archive, destination):
    destination.mkdir()
    with zipfile.ZipFile(archive) as bundle:
        entries = bundle.infolist()
        names = set()
        symlinks = []
        for entry in entries:
            target = (destination / entry.filename).resolve()
            assert target.is_relative_to(destination.resolve()), 'Unsafe archive member'
            assert str(target) not in names, 'Duplicate archive member'
            names.add(str(target))
            mode = entry.external_attr >> 16
            if (mode & 0o170000) == 0o120000:
                link = bundle.read(entry).decode('utf-8')
                assert not Path(link).is_absolute()
                assert (target.parent / link).resolve().is_relative_to(destination.resolve()), 'Unsafe archive symlink'
                symlinks.append((target, link))
                continue
            if entry.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with target.open('xb') as stream:
                stream.write(bundle.read(entry))
            if mode & 0o111:
                target.chmod(target.stat().st_mode | (mode & 0o111))
        for target, link in symlinks:
            assert not target.exists()
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(link)
        for target, link in symlinks:
            assert target.resolve(strict=True).is_file(), 'Review directory/archive link graph explicitly'


def pin_tree(directory):
    pins = {}
    for path in directory.rglob('*'):
        if path.is_file():
            digest = sha(path)
            pins[str(path)] = digest
            pins[str(path.resolve())] = digest
    return pins


def build_command(compiler, stdlib, sdk, sources, binary, cache):
    cache.mkdir(parents=True)
    arguments = [str(compiler), 'compile', *map(str, sources), '-O3', '--safe=no',
            '--backend=llvm', '--target', 'macos-aarch64', '--fp-math=strict',
            '--threads', '1', '--single-module=yes', '--ansi=no',
            '--stdlib', str(stdlib), '--cc', str(CLANG), '--linker=cc',
            '--macos-sdk', str(sdk), '--macos-min-version', '14.4', '--macos-sdk-version', '14.4',
            '--build-dir', str(cache), '--obj-out', str(cache / 'objects'),
            '--llvm-out', str(cache / 'llvm'), '--asm-out', str(cache / 'asm'),
            '--emit-llvm', '--emit-asm', '--keep-obj', '--print-linking', '--print-input']
    names = {p.name for p in sources}
    if 'gmp.c3' in names:
        arguments += ['-z', str(GMP / 'lib/libgmp.10.dylib')]
    if 'pcre2.c3' in names:
        arguments += ['-z', str(PCRE / 'lib/libpcre2-8.0.dylib')]
    return arguments + ['-o', str(binary)]


def native_images(guard, harness, binary, environment, output, lock_fd, allowed_roots, scored=True):
    architecture = owned_command(guard, harness, ['/usr/bin/lipo', '-archs', str(binary)],
                                 environment, output, lock_fd, 30, 'architecture')['stdout'].decode().split()
    if scored:
        assert architecture == ['arm64'], ('Scored program is not exactly native ARM64', architecture)
        header = owned_command(guard, harness, ['/usr/bin/otool', '-hv', str(binary)],
                               environment, output, lock_fd, 30, 'mach-header')['stdout'].decode()
        assert 'ARM64' in header, 'Measured Mach-O header is not ARM64'
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
        return references.dna_fixtures() + (('header-eof', b'>THREE'), ('empty-input', b''),
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


def preflight_inputs(references):
    """Validate every intended case name before any compiler/native child."""
    inventory = {}
    for benchmark in sorted(set(NUMERIC) | {'regexredux', 'revcomp', 'knucleotide'}):
        if benchmark in NUMERIC:
            assert all(type(n) is int and n >= 0 for n in NUMERIC[benchmark])
            cases = [('n-' + str(n), None) for n in NUMERIC[benchmark]]
        else:
            cases = list(inputs(benchmark, references))
            assert all(type(payload) is bytes for _, payload in cases), 'Stdin fixtures must contain complete byte input'
        labels = [name for name, _ in cases]
        assert labels and all(type(name) is str and name and name.replace('-', '').replace('_', '').isalnum()
                              for name in labels), ('Unsafe/empty case label', benchmark)
        assert len(set(labels)) == len(labels), ('Duplicate fixture labels', benchmark)
        inventory[benchmark] = labels
    assert len(inventory) == 10
    return inventory


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


def seal(proposal_file):
    print('WAIT LOCK: seal C3 source/preparation proposal', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not proposal_file.exists()
        guard = load_guard()
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        import references
        inventory = preflight_inputs(references)
        output = HERE / 'proposal-seals' / proposal_file.stem
        fixture_environment = dict(os.environ, PYTHONPATH=str(REVISION_SOURCE))
        fixture_receipt = output / 'ownership-fixtures-inventory.json'
        owned_command(guard, harness, [sys.executable, '-B', str(REVISION_SOURCE / 'fixture_runner.py'),
                      '--set', 'ownership', '--receipt', str(fixture_receipt)],
                      fixture_environment, output, lock.fileno(), 30, 'ownership-fixtures')
        import fixture_runner
        fixture_runner.validate_receipt(json.loads(fixture_receipt.read_text()), 'ownership')
        sdk_result = owned_command(guard, harness, ['/usr/bin/xcrun', '--show-sdk-path'],
                                  dict(os.environ), output, lock.fileno(), 30, 'sdk-path')
        sdk = Path(sdk_result['stdout'].decode().strip()).resolve(strict=True)
        assert sdk.name == 'MacOSX14.4.sdk'
        required = [CLANG, Path('/usr/bin/xcrun'), Path('/usr/bin/ld'), Path('/usr/bin/lipo'),
                    Path('/usr/bin/otool'), PCRE / 'include/pcre2.h', PCRE / 'lib/libpcre2-8.0.dylib',
                    GMP / 'include/gmp.h', GMP / 'lib/libgmp.10.dylib', GUARD, SUITE / 'run.py',
                    SUITE / 'prepare.py', sdk / 'SDKSettings.json', sdk / 'usr/include/stdio.h',
                    sdk / 'usr/include/pthread.h', sdk / 'usr/lib/libSystem.tbd']
        existing = json.loads((GUARD.parent / 'remaining-owner/manifest.json').read_text())['pins']
        pins = merge(existing, {str(p.resolve()): sha(p) for p in static_paths() + required},
                     {str(p): sha(p) for p in required}, pin_tree(output), pin_tree(sdk / 'usr/include'),
                     pin_tree(sdk / 'usr/lib'), pin_tree(CLANG.parent.parent / 'lib'))
        profiles = json.loads((HERE / 'profiles.json').read_text())['profiles']
        assert len(profiles) == 20 and len({p['id'] for p in profiles}) == 20
        assert {p['benchmark'] for p in profiles} == set(harness.WORK)
        for row in profiles:
            assert row['compiled_source_bundle'] == [row['source'], *row['general_library_support']]
            assert all((HERE / name).is_file() for name in row['compiled_source_bundle'])
            assert 1 <= row['maximum_compute_workers'] <= 8
        proposal = dict(status='sealed-awaiting-root-approval', created_at=guard.now(), pins=pins,
            profiles=profiles, snapshot_paths=[str(p) for p in static_paths()], sdk=str(sdk),
            compiler_archive_url=C3_URL, compiler_archive_sha256=C3_SHA,
            compiler_source_commit=C3_COMMIT, compiler_source_tag=C3_TAG,
            publisher_hash_url='https://github.com/c3lang/c3c/releases/expanded_assets/v0.8.5',
            cached_latest_discrepancy='cached /releases/latest/master said0.8.4; direct0.8.5 tag selected',
            explicit_backend='llvm', explicit_scored_target='macos-aarch64',
            compiler_host_architecture_separate=True, scored_program_required_architecture='arm64',
            case_inventory=inventory, mock_fixture_methods_required=11,
            actual_mock_fixture_receipt=str(fixture_receipt), actual_mock_fixture_receipt_sha256=sha(fixture_receipt),
            shared_timing_lock=str(LOCK), guard_sha256=GUARD_SHA, full_processes_authorized=0,
            full_timings=False, chart_admission=False,
            required_pending_gates=['actual source-work/allocation/retention proof', 'canonical C3 LOC audit',
                                    'independent per-benchmark full manifest review'])
        new_json(proposal_file, proposal)
        print(json.dumps(dict(proposal=str(proposal_file), sha256=sha(proposal_file)), indent=2), flush=True)


def run(digest, revision, proposal_file):
    assert revision and revision.replace('-', '').isalnum()
    print('WAIT LOCK: guarded private C3 preparation', revision, flush=True)
    with (HERE / 'preparation-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert sha(proposal_file) == digest, 'Exact reviewed C3 proposal changed'
            proposal = json.loads(proposal_file.read_text())
            guard = load_guard()
            guard.verify(proposal['pins'])
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            import references
            assert preflight_inputs(references) == proposal['case_inventory']
            output = HERE / 'qualification' / revision
            assert not output.exists(), 'Never implicitly repeat or resume a revision'
            output.mkdir(parents=True)
            guard.OUT = output
            guard.REPORT = output / 'report.json'
            report = dict(status='preparing', started_at=guard.now(), proposal_sha256=digest,
                          attempts=[], active=None, full_performance_runs=0, chart_admission=False,
                          never_chart_eligible=True, no_automatic_retry=True)
            guard.save(guard.REPORT, report)
            try:
                snapshot = output / 'source-snapshot'
                snapshot.mkdir()
                for filename in proposal['snapshot_paths']:
                    source = Path(filename)
                    destination = snapshot / source.relative_to(HERE)
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, destination)
                    assert sha(destination) == proposal['pins'][filename]
                build = HERE / '.build' / revision
                assert not build.exists()
                build.mkdir(parents=True)
                archive = build / 'c3-macos.zip'
                download(C3_URL, archive)
                assert sha(archive) == C3_SHA, 'Official publisher archive digest mismatch'
                extract(archive, build / 'c3')
                compilers = [p for p in (build / 'c3').rglob('c3c') if p.is_file()]
                assert len(compilers) == 1, 'Ambiguous official compiler layout'
                compiler = compilers[0]
                stdlibs = [p.parents[2] for p in (build / 'c3').rglob('libc.c3')
                           if p.parent.name == 'libc' and p.parent.parent.name == 'std']
                assert len(stdlibs) == 1, 'Ambiguous official standard library layout'
                stdlib = stdlibs[0]
                assert (stdlib / 'std/libc/libc.c3').is_file(), 'Official library root must contain std/'
                sdk = Path(proposal['sdk'])
                environment = dict(PATH=str(CLANG.parent) + ':/usr/bin:/bin:/usr/sbin:/sbin', LC_ALL='C', SDKROOT=str(sdk), DYLD_LIBRARY_PATH=LIBRARY_PATH,
                                   OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
                for key in ('C3FLAGS','C3_LIB','CFLAGS','CPPFLAGS','LDFLAGS','DYLD_INSERT_LIBRARIES',
                            'DYLD_FRAMEWORK_PATH','DYLD_FALLBACK_LIBRARY_PATH'):
                    environment.pop(key, None)
                version = owned_command(guard, harness, [str(compiler), '--version'], environment,
                                        output / 'bootstrap', lock.fileno(), 30, 'c3-version')['stdout'].decode()
                assert re.search(r'^C3 Compiler Version:\s+0\.8\.5\s*$', version, re.M), version
                git_hash = re.search(r'^Git Hash:\s+(\w+)', version, re.M)
                assert git_hash and len(git_hash[1]) >= 8 and C3_COMMIT.startswith(git_hash[1]), version
                llvm = re.search(r'^LLVM version:\s+(\d+\.\d+(?:\.\d+)?)', version, re.M)
                assert llvm and re.search(r'^Backends:\s+.*LLVM', version, re.M), version
                pins = merge(proposal['pins'], {str(proposal_file): digest, str(archive): sha(archive)},
                             pin_tree(build / 'c3'), pin_tree(snapshot))
                pins = merge(pins, native_images(guard, harness, compiler, environment,
                    output / 'bootstrap/images', lock.fileno(), [build / 'c3', GMP, PCRE, CLANG.parent.parent], scored=False))
                new_json(output / 'bootstrap/actual-toolchain.json', dict(compiler=str(compiler),
                    compiler_sha256=sha(compiler), compiler_version=version, bundled_llvm_version=llvm[1],
                    explicit_backend='llvm', explicit_target='macos-aarch64', linker=str(CLANG),
                    sdk=str(sdk), compiler_host_image_review=str(output / 'bootstrap/images/dependency-review.json')))
                binaries = build / 'bin'
                binaries.mkdir()
                rows = []
                for declared in proposal['profiles']:
                    sources = [snapshot / name for name in declared['compiled_source_bundle']]
                    binary = binaries / declared['id']
                    cache = build / 'cache' / declared['id']
                    command = build_command(compiler, stdlib, sdk, sources, binary, cache)
                    owned_command(guard, harness, command, environment, output / 'builds' / declared['id'],
                                  lock.fileno(), 180, 'build')
                    pins = merge(pins, pin_tree(cache), {str(binary): sha(binary)})
                    pins = merge(pins, native_images(guard, harness, binary, environment,
                        output / 'builds' / declared['id'] / 'images', lock.fileno(),
                        [build / 'c3', GMP, PCRE, CLANG.parent.parent]))
                    llvm_files = list((cache / 'llvm').rglob('*.ll'))
                    assert llvm_files, 'No retained actual LLVM IR'
                    assert any(re.search(r'target triple = "(?:arm64|aarch64)-apple-macos', p.read_text())
                               for p in llvm_files), 'Actual IR target is not native macOS ARM64'
                    row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='c3',
                        source=str(sources[0]), source_sha256=sha(sources[0]), binary_sha256=sha(binary),
                        source_bundle=[dict(path=str(p), sha256=sha(p)) for p in sources],
                        support={str(p): sha(p) for p in sources[1:]},
                        command=[str(binary), '{n}'], cwd=str(HERE), runtime_environment={
                            'LC_ALL':'C','DYLD_LIBRARY_PATH':LIBRARY_PATH,
                            'OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1','VECLIB_MAXIMUM_THREADS':'1'},
                        compiler_version='0.8.5', bundled_llvm_version=llvm[1], explicit_backend='llvm',
                        explicit_target='macos-aarch64', build_command=command, cache=str(cache),
                        measured_architecture='arm64')
                    rows.append(row)
                    benchmark = row['benchmark']
                    for n in NUMERIC.get(benchmark, ()):
                        probe(guard, harness, row, n, None, getattr(references, benchmark)(n),
                              pins, output, lock.fileno(), report, 'n-' + str(n))
                    if benchmark not in NUMERIC:
                        for case, data in inputs(benchmark, references):
                            probe(guard, harness, row, harness.WORK[benchmark][0], data,
                                  getattr(references, benchmark)(data), pins, output, lock.fileno(), report, case)
                guard.verify(pins)
                report.update(status='bounded-output-ownership-passed-awaiting-work-and-LOC-review',
                    finished_at=guard.now(), pins=pins, profiles=rows, compiler=str(compiler), stdlib=str(stdlib),
                    runtime_environment=environment, build=str(build), sdk=str(sdk),
                    remaining_gates=proposal['required_pending_gates'], full_manifests_sealed=0)
                guard.save(guard.REPORT, report)
            except BaseException as error:
                report.update(status='stopped-unqualified', finished_at=guard.now(),
                              error=str(error) or type(error).__name__, error_type=type(error).__name__)
                guard.save(guard.REPORT, report)
                raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Preparation interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--run', action='store_true')
    parser.add_argument('--proposal-file', required=True, type=proposal_path)
    parser.add_argument('--proposal-sha256')
    parser.add_argument('--revision')
    args = parser.parse_args()
    if args.seal:
        seal(args.proposal_file)
    else:
        assert args.proposal_sha256 and args.revision
        run(args.proposal_sha256, args.revision, args.proposal_file)


if __name__ == '__main__':
    main()
