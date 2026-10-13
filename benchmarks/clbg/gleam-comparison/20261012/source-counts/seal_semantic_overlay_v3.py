"""Resolve four exact retained semantic gates, then seal reviewed LOC20.

This reads original qualified owner AST/events only. No source parser, counter,
compiler, fixture, runtime command or dataset runs. Original nulls are preserved.
"""
import argparse
from contextlib import ExitStack
from copy import deepcopy
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
BASE = HERE / 'seal_overlay_v2.py'
FACTS = HERE / 'source-owner-facts-v1.json'
NOTE = HERE / 'SEMANTIC_REVIEW_V2.md'
LOCK = Path('/tmp/mech-clbg-timing.lock')
SUITE_SHA = 'e2b4a772153c9997d4ab5f749dfa950cbcf4ebaad825201b3c6338454f4e2594'
HELPER_IDS = ['named-header-and-value','inline-first-callback','expanded-first-callback','independent-callback-effect',
    'two-real-callbacks','inline-receive-arm-body-close','expanded-receive-arm-body-close','nested-receive-two-outer-arms',
    'packed-case-pattern-and-data','independent-case-arm-effect','runner-try-catch-two-effects','literal-keywords-and-separators',
    'wrapped-data-retains-physical-row','independent-chained-matches','additional-anonymous-clause','unknown-authored-include',
    'actual-parser-error','unknown-comprehension-owner','earlier-effect-before-callback','intervening-effect-between-callbacks',
    'written-selector-matches-remain-gated']
VENDOR_DECLARATIONS = {
    'drop': 'pub fn drop(from list: List(a), up_to n: Int) -> List(a) {\n  case n <= 0 {\n    True -> list\n    False ->\n      case list {\n        [] -> []\n        [_, ..rest] -> drop(rest, n - 1)\n      }\n  }\n}',
    'take': 'pub fn take(from list: List(a), up_to n: Int) -> List(a) {\n  take_loop(list, n, [])\n}',
    'take_loop': 'fn take_loop(list: List(a), n: Int, acc: List(a)) -> List(a) {\n  case n <= 0 {\n    True -> reverse(acc)\n    False ->\n      case list {\n        [] -> reverse(acc)\n        [first, ..rest] -> take_loop(rest, n - 1, [first, ..acc])\n      }\n  }\n}',
    'zip': 'pub fn zip(list: List(a), with other: List(b)) -> List(#(a, b)) {\n  zip_loop(list, other, [])\n}',
    'zip_loop': 'fn zip_loop(one: List(a), other: List(b), acc: List(#(a, b))) -> List(#(a, b)) {\n  case one, other {\n    [first_one, ..rest_one], [first_other, ..rest_other] ->\n      zip_loop(rest_one, rest_other, [#(first_one, first_other), ..acc])\n    _, _ -> reverse(acc)\n  }\n}',
    'reverse': '@external(erlang, "lists", "reverse")\npub fn reverse(list: List(a)) -> List(a) {\n  reverse_and_prepend(list, [])\n}',
    'reverse_and_prepend': '@external(erlang, "lists", "reverse")\nfn reverse_and_prepend(list prefix: List(a), to suffix: List(a)) -> List(a) {\n  case prefix {\n    [] -> suffix\n    [first, ..rest] -> reverse_and_prepend(list: rest, to: [first, ..suffix])\n  }\n}',
    'map': 'pub fn map(list: List(a), with fun: fn(a) -> b) -> List(b) {\n  map_loop(list, fun, [])\n}',
    'map_loop': 'fn map_loop(list: List(a), fun: fn(a) -> b, acc: List(b)) -> List(b) {\n  case list {\n    [] -> reverse(acc)\n    [first, ..rest] -> map_loop(rest, fun, [fun(first), ..acc])\n  }\n}',
    'each': 'pub fn each(list: List(a), f: fn(a) -> b) -> Nil {\n  case list {\n    [] -> Nil\n    [first, ..rest] -> {\n      f(first)\n      each(rest, f)\n    }\n  }\n}',
}


def walk(node):
    yield node
    for child in node['children']: yield from walk(child)


def arguments(node):
    assert node['kind'] == 'call'
    values = node['children'][1:]
    assert all(value['kind'] == 'argument' and value['label'] is None and value['spread'] is False and len(value['children']) == 1 for value in values)
    return [value['children'][0] for value in values]


def source_events(metric, source, language):
    units = metric['canonical_events']
    assert units and all(type(item['units']) is int and item['units'] in (0, 1) for item in units)
    anchors = sorted((item['start'], item['end']) for item in units)
    assert all(type(start) is type(end) is int and 0 <= start < end <= len(source) for start, end in anchors)
    assert all(left[1] <= right[0] for left, right in zip(anchors, anchors[1:])), 'Overlapping retained unit anchors'
    if language == 'gleam':
        assert all(item['units'] == 1 for item in units)
        assert len(metric['expansions']) == len(units)
        for unit, expansion in zip(units, metric['expansions']):
            assert {key: value for key, value in expansion.items() if key != 'original_source'} == unit
            assert 0 <= unit['full_owner_start'] < unit['full_owner_end'] <= len(source)
            assert source[unit['full_owner_start']:unit['full_owner_end']] == expansion['original_source']
    else:
        for unit in units:
            assert source[unit['start']:unit['end']] == unit['original_source']
            assert unit['units'] == 1 or unit['kind'] == 'structural-suffix'
    return sum(item['units'] for item in units)


def resolve_regex(profile, original, source, facts, vendor):
    selected = [item for item in facts['semantic_prefixes'] if item['profile'] == profile]
    assert selected and original['normalized_loc'] is None
    assert original['diagnostics'] == [dict(kind=item['diagnostic_kind'], offset=item['start'],
        message='Earlier callable operand requires source-bound semantic ownership review.') for item in selected]
    assert original['binding_context_bytes_reconciled'] is True and original['proposal_only'] is True
    declarations = original['original_owner_ast']
    imported = [node for node in declarations if node['kind'] == 'import' and node['alias'] == 'list']
    assert len(imported) == 1 and imported[0]['module'] == 'gleam/list' and imported[0]['selectors'] == []
    assert not any(node.get('name') in ('list', 'patterns') and node['kind'] in ('constant', 'external-function') for node in declarations)
    pattern_defs = [node for node in declarations if node['kind'] == 'function' and node['name'] == 'patterns']
    mains = [node for node in declarations if node['kind'] == 'function' and node['name'] == 'main']
    assert len(pattern_defs) == len(mains) == 1
    pattern, main = pattern_defs[0], mains[0]
    assert pattern['formals'] == main['formals'] == []
    assert re.sub(r'\s+', '', source[pattern['returns']['start']:pattern['returns']['end']]) == 'List(String)'
    statements = pattern['children'][0]['children']
    assert len(statements) == 1 and statements[0]['kind'] == 'expression-statement'
    literal = statements[0]['children'][0]
    assert literal['kind'] == 'list' and len(literal['children']) == 9
    assert all(node['kind'] == 'atom' and node['token_kind'] == 'string' for node in literal['children'])
    nodes = list(walk(main)); certificates = []
    for item in selected:
        candidates = [node for node in nodes if node['kind'] == 'call' and node['start'] == item['start'] and node['end'] == item['end']]
        assert len(candidates) == 1
        candidate = candidates[0]
        assert source[candidate['start']:candidate['end']] == item['exact_call']
        containing = [node for node in nodes if node['kind'] == 'call' and len(node['children']) == 3 and arguments(node)[0] == candidate and arguments(node)[1]['kind'] == 'anonymous-function']
        assert len(containing) == 1
        whole = containing[0]; first = arguments(whole)[1]; callee = whole['children'][0]
        assert candidate['end'] < first['start']
        assert callee['kind'] == 'field' and callee['children'][0]['kind'] == 'atom' and callee['children'][0]['word'] == 'list'
        assert callee['field'] == ('map' if item['exact_call'].startswith(('list.take(', 'list.drop(')) else 'each')
        if item['exact_call'] == 'patterns()':
            assert candidate['children'][0]['kind'] == 'atom' and candidate['children'][0]['word'] == 'patterns' and arguments(candidate) == []
            refs = [dict(kind='separately-counted-local-literal-definition', start=pattern['start'], end=pattern['end'], exact_source=source[pattern['start']:pattern['end']], original_owner_ast=deepcopy(pattern))]
            lexical = {}
        else:
            nested = candidate['children'][0]
            assert nested['kind'] == 'field' and nested['children'][0]['kind'] == 'atom' and nested['children'][0]['word'] == 'list'
            values = arguments(candidate)
            assert len(values) == 2 and values[0]['kind'] == 'atom' and values[0]['word'] == 'sources'
            if nested['field'] in ('take', 'drop'):
                assert values[1]['kind'] == 'atom' and values[1]['token_kind'] == 'number' and values[1]['word'] == '7'
            else:
                assert nested['field'] == 'zip' and values[1]['kind'] == 'atom' and values[1]['word'] == 'counts'
            before = [node for node in main['children'][0]['children'] if node['end'] <= candidate['start']]
            source_bindings = [node for node in before if node['kind'] == 'binding' and source[node['start']:node['end']] == 'let sources = patterns()']
            assert len(source_bindings) == 1
            assert not any(node['kind'] == 'binding' and source[node['pattern']['start']:node['pattern']['end']].strip() in ('list', 'patterns', 'sources') and node is not source_bindings[0] for node in before)
            if nested['field'] == 'zip':
                for expected in ('let counts = list.append(list.map(tasks, process.join), local)',):
                    assert len([node for node in before if source[node['start']:node['end']] == expected]) == 1
            names = {'take': ('take', 'take_loop', 'reverse', 'reverse_and_prepend'), 'drop': ('drop',), 'zip': ('zip', 'zip_loop', 'reverse', 'reverse_and_prepend')}[nested['field']]
            refs = [deepcopy(vendor[name]) for name in names]
            lexical = dict(sources='List(String)', tasks='List(process.Task(Int))', local='List(Int)', counts='List(Int)')
        certificates.append(dict(original_diagnostic=deepcopy(original['diagnostics'][len(certificates)]), contract=item['callee_contract'],
            start=candidate['start'], end=candidate['end'], exact_call=item['exact_call'], original_call_ast=deepcopy(candidate),
            containing_expression_ast=deepcopy(whole), first_direct_callback_ast=deepcopy(first), enclosing_main_ast=deepcopy(main),
            lexical_types_reviewed=lexical, lexical_types_bound_to_original_qualified_compiler=True, resolved_declarations=refs,
            local_patterns_definition=deepcopy(pattern), callbacks_bodies_and_written_closes_retained=True, original_diagnostic_preserved=True))
    return certificates


def prepare(args, base, pins, stack, output):
    merge, verify, read = base.merge, base.verify, base.read
    merge(pins, {str(FACTS): args.facts_sha256, str(args.peer): args.peer_sha256, str(args.closure): args.closure_sha256,
        str(base.GLEAM_COUNTS): base.GLEAM_SHA, str(base.ERLANG_COUNTS): base.ERLANG_SHA,
        str(base.PREPARATION): base.PREPARATION_SHA, str(base.SUITE): SUITE_SHA, str(base.HOST): base.HOST_SHA})
    verify(pins)
    facts, peer = read(FACTS), read(args.peer)
    assert facts['status'] == 'actual-source-meaning-reviewed-awaiting-independent-peer-and-guarded-seal' and facts['language'] == 'gleam'
    assert facts['canonical_LOC_admitted'] is facts['chart_admission'] is facts['sources_or_timings_mutated'] is facts['source_counts_overridden'] is False
    assert facts['original_gleam_replay'] == dict(path=base.GLEAM_COUNTS.relative_to(base.CLBG).as_posix(), sha256=base.GLEAM_SHA)
    assert facts['original_erlang_replay'] == dict(path=base.ERLANG_COUNTS.relative_to(base.CLBG).as_posix(), sha256=base.ERLANG_SHA)
    assert facts['actual_suite'] == dict(path=base.SUITE.relative_to(base.CLBG).as_posix(), sha256=SUITE_SHA)
    assert peer['status'] == 'accepted-exact-gleam-erlang30-source-meanings-and20-bundles' and peer['review_identity'] and peer['review_identity'] != facts['review_identity']
    assert peer['reviewed_facts'] == dict(path=str(FACTS), sha256=args.facts_sha256)
    assert peer['actual_suite'] == dict(path=str(base.SUITE), sha256=SUITE_SHA)
    assert peer['all_owner_events_and_resolution_certificates_reviewed'] is peer['four_exact_semantic_prefixes_reviewed'] is peer['all20_complete_compiler_bundles_and_totals_reviewed'] is True
    assert len(peer['profile_ids']) == len(set(peer['profile_ids'])) == 20 and set(facts['profiles']) == set(peer['profile_ids']) == base.IDS
    assert set(facts['helper_sources']) == {name+suffix for name,suffix in (('clbg_io','.gleam'),('clbg_reference','.gleam'),('clbg_process','.gleam'),('clbg_vector','.gleam'),('clbg_regex','.gleam'),('clbg_io_ffi','.erl'),('clbg_process_ffi','.erl'),('clbg_vector_ffi','.erl'),('clbg_regex_ffi','.erl'),('clbg_runner','.erl'))}
    closed = read(args.closure)
    assert closed['status'] == 'captured-original-gleam-count-and-suite-closed-receipts' and closed['actual_suite'] == peer['actual_suite']
    base.closed_jobs(closed['closed_original_jobs'], pins, stack)
    gleam, erlang, preparation, suite = read(base.GLEAM_COUNTS), read(base.ERLANG_COUNTS), read(base.PREPARATION), read(base.SUITE)
    assert gleam['qualification'] == erlang['qualification'] == dict(path=str(base.PREPARATION), sha256=base.PREPARATION_SHA)
    assert gleam['status'] == 'actual20-source-proposals-awaiting-Erlang-count-and-semantic-review'
    assert erlang['status'] == 'original-authored-Erlang-helper-count-proposals-awaiting-semantic-review'
    assert gleam['canonical_LOC_admitted'] is gleam['chart_admission'] is erlang['canonical_LOC_admitted'] is erlang['chart_admission'] is False
    for doc in (gleam, erlang, preparation): merge(pins, doc['pins'])
    merge(pins, suite['provenance']); verify(pins)
    for report, expected in ((base.GLEAM_COUNTS, 'completed-source-proposals'), (base.ERLANG_COUNTS, 'completed-helper-count-proposals')):
        path = report.with_name('terminal-pins.json'); terminal = read(path)
        assert terminal['status'] == expected and terminal['actual_common_mutex_held'] is True
        assert terminal['pins'][str(report)] == pins[str(report)] and terminal['canonical_LOC_admitted'] is False
        merge(pins, terminal['pins']); merge(pins, terminal['prerequisite_pins']); merge(pins, {str(path): base.sha(path)})
    fixture = gleam['fixture_receipt']; actual = read(fixture['path'])
    assert pins[fixture['path']] == fixture['sha256'] and actual['status'] == 'passed-awaiting-root-source-review'
    assert actual['tests_run'] == actual['expected_count'] == len(set(actual['started_ids'])) == 34
    assert sorted(actual['expected_ids']) == sorted(actual['discovered_ids']) == sorted(actual['started_ids'])
    assert actual['pins_before'] == actual['pins_after'] and actual['actual_common_mutex_held'] is actual['pin_reconciliation_passed'] is True
    assert all(type(actual[key]) is int and actual[key] == 0 for key in ('failures','errors','skipped','expected_failures','unexpected_successes'))
    for declaration, count, status, ids in ((erlang['fixture_receipt'], 21, 'passed-original-source-owner-fixtures', HELPER_IDS),
            (read(erlang['fixture_receipt']['path'])['legacy_fixture_receipt'], 5, 'passed', ['independent-bindings','tuple-data','call-arguments','list-data-and-comment','nested-block'])):
        observed = read(declaration['path'])
        assert pins[declaration['path']] == declaration['sha256'] and observed['status'] == status and type(observed['actual_cases']) is int and observed['actual_cases'] == count
        assert observed['expected_ids'] == observed['started_ids'] == ids
        assert all(type(observed[key]) is int and observed[key] == 0 for key in ('failures','errors','skips'))
        if count == 21:
            assert [row['id'] for row in observed['raw_case_receipts']] == ids and observed['legacy_sequence_metric_preserved_separately'] is True
        else:
            assert observed['metric'] == 'preserved-legacy-expression-sequence-metric-not-relabelled-as-new-owner-metric'
    binding = facts['vendor_binding']; vendor_path = base.CLBG / binding['module_path']; vendor_source = vendor_path.read_text()
    assert pins[str(vendor_path)] == binding['module_sha256'] and pins[str(base.CLBG / binding['version_path'])] == binding['version_sha256']
    assert binding['version'] == '1.0.5' and 'version = "1.0.5"' in (base.CLBG / binding['version_path']).read_text()
    assert set(binding['reviewed_declarations']) == set(VENDOR_DECLARATIONS) and binding['generic_library_boundary_preserved'] is True
    vendor = {}
    for name, text in VENDOR_DECLARATIONS.items():
        assert vendor_source.count(text) == 1
        start = vendor_source.index(text)
        vendor[name] = dict(path=str(vendor_path), sha256=binding['module_sha256'], name=name, start=start, end=start+len(text), exact_source=text, role='pinned-generic-vendor-value-service')
    counts, retained, resolved = {}, {}, {}
    assert {path for path, item in gleam['source_counts'].items() if item['source_language'] == 'erl'} == set(erlang['source_counts'])
    for path, item in gleam['source_counts'].items():
        counted = erlang['source_counts'][path] if item['source_language'] == 'erl' else item
        assert counted['source_sha256'] == item['source_sha256'] == pins[path] == pins[item['origin']]
        assert counted['origin'] == item['origin'] and counted['role'] == item['role']
        if item['source_language'] == 'erl':
            assert item['metric']['normalized_loc'] is None and item['metric']['diagnostics'] == [dict(kind='qualified-original-Erlang-AST-count-receipt-pending', offset=0)]
        source = Path(path).read_text()
        assert Path(path).read_bytes() == Path(item['origin']).read_bytes()
        profile = next((key for key, row in gleam['profiles'].items() if row['source'] == path), None)
        meaning = facts['profiles'][profile] if profile else facts['helper_sources'][Path(path).name]
        assert meaning['source_sha256'] == item['source_sha256'] and item['role'] == ('application' if profile else 'application-helper')
        assert meaning['owner_review'] and (profile is not None or meaning['role'] == item['role'])
        metric = deepcopy(counted['metric']); units = source_events(metric, source, item['source_language'])
        expected = meaning['main_normalized_loc'] if profile else meaning['normalized_loc']
        assert metric['physical_loc'] == (meaning['main_physical_loc'] if profile else meaning['physical_loc'])
        certificates = []
        if metric['diagnostics']:
            assert profile in ('gleam-regexredux-compact', 'gleam-regexredux-optimized') and expected is None
            certificates = resolve_regex(profile, metric, source, facts, vendor)
            assert units == meaning['reviewed_owner_units_after_exact_semantic_gate']
            retained[path] = deepcopy(metric)
            metric = dict(metric, normalized_loc=units, diagnostics=[], semantic_gate_resolved_by_exact_source_review=True)
            resolved[path] = dict(source_sha256=item['source_sha256'], original_metric=retained[path], certificates=certificates, event_units_unchanged=True)
        else:
            assert metric['normalized_loc'] == expected == units
        counts[path] = dict(item, metric=metric, original_count_evidence=dict(path=str(base.ERLANG_COUNTS if item['source_language'] == 'erl' else base.GLEAM_COUNTS), sha256=base.ERLANG_SHA if item['source_language'] == 'erl' else base.GLEAM_SHA),
            exact_owner_meaning=deepcopy(meaning), semantic_certificates=certificates)
    assert len(resolved) == 2 and sum(len(item['certificates']) for item in resolved.values()) == 4
    assert suite['language'] == 'gleam' and suite['status'] == 'ten-native-workloads-reconciled-awaiting-canonical-LOC-and-publication'
    assert set(suite['benchmarks']) == set(base.BENCHMARKS) and suite['provenance'][str(base.GLEAM / 'studies/full-v4/index.json')] == base.INDEX_SHA
    assert suite['canonical_full_native_processes'] == 90 and suite['candidate_sources'] == 20 and suite['actual_samples_per_source'] == 3
    assert suite['source_timing_relabels'] == suite['source_or_report_mutations'] == 0 and suite['chart_admitted'] is False
    rows = {row['id']: row for row in preparation['profiles']}
    for bench in suite['benchmarks'].values():
        candidates = [row for row in bench['profiles'].values() if row['language'] == 'gleam']
        controls = [row for row in bench['profiles'].values() if row['language'] == 'mech']
        assert len(bench['profiles']) == 3 and len(candidates) == 2 and len(controls) == 1 and {row['variant'] for row in candidates} == {'compact','optimized'}
    scored = {row['id']: row for bench in suite['benchmarks'].values() for row in bench['profiles'].values() if row['language'] == 'gleam'}
    assert len(preparation['profiles']) == 20 and set(rows) == set(scored) == set(gleam['profiles']) == base.IDS
    staged, profiles, inventories, used = {}, {}, {}, set()
    review_ref = dict(status='passed', review_identity=facts['review_identity'], evidence_path=str(FACTS), evidence_sha256=args.facts_sha256,
        independent_peer=dict(path=str(args.peer), sha256=args.peer_sha256, review_identity=peer['review_identity']))
    for identity in sorted(base.IDS):
        row, timed, replay, meaning = rows[identity], scored[identity], gleam['profiles'][identity], facts['profiles'][identity]
        assert identity == 'gleam-' + row['benchmark'] + '-' + row['variant'] and row['benchmark'] == timed['benchmark'] and row['variant'] == timed['variant']
        assert row['source'] == timed['source'] == replay['source'] and row['source_sha256'] == timed['source_sha256'] == replay['source_sha256'] == meaning['source_sha256']
        assert row['binary_sha256'] == timed['binary_sha256'] == replay['binary_sha256'] and row['source_bundle'] == timed['source_bundle']
        bound = gleam['compiler_bound_input_inventory'][identity]; actual_inventory = read(bound['path'])
        assert bound['path'] == row['compiled_input_inventory'] and bound['sha256'] == row['compiled_input_inventory_sha256'] == pins[bound['path']]
        assert actual_inventory['status'] == 'compiled-exact-authored-input-inventory' and actual_inventory['profile'] == identity
        assert actual_inventory['source'] == row['source'] and actual_inventory['source_sha256'] == row['source_sha256']
        assert actual_inventory['source_bundle'] == bound['source_bundle'] == row['source_bundle']
        assert actual_inventory['compiled_artifacts'] == bound['compiled_artifacts'] == row['compiled_artifacts'] and actual_inventory['runtime_classpath'] == bound['runtime_classpath'] == row['runtime_classpath']
        assert len(row['source_bundle']) == len({item['path'] for item in row['source_bundle']}) and row['source_bundle'][0]['path'] == row['source']
        assert [Path(item['path']).name for item in row['source_bundle'][1:]] == meaning['helper_modules']
        bundle = []
        for item in row['source_bundle']:
            value = counts[item['path']]; metric = value['metric']; used.add(item['path'])
            assert item['sha256'] == value['source_sha256'] and item['origin'] == value['origin'] and item['role'] == value['role']
            assert metric['diagnostics'] == [] and type(metric['normalized_loc']) is type(metric['physical_loc']) is int and min(metric['normalized_loc'], metric['physical_loc']) > 0
            bundle.append(dict(item, normalized_loc=metric['normalized_loc'], physical_loc=metric['physical_loc'], ownership_review=deepcopy(review_ref), unresolved_diagnostics=[]))
        physical, canonical = sum(item['physical_loc'] for item in bundle), sum(item['normalized_loc'] for item in bundle)
        assert physical == meaning['effective_physical_loc'] and canonical == meaning.get('effective_normalized_loc', meaning.get('effective_normalized_loc_after_exact_semantic_gate'))
        profiles[identity] = dict(benchmark=row['benchmark'], variant=row['variant'], source=row['source'], source_sha256=row['source_sha256'], producer_loc=None,
            physical_loc=physical, canonical_loc=canonical, source_bundle=bundle, semantic_review=deepcopy(review_ref))
        path = output / 'compiler-inputs' / (identity + '.json')
        files = [dict(path=item['path'], sha256=item['sha256']) for item in row['source_bundle']]
        staged[identity] = (path, dict(status='qualified-exact-authored-compiler-inputs', profile=identity, source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], files=files,
            original_compiler_inventory=bound, exact_original_inventory=actual_inventory, source_role_review=deepcopy(review_ref), inventory_derived_from_original_actual_compile_receipt=True))
        inventories[identity] = dict(status='qualified-exact-authored-compiler-inputs', source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], evidence_path=str(path), evidence_sha256=None, files=files)
    assert used == set(counts)
    verify(pins); output.mkdir()
    meaning_path = output / 'source-meaning-resolution.json'
    base.new(meaning_path, dict(status='reviewed-source-bound-meaning-and-four-exact-prefix-gates', source_counts=counts, preserved_original_unqualified_regex_metrics=retained,
        exact_semantic_resolutions=resolved, review=review_ref, original_gleam_replay=facts['original_gleam_replay'], original_erlang_replay=facts['original_erlang_replay'],
        actual_suite=dict(path=str(base.SUITE), sha256=SUITE_SHA), vendor_declarations=vendor, pins=deepcopy(pins),
        source_parser_executions=0, original_source_metrics_mutated=False, actual_native_subjects=0, chart_admission=False))
    pins[str(meaning_path)] = base.sha(meaning_path)
    for profile in profiles.values():
        for item in profile['source_bundle']: item['count_receipt'] = dict(path=str(meaning_path), sha256=pins[str(meaning_path)], counted_source=item['path'])
    for identity, (path, evidence) in staged.items():
        base.new(path, evidence); pins[str(path)] = base.sha(path); inventories[identity]['evidence_sha256'] = pins[str(path)]
    verify(pins)
    grammar = deepcopy(gleam['grammar_snapshot_pins'])
    packet = base.CLBG / 'loc-normalization/20261012/gleam-erlang-helpers-v1'
    for leaf in ('qualify_helpers.py','original_owner_probe.erl','erlang_owners.py','owner_fixture_cases.py','OWNER_RULES_PROPOSAL.md','REVIEW.md'):
        path = str(packet / leaf); grammar[path] = pins[path]
    overlay = dict(schema_version=1, language='gleam', status='qualified-exact-source-LOC', profile_count=20, complete_authored_source_bundles=True, unresolved=[],
        normalization_policy=dict(gleam='qualified-v2-owners-with-four-exact-source-meaning-certificates', erlang='qualified-original-owner-v1'),
        fixture_receipt=dict(path=fixture['path'], sha256=fixture['sha256'], status='passed', actual_methods=34),
        replay_receipts=[dict(path=str(base.GLEAM_COUNTS), sha256=base.GLEAM_SHA), dict(path=str(base.ERLANG_COUNTS), sha256=base.ERLANG_SHA), dict(path=str(meaning_path), sha256=pins[str(meaning_path)])],
        grammar_snapshot_pins=grammar, compiler_bound_input_inventory=inventories, profiles=profiles, pins=deepcopy(pins),
        canonical_LOC_admitted=True, chart_admission=False, original_sources_or_timings_mutated=False, actual_native_subjects=0, actual_signals_sent=0, fixture_reexecutions=0)
    path = output / 'qualified-loc-overlay.json'; base.new(path, overlay)
    print(json.dumps(dict(overlay=str(path), sha256=base.sha(path), profiles=20, exact_semantic_gates=4, native_processes=0, chart_admission=False)), flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--base-sealer-sha256', required=True); parser.add_argument('--facts-sha256', required=True)
    parser.add_argument('--peer-review', dest='peer', type=Path, required=True); parser.add_argument('--peer-sha256', required=True)
    parser.add_argument('--closed-job-pins', dest='closure', type=Path, required=True); parser.add_argument('--closed-job-pins-sha256', dest='closure_sha256', required=True)
    parser.add_argument('--revision', required=True); args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', args.revision)
    for value in (args.base_sealer_sha256,args.facts_sha256,args.peer_sha256,args.closure_sha256): assert re.fullmatch('[a-f0-9]{64}', value)
    args.peer, args.closure = args.peer.resolve(strict=True), args.closure.resolve(strict=True)
    assert args.peer.is_relative_to(HERE) and args.closure.is_relative_to(HERE)
    sys.dont_write_bytecode = True
    print('WAIT common mutex: actual Gleam/Erlang exact semantic LOC20 seal', flush=True)
    with LOCK.open('a+') as lock, ExitStack() as stack:
        fcntl.flock(lock, fcntl.LOCK_EX)
        opened, named = os.fstat(lock.fileno()), LOCK.stat(); assert (opened.st_dev, opened.st_ino) == (named.st_dev, named.st_ino)
        import hashlib
        def digest(path):
            with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()
        output, journal = HERE / args.revision, HERE / 'semantic-seal-outcomes' / args.revision
        journal.mkdir(parents=True)
        pins = {str(BASE): args.base_sealer_sha256, str(FACTS): args.facts_sha256, str(args.peer): args.peer_sha256,
            str(args.closure): args.closure_sha256, str(Path(__file__).resolve()): digest(__file__), str(NOTE): digest(NOTE),
            str(Path(sys.executable).resolve()): digest(Path(sys.executable).resolve())}
        error = None; integrity = []
        try:
            assert not output.exists()
            assert digest(BASE) == args.base_sealer_sha256
            name = 'byte_pinned_gleam_seal_helpers_v2'; assert name not in sys.modules
            spec = importlib.util.spec_from_file_location(name, BASE); base = importlib.util.module_from_spec(spec); sys.modules[name] = base; spec.loader.exec_module(base)
            assert Path(base.__file__).resolve() == BASE and digest(BASE) == args.base_sealer_sha256
            prepare(args, base, pins, stack, output)
        except BaseException as failure:
            error = dict(error=str(failure) or type(failure).__name__, error_type=type(failure).__name__)
            (journal / 'actual-error.log').write_text(traceback.format_exc()); raise
        finally:
            collected = {}; candidates = dict(pins)
            candidates.update({str(path): None for folder in (output, journal) if folder.exists() for path in folder.rglob('*') if path.is_file()})
            for path, expected in candidates.items():
                try:
                    actual = digest(path)
                    collected[path] = actual
                    assert expected is None or actual == expected
                except BaseException as failure: integrity.append(dict(path=path, error=str(failure) or type(failure).__name__, error_type=type(failure).__name__, expected_sha256=expected, actual_sha256=collected.get(path)))
            with (journal / 'outcome.json').open('x') as stream:
                json.dump(dict(status='failed-terminal-integrity' if integrity else 'failed-overlay-stage' if error else 'completed-overlay-stage', body_error=error, integrity_errors=integrity,
                    pins=collected, actual_common_mutex_held=True, actual_native_subjects=0, actual_signals_sent=0, fixture_reexecutions=0, chart_admission=False, original_sources_or_timings_mutated=False), stream, indent=2)
                stream.write('\n')
            if integrity: raise RuntimeError('Gleam exact semantic LOC terminal integrity failed')


if __name__ == '__main__': main()
