"""Map existing paired LOC display fields to exact application source pins.

This read-only mapping never admits a study, recounts code, changes a producer
LOC/ratio, or touches timing samples. Apply a separate display overlay only
after the referenced source identities have passed the shared LOC audit.
"""
import json
from pathlib import Path


LOC_RATIO_FIELDS = {
    'control_loc': 'control_loc_x_mech',
    'fastest_loc': 'fastest_loc_x_mech',
    'smallest_loc': 'smallest_loc_x_mech',
}
NODE_PI_REPORT = 'pidigits-work-audit/20261011/results/timings.json'
NODE_PI_AUDIT = 'pidigits-work-audit/20261011/audit.json'
NODE_PI_CONTROL_SNAPSHOT = 'pidigits-work-audit/20261011/evidence/control-audit-snapshot.json'
FROZEN_SUMMARY_SCHEMAS = {
    'zig-odin-roc-elm-comparison/20261010/results/summary.json': dict(
        snapshot='zig-odin-roc-elm-comparison/20261010/results/explorer-before.json',
        snapshot_pin='previous_explorer_sha256', selector='benchmark'),
    'remaining-languages-comparison/20261011/results/summary.json': dict(
        snapshot='remaining-languages-comparison/20261011/evidence/site-before.json',
        snapshot_pin='preserved_explorer_sha256', selector='timing-report-and-benchmark'),
}


def collect_display_comparisons(root, current_data):
    """Collect display bindings from already imported comparisons only.

    Unfinished study manifests and reports are deliberately not discovered.
    Current study importers remain responsible for completion/admission. A
    missing historical denominator identity is reported, never replaced with
    the current Mech endpoint or inferred from an equal numerical LOC count.
    """
    root = Path(root)
    documents, report_paths, sources, comparisons, unresolved = {}, [], {}, [], []

    def path_for(relative):
        path = Path(relative)
        return path if path.is_absolute() else root / path

    def load(relative):
        if not relative:
            return None
        path = path_for(relative)
        name = str(path)
        if name not in documents:
            documents[name] = json.loads(path.read_text()) if path.is_file() else None
            if documents[name] is not None:
                try:
                    report_paths.append(path.relative_to(root).as_posix())
                except ValueError:
                    report_paths.append(str(path))
        return documents[name]

    def ref_from_row(row, origin):
        if not isinstance(row, dict):
            return None
        source, digest = row.get('source'), row.get('source_sha256')
        if not isinstance(source, str) or not isinstance(digest, str) or len(digest) != 64:
            return None
        return dict(path=str(path_for(source)), sha256=digest, language='mech',
                    identity_evidence=origin, control_key=row.get('key', row.get('control_key')))

    def refs_from_program(program, files, origin):
        result = []
        if not program:
            return result
        for identity in program.get('files', []):
            file = files.get(identity)
            if not file or file.get('role') == 'runtime-support':
                continue
            digest, source = file.get('sha256', identity), file.get('path')
            if source and isinstance(digest, str) and len(digest) == 64:
                result.append(dict(path=str(path_for(source)), sha256=digest,
                    language=program.get('language'), identity_evidence=origin))
        return result

    def baseline_ref(audit, benchmark, role, audit_path):
        if not audit:
            return None
        baseline = audit.get('baseline_endpoints', {}).get(benchmark, {})
        key = baseline.get(role + '_key')
        if not key:
            return None
        # The snapshot binds the historical endpoint key to source bytes;
        # retaining only its producer LOC number is not sufficient.
        snapshot_path = audit.get('baseline_snapshot')
        if isinstance(snapshot_path, dict):
            snapshot_path = snapshot_path.get('path')
        snapshot = load(snapshot_path) if snapshot_path else None
        if snapshot:
            program = snapshot.get('programs', {}).get(key)
            refs = refs_from_program(program, snapshot.get('files', {}),
                dict(source_audit=audit_path, baseline_snapshot=snapshot_path, baseline_key=key))
            if len(refs) == 1:
                return refs[0]
        # Some producers record complete baseline source objects directly.
        for container in (audit.get('controls', []), audit.get('programs', [])):
            rows = container.values() if isinstance(container, dict) else container
            for row in rows:
                if row.get('key') == key or row.get('published_chart_key') == key:
                    ref = ref_from_row(row, dict(source_audit=audit_path, baseline_key=key))
                    if ref:
                        return ref
        return None

    def node_pi_control_ref(program, paired):
        """Bind Node's two producer LOC fields to its one measured control."""
        if (program.get('key') != 'pidigits:pidigits-node-2-two-extracts' or
                program.get('report') != NODE_PI_REPORT or
                program.get('source_report') != NODE_PI_AUDIT):
            return None
        report, snapshot = load(NODE_PI_REPORT), load(NODE_PI_CONTROL_SNAPSHOT)
        if not report or not snapshot:
            return None
        assert report.get('status') == 'complete', 'Node pi paired report is incomplete'
        controls = [row for row in snapshot.get('controls', [])
                    if row.get('benchmark') == 'pidigits' and row.get('language') == 'mech']
        assert len(controls) == 1, 'Node pi requires one exact frozen pi control'
        control = controls[0]
        measured_control = report['programs'][control['key']]
        for field in ('key', 'source', 'source_sha256', 'loc'):
            assert measured_control[field] == control[field], ('Node pi control identity differs', field)
        producer = report['comparison']
        assert producer['mech_loc'] == paired['fastest_loc'] == paired['smallest_loc'] == control['loc'], 'Node pi control LOC differs'
        assert producer['mech_seconds'] == paired['seconds'] == measured_control['median_seconds'], 'Node pi control timing differs'
        assert producer['time_x_mech'] == paired['time_x_mech'], 'Node pi time ratio differs'
        assert producer['loc_x_mech'] == paired['fastest_loc_x_mech'] == paired['smallest_loc_x_mech'], 'Node pi producer LOC ratio differs'
        # The producer explicitly computed both labels from this same timed
        # control. Its snapshot's separate compact35 endpoint is not involved.
        return ref_from_row(control, dict(program_key=program['key'],
            control_snapshot=NODE_PI_CONTROL_SNAPSHOT,
            expected_snapshot_sha256=report['control_audit_sha256'],
            comparison_report=NODE_PI_REPORT, control_key=control['key'],
            binding='Both producer LOC fields explicitly use the single measured pi control.'))

    def frozen_summary_ref(program, paired, role):
        """Resolve a historical profile through its producer's pinned snapshot."""
        summary_path = program.get('loc_report')
        schema = FROZEN_SUMMARY_SCHEMAS.get(summary_path)
        if not schema:
            return None
        summary = load(summary_path)
        snapshot_path = schema['snapshot']
        snapshot = load(snapshot_path)
        if not summary or not snapshot:
            return None
        assert summary.get('status') == 'passed', 'Paired LOC producer summary is incomplete'
        snapshot_digest = summary[schema['snapshot_pin']]
        assert isinstance(snapshot_digest, str) and len(snapshot_digest) == 64, 'Missing historical snapshot pin'
        benchmark = program['benchmark']
        if schema['selector'] == 'benchmark':
            selector = benchmark
        else:
            # Later compiler confirmations can preserve an older paired LOC
            # comparison. Its explicit previous measurement identifies the
            # original producer's report; never select by matching a number.
            previous_report = program.get('previous_measurement', {}).get('report')
            producer_report = previous_report or program.get('report')
            if not producer_report:
                return None
            selector = Path(producer_report).name + ':' + benchmark
        control = summary.get('controls', {}).get(selector)
        if not control:
            return None
        bench = next((row for row in snapshot.get('benchmarks', []) if row.get('key') == benchmark), None)
        group = next((row for row in bench.get('groups', []) if row.get('language') == 'mech'), None) if bench else None
        if not group:
            return None
        fast = snapshot['programs'][group['fast']]
        small = snapshot['programs'][group['small']]
        assert control['chart_key'] == group['fast'], 'Historical fast endpoint key differs'
        assert control['fastest_loc'] == fast['loc'], 'Historical fast producer LOC differs'
        assert control['smallest_loc'] == small['loc'], 'Historical compact producer LOC differs'
        assert control['source_sha256'] in fast['files'], 'Historical fast source pin differs'
        if 'smallest_key' in control:
            assert control['smallest_key'] == group['small'], 'Historical compact endpoint key differs'
        if 'smallest_source_sha256' in control:
            assert sorted(control['smallest_source_sha256']) == sorted(small['files']), 'Historical compact source pins differ'
        field = role + '_loc'
        assert paired[field] == control[field], 'Displayed paired LOC does not match its frozen producer'
        assert paired['seconds'] == control['seconds'], 'Displayed paired timing control does not match its frozen producer'
        endpoint_key = group['fast' if role == 'fastest' else 'small']
        endpoint = snapshot['programs'][endpoint_key]
        refs = refs_from_program(endpoint, snapshot.get('files', {}), dict(
            program_key=program['key'], producer_summary=summary_path,
            control_snapshot=snapshot_path, expected_snapshot_sha256=snapshot_digest,
            control_selector=selector, control_key=control['key'],
            historical_endpoint_key=endpoint_key, role=role,
            paired_source_report=program.get('source_report'),
            producer_timing_report=control.get('timing_report', program.get('previous_measurement', {}).get('report', program.get('report')))))
        assert len(refs) == 1 and endpoint.get('language') == 'mech', 'Historical paired denominator needs one complete Mech application source'
        return refs[0]

    def remember(ref, benchmark, count, program_key, field):
        identity = ref['sha256']
        source = sources.setdefault(identity, dict(path=ref['path'], sha256=identity,
            language='mech', benchmarks=[], producer_locs=[], uses=[]))
        if benchmark not in source['benchmarks']:
            source['benchmarks'].append(benchmark)
        if count not in source['producer_locs']:
            source['producer_locs'].append(count)
        source['uses'].append(dict(program_key=program_key, field_path=field))

    for key, program in current_data.get('programs', {}).items():
        paired = program.get('mech_comparison')
        if not isinstance(paired, dict):
            continue
        benchmark = program['benchmark']
        prefix = ['programs', key, 'mech_comparison']
        audit_path = paired.get('source_audit') or program.get('source_report')
        audit = load(audit_path) if audit_path else None
        direct = ref_from_row(dict(paired, key=paired.get('control_key')),
            dict(program_key=key, comparison_source_audit=audit_path,
                 comparison_report=paired.get('report'), control_key=paired.get('control_key')))
        compact = paired.get('compact_source_control')
        compact_ref = ref_from_row(compact,
            dict(program_key=key, field='mech_comparison.compact_source_control',
                 comparison_source_audit=audit_path, comparison_report=paired.get('report')))
        node_control_ref = node_pi_control_ref(program, paired)
        bindings = []

        def bind(field, count, ref, ratio_field=None, nested=False):
            field_path = prefix + (['compact_source_control'] if nested else []) + [field]
            if not ref:
                unresolved.append(dict(program_key=key, benchmark=benchmark, field_path=field_path,
                    producer_loc=count, reason='Historical denominator source pin is missing; do not substitute a current endpoint or infer identity from equal LOC.',
                    source_audit=audit_path, comparison_report=paired.get('report')))
                return
            bindings.append(dict(field_path=field_path, producer_loc=count,
                ratio_path=prefix + [ratio_field] if ratio_field else None,
                producer_ratio=paired.get(ratio_field) if ratio_field else None,
                denominator_source=ref))
            remember(ref, benchmark, count, key, field_path)

        for field, ratio_field in LOC_RATIO_FIELDS.items():
            if field not in paired:
                continue
            if field == 'control_loc':
                ref = direct
            elif node_control_ref and field in ('fastest_loc', 'smallest_loc'):
                ref = node_control_ref
            elif field == 'smallest_loc' and compact_ref:
                ref = compact_ref
            else:
                role = 'fastest' if field == 'fastest_loc' else 'smallest'
                ref = baseline_ref(audit, benchmark, role, audit_path)
                if ref is None:
                    ref = frozen_summary_ref(program, paired, role)
                # The new Lua-family schema explicitly names a fast timed
                # source and a separate compact source inside the comparison.
                if ref is None and field == 'fastest_loc' and compact_ref:
                    ref = direct
            bind(field, paired[field], ref, ratio_field)
        if isinstance(compact, dict) and 'loc' in compact:
            bind('loc', compact['loc'], compact_ref, nested=True)
        if bindings:
            comparisons.append(dict(program_key=key, benchmark=benchmark,
                language=program['language'], numerator_loc=program['loc'],
                numerator_sources=refs_from_program(program, current_data.get('files', {}),
                    dict(program_key=key, report=program.get('report'), source_report=program.get('source_report'))),
                comparison_report=paired.get('report'), comparison_source_audit=audit_path,
                bindings=bindings,
                overlay_rule='Resolve numerator and denominator LOC by exact verified source hashes; add display-only LOC and LOC ratios. Preserve producer LOC/ratios, seconds, time ratios and every sample.'))

    return dict(comparisons=comparisons, source_files=sorted(sources.values(), key=lambda s:s['sha256']),
                unresolved=unresolved, report_paths=list(dict.fromkeys(report_paths)),
                scope='Already imported paired comparisons only; no pending-study admission or timing changes.')
