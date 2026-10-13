"""Bridge exact qualified Gleam owner evidence to the original ready record.

The record callback receives the caller's SHA-checked actual meaning receipt
and already verified source text. Original admission I/O and all original
validators remain in the copied service; this callback changes only the
presentation schema and performs no source counting or runtime operation.
"""
from copy import deepcopy
from pathlib import Path

FORMAT = 'source-bound-canonical-owner-audit-v1'


def admission_service(ready, meaning, reference):
    require = ready.require
    require(meaning['status'] == 'reviewed-source-bound-meaning-and-four-exact-prefix-gates' and
        meaning['source_parser_executions'] == meaning['actual_native_subjects'] == 0 and
        meaning['original_source_metrics_mutated'] is meaning['chart_admission'] is False,
        'Different actual qualified owner meaning receipt')
    counts = meaning['source_counts']
    require(isinstance(counts, dict) and len(counts) == 104, 'Exact104 counted compiler inputs required')
    seen_sources, seen_programs = set(), set()

    def record(language, meta, item):
        require(language == 'gleam' and 'expansions' not in item['metric'],
            'Schema bridge is restricted to the unchanged Gleam aggregate overlay')
        require(item['row']['key'] not in seen_programs, 'Duplicate Gleam record projection')
        sources = []
        for source in item['source_files']:
            path = source['path']; original = counts[path]; metric = original['metric']
            receipt = source['count_receipt']
            owner = original['exact_owner_meaning']
            if source['role'] == 'application':
                expected, physical = owner['main_normalized_loc'], owner['main_physical_loc']
                if expected is None:
                    resolution = meaning['exact_semantic_resolutions'][path]
                    preserved = meaning['preserved_original_unqualified_regex_metrics'][path]
                    require(resolution['source_sha256'] == source['sha256'] and
                        resolution['original_metric'] == preserved and preserved['normalized_loc'] is None and
                        resolution['event_units_unchanged'] is True and
                        resolution['certificates'] == original['semantic_certificates'] and
                        metric['canonical_events'] == preserved['canonical_events'] and
                        metric['expansions'] == preserved['expansions'] and
                        metric['semantic_gate_resolved_by_exact_source_review'] is True,
                        'Resolved regex owners differ from their actual retained evidence')
                    expected = owner['reviewed_owner_units_after_exact_semantic_gate']
            else:
                expected, physical = owner['normalized_loc'], owner['physical_loc']
            require(receipt == dict(reference, counted_source=path) and
                source['sha256'] == original['source_sha256'] and
                source['origin'] == original['origin'] and source['role'] == original['role'] and
                source['normalized_loc'] == metric['normalized_loc'] == expected and
                source['physical_loc'] == metric['physical_loc'] == physical and
                metric['diagnostics'] == [], 'Another source, count or unresolved meaning in record bridge')
            events, expansions, code = metric['canonical_events'], metric['expansions'], source['code']
            require(events and type(metric['normalized_loc']) is int and
                sum(event['units'] for event in events) == metric['normalized_loc'], 'Actual owner units differ')
            for event in events:
                require(type(event['start']) is type(event['end']) is type(event['units']) is int and
                    0 <= event['start'] < event['end'] <= len(code) and event['units'] in (0, 1),
                    'Invalid retained canonical owner span')
            language_name = original['source_language']
            if language_name == 'gleam':
                require(len(expansions) == len(events), 'Incomplete actual Gleam owner expansions')
                for event, expansion in zip(events, expansions):
                    require({key:value for key,value in expansion.items() if key != 'original_source'} == event and
                        0 <= event['full_owner_start'] < event['full_owner_end'] <= len(code) and
                        code[event['full_owner_start']:event['full_owner_end']] == expansion['original_source'],
                        'Actual Gleam expansion is not bound to its written owner')
            else:
                require(language_name == 'erl', 'Unexpected counted source language')
                rows = code.splitlines()
                for event in events:
                    require(code[event['start']:event['end']] == event['original_source'] and
                        (event['units'] == 1 or event['kind'] == 'structural-suffix'),
                        'Actual Erlang owner is not bound to original source')
                require(metric['physical_loc'] + sum(expansion['extra_lines'] for expansion in expansions) ==
                    metric['normalized_loc'], 'Actual Erlang line expansion total differs')
                for expansion in expansions:
                    require(type(expansion['line']) is type(expansion['units']) is type(expansion['extra_lines']) is int and
                        1 <= expansion['line'] <= len(rows) and expansion['original'] == rows[expansion['line'] - 1] and
                        expansion['extra_lines'] == expansion['units'] - 1,
                        'Actual Erlang expansion is not its recorded source line')
                require([event for expansion in expansions for event in expansion['canonical_events']] == events,
                    'Erlang line expansion omitted or changed an actual owner')
            sources.append(dict(path=path, name=Path(path).name, sha256=source['sha256'], origin=source['origin'],
                role=source['role'], source_language=language_name, normalized_loc=metric['normalized_loc'],
                physical_loc=metric['physical_loc'], count_receipt=deepcopy(receipt),
                canonical_events=deepcopy(events), expansions=deepcopy(expansions)))
            seen_sources.add(path)
        require([source['path'] for source in sources] == [source['path'] for source in item['metric']['source_bundle']] and
            sum(source['normalized_loc'] for source in sources) == item['metric']['normalized_loc'] and
            sum(source['physical_loc'] for source in sources) == item['metric']['physical_loc'],
            'Complete actual source bundle is required for record projection')
        # Only the private local record view receives the missing schema member.
        # Its value is explicitly per-source evidence, never fabricated line extras.
        view = dict(item, metric=dict(item['metric'], expansions=deepcopy(sources)))
        result = ready.program_record(language, meta, view)
        result.update(loc_expansion_format=FORMAT, loc_owner_evidence=deepcopy(reference))
        seen_programs.add(item['row']['key'])
        return result

    namespace = dict(ready.prepare_ready.__globals__)
    namespace['program_record'] = record
    namespace['prepare_ready'] = ready.private_function(ready.prepare_ready, namespace)
    admit = ready.private_function(ready.add_ready_native_languages, namespace)

    def complete():
        require(len(seen_programs) == 20 and seen_sources == set(counts),
            'Schema bridge did not preserve all20 records and all104 actual inputs')
        return dict(status='exact-original-owner-evidence-projected-without-source-count-rerun',
            source_profiles=20, counted_compiler_inputs=104, owner_receipt=deepcopy(reference),
            record_format=FORMAT, original_qualified_overlay_mutated=False,
            original_ready_source_mutated=False, invented_line_expansions=0, fixture_reexecutions=0)

    return admit, complete
