"""Literal chart aliases and source-bound Mech build lookup; no I/O or probes.

The caller installs these narrow functions in the independently pinned core.
The original measurement/source/sample/confidence implementation still runs.
"""
from pathlib import Path


def authored_bundle(core, history, displayed):
    keys = displayed.get('files')
    core.require(type(keys) is list and keys and len(keys) == len(set(keys)),
                 'Missing exact displayed source bundle for native build lookup')
    required = []
    for key in keys:
        item = history.data['files'][key]
        source = item.get('path')
        core.require(type(source) is str and source and not Path(source).is_absolute()
                     and '..' not in Path(source).parts and core.digest(item.get('sha256')),
                     'Invalid exact displayed source path/SHA for native build lookup')
        required.append((str(history.root / source), item['sha256']))
    core.require(len(required) == len(set(required)), 'Duplicate authored source binding')
    return tuple(sorted(required))


def recorded_bundle(core, build):
    # Read only these explicit saved source shapes. No basename, resolve,
    # suffix, digest-only or binary-equality inference supplies a source path.
    source = build.get('source')
    if type(source) is str and core.digest(build.get('source_sha256')):
        values = [(source, build['source_sha256'])]
    elif type(source) is dict and type(source.get('sha256')) is dict:
        values = list(source['sha256'].items())
    elif type(source) is dict and type(source.get('path')) is str and core.digest(source.get('sha256')):
        values = [(source['path'], source['sha256'])]
    else:
        return None
    if not values or not all(type(path) is str and Path(path).is_absolute()
                             and '..' not in Path(path).parts and core.digest(digest)
                             for path, digest in values):
        return None
    if len(values) != len(set(values)):
        return None
    return tuple(sorted(values))


def install(core):
    original_records = core.record_candidates
    original_builds = core.build_candidates
    original_measurement = core.measurement
    active_bundle = None

    def record_candidates(report, displayed):
        matches = original_records(report, displayed)
        values = report.get('programs')
        items = enumerate(values) if type(values) is list else values.items() if type(values) is dict else ()
        for index, row in items:
            if type(row) is not dict or type(row.get('chart_key')) is not str or row['chart_key'] != displayed['key']:
                continue
            # An explicit alias does not weaken the independently saved
            # resident after-phase/source-key contract.
            if displayed['language'] == 'mech-resident' and not (
                    row.get('source_key') == displayed['key'].removesuffix(':resident') and row.get('phase') == 'after'):
                continue
            if (row.get('language', displayed['language']) == displayed['language'] and
                    row.get('benchmark', report.get('benchmark', displayed['benchmark'])) == displayed['benchmark'] and
                    not any(prior_index == index for prior_index, _ in matches)):
                matches.append((index, row))
        return matches

    def build_candidates(value, binary, digest_value, path=''):
        if active_bundle is not None and not (
                type(binary) is str and Path(binary).is_absolute() and core.digest(digest_value)):
            return []
        found = original_builds(value, binary, digest_value, path)
        if active_bundle is None:
            # The preserved standalone helper is also exercised directly by
            # the old pure tests. Every actual generic-Mech measurement below
            # installs the complete displayed authored bundle before lookup.
            return found
        return [(field, build) for field, build in found
                if recorded_bundle(core, build) == active_bundle]

    def measurement(history, displayed):
        nonlocal active_bundle
        core.require(active_bundle is None, 'Unexpected nested native build measurement')
        needs_build_context = displayed['language'] == 'mech' and displayed['report'] not in (core.ARRAY, core.APPLICATION)
        if needs_build_context:
            active_bundle = authored_bundle(core, history, displayed)
        try:
            return original_measurement(history, displayed)
        finally:
            active_bundle = None

    core.record_candidates = record_candidates
    core.build_candidates = build_candidates
    core.measurement = measurement
    return dict(record_selector='original identifiers plus exact literal chart_key; resident after-phase unchanged',
                native_build_selector='original binary path/SHA plus complete displayed authored path/SHA bundle',
                actual_measurement_body='unchanged original measurement',
                standalone_pure_build_helper='unchanged outside an actual measurement context',
                source_binding_absence='MissingHistory; pending/unclassified, no binary-only borrowing')
