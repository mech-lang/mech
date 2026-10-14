"""Official Kotlin/JVM compiler intake using the closed, retained Clojure JDK."""
import argparse
import base64
from contextlib import ExitStack
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import stat
import sys
import traceback
import unicodedata
import urllib.request
import zipfile

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1]
DRAFT = H / 'drafts/private-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BENCHES = ('binarytrees','fannkuchredux','fasta','knucleotide','mandelbrot','nbody','pidigits','regexredux','revcomp','spectralnorm')
INPUTS = [H / name for name in ('RESEARCH_PLAN.md','TOOLCHAIN_CANDIDATE.json')] + [DRAFT / name
    for name in ('profiles.json','GATES.json','REVIEW.md','ROOT_REVIEW.md','support/Support.kt')] + [
    DRAFT / 'sources' / name / 'Main.kt' for name in BENCHES]
FILES = ('intake.py','RuntimeProbe.java','REVIEW.md')
PAYLOAD_URL = 'https://github.com/JetBrains/kotlin/releases/download/v2.4.21/kotlin-compiler-2.4.21.zip'
PAYLOAD_SHA = '7cc140e76daf416a0424a5557f99afd7ca89ebd463ea1101261e3e01c33e75cd'
CHECKSUM_URL = PAYLOAD_URL + '.sha256'
CHECKSUM_SHA = '69012ce92719f8fa0524ca69954e2c36800edd76c8cd7f74965550e53650ac05'
DONOR = C / 'clojure-comparison/20261012'
METADATA = C / 'continuation/publication-clojure-metadata-v1/qualified/actual-metadata-v1'
OWNER = C / 'continuation/owned-jobs/clojure-publication-metadata-v1'
ROOT = C / 'continuation/jobs/clojure-metadata-closed-review-v1/review.json'
METADATA_SHA = '8bf96ed71bd799b80fb6f062ff7b5b76c92d6e992e3e40ef307811f3c2c1b9ff'
OUTCOME_SHA = '770e27225303bd416f6d0d5fd98a45fab01e185274cc52b8b23e39a97e58db52'
TERMINAL_SHA = 'ccb781426b82f4f1a729fed369a7940f17b3088f9cb14dddd3dc6143e210bdd2'
RUNTIME_SHA = 'a23352ac520614665400ac0f14d5d088cfd0bef9cb4365f25002b21273b3c02a'
JDK = DONOR / '.build/private-v1/jdk/jdk-25.0.4.1+1/Contents/Home'
JAVA_SHA = '83c9c5310d044e7c4262432854eb1bc5f49763d7c1e9971777a646c91b96c06d'
JVM_SHA = 'c0c9cc842a9d33857be8262157d19ee9c1e15caf99a9bde784033b351b5dd117'
FLAGS = ['-XX:ActiveProcessorCount=8','-Xms64m','-Xmx8g','-XX:+UseG1GC']
SERVICE = C / 'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_SHA = '78b9f2870e0627b2aa6cc87ddcbd9ba54c1581ef7f28392f1859b79509dff9d4'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert re.fullmatch('[a-f0-9]{64}', digest) and sha(path) == digest, path
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Changed Kotlin prerequisite', path)


def cold(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules
    raw = bind(path, pins[str(path)], pins).read_bytes()
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path and sys.modules[name] is module
    return module


def retained_jdk(args, pins):
    root = read(ROOT, args.runtime_review_sha256, pins)
    assert root['status'] == 'passed-closed-Clojure-metadata-artifact-pin-review'
    assert root['verified_pin_count'] == 5210 and root['closed_lease'] is root['common_mutex_held'] is True
    assert root['metadata_path'] == str(METADATA / 'metadata.json') and root['metadata_sha256'] == METADATA_SHA
    assert root['outcome_sha256'] == OUTCOME_SHA and root['terminal_sha256'] == TERMINAL_SHA
    assert root['chart_admission'] is False and root['benchmark_runs'] == 0
    done = read(OWNER / 'terminal.json', TERMINAL_SHA, pins)
    request, start, worker = [read(OWNER / name, sha(OWNER / name), pins)
        for name in ('request.json','command-started.json','worker-started.json')]
    command = [PYTHON,'-I','-S','-B','-u',str(METADATA.parents[1] / 'qualify_descriptor.py'),
        '--revision','actual-metadata-v1','--qualifier-sha256','d148b0e178b25e06485888cdf46293f141a4991a2dc8780aaa40fdb58289c83e',
        '--descriptor-sha256','9d8f4ec52a3e9bebc4a89257a4d1fbb1a6cca56dee267c7c5ed128145885267a',
        '--review-sha256','6a1903eee578db29cabd946e00a6ff92bd9679447353b7493da9b23f6afe4ab5']
    assert request['command'] == start['command'] == worker['command'] == command
    assert request['cwd'] == start['cwd'] == worker['cwd'] == str(C.parents[1])
    assert request['common_mutex_owned_by_target_command'] is True
    assert request['qualification_granted'] is request['automatic_retry'] is False
    assert done['status'] == 'command-exited-zero-awaiting-report-review'
    assert type(done['exit_code']) is int and done['exit_code'] == 0 and done['stop_signals'] == []
    assert done['actual_command_started'] is True and done['command_identity_observation_missing'] is False
    assert not any(done.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
    assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid'] == 97908
    assert start['command_identity_observation_missing'] is False
    assert worker['worker_identity'] == done['worker_identity'] and done['worker_identity']['pid'] == 97904
    observed = done['command_identity']['listing'].split(None, 7)
    assert observed[:2] == ['97908','97904'] and observed[7].split() == command
    outcome = read(METADATA / 'outcome.json', OUTCOME_SHA, pins)
    assert outcome['status'] == 'metadata-qualified-awaiting-root-current-candidate-review'
    assert outcome['native_subjects'] == outcome['fixture_reexecutions'] == 0 and outcome['chart_admission'] is False
    assert not any(outcome.get(key) for key in ('error','error_type','integrity_errors','available_bytes_not_fully_pinned'))
    for values in (outcome['prerequisite_pins'], outcome['available_evidence_pins']):
        for path, digest in values.items():
            bind(path, digest, pins)
    metadata = read(METADATA / 'metadata.json', METADATA_SHA, pins)
    assert metadata['status'] == 'reviewed-actual-native-language-metadata'
    assert all(metadata[key] is True for key in ('actual_compiler_version_verified','actual_native_architecture_verified','actual_memory_policy_verified'))
    assert metadata['scored_native_architecture'] == 'arm64' and metadata['emulation_used'] is False
    for path, digest in metadata['pins'].items():
        bind(path, digest, pins)
    runtime = read(DONOR / 'qualification/private-v1/runtime-receipt.json', RUNTIME_SHA, pins)
    assert runtime['status'] == 'qualified-native-arm64-jvm-identity'
    assert runtime['java_home'] == str(JDK) and runtime['java'] == str(JDK / 'bin/java')
    assert runtime['java_runtime_version'] == '25.0.4.1+1' and runtime['measured_architecture'] == 'arm64'
    assert runtime['collector'] == 'G1' and runtime['flags'] == FLAGS
    bind(JDK / 'bin/java', JAVA_SHA, pins)
    bind(JDK / 'lib/server/libjvm.dylib', JVM_SHA, pins)
    for path in JDK.rglob('*'):
        if path.is_file():
            assert str(path) in pins, ('Unpinned retained JDK file', path)
            bind(path, pins[str(path)], pins)
    fixture_path = DONOR / 'proposal-seals/private-v1/generic-ownership-inventory.json'
    fixture = read(fixture_path, '44178d322cf7e5b390d3150c6e4cfb73b1e8758ab9bb8953dc7f6aa769f7b1e3', pins)
    assert fixture['status'] == 'passed' and fixture['inventory'] == 'ownership' and fixture['successful'] is True
    assert type(fixture['tests_run']) is type(fixture['discovered_tests']) is int
    assert fixture['tests_run'] == fixture['discovered_tests'] == 11
    assert len(set(fixture['expected_test_ids'])) == len(fixture['actual_test_ids']) == len(set(fixture['actual_test_ids'])) == 11
    assert set(fixture['expected_test_ids']) == set(fixture['actual_test_ids'])
    assert all(fixture[key] == [] for key in ('skipped','failures','errors','expected_failures','unexpected_successes'))
    command_path = fixture_path.with_name('ownership-fixtures.json')
    owned(read(command_path, pins[str(command_path)], pins))
    return runtime


def owned(value):
    assert value['inherited_common_mutex'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == 0
    assert not any(value.get(key) for key in ('error','error_type','cleanup_errors','remaining_group_members'))


def download(url, path, limit):
    request = urllib.request.Request(url, headers={'User-Agent':'mech-Kotlin-official-intake'})
    with urllib.request.urlopen(request, timeout=45) as response, path.open('xb') as stream:
        size = 0
        while chunk := response.read(65536):
            size += len(chunk)
            assert size <= limit, 'Publisher response exceeds reviewed bound'
            stream.write(chunk)


def extract(archive_path, destination):
    destination.mkdir(exist_ok=False)
    with zipfile.ZipFile(archive_path) as archive:
        entries = archive.infolist()
        assert 0 < len(entries) <= 8192 and sum(entry.file_size for entry in entries) <= 512 * 1024**2
        names, records = set(), []
        for entry in entries:
            name = PurePosixPath(entry.filename)
            assert entry.filename and '\\' not in entry.filename and not name.is_absolute()
            assert all(part not in ('','.','..') for part in entry.filename.rstrip('/').split('/'))
            assert name.parts[0] == 'kotlinc' and not entry.flag_bits & 1
            key = unicodedata.normalize('NFD', str(name)).casefold()
            assert key not in names, 'Duplicate/case-colliding compiler archive member'
            names.add(key)
            mode = entry.external_attr >> 16
            kind = stat.S_IFMT(mode)
            assert kind in (0, stat.S_IFDIR) if entry.is_dir() else kind in (0, stat.S_IFREG)
            target = destination.joinpath(*name.parts)
            assert target.resolve().is_relative_to(destination.resolve())
            if entry.is_dir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.open(entry) as source, target.open('xb') as output:
                    shutil.copyfileobj(source, output)
                assert target.stat().st_size == entry.file_size
                target.chmod(0o444 | (mode & 0o111))
            records.append(dict(name=entry.filename, bytes=entry.file_size, crc32=entry.CRC,
                original_mode=mode, directory=entry.is_dir()))
    return records


def run(args, output, build, pins, common_fd):
    packet_path = Path(args.packet).resolve(strict=True)
    assert not packet_path.stat().st_mode & 0o222
    packet = read(packet_path, args.packet_sha256, pins)
    expected = {str(path) for path in INPUTS} | {str(P / name) for name in FILES} | {str(Path(sys.executable).resolve())}
    assert set(packet) == {'schema','pins'} and packet['schema'] == 'frozen-kotlin-retained-jdk-intake-v1'
    assert set(packet['pins']) == expected and len(INPUTS) == 17
    for path, digest in packet['pins'].items():
        if path != str(Path(sys.executable).resolve()):
            assert not Path(path).stat().st_mode & 0o222
        bind(path, digest, pins)
    runtime = retained_jdk(args, pins)
    candidate = read(H / 'TOOLCHAIN_CANDIDATE.json', pins[str(H / 'TOOLCHAIN_CANDIDATE.json')], pins)
    assert candidate['compiler_version'] == '2.4.21' and candidate['archive_url'] == PAYLOAD_URL
    assert candidate['publisher_archive_sha256_observed'] == PAYLOAD_SHA
    assert candidate['publisher_checksum_url'] == CHECKSUM_URL and candidate['qualified'] is candidate['chart_admission'] is False
    bind(SERVICE, SERVICE_SHA, pins)
    service = cold(SERVICE, 'kotlin_retained_original_owned_service', pins)
    guard = cold(service.GUARD, 'kotlin_retained_original_guard', pins)
    harness = guard.load_harness()
    for module in (harness, harness.compact, harness.compact.common):
        path = Path(module.__file__).resolve(strict=True)
        bind(path, pins[str(path)], pins)
    tool_flags = FLAGS + ['-Duser.home=' + str(build / 'home'), '-Djava.io.tmpdir=' + str(build / 'tmp')]
    environment = dict(PATH=str(JDK / 'bin') + ':/usr/bin:/bin:/usr/sbin:/sbin', JAVA_HOME=str(JDK), JAVACMD=str(JDK / 'bin/java'),
        JAVA_OPTS=' '.join(tool_flags), JAVA_TOOL_OPTIONS='', JDK_JAVA_OPTIONS='', _JAVA_OPTIONS='', CLASSPATH='',
        KOTLIN_RUNNER='', KOTLIN_COMPILER='', KOTLIN_TOOL='', LC_ALL='C', LANG='C',
        HOME=str(build / 'home'), XDG_CONFIG_HOME=str(build / 'config'), TMPDIR=str(build / 'tmp') + os.sep,
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
    for name in ('home','config','tmp'):
        (build / name).mkdir(exist_ok=False)
    for path in ('/bin/bash','/usr/bin/env','/usr/bin/uname','/usr/bin/dirname'):
        bind(path, sha(path), pins)
    assert not list(service.HERE.glob('*.class')) and not (service.HERE / '.java').exists()
    checksum, payload = build / 'kotlin-compiler-2.4.21.zip.sha256', build / 'kotlin-compiler-2.4.21.zip'
    verify(pins)
    download(CHECKSUM_URL, checksum, 4096)
    bind(checksum, CHECKSUM_SHA, pins)
    assert re.fullmatch(PAYLOAD_SHA + r'\s*', checksum.read_text())
    download(PAYLOAD_URL, payload, 128 * 1024**2)
    bind(payload, PAYLOAD_SHA, pins)
    members = extract(payload, build / 'compiler')
    compiler = build / 'compiler/kotlinc'
    for path in compiler.rglob('*'):
        assert not path.is_symlink()
        if path.is_file():
            pins[str(path)] = sha(path)
    wrapper = compiler / 'bin/kotlinc'
    assert wrapper.is_file() and os.access(wrapper, os.X_OK)
    text = wrapper.read_text()
    assert text.startswith('#!/usr/bin/env bash\n')
    assert all(token in text for token in ('JAVACMD="$JAVA_HOME/bin/java"','K2JVMCompiler','kotlin-preloader.jar','kotlin-compiler.jar'))
    jars = {str(path): pins[str(path)] for path in sorted((compiler / 'lib').glob('*.jar'))}
    assert jars and all((compiler / 'lib' / name).is_file() for name in
        ('kotlin-preloader.jar','kotlin-compiler.jar','kotlin-stdlib.jar'))
    save(output / 'publisher-intake.json', dict(payload_url=PAYLOAD_URL, payload_sha256=PAYLOAD_SHA,
        checksum_url=CHECKSUM_URL, checksum_sha256=CHECKSUM_SHA, archive_members=members,
        extracted_root=str(compiler), complete_compiler_jars=jars, wrapper=str(wrapper), wrapper_sha256=sha(wrapper),
        compiler_dependency_execution_review_qualified=False))
    profiles = read(DRAFT / 'profiles.json', pins[str(DRAFT / 'profiles.json')], pins)
    assert profiles['language'] == 'kotlin' and profiles['physical_baseline_main_count'] == 10
    assert profiles['distinct_optimizer_claim'] is False and profiles['authored_general_helpers'] == ['support/Support.kt']
    assert profiles['authored_launcher_helpers'] == [] and profiles['all_authored_helpers_counted'] is True
    rows = profiles['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {(bench, form) for bench in BENCHES for form in ('compact','optimized')}
    forms = []
    for row in rows:
        bench, form = row['benchmark'], row['variant']
        assert row['id'] == 'kotlin-' + bench + '-' + form and row['source'] == 'sources/' + bench + '/Main.kt'
        assert row['alias_of'] == 'kotlin-' + bench + '-baseline-v1' and row['authored_helpers'] == ['support/Support.kt']
        assert row['stdin'] is (bench in ('regexredux','revcomp','knucleotide'))
        assert row['maximum_subject_workers'] == 1 and row['source_input_materialized'] is False
        assert row['canonical_loc'] is row['formatted_loc'] is None and row['work_qualified'] is row['chart_admission'] is False
        folder = build / 'forms' / row['id']
        folder.mkdir(parents=True, exist_ok=False)
        classes = folder / 'classes'
        classes.mkdir()
        sources = []
        for origin, name, role in ((DRAFT / row['source'],'Main.kt','application'),
            (DRAFT / 'support/Support.kt','Support.kt','application-helper')):
            target = folder / name
            with target.open('xb') as stream:
                stream.write(origin.read_bytes())
            assert sha(target) == pins[str(origin)]
            target.chmod(0o444)
            pins[str(target)] = pins[str(origin)]
            sources.append(dict(path=str(target), sha256=pins[str(target)], origin=str(origin), role=role, counted=True))
        forms.append(dict(id=row['id'], benchmark=bench, variant=form, alias_of=row['alias_of'], authored_inputs=sources,
            compile_command_proposed=[str(wrapper), str(folder / 'Support.kt'), str(folder / 'Main.kt'), '-jvm-target','17','-no-reflect','-d',str(classes)],
            runtime_command_proposed=[str(JDK / 'bin/java'), *FLAGS, '-cp', os.pathsep.join([str(classes),str(compiler / 'lib/kotlin-stdlib.jar')]),
                'clbg.' + bench + '.MainKt','{n}'], authored_launcher_count=0,
            main_dispatch='source-defined Kotlin top-level main; generated MainKt class still unverified',
            subject_compiler_executed=False, runtime_classpath_qualified=False, source_work_qualified=False,
            canonical_loc=None, formatted_loc=None, chart_admission=False))
    save(output / 'source-input-inventory.json', dict(status='exact20-full-inputs-materialized-not-compiled',
        forms=forms, distinct_main_inputs=20, counted_helper_inputs=20, authored_launcher_inputs=0,
        original_input_packet=dict(path=str(packet_path), sha256=args.packet_sha256), tool_probe_excluded_from_scored_LOC=True))
    verify(pins)
    version_command = [str(wrapper), '-version']
    version = service.owned_command(guard, harness, version_command, environment, output / 'version', common_fd, 120, 'compiler-version')
    owned(version['detail'])
    version_text = (version['stdout'] + version['stderr']).decode('utf-8')
    assert re.search(r'kotlinc-jvm 2\.4\.21\b', version_text)
    probe_classes = build / 'probe-classes'
    probe_classes.mkdir()
    compile_command = [str(JDK / 'bin/javac'), *['-J' + flag for flag in tool_flags],
        '-encoding','UTF-8','-d',str(probe_classes),str(P / 'RuntimeProbe.java')]
    compiled = service.owned_command(guard, harness, compile_command, environment, output / 'probe-build', common_fd, 120, 'metadata-probe-compile')
    owned(compiled['detail'])
    assert compiled['stdout'] == compiled['stderr'] == b''
    assert {path.name for path in probe_classes.iterdir()} == {'RuntimeProbe.class'}
    bind(probe_classes / 'RuntimeProbe.class', sha(probe_classes / 'RuntimeProbe.class'), pins)
    probe_command = [str(JDK / 'bin/java'), *tool_flags, '-cp',str(probe_classes),'RuntimeProbe']
    probed = service.owned_command(guard, harness, probe_command, environment, output / 'probe', common_fd, 120, 'runtime-configuration')
    owned(probed['detail'])
    assert probed['stderr'] == b''
    values, flags, collectors = {}, [], []
    for line in probed['stdout'].decode('ascii').splitlines():
        key, encoded = line.split('=',1)
        value = base64.b64decode(encoded, validate=True).decode('utf-8')
        if key == 'JVM_ARG': flags.append(value)
        elif key == 'COLLECTOR': collectors.append(value)
        else:
            assert key not in values
            values[key] = value
    required = dict(JAVA_HOME=str(JDK), RUNTIME_VERSION='25.0.4.1+1-LTS', VENDOR='Eclipse Adoptium', OS_ARCH='aarch64',
        CLASS_PATH=str(probe_classes), PROCESSORS='8', USER_HOME=str(build / 'home'), JAVA_TMPDIR=str(build / 'tmp'))
    assert all(values[key] == value for key,value in required.items()) and flags == tool_flags
    assert set(values) == set(required) | {'VM_NAME','VM_INFO','COMPILER','MAX_HEAP','FILE_ENCODING'}
    assert '64-Bit' in values['VM_NAME'] and 'mixed mode' in values['VM_INFO'] and 'Tiered' in values['COMPILER']
    assert 0 < int(values['MAX_HEAP']) <= 8 * 1024**3
    assert collectors == ['G1 Young Generation','G1 Concurrent GC','G1 Old Generation']
    verify(pins)
    save(output / 'intake-result.json', dict(status='official-compiler-and-retained-JDK-intake-passed-awaiting-subject-and-dependency-review',
        compiler=str(wrapper), compiler_version_stdout_stderr=version_text, compiler_vendor_pins=jars,
        retained_JDK_lineage=runtime, actual_generic_runtime_probe=values, actual_jvm_flags=flags, actual_collectors=collectors,
        exact_runtime_environment=environment, authored_forms=20, counted_main_inputs=20, counted_support_inputs=20,
        authored_workload_launcher_inputs=0, owned_tool_commands=3, vendor_wrapper_version_descendant_source_route=True,
        actual_descendant_JVM_process_count_observed=False, original_ownership_fixtures_reused=11, fixture_reexecutions=0,
        original_JDK_reinstalled=False, subject_compiler_commands=0, native_benchmarks=0, raw_stdio_qualified=False,
        compiler_native_dependency_execution_qualified=False, subject_JIT_method_evidence_qualified=False,
        source_work_qualified=False, canonical_loc=None, formatted_loc=None, runtime_configuration_admitted=False,
        chart_admission=False, pins=dict(pins)))


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','packet','packet-sha256','runtime-review-sha256'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    assert sys.dont_write_bytecode and re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', args.revision)
    assert all(re.fullmatch('[a-f0-9]{64}', value) for key,value in vars(args).items() if key.endswith('_sha256'))
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Kotlin intake interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP,signal.SIGALRM):
        signal.signal(signum, interrupted)
    print('WAIT common mutex: Kotlin official compiler intake and generic JDK probes', flush=True)
    with ExitStack() as leases, LOCK.open('r+') as common:
        dependency = leases.enter_context((OWNER / 'owner.lock').open('rb'))
        fcntl.flock(dependency, fcntl.LOCK_SH | fcntl.LOCK_NB)
        held, named = os.fstat(dependency.fileno()), (OWNER / 'owner.lock').stat()
        assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
        owner_path = H / 'retained-jdk-intake-v1-owner.lock'
        owner = leases.enter_context(owner_path.open('a+'))
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        held, named = os.fstat(owner.fileno()), owner_path.stat()
        assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
        fcntl.flock(common, fcntl.LOCK_EX)
        held, named = os.fstat(common.fileno()), LOCK.stat()
        assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino) and held.st_ino == 79859134
        output, build = H / 'jobs' / args.revision, H / '.build' / args.revision
        assert not output.exists() and not build.exists()
        output.mkdir(parents=True)
        build.mkdir(parents=True)
        pins = {}
        outcome = dict(status='failed-Kotlin-toolchain-intake', chart_admission=False, fixture_reexecutions=0,
            subject_compiler_commands=0, native_benchmarks=0, automatic_retry=False)
        signal.alarm(600)
        try:
            run(args, output, build, pins, common.fileno())
            outcome['status'] = 'official-compiler-and-retained-JDK-intake-passed-awaiting-subject-and-dependency-review'
        except BaseException as error:
            outcome.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
            (output / 'actual-error.log').write_text(traceback.format_exc())
            raise
        finally:
            signal.alarm(0)
            errors, available = [], {}
            for path, digest in pins.items():
                try:
                    assert sha(path) == digest
                except BaseException as error:
                    errors.append(dict(path=path, error=str(error)))
            # Private HOME/config/tmp are isolated runtime state. Evidence and
            # extracted compiler/form/class bytes are pinned; no credential read.
            for root in (output, build / 'compiler', build / 'forms', build / 'probe-classes'):
                if root.exists():
                    for path in root.rglob('*'):
                        if path.is_file():
                            try:
                                available[str(path)] = sha(path)
                            except BaseException as error:
                                errors.append(dict(path=str(path), error=str(error)))
            for path in build.glob('kotlin-compiler-*'):
                if path.is_file():
                    try:
                        available[str(path)] = sha(path)
                    except BaseException as error:
                        errors.append(dict(path=str(path), error=str(error)))
            if errors:
                outcome.update(status='failed-terminal-integrity', integrity_errors=errors, available_bytes_not_fully_pinned=True)
            outcome.update(finished_at=datetime.now(timezone.utc).isoformat(), prerequisite_pins=pins,
                available_evidence_pins=available, exclusive_common_mutex_held=True, common_mutex_inode=79859134,
                original_JDK_and_sources_preserved=not errors)
            save(output / 'outcome.json', outcome)
            if errors:
                raise RuntimeError('Kotlin intake terminal integrity failed; actual evidence preserved')


if __name__ == '__main__':
    main()
