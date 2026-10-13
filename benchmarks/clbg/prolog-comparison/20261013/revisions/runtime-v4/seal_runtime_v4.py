"""Additive fixture-host correction; original v2 parser/bootstrap/tests stay exact.

Unexecuted. Root owns the single durable launch and common mutex. No runtime
download, disk attachment, native tool, benchmark, retry or implicit resume is
available here. An existing bundled Python hosts the same six frozen fixtures.
"""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import traceback

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parents[1]
ORIGINAL = HERE / 'revisions/runtime-v2'
DIAGNOSTIC = HERE / 'revisions/runtime-v3'
FAILED = HERE / 'proposal-seals/runtime-v3'
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON_ROOT = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python')
PYTHON = PYTHON_ROOT / 'bin/python3.12'
FAILED_REPORT_SHA = '5f301491fe64b8872e6737ef1dfd945e99a9dddc49c9a73ad44e8ca1b89423b6'
FAILED_RECEIPT_SHA = '4ad5eb929a0377ab9004864c4a94562ee76336c1cc05ccb7ebd7714c98c30b06'
FAILED_DISCOVERY_SHA = 'd4c2daf2e3b85beeb34af65bda7b3d086c0940edf5d25aad85ec3885a925ce89'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, digest in pins.items():
        assert sha(path) == digest, ('Pinned bytes changed', path)


def retain(pins, path, digest=None):
    path = Path(path).resolve(strict=True)
    actual = sha(path)
    assert digest is None or actual == digest, ('Unexpected prior bytes', path)
    assert str(path) not in pins or pins[str(path)] == actual
    pins[str(path)] = actual
    return actual


def fresh(path, name, pins):
    path = Path(path).resolve(strict=True)
    assert name not in sys.modules and sha(path) == pins[str(path)]
    source = path.read_bytes()
    assert hashlib.sha256(source).hexdigest() == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(source, str(path), 'exec'), module.__dict__)
    assert module is sys.modules[name] and Path(module.__file__).resolve() == path
    assert sha(path) == pins[str(path)]
    return module


def prior_failure(pins):
    report_path, receipt_path = FAILED/'report.json', FAILED/'fixture-receipt.json'
    discovery_path, terminal_path = FAILED/'actual-loader-discovery.json', FAILED/'terminal-pin.json'
    retain(pins, report_path, FAILED_REPORT_SHA)
    retain(pins, receipt_path, FAILED_RECEIPT_SHA)
    retain(pins, discovery_path, FAILED_DISCOVERY_SHA)
    retain(pins, terminal_path)
    report, receipt, discovery, terminal = [json.loads(path.read_text())
        for path in (report_path, receipt_path, discovery_path, terminal_path)]
    assert report['status'] == terminal['status'] == 'stopped-unqualified'
    assert terminal['report'] == str(report_path) and terminal['report_sha256'] == FAILED_REPORT_SHA
    assert terminal['partial_evidence_pins'] == report['pins']
    assert report['terminal_evidence_complete'] is terminal['inherited_common_mutex'] is True
    assert report['chart_admission'] is report['runtime_configuration_admitted'] is report['source_work_qualified'] is False
    assert receipt['status'] == 'failed' and receipt['execution_completed'] is False
    assert receipt['actual_test_ids'] == [] and type(receipt['tests_run']) is int and receipt['tests_run'] == 0
    assert receipt['discovered_test_ids'] == discovery['discovered_test_ids'] == ['unittest.loader._FailedTest.test_image_closure_v2']
    assert type(discovery['discovered_tests']) is int and discovery['discovered_tests'] == 1
    assert len(discovery['loader_errors']) == len(discovery['failed_loader_tests']) == 1
    failed = discovery['failed_loader_tests'][0]
    assert failed['exception_type'] == 'ImportError'
    for literal in ('pyexpat.cpython-314-darwin.so', '_XML_SetAllocTrackerActivationThreshold',
                    '/usr/lib/libexpat.1.dylib'):
        assert literal in failed['exception'] and literal in discovery['loader_errors'][0]
    for path, digest in report['pins'].items():
        retain(pins, path, digest)
    return dict(report=str(report_path), report_sha256=FAILED_REPORT_SHA,
        fixture_receipt=str(receipt_path), fixture_receipt_sha256=FAILED_RECEIPT_SHA,
        discovery=str(discovery_path), discovery_sha256=FAILED_DISCOVERY_SHA,
        stopped_before_actual_fixture_start=True, actual_started_tests=0,
        cause='Homebrew pyexpat extension/system Expat symbol mismatch')


def python_host(pins):
    assert Path(sys.executable).resolve(strict=True) == PYTHON.resolve(strict=True)
    assert sys.version_info[:3] == (3, 12, 14)
    assert Path(sys.base_prefix).resolve() == PYTHON_ROOT.resolve()
    assert sys.flags.ignore_environment == 1 and sys.flags.no_site == 1 and sys.dont_write_bytecode is True
    overrides = {key: value for key, value in os.environ.items() if key.startswith('DYLD_') and value}
    assert overrides == {}, ('Unselected fixture-host loader overrides', sorted(overrides))
    stdlib = PYTHON_ROOT / 'lib/python3.12'
    for path in [PYTHON, PYTHON_ROOT/'lib/libpython3.12.dylib',
            stdlib/'_sysconfigdata__darwin_darwin.py', stdlib/'config-3.12-darwin/Makefile',
            stdlib/'plistlib.py', stdlib/'xml/__init__.py', stdlib/'xml/parsers/__init__.py',
            stdlib/'xml/parsers/expat.py']:
        retain(pins, path)
    assert 'pyexpat' in sys.builtin_module_names
    assert importlib.util.find_spec('pyexpat').origin == 'built-in'
    verify(pins)
    return dict(executable=str(PYTHON.resolve()), executable_sha256=pins[str(PYTHON.resolve())],
        version=sys.version, base_prefix=str(Path(sys.base_prefix).resolve()),
        pyexpat_builtin_before_fixture_import=True, pyexpat_spec_origin='built-in',
        ignore_python_environment=True, site_startup_disabled=True,
        dyld_overrides=overrides, sys_path=list(sys.path),
        separately_downloaded_runtime=False, source_metadata_is_not_loaded_image_proof=True)


def terminal(output, report, pins):
    partial, errors = dict(pins), []
    for path in sorted(output.rglob('*')):
        if path.is_file() and path.name not in {'report.json', 'terminal-pin.json'}:
            try:
                partial[str(path.resolve())] = sha(path)
            except BaseException as failure:
                errors.append(dict(path=str(path), error=str(failure) or type(failure).__name__))
    for path, digest in pins.items():
        try:
            assert sha(path) == digest, ('Pinned bytes changed', path)
        except BaseException as failure:
            errors.append(dict(path=path, error=str(failure) or type(failure).__name__))
    if errors:
        report.update(preceding_status=report['status'], status='failed-terminal-integrity',
                      terminal_artifact_errors=errors)
    report.update(pins=partial, terminal_evidence_complete=not errors)
    save(output/'report.json', report)
    save(output/'terminal-pin.json', dict(status=report['status'], report=str(output/'report.json'),
        report_sha256=sha(output/'report.json'), partial_evidence_pins=partial,
        inherited_common_mutex=True, chart_admission=False, full_processes_authorized=0,
        runtime_configuration_admitted=False, qualification_stage='runtime-v4'))
    if errors:
        raise RuntimeError('Fixture-host terminal integrity failed; partial pins preserved')


def seal(args):
    output = HERE/'proposal-seals/runtime-v4'
    proposal, compatibility = HERE/'proposals/runtime-v4.json', HERE/'proposals/runtime-v2.json'
    assert not output.exists() and not proposal.exists() and not compatibility.exists(), 'Exclusive new stage; no retry/resume/replacement'
    output.mkdir(parents=True)
    pins = {}
    report = dict(status='sealing', qualification_stage='runtime-v4', inherited_common_mutex=True,
        chart_admission=False, runtime_configuration_admitted=False, source_work_qualified=False,
        full_processes_authorized=0, native_processes_authorized=0,
        runtime_processes_authorized=0, download_processes_authorized=0,
        actual_native_subjects=0, automatic_retry=False)
    error = None
    try:
        for path in sorted(PACKET.glob('*.py')) + sorted(PACKET.glob('*.md')):
            retain(pins, path)
        report['prior_v3_failure'] = prior_failure(pins)
        report['fixture_python_host'] = python_host(pins)
        diagnostic = fresh(DIAGNOSTIC/'diagnose_fixture_loader_v3.py', 'prolog_runtime_v4_prior_diagnostic', pins)
        report['prior_v2_failure'] = diagnostic.prior_failure(pins)
        boot = fresh(ORIGINAL/'bootstrap_runtime_v2.py', 'prolog_runtime_v4_frozen_bootstrap', pins)
        assert boot.PACKET == ORIGINAL and boot.HERE == HERE
        preparation, original, pins = boot.load_preparation(args, pins)
        service, references, guard, harness, pins = boot.reused_service(preparation, original, pins)
        runner = fresh(DIAGNOSTIC/'fixture_runner_v3.py', 'prolog_runtime_v4_retained_runner', pins)
        assert runner.IDS == boot.RUNTIME_IDS
        actual = runner.run(output, pins, sha, verify, save, ORIGINAL)
        report['local_fixture_receipt'] = actual
        boot.runtime_fixtures(actual)
        assert actual['loader_errors'] == actual['failed_loader_tests'] == []
        expat = sys.modules['pyexpat']
        assert expat.__spec__.origin == 'built-in' and getattr(expat, '__file__', None) is None
        for name, relative in [('plistlib', 'plistlib.py'), ('xml', 'xml/__init__.py'),
                ('xml.parsers', 'xml/parsers/__init__.py'), ('xml.parsers.expat', 'xml/parsers/expat.py')]:
            path = Path(sys.modules[name].__file__).resolve()
            assert path == (PYTHON_ROOT/'lib/python3.12'/relative).resolve()
            assert sha(path) == pins[str(path)]
        report['fixture_python_host'].update(actual_pyexpat_builtin_after_fixtures=True,
            actual_expat_version=expat.EXPAT_VERSION, actual_pyexpat_spec_origin=expat.__spec__.origin)
        verify(pins)
        report.update(status='sealed-runtime-bootstrap-awaiting-root-review',
            prolog_proposal=str(HERE/'proposals/private-v1.json'), prolog_proposal_sha256=args.proposal_sha256,
            retained_prolog_fixture_count=6, retained_generic_fixture_count=11,
            retained_runtime_fixture_count=6, publisher_dmg_url=boot.DMG_URL,
            publisher_dmg_sha256=boot.DMG_SHA, original_bootstrap=str(ORIGINAL/'bootstrap_runtime_v2.py'),
            correction='existing bundled Python host with builtin pyexpat; exact original v2 fixtures/parser/bootstrap',
            frozen_bootstrap_metadata_path=str(compatibility),
            metadata_path_role='new byte-identical v4 proposal alias at original bootstrap expected path; v2 failed seal untouched')
    except BaseException as failure:
        error = failure
        report.update(status='stopped-unqualified', error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
        try:
            with (output/'actual-error.log').open('x') as stream:
                stream.write(traceback.format_exc())
        except BaseException as log_error:
            report['error_log_write_failure'] = str(log_error) or type(log_error).__name__
    finally:
        if (output/'fixture-receipt.json').is_file() and 'local_fixture_receipt' not in report:
            try:
                report['local_fixture_receipt'] = json.loads((output/'fixture-receipt.json').read_text())
            except BaseException as failure:
                report['fixture_receipt_read_failure'] = str(failure) or type(failure).__name__
        terminal(output, report, pins)
    if error is not None:
        raise error
    assert report['status'] == 'sealed-runtime-bootstrap-awaiting-root-review' and report['terminal_evidence_complete'] is True
    publication = dict(status='failed', report=str(output/'report.json'),
        actual_proposal_pins={}, qualification_stage='runtime-v4',
        automatic_retry=False, runtime_configuration_admitted=False)
    try:
        verify(report['pins'])
        save(proposal, report)
        publication['actual_proposal_pins'][str(proposal)] = sha(proposal)
        save(compatibility, report)
        publication['actual_proposal_pins'][str(compatibility)] = sha(compatibility)
        assert proposal.read_bytes() == compatibility.read_bytes()
        verify(report['pins'])
        verify(publication['actual_proposal_pins'])
        publication['status'] = 'completed'
    except BaseException as failure:
        publication.update(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
        raise
    finally:
        save(output/'metadata-publication-outcome.json', publication)
        save(output/'metadata-publication-terminal.json', dict(status=publication['status'],
            outcome=str(output/'metadata-publication-outcome.json'),
            outcome_sha256=sha(output/'metadata-publication-outcome.json'),
            actual_proposal_pins=publication['actual_proposal_pins'],
            success_requires_actual_command_exit_zero=True, runtime_configuration_admitted=False))
    print(json.dumps(dict(proposal=str(proposal), sha256=sha(proposal),
        original_bootstrap_metadata=str(compatibility), original_bootstrap_metadata_sha256=sha(compatibility),
        status=report['status'], qualification_stage='runtime-v4', runtime_configuration_admitted=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--seal', required=True, action='store_true')
    parser.add_argument('--proposal-sha256', required=True)
    args = parser.parse_args()
    print('WAIT common mutex: same six frozen runtime fixtures on selected existing Python host', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        held, named = os.fstat(lock.fileno()), LOCK.stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        seal(args)


if __name__ == '__main__':
    main()
