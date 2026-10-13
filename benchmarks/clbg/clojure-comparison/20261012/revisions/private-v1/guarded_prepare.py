"""Unexecuted Clojure candidate: metadata seal, then explicitly reviewed preparation.

All download/hash/fixtures/compiler/native work is serialized by the common
mutex. The first seal fetches checksum metadata only, not executable payloads.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sys
import tarfile
import urllib.request

HERE = Path(__file__).resolve().parents[2]
REVISION_SOURCE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
SERVICE = CLBG / 'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_FIXTURES = SERVICE.with_name('fixture_runner.py')
REFERENCE = SERVICE.with_name('references.py')
DNA_REFERENCE = CLBG / 'v-report-only/20261012/dna_oracles.py'
JDK_VERSION = '25.0.4.1+1'
JDK_RELEASE = 'https://github.com/adoptium/temurin25-binaries/releases/tag/jdk-25.0.4.1%2B1'
JDK_ARCHIVE = 'OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.4.1_1.tar.gz'
JDK_URL = 'https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4.1%2B1/' + JDK_ARCHIVE
CLOJURE_VERSION = '1.12.6'
ARTIFACTS = [('clojure', '1.12.6'), ('spec.alpha', '0.5.238'), ('core.specs.alpha', '0.4.74')]
BENCHMARKS = {'binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
              'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux'}
JVM_FLAGS = ['-XX:ActiveProcessorCount=8', '-Xms64m', '-Xmx8g', '-XX:+UseG1GC']
GATES = {'sampler_exited_normally', 'authoritative_native_group_received',
    'native_inherited_common_mutex', 'native_exited_normally', 'no_execution_error',
    'no_wrapper_integrity_error', 'no_resource_violation', 'no_resource_monitor_errors',
    'no_compiler_power_monitor_errors', 'no_monitor_finish_error', 'no_execute_cleanup_error',
    'no_remaining_execute_group', 'no_supervisor_cleanup', 'no_supervisor_remaining_group',
    'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged'}


def exact_true_checks(attempt):
    checks = attempt['qualification_checks']
    assert type(checks) is dict and set(checks) == GATES
    assert all(type(value) is bool and value is True for value in checks.values())
    assert attempt['qualified'] is True and attempt['full_workload'] is False
    assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert attempt['pins_unchanged_at_end'] is True


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def profiles(value):
    assert value['language'] == 'clojure' and value['compiler_version'] == CLOJURE_VERSION
    rows = value['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {
        (benchmark, variant) for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
    for row in rows:
        expected_namespace = 'clbg.' + row['benchmark'] + '.' + row['variant']
        expected_source = 'sources/' + expected_namespace.replace('.', '/') + '.clj'
        assert row['namespace'] == expected_namespace and row['source'] == expected_source
        assert row['compiled_source_bundle'] == [expected_source, 'support/clbg/support.clj']
        assert type(row['maximum_compute_workers']) is int and 1 <= row['maximum_compute_workers'] <= 8
        assert row['loc'] is None and row['eligible_for_chart'] is False
        for name in row['compiled_source_bundle']:
            path = Path(name)
            assert not path.is_absolute() and '..' not in path.parts and path.suffix == '.clj'
            assert (HERE / path).is_file()
    return rows


def static_paths():
    return sorted(set(HERE.glob('*.md')) | {HERE / 'profiles.json'} |
        set((HERE / 'sources').rglob('*.clj')) | set((HERE / 'support').rglob('*.clj')) |
        set(REVISION_SOURCE.glob('*.py')) | set(REVISION_SOURCE.glob('*.md')))


def artifact_url(name, version):
    return 'https://repo.maven.apache.org/maven2/org/clojure/' + name + '/' + version + '/' + name + '-' + version + '.jar'


def fetch(url, destination, maximum_bytes):
    destination = Path(destination)
    assert not destination.exists(), 'Exclusive download, no implicit retry'
    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(url, headers={'User-Agent': 'mech-local-clojure-preparation'})
    with urllib.request.urlopen(request, timeout=60) as response, destination.open('xb') as stream:
        count = 0
        while chunk := response.read(65536):
            count += len(chunk)
            assert count <= maximum_bytes, 'Publisher payload exceeds reviewed bound'
            stream.write(chunk)


def checksum(path, length):
    digest = Path(path).read_text().strip().split()[0].lower()
    assert re.fullmatch('[0-9a-f]{' + str(length) + '}', digest), 'Malformed publisher checksum'
    return digest


def extract_jdk(archive, output):
    assert not output.exists()
    output.mkdir(parents=True)
    with tarfile.open(archive, 'r:gz') as bundle:
        seen = set()
        for entry in bundle.getmembers():
            path = Path(entry.name)
            assert not path.is_absolute() and '..' not in path.parts and entry.name not in seen
            seen.add(entry.name)
            assert entry.isdir() or entry.isfile() or entry.issym(), 'No hard/device archive member'
            if entry.issym():
                target = Path(entry.linkname)
                assert not target.is_absolute()
                assert (output / path.parent / target).resolve().is_relative_to(output.resolve())
        bundle.extractall(output, filter='data')
    homes = list(output.glob('*/Contents/Home'))
    assert len(homes) == 1
    for path in output.rglob('*'):
        if path.is_symlink():
            target = path.resolve(strict=True)
            assert target.is_relative_to(output.resolve()) and target.exists()
    return homes[0]


def controlled_environment(java_home):
    return dict(LC_ALL='C', LANG='C', JAVA_HOME=str(java_home), CLASSPATH='',
        JAVA_TOOL_OPTIONS='', JDK_JAVA_OPTIONS='', _JAVA_OPTIONS='',
        OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')


def compile_command(java, package, classes, jars, namespace):
    assert re.fullmatch(r'clbg\.[a-z]+\.(compact|optimized)', namespace)
    classpath = os.pathsep.join(map(str, [package / 'sources', package / 'support', *jars]))
    expression = '(binding [*compile-path* ' + json.dumps(str(classes)) + ' *compiler-options* {:direct-linking true}] (compile \'clbg.support) (compile \'' + namespace + '))'
    return [str(java), *JVM_FLAGS, '-cp', classpath, 'clojure.main', '-e', expression]


def runtime_command(java, classes, jars, namespace, n='{n}'):
    assert re.fullmatch(r'clbg\.[a-z]+\.(compact|optimized)', namespace)
    classpath = os.pathsep.join(map(str, [classes, *jars]))
    assert not any(part.name in ('sources', 'support') for part in [classes, *jars])
    return [str(java), *JVM_FLAGS, '-cp', classpath, namespace, str(n)]


def input_cases(service, references, benchmark):
    cases = tuple(service.inputs(benchmark, references))
    if benchmark == 'revcomp':
        cases += (('preheader', b'ACGT\n;comment\r\n>one\rA\n>empty'),)
    return cases


def inventory(service, references):
    values = {}
    for benchmark in sorted(BENCHMARKS):
        cases = [('n-' + str(n), None) for n in service.NUMERIC[benchmark]] if benchmark in service.NUMERIC else input_cases(service, references, benchmark)
        labels = [name for name, data in cases]
        assert labels and len(labels) == len(set(labels))
        assert all(re.fullmatch(r'[a-z0-9][a-z0-9-]*', name) for name in labels)
        assert all(data is None or type(data) is bytes for name, data in cases)
        values[benchmark] = labels
    return values


def seal(proposal_file):
    print('WAIT common mutex: Clojure metadata/checksum seal', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not proposal_file.exists()
        service = module('clojure_generic_owned_service', SERVICE)
        guard = service.load_guard()
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        reference = module('clojure_seal_reference', REFERENCE)
        rows = profiles(json.loads((HERE / 'profiles.json').read_text()))
        case_inventory = inventory(service, reference)
        output = HERE / 'proposal-seals' / proposal_file.stem
        assert not output.exists()
        output.mkdir(parents=True)
        ownership_receipt = output / 'generic-ownership-inventory.json'
        service.owned_command(guard, harness, [sys.executable, '-B', str(SERVICE_FIXTURES), '--set', 'ownership', '--receipt', str(ownership_receipt)],
            dict(os.environ, PYTHONPATH=str(SERVICE.parent)), output, lock.fileno(), 30, 'ownership-fixtures')
        service_fixture = module('clojure_generic_fixture_inventory', SERVICE_FIXTURES)
        service_fixture.validate_receipt(json.loads(ownership_receipt.read_text()), 'ownership')
        local_receipt = output / 'clojure-command-bundle-inventory.json'
        service.owned_command(guard, harness, [sys.executable, '-B', str(REVISION_SOURCE / 'fixture_runner.py'), '--receipt', str(local_receipt)],
            dict(os.environ, PYTHONPATH=str(REVISION_SOURCE)), output, lock.fileno(), 30, 'candidate-fixtures')
        local_fixture = module('clojure_local_fixture_inventory', REVISION_SOURCE / 'fixture_runner.py')
        local_fixture.validate_receipt(json.loads(local_receipt.read_text()))
        jdk_checksum = output / (JDK_ARCHIVE + '.sha256')
        fetch(JDK_URL + '.sha256.txt', jdk_checksum, 4096)
        jdk_sha256 = checksum(jdk_checksum, 64)
        artifacts = []
        for name, version in ARTIFACTS:
            path = output / (name + '-' + version + '.jar.sha1')
            fetch(artifact_url(name, version) + '.sha1', path, 4096)
            artifacts.append(dict(name=name, version=version, url=artifact_url(name, version),
                publisher_sha1=checksum(path, 40), publisher_checksum=str(path)))
        saved_runtime = CLBG / 'matlab-comparison/20261010/results'
        dependencies = [service.GUARD, service.SUITE / 'run.py', service.SUITE / 'prepare.py',
            REFERENCE, DNA_REFERENCE, saved_runtime / 'builds.json', saved_runtime / 'java-install.json', Path(tarfile.__file__)]
        service_paths = set(SERVICE.parent.glob('*.py')) | set(SERVICE.parent.glob('*.md'))
        pins = service.merge({str(path): sha(path) for path in static_paths() + dependencies},
            {str(path): sha(path) for path in service_paths}, service.pin_tree(output))
        proposal = dict(status='sealed-awaiting-root-review', language='clojure', created_at=guard.now(),
            profiles=rows, snapshot_paths=[str(path) for path in static_paths()], pins=pins,
            case_inventory=case_inventory, actual_fixture_receipts=[str(ownership_receipt), str(local_receipt)],
            jdk_version=JDK_VERSION, jdk_archive_url=JDK_URL, jdk_archive_sha256=jdk_sha256,
            jdk_release=JDK_RELEASE, artifacts=artifacts, runtime_flags=JVM_FLAGS,
            compiler_version=CLOJURE_VERSION, backend='JVM bytecode then fresh-process ARM64 HotSpot JIT',
            collector='explicit G1, pending actual runtime verification',
            shared_timing_lock=str(LOCK), guard_sha256=service.GUARD_SHA,
            compiler_or_subject_processes=0, executable_payload_downloads=0,
            full_processes_authorized=0, full_timings=False, chart_admission=False,
            no_automatic_retry=True, canonical_LOC_admitted=False,
            mandatory_pending_gates=['actual source-bound tree allocation/retention',
                'seven complete growing maps/windows and owner handoff', 'both actual Pi quotients',
                'compiled bytecode/source work review', 'actual native retained common-mutex descriptor',
                'canonical exact-authored LOC overlay',
                'root-reviewed full manifests and nine actual samples per benchmark'])
        guard.verify(pins)
        new_json(proposal_file, proposal)
        print(json.dumps(dict(proposal=str(proposal_file), sha256=sha(proposal_file)), indent=2), flush=True)


def prepare(proposal_file, digest, revision):
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', revision or '')
    print('WAIT common mutex: Clojure explicit guarded preparation', revision, flush=True)
    with (HERE / 'preparation-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert sha(proposal_file) == digest, 'Exact reviewed proposal changed'
            proposal = json.loads(proposal_file.read_text())
            for path, pin in proposal['pins'].items():
                assert sha(path) == pin, ('Sealed dependency changed before import', path)
            service = module('clojure_generic_native_service', SERVICE)
            guard = service.load_guard()
            guard.verify(proposal['pins'])
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            reference = module('clojure_native_reference', REFERENCE)
            dna = module('clojure_native_exact_dna', DNA_REFERENCE)
            reference.knucleotide = dna.knucleotide
            assert inventory(service, reference) == proposal['case_inventory']
            output = HERE / 'qualification' / revision
            build = HERE / '.build' / revision
            assert not output.exists() and not build.exists(), 'Exclusive revision, no retry'
            output.mkdir(parents=True)
            build.mkdir(parents=True)
            guard.OUT, guard.REPORT = output, output / 'report.json'
            report = dict(status='preparing', started_at=guard.now(), proposal_sha256=digest,
                attempts=[], active=None, full_performance_runs=0, chart_admission=False,
                canonical_LOC_admitted=False, no_automatic_retry=True)
            guard.save(guard.REPORT, report)
            try:
                snapshot = output / 'source-snapshot'
                snapshot.mkdir()
                for name in proposal['snapshot_paths']:
                    source = Path(name)
                    assert source.is_relative_to(HERE) and name in proposal['pins']
                    target = snapshot / source.relative_to(HERE)
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, target)
                    assert sha(target) == proposal['pins'][name]
                archive = build / JDK_ARCHIVE
                fetch(proposal['jdk_archive_url'], archive, 512 * 1024 * 1024)
                assert sha(archive) == proposal['jdk_archive_sha256'], 'Exact official JDK payload mismatch'
                java_home = extract_jdk(archive, build / 'jdk')
                java = java_home / 'bin/java'
                runtime_environment = controlled_environment(java_home)
                environment = dict(os.environ, **runtime_environment)
                pins = service.merge(proposal['pins'], {str(proposal_file): digest, str(archive): sha(archive)},
                    service.pin_tree(snapshot), service.pin_tree(build / 'jdk'))
                tools_output = output / 'runtime-identity'
                for executable in ('java', 'javac', 'javap'):
                    result = service.owned_command(guard, harness, [str(java_home / ('bin/' + executable)), '-version'],
                        environment, tools_output, lock.fileno(), 30, executable + '-version')
                    assert JDK_VERSION in (result['stdout'] + result['stderr']).decode() or executable != 'java'
                properties = service.owned_command(guard, harness, [str(java), *JVM_FLAGS, '-XshowSettings:properties', '-version'],
                    environment, tools_output, lock.fileno(), 30, 'jvm-properties')
                detail = (properties['stdout'] + properties['stderr']).decode()
                assert 'os.arch = aarch64' in detail and 'java.runtime.version = ' + JDK_VERSION in detail
                assert 'Eclipse Adoptium' in detail
                for name, image in [('java', java), ('libjvm', java_home / 'lib/server/libjvm.dylib')]:
                    pins = service.merge(pins, service.native_images(guard, harness, image, environment,
                        output / 'runtime-images' / name, lock.fileno(), [java_home]))
                jars = []
                for artifact in proposal['artifacts']:
                    path = build / 'vendor' / (artifact['name'] + '-' + artifact['version'] + '.jar')
                    fetch(artifact['url'], path, 32 * 1024 * 1024)
                    with path.open('rb') as stream:
                        assert hashlib.file_digest(stream, 'sha1').hexdigest() == artifact['publisher_sha1']
                    pins[str(path)] = sha(path)
                    jars.append(path)
                service.owned_command(guard, harness, [str(java), *JVM_FLAGS, '-cp', os.pathsep.join(map(str, jars)), 'clojure.main', '-e',
                    '(assert (= "1.12.6" (clojure-version))) (assert (= 8 (.availableProcessors (Runtime/getRuntime)))) (doseq [g (java.lang.management.ManagementFactory/getGarbageCollectorMXBeans)] (println (.getName g)))'],
                    environment, tools_output, lock.fileno(), 30, 'language-collector-identity')
                collector = (tools_output / 'language-collector-identity.stdout').read_text()
                assert 'G1 Young Generation' in collector and 'G1 Old Generation' in collector
                new_json(output / 'runtime-receipt.json', dict(status='qualified-native-arm64-jvm-identity',
                    compiler_version=CLOJURE_VERSION, java_home=str(java_home), java=str(java),
                    java_runtime_version=JDK_VERSION, measured_architecture='arm64',
                    backend='JVM bytecode then HotSpot JIT', collector='G1', flags=JVM_FLAGS,
                    vendor_jars=[dict(path=str(path), sha256=sha(path)) for path in jars]))
                pins = service.merge(pins, service.pin_tree(tools_output), service.pin_tree(output / 'runtime-images'),
                    {str(output / 'runtime-receipt.json'): sha(output / 'runtime-receipt.json')})
                rows = []
                for declared in proposal['profiles']:
                    package = build / 'packages' / declared['id']
                    classes = build / 'classes' / declared['id']
                    classes.mkdir(parents=True)
                    inputs = []
                    for index, name in enumerate(declared['compiled_source_bundle']):
                        source = snapshot / name
                        target = package / name
                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(source, target)
                        assert sha(target) == sha(source)
                        inputs.append(dict(path=str(target), sha256=sha(target), origin=str(source),
                            role='application' if index == 0 else 'application-helper'))
                    command = compile_command(java, package, classes, jars, declared['namespace'])
                    service.owned_command(guard, harness, command, environment, output / 'builds' / declared['id'],
                        lock.fileno(), 180, 'compile')
                    main_class = classes / (declared['namespace'].replace('.', '/') + '.class')
                    assert main_class.is_file() and (classes / 'clbg/support__init.class').is_file()
                    class_pins = service.pin_tree(classes)
                    assert class_pins and all(Path(path).suffix == '.class' for path in class_pins)
                    pins = service.merge(pins, service.pin_tree(package), class_pins)
                    receipt = output / 'builds' / declared['id'] / 'compiler-input-inventory.json'
                    new_json(receipt, dict(status='compiled-exact-authored-input-inventory', profile=declared['id'],
                        source=inputs[0]['path'], source_sha256=inputs[0]['sha256'], source_bundle=inputs,
                        compiler_jar=str(jars[0]), compiler_sha256=sha(jars[0]), build_command=command,
                        compiled_artifacts=class_pins, runtime_classpath=[str(classes), *map(str, jars)]))
                    pins[str(receipt)] = sha(receipt)
                    row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='clojure',
                        source=inputs[0]['path'], source_sha256=inputs[0]['sha256'], source_bundle=inputs,
                        binary=str(java), binary_sha256=sha(java),
                        support=service.merge(class_pins, service.pin_tree(java_home),
                            {str(path): sha(path) for path in jars}, {entry['path']: entry['sha256'] for entry in inputs[1:]}),
                        command=runtime_command(java, classes, jars, declared['namespace']), cwd=str(HERE),
                        runtime_environment=runtime_environment, compiler_version=CLOJURE_VERSION,
                        runtime_version=JDK_VERSION, measured_architecture='arm64', backend='JVM HotSpot JIT',
                        compiled_input_inventory=str(receipt), compiled_input_inventory_sha256=sha(receipt),
                        compiled_artifacts=class_pins, runtime_classpath=[str(classes), *map(str, jars)])
                    guard.verify(pins)
                    for n in service.NUMERIC.get(row['benchmark'], ()):
                        service.probe(guard, harness, row, n, None, getattr(reference, row['benchmark'])(n),
                            pins, output, lock.fileno(), report, 'n-' + str(n))
                    if row['benchmark'] not in service.NUMERIC:
                        for case, data in input_cases(service, reference, row['benchmark']):
                            service.probe(guard, harness, row, harness.WORK[row['benchmark']][0], data,
                                getattr(reference, row['benchmark'])(data), pins, output, lock.fileno(), report, case)
                    rows.append(row)
                assert len(rows) == 20
                assert len(report['attempts']) == sum(len(proposal['case_inventory'][row['benchmark']]) for row in rows)
                for attempt in report['attempts']:
                    exact_true_checks(attempt)
                terminal = {str(path): sha(path) for path in output.rglob('*') if path.is_file() and path != guard.REPORT}
                pins = service.merge(pins, terminal)
                guard.verify(pins)
                report.update(status='bounded-output-ownership-passed-awaiting-actual-work-and-full-review',
                    finished_at=guard.now(), profiles=rows, pins=pins, pins_unchanged_at_end=True,
                    full_manifests_sealed=0, mandatory_pending_gates=proposal['mandatory_pending_gates'])
                guard.save(guard.REPORT, report)
            except BaseException as error:
                report.update(status='stopped-unqualified', finished_at=guard.now(), active=None,
                    error=str(error) or type(error).__name__, error_type=type(error).__name__)
                guard.save(guard.REPORT, report)
                raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Clojure preparation interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--prepare', action='store_true')
    parser.add_argument('--proposal-file', type=Path, required=True)
    parser.add_argument('--proposal-sha256')
    parser.add_argument('--revision')
    args = parser.parse_args()
    proposal = args.proposal_file.resolve()
    assert proposal.is_relative_to(HERE / 'proposals') and proposal.suffix == '.json'
    if args.seal:
        seal(proposal)
    else:
        assert args.proposal_sha256
        prepare(proposal, args.proposal_sha256, args.revision)


if __name__ == '__main__':
    main()
