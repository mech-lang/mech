"""Unexecuted formatter-only measurement entry; root supplies a reviewed plan.

Every import of authored code, hash, formatter process and lexical count occurs
inside the shared mutex. This entry does not run scored applications or publish
data. Originals are read-only; each invocation receives its own copied source.
"""
import argparse
from contextlib import contextmanager
import copy
import fcntl
import hashlib
import importlib.abc
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import traceback
from types import SimpleNamespace

PACKET = Path(__file__).resolve().parent
CLBG = PACKET.parents[2]
LOCK = Path('/tmp/mech-clbg-timing.lock')
SERVICE = CLBG / 'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_PROPOSAL = CLBG / 'c3-comparison/20261012/proposals/private-v1.json'
SERVICE_SHA = '43438603d675472393050d05928e488a20f16040493935aca676007fc0ada050'
OWNERSHIP_IDS = [
    'test_guarded_prepare.OwnershipChecks.test_popen_failure_has_no_foreign_cleanup',
    'test_guarded_prepare.OwnershipChecks.test_identity_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_journal_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_timeout_preserves_complete_drained_output',
    'test_guarded_prepare.OwnershipChecks.test_orphan_compiler_child_is_fatal',
    'test_guarded_prepare.OwnershipChecks.test_cleanup_failure_is_fatal_and_waits',
    'test_guarded_prepare.OwnershipChecks.test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group',
    'test_guarded_prepare.ProbeRestorationChecks.test_setup_failure_restores_present_state',
    'test_guarded_prepare.ProbeRestorationChecks.test_absent_input_key_is_removed_on_failure',
    'test_guarded_prepare.ProbeRestorationChecks.test_parent_failure_archives_actual_sampler_bytes',
    'test_guarded_prepare.ProbeRestorationChecks.test_trace_failure_archives_and_stops',
]
STATUSES = {'qualified', 'unavailable', 'pending', 'failed'}
DEFINITION = 'nonblank-noncomment-physical-lines-v1'


def require(value, message):
    if not value:
        raise ValueError(message)


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
        require(sha(path) == digest, 'Pinned bytes changed: ' + path)


def held_mutex(fd):
    actual, named = os.fstat(fd), LOCK.stat()
    require((actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino), 'Common mutex inode differs')


def merge(pins, extra):
    for path, digest in extra.items():
        require(path not in pins or pins[path] == digest, 'Conflicting pin: ' + path)
        pins[path] = digest


class ColdLoader(importlib.abc.Loader):
    def __init__(self, path, pins, observations):
        self.path, self.pins, self.observations = path, pins, observations

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        raw = self.path.read_bytes()
        require(hashlib.sha256(raw).hexdigest() == self.pins[str(self.path)], 'Cold import source changed')
        exec(compile(raw, str(self.path), 'exec'), module.__dict__)
        require(Path(module.__file__).resolve() == self.path, 'Cold import module path changed')
        require(sha(self.path) == self.pins[str(self.path)], 'Cold import source changed after execution')
        self.observations.append(dict(module=module.__name__, path=str(self.path), sha256=self.pins[str(self.path)]))


def standard_origin(name, path, stdlib_root, standard_names):
    """Permit only the selected Python's genuine standard-library namespace."""
    path, stdlib_root = Path(path), Path(stdlib_root)
    return (name.split('.')[0] in standard_names and path.is_relative_to(stdlib_root) and
            not any(part in ('site-packages', 'dist-packages') for part in path.relative_to(stdlib_root).parts))


@contextmanager
def cold_sources(pins, modules, observations):
    """Use exact .py sources, never sibling pyc, for reviewed authored modules."""
    spec_factory = importlib.util.spec_from_file_location
    roots = {CLBG.resolve()}
    roots.update(Path(entry['root']).resolve() for entry in modules.values() if 'root' in entry)
    stdlib_root = Path(os.__file__).resolve().parent
    standard_names = set(sys.stdlib_module_names)
    def record_standard(name, origin, phase):
        require(standard_origin(name, origin, stdlib_root, standard_names),
                'Unrecorded external import origin: ' + str(origin))
        actual = sha(origin)
        merge(pins, {str(origin): actual})
        observations.append(dict(module=name, path=str(origin), sha256=actual,
                                 origin='selected-python-standard-library', phase=phase))
    for name, loaded in list(sys.modules.items()):
        filename = getattr(loaded, '__file__', None)
        if filename:
            origin = Path(filename).resolve()
            if origin == Path(__file__).resolve():
                continue
            require(not any(origin.is_relative_to(root) for root in roots), 'Prior cached reviewed-code module')
            if name in modules:
                require(Path(modules[name]['path']).resolve() == origin and
                        modules[name]['sha256'] == pins.get(str(origin)), 'Declared cached module origin differs')
            record_standard(name, origin, 'already-loaded-standard-bootstrap')
    paths = {Path(path).resolve(): digest for path, digest in pins.items() if Path(path).suffix == '.py'}

    def source_spec(name, location, *, loader=None, submodule_search_locations=None):
        path = Path(location).resolve()
        if path not in paths and str(path) not in pins:
            record_standard(name, path, 'before-import')
            if path.suffix == '.py':
                paths[path] = pins[str(path)]
        if path in paths:
            loader = ColdLoader(path, pins, observations)
        else:
            require(str(path) in pins and any(path.name.endswith(suffix)
                    for suffix in importlib.machinery.EXTENSION_SUFFIXES),
                    'Authored/vendor code requires pinned Python source or a native extension image')
            require(sha(path) == pins[str(path)], 'Pinned native import image changed')
            observations.append(dict(module=name, path=str(path), sha256=pins[str(path)],
                                     origin='pinned-native-extension', phase='before-import'))
        return spec_factory(name, location, loader=loader, submodule_search_locations=submodule_search_locations)

    class Finder(importlib.abc.MetaPathFinder):
        def find_spec(self, fullname, path=None, target=None):
            entry = modules.get(fullname)
            if entry is None:
                found = importlib.machinery.PathFinder.find_spec(fullname, path)
                if found is None or found.origin in (None, 'built-in', 'frozen'):
                    return None
                origin = Path(found.origin).resolve()
                return source_spec(fullname, origin,
                    submodule_search_locations=list(found.submodule_search_locations)
                    if found.submodule_search_locations is not None else None)
            filename = Path(entry['path']).resolve()
            require(pins.get(str(filename)) == entry['sha256'], 'Import catalogue pin mismatch')
            return source_spec(fullname, filename,
                submodule_search_locations=[str(filename.parent)] if filename.name == '__init__.py' else None)

    finder = Finder()
    importlib.util.spec_from_file_location = source_spec
    sys.meta_path.insert(0, finder)
    try:
        yield source_spec
    finally:
        sys.meta_path.remove(finder)
        importlib.util.spec_from_file_location = spec_factory


def fresh(path, name, source_spec):
    require(name not in sys.modules, 'Cached measurement helper')
    spec = source_spec(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def retained_service(pins, source_spec):
    require(sha(SERVICE_PROPOSAL) == SERVICE_SHA, 'Actual generic ownership proposal changed')
    proposal = json.loads(SERVICE_PROPOSAL.read_bytes())
    require(proposal['status'] == 'sealed-awaiting-root-approval', 'Generic ownership proposal not sealed')
    receipt_path = Path(proposal['actual_mock_fixture_receipt'])
    require(proposal['actual_mock_fixture_receipt_sha256'] == proposal['pins'][str(receipt_path)], 'Ownership receipt pin')
    # The caller has already merged/verified all retained Python and raw receipt pins.
    verify(pins)
    receipt = json.loads(receipt_path.read_bytes())
    require(receipt['status'] == 'passed' and receipt['inventory'] == 'ownership' and receipt['successful'] is True,
            'Actual ownership fixtures did not pass')
    require(receipt['expected_test_ids'] == OWNERSHIP_IDS, 'Ownership catalogue differs')
    for key in ('tests_run', 'discovered_tests'):
        require(type(receipt[key]) is int and receipt[key] == 11, 'Exact ownership count required')
    require(len(receipt['actual_test_ids']) == len(set(receipt['actual_test_ids'])) == 11 and
            set(receipt['actual_test_ids']) == set(OWNERSHIP_IDS), 'Actual ownership started IDs differ')
    require(all(receipt[key] == [] for key in ('skipped', 'failures', 'errors', 'expected_failures', 'unexpected_successes')),
            'Ownership outcomes incomplete')
    service = fresh(SERVICE, 'formatted_reviewed_service', source_spec)
    guard = service.load_guard()
    harness = guard.load_harness()
    for module in (guard, harness, harness.compact, harness.compact.common):
        filename = str(Path(module.__file__).resolve())
        require(filename in pins and sha(filename) == pins[filename], 'Ownership helper identity differs')
    harness.install_interrupt_handlers()
    verify(pins)
    return service, guard, harness


def successful(detail, command):
    require(detail['command'] == command and detail['inherited_common_mutex'] is True, 'Formatter command/mutex differs')
    require(type(detail['exit_status']) is int and detail['exit_status'] == 0 and
            type(detail['final_exit_status']) is int and detail['final_exit_status'] == 0, 'Formatter exited unsuccessfully')
    require(not detail.get('error') and not detail.get('cleanup_error') and not detail.get('cleanup_errors') and
            not detail.get('remaining_group_members'), 'Formatter integrity/ownership failure')


def owned(service, guard, harness, command, environment, output, lock_fd, timeout, name, cwd):
    """Only adapt fixed cwd; frozen service retains its complete lifecycle code."""
    original = service.subprocess
    held_mutex(lock_fd)
    def launch(argv, **kwargs):
        require(argv == command and kwargs['stdin'] == subprocess.DEVNULL and
                kwargs['start_new_session'] is True and kwargs['pass_fds'] == (lock_fd,), 'Service launch policy differs')
        kwargs['cwd'] = cwd
        return original.Popen(argv, **kwargs)
    service.subprocess = SimpleNamespace(**{name: getattr(original, name) for name in dir(original) if not name.startswith('__')})
    service.subprocess.Popen = launch
    try:
        result = service.owned_command(guard, harness, command, environment, output, lock_fd, timeout, name)
        successful(result['detail'], command)
        return result
    finally:
        service.subprocess = original


def physical(source, counter, loaded):
    require(counter['function'] in {'mask_source', 'mask_foreign'}, 'Only lexical masking is permitted')
    module = loaded[counter['module']]
    require(Path(module.__file__).resolve() == Path(counter['module_path']).resolve(), 'Actual lexical module path differs')
    function = getattr(module, counter['function'])
    result = function(source, counter['argument'])
    require(type(result) is dict and type(result['cleaned']) is str and len(result['cleaned']) == len(source), 'Lexer span changed')
    boundaries = '\n\r\v\f\x1c\x1d\x1e\x85\u2028\u2029'
    require(all(original == masked for original, masked in zip(source, result['cleaned'])
                if original in boundaries or masked in boundaries), 'Lexer physical row boundaries changed')
    require(type(result['diagnostics']) is list and not result['diagnostics'], 'Formatted source has unresolved lexical diagnostics')
    return sum(bool(row.strip()) for row in result['cleaned'].splitlines(keepends=True))


def command_for(recipe, tool, source, target):
    require(recipe['output_mode'] in {'in-place', 'file', 'stdout'}, 'Unknown formatter output mode')
    require(type(recipe['arguments']) is list and all(type(arg) is str for arg in recipe['arguments']), 'Formatter argv must be literal')
    require(any('{input}' in arg for arg in recipe['arguments']), 'Formatter must consume the isolated original copy')
    require(all('{' not in arg.replace('{input}', '').replace('{output}', '') and
                '}' not in arg.replace('{input}', '').replace('{output}', '') for arg in recipe['arguments']), 'Unknown argv substitution')
    return [tool] + [arg.replace('{input}', str(source)).replace('{output}', str(target)) for arg in recipe['arguments']]


def measure_file(item, recipe, counter, output, pins, service, guard, harness, lock_fd, loaded, versions):
    require(not Path(item['path']).is_symlink(), 'Scored author input is a symlink')
    original = Path(item['path']).resolve(strict=True)
    require(pins.get(str(original)) == item['sha256'], 'Scored source was not in the reviewed plan pin closure')
    require(not original.is_symlink() and sha(original) == item['sha256'], 'Scored source changed')
    result = copy.deepcopy(item)
    result.update(status=recipe['status'], physical_loc=None, formatter_evidence=None, counter_evidence=None)
    if recipe['status'] != 'qualified':
        require(recipe['status'] in {'unavailable', 'pending'} and recipe['reason'] and recipe['official_sources'],
                'Missing formatter requires explicit primary evidence/reason')
        result['reason'] = recipe['reason']
        return result
    require(recipe['provider'] == 'language-project' and recipe['official_sources'], 'Native formatter provenance missing')
    tool = Path(recipe['tool']['path']).resolve(strict=True)
    config = recipe['config']
    require(pins[str(tool)] == recipe['tool']['sha256'] and pins[config['path']] == config['sha256'], 'Formatter tool/config pin mismatch')
    require(pins[str(Path(counter['module_path']).resolve())] == counter['module_sha256'], 'Lexical source pin mismatch')
    output.mkdir(parents=True)
    source = output / original.name
    with source.open('xb') as stream:
        stream.write(original.read_bytes())
    require(sha(source) == item['sha256'], 'Formatter copy differs')
    target = source if recipe['output_mode'] == 'in-place' else output / ('formatted' + original.suffix)
    environment = dict(recipe['environment'])
    home = output / 'home'; home.mkdir()
    environment.update(HOME=str(home), XDG_CONFIG_HOME=str(home), LANG='C', LC_ALL='C')
    require(all(type(key) is str and type(value) is str for key, value in environment.items()), 'Invalid formatter environment')
    require(not any(key.startswith(('DYLD_', 'LD_', 'PYTHON', 'NODE_OPTIONS')) for key in environment), 'Unreviewed formatter environment injection')
    save(output / 'environment.json', dict(environment=environment, cwd=str(output)))
    version_key = (str(tool), recipe['tool']['sha256'], recipe['tool']['version'])
    if version_key not in versions:
        version_cmd = [str(tool)] + recipe['version_arguments']
        version = owned(service, guard, harness, version_cmd, environment, output / 'processes', lock_fd, 30, 'version', output)
        require(version['stdout'].decode('utf-8') == recipe['expected_version_stdout'] and
                version['stderr'].decode('utf-8') == recipe.get('expected_version_stderr', ''), 'Observed formatter version differs')
        version_file = output / 'version-evidence.json'
        save(version_file, dict(version=recipe['tool']['version'], stdout=version['stdout'].decode('utf-8'),
            stderr=version['stderr'].decode('utf-8'), tool=recipe['tool'],
            process_receipt=dict(path=str(output / 'processes/version.json'), sha256=sha(output / 'processes/version.json'))))
        versions[version_key] = dict(path=str(version_file), sha256=sha(version_file), binding_field='/version',
                                    binding=recipe['tool']['version'])
    command = command_for(recipe, str(tool), source, target)
    try:
        formatted = owned(service, guard, harness, command, environment, output / 'processes', lock_fd,
                          recipe['timeout_seconds'], 'format', output)
    except BaseException:
        receipt_path = output / 'processes/format.json'
        if not receipt_path.is_file():
            raise
        detail = json.loads(receipt_path.read_bytes())
        # Only a recorded formatter rejection with completely clean owned
        # cleanup may become a failed file. Integrity/timeouts/signals stop.
        clean_source_failure = (detail.get('command') == command and detail.get('inherited_common_mutex') is True and
            type(detail.get('exit_status')) is int and detail['exit_status'] > 0 and
            type(detail.get('final_exit_status')) is int and detail['final_exit_status'] == detail['exit_status'] and
            detail.get('error_type') == 'AssertionError' and not detail.get('cleanup_error') and
            not detail.get('cleanup_errors') and not harness.group_members(detail['pid']))
        if not clean_source_failure:
            raise
        require(sha(original) == item['sha256'], 'Formatter rejection changed scored original')
        result.update(status='failed', reason='Native formatter rejected this exact source; failed process bytes retained',
            failure_receipt=dict(path=str(receipt_path), sha256=sha(receipt_path)), ownership_verified=True)
        return result
    if recipe['output_mode'] == 'stdout':
        with target.open('xb') as stream:
            stream.write(formatted['stdout'])
    require(target.is_file() and not target.is_symlink(), 'Formatter output missing or redirected')
    raw = target.read_bytes()
    # Empty UTF-8 helpers can contribute zero lines to a positive whole bundle.
    require(b'\x00' not in raw, 'Formatter output is not source text')
    try:
        count = physical(raw.decode('utf-8'), counter, loaded)
    except ValueError as failure:
        require(sha(original) == item['sha256'], 'Lexical failure changed scored original')
        save(output / 'failed-count.json', dict(status='failed', error=str(failure), output_sha256=sha(target), counter=counter))
        result.update(status='failed', reason=str(failure),
            failure_receipt=dict(path=str(output / 'failed-count.json'), sha256=sha(output / 'failed-count.json')),
            ownership_verified=True)
        return result
    require(sha(original) == item['sha256'], 'Formatter changed scored original')
    formatter = dict(provider=recipe['provider'], official_sources=recipe['official_sources'], tool=recipe['tool'],
        config=config, input=dict(path=str(original), sha256=item['sha256']),
        output=dict(path=str(target), sha256=sha(target)),
        process_receipt=dict(path=str(output / 'processes/format.json'), sha256=sha(output / 'processes/format.json')),
        version_receipt=versions[version_key],
        source_preservation_verified=True, ownership_verified=True, no_application_execution=True)
    save(output / 'formatter-evidence.json', formatter)
    formatter['receipt'] = dict(path=str(output / 'formatter-evidence.json'), sha256=sha(output / 'formatter-evidence.json'),
                               binding_field='', binding=copy.deepcopy(formatter))
    counted = dict(definition=DEFINITION, module_path=counter['module_path'], module_sha256=counter['module_sha256'],
        function=counter['function'], argument=counter['argument'], diagnostics=[], no_semantic_normalization=True)
    count_binding = dict(counted, input_sha256=sha(target), physical_loc=count)
    save(output / 'physical-count.json', count_binding)
    counted['receipt'] = dict(path=str(output / 'physical-count.json'), sha256=sha(output / 'physical-count.json'),
                             binding_field='', binding=count_binding)
    result.update(status='qualified', physical_loc=count, formatter_evidence=formatter, counter_evidence=counted)
    target.chmod(0o444)
    return result


def pending_execution_file(item, pins):
    """Keep an unbound measured profile blank without invoking a formatter."""
    require(not Path(item['path']).is_symlink(), 'Scored author input is a symlink')
    original = Path(item['path']).resolve(strict=True)
    require(str(original) == item['path'] and pins.get(str(original)) == item['sha256'] and
            sha(original) == item['sha256'], 'Pending scored source identity differs')
    result = copy.deepcopy(item)
    result.update(status='pending', physical_loc=None, formatter_evidence=None, counter_evidence=None,
                  reason='Independent actual measured profile/source/executable/command image receipt is not bound')
    return result


def terminal(output, report, pins, body_error, lock_fd):
    partial = dict(pins); errors = []
    try:
        held_mutex(lock_fd)
    except BaseException as failure:
        errors.append(dict(path=str(LOCK), error=str(failure), error_type=type(failure).__name__))
    for path in sorted(output.rglob('*')):
        if path.is_file() and path.name not in {'qualification.json', 'terminal.json'}:
            try:
                merge(partial, {str(path.resolve()): sha(path)})
            except BaseException as failure:
                errors.append(dict(path=str(path), error=str(failure), error_type=type(failure).__name__))
    for path, digest in partial.items():
        try:
            require(sha(path) == digest, 'Pinned source/output changed')
        except BaseException as failure:
            errors.append(dict(path=path, error=str(failure), error_type=type(failure).__name__))
    if body_error or errors:
        report['status'] = 'failed-terminal-integrity' if errors else 'stopped-formatted-loc-measurement'
    report['pins'] = partial
    save(output / 'qualification.json', report)
    save(output / 'terminal.json', dict(status=report['status'], body_error=body_error, integrity_errors=errors,
        available_bytes_not_fully_pinned=bool(errors), inherited_common_mutex=True,
        qualification=str(output / 'qualification.json'), qualification_sha256=sha(output / 'qualification.json'),
        verified_pins=partial, chart_admission=False, actual_application_processes=0))
    if errors:
        raise RuntimeError('Formatted LOC terminal integrity failed; partial readable evidence retained')


def fixture_gate(plan, pins, source_spec):
    evidence = plan['fixture_qualification']
    receipt_path = Path(evidence['receipt_path']).resolve(strict=True)
    terminal_path = Path(evidence['terminal_path']).resolve(strict=True)
    require(pins[str(receipt_path)] == evidence['receipt_sha256'] and
            pins[str(terminal_path)] == evidence['terminal_sha256'], 'Root-reviewed formatter fixture pins differ')
    receipt = json.loads(receipt_path.read_bytes())
    terminal = json.loads(terminal_path.read_bytes())
    require(receipt['status'] == terminal['status'] == 'passed-formatted-loc-fixtures' and
            terminal['body_error'] is None and terminal['integrity_errors'] == [] and
            terminal['available_bytes_not_fully_pinned'] is False and terminal['inherited_common_mutex'] is True and
            terminal['receipt'] == str(receipt_path) and terminal['receipt_sha256'] == evidence['receipt_sha256'],
            'Actual formatter fixtures did not close successfully')
    for key in ('actual_native_subjects', 'actual_formatter_processes'):
        require(type(receipt[key]) is int and receipt[key] == 0 and type(terminal[key]) is int and terminal[key] == 0,
                'Fixture phase performed unexpected native work')
    require(type(receipt['actual_signals_sent']) is int and receipt['actual_signals_sent'] == 0, 'Fixture phase signalled a process')
    verify(terminal['pins']); merge(pins, terminal['pins'])
    runner = fresh(PACKET / 'qualify_fixtures.py', 'formatted_fixture_catalogue', source_spec)
    require(receipt['expected_test_ids'] == runner.IDS, 'Formatter fixture catalogue differs')
    for key in ('discovered_tests', 'tests_run'):
        require(type(receipt[key]) is int and receipt[key] == 23, 'Exact twenty-three actual fixtures required')
    for key in ('discovered_test_ids', 'actual_test_ids'):
        require(len(receipt[key]) == len(set(receipt[key])) == 23 and set(receipt[key]) == set(runner.IDS), 'Actual fixture ID set differs')
    require(all(receipt[key] == [] for key in ('failures','errors','skipped','expected_failures','unexpected_successes')),
            'Formatter fixtures include a failed/skipped outcome')
    verify(pins)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--plan', required=True)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    require(args.revision and all(c.isalnum() or c in '-_' for c in args.revision), 'Invalid unique revision')
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        actual, named = os.fstat(lock.fileno()), LOCK.stat()
        require((actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino), 'Common mutex inode differs')
        output = PACKET / 'jobs' / args.revision
        require(output.resolve().is_relative_to(PACKET.resolve()) and not output.exists(), 'Output is not fresh/contained')
        output.mkdir(parents=True)
        pins = {}; error = None; fd = None
        report = dict(status='started-formatted-loc-measurement', rows={}, measurement_identities={},
                      chart_admission=False, actual_application_processes=0, inherited_common_mutex=True)
        try:
            fd = fcntl.fcntl(lock.fileno(), fcntl.F_DUPFD, 64)
            os.set_inheritable(fd, True)
            plan_path = Path(args.plan).resolve(strict=True)
            require(sha(plan_path) == args.plan_sha256, 'Root-reviewed formatter plan changed')
            plan = json.loads(plan_path.read_bytes())
            require(plan['status'] == 'root-reviewed-formatted-loc-plan' and plan['physical_definition'] == DEFINITION,
                    'Formatter plan/definition not reviewed')
            require(plan['chart_admission'] is False and plan['application_execution_authorized'] is False, 'Wrong plan scope')
            merge(pins, plan['pins']); merge(pins, {str(plan_path): args.plan_sha256})
            require(pins[str(Path(__file__).resolve())] == sha(__file__), 'Worker was not reviewed')
            require(pins[str(Path(sys.executable).resolve())] == sha(Path(sys.executable).resolve()), 'Python executable differs')
            verify(pins)
            require(sha(SERVICE_PROPOSAL) == SERVICE_SHA, 'Generic service proposal changed')
            retained = json.loads(SERVICE_PROPOSAL.read_bytes())
            receipt_path = Path(retained['actual_mock_fixture_receipt'])
            merge(pins, {path: digest for path, digest in retained['pins'].items()
                if Path(path).suffix == '.py' or Path(path).parent == receipt_path.parent})
            merge(pins, {str(SERVICE_PROPOSAL): SERVICE_SHA}); verify(pins)
            dataset_path = Path(plan['source_dataset']['path']).resolve(strict=True)
            require(pins[str(dataset_path)] == plan['source_dataset']['sha256'], 'Dataset pin differs')
            baseline = json.loads(dataset_path.read_bytes())
            require(not any(language['key'].casefold() in {'v', 'red'} for language in baseline['languages']), 'V/Red cannot enter formatted dataset')
            observations = []
            with cold_sources(pins, plan['code_modules'], observations) as source_spec:
                fixture_gate(plan, pins, source_spec)
                service, guard, harness = retained_service(pins, source_spec)
                overlay = fresh(PACKET / 'overlay.py', 'formatted_overlay', source_spec)
                overlay.validate_exclusions(baseline)
                baseline_catalog = overlay.catalog(baseline)
                loaded = {}
                for name in {entry['module'] for entry in plan['counters'].values()}:
                    entry = plan['code_modules'][name]
                    loaded[name] = fresh(entry['path'], name, source_spec) if name not in sys.modules else sys.modules[name]
                for counter in plan['counters'].values():
                    entry = plan['code_modules'][counter['module']]
                    require(Path(entry['path']).resolve() == Path(counter['module_path']).resolve() and
                            entry['sha256'] == counter['module_sha256'] and
                            Path(loaded[counter['module']].__file__).resolve() == Path(counter['module_path']).resolve() and
                            sha(counter['module_path']) == counter['module_sha256'], 'Actual lexical catalogue/module identity differs')
                require(set(plan['profiles']) == set(plan['measurement_identities']), 'Profile/measurement inventory differs')
                report['profile_keys'] = list(plan['profiles'])
                versions = {}
                for index, (key, row) in enumerate(plan['profiles'].items()):
                    require(key in baseline_catalog and row['language'].casefold() not in {'v', 'red'}, 'Unknown/excluded profile')
                    require(overlay.exact_value(overlay.identity(baseline, key), plan['measurement_identities'][key]),
                            'Saved measurement identity differs')
                    inventory = row.get('compiler_inventory')
                    inventory_path = None
                    if inventory is not None:
                        inventory_path = Path(inventory['evidence_path']).resolve(strict=True)
                        require(pins[str(inventory_path)] == inventory['evidence_sha256'], 'Compiler input receipt pin differs')
                        actual_inventory = overlay.pointer_value(json.loads(inventory_path.read_bytes()), inventory['binding_field'])
                        require(overlay.exact_value(actual_inventory, inventory['binding']), 'Actual compiler input inventory differs')
                    overlay.validate_bundle(baseline, key, row['files'], inventory['binding'] if inventory is not None else None)
                    measurement_evidence = row.get('measurement_evidence')
                    actual_receipts = {}
                    if measurement_evidence is not None:
                        measurement_path = Path(measurement_evidence['path']).resolve(strict=True)
                        require(str(measurement_path) == measurement_evidence['path'] and
                                pins[str(measurement_path)] == measurement_evidence['sha256'],
                                'Independent actual measured execution receipt pin differs')
                        actual_receipts[str(measurement_path)] = json.loads(measurement_path.read_bytes())
                    forbidden = {str(dataset_path), str(plan_path), str(output / 'qualification.json'), str(output / 'terminal.json')}
                    if inventory_path is not None:
                        forbidden.add(str(inventory_path))
                    bound_execution = overlay.measured_execution(dict(pins=pins, receipts=actual_receipts), baseline,
                        key, row['files'], inventory['binding'] if inventory is not None else None, measurement_evidence, forbidden)
                    files = []
                    for file_index, item in enumerate(row['files']):
                        if not bound_execution:
                            files.append(pending_execution_file(item, pins))
                            continue
                        recipe = plan['formatters'][item['language']]
                        require(recipe['status'] in {'qualified', 'unavailable', 'pending'}, 'Unreviewed formatter route')
                        counter = plan['counters'].get(item['language'])
                        require(recipe['status'] != 'qualified' or counter is not None, 'Native formatter lacks reviewed physical lexer')
                        files.append(measure_file(item, recipe, counter, output / 'profiles' / str(index) / str(file_index),
                            pins, service, guard, harness, fd, loaded, versions))
                    statuses = {item['status'] for item in files}
                    status = ('qualified' if statuses == {'qualified'} else 'failed' if 'failed' in statuses else
                              'unavailable' if 'unavailable' in statuses else 'pending')
                    report['rows'][key] = dict(key=key, language=row['language'], files=files,
                        compiler_inventory=inventory, measurement_evidence=measurement_evidence,
                        status=status, complete=status == 'qualified',
                        formatted_loc=sum(item['physical_loc'] for item in files) if status == 'qualified' else None,
                        reasons=[item['reason'] for item in files if item['status'] != 'qualified'])
                    report['measurement_identities'][key] = plan['measurement_identities'][key]
                    verify(pins)
                report.update(status='completed-formatted-loc-measurement', source_dataset_sha256=plan['source_dataset']['sha256'],
                    source_dataset_path=str(dataset_path), physical_definition=DEFINITION, cold_imports=observations)
        except BaseException as failure:
            error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            try:
                with (output / 'exception.log').open('x') as stream:
                    traceback.print_exc(file=stream)
            except BaseException as journal_error:
                report['exception_journal_error'] = str(journal_error)
            raise
        finally:
            try:
                terminal(output, report, pins, error, fd if fd is not None else lock.fileno())
            finally:
                if fd is not None:
                    os.close(fd)


if __name__ == '__main__':
    main()
