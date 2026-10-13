"""Additional assertions inside two preserved fixture IDs; synthetic, no I/O.

Each method runs its full original fixture first. The guard's existing fresh
unittest snapshot, ten-ID start/outcome accounting and receipt framework stay.
"""
from copy import deepcopy
from pathlib import Path


def check_chart_key(case, core):
    displayed = dict(id='mech-binarytrees-parallel', key='binarytrees:mech-binarytrees-parallel',
                     benchmark='binarytrees', language='mech')
    row = dict(id='mech-control', key='binarytrees:mech-control', chart_key=displayed['key'],
               benchmark='binarytrees', language='mech', samples_seconds=[1., 2., 3.])
    report = dict(programs={'binarytrees:mech-control': row})
    before = deepcopy(report)
    case.assertEqual(core.record_candidates(report, displayed), [('binarytrees:mech-control', row)])
    case.assertEqual(core.record_candidates(dict(programs=[row]), displayed), [(0, row)])
    for change in (dict(chart_key='binarytrees:other'), dict(chart_key=None), dict(chart_key=True),
                   dict(chart_key=displayed['key'] + ':suffix'), dict(language='c'), dict(benchmark='nbody')):
        case.assertEqual(core.record_candidates(dict(programs={'saved': dict(row, **change)}), displayed), [])
    # Equal samples supply no alias, and two exact alias records remain
    # ambiguous. One row matching both original ID and alias is counted once.
    case.assertEqual(core.record_candidates(dict(programs={'saved': {k:v for k,v in row.items() if k != 'chart_key'}}), displayed), [])
    case.assertEqual(len(core.record_candidates(dict(programs=[row, dict(row)]), displayed)), 2)
    case.assertEqual(len(core.record_candidates(dict(programs=[dict(row, id=displayed['id'])]), displayed)), 1)
    resident = dict(displayed, id='tree:resident', key='binarytrees:tree:resident', language='mech-resident')
    saved = dict(chart_key=resident['key'], source_key='binarytrees:tree', phase='before',
                 benchmark='binarytrees', language='mech-resident')
    case.assertEqual(core.record_candidates(dict(programs=[saved]), resident), [])
    case.assertEqual(core.record_candidates(dict(programs=[dict(saved, phase='after', source_key='binarytrees:other')]), resident), [])
    case.assertEqual(core.record_candidates(dict(programs=[dict(saved, phase='after')]), resident), [(0, dict(saved, phase='after'))])
    case.assertEqual(report, before)


def check_source_bound_build(case, core):
    root = Path('/synthetic/clbg')
    source = str(root / 'binarytrees/binarytrees-parallel.mec')
    binary = str(root / 'saved/builds/binarytrees-after')
    source_sha, binary_sha = 'a' * 64, 'b' * 64
    displayed = dict(id='mech-binarytrees-parallel', key='binarytrees:mech-binarytrees-parallel',
                     benchmark='binarytrees', language='mech', files=['source'],
                     report='synthetic/timings.json', source_report='synthetic/builds.json',
                     samples=[1., 2., 3.], seconds=2.)
    saved = dict(key='binarytrees:mech-control', chart_key=displayed['key'], language='mech', benchmark='binarytrees',
                 source=source, source_sha256=source_sha, binary=binary, binary_sha256=binary_sha,
                 command=[binary, '{n}'], samples_seconds=[1., 2., 3.], median_seconds=2.)
    correct = dict(source=source, source_sha256=source_sha, binary=binary, binary_sha256=binary_sha,
                   build_command=['/synthetic/compiler', source, '-o', binary])
    other = dict(correct, source=str(root / 'saved/current-before/binarytrees-parallel.mec'), source_sha256='c' * 64)
    reports = {displayed['report']: dict(programs={'binarytrees:mech-control': saved}),
               displayed['source_report']: dict(programs=[dict(builds={'after': correct, 'current-before': other})])}

    class History:
        def __init__(self):
            self.root = root
            self.data = dict(files={'source': dict(path='binarytrees/binarytrees-parallel.mec', sha256=source_sha)})
        def report(self, path):
            return reports[path]

    history = History(); before = deepcopy(reports)
    # The retained binary-only helper sees two equal output artifacts; the
    # actual measurement must select only the authored after-source receipt.
    case.assertEqual(len(core.build_candidates(reports[displayed['source_report']], binary, binary_sha)), 2)
    actual = core.measurement(history, displayed)
    case.assertEqual(actual['origin'], correct)
    case.assertEqual(actual['origin_field'], '/programs/0/builds/after')
    case.assertEqual(actual['saved'], saved)
    case.assertEqual(reports, before)
    for rejected in (other, dict(correct, source=other['source']), dict(correct, source_sha256='c' * 64),
                     {k:v for k,v in correct.items() if k != 'source'},
                     {k:v for k,v in correct.items() if k != 'source_sha256'},
                     dict(correct, binary='/synthetic/other-output'), dict(correct, binary_sha256='d' * 64)):
        reports[displayed['source_report']] = dict(programs=[rejected])
        with case.assertRaises(core.MissingHistory):
            core.measurement(history, displayed)
    reports[displayed['source_report']] = dict(programs=[correct, dict(correct)])
    with case.assertRaises(core.MissingHistory):
        core.measurement(history, displayed)
    # A single recorded source cannot satisfy a whole two-file authored bundle.
    history.data['files']['helper'] = dict(path='binarytrees/helper.mec', sha256='e' * 64)
    reports[displayed['source_report']] = dict(programs=[correct])
    with case.assertRaises(core.MissingHistory):
        core.measurement(history, dict(displayed, files=['source', 'helper']))
    bundle = dict(correct, source=dict(sha256={source: source_sha, str(root/'binarytrees/helper.mec'): 'e' * 64}))
    reports[displayed['source_report']] = dict(programs=[bundle])
    case.assertEqual(core.measurement(history, dict(displayed, files=['source', 'helper']))['origin'], bundle)
    reports[displayed['source_report']] = dict(programs=[dict(bundle, source=dict(sha256={source: source_sha}))])
    with case.assertRaises(core.MissingHistory):
        core.measurement(history, dict(displayed, files=['source', 'helper']))
    # Every exceptional lookup must clear its private context.
    reports[displayed['source_report']] = before[displayed['source_report']]
    case.assertEqual(core.measurement(history, displayed)['origin'], correct)


def extend(module, core):
    bindings = {}
    for name, check in (
            ('test_duplicate_or_wrong_language_record_is_never_a_sample_join', check_chart_key),
            ('test_native_link_requires_exact_output_and_cargo_artifact_pin', check_source_bound_build)):
        original = getattr(module.HistoryEvidence, name)
        def extended(case, original=original, check=check):
            original(case)
            check(case, core)
        extended.__name__ = name
        setattr(module.HistoryEvidence, name, extended)
        bindings[name] = dict(original_method_source=original.__code__.co_filename,
                              extension_method_source=extended.__code__.co_filename,
                              extra_assertion_source=check.__code__.co_filename,
                              full_original_method_executed_first=True)
    return bindings
