"""Source-only Gleam candidate: guarded metadata seal and BEAM qualification.

No command has been run. Root review precedes the metadata seal; its exact
digest precedes bootstrap. This emits no full timing or chart admission.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import io
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
OTP = Path('/opt/homebrew/Cellar/erlang/29.1.1_2')
ERL = OTP / 'bin/erl'
ERLC = OTP / 'bin/erlc'
OTP_LIB = OTP / 'lib/erlang'
ENGINE = OTP_LIB / 'erts-17.1/bin/beam.smp'
ERLEXEC = OTP_LIB / 'erts-17.1/bin/erlexec'
GLEAM_VERSION = '1.19.0'
GLEAM_COMMIT = '904f81cc85f3bdc03fc3d0a6695055ce7ce4a64c'
GLEAM_URL = 'https://github.com/gleam-lang/gleam/releases/download/v1.19.0/gleam-v1.19.0-aarch64-apple-darwin.tar.gz'
GLEAM_SHA = '7abff14c1acc7eb725ef4883539c6b48de72b58593b331cac0ff23e0a5f068c7'
STDLIB_VERSION = '1.0.5'
STDLIB_RELEASE = 'https://hex.pm/api/packages/gleam_stdlib/releases/1.0.5'
STDLIB_URL = 'https://repo.hex.pm/tarballs/gleam_stdlib-1.0.5.tar'
BENCHMARKS = {'binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
              'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux'}
RUNTIME_FLAGS = ['+S', '8:8', '+A', '1', '+SDcpu', '1:1', '+SDio', '1']


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def load_service():
    # Only generic audited ownership/probe/image/pinning functions are called.
    # No C3 bootstrap, source controller, sealer or full runner is invoked.
    spec = importlib.util.spec_from_file_location('gleam_generic_owned_native_service', SERVICE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def profiles(value):
    rows = value['profiles']
    assert value['language'] == 'gleam' and value['compiler_version'] == GLEAM_VERSION
    assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == {
        (benchmark, variant) for benchmark in BENCHMARKS for variant in ('compact', 'optimized')}
    for row in rows:
        bundle = row['compiled_source_bundle']
        assert bundle and bundle[0] == row['source'] and len(bundle) == len(set(bundle))
        assert type(row['maximum_compute_workers']) is int and 1 <= row['maximum_compute_workers'] <= 8
        assert row['loc'] is None and row['eligible_for_chart'] is False
        destinations = ['benchmark.gleam' if i == 0 else Path(name).name for i, name in enumerate(bundle)]
        assert len(destinations) == len(set(destinations)), 'Compiler input destination collision'
        assert {Path(name).stem for name in bundle if name.endswith('.gleam')}.isdisjoint(
            {Path(name).stem for name in bundle if name.endswith('.erl')}), 'Gleam/FFI module collision'
        for name in bundle:
            path = Path(name)
            assert not path.is_absolute() and '..' not in path.parts
            assert path.suffix in ('.gleam', '.erl')
            assert (HERE / path).is_file()
    return rows


def static_paths():
    prior=json.loads((HERE/'proposals/private-v1.json').read_text())
    additions=set(REVISION_SOURCE.glob('*.py')) | set(REVISION_SOURCE.glob('*.md')) | {REVISION_SOURCE/'profiles.json'} | set((REVISION_SOURCE/'sources').glob('*/*.gleam'))
    return sorted({Path(p) for p in prior['snapshot_paths']} | additions)


def fetch(url, destination, maximum_bytes=32 * 1024 * 1024):
    assert not destination.exists()
    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(url, headers={'User-Agent': 'mech-local-benchmark-preparation'})
    with urllib.request.urlopen(request, timeout=60) as response, destination.open('xb') as stream:
        total = 0
        while chunk := response.read(65536):
            total += len(chunk)
            assert total <= maximum_bytes, 'Unexpected archive/metadata size'
            stream.write(chunk)


def extract(source, destination):
    assert not destination.exists()
    destination.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(source), mode='r:*') as archive:
        names = []
        for member in archive.getmembers():
            name = Path(member.name)
            assert not name.is_absolute() and '..' not in name.parts
            assert member.isdir() or member.isfile(), 'No archive link/device may become an executable input'
            assert member.name not in names
            names.append(member.name)
        archive.extractall(destination, filter='data')


def hex_contents(source):
    with tarfile.open(fileobj=io.BytesIO(source), mode='r:') as archive:
        members = archive.getmembers()
        assert {member.name for member in members} == {'VERSION', 'CHECKSUM', 'metadata.config', 'contents.tar.gz'}
        assert len(members) == 4 and all(member.isfile() for member in members)
        values = {member.name: archive.extractfile(member).read() for member in members}
        assert values['VERSION'] == b'3'
        inner = hashlib.sha256(values['VERSION'] + values['metadata.config'] + values['contents.tar.gz']).hexdigest()
        assert values['CHECKSUM'].decode().lower() == inner
        return values['contents.tar.gz']


def compile_command(compiler, package, output, libraries):
    return [str(compiler), 'compile-package', '--target', 'erlang', '--no-dev',
            '--package', str(package), '--out', str(output), '--lib', str(libraries)]


def runtime_command(application, libraries, n='{n}'):
    return [str(ERL), *RUNTIME_FLAGS, '-noshell', '-noinput', '-pa', str(application / 'ebin'),
            str(libraries / 'gleam_stdlib/ebin'), '-s', 'clbg_runner', 'main', '-extra', str(n)]


def inputs(benchmark, references):
    cases = load_service().inputs(benchmark, references)
    if benchmark == 'revcomp':
        cases += (('preheader-ignored', b'ACGT\n>one\nA\n'),)
    return cases


def preflight_inputs(references):
    service = load_service()
    inventory = {}
    for benchmark in sorted(BENCHMARKS):
        cases = [('n-' + str(n), None) for n in service.NUMERIC[benchmark]] if benchmark in service.NUMERIC else list(inputs(benchmark, references))
        labels = [name for name, _ in cases]
        assert labels and len(labels) == len(set(labels))
        assert all(name and name.replace('-', '').replace('_', '').isalnum() for name in labels)
        inventory[benchmark] = labels
    assert 2 * sum(map(len, inventory.values())) == 156
    return inventory


def seal(path):
    print('WAIT LOCK: Gleam source/preparation metadata seal', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not path.exists()
        service = load_service()
        guard = service.load_guard()
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        import references
        import fixture_runner
        import proof_reuse
        prior_path=HERE/'proposals/private-v1.json'
        assert sha(prior_path)==proof_reuse.PRIOR_SHA
        prior=json.loads(prior_path.read_text())
        guard.verify(prior['pins'])
        rows = profiles(json.loads((REVISION_SOURCE / 'profiles.json').read_text()))
        inventory = preflight_inputs(references)
        output = HERE / 'proposal-seals' / path.stem
        assert not output.exists()
        output.mkdir(parents=True)
        service_receipt = output / 'generic-ownership-inventory.json'
        service.owned_command(guard, harness, [sys.executable, '-B', str(SERVICE_FIXTURES),
            '--set', 'ownership', '--receipt', str(service_receipt)],
            dict(os.environ, PYTHONPATH=str(SERVICE.parent)), output, lock.fileno(), 30, 'generic-ownership-fixtures')
        service_fixtures = importlib.util.spec_from_file_location('gleam_generic_fixture_inventory', SERVICE_FIXTURES)
        module = importlib.util.module_from_spec(service_fixtures)
        service_fixtures.loader.exec_module(module)
        module.validate_receipt(json.loads(service_receipt.read_text()), 'ownership')
        local_receipt = output / 'gleam-bundle-inventory.json'
        service.owned_command(guard, harness, [sys.executable, '-B', str(REVISION_SOURCE / 'fixture_runner.py'),
            '--receipt', str(local_receipt)], dict(os.environ, PYTHONPATH=str(REVISION_SOURCE)),
            output, lock.fileno(), 30, 'gleam-bundle-fixtures')
        fixture_runner.validate_receipt(json.loads(local_receipt.read_text()))
        reused=proof_reuse.reconstruct(sys.modules[__name__],service,guard)
        reuse_path=output/'prior-app-reuse.json'
        new_json(reuse_path,reused)
        release=Path(prior['stdlib_publisher_metadata'])
        metadata=json.loads(release.read_text())
        assert metadata['version']==STDLIB_VERSION and not metadata.get('retirement') and metadata['requirements']=={}
        checksum=metadata['checksum'].lower()
        assert checksum==prior['stdlib_archive_sha256']
        assert (OTP_LIB / 'releases/29/OTP_VERSION').read_text().strip() == '29.1.1'
        required = [ERL, ERLC, ERLEXEC, ENGINE, OTP_LIB / 'releases/29/OTP_VERSION',
                    OTP / 'INSTALL_RECEIPT.json', service.GUARD, service.SUITE / 'run.py', service.SUITE / 'prepare.py']
        service_paths = set(SERVICE.parent.glob('*.py')) | set(SERVICE.parent.glob('*.md'))
        pins = service.merge(prior['pins'],reused['pins'],{str(prior_path):proof_reuse.PRIOR_SHA}, {str(p): sha(p) for p in static_paths() + required},
            {str(p.resolve()): sha(p) for p in required}, service.pin_tree(output),
            {str(p): sha(p) for p in service_paths}, service.pin_tree(OTP_LIB))
        proposal = dict(status='sealed-awaiting-root-review', language='gleam', created_at=guard.now(),
            prior_app_reuse=str(reuse_path),prior_app_reuse_sha256=sha(reuse_path),
            reused_bounded_cases=4,reused_profile_ids=['gleam-binarytrees-compact'],fresh_profile_builds_required=19,
            fresh_bounded_attempts_required=152,toolchain_binding=reused['toolchain'],
            source_repairs='Six expression grouping pairs in five sources; scoped compiler-input list renamed to avoid shadowing inputs function',
            profiles=rows, snapshot_paths=[str(p) for p in static_paths() if p.is_relative_to(HERE)], pins=pins,
            compiler_version=GLEAM_VERSION, compiler_commit=GLEAM_COMMIT,
            compiler_archive_url=GLEAM_URL, compiler_archive_sha256=GLEAM_SHA,
            stdlib_version=STDLIB_VERSION, stdlib_archive_url=STDLIB_URL,
            stdlib_archive_sha256=checksum, stdlib_publisher_metadata=str(release),
            case_inventory=inventory, actual_fixture_receipts=[str(service_receipt), str(local_receipt)],
            exact_actual_fixture_methods=19, fixture_skips=0, complete_bounded_outputs=156,
            backend='Erlang abstract forms to BEAM on native ARM64 OTP load-time JIT',
            runtime_flags=RUNTIME_FLAGS, shared_timing_lock=str(LOCK), guard_sha256=service.GUARD_SHA,
            full_processes_authorized=0, chart_admission=False, no_automatic_retry=True,
            required_pending_gates=['actual compiler/runtime/BEAM and complete bounded qualification',
                'actual tree allocation/retention; seven maps/windows; both Pi operands; strict numeric recurrence',
                'canonical authored Gleam/Erlang LOC', 'independently reviewed exact full-study index'])
        new_json(path, proposal)
        print(json.dumps(dict(proposal=str(path), sha256=sha(path)), indent=2), flush=True)


def run(path, digest, revision):
    print('WAIT LOCK: Gleam exact BEAM bounded preparation ' + revision, flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert sha(path) == digest
        proposal = json.loads(path.read_text())
        assert proposal['status'] == 'sealed-awaiting-root-review'
        service = load_service()
        guard = service.load_guard()
        guard.verify(proposal['pins'])
        harness = guard.load_harness()
        harness.install_interrupt_handlers()
        import references
        import proof_reuse
        reuse_path=Path(proposal['prior_app_reuse'])
        assert sha(reuse_path)==proposal['prior_app_reuse_sha256']
        reuse=json.loads(reuse_path.read_text())
        proof_reuse.validate_packet(reuse,sys.modules[__name__],service,guard)
        assert preflight_inputs(references) == proposal['case_inventory']
        output = HERE / 'qualification' / revision
        build = HERE / '.build' / revision
        assert not output.exists() and not build.exists()
        output.mkdir(parents=True)
        build.mkdir(parents=True)
        guard.OUT, guard.REPORT = output, output / 'report.json'
        report = dict(status='preparing', language='gleam', started_at=guard.now(), active=None,
                      attempts=[], profiles=[], full_performance_runs=0, chart_admission=False,
                      no_automatic_retry=True, proposal=str(path), proposal_sha256=digest,
                      prior_bounded_proof_reuse=str(reuse_path),prior_bounded_proof_reuse_sha256=proposal['prior_app_reuse_sha256'],
                      reused_bounded_cases=4,reused_cases_relabelled_as_attempts=False)
        guard.save(guard.REPORT, report)
        try:
            snapshot = output / 'source-snapshot'
            for name in proposal['snapshot_paths']:
                source = Path(name)
                target = snapshot / source.relative_to(HERE)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)
                assert sha(target) == proposal['pins'][name]
            binding=proposal['toolchain_binding']
            assert binding==reuse['toolchain']
            compiler=Path(binding['compiler']);libraries=Path(binding['libraries'])
            environment=dict(reuse['profile']['runtime_environment'],ERL_CRASH_DUMP=str(output/'erl_crash.dump'))
            metadata=reuse['actual_toolchain']
            new_json(build/'toolchain-reuse.json',dict(binding,source='exact unchanged verified private-v1 compiler/stdlib/version/runtime receipts'))
            pins=service.merge(proposal['pins'],reuse['pins'],{str(path):digest},service.pin_tree(snapshot),service.pin_tree(build))
            fresh_builds=0
            for declared in proposal['profiles']:
                if declared['id']=='gleam-binarytrees-compact':
                    row=dict(reuse['profile'])
                    assert row['compiled_source_bundle']==declared['compiled_source_bundle'] and row['source_sha256']==sha(snapshot/declared['source'])
                    row.update(qualification_origin='exact-unchanged-private-v1-app-and-four-records',new_native_cases=0,prior_bounded_records=reuse['prior_records'])
                    report['profiles'].append(row)
                    guard.save(guard.REPORT,report)
                    continue
                fresh_builds+=1
                package = build / 'packages' / declared['id']
                src = package / 'src'
                src.mkdir(parents=True)
                compiled_inputs = []
                for index, name in enumerate(declared['compiled_source_bundle']):
                    source = snapshot / name
                    target = src / ('benchmark.gleam' if index == 0 else source.name)
                    shutil.copyfile(source, target)
                    assert sha(target) == sha(source)
                    compiled_inputs.append(dict(path=str(target), sha256=sha(target), origin=str(source),
                        role='application' if index == 0 else 'application-helper'))
                (package / 'gleam.toml').write_text('name = "benchmark"\nversion = "1.0.0"\ntarget = "erlang"\n[dependencies]\ngleam_stdlib = "== 1.0.5"\n')
                application = build / 'applications' / declared['id']
                command = compile_command(compiler, package, application, libraries)
                service.owned_command(guard, harness, command, environment, output / 'builds' / declared['id'],
                    lock.fileno(), 180, 'compile')
                assert (application / 'ebin/benchmark.beam').is_file() and (application / 'ebin/clbg_runner.beam').is_file()
                artifact_pins = service.pin_tree(application)
                pins = service.merge(pins, service.pin_tree(package), artifact_pins)
                receipt = output / 'builds' / declared['id'] / 'compiler-input-inventory.json'
                new_json(receipt, dict(status='compiled-exact-authored-input-inventory', profile=declared['id'],
                    source=compiled_inputs[0]['path'], source_sha256=compiled_inputs[0]['sha256'], source_bundle=compiled_inputs,
                    compiler=str(compiler), compiler_sha256=sha(compiler), build_command=command,
                    compiled_artifacts=artifact_pins, runtime_classpath=[str(application / 'ebin'), str(libraries / 'gleam_stdlib/ebin')]))
                pins[str(receipt)] = sha(receipt)
                row = dict(declared, key=declared['benchmark'] + ':' + declared['id'], language='gleam',
                    source=compiled_inputs[0]['path'], source_sha256=compiled_inputs[0]['sha256'], source_bundle=compiled_inputs,
                    binary_sha256=sha(ERL), support=service.merge({entry['path']:entry['sha256'] for entry in compiled_inputs[1:]},
                        artifact_pins, service.pin_tree(libraries), {str(ENGINE):sha(ENGINE), str(ERLEXEC):sha(ERLEXEC)}),
                    command=runtime_command(application, libraries), cwd=str(HERE),
                    runtime_environment=environment, compiler_version=GLEAM_VERSION, runtime_version='29.1.1',
                    measured_architecture='arm64', backend='BEAM load-time JIT',
                    compiled_input_inventory=str(receipt), compiled_input_inventory_sha256=sha(receipt),
                    compiled_artifacts=artifact_pins, runtime_classpath=[str(application / 'ebin'), str(libraries / 'gleam_stdlib/ebin')])
                for n in service.NUMERIC.get(row['benchmark'], ()):
                    service.probe(guard, harness, row, n, None, getattr(references, row['benchmark'])(n),
                        pins, output, lock.fileno(), report, 'n-' + str(n))
                if row['benchmark'] not in service.NUMERIC:
                    for case, data in inputs(row['benchmark'], references):
                        service.probe(guard, harness, row, harness.WORK[row['benchmark']][0], data,
                            getattr(references, row['benchmark'])(data), pins, output, lock.fileno(), report, case)
                report['profiles'].append(row)
                guard.save(guard.REPORT, report)
            assert len(report['profiles']) == 20 and fresh_builds==proposal['fresh_profile_builds_required']==19
            assert len(report['attempts'])==proposal['fresh_bounded_attempts_required']==152 and report['reused_bounded_cases']==4
            pins = service.merge(pins, {str(p):sha(p) for p in output.rglob('*') if p.is_file() and p != guard.REPORT})
            guard.verify(pins)
            report.update(status='bounded-output-ownership-passed-awaiting-work-and-LOC-review', finished_at=guard.now(),
                          pins=pins, actual_toolchain=metadata, compiler=str(compiler), build=str(build), libraries=str(libraries),
                          fresh_profile_builds=fresh_builds,full_manifests_sealed=0)
            guard.save(guard.REPORT, report)
        except BaseException as error:
            report.update(status='stopped-unqualified', active=None, finished_at=guard.now(),
                          error=str(error) or type(error).__name__, error_type=type(error).__name__)
            guard.save(guard.REPORT, report)
            raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Gleam preparation interrupted: ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--run', action='store_true')
    parser.add_argument('--proposal-file', required=True, type=Path)
    parser.add_argument('--proposal-sha256')
    parser.add_argument('--revision')
    args = parser.parse_args()
    path = args.proposal_file.resolve()
    assert path.is_relative_to(HERE) and path.suffix == '.json'
    if args.seal:
        seal(path)
    else:
        assert args.proposal_sha256 and args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
        run(path, args.proposal_sha256, args.revision)


if __name__ == '__main__':
    main()
