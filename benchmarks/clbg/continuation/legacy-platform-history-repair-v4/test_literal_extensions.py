"""Exact-five assertions inside two existing IDs; no filesystem or commands."""
from copy import deepcopy
from pathlib import Path

CLBG = Path(__file__).resolve().parents[2]


def synthetic(core, routes, diagnosed, key):
    proof = diagnosed[key]['literal_route_diagnostic']
    language, relative, selector, source_relative, source_selector = routes.ROUTES[key]
    displayed = deepcopy(diagnosed[key]['identity'])
    displayed['files'] = ['file-' + str(i) for i in range(len(proof['displayed_files']))]
    data = dict(files={name:deepcopy(value) for name, value in zip(displayed['files'], proof['displayed_files'])})
    saved = dict(id=proof['saved_id'], key=proof['saved_key'], language=language,
        benchmark=displayed['benchmark'], samples_seconds=deepcopy(proof['samples']),
        median_seconds=proof['median_seconds'], command=deepcopy(proof['command']),
        binary_sha256=proof['binary_sha256'])
    origin = dict(source=proof['recorded_source'], source_sha256=proof['recorded_source_sha256'],
        binary=proof['command'][0], binary_sha256=proof['binary_sha256'],
        compile_command=deepcopy(proof['compile_command']))
    if relative == source_relative:
        saved.update(origin)
        origin = saved
    reports = {}
    def put(report, pointer, row):
        name = pointer.split('/')[2]
        if name.isdecimal():
            values = report.setdefault('programs', [{} for _ in range(int(name) + 1)])
            values[int(name)] = row
        else:
            report.setdefault('programs', {})[name] = row
    put(reports.setdefault(relative, {}), selector, saved)
    put(reports.setdefault(source_relative, {}), source_selector, origin)
    allowed = {str(CLBG / item['path']):item['sha256'] for item in proof['displayed_files']}
    allowed.update({proof['recorded_source']:proof['recorded_source_sha256'], proof['command'][0]:proof['binary_sha256']})
    class Reads:
        def __init__(self): self.calls = []
        def read(self, path, digest=None):
            assert allowed[str(path)] == digest
            self.calls.append((str(path), digest))
            return b''
    class History:
        root = CLBG
        source_files = core.History.source_files
        def __init__(self): self.data, self.reads = data, Reads()
        def report(self, relative): return reports[relative]
    return History(), displayed, saved, origin, reports


def check_literal_identity(case, core, routes, diagnosed):
    for key in routes.ROUTES:
        history, displayed, saved, origin, reports = synthetic(core, routes, diagnosed, key)
        before = deepcopy((displayed, reports, history.data))
        actual = core.measurement(history, displayed)
        case.assertIs(actual['saved'], saved)
        case.assertIs(actual['origin'], origin)
        case.assertEqual(actual['selector'], routes.ROUTES[key][2])
        case.assertEqual(actual['origin_field'], routes.ROUTES[key][4])
        case.assertEqual((displayed, reports, history.data), before)
        # The literal pointer supplies no ambient alias outside measurement.
        case.assertEqual(core.record_candidates(reports[displayed['report']], displayed), [])
        for change in (dict(report='unreviewed/report.json'), dict(language='cpp'),
                       dict(benchmark='spectralnorm'), dict(samples=[1., 2., 3.]), dict(seconds=99.)):
            with case.assertRaises(ValueError): core.measurement(history, dict(displayed, **change))
        for change in (dict(id='unreviewed-row'), dict(key='unreviewed-row'), dict(language='cpp'), dict(benchmark='other'),
                       dict(samples_seconds=[1., 2., 3.]), dict(median_seconds=99.)):
            current, row, timed, _, _ = synthetic(core, routes, diagnosed, key)
            timed.update(change)
            with case.assertRaises(ValueError): core.measurement(current, row)
        # An existing exact displayed identifier cannot be hidden by a route.
        current, row, timed, _, values = synthetic(core, routes, diagnosed, key)
        competitor = dict(timed, id=row['id'], key=row['key'])
        pool = values[row['report']]['programs']
        if type(pool) is list: pool.append(competitor)
        else: pool['extra-exact-display-identity'] = competitor
        with case.assertRaises(ValueError): core.measurement(current, row)
        case.assertEqual(core.measurement(history, displayed)['selector'], routes.ROUTES[key][2])


def check_literal_source_binary(case, core, routes, diagnosed):
    for key in routes.ROUTES:
        proof = diagnosed[key]['literal_route_diagnostic']
        for change in (dict(source=proof['recorded_source'] + '.other'), dict(source_sha256='0' * 64),
                       dict(binary=proof['command'][0] + '.other'), dict(binary_sha256='0' * 64),
                       dict(compile_command=['compiler', '-o', proof['command'][0] + '.other'])):
            history, displayed, _, origin, _ = synthetic(core, routes, diagnosed, key)
            origin.update(change)
            with case.assertRaises(ValueError): core.measurement(history, displayed)
        history, displayed, timed, _, _ = synthetic(core, routes, diagnosed, key)
        timed['command'] = [proof['command'][0] + '.other', '{n}']
        with case.assertRaises(ValueError): core.measurement(history, displayed)
        history, displayed, _, _, _ = synthetic(core, routes, diagnosed, key)
        history.data['files']['extra'] = dict(path='unreviewed/helper.mec', sha256='0' * 64)
        with case.assertRaises(ValueError): core.measurement(history, dict(displayed, files=displayed['files'] + ['extra']))
        with case.assertRaises(ValueError): core.measurement(history, dict(displayed, files=[]))
        history, displayed, _, _, _ = synthetic(core, routes, diagnosed, key)
        history.data['files'][displayed['files'][0]]['sha256'] = '0' * 64
        with case.assertRaises(ValueError): core.measurement(history, displayed)
        history, displayed, _, _, _ = synthetic(core, routes, diagnosed, key)
        actual = core.measurement(history, displayed)
        case.assertIn((proof['recorded_source'], proof['recorded_source_sha256']), history.reads.calls)
        case.assertIn((proof['command'][0], proof['binary_sha256']), history.reads.calls)
        case.assertEqual(actual['command'], proof['command'])


def extend(module, core, routes, diagnosed):
    bindings = {}
    for name, check in (
        ('test_duplicate_or_wrong_language_record_is_never_a_sample_join', check_literal_identity),
        ('test_native_link_requires_exact_output_and_cargo_artifact_pin', check_literal_source_binary)):
        original = getattr(module.HistoryEvidence, name)
        def extended(case, original=original, check=check):
            original(case)
            check(case, core, routes, diagnosed)
        extended.__name__ = name
        setattr(module.HistoryEvidence, name, extended)
        bindings[name] = dict(prior_method_source=original.__code__.co_filename,
            extension_method_source=extended.__code__.co_filename, extra_assertion_source=check.__code__.co_filename,
            full_original_and_v3_methods_executed_first=True)
    return bindings
