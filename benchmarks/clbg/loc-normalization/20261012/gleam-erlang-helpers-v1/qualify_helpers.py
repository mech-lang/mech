"""Original Erlang AST helper count candidate; zero benchmark/timing processes."""
import argparse
import copy
import fcntl
import hashlib
import importlib
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[2]
GLEAM = CLBG / 'gleam-comparison/20261012'
ERLANG = CLBG / 'erlang-comparison/20261010'
LOCK = Path('/tmp/mech-clbg-timing.lock')
TOOL = ERLANG / 'support/loc_probe.erl'
TOOL_SHA = '4397dec13918804bf0fe83b454e1a187ed7da6e85988547b741a28fff36daf5b'
QUALIFIER = GLEAM / 'revisions/work-full-v3'
PROPOSAL_SHA = '57382cca1f673e89cff4553531cc4ccd47c648f9c69e00caf301b62a83c20a5d'
FIXTURES = (
    ('independent-bindings','f(X) -> Y = X, Z = X, Y + Z.',5),
    ('tuple-data','f(X) -> {X, X, X}.',3),
    ('call-arguments','f(X) -> g(X, X, X).',3),
    ('list-data-and-comment','f(X) -> [X, X, X]. % ignored, comment',3),
    ('nested-block','f(X) -> begin Y = X, Y end.',4),
)


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def merge(*groups):
    pins = {}
    for group in groups:
        for path,digest in group.items():
            assert path not in pins or pins[path] == digest
            pins[path] = digest
    return pins


def verify(pins):
    for path,digest in pins.items(): assert sha(path) == digest, ('Pinned source/runtime changed',path)


def save_new(path,value):
    with Path(path).open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')


def run(args):
    print('WAIT common mutex: original authored Erlang helper AST counts',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        descriptor,named = os.fstat(lock.fileno()),LOCK.stat()
        assert (descriptor.st_dev,descriptor.st_ino) == (named.st_dev,named.st_ino)
        output = HERE / 'jobs' / args.revision
        assert not output.exists(); output.mkdir(parents=True)
        pins = {}; terminal = 'failed-stop'
        fixture_path = output/'fixture-receipt.json'
        started = []; outcomes = []
        fixture_status = 'not-started'
        owner_started = []; owner_outcomes = []; owner_status = 'not-started'
        owner_ids = []
        try:
            audit_path = ERLANG / 'results/final-loc-audit.json'; audit = json.loads(audit_path.read_text())
            assert audit['status'] == 'passed' and len(audit['rows']) == 21
            assert sha(TOOL) == audit['loc_tool_sha256'] == TOOL_SHA
            assert sha(ERLANG/'audit_loc_final.py') == audit['audit_script_sha256']
            assert sha(ERLANG/'results/timings.json') == audit['timing_report_sha256']
            assert sha(ERLANG/'source-audit.json') == audit['source_audit_sha256']
            pins = {str(path):sha(path) for path in (TOOL,audit_path,ERLANG/'audit_loc_final.py',
                ERLANG/'results/timings.json',ERLANG/'source-audit.json',Path(__file__).resolve(),HERE/'REVIEW.md',
                HERE/'combine_proposals.py',Path(sys.executable).resolve())}
            merge_files = ('erlang_owners.py','owner_fixture_cases.py','original_owner_probe.erl','OWNER_RULES_PROPOSAL.md')
            pins = merge(pins,{str(HERE/name):sha(HERE/name) for name in merge_files})
            assert not any(name in sys.modules for name in ('erlang_owners','owner_fixture_cases'))
            sys.path.insert(0,str(HERE))
            try:
                owners = importlib.import_module('erlang_owners')
                owner_cases = importlib.import_module('owner_fixture_cases')
            finally: sys.path.pop(0)
            for name in ('erlang_owners','owner_fixture_cases'):
                path = HERE/(name+'.py')
                assert Path(sys.modules[name].__file__).resolve() == path and sha(path) == pins[str(path)]
            owner_ids = list(owner_cases.EXPECTED_IDS)
            assert len(owner_ids) == len(set(owner_ids)) == len(owner_cases.CASES) == 21
            verify(pins)
            native_packet = GLEAM/'revisions/private-v5'
            code_pins = {str(path):sha(path) for folder in (QUALIFIER,native_packet) for path in folder.iterdir()
                if path.is_file() and path.suffix in ('.py','.md')}
            pins = merge(pins,code_pins); verify(pins)
            packet_modules = {'qualify_work':QUALIFIER,'work_traces':QUALIFIER,'fixture_runner':QUALIFIER,'full_study':QUALIFIER,
                'references':native_packet,'guarded_prepare':native_packet,'proof_reuse':native_packet,
                'native_reuse':native_packet,'preserved_path_failure':native_packet}
            assert not any(name in sys.modules for name in packet_modules)
            sys.path.insert(0,str(QUALIFIER)); qualifier = importlib.import_module('qualify_work'); sys.path.pop(0)
            for name,folder in packet_modules.items():
                module = sys.modules[name]; path = folder/(name+'.py')
                assert Path(module.__file__).resolve() == path and sha(path) == pins[str(path)]
            verify(pins)
            guard = qualifier.preparation.load_guard(); harness = guard.load_harness()
            qualified_path = GLEAM/'qualification/private-v5/report.json'
            qualified = qualifier.qualified_report(qualified_path,guard)
            assert qualified['proposal_sha256'] == PROPOSAL_SHA
            assert len(qualified['profiles']) == 20 and qualified['reused_bounded_cases'] == 110 and len(qualified['attempts']) == 46
            pins = merge(pins,qualified['pins'],{str(qualified_path):sha(qualified_path)})
            preparation = qualifier.preparation
            assert sha(preparation.SERVICE) == pins[str(preparation.SERVICE)]
            service = preparation.load_service()
            assert Path(service.__file__).resolve() == preparation.SERVICE.resolve()
            verify(pins)
            for path in (preparation.ERL,preparation.ERLC,preparation.ENGINE,preparation.ERLEXEC):
                assert str(path) in pins and sha(path) == pins[str(path)]
            otp_parser_root = preparation.OTP_LIB/'lib/stdlib-8.1'
            for name in ('erl_scan','erl_parse','erl_anno','json'):
                for folder,suffix in (('src','.erl'),('ebin','.beam')):
                    path = otp_parser_root/folder/(name+suffix)
                    assert str(path) in pins and sha(path) == pins[str(path)]
            assert all(row['runtime_version'] == '29.1.1' for row in qualified['profiles'])
            neutral_path = CLBG/'loc-normalization/20261012/jobs/boundary-audit-18/fixture-receipt.json'
            assert sha(neutral_path) == 'b00afe351c40fb38e03a99ec81fe3d71bd69a82726f337aa87dfb40cfcaa6b6a'
            neutral = json.loads(neutral_path.read_text())
            assert neutral['status'] == 'passed' and neutral['tests_run'] == 672 and neutral['exclusive_mutex_held'] is True
            pygments_path = Path(neutral['pygments_module']).resolve()
            lexical_pins = {str(path.resolve()):sha(path) for path in pygments_path.parent.rglob('*.py')}
            pins = merge(pins,lexical_pins,{str(neutral_path):sha(neutral_path)})
            assert sha(pygments_path) == neutral['pygments_module_sha256']
            assert not any(name == 'pygments' or name.startswith('pygments.') for name in sys.modules)
            sys.path.insert(0,str(pygments_path.parent.parent))
            from pygments import lex
            from pygments.lexers import get_lexer_by_name
            from pygments.token import Comment
            assert Path(sys.modules['pygments'].__file__).resolve() == pygments_path
            assert sys.modules['pygments'].__version__ == '2.19.2'
            lexer = get_lexer_by_name('erlang',stripnl=False,ensurenl=False)
            verify(pins)
            environment = dict(qualified['profiles'][0]['runtime_environment'],
                ERL_CRASH_DUMP=str(output/'erl_crash.dump'),ERL_FLAGS='+S 1:1 +SDcpu 1 +SDio 1 +A 1')
            tool_dir = output/'tool'; tool_dir.mkdir()
            copied = tool_dir/'loc_probe.erl'; copied.write_bytes(TOOL.read_bytes())
            assert sha(copied) == TOOL_SHA; pins[str(copied)] = TOOL_SHA
            owner_tool = tool_dir/'original_owner_probe.erl'; owner_tool.write_bytes((HERE/'original_owner_probe.erl').read_bytes())
            assert sha(owner_tool) == pins[str(HERE/'original_owner_probe.erl')]
            pins[str(owner_tool)] = sha(owner_tool)
            service.owned_command(guard,harness,[str(preparation.ERLC),'-o',str(tool_dir),str(copied)],
                environment,output/'build-counter',lock.fileno(),60,'build-original-count-tool')
            service.owned_command(guard,harness,[str(preparation.ERLC),'-o',str(tool_dir),str(owner_tool)],
                environment,output/'build-owner-probe',lock.fileno(),60,'build-original-owner-probe')
            assert (tool_dir/'loc_probe.beam').is_file()
            pins = merge(pins,service.pin_tree(tool_dir),service.pin_tree(output/'build-counter'))
            def count(source,label):
                text = source.read_text()
                clean = ''.join('\n'*value.count('\n') if token in Comment else value for token,value in lex(text,lexer))
                assert not re.search(r'-\s*(?:include|include_lib|define)\s*\(|\bparse_transform\b',clean), 'Additional authored include/macro/transform source needs independent inventory'
                physical = sum(bool(line.strip()) for line in clean.splitlines())
                record = service.owned_command(guard,harness,[str(preparation.ERL),'+S','1:1','-noshell','-pa',str(tool_dir),
                    '-run','loc_probe','main',str(source)],environment,output/label,lock.fileno(),30,'original-source-AST-count')
                values = json.loads(record['stdout'])
                assert type(values) is list
                lines = text.splitlines(); seen = set(); expansions = []
                for value in values:
                    assert type(value) is dict and set(value) == {'line','extra'}
                    line,extra = value['line'],value['extra']
                    assert type(line) is int and type(extra) is int and 0 < line <= len(lines) and extra > 0 and line not in seen
                    seen.add(line)
                    expansions.append(dict(line=line,extra=extra,source=lines[line-1],
                        reason='Actual original Erlang clause/block expression sequence; tuple/list/call/guard separators excluded'))
                return dict(physical_loc=physical,normalized_loc=physical+sum(value['extra'] for value in expansions),
                    expansions=expansions,actual_AST_stdout=str(output/label/'original-source-AST-count.stdout'),
                    actual_command_receipt=str(output/label/'original-source-AST-count.json'),
                    original_source=str(source),original_source_sha256=sha(source),source_language='erlang',
                    generated_source_substitute=False,chart_admission=False,semantic_admission=False)
            fixture_status = 'failed'
            for name,body,expected in FIXTURES:
                source = output/('fixture-'+name+'.erl')
                source.write_text('-module(loc_fixture).\n-export([f/1]).\n'+body+'\n')
                started.append(name); actual = count(source,'fixture-'+name)
                outcomes.append(dict(id=name,expected=expected,actual=actual))
                assert actual['normalized_loc'] == expected
            assert started == [value[0] for value in FIXTURES] and len(set(started)) == 5
            legacy_path = output/'legacy-fixture-receipt.json'
            save_new(legacy_path,dict(status='passed',expected_ids=started,started_ids=started,actual_cases=5,
                failures=0,errors=0,skips=0,raw_case_receipts=outcomes,actual_common_mutex_held=True,
                actual_benchmark_processes=0,original_counter_source_sha256=TOOL_SHA,
                metric='preserved-legacy-expression-sequence-metric-not-relabelled-as-new-owner-metric'))
            fixture_status = 'passed'
            def owner_count(source,label):
                record = service.owned_command(guard,harness,[str(preparation.ERL),'+S','1:1','-noshell','-pa',str(tool_dir),
                    '-run','original_owner_probe','main',str(source)],environment,output/label,lock.fileno(),30,'original-source-scanner-AST')
                evidence = json.loads(record['stdout'])
                modules = evidence['actual_loaded_modules']
                assert [item['module'] for item in modules] == ['erl_scan','erl_parse','erl_anno','json']
                for item in modules:
                    path = otp_parser_root/'ebin'/(item['module']+'.beam')
                    assert Path(item['path']).resolve() == path.resolve() and sha(path) == pins[str(path)]
                metric = owners.project(source.read_text(),evidence)
                return dict(metric,actual_original_scanner_AST_stdout=str(output/label/'original-source-scanner-AST.stdout'),
                    actual_command_receipt=str(output/label/'original-source-scanner-AST.json'),
                    original_source=str(source),original_source_sha256=sha(source),source_language='erlang',
                    source_counter_metric='explicit-original-clause-callback-control-arm-body-and-written-close-owners',
                    semantic_admission=False)
            owner_status = 'failed'
            for name,body,arity,expected in owner_cases.CASES:
                source = output/('owner-fixture-'+name+'.erl')
                source.write_text('-module(owner_fixture).\n-export([f/'+str(arity)+']).\n'+body+'\n')
                owner_started.append(name); actual = owner_count(source,'owner-fixture-'+name)
                owner_outcomes.append(dict(id=name,expected_normalized_loc=expected,actual=actual))
                if expected is None:
                    assert actual['normalized_loc'] is None and actual['diagnostics']
                else:
                    assert type(expected) is int and actual['normalized_loc'] == expected and not actual['diagnostics']
                if name == 'named-header-and-value':
                    original = json.loads(Path(actual['actual_original_scanner_AST_stdout']).read_text())
                    corruptions = []
                    bad = copy.deepcopy(original); bad['tokens'][0]['column'] += 1; corruptions.append(('wrong-scanner-source-column',bad))
                    function = next(index for index,node in enumerate(original['forms']) if node[0] == 'function')
                    bad = copy.deepcopy(original); bad['forms'][function][2] = 'other'; corruptions.append(('wrong-original-function-name',bad))
                    bad = copy.deepcopy(original); bad['forms'][function][3] = True; corruptions.append(('boolean-original-arity',bad))
                    bad = copy.deepcopy(original); bad['forms'][function][4][0][4] = []; corruptions.append(('omitted-original-body-expression',bad))
                    bad = copy.deepcopy(original); bad['forms'].append(['error',[1,1],'unqualified original form']); corruptions.append(('error-form-inside-claimed-success',bad))
                    actual['actual_AST_corruption_subcases'] = []
                    for label,bad in corruptions:
                        rejected = owners.project(source.read_text(),bad)
                        assert rejected['normalized_loc'] is None and rejected['diagnostics']
                        actual['actual_AST_corruption_subcases'].append(dict(id=label,status='rejected',diagnostics=rejected['diagnostics']))
                if name == 'inline-receive-arm-body-close':
                    original = json.loads(Path(actual['actual_original_scanner_AST_stdout']).read_text())
                    bad = copy.deepcopy(original)
                    function = next(node for node in bad['forms'] if node[0] == 'function')
                    node = function[4][0][4][0]
                    assert node[0] == 'receive'
                    node[1] = [999,1]
                    rejected = owners.project(source.read_text(),bad)
                    assert rejected['normalized_loc'] is None and rejected['diagnostics']
                    actual['actual_AST_corruption_subcases'] = [dict(id='unrelated-original-control-location',status='rejected',diagnostics=rejected['diagnostics'])]
            assert owner_started == owner_ids and len(set(owner_started)) == 21
            save_new(fixture_path,dict(status='passed-original-source-owner-fixtures',expected_ids=owner_ids,
                started_ids=owner_started,actual_cases=21,failures=0,errors=0,skips=0,raw_case_receipts=owner_outcomes,
                actual_common_mutex_held=True,actual_benchmark_processes=0,legacy_sequence_metric_preserved_separately=True,
                legacy_fixture_receipt=dict(path=str(legacy_path),sha256=sha(legacy_path),actual_cases=5)))
            owner_status = 'passed'
            counts,origins = {},{}
            for row in qualified['profiles']:
                inventory_path = Path(row['compiled_input_inventory'])
                assert sha(inventory_path) == row['compiled_input_inventory_sha256'] == pins[str(inventory_path)]
                inventory = json.loads(inventory_path.read_text()); assert inventory['source_bundle'] == row['source_bundle']
                for item in row['source_bundle']:
                    source,origin = Path(item['path']),Path(item['origin'])
                    if source.suffix != '.erl': continue
                    assert item['role'] == 'application-helper' and source.read_bytes() == origin.read_bytes()
                    assert sha(source) == sha(origin) == item['sha256'] == pins[str(source)]
                    pins = merge(pins,{str(origin):item['sha256']})
                    key = (str(origin),item['sha256'])
                    if key not in origins:
                        origins[key] = owner_count(origin,'helper-'+str(len(origins)+1))
                        assert not origins[key]['diagnostics'] and type(origins[key]['normalized_loc']) is int, 'Actual authored helper ownership remains unresolved'
                    counts[str(source)] = dict(source_sha256=item['sha256'],origin=str(origin),role=item['role'],metric=origins[key],
                        compiler_bound_input_inventory=dict(path=str(inventory_path),sha256=row['compiled_input_inventory_sha256']))
            assert counts
            pins = merge(pins,{path:digest for path,digest in service.pin_tree(output).items()})
            verify(pins)
            for name,module in sys.modules.items():
                if name == 'pygments' or name.startswith('pygments.'):
                    path = Path(module.__file__).resolve(); assert str(path) in pins and sha(path) == pins[str(path)]
            result_path = output/'helper-count-proposals.json'
            save_new(result_path,dict(status='original-authored-Erlang-helper-count-proposals-awaiting-semantic-review',
                qualification=dict(path=str(qualified_path),sha256=pins[str(qualified_path)]),source_counts=counts,
                fixture_receipt=dict(path=str(fixture_path),sha256=sha(fixture_path),actual_cases=21),
                owner_rule_proposal=dict(path=str(HERE/'OWNER_RULES_PROPOSAL.md'),sha256=pins[str(HERE/'OWNER_RULES_PROPOSAL.md')]),
                original_scanner_AST_probe=dict(path=str(HERE/'original_owner_probe.erl'),sha256=pins[str(HERE/'original_owner_probe.erl')]),
                counter_source=dict(path=str(TOOL),sha256=TOOL_SHA),prior_qualified_reader_audit=dict(path=str(audit_path),sha256=pins[str(audit_path)]),
                runtime_version='29.1.1',fresh_count_only_environment=environment,pins=pins,actual_common_mutex_held=True,
                python_executable=str(Path(sys.executable).resolve()),python_version=sys.version,
                actual_benchmark_processes=0,full_performance_runs=0,canonical_LOC_admitted=False,chart_admission=False))
            terminal = 'completed-helper-count-proposals'
            print(json.dumps(dict(path=str(result_path),sha256=sha(result_path),measured_helper_copies=len(counts),original_aliases=len(origins))),flush=True)
        except BaseException as error:
            with (output/'exception.log').open('x') as stream: stream.write(traceback.format_exc())
            save_new(output/'failure.json',dict(status='failed-stop',error=str(error) or type(error).__name__,error_type=type(error).__name__,
                actual_common_mutex_held=True,actual_benchmark_processes=0,canonical_LOC_admitted=False))
            raise
        finally:
            legacy_path = output/'legacy-fixture-receipt.json'
            if not legacy_path.exists():
                save_new(legacy_path,dict(status=fixture_status,expected_ids=[value[0] for value in FIXTURES],
                    started_ids=started,actual_cases=len(started),raw_case_receipts=outcomes,actual_common_mutex_held=True,
                    actual_benchmark_processes=0,original_counter_source_sha256=TOOL_SHA,no_synthetic_completed_fixture_status=True))
            if not fixture_path.exists():
                save_new(fixture_path,dict(status=owner_status,expected_ids=owner_ids,started_ids=owner_started,
                    actual_cases=len(owner_started),raw_case_receipts=owner_outcomes,actual_common_mutex_held=True,
                    actual_benchmark_processes=0,no_synthetic_completed_fixture_status=True))
            captured = {}
            try:
                verify(pins)
                for path in sorted(output.rglob('*')):
                    if path.is_file(): captured[str(path)] = sha(path)
            except BaseException as error:
                save_new(output/'terminal-pins.json',dict(status='failed-terminal-integrity',preceding_status=terminal,error=str(error),
                    newly_captured_partial_terminal_pins=captured,actual_common_mutex_held=True,canonical_LOC_admitted=False))
                raise
            save_new(output/'terminal-pins.json',dict(status=terminal,pins=captured,prerequisite_pins=pins,actual_common_mutex_held=True,canonical_LOC_admitted=False))


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--revision',required=True); args = parser.parse_args()
    assert args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
    run(args)


if __name__ == '__main__': main()
