"""Five actual reviewed historical selectors; no inferred aliases or I/O."""
from copy import deepcopy

ROUTES = {
    'fasta:mech-fasta-final-compact-v2': ('mech', 'work-audit/20261009/timings.json', '/programs/fasta:mech-fasta-final-compact-v2:floating', 'work-audit/20261009/timings.json', '/programs/fasta:mech-fasta-final-compact-v2:floating'),
    'fasta:mech-fasta-final-expanded-v2': ('mech', 'fasta/investigations/20261009-floating-cdf/results.json', '/programs/mech-current-floating', 'work-audit/20261009/builds.json', '/programs/fasta:mech-fasta-final-expanded-v2:floating'),
    'fasta:rust7-floating-16-local': ('rust', 'fasta/investigations/20261009-floating-cdf/results.json', '/programs/rust7-floating-16', 'fasta/investigations/20261009-floating-cdf/results.json', '/programs/rust7-floating-16'),
    'nbody:compact-baseline': ('mech', 'nbody/experiments/20261011-shared-products/replication-50m.json', '/programs/4', 'nbody/experiments/20261011-shared-products/replication-50m.json', '/programs/4'),
    'nbody:pairwise-mass-tails': ('mech', 'nbody/experiments/20261011-shared-products/replication-50m.json', '/programs/6', 'nbody/experiments/20261011-shared-products/replication-50m.json', '/programs/6'),
}


def install(core, diagnosed):
    original_records, original_measurement = core.record_candidates, core.measurement
    active = None
    core.require(set(ROUTES) <= set(diagnosed), 'Missing actual literal-route diagnosis')
    for key, (_, _, saved, source_report, source) in ROUTES.items():
        item = diagnosed[key]
        proof = item['literal_route_diagnostic']
        core.require(item['status'] == 'v3-mapping-error' and proof['status'] == 'literal-route-proof-for-review-only'
                     and proof['saved_selector'] == saved and proof['source_report'] == source_report
                     and proof['source_selector'] == source and proof['annotation_created'] is False,
                     'Changed independently reviewed literal route: ' + key)

    def record_candidates(report, displayed):
        found = original_records(report, displayed)
        if active is None or displayed['key'] != active['key']:
            return found
        selected = [item for item in active['reports'] if report is item[0]]
        if not selected:
            return found
        core.require(len(selected) == 1, 'Ambiguous active literal report')
        _, index, row = selected[0]
        core.require(not found or len(found) == 1 and found[0][0] == index and found[0][1] is row,
                     'Literal route cannot hide another historical identity')
        return [(index, row)]

    def selected_row(history, relative, selector):
        core.require(selector.startswith('/programs/') and len(selector.split('/')) == 3, 'Nonliteral program selector')
        report = history.report(relative)
        values = report['programs']
        name = selector.split('/')[2].replace('~1', '/').replace('~0', '~')
        index = int(name) if type(values) is list else name
        row = core.pointer(report, selector)
        core.require(type(row) is dict, 'Literal selector does not identify a saved record')
        return report, index, row

    def measurement(history, displayed):
        nonlocal active
        key = displayed['key']
        if key not in ROUTES:
            return original_measurement(history, displayed)
        core.require(active is None, 'Nested literal-route measurement')
        language, relative, selector, source_relative, source_selector = ROUTES[key]
        diagnosis, proof = diagnosed[key], diagnosed[key]['literal_route_diagnostic']
        identity = {name:deepcopy(displayed.get(name)) for name in core.IDENTITY_FIELDS}
        files = [dict(path=history.data['files'][file]['path'], sha256=history.data['files'][file]['sha256']) for file in displayed['files']]
        core.require(identity == diagnosis['identity'] and files == diagnosis['files'] == proof['displayed_files'], 'Displayed literal-route identity changed')
        core.require(displayed['report'] == relative and displayed['language'] == language
                     and displayed['benchmark'] == key.split(':', 1)[0], 'Literal report/language/benchmark changed')
        report, index, saved = selected_row(history, relative, selector)
        source_report, source_index, origin = selected_row(history, source_relative, source_selector)
        core.require(saved.get('id') == proof['saved_id'] and saved.get('key') == proof['saved_key'], 'Saved literal identity changed')
        core.require(saved.get('language', language) == language and saved.get('benchmark', report.get('benchmark', displayed['benchmark'])) == displayed['benchmark'], 'Saved literal language/benchmark changed')
        core.samples(displayed, saved)
        core.require(saved['samples_seconds'] == proof['samples'] and saved.get('median_seconds') == proof['median_seconds'], 'Literal samples/median changed')
        core.require(saved.get('command') == proof['command'] and saved.get('binary_sha256') == proof['binary_sha256'], 'Literal timed binary identity changed')
        core.require(origin.get('binary', saved['command'][0]) == saved['command'][0]
                     and origin.get('binary_sha256') == proof['binary_sha256'], 'Literal build output differs from timed binary')
        core.require(origin.get('source') == proof['recorded_source'] and origin.get('source_sha256') == proof['recorded_source_sha256'], 'Literal authored source path/SHA changed')
        core.require(core.native_compile(origin, saved['command']) == proof['compile_command'], 'Literal native build command changed')
        core.require(history.source_files(displayed, origin) == files, 'Literal complete authored bundle changed')
        history.reads.read(proof['recorded_source'], proof['recorded_source_sha256'])
        history.reads.read(saved['command'][0], proof['binary_sha256'])
        reports = [(report, index, saved)]
        if source_report is not report:
            reports.append((source_report, source_index, origin))
        else:
            core.require(source_index == index and origin is saved, 'Conflicting literal selectors in one report')
        active = dict(key=key, reports=reports)
        try:
            result = original_measurement(history, displayed)
            core.require(result['saved'] is saved and result['origin'] is origin
                         and result['relative'] == relative and result['selector'] == selector
                         and result['source_relative'] == source_relative and result['origin_field'] == source_selector
                         and result['command'] == proof['command'], 'Preserved measurement selected different literal evidence')
            return result
        finally:
            active = None

    core.record_candidates, core.measurement = record_candidates, measurement
    return dict(status='five-actual-literal-routes-installed', route_count=5,
        selector_rule='exact report object and reviewed JSON pointer inside exact displayed identity',
        source_binary_sample_binding='unchanged actual proof and original measurement body',
        other_128_routes='unchanged v3 helpers and MissingHistory policy')
