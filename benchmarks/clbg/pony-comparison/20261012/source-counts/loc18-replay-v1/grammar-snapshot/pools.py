"""Collect the admitted legacy measured pool without pruning to LOC endpoints.

This module reads saved metadata only. It neither runs programs nor hashes or
lexes sources. Consumers must verify the supplied source identities themselves.
Historical reports, samples and source files are never changed.
"""
import json
import math
from pathlib import Path


LANGUAGES = frozenset({
    'c', 'cpp', 'rust', 'go', 'java', 'chapel', 'julia', 'lua', 'luajit',
    'python', 'ruby', 'perl', 'php', 'javascript',
})
BASE_REPORT = 'retime/results/apple-m3-ac-normal-20261008.json'
SCRIPTING_REPORTS = (
    'fast-scripting/results/apple-m3-ac-normal-20261009.json',
    'fast-scripting/results/runtime-policies.json',
)
FINAL_REPORTS = (
    ('regexredux/results/apple-m3-ac-normal-20261009.json', 'regexredux'),
    ('pidigits/results/apple-m3-ac-normal-20261009.json', 'pidigits'),
    ('remaining-conformance/signed-division-confirmation.json', None),
)
NBODY_INITIAL = 'nbody/experiments/20261009-local-simd/final-confirmation-50m.json'
NBODY_LATEST = 'nbody/experiments/20261011-shared-products/replication-50m.json'
NBODY_SUMMARY = 'nbody/experiments/20261011-shared-products/summary.json'
FASTA_REPORT = 'fasta/investigations/20261009-floating-cdf/results.json'
NODE_REPORT = 'pidigits-work-audit/20261011/results/timings.json'
NODE_AUDIT = 'pidigits-work-audit/20261011/audit.json'
PREPARED_MANIFESTS = (
    'lua-pidigits-comparison/20261012/full-study/manifest.json',
    'luajit-pidigits-comparison/20261012/full-study/manifest.json',
)
OPTIONAL_FULL_STUDIES = (
    'lua-pidigits-comparison/20261012/results/full-study.json',
    'luajit-pidigits-comparison/20261012/results/full-study.json',
)
BENCHMARKS = ('nbody', 'mandelbrot', 'knucleotide', 'spectralnorm', 'revcomp',
              'fannkuchredux', 'fasta', 'binarytrees', 'regexredux', 'pidigits')
LEGACY_FINAL_RECORDS = frozenset(relative for relative, _ in FINAL_REPORTS)


def _original_loc(row):
    if row.get('loc') is not None:
        return row['loc']
    source = row.get('source')
    if isinstance(source, dict):
        return source.get('nonblank_noncomment_lines')
    return None


def _samples(row):
    return list(row.get('samples_seconds', row.get('samples', [])))


def _has_measurements(row):
    samples = _samples(row)
    return (len(samples) >= 3 and all(isinstance(s, (int, float)) and
            math.isfinite(s) and s > 0 for s in samples) and
            row.get('median_seconds', row.get('seconds')) is not None)


def _qualification(row, report, relative):
    """Retain the report's actual admission evidence, including older schemas."""
    samples = _samples(row)
    raw_protocol = report.get('protocol', {})
    protocol = raw_protocol if isinstance(raw_protocol, dict) else {}
    status = report.get('status')
    validations = row.get('sample_validation', row.get('full_output_validation', []))
    explicit = (isinstance(validations, list) and len(validations) == len(samples)
                and bool(samples) and all(isinstance(v, dict) and v.get('reference_match') is True
                                          for v in validations))
    basis = 'per-sample reference comparisons' if explicit else None
    # N-body confirmations persist every full stdout and their energy reference.
    stdout = row.get('sample_stdout', [])
    expected = report.get('expected_energy', [])
    if not explicit and expected and len(stdout) == len(samples) and samples:
        try:
            explicit = all(len(values := [float(x) for x in output.splitlines()]) == len(expected)
                           and all(abs(a-b) <= 1e-8 for a,b in zip(values, expected))
                           for output in stdout)
        except (ValueError, TypeError):
            explicit = False
        if explicit:
            basis = 'saved full-size stdout matches saved energy reference within 1e-8'
    # These fixed older final reports only export accepted records; they do not
    # have row-level output digests or a status field. Preserve that weaker,
    # report-declared evidence explicitly instead of inventing digest records.
    declared = (relative in LEGACY_FINAL_RECORDS and status is None and
                'every' in protocol.get('validation', '').lower() and
                'sample' in protocol.get('validation', '').lower() and
                len(samples) == protocol.get('samples'))
    if not explicit and declared:
        basis = 'legacy accepted-record report declares full reference equality for every sample'
    complete = status in ('complete', 'passed') or declared
    validated = explicit or declared
    details = row.get('sample_details', [])
    full_details = (len(details) == len(samples) and bool(samples) and
                    all(d.get('full_workload') is True for d in details))
    full_work = (report.get('steps') == 50_000_000 if 'steps' in report else
                 bool(protocol.get('workload') or protocol.get('workloads') or
                      report.get('workload') or full_details or
                      row.get('full_validation_current_batch')))
    expected_count = report.get('sample_count', report.get('samples', protocol.get('samples')))
    count_matches = expected_count == len(samples)
    return dict(admitted=bool(_has_measurements(row) and complete and validated and full_work and count_matches),
                report_status=status, report_completed=complete, full_workload=full_work,
                sample_count=len(samples), full_output_validated=validated,
                declared_sample_count=expected_count, sample_count_matches=count_matches,
                sample_validation=validations, validation_basis=basis,
                saved_sample_stdout=stdout, expected_energy=expected,
                full_validation=row.get('full_validation', row.get('correctness_full')),
                report_validation=protocol.get('validation'),
                workload=protocol.get('workload', protocol.get('workloads',
                    report.get('workload', report.get('steps', raw_protocol if isinstance(raw_protocol, str) else None)))))


def _resolve(root, benchmark, relative):
    """Use the packaging pipeline's bounded exact-path order, without a scan."""
    path = Path(relative)
    choices = ([path] if path.is_absolute() else
               [root / benchmark / path, root / 'compact' / path, root / path])
    for candidate in choices:
        if candidate.is_file():
            return str(candidate)
    # Retain the exact expected path for a consumer to diagnose, rather than
    # silently substituting a different source by name or code similarity.
    return str(choices[0])


def _source_files(root, benchmark, row):
    source = row.get('source')
    paths = source.get('paths', []) if isinstance(source, dict) else []
    if not paths:
        path = source if isinstance(source, str) else row.get('path')
        paths = [path] if path else []
    hashes = source.get('sha256') if isinstance(source, dict) else None
    fallback = row.get('source_sha256') or row.get('sha256')
    result = []
    for relative in paths:
        expected = (hashes.get(relative) if isinstance(hashes, dict) else hashes) or fallback
        result.append(dict(path=_resolve(root, benchmark, relative), sha256=expected))
    return result


def _measurement_metadata(row, report, previous=None):
    """Keep the exact invocation and policies when a former nonendpoint wins."""
    previous = previous or {}
    fields = (
        'runtime_policy', 'execution_mode', 'upstream_execution', 'command', 'cwd',
        'runtime_environment', 'environment_overrides', 'environment',
        'compile_command', 'generation_command', 'native_flags', 'backend',
        'required_work_validated', 'eligible_for_chart', 'algorithm',
        'sample_validation', 'full_validation', 'correctness_full',
        'full_validation_current_batch', 'full_output_validation',
        'sample_details', 'runtime', 'binary_sha256', 'executable_sha256',
        'original_physical_loc',
    )
    validations = frozenset({'sample_validation', 'full_validation', 'correctness_full',
                             'full_validation_current_batch', 'full_output_validation', 'sample_details'})
    same_report = not previous or previous.get('report') == row.get('report', previous.get('report'))
    metadata = {field:(row[field] if row.get(field) is not None else
                      previous.get(field) if field not in validations or same_report else None)
                for field in fields}
    metadata['source_url'] = row.get('source_url') or row.get('url') or previous.get('source_url')
    protocol = report.get('protocol', previous.get('measurement_protocol'))
    metadata['measurement_protocol'] = protocol
    metadata['timing_boundary'] = row.get('timing_boundary') or previous.get('timing_boundary')
    if metadata['timing_boundary'] is None:
        metadata['timing_boundary'] = protocol.get('timed_region') if isinstance(protocol, dict) else protocol
    metadata['runtime_policy_evidence'] = dict(
        invocation=metadata['command'], execution_mode=metadata['execution_mode'],
        runtime_policy=metadata['runtime_policy'],
        runtime_environment=metadata['runtime_environment'],
        environment_overrides=metadata['environment_overrides'], environment=metadata['environment'],
        policy_protocol=protocol)
    return metadata


def collect_pool(root: Path, current_data: dict):
    """Return all admitted measured legacy candidates plus unmeasured prepares.

    Same-id measurement overlays replace earlier samples. Source metadata is
    inherited for confirmation rows that intentionally omit it. Known work
    exclusions are authoritative; normalized current application rows carry
    their existing audited counts for reconciliation instead of recounting.
    """
    root = Path(root)
    reports, pending, reconciliation, pool, loaded_reports = [], [], [], {}, {}
    uncovered_reports = []
    excluded = {
        row['key']: row.get('reason', 'Excluded by current required-work audit.')
        for row in current_data.get('exclusions', []) if row.get('key')
    }

    def load(relative, optional=False):
        path = root / relative
        if optional and not path.is_file():
            return None
        reports.append(relative)
        loaded_reports[relative] = json.loads(path.read_text())
        return loaded_reports[relative]

    def put(row, relative, default=None, source_report=None):
        language = row.get('language')
        benchmark = row.get('benchmark', default)
        if language not in LANGUAGES or not benchmark:
            return
        key = row.get('key') or benchmark + ':' + row['id']
        qualification = _qualification(row, loaded_reports[relative], relative)
        if row.get('excluded_full') or not qualification['admitted']:
            pending.append(dict(key=key, benchmark=benchmark, language=language,
                id=row['id'], original_loc=_original_loc(row), report=relative,
                source_files=_source_files(root, benchmark, row), samples=_samples(row),
                seconds=None, qualification=qualification,
                status='pending-qualification-and-measurement',
                reason='No completed admitted set of at least three validated full-size measurements.'))
            return
        previous = pool.get(key)
        sources = _source_files(root, benchmark, row)
        if not sources and previous:
            sources = previous['source_files']
            source_report = previous.get('source_report', previous['report'])
        algorithm = row.get('algorithm') or (previous or {}).get('algorithm')
        original_loc = _original_loc(row)
        if original_loc is None and previous:
            original_loc = previous['original_loc']
        candidate = dict(key=key, id=row['id'], benchmark=benchmark, language=language,
            original_loc=original_loc, samples=_samples(row),
            seconds=row.get('median_seconds', row.get('seconds')), report=relative,
            source_report=source_report or relative, source_files=sources,
            algorithm=algorithm, current_keys=[], aliases=[], already_audited=False,
            qualification=qualification)
        candidate.update(_measurement_metadata(dict(row, report=relative), loaded_reports[relative], previous))
        candidate['required_work_evidence'] = dict(algorithm=candidate.get('algorithm'),
            required_work_validated=candidate.get('required_work_validated'),
            eligible_for_chart=candidate.get('eligible_for_chart'))
        candidate['full_output_evidence'] = dict(qualification=qualification,
            sample_validation=candidate.get('sample_validation'),
            full_output_validation=candidate.get('full_output_validation'),
            full_validation=candidate.get('full_validation'),
            correctness_full=candidate.get('correctness_full'),
            full_validation_current_batch=candidate.get('full_validation_current_batch'))
        for field in ('loc_metric', 'normalized_loc', 'physical_loc', 'loc_expansions',
                      'loc_policy', 'sample_validation', 'full_validation', 'eligible_for_chart'):
            if field in row:
                candidate[field] = row[field]
        pool[key] = candidate

    report = load(BASE_REPORT)
    for row in report['programs']:
        put(row, BASE_REPORT)
    # Preserve failed qualification reasons from the original source sweeps,
    # without reviving superseded timings or expanding the measured universe.
    for relative in report.get('original_reports', {}):
        load(relative, optional=True)
    for relative in SCRIPTING_REPORTS:
        report = load(relative)
        if report.get('status') == 'complete':
            for row in report['programs']:
                put(row, relative)
    for relative, default in FINAL_REPORTS:
        for row in load(relative)['programs']:
            put(row, relative, default)
    for row in load(NBODY_INITIAL)['programs']:
        if row.get('language') in LANGUAGES:
            put(row, NBODY_INITIAL, 'nbody')
    report = load(NBODY_LATEST, optional=True)
    if report and report.get('status') == 'complete':
        summary = load(NBODY_SUMMARY)
        for row in report['programs']:
            if row.get('language') not in LANGUAGES:
                continue
            source = summary['control_sources'].get(row['id'], {})
            put(dict(row, **source), NBODY_LATEST, 'nbody', NBODY_SUMMARY)

    # Only the required-work-approved Rust adaptation is admitted by the
    # current pipeline; exploratory original/integer variants are not added.
    report = load(FASTA_REPORT, optional=True)
    if report and report.get('status') == 'passed':
        row = report['programs']['rust7-floating-16']
        key = 'fasta:rust7-floating-16-local'
        current = current_data.get('programs', {}).get(key, {})
        put(dict(row, key=key, id='rust7-floating-16-local', language='rust',
                 benchmark='fasta', loc=current.get('loc'),
                 algorithm=current.get('algorithm', dict(strict=True))), FASTA_REPORT)

    report = load(NODE_REPORT, optional=True)
    if report and report.get('status') == 'complete':
        audit = load(NODE_AUDIT)
        for correction in audit.get('corrections', []):
            for old_key in correction.get('replaces_keys', []):
                excluded[old_key] = 'Replaced by the measured two-extract required-work correction.'
            row = report['programs'][correction['key']]
            put(dict(correction, **row), NODE_REPORT, source_report=NODE_AUDIT)

    current_programs = current_data.get('programs', {})
    current_files = current_data.get('files', {})
    for key, current in current_programs.items():
        if current.get('language') not in LANGUAGES:
            continue
        candidate = pool.get(key)
        if candidate is None:
            # Admit explicitly measured additive current rows absent from the
            # frozen legacy reports. This also supports later reviewed imports.
            row = dict(current, samples_seconds=current.get('samples', []),
                       median_seconds=current.get('seconds'))
            if not _has_measurements(row):
                continue
            sources = []
            for identity in current.get('files', []):
                file = current_files[identity]
                if file.get('role') == 'runtime-support':
                    continue
                sources.append(dict(path=_resolve(root, current['benchmark'], file['path']),
                                    sha256=file.get('sha256', identity)))
            candidate = dict(key=key, id=current['id'], benchmark=current['benchmark'],
                language=current['language'], original_loc=current['loc'],
                samples=list(current['samples']), seconds=current['seconds'],
                report=current['report'], source_report=current.get('source_report', current['report']),
                source_files=sources, algorithm=current.get('algorithm'), current_keys=[],
                aliases=[], already_audited=False, additive_current_row=True)
            report = loaded_reports.get(current['report'])
            if report is None:
                report = load(current['report'], optional=True)
            candidate['qualification'] = _qualification(row, report or {}, current['report'])
            if not candidate['qualification']['admitted']:
                pending.append(dict(candidate, seconds=None,
                    status='pending-qualification-and-measurement',
                    reason='Current additive row lacks completed report and full-output admission evidence.'))
                continue
            candidate.update(_measurement_metadata(row, report or {}))
            pool[key] = candidate
        candidate['current_keys'].append(key)
        # The currently imported measurement remains authoritative if it is
        # newer than a frozen record. Preserve historical LOC separately.
        candidate.update(samples=list(current.get('samples', candidate['samples'])),
                         seconds=current.get('seconds', candidate['seconds']),
                         report=current.get('report', candidate['report']),
                         source_report=current.get('source_report', candidate['source_report']))
        current_report = loaded_reports.get(candidate['report'], {})
        candidate.update(_measurement_metadata(current, current_report, candidate))
        candidate['required_work_evidence'] = dict(algorithm=candidate.get('algorithm'),
            required_work_validated=candidate.get('required_work_validated'),
            eligible_for_chart=candidate.get('eligible_for_chart'))
        candidate['full_output_evidence'] = dict(qualification=candidate['qualification'],
            sample_validation=candidate.get('sample_validation'),
            full_output_validation=candidate.get('full_output_validation'),
            full_validation=candidate.get('full_validation'),
            correctness_full=candidate.get('correctness_full'),
            full_validation_current_batch=candidate.get('full_validation_current_batch'))
        if current.get('loc_metric') in ('assignment-normalized', 'statement-normalized'):
            candidate.update(already_audited=True, authoritative_loc=current['loc'],
                loc_metric=current['loc_metric'], physical_loc=current.get('physical_loc'),
                loc_expansions=current.get('loc_expansions', []))
            reconciliation.append(dict(key=key, status='existing-normalized-count-retained',
                authoritative_loc=current['loc'], loc_metric=current['loc_metric'],
                source_files=candidate['source_files']))

    # Abbreviated native n-body ids are existing identities, not new sources.
    for identity, upstream_identity in (('chapel-3', 'nbody-chapel-3'),
                                        ('rust-3', 'nbody-rust-3')):
        key = 'nbody:' + identity
        if key in pool:
            alias = 'nbody:' + upstream_identity
            pool[key]['aliases'].append(alias)
            previous = pool.get(alias)
            if previous and sorted(s['sha256'] for s in previous['source_files']) == sorted(
                    s['sha256'] for s in pool[key]['source_files']):
                reconciliation.append(dict(key=alias, status='superseded-alias-retired',
                    canonical_key=key, previous_report=previous['report'],
                    report=pool[key]['report'], source_files=previous['source_files']))
                pool[key]['current_keys'].extend(k for k in previous['current_keys']
                                                  if k not in pool[key]['current_keys'])
                del pool[alias]

    for key in list(pool):
        candidate = pool[key]
        algorithm = candidate.get('algorithm') or {}
        reason = excluded.get(key)
        if algorithm.get('strict') is False:
            reason = reason or algorithm.get('reason', 'Required-work mismatch.')
        if candidate.get('eligible_for_chart') is False:
            reason = reason or 'Explicitly ineligible in the admitted report.'
        if reason:
            reconciliation.append(dict(key=key, status='excluded', reason=reason,
                report=candidate['report'], source_files=candidate['source_files']))
            del pool[key]

    for relative in PREPARED_MANIFESTS:
        manifest = load(relative, optional=True)
        if not manifest:
            continue
        for row in manifest.get('profiles', []):
            if row.get('language') not in LANGUAGES or row.get('key') in pool:
                continue
            pending.append(dict(key=row['key'], id=row['id'], benchmark=row['benchmark'],
                language=row['language'], original_loc=row.get('loc'),
                original_physical_loc=row.get('original_physical_loc'),
                authoritative_loc=row.get('loc'), already_audited=row.get('loc_metric') in
                ('assignment-normalized', 'statement-normalized'), loc_metric=row.get('loc_metric'),
                source_files=_source_files(root, row['benchmark'], row), report=relative,
                samples=[], seconds=None, full_validation=row.get('full_validation'),
                status='pending-qualified-output-measurement',
                reason='Prepared and output-validated; three scored full-size samples are still required.'))

    # A newly finished guarded study must be reconciled against its sealed
    # manifest and every durable native execution record. Completion flags and
    # endpoint-only current_data cannot establish its complete four-source pool.
    # Keep this explicit blocker until the independent full-record collector is
    # wired; the parent audit must require coverage_complete before certification.
    for relative in OPTIONAL_FULL_STUDIES:
        report = load(relative, optional=True)
        if report and report.get('status') == 'complete':
            diagnostic = dict(report=relative, status='uncovered-complete-report', blocking=True,
                manifest=report.get('manifest'), manifest_sha256=report.get('manifest_sha256'),
                reason='Completed guarded pi study requires sealed-manifest and durable full-record validation for every Lua/LuaJIT profile before measured-pool certification; endpoint-only rows are insufficient.')
            uncovered_reports.append(diagnostic)
            reconciliation.append(diagnostic)

    # Listed pinned sources that were not successfully measured remain visible
    # to the audit as missing candidates, never as chart points. Source identity
    # resolves compiler/runtime variant names without treating them as new code.
    measured_sources = {
        (p['benchmark'], p['language'], tuple(sorted(s['sha256'] for s in p['source_files'])))
        for p in pool.values() if p['source_files']
    }
    listed = []
    for benchmark in BENCHMARKS:
        relative = benchmark + '/manifest.json'
        manifest = load(relative, optional=True)
        if manifest:
            listed.extend((relative, benchmark, benchmark, row) for row in manifest.get('programs', []))
    relative = 'compact/manifest.json'
    manifest = load(relative, optional=True)
    if manifest:
        for row in [*manifest.get('programs', []), *manifest.get('all_listed', [])]:
            if row.get('benchmark'):
                listed.append((relative, row['benchmark'], 'compact', row))
    unsupported = []
    for relative, report in loaded_reports.items():
        for row in report.get('excluded', []):
            if isinstance(row, dict) and row.get('id'):
                unsupported.append((row.get('benchmark', report.get('benchmark')), row, relative))
    pending_identities = {
        (p['benchmark'], p['language'], tuple(sorted(s['sha256'] for s in p['source_files'])))
        for p in pending if p.get('source_files') and all(s.get('sha256') for s in p['source_files'])
    }
    for relative, benchmark, owner, row in listed:
        language = row.get('language')
        if language not in LANGUAGES or not row.get('path') or not row.get('sha256'):
            continue
        if not isinstance(row['sha256'], str) or len(row['sha256']) != 64:
            continue
        source = Path(row['path'])
        source = source if source.is_absolute() else root / owner / source
        sources = [dict(path=str(source), sha256=row['sha256'])]
        identity = (benchmark, language, (row['sha256'],))
        if identity in measured_sources or identity in pending_identities:
            continue
        key = benchmark + ':' + row['id']
        reason, status, evidence = None, 'pending-qualification-and-measurement', []
        for correction in reconciliation:
            if correction.get('status') != 'excluded':
                continue
            match = correction['key'] == key or (correction['key'].startswith(key + '-') and
                    any(s.get('sha256') == row['sha256'] for s in correction.get('source_files', [])))
            if match:
                reason, status = correction['reason'], 'work-excluded'
                evidence.append(correction.get('report'))
                break
        if reason is None and key in excluded:
            reason, status = excluded[key], 'work-excluded'
        if reason is None:
            for default, failure, origin in unsupported:
                if (failure['id'] == row['id'] or failure['id'].startswith(row['id'] + '-')) and (default in (None, benchmark)):
                    reason = failure.get('reason', 'Previously failed build/runtime/output qualification.')
                    status = 'unsupported-or-output-failed'
                    evidence.append(origin)
                    break
        pending.append(dict(key=key, id=row['id'], benchmark=benchmark, language=language,
            original_loc=_original_loc(row), source_files=sources, report=relative,
            samples=[], seconds=None, algorithm=None, status=status,
            reason=reason or 'Pinned listed source has no admitted three-sample full-size measurement; work and output qualification remain required.',
            exclusion_evidence=[e for e in evidence if e]))
        pending_identities.add(identity)

    # A failed measured attempt may already have created a pending record
    # before its listed manifest was consulted. Apply the same distinctions.
    for item in pending:
        if item['key'] in excluded:
            item.update(status='work-excluded', reason=excluded[item['key']])
            continue
        for default, failure, origin in unsupported:
            if (failure['id'] == item['id'] or failure['id'].startswith(item['id'] + '-')) and default in (None, item['benchmark']):
                item.update(status='unsupported-or-output-failed',
                    reason=failure.get('reason', 'Previously failed build/runtime/output qualification.'),
                    exclusion_evidence=[origin])
                break

    for item in pending:
        item['pending_classification'] = {
            'work-excluded': 'work-excluded',
            'unsupported-or-output-failed': 'unsupported',
        }.get(item.get('status'), 'needs-qualification-and-measurement')

    return dict(candidates=sorted(pool.values(), key=lambda p:p['key']),
                pending_candidates=pending, report_paths=list(dict.fromkeys(reports)),
                reconciliation=reconciliation, uncovered_reports=uncovered_reports,
                coverage_complete=not uncovered_reports)
