"""Locked additive proposals for all actual Gleam inputs; no admission/native."""
import argparse
import copy
import fcntl
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import traceback

CLBG = next(path for path in Path(__file__).resolve().parents if path.name == 'clbg')
ROOT = CLBG / 'gleam-comparison/20261012'
PACKET = CLBG / 'loc-normalization/20261012/gleam-proposal-v2'
V1 = PACKET.with_name('gleam-proposal-v1') / 'jobs/fixtures-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
QUALIFIER = ROOT / 'revisions/work-full-v3'
QUALIFICATION = ROOT / 'qualification/private-v5/report.json'
PROPOSAL_SHA = '57382cca1f673e89cff4553531cc4ccd47c648f9c69e00caf301b62a83c20a5d'


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify(pins):
    for path,digest in pins.items(): assert sha(path) == digest, ('Changed pinned input',path)


def merge(*groups):
    pins = {}
    for group in groups:
        for path,digest in group.items():
            assert path not in pins or pins[path] == digest, ('Pin conflict',path)
            pins[path] = digest
    return pins


def save_new(path,value):
    with Path(path).open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')


def imported(name,path,pins):
    assert name not in sys.modules
    assert sha(path) == pins[str(path)]
    spec = importlib.util.spec_from_file_location(name,path)
    module = importlib.util.module_from_spec(spec); sys.modules[name] = module
    spec.loader.exec_module(module)
    assert sys.modules[name] is module and Path(module.__file__).resolve() == path.resolve()
    verify(pins)
    return module


def old_projection(pins):
    # Preserve the actual v1 implementation without letting its bare import
    # substitute the current reader. The temporary binding is restored before
    # any new source normalization; execution is single-threaded under mutex.
    reader_path,units_path = V1 / 'snapshot/gleam_reader.py', V1 / 'snapshot/gleam_units.py'
    reader = imported('preserved_gleam_v1_reader',reader_path,pins)
    prior = sys.modules.get('gleam_reader')
    sys.modules['gleam_reader'] = reader
    try: units = imported('preserved_gleam_v1_units',units_path,pins)
    finally:
        if prior is None: del sys.modules['gleam_reader']
        else: sys.modules['gleam_reader'] = prior
    return units


def run(args):
    print('WAIT common mutex: Gleam actual20 compiler-input source proposals',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        descriptor,named = os.fstat(lock.fileno()),LOCK.stat()
        assert (descriptor.st_dev,descriptor.st_ino) == (named.st_dev,named.st_ino)
        output = PACKET / 'jobs' / args.revision
        assert not output.exists(); output.mkdir(parents=True)
        pins = {}; terminal = 'failed-stop'
        try:
            fixture_path = args.fixture_receipt.resolve(); job = fixture_path.parent
            assert sha(fixture_path) == args.fixture_sha256
            fixture = json.loads(fixture_path.read_text())
            assert fixture['status'] == 'passed-awaiting-root-source-review'
            assert type(fixture['tests_run']) is int and fixture['tests_run'] == fixture['expected_count'] == 34
            assert fixture['failures'] == fixture['errors'] == fixture['skipped'] == fixture['expected_failures'] == fixture['unexpected_successes'] == 0
            assert fixture['actual_common_mutex_held'] is True and fixture['pin_reconciliation_passed'] is True
            assert fixture['pins_before'] == fixture['pins_after'] and fixture['source_counting_started'] is False
            assert fixture['benchmark_processes_started'] == fixture['fixture_native_probes'] == 0
            terminal_path = job / 'fixture-receipt-pin.json'; saved = json.loads(terminal_path.read_text())
            assert saved['terminal_worker_status'] == saved['saved_receipt_status'] == fixture['status']
            assert saved['receipt'] == str(fixture_path) and saved['receipt_sha256'] == args.fixture_sha256
            pins = merge(fixture['pins_after'],saved['terminal_artifact_pins'],
                {str(fixture_path):args.fixture_sha256,str(terminal_path):sha(terminal_path),str(Path(__file__).resolve()):sha(__file__)})
            verify(pins)
            snapshot = job / 'snapshot'
            assert sha(snapshot / 'replay_sources.py') == sha(__file__)
            runner = job / 'worker.py'
            runner_module = imported('qualified_gleam_v2_fixture_runner',runner,pins)
            assert sorted(fixture['expected_ids']) == sorted(fixture['discovered_ids']) == sorted(fixture['started_ids']) == sorted(runner_module.EXPECTED_IDS)
            assert len(set(fixture['started_ids'])) == len(fixture['started_ids']) == 34
            assert sys.version == fixture['python_version'] and str(Path(sys.executable).resolve()) in pins
            assert not any(name in sys.modules for name in ('qualify_work','work_traces','references','fixture_runner','guarded_prepare','native_reuse'))
            qualified_code_pins = {str(path):sha(path) for path in QUALIFIER.iterdir() if path.is_file() and path.suffix in ('.py','.md')}
            pins = merge(pins,qualified_code_pins); verify(pins)
            sys.path.insert(0,str(QUALIFIER))
            qualification_module = importlib.import_module('qualify_work')
            sys.path.pop(0)
            assert Path(qualification_module.__file__).resolve() == QUALIFIER / 'qualify_work.py'
            guard = qualification_module.preparation.load_guard()
            qualification = qualification_module.qualified_report(QUALIFICATION,guard)
            assert qualification['proposal_sha256'] == PROPOSAL_SHA
            assert len(qualification['profiles']) == 20 and len(qualification['attempts']) == 46
            assert qualification['reused_bounded_cases'] == 110 and qualification['fresh_profile_builds'] == 6
            assert qualification['reused_cases_relabelled_as_attempts'] is False
            pins = merge(pins,qualification['pins'],{str(QUALIFICATION):sha(QUALIFICATION)})
            verify(pins)
            old = old_projection(pins)
            assert not any(name in sys.modules for name in ('gleam_reader','gleam_units','source_resolution'))
            sys.path.insert(0,str(snapshot))
            units = importlib.import_module('gleam_units')
            reader = importlib.import_module('gleam_reader')
            for name in ('gleam_reader','gleam_units','source_resolution'):
                assert Path(sys.modules[name].__file__).resolve() == snapshot / (name+'.py')
            sys.path.pop(0); verify(pins)
            vendor = ROOT / '.build/private-v1/stdlib'
            vendor_paths = [vendor / ('src/gleam/'+name+'.gleam') for name in ('list','dict','order')]
            assert 'version = "1.0.5"' in (vendor/'gleam.toml').read_text()
            for path in [vendor/'gleam.toml',*vendor_paths]: assert sha(path) == pins[str(path)]
            inventories,counts,profiles = {},{},{}
            for row in qualification['profiles']:
                inventory_path = Path(row['compiled_input_inventory'])
                assert sha(inventory_path) == row['compiled_input_inventory_sha256'] == pins[str(inventory_path)]
                inventory = json.loads(inventory_path.read_text())
                assert inventory['status'] == 'compiled-exact-authored-input-inventory' and inventory['profile'] == row['id']
                assert inventory['source_bundle'] == row['source_bundle'] and inventory['source'] == row['source']
                assert inventory['source_sha256'] == row['source_sha256'] and inventory['compiled_artifacts'] == row['compiled_artifacts']
                assert inventory['runtime_classpath'] == row['runtime_classpath']
                inventories[row['id']] = dict(path=str(inventory_path),sha256=row['compiled_input_inventory_sha256'],
                    source_bundle=row['source_bundle'],compiled_artifacts=row['compiled_artifacts'],runtime_classpath=row['runtime_classpath'])
                sources = {}
                for item in row['source_bundle']:
                    path = Path(item['path']); origin = Path(item['origin'])
                    assert sha(path) == sha(origin) == item['sha256'] == pins[str(path)]
                    assert path.read_bytes() == origin.read_bytes()
                    if path.suffix == '.gleam': sources[str(path)] = dict(text=path.read_text(),sha256=item['sha256'],module=path.stem,role=item['role'])
                    else: assert path.suffix == '.erl'
                for path in vendor_paths:
                    sources[str(path)] = dict(text=path.read_text(),sha256=pins[str(path)],module='gleam/'+path.stem,role='vendor')
                context = dict(status='exact-qualified-compiled-bundle-proposed-semantic-bindings',vendor_version='1.0.5',sources=sources,
                    compiler_bound_input_inventory=inventories[row['id']])
                bundle = []
                for item in row['source_bundle']:
                    path = Path(item['path'])
                    if path.suffix == '.gleam':
                        source = path.read_text()
                        result = units.propose_units(source,str(path),context)
                        previous = old.propose_units(source)
                        lexical = reader.lex(source)
                        clean = list(source)
                        for span in lexical['comment_spans']:
                            for offset in range(span['start'],span['end']):
                                if not clean[offset].isspace(): clean[offset] = ' '
                        result['physical_loc'] = sum(bool(line.strip()) for line in ''.join(clean).splitlines())
                        result['preserved_v1_diagnostics'] = previous['diagnostics']
                        result['preserved_v1_normalized_loc'] = previous['normalized_loc']
                        result['expansions'] = [dict(event,original_source=source[event['full_owner_start']:event['full_owner_end']]) for event in result['canonical_events']]
                    else:
                        result = dict(normalized_loc=None,physical_loc=None,diagnostics=[dict(kind='qualified-original-Erlang-AST-count-receipt-pending',offset=0)],
                            chart_admission=False,generated_source_substitute=False,authored_original_source_required=True)
                    counts[str(path)] = dict(source_sha256=item['sha256'],origin=item['origin'],role=item['role'],source_language=path.suffix[1:],metric=result)
                    bundle.append(dict(item,normalized_loc=result['normalized_loc'],physical_loc=result['physical_loc'],diagnostics=result['diagnostics']))
                profiles[row['id']] = dict(id=row['id'],source=row['source'],source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],source_bundle=bundle,
                    canonical_loc=None,source_language_counts_complete=all(item['normalized_loc'] is not None for item in bundle))
            verify(pins)
            unresolved = [dict(source=path,diagnostic=issue) for path,value in counts.items() for issue in value['metric']['diagnostics']]
            result = output / 'source-count-proposals.json'
            save_new(result,dict(status='actual20-source-proposals-awaiting-Erlang-count-and-semantic-review',
                qualification=dict(path=str(QUALIFICATION),sha256=pins[str(QUALIFICATION)]),
                fixture_receipt=dict(path=str(fixture_path),sha256=args.fixture_sha256,actual_methods=34),
                compiler_bound_input_inventory=inventories,profile_count=20,source_counts=counts,profiles=profiles,unresolved=unresolved,
                grammar_snapshot_pins={path:digest for path,digest in pins.items() if Path(path).is_relative_to(snapshot)},
                pins=pins,actual_common_mutex_held=True,native_processes_started=0,full_performance_runs=0,
                canonical_LOC_admitted=False,chart_admission=False,count_override=False,parser_pass_is_not_semantic_certification=True))
            terminal = 'completed-source-proposals'
            print(json.dumps(dict(path=str(result),sha256=sha(result),profiles=20,unresolved=len(unresolved),canonical_LOC_admitted=False)),flush=True)
        except BaseException as error:
            raw = output / 'exception.log'
            with raw.open('x') as stream: stream.write(traceback.format_exc())
            save_new(output/'failure.json',dict(status='failed-stop',error=str(error) or type(error).__name__,error_type=type(error).__name__,
                actual_common_mutex_held=True,native_processes_started=0,canonical_LOC_admitted=False,chart_admission=False))
            raise
        finally:
            captured = {}
            try:
                verify(pins)
                for path in sorted(output.rglob('*')):
                    if path.is_file(): captured[str(path)] = sha(path)
            except BaseException as integrity:
                save_new(output/'terminal-pins.json',dict(status='failed-terminal-integrity',terminal_worker_status=terminal,
                    error=str(integrity) or type(integrity).__name__,pins=captured,actual_common_mutex_held=True,
                    canonical_LOC_admitted=False,chart_admission=False))
                raise
            save_new(output/'terminal-pins.json',dict(status=terminal,pins=captured,prerequisite_pins=pins,
                actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixture-receipt',type=Path,required=True)
    parser.add_argument('--fixture-sha256',required=True)
    parser.add_argument('--revision',required=True)
    args = parser.parse_args()
    assert args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
    run(args)


if __name__ == '__main__': main()
