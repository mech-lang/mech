"""Additive private Nim bootstrap and bounded output/ownership qualification.

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
PROPOSAL = HERE / 'proposals/private-v6.json'
CLANG = Path('/opt/homebrew/opt/llvm@22/bin/clang-22').resolve()
PCRE = Path('/opt/homebrew/Cellar/pcre/8.45')
GMP = Path('/opt/homebrew/Cellar/gmp/6.3.0')
LIBRARY_PATH = str(PCRE / 'lib') + ':' + str(GMP / 'lib')
NIM_URL = 'https://github.com/nim-lang/nightlies/releases/download/2026-09-08-version-2-2-8e8fbf60693418dc95bb0d762fd660231d08a583/nim-2.2.12-macosx_arm64.tar.xz'
NIM_SHA = '130a43eff76c1c3cc78b89329a7efd892c1f65839dbc6d8e53651f3f77516fe5'
BIGINT_URL = 'https://codeload.github.com/nim-lang/bigints/tar.gz/3b5a106b88ec1baa904a059a1430c8335d885075'
GMP_WRAPPER_URL = 'https://codeload.github.com/FedeOmoto/nim-gmp/tar.gz/4e9a6a05bfc274abd02a241f2d7c9162b2300191'
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
    spec = importlib.util.spec_from_file_location('nim_private_native_guard', GUARD)
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
    # Frozen v5 declared inputs plus this explicit additive repair; never a
    # mutable root glob that can silently absorb another controller revision.
    prior = json.loads((HERE / 'proposals/private-v5.json').read_text())
    additions = ('guarded_prepare_v6.py', 'source_proofs_v6.py', 'primitive_reuse_v6.py',
                 'test_guarded_prepare_v6.py', 'test_primitive_reuse_v6.py',
                 'fixture_runner_v6.py', 'REVISION-V6.md')
    return sorted({Path(p) for p in prior['snapshot_paths']} | {HERE / p for p in additions})


def proposal_path(value):
    path = Path(value).resolve()
    assert path.is_relative_to(HERE) and path.suffix == '.json', 'Proposal must be an additive cohort JSON file'
    return path


def seal(proposal_file):
    print('WAIT LOCK: seal additive Nim diagnostic/reuse v6 proposal', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not proposal_file.exists(), 'Never replace an existing reviewed proposal'
        guard = load_guard()
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        import primitive_reuse_v6 as primitive
        import proof_reuse_v5
        import references
        import fixture_runner_v6
        prior_path = HERE / 'proposals/private-v5.json'
        assert sha(prior_path) == primitive.PRIOR_SHA
        prior = json.loads(prior_path.read_text())
        guard.verify(prior['pins'])
        reuse_path = Path(prior['prior_proof_reuse'])
        assert sha(reuse_path) == prior['prior_proof_reuse_sha256']
        production = json.loads(reuse_path.read_text())
        proof_reuse_v5.validate_prior_reports(production)
        guard.verify(production['pins'])
        assert set(production['profiles']) == proof_reuse_v5.PROFILES and production['reused_bounded_cases'] == 154
        assert not production['pending_reused_profile_cases']
        output = HERE / 'proposal-seals' / proposal_file.stem
        receipt = output / 'ownership-fixtures-inventory.json'
        owned_command(guard,harness,[sys.executable,'-B',str(HERE / 'fixture_runner_v6.py'),
            '--set','ownership','--receipt',str(receipt)],dict(os.environ),output,lock.fileno(),30,'ownership-fixtures')
        fixture_runner_v6.validate_receipt(json.loads(receipt.read_text()),'ownership')
        reused = primitive.reconstruct(sys.modules[__name__],guard,harness)
        primitive_path = output / 'prior-primitive-reuse.json'
        new_json(primitive_path,reused)
        case_inventory = preflight_inputs(references)
        assert case_inventory == prior['case_inventory']
        pins = merge(prior['pins'], production['pins'], reused['pins'],
            {str(p):sha(p) for p in static_paths()}, {str(prior_path):primitive.PRIOR_SHA}, pin_tree(output))
        proposal = dict(prior, status='sealed-awaiting-root-approval',created_at=guard.now(),pins=pins,
            snapshot_paths=[str(p) for p in static_paths()],
            prior_primitive_reuse=str(primitive_path),prior_primitive_reuse_sha256=sha(primitive_path),
            reused_primitive_ids=primitive.IDS,reused_primitive_cases=3,
            fresh_source_proof_attempts_required=21, fresh_profile_builds_required=0,
            prior_failed_diagnostic_retained=reused['failed_diagnostic'],
            actual_mock_fixtures=dict(receipt=str(receipt),sha256=sha(receipt),methods=23,zero_skips=True),
            mock_fixture_methods_required=23,toolchain_binding=reused['toolchain'],
            correction='Unscored ORC diagnostic local distinct renamed threadIds only; bounded owned-child exceptional cleanup',
            full_processes_authorized=0,full_timings=False,chart_admission=False)
        new_json(proposal_file,proposal)
        print(json.dumps(dict(proposal=str(proposal_file),sha256=sha(proposal_file)),indent=2),flush=True)


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
    request = urllib.request.Request(url, headers={'User-Agent': 'mech-private-nim-source-audit'})
    with urllib.request.urlopen(request, timeout=60) as response, destination.open('xb') as stream:
        while chunk := response.read(1024 * 1024):
            stream.write(chunk)


def extract(archive, destination):
    destination.mkdir()
    with tarfile.open(archive) as bundle:
        bundle.extractall(destination, filter='data')


def pin_tree(directory):
    return {str(path.resolve()): sha(path) for path in directory.rglob('*') if path.is_file()}


def build_command(compiler, toolchain, bigint, gmp_wrapper, sdk, source, binary, cache):
    return [str(compiler), 'c', '--skipUserCfg:on', '--skipParentCfg:on', '--skipProjCfg:on',
            '-d:release', '--mm:orc', '--threads:on', '--parallelBuild:1', '--cc:clang',
            '--clang.exe:' + str(CLANG), '--clang.linkerexe:' + str(CLANG),
            '--lib:' + str(toolchain / 'lib'), '--path:' + str(bigint / 'src'),
            '--path:' + str(gmp_wrapper / 'src'),
            '--nimcache:' + str(cache), '--out:' + str(binary),
            '--passC:-isysroot ' + str(sdk), '--passL:-isysroot ' + str(sdk),
            '--passC:-mmacosx-version-min=14.4', '--passL:-mmacosx-version-min=14.4',
            '--passC:-ffp-contract=off',
            '--passC:-I' + str(PCRE / 'include'), '--passC:-I' + str(GMP / 'include'),
            '--passL:-L' + str(GMP / 'lib'),
            '--passL:' + str(PCRE / 'lib/libpcre.1.dylib'), '--passL:' + str(GMP / 'lib/libgmp.10.dylib'),
            str(source)]


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


def run(digest, revision, proposal_file):
    assert revision and revision.replace('-', '').isalnum(), 'Use an explicit fresh revision name'
    print('WAIT LOCK: guarded private Nim preparation', revision, flush=True)
    with (HERE / 'preparation-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert sha(proposal_file) == digest, 'Exact root-reviewed proposal changed'
            proposal = json.loads(proposal_file.read_text())
            guard = load_guard()
            guard.verify(proposal['pins'])
            proposal['pins'] = merge(proposal['pins'], {str(proposal_file): digest})
            import proof_reuse_v5
            import primitive_reuse_v6
            primitive = json.loads(Path(proposal['prior_primitive_reuse']).read_text())
            assert sha(proposal['prior_primitive_reuse']) == proposal['prior_primitive_reuse_sha256']
            primitive_reuse_v6.validate_packet(primitive, sys.modules[__name__], guard, harness=None)
            reuse = json.loads(Path(proposal['prior_proof_reuse']).read_text())
            assert sha(proposal['prior_proof_reuse']) == proposal['prior_proof_reuse_sha256']
            assert set(reuse['profiles']) == set(proposal['reused_profile_ids']) == proof_reuse_v5.PROFILES
            guard.verify(reuse['pins'])
            proof_reuse_v5.validate_prior_reports(reuse)
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            import references
            assert preflight_inputs(references) == proposal['case_inventory']
            assert reuse['pending_reused_profile_cases'] == proposal['pending_reused_profile_cases'] == proof_reuse_v5.PENDING_REUSED_CASES
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
                          reused_bounded_cases=154, reused_cases_relabelled_as_attempts=False,
                          prior_primitive_proof_reuse=proposal['prior_primitive_reuse'],
                          prior_primitive_proof_reuse_sha256=proposal['prior_primitive_reuse_sha256'],
                          reused_primitive_cases=3, reused_primitive_cases_relabelled_as_attempts=False,
                          prior_failed_diagnostic_retained=proposal['prior_failed_diagnostic_retained'])
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
                binding = proposal['toolchain_binding']
                assert binding == primitive['toolchain']
                compiler = Path(binding['compiler'])
                toolchain = Path(binding['toolchain'])
                bigint = Path(binding['bigint'])
                gmp_wrapper = Path(binding['gmp_wrapper'])
                new_json(build / 'toolchain-reuse.json', dict(binding,
                    source='exact unchanged private-v5 publisher archive/compatibility/version receipts',
                    never_chart_eligible=True))
                sdk = Path(proposal['sdk'])
                environment = dict(os.environ, LC_ALL='C', SDKROOT=str(sdk),
                                   DYLD_LIBRARY_PATH=LIBRARY_PATH,
                                   NIMBLE_DIR=str(build / 'nimble-unused'),
                                   OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
                for key in ('NIM_CONFIG_PATH', 'NIM_LIB_PREFIX', 'NIMBLE_PATH', 'NIMFLAGS', 'CFLAGS', 'CPPFLAGS', 'LDFLAGS'):
                    environment.pop(key, None)
                for key in ('DYLD_INSERT_LIBRARIES', 'DYLD_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH'):
                    environment.pop(key, None)
                pins = merge(proposal['pins'], primitive['pins'], pin_tree(snapshot), pin_tree(build))
                import source_proofs_v6 as source_proofs
                binaries = build / 'bin'
                binaries.mkdir()
                rows = []
                fresh_builds = 0
                for declared in proposal['profiles']:
                    if declared['id'] in reuse['profiles']:
                        prior = reuse['profiles'][declared['id']]
                        assert prior['declaration'] == declared
                        row = proof_reuse_v5.profile_row(prior)
                        rows.append(row)
                        old_cases = {record['case'] for record in prior['prior_records']}
                        assert old_cases <= set(proposal['case_inventory'][declared['benchmark']])
                        if declared['benchmark'] in NUMERIC:
                            missing = [('n-' + str(n), n, None) for n in NUMERIC[declared['benchmark']]
                                       if 'n-' + str(n) not in old_cases]
                        else:
                            missing = [(case, harness.WORK[declared['benchmark']][0], data)
                                       for case, data in inputs(declared['benchmark'], references) if case not in old_cases]
                        assert [case for case, _, _ in missing] == proposal['pending_reused_profile_cases'].get(declared['id'], [])
                        for case, n, data in missing:
                            expected = getattr(references, declared['benchmark'])(n if data is None else data)
                            probe(guard, harness, row, n, data, expected, pins, output, lock.fileno(), report, case)
                        row['new_native_cases'] = len(missing)
                        continue
                    fresh_builds += 1
                    source = snapshot / declared['source']
                    binary = binaries / declared['id']
                    cache = build / 'cache' / declared['id']
                    command = build_command(compiler, toolchain, bigint, gmp_wrapper, sdk, source, binary, cache)
                    owned_command(guard, harness, command, environment, output / 'builds' / declared['id'],
                                  lock.fileno(), 180, 'build')
                    pins = merge(pins, pin_tree(cache), {str(binary): sha(binary)})
                    pins = merge(pins, native_images(guard, harness, binary, environment,
                        output / 'builds' / declared['id'] / 'images', lock.fileno(),
                        [build / 'nim', GMP, PCRE, CLANG.parent.parent]))
                    guard.verify(pins)
                    row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='nim',
                               source=str(source), source_sha256=sha(source), binary_sha256=sha(binary),
                               command=[str(binary), '{n}'], cwd=str(HERE), runtime_environment={
                                   'LC_ALL': 'C', 'DYLD_LIBRARY_PATH': LIBRARY_PATH,
                                   'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1', 'VECLIB_MAXIMUM_THREADS': '1'})
                    rows.append(row)
                    benchmark = row['benchmark']
                    for n in NUMERIC.get(benchmark, ()):
                        expected = getattr(references, benchmark)(n)
                        probe(guard, harness, row, n, None, expected, pins, output, lock.fileno(), report, 'n-' + str(n))
                    if benchmark not in NUMERIC:
                        for case, data in inputs(benchmark, references):
                            probe(guard, harness, row, harness.WORK[benchmark][0], data,
                                  getattr(references, benchmark)(data), pins, output, lock.fileno(), report, case)
                assert fresh_builds == proposal['fresh_profile_builds_required'] == 0
                proofs = source_proofs.run(guard, harness, proposal, snapshot, build, compiler, toolchain,
                                          bigint, gmp_wrapper, sdk, environment, pins, lock.fileno(), report)
                assert len(report['attempts']) == proposal['fresh_source_proof_attempts_required'] == 21
                assert len(proofs) == 24 and sum(p.get('origin') == 'retained-private-v5' for p in proofs) == 3
                pins = merge(pins, {p:d for p,d in pin_tree(output).items() if Path(p) != guard.REPORT})
                guard.verify(pins)
                report.update(status='bounded-output-ownership-passed-awaiting-work-and-LOC-review',
                              finished_at=guard.now(), proofs=proofs, pins=pins, profiles=rows,
                              fresh_profile_builds=fresh_builds, build=str(build), toolchain_binding=binding,
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
