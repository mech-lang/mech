"""Additive exact Nim frontend inputs and unchanged LOC18 source proposals."""
import argparse
import fcntl
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

CLBG = next(path for path in Path(__file__).resolve().parents if path.name == 'clbg')
HERE = CLBG/'loc-normalization/20261012/nim-source-replay-v1'
NIM = CLBG/'nim-comparison/20261012'
QUALIFIER = NIM/'revisions/work-full-v4'
QUALIFICATION = NIM/'qualification/private-v7/report.json'
PROPOSAL_SHA = '519e47c1ac3ccec583e58cda70e7817d75aeae194987978346c8e57a85d2bcb9'
NEUTRAL = HERE.parent/'jobs/boundary-audit-18/fixture-receipt.json'
NEUTRAL_SHA = 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
LOCK = Path('/tmp/mech-clbg-timing.lock')
FILES = ('qualify_replay.py','input_inventory.py','test_input_inventory.py','REVIEW.md')
METHODS = ('test_fresh_frontend_command_preserves_original_semantics',
    'test_import_include_and_repeated_includes_preserve_actual_owners',
    'test_unknown_or_vendor_prefix_escape_is_rejected','test_main_and_all_authored_helpers_are_mandatory',
    'test_unrecognized_cached_relative_and_ansi_events_are_rejected',
    'test_saved_owned_receipt_rejects_nonzero_boolean_and_cleanup')
EXPECTED_IDS = sorted('test_input_inventory.InputInventoryFixtures.'+name for name in METHODS)


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()


def merge(pins,group):
    for path,digest in group.items():
        assert path not in pins or pins[path] == digest, ('Conflicting exact input pin',path)
        pins[path] = digest


def verify(pins):
    for path,digest in pins.items(): assert sha(path) == digest, ('Changed exact input',path)


def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')


def imported(name,path,pins):
    assert name not in sys.modules and sha(path) == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name,path)
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    spec.loader.exec_module(module)
    assert sys.modules[name] is module and Path(module.__file__).resolve() == path.resolve()
    verify(pins)
    return module


def snapshot(output,pins):
    directory = output/'snapshot'; directory.mkdir()
    for name in FILES:
        original = HERE/name; copied = directory/name
        digest = sha(original); copied.write_bytes(original.read_bytes()); assert sha(copied) == digest
        copied.chmod(0o444); merge(pins,{str(original):digest,str(copied):digest})
    merge(pins,{str(Path(sys.executable).resolve()):sha(Path(sys.executable).resolve())})
    assert sys.version_info[:2] == (3,14)
    return directory


def unittest_snapshot(output,pins):
    assert not any(name == 'unittest' or name.startswith('unittest.') for name in sys.modules)
    spec = importlib.util.find_spec('unittest'); assert spec is not None and spec.origin is not None
    original = Path(spec.origin).resolve().parent
    copied = output/'runtime/unittest'; copied.mkdir(parents=True)
    for path in original.rglob('*.py'):
        target = copied/path.relative_to(original); target.parent.mkdir(parents=True,exist_ok=True)
        digest = sha(path); target.write_bytes(path.read_bytes()); assert sha(target) == digest
        target.chmod(0o444); merge(pins,{str(path):digest,str(target):digest})
    verify(pins)
    sys.path.insert(0,str(copied.parent))
    try: module = importlib.import_module('unittest')
    finally: sys.path.pop(0)
    assert Path(module.__file__).resolve() == copied/'__init__.py'
    return module,copied


def unittest_identity(copied,pins):
    for name,module in sys.modules.items():
        if name == 'unittest' or name.startswith('unittest.'):
            path = Path(module.__file__).resolve()
            assert path.is_relative_to(copied) and sha(path) == pins[str(path)]


def fixtures(directory,output,pins):
    found,started,results = [],[],[]
    before = {}; passed = False; completed = False; reconciled = False; fixture_error = None
    unittest = None; copied = None
    try:
        unittest,copied = unittest_snapshot(output,pins)
        module = imported('test_input_inventory',directory/'test_input_inventory.py',pins)
        suite = unittest.defaultTestLoader.loadTestsFromModule(module)
        def flat(value):
            for item in value:
                if isinstance(item,unittest.TestSuite): yield from flat(item)
                else: yield item
        found = [item.id() for item in flat(suite)]
        assert sorted(found) == EXPECTED_IDS and len(found) == len(set(found)) == 6
        unittest_identity(copied,pins); verify(pins); before = dict(pins)
        class ActualResult(unittest.TextTestResult):
            def __init__(self,*values): super().__init__(*values); results.append(self)
            def startTest(self,test): started.append(test.id()); super().startTest(test)
        with (output/'fixtures.log').open('x') as stream:
            result = unittest.TextTestRunner(stream=stream,verbosity=2,resultclass=ActualResult).run(suite)
            stream.flush()
        completed = True
        unittest_identity(copied,pins); verify(pins); reconciled = True
        passed = (result.wasSuccessful() and result.testsRun == 6 and sorted(started) == EXPECTED_IDS and
            len(set(started)) == 6 and not result.skipped and not result.expectedFailures and not result.unexpectedSuccesses)
        assert passed, 'Actual source input fixtures did not all complete'
    except BaseException as error:
        fixture_error = dict(error=str(error) or type(error).__name__,error_type=type(error).__name__)
        raise
    finally:
        result = results[0] if results else None
        receipt = dict(status='passed-source-input-fixtures-awaiting-root-review' if passed else 'failed-source-input-fixtures',
            expected_ids=EXPECTED_IDS,discovered_ids=found,started_ids=started,tests_run=result.testsRun if result else 0,
            failures=len(result.failures) if result else 0,errors=len(result.errors) if result else 0,
            skipped=len(result.skipped) if result else 0,expected_failures=len(result.expectedFailures) if result else 0,
            unexpected_successes=len(result.unexpectedSuccesses) if result else 0,execution_completed=completed,fixture_error=fixture_error,
            pins_before=before,pins_after=dict(pins),pin_reconciliation_passed=reconciled,actual_common_mutex_held=True,
            python_version=sys.version,python_executable=str(Path(sys.executable).resolve()),
            unittest_module=str(Path(unittest.__file__).resolve()) if unittest else None,
            unittest_module_sha256=pins[str(Path(unittest.__file__).resolve())] if unittest else None,
            actual_compiler_processes=0,actual_benchmark_processes=0,source_normalization_started=False,
            chart_admission=False,canonical_LOC_admitted=False)
        save(output/'fixture-receipt.json',receipt)


def fixture_prerequisite(args,pins):
    path = args.fixture_receipt.resolve(); assert sha(path) == args.fixture_sha256
    receipt = json.loads(path.read_text())
    assert receipt['status'] == 'passed-source-input-fixtures-awaiting-root-review'
    assert type(receipt['tests_run']) is int and receipt['tests_run'] == 6
    for name in ('expected_ids','discovered_ids','started_ids'):
        assert sorted(receipt[name]) == EXPECTED_IDS and len(receipt[name]) == len(set(receipt[name])) == 6
    assert all(type(receipt[name]) is int and receipt[name] == 0 for name in
        ('failures','errors','skipped','expected_failures','unexpected_successes','actual_compiler_processes','actual_benchmark_processes'))
    assert receipt['actual_common_mutex_held'] is receipt['pin_reconciliation_passed'] is True
    assert receipt['execution_completed'] is True and receipt['fixture_error'] is None
    assert receipt['pins_before'] == receipt['pins_after'] and receipt['python_version'] == sys.version
    terminal_path = path.parent/'terminal-pins.json'; terminal = json.loads(terminal_path.read_text())
    assert terminal['status'] == 'completed-fixture-only' and terminal['actual_common_mutex_held'] is True
    assert terminal['pins'][str(path)] == args.fixture_sha256
    merge(pins,receipt['pins_after']); merge(pins,terminal['pins']); merge(pins,terminal['prerequisite_pins'])
    merge(pins,{str(path):args.fixture_sha256,str(terminal_path):sha(terminal_path)})
    for name in FILES: assert sha(HERE/name) == receipt['pins_after'][str(HERE/name)]
    verify(pins)


def qualified(pins):
    raw = json.loads(QUALIFICATION.read_text())
    assert raw['proposal_sha256'] == PROPOSAL_SHA
    merge(pins,raw['pins']); merge(pins,{str(QUALIFICATION):sha(QUALIFICATION)})
    verify(pins)
    modules = {'qualify_work':QUALIFIER,'tree_work':QUALIFIER,
        'guarded_prepare_v7':NIM,'proof_reuse_v5':NIM,'primitive_reuse_v6':NIM,'ownership_reuse_v7':NIM,
        'fixture_runner':NIM,'references':NIM}
    cohort_modules = {path.stem:path for path in NIM.glob('*.py')}
    assert not any(name in sys.modules for name in {*modules,*cohort_modules})
    merge(pins,{str(path):sha(path) for path in QUALIFIER.iterdir() if path.is_file() and path.suffix in ('.py','.md')})
    verify(pins)
    sys.path.insert(0,str(QUALIFIER))
    try: qualifier = importlib.import_module('qualify_work')
    finally: sys.path.pop(0)
    for name,folder in modules.items():
        path = folder/(name+'.py')
        assert Path(sys.modules[name].__file__).resolve() == path and sha(path) == pins[str(path)]
    for name,path in cohort_modules.items():
        if name in sys.modules:
            assert Path(sys.modules[name].__file__).resolve() == path and sha(path) == pins[str(path)]
    verify(pins)
    guard = qualifier.preparation.load_guard(); harness = guard.load_harness()
    report,reuse = qualifier.qualified_report(QUALIFICATION,guard)
    assert report['proposal_sha256'] == PROPOSAL_SHA and len(report['profiles']) == 20
    assert report['reused_bounded_cases'] == 154 and len(report['proofs']) == 24 and len(report['attempts']) == 20
    merge(pins,reuse['pins']); verify(pins)
    assert Path(guard.__file__).resolve() == qualifier.preparation.GUARD and sha(guard.__file__) == qualifier.preparation.GUARD_SHA
    return qualifier,guard,harness,report,reuse


def grammar(pins):
    assert sha(NEUTRAL) == NEUTRAL_SHA
    receipt = json.loads(NEUTRAL.read_text())
    assert receipt['status'] == 'passed' and type(receipt['tests_run']) is int and receipt['tests_run'] == 672
    assert receipt['failures'] == receipt['errors'] == 0 and receipt['exclusive_mutex_held'] is True
    assert receipt['common_timing_mutex'] == str(LOCK) and receipt['python_version'] == sys.version
    merge(pins,receipt['frozen_dependency_pins']); merge(pins,{str(NEUTRAL):NEUTRAL_SHA}); verify(pins)
    pygments = Path(receipt['pygments_module']).resolve()
    assert sha(pygments) == receipt['pygments_module_sha256']
    assert not any(name == 'pygments' or name.startswith('pygments.') for name in sys.modules)
    merge(pins,{str(path):sha(path) for path in pygments.parent.rglob('*.py')})
    sys.path.insert(0,str(pygments.parent.parent))
    import pygments as lexical
    assert Path(lexical.__file__).resolve() == pygments and lexical.__version__ == receipt['pygments_version'] == '2.19.2'
    frozen = NEUTRAL.parent/'snapshot'
    names = ('foreign_nim','foreign_future_mask','foreign_core','canonical')
    assert not any(name in sys.modules for name in names)
    sys.path.insert(0,str(frozen))
    try: normalizer = importlib.import_module('foreign_nim')
    finally: sys.path.pop(0)
    for name in names:
        path = frozen/(name+'.py')
        assert Path(sys.modules[name].__file__).resolve() == path and sha(path) == pins[str(path)]
    verify(pins)
    return normalizer


def collect(adapter,qualifier,guard,harness,report,reuse,output,lock_fd,pins,state):
    normalizer = grammar(pins)
    inventories,counts,profiles = {},{},{}
    state['inventories'] = inventories; state['source_counts'] = counts; state['profiles'] = profiles
    for row in report['profiles']:
        identifier = row['id']; prior = reuse['profiles'][identifier]
        command = prior['original_build_command']; source = Path(row['source']); binary = Path(row['command'][0])
        assert command[-1] == str(source) and '--out:'+str(binary) in command
        build_receipt = source.parents[3]/'builds'/identifier/'build.json'
        assert sha(build_receipt) == pins[str(build_receipt)]
        adapter.completed_owned_receipt(json.loads(build_receipt.read_text()),command)
        compiler = Path(command[0]); toolchain = compiler.parent.parent
        assert sha(compiler) == pins[str(compiler)] and toolchain.name == 'nim-2.2.12'
        for name in ('modulegraphs','semstmts','pipelines','commands','extccomp','msgs'):
            path = toolchain/'compiler'/(name+'.nim'); assert sha(path) == pins[str(path)]
        assert 'onProcessing(c.graph, f, "include", c.module)' in (toolchain/'compiler/semstmts.nim').read_text()
        assert 'processModuleAux("import")' in (toolchain/'compiler/pipelines.nim').read_text()
        assert 'if conf.globalOptions * {optCompileOnly, optGenScript} == {optCompileOnly}:' in (toolchain/'compiler/extccomp.nim').read_text()
        snapshot_root = source.parents[2]
        helpers = [snapshot_root/name for name in (*row['application_helpers'],*row['general_library_support'])]
        assert len(helpers) == len(set(helpers))
        expected_origins = {str(source):NIM/'sources'/row['benchmark']/(row['variant']+'.nim')}
        expected_origins.update({str(path):NIM/path.relative_to(snapshot_root) for path in helpers})
        for path,origin in expected_origins.items():
            assert sha(path) == sha(origin) == pins[path] and Path(path).read_bytes() == origin.read_bytes()
            merge(pins,{str(origin):sha(origin)})
        vendor_roots = [Path(value.split(':',1)[1]) for value in command if value.startswith(('--lib:','--path:'))]
        assert len(vendor_roots) == 3 and vendor_roots[0] == toolchain/'lib'
        work = output/'frontend'/identifier; work.mkdir(parents=True)
        cache = work/'cache'; cache.mkdir(); never_binary = work/'never-scored-binary'
        frontend_command = adapter.audit_command(command,cache,never_binary)
        state['compiler_commands_started'] += 1
        actual = qualifier.preparation.owned_command(guard,harness,frontend_command,row['runtime_environment'],work,lock_fd,120,'frontend-inputs')
        adapter.completed_owned_receipt(actual['detail'],frontend_command)
        assert not never_binary.exists() and not any(cache.rglob('*.o')), 'Frontend audit unexpectedly built native output'
        parsed = adapter.processing_inputs(actual['stderr'].decode(),source,helpers,vendor_roots)
        for item in parsed['files']:
            path = Path(item['path'])
            known = pins.get(str(path))
            if known is None:
                known = next((pins[alias] for alias in item['lookup_aliases'] if alias in pins),None)
            assert known is not None and sha(path) == known, ('Unpinned compiler-loaded source',str(path))
            merge(pins,{str(path):known,**{alias:known for alias in item['lookup_aliases']}})
            item['sha256'] = known
        bundle = []
        for item in parsed['authored_files']:
            path = Path(item['path']); origin = expected_origins[str(path)]
            assert sha(origin) == item['sha256']
            if str(path) not in counts:
                metric = normalizer.normalize(path.read_text(),'nim')
                counts[str(path)] = dict(source_sha256=item['sha256'],origin=str(origin),role=item['role'],source_language='nim',
                    metric=metric,original_bytes_equal=True,generated_source_substitute=False)
            metric = counts[str(path)]['metric']
            bundle.append(dict(path=str(path),sha256=item['sha256'],origin=str(origin),role=item['role'],
                normalized_loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],diagnostics=metric['diagnostics']))
        evidence = work/'compiler-input-inventory.json'
        value = dict(status='actual-exact-authored-Nim-frontend-inputs-awaiting-independent-review',id=identifier,
            source=row['source'],source_sha256=row['source_sha256'],binary=str(binary),binary_sha256=row['binary_sha256'],
            original_build_command=command,original_build_receipt=dict(path=str(build_receipt),sha256=pins[str(build_receipt)]),
            audit_command=frontend_command,audit_receipt=str(work/'frontend-inputs.json'),
            original_semantic_flags_preserved=True,original_binary_rebuilt=False,compiler_frontend_only=True,
            vendor_roots=list(map(str,vendor_roots)),processing_evidence=parsed,source_bundle=bundle,
            actual_common_mutex_held=True,generated_source_substitute=False,canonical_LOC_admitted=False)
        save(evidence,value)
        inventories[identifier] = dict(path=str(evidence),sha256=sha(evidence),source_bundle=bundle,
            source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'])
        profiles[identifier] = dict(id=identifier,benchmark=row['benchmark'],variant=row['variant'],source=row['source'],
            source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],source_bundle=bundle,canonical_loc=None,
            diagnostics=[dict(source=item['path'],diagnostic=issue) for item in bundle for issue in item['diagnostics']])
        merge(pins,qualifier.preparation.pin_tree(work)); verify(pins)
        print('PROPOSED exact Nim source bundle: '+identifier,flush=True)
    assert len(inventories) == len(profiles) == 20 and state['compiler_commands_started'] == 20
    return inventories,counts,profiles


def run(args):
    print('WAIT common mutex: Nim source input fixtures/replay only',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        held,named = os.fstat(lock.fileno()),LOCK.stat(); assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
        output = HERE/'jobs'/args.revision; assert not output.exists(); output.mkdir(parents=True)
        pins = {}; terminal = 'failed-stop'
        state = dict(status='stopped-unqualified',compiler_commands_started=0,actual_benchmark_processes=0,
            full_performance_runs=0,canonical_LOC_admitted=False,chart_admission=False,actual_common_mutex_held=True)
        try:
            directory = snapshot(output,pins)
            adapter = imported('input_inventory',directory/'input_inventory.py',pins)
            if args.fixtures_only:
                fixtures(directory,output,pins); terminal = 'completed-fixture-only'
            else:
                assert args.fixture_receipt is not None and args.fixture_sha256 is not None
                fixture_prerequisite(args,pins)
                qualifier,guard,harness,report,reuse = qualified(pins)
                inventories,counts,profiles = collect(adapter,qualifier,guard,harness,report,reuse,output,lock.fileno(),pins,state)
                verify(pins)
                save(output/'source-count-proposals.json',dict(status='actual20-Nim-compiler-input-and-LOC18-proposals-awaiting-semantic-review',
                    qualification=dict(path=str(QUALIFICATION),sha256=pins[str(QUALIFICATION)]),profile_count=20,
                    compiler_bound_input_inventory=inventories,profiles=profiles,source_counts=counts,
                    grammar_fixture=dict(path=str(NEUTRAL),sha256=NEUTRAL_SHA,actual_methods=672),
                    source_input_fixture=dict(path=str(args.fixture_receipt.resolve()),sha256=args.fixture_sha256,actual_methods=6),
                    grammar_snapshot_pins={path:digest for path,digest in pins.items() if Path(path).is_relative_to(NEUTRAL.parent/'snapshot')},
                    unresolved=[dict(source=path,diagnostic=issue) for path,value in counts.items() for issue in value['metric']['diagnostics']],
                    original_scored_sources_and_binaries_preserved=True,original_timing_fingerprints_preserved=True,
                    actual_frontend_compiler_commands=20,actual_native_backend_compilations=0,actual_benchmark_processes=0,
                    actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False,pins=dict(pins)))
                terminal = 'completed-source-count-proposals'
        except BaseException as error:
            state.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            with (output/'exception.log').open('x') as stream: stream.write(traceback.format_exc())
            save(output/'stopped-report.json',state); raise
        finally:
            captured = {}
            try:
                verify(pins)
                for path in sorted(output.rglob('*')):
                    if path.is_file(): captured[str(path)] = sha(path)
            except BaseException as error:
                save(output/'terminal-pins.json',dict(status='failed-terminal-integrity',preceding_status=terminal,
                    error=str(error) or type(error).__name__,pins=captured,prerequisite_pins=pins,
                    actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False))
                raise
            save(output/'terminal-pins.json',dict(status=terminal,pins=captured,prerequisite_pins=pins,
                actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False))
        print(json.dumps(dict(status=terminal,output=str(output),native_subjects=0)),flush=True)


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--revision',required=True)
    parser.add_argument('--fixtures-only',action='store_true')
    parser.add_argument('--fixture-receipt',type=Path); parser.add_argument('--fixture-sha256')
    args = parser.parse_args(); assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}',args.revision)
    run(args)


if __name__ == '__main__': main()
