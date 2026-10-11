"""Import frozen, validated full-workload samples and their fresh Mech controls."""
import json
import math
import statistics
from datetime import datetime
from pathlib import Path

COHORT = 'r-apl-uiua-comparison/20261011'
LANGUAGES = {
    'r': dict(label='R', color='#198ce7', execution='Interpreted',
              runtime_policy='GNU R 4.6.1 · ARM64 · bytecode JIT enabled',
              color_source='https://github.com/github-linguist/linguist/blob/master/lib/linguist/languages.yml'),
    'apl': dict(label='APL (GNU APL)', color='#5a8164', execution='Interpreted',
                runtime_policy='GNU APL 2.0 · native ARM64 interpreter · serial array primitives on macOS',
                color_source='https://github.com/github-linguist/linguist/blob/master/lib/linguist/languages.yml'),
    'uiua': dict(label='Uiua', color='#947bec', execution='Interpreted',
                 runtime_policy='Uiua 0.19.1 · official native ARM64 interpreter',
                 color_source='https://www.uiua.org/styles-c79bdef28b4fbac6.css'),
}
# This reviewed runner predates recorded environment/fingerprint fields and the
# signal/group-completion repair. Only its completed single-process regexredux
# and revcomp samples have been approved for preservation.
LEGACY_RUNNER_SHA256 = 'ff9d82e436472a4ad3b895c4aa4c3bc409d1c4b85bc504c69865df5854494a3a'
LEGACY_SINGLE_PROCESS_BENCHMARKS = {'regexredux', 'revcomp'}
LEGACY_INTERRUPT_CORRECTION_SHA256 = '02d033fa64c7446717661e4c5ba9fcc291bac6766fb9b039c13f101d54bbe98c'
APPROVED_MAINTENANCE_EVENT_SHA256 = '5a4d7ea942b7785913233f5e4bc285d10fade6d73515a981d270650619961303'


def _support_pins(value):
    """Use the common preparation schema's runtime/support fingerprint."""
    pins = {}
    if isinstance(value, dict):
        if value.get('path') and isinstance(value.get('sha256'), str):
            pins[value['path']] = value['sha256']
        for name, child in value.items():
            if name in ('pins', 'libraries', 'pin_files', 'support_pins') and isinstance(child, dict):
                for path, digest in child.items():
                    if isinstance(digest, str) and len(digest) == 64:
                        assert Path(path).is_file(), ('Missing required support pin', path)
                        pins[str(Path(path).absolute())] = digest
                        pins[str(Path(path).resolve())] = digest
            if isinstance(child, (dict, list)):
                pins.update(_support_pins(child))
    elif isinstance(value, list):
        for child in value:
            pins.update(_support_pins(child))
    return pins


def _execution_fingerprint(row):
    return json.dumps(dict(source_sha256=row['source_sha256'],
        binary_sha256=row.get('binary_sha256'),
        command=[argument.replace(row['source'], '{source}') for argument in row['command']],
        cwd=row['cwd'], environment=row.get('runtime_environment', {}),
        support=_support_pins(row)), sort_keys=True)


def _positive_number(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def _loc_metadata(row):
    if 'physical_loc' in row and 'loc_expansions' in row:
        return row['physical_loc'], row['loc_expansions']
    # Original GNU APL manifests retained the exact no-packing rule and LOC,
    # rather than the redundant physical/expansion fields of later manifests.
    assert row['language'] == 'apl'
    assert row['loc_rule'] == ('One independent statement per line; comments/blanks and standalone closing ∇ excluded; '
                               'function headers, I/O and termination count. No diamonds or combined assignments.')
    statements = []
    for line in Path(row['source']).read_text().splitlines():
        code, quoted, i = '', False, 0
        while i < len(line):
            char = line[i]
            if char == "'":
                if quoted and i + 1 < len(line) and line[i + 1] == "'":
                    i += 2
                    continue
                quoted = not quoted
            elif not quoted:
                if char == '⍝':
                    break
                code += char
            i += 1
        assert not quoted, ('Unclosed APL string', row['key'])
        if not code.strip() or code.strip() == '∇':
            continue
        assert '⋄' not in code and code.count('←') <= 1, ('Packed APL statements', row['key'])
        statements.append(code)
    assert len(statements) == row['loc'], ('APL source LOC differs from frozen count', row['key'])
    return len(statements), []


class _LegacyEvidence:
    """Verify immutable migration ancestry without retrofilling attempt records."""
    def __init__(self, root, report, audit, load, sha, provenance):
        self.root, self.report, self.audit = root, report, audit
        self.load, self.sha, self.provenance = load, sha, provenance
        self.snapshots, self.verified = {}, {}
        self.corrections = []

    def snapshot(self, path, expected):
        path = Path(path).resolve()
        name = path.relative_to(self.root.resolve()).as_posix()
        assert self.sha(path) == expected, ('Legacy snapshot changed', name)
        if (name, expected) not in self.snapshots:
            self.snapshots[(name, expected)] = self.load(name)
        return self.snapshots[(name, expected)]

    def equivalent(self, before, after):
        for field in ('key', 'id', 'benchmark', 'language', 'source', 'source_sha256',
                      'binary', 'binary_sha256', 'command', 'cwd', 'runtime_environment',
                      'loc', 'physical_loc', 'loc_expansions', 'timing_alias_of', 'input_mode',
                      'maximum_workers', 'execution_fingerprint'):
            assert before.get(field) == after.get(field), ('Legacy execution changed', after['key'], field)
        for field in ('path', 'sha256', 'version', 'architecture'):
            assert (before.get('runtime') or {}).get(field) == (after.get('runtime') or {}).get(field)
        old = json.loads(_execution_fingerprint(before))
        new = json.loads(_execution_fingerprint(after))
        old_support, new_support = old.pop('support'), new.pop('support')
        assert old == new, ('Legacy execution fingerprint changed', after['key'])
        assert all(new_support.get(path) == digest for path, digest in old_support.items()), (
            'Legacy support pin changed', after['key'])

    def original_runner(self, prior_audit, legacy):
        cohort = (self.root / COHORT).absolute()
        expected = prior_audit['pins'].get(str(cohort / 'run.py')) or prior_audit['pins'].get(str((cohort / 'run.py').resolve()))
        if legacy:
            assert expected == LEGACY_RUNNER_SHA256, 'Legacy runner has not been approved'
        else:
            assert expected != LEGACY_RUNNER_SHA256
            assert expected == self.audit['pins'][str(cohort / 'run.py')], 'Recorded ancestor uses an unverified runner'
        candidates = [Path(revision['prior_report']).resolve().parent / 'run.py'
                      for revision in self.report.get('guard_revisions', [])]
        if not legacy:
            candidates.append(cohort / 'run.py')
        for runner in candidates:
            if not runner.is_file():
                continue
            runner.relative_to(cohort.resolve())
            if self.sha(runner) != expected:
                continue
            prepare = runner.with_name('prepare.py')
            prepare_hash = prior_audit['pins'][str(cohort / 'prepare.py')]
            assert self.sha(prepare) == prepare_hash, 'Archived preparation differs from its audit pin'
            for path in (runner, prepare):
                self.provenance[path.relative_to(self.root.resolve()).as_posix()] = self.sha(path)
            # The original runner inherited ENV and monitoring from these pinned
            # shared runners; their contents must remain unchanged.
            shared = (cohort.parents[1] / 'compact/run.py',
                      cohort.parents[1] / 'array-comparison/all-20261009/run.py')
            for path in shared:
                digest = prior_audit['pins'][str(path)]
                assert self.audit['pins'].get(str(path)) == digest and self.sha(path) == digest
            return dict(path=str(runner), sha256=expected, preparation=str(prepare),
                        preparation_sha256=prepare_hash)
        raise AssertionError('Pinned original legacy runner snapshot is missing')

    def exact_attempt(self, current, original, revision):
        migration_fields = {'attempt_number', 'prior_guard_revision_attempt_number'}
        before = {k: v for k, v in current.items() if k not in migration_fields}
        archived = {k: v for k, v in original.items() if k not in migration_fields}
        if before != archived and current.get('post_measurement_exclusion'):
            classification = revision['classification_correction']
            assert revision['type'] == classification['type'] == 'post_measurement_exclusion'
            assert classification['permitted_changed_attempt_fields'] == [
                'error', 'error_type', 'post_measurement_exclusion']
            assert classification['event_sha256'] == APPROVED_MAINTENANCE_EVENT_SHA256
            event = self.snapshot(classification['event'], APPROVED_MAINTENANCE_EVENT_SHA256)
            assert event['type'] == 'external-maintenance-overlap' and event['never_chart_eligible'] is True
            assert event['preserve_original_automatic_environment'] is True
            assert event['require_full_replacement_samples'] is True
            assert original['attempt_number'] in classification['attempt_numbers']
            assert not original.get('error') and not original.get('post_measurement_exclusion')
            assert original['valid_environment'] is True and original['full_workload'] is True
            assert original['validation']['reference_match'] is True
            signatures = [row['signature'] for row in event['affected_attempts']
                          if row['language'] == self.report['language']
                          and all(original.get(key) == value for key, value in row['signature'].items())]
            assert len(signatures) == 1, 'Excluded trial is not in the approved maintenance event'
            window = event['conservative_window']
            assert datetime.fromisoformat(original['started_at']) < datetime.fromisoformat(window['end'])
            assert datetime.fromisoformat(original['finished_at']) > datetime.fromisoformat(window['start'])
            assert classification['reason'] == event['reason']
            assert current['error_type'] == 'PostMeasurementExclusion'
            assert current['error'] == 'Post-measurement external-maintenance exclusion: ' + event['reason']
            assert current['post_measurement_exclusion'] == dict(
                event=classification['event'], event_sha256=APPROVED_MAINTENANCE_EVENT_SHA256,
                reason=event['reason'], classified_at=classification['at'],
                original_attempt_number=original['attempt_number'],
                original_automatic_valid_environment=True, conservative_maintenance_window=window)
            for field in classification['permitted_changed_attempt_fields']:
                before.pop(field)
            record = dict(type='post_measurement_exclusion', event=classification['event'],
                event_sha256=APPROVED_MAINTENANCE_EVENT_SHA256, reason=event['reason'],
                attempt_number=current['attempt_number'], original_attempt_number=original['attempt_number'],
                original_seconds=original['seconds'], original_automatic_valid_environment=True,
                prior_report=revision['prior_report'], prior_report_sha256=revision['prior_report_sha256'],
                never_chart_eligible=True)
            if record not in self.corrections:
                self.corrections.append(record)
        if before != archived and current.get('infrastructure_interruption') is True:
            assert current.get('error') and original.get('error')
            assert current['error_type'] == 'KeyboardInterrupt'
            assert current['original_error_type'] == original['error_type'] == 'OutputValidationError'
            assert current['original_error'] == original['error']
            correction_path = Path(revision['prior_report']).parent / 'interrupt-classification-correction.json'
            correction = self.snapshot(correction_path, LEGACY_INTERRUPT_CORRECTION_SHA256)
            assert correction['report'] == 'results/timings-' + self.report['language'] + '.json'
            assert correction['attempt'] == original['attempt_number']
            assert correction['original_archive_unchanged'] is True
            assert correction['corrected_classification'] == 'infrastructure KeyboardInterrupt'
            for field in ('infrastructure_interruption', 'original_error_type', 'original_error'):
                before.pop(field)
            before.update(error=original['error'], error_type=original['error_type'])
            record = dict(path=str(correction_path), sha256=LEGACY_INTERRUPT_CORRECTION_SHA256,
                attempt_number=current['attempt_number'], original_attempt_number=original['attempt_number'],
                original_error=original['error'], original_error_type=original['error_type'],
                corrected_error=current['error'], corrected_error_type=current['error_type'], never_chart_eligible=True)
            if record not in self.corrections:
                self.corrections.append(record)
        assert before == archived, ('Legacy attempt differs from archive', current['key'], current['attempt_number'])

    def reconcile(self, attempt, owner):
        key = (owner['key'], attempt['attempt_number'])
        if key in self.verified:
            return self.verified[key]
        current, context, chain, visited = attempt, self.report, [], set()
        while True:
            number = current.get('prior_guard_revision_attempt_number')
            assert type(number) is int and number > 0, ('Legacy attempt lineage missing', key)
            revisions = [revision for revision in context.get('guard_revisions', [])
                         if owner['key'] in revision['preserved_keys']]
            assert revisions, ('Legacy preservation revision missing', key)
            revision = revisions[-1]
            identity = (revision['prior_report'], number)
            assert identity not in visited, 'Legacy migration lineage cycle'
            visited.add(identity)
            prior_report = self.snapshot(revision['prior_report'], revision['prior_report_sha256'])
            prior_audit = self.snapshot(revision['prior_source_audit'], revision['prior_source_audit_sha256'])
            assert prior_report['source_audit_sha256'] == revision['prior_source_audit_sha256']
            assert prior_audit['status'] == 'reviewed'
            assert prior_report['language'] == prior_audit['language'] == self.report['language']
            assert prior_report['workloads'] == prior_audit['workloads'] == self.audit['workloads']
            assert prior_audit['power_required'] == self.audit['power_required']
            assert 1 <= number <= len(prior_report['attempts'])
            original = prior_report['attempts'][number - 1]
            assert original['attempt_number'] == number
            self.exact_attempt(current, original, revision)
            old_row = prior_report['programs'][owner['key']]
            frozen = next(row for row in prior_audit['programs'] + prior_audit['controls']
                          if row['key'] == owner['key'])
            for field, value in frozen.items():
                if field in ('samples_seconds', 'sample_validation', 'attempts'):
                    # An ancestor's preparation can include empty timing slots;
                    # exact archived attempts establish its measurements.
                    assert value == [], ('Frozen audit contains measurements', key, field)
                elif field != 'measurement_status':
                    assert old_row.get(field) == value, ('Old report differs from its frozen audit', key, field)
            self.equivalent(old_row, owner)
            for field in ('source', 'binary'):
                if old_row.get(field):
                    path = old_row[field]
                    assert prior_audit['pins'][path] == old_row[field + '_sha256']
                    assert self.audit['pins'][path] == old_row[field + '_sha256']
            runtime = old_row.get('runtime')
            if runtime:
                path = runtime['path']
                assert (prior_audit['pins'].get(path) or prior_audit['pins'].get(str(Path(path).resolve()))) == runtime['sha256']
                assert (self.audit['pins'].get(path) or self.audit['pins'].get(str(Path(path).resolve()))) == runtime['sha256']
            chain.append(dict(prior_report=revision['prior_report'], prior_report_sha256=revision['prior_report_sha256'],
                prior_source_audit=revision['prior_source_audit'],
                prior_source_audit_sha256=revision['prior_source_audit_sha256'],
                prior_attempt_number=number, migration_reason=revision['reason']))
            if original.get('prior_guard_revision_attempt_number') is None:
                unrecorded = 'runtime_environment' not in original and 'execution_fingerprint' not in original
                if not unrecorded:
                    assert original['runtime_environment'] == old_row.get('runtime_environment', {})
                    assert original['execution_fingerprint'] == _execution_fingerprint(old_row)
                runner = self.original_runner(prior_audit, unrecorded)
                break
            current, context = original, prior_report
        proof = dict(mode='reconciled-from-pinned-old-row-and-runner' if unrecorded else 'recorded-at-execution', timing_source=owner['key'],
            attempt_number=attempt['attempt_number'], runtime_environment=old_row.get('runtime_environment', {}),
            execution_fingerprint=_execution_fingerprint(owner), lineage=chain, original_runner=runner,
            environment_scope='Frozen row overrides and pinned runner policy; complete inherited ambient values were not captured.')
        if unrecorded:
            proof['limitation'] = ('Environment and execution fingerprint were reconciled from the pinned original row and runner; '
                'they were not recorded on the original attempt. The original signal/group-completion guard '
                'was weaker; preserved completed samples use reviewed single-process implementations. '
                'The complete inherited ambient environment was not recorded.')
        self.verified[key] = proof
        return proof


def _sample_evidence(row, report, audit, legacy):
    """Reconcile samples with their indexed attempts, including alias ownership."""
    owner = report['programs'][row.get('timing_alias_of') or row['key']]
    if owner is not row:
        assert not owner.get('timing_alias_of'), ('Unflattened alias', row['key'])
        assert owner['benchmark'] == row['benchmark'] and owner['language'] == row['language']
        assert _execution_fingerprint(owner) == _execution_fingerprint(row), ('Alias execution differs', row['key'])
        assert not row['attempts'], ('Alias has independent attempts', row['key'])
        assert row['samples_seconds'] == owner['samples_seconds']
        assert row['sample_validation'] == owner['sample_validation']
        assert row.get('full_failure') == owner.get('full_failure')
        if row['samples_seconds']:
            assert row.get('aliased_sample_source') == owner['key'], ('Alias provenance missing', row['key'])
    indexes = owner['attempts']
    assert len(indexes) == len(set(indexes)), ('Repeated attempt', owner['key'])
    assert indexes == [index for index, attempt in enumerate(report['attempts'], 1)
                       if attempt['key'] == owner['key']], ('Attempt ownership differs', owner['key'])
    accepted = []
    full_n = audit['workloads'][owner['benchmark']][1]
    expected_command = [argument.replace('{n}', str(full_n)) for argument in owner['command']]
    for index in indexes:
        assert type(index) is int and 1 <= index <= len(report['attempts'])
        attempt = report['attempts'][index - 1]
        for field in ('key', 'id', 'benchmark', 'language', 'source_sha256', 'cwd'):
            assert attempt[field] == owner[field], ('Attempt differs from frozen execution', owner['key'], field)
        assert attempt['attempt_number'] == index
        assert attempt['command'] == expected_command
        assert attempt['full_workload'] is True and attempt['workload_n'] == full_n
        recorded = 'runtime_environment' in attempt and 'execution_fingerprint' in attempt
        if attempt.get('post_measurement_exclusion'):
            assert recorded and type(attempt.get('prior_guard_revision_attempt_number')) is int
        if recorded:
            assert attempt['runtime_environment'] == owner.get('runtime_environment', {})
            assert attempt['execution_fingerprint'] == _execution_fingerprint(owner)
            if attempt.get('prior_guard_revision_attempt_number') is not None:
                legacy.reconcile(attempt, owner)
        else:
            assert 'runtime_environment' not in attempt and 'execution_fingerprint' not in attempt
            legacy.reconcile(attempt, owner)
        environment_valid = (
            attempt['power_at_start'] == audit['power_required']
            and attempt['power_at_end'] == audit['power_required']
            and not any(attempt[field] for field in ('competing_compilers', 'power_changes',
                                                   'monitor_errors', 'resource_monitor_errors'))
            and not attempt.get('monitor_finish_error'))
        assert attempt['valid_environment'] is environment_valid
        if environment_valid and not attempt.get('error'):
            validation = attempt.get('validation', {})
            assert attempt['exit_status'] == 0 and not attempt.get('resource_violation')
            assert validation.get('reference_match') is True
            assert attempt['stdout_bytes'] == validation['bytes']
            assert attempt['stdout_sha256'] == validation['sha256']
            assert _positive_number(attempt['seconds'])
            if not recorded:
                assert owner['benchmark'] in LEGACY_SINGLE_PROCESS_BENCHMARKS
                assert owner.get('maximum_workers') == 1 or owner['language'] == 'mech', (
                    'Legacy completion requires a reviewed single-process implementation', owner['key'])
            accepted.append(attempt)
    samples, validations = row['samples_seconds'], row['sample_validation']
    assert [attempt['seconds'] for attempt in accepted] == samples, ('Sample evidence differs', row['key'])
    assert [attempt['validation'] for attempt in accepted] == validations
    if samples and 'median_seconds' in row:
        assert row['median_seconds'] == statistics.median(samples)
    if len(samples) >= 2 and 'sample_stdev_seconds' in row:
        assert row['sample_stdev_seconds'] == statistics.stdev(samples)
    return len(samples) >= 3 and not row.get('full_failure')


def _execution_metadata(row, report, legacy):
    owner = report['programs'][row.get('timing_alias_of') or row['key']]
    evidence = []
    for index in owner['attempts']:
        attempt = report['attempts'][index - 1]
        if not attempt['valid_environment'] or attempt.get('error'):
            continue
        proof = legacy.verified.get((owner['key'], index))
        if proof is None:
            proof = dict(mode='recorded-at-execution', timing_source=owner['key'], attempt_number=index,
                runtime_environment=attempt['runtime_environment'], execution_fingerprint=attempt['execution_fingerprint'],
                environment_scope='Recorded row overrides and pinned runner policy; complete inherited ambient values were not captured.')
        evidence.append(proof)
    return evidence


def _check_summary(comparison, candidates, control, baseline, admitted):
    """The displayed endpoints must describe exactly the admitted report rows."""
    valid = [row for row in candidates if admitted[row['key']]]
    assert comparison['prepared_candidates'] == len(candidates)
    assert comparison['admitted_candidates'] == len(valid)
    assert comparison['control_admitted'] is admitted[control['key']]
    failed = {row['key']: row['full_failure'] for row in candidates if row.get('full_failure')}
    assert {row['key']: row['failure'] for row in comparison['failed_candidates']} == failed
    pending = {row['key']: (len(row['samples_seconds']), row['measurement_status'])
               for row in candidates if not admitted[row['key']] and not row.get('full_failure')}
    assert {row['key']: (row['valid_samples'], row['status'])
            for row in comparison['pending_candidates']} == pending
    if not valid or not admitted[control['key']]:
        assert comparison['fastest'] is None and comparison['smallest'] is None
        assert comparison['coverage'] == 'pending'
        return False
    fastest = min(valid, key=lambda row: (statistics.median(row['samples_seconds']), row['loc'], row['key']))
    smallest = min(valid, key=lambda row: (row['loc'], statistics.median(row['samples_seconds']), row['key']))
    control_seconds = statistics.median(control['samples_seconds'])
    for role, row in (('fastest', fastest), ('smallest', smallest)):
        endpoint = comparison[role]
        for field in ('key', 'id', 'source', 'source_sha256', 'command', 'loc',
                      'samples_seconds', 'sample_validation'):
            assert endpoint[field] == row[field], ('Summary endpoint differs', row['key'], field)
        seconds = statistics.median(row['samples_seconds'])
        mech_loc = baseline['fastest_loc' if role == 'fastest' else 'smallest_loc']
        expected = dict(seconds=seconds, stdev_seconds=statistics.stdev(row['samples_seconds']),
            time_x_mech=seconds / control_seconds, loc_x_mech=row['loc'] / mech_loc,
            mech_seconds=control_seconds, mech_samples_seconds=control['samples_seconds'],
            mech_loc=mech_loc, control_key=control['key'], timing_alias_of=row.get('timing_alias_of'))
        for field, value in expected.items():
            assert endpoint[field] == value, ('Summary ratio differs', row['key'], field)
    assert comparison['coverage'] == ('complete-prepared-candidates' if len(valid) == len(candidates)
                                      else 'partial-prepared-candidates')
    return True


def add_r_apl_uiua_languages(root, data, load, measured, add_file, sha):
    root = Path(root)
    summary_name = COHORT+'/results/summary.json'
    if not (root/summary_name).is_file():
        return
    summary = load(summary_name)
    assert summary['status'] in ('partial', 'complete')
    report_pins = {(Path(path) if Path(path).is_absolute() else root / path).resolve(): digest
                   for path, digest in summary['reports'].items()}
    for path, digest in report_pins.items():
        assert sha(path) == digest, ('Summary report changed', str(path))
    comparisons = {(row['language'], row['benchmark']): row for row in summary['comparisons']}
    assert len(comparisons) == len(summary['comparisons']), 'Duplicate summary comparison'
    admitted = {}
    failures = []
    checked_comparisons = set()
    checked_reports = set()
    control_failures = []
    report_history = {}
    for language, metadata in LANGUAGES.items():
        audit_name = COHORT+'/source-audit-'+language+'.json'
        report_name = COHORT+'/results/timings-'+language+'.json'
        if not (root/audit_name).is_file() or not (root/report_name).is_file():
            continue
        audit, report = load(audit_name), load(report_name)
        assert report_pins.get((root/report_name).resolve()) == sha(root/report_name)
        checked_reports.add((root/report_name).resolve())
        assert audit['status'] == 'reviewed' and audit['language'] == report['language'] == language
        assert type(report['sample_count']) is int and report['sample_count'] >= 3
        assert Path(report['source_audit']).resolve() == (root/audit_name).resolve()
        assert report['source_audit_sha256'] == sha(root/audit_name)
        assert audit['power_required'] == dict(source='AC Power', low_power_mode=0)
        assert report['workloads'] == audit['workloads']
        assert report['baseline_endpoints'] == audit['baseline_endpoints']
        pins = audit.get('pins', {}) | audit.get('libraries', {})
        for path, digest in pins.items():
            assert sha(path) == digest, path
        frozen = {row['key']: row for row in audit['programs'] + audit['controls']}
        assert len(frozen) == len(audit['programs']) + len(audit['controls'])
        assert set(report['programs']) == set(frozen)
        for key, row in report['programs'].items():
            assert row['key'] == key
            for field, value in frozen[key].items():
                if field in ('samples_seconds', 'sample_validation', 'attempts'):
                    # Some candidate manifests include empty measurement slots.
                    # Actual contents are reconciled against every attempt below.
                    assert value == [], ('Frozen audit contains measurements', key, field)
                elif field != 'measurement_status':
                    assert row.get(field) == value, ('Report differs from frozen audit', key, field)
            assert row['algorithm']['strict'] is True and row['algorithm']['native_benchmark_kernel'] is False
            assert sha(row['source']) == row['source_sha256']
        assert all(attempt['key'] in frozen for attempt in report['attempts'])
        legacy = _LegacyEvidence(root, report, audit, load, sha, data['provenance'])
        eligibility = {key: _sample_evidence(row, report, audit, legacy)
                       for key, row in report['programs'].items()}
        report_history[language] = dict(source_report=report_name,
            guard_revisions=report.get('guard_revisions', []),
            historical_attempts=report.get('historical_attempts', []),
            retired_candidates=report.get('retired_candidates', {}),
            excluded_attempt_corrections=legacy.corrections)
        language_rows = [report['programs'][row['key']] for row in audit['programs']]
        admitted[language] = []
        ready = {}
        for benchmark, baseline in audit['baseline_endpoints'].items():
            candidates = [row for row in language_rows if row['benchmark'] == benchmark]
            control = report['programs'][baseline['control_key']]
            assert control['language'] == 'mech' and control['benchmark'] == benchmark
            assert _positive_number(baseline['fastest_loc']) and _positive_number(baseline['smallest_loc'])
            ready[benchmark] = _check_summary(comparisons[(language, benchmark)], candidates,
                                             control, baseline, eligibility)
            checked_comparisons.add((language, benchmark))
            if control.get('full_failure'):
                control_failures.append(dict(language=language, benchmark=benchmark, key=control['key'],
                                             failure=control['full_failure']))
        coverage = summary['coverage'][language]
        assert coverage['status'] == report['status']
        assert coverage['prepared_benchmarks'] == len(ready)
        assert coverage['admitted_benchmarks'] == sum(ready.values())
        expected_failures = {row['key']: row['full_failure'] for row in language_rows if row.get('full_failure')}
        assert {row['key']: row['failure'] for row in coverage['failures']} == expected_failures
        expected_pending = {row['key']: (len(row['samples_seconds']), row['measurement_status'])
                            for row in language_rows if not eligibility[row['key']] and not row.get('full_failure')}
        assert {row['key']: (row['valid_samples'], row['status']) for row in coverage['pending']} == expected_pending
        for row in language_rows:
            samples, validations = row['samples_seconds'], row['sample_validation']
            if row.get('full_failure'):
                failure = dict(language=language, benchmark=row['benchmark'], key=row['key'],
                               failure=row['full_failure'])
                failures.append(failure)
                data['exclusions'].append(dict(failure,
                    reason='Full workload did not complete with a validated output; no timing is plotted.'))
            if not eligibility[row['key']] or not ready[row['benchmark']]:
                continue
            assert row['small_validation']['reference_match'] is True
            if not any(item['key'] == language for item in data['languages']):
                data['languages'].append(dict(key=language, line_style='solid', **metadata))
            baseline = audit['baseline_endpoints'][row['benchmark']]
            control = report['programs'][baseline['control_key']]
            control_seconds = statistics.median(control['samples_seconds'])
            physical_loc, loc_expansions = _loc_metadata(row)
            measured(row['key'], row['benchmark'], language, row['loc'], samples,
                row['source'], row['source_sha256'], report_name, audit_name,
                physical_loc=physical_loc, loc_expansions=loc_expansions, loc_metric='statement-normalized',
                source_url=row.get('source_url'),
                source_provenance=row.get('source_provenance', 'Locally authored compact and optimized application sources, with upstream references and adaptations recorded in the source audit.'),
                adaptations=row.get('adaptations', []), local_adaptation=True,
                algorithm=row['algorithm'], required_work_validated=True,
                full_output_validation=validations, execution_mode='interpreted',
                sample_execution_metadata=_execution_metadata(row, report, legacy),
                runtime_policy=metadata['runtime_policy'], runtime_arguments=row['command'],
                timing_boundary=audit['timing_boundary'], timing_summary=audit['timing_boundary'],
                timing_alias_of=row.get('timing_alias_of'),
                aliased_sample_source=row.get('aliased_sample_source'),
                execution_fingerprint=_execution_fingerprint(row), runtime=row.get('runtime'),
                runtime_environment=row.get('runtime_environment', {}), support_pins=_support_pins(row),
                mech_comparison=dict(seconds=control_seconds, control_key=control['key'],
                    samples_seconds=control['samples_seconds'], report=report_name, source_audit=audit_name,
                    source=control['source'], source_sha256=control['source_sha256'],
                    sample_execution_metadata=_execution_metadata(control, report, legacy),
                    fastest_loc=baseline['fastest_loc'], smallest_loc=baseline['smallest_loc'],
                    time_x_mech=statistics.median(samples) / control_seconds,
                    fastest_loc_x_mech=row['loc'] / baseline['fastest_loc'],
                    smallest_loc_x_mech=row['loc'] / baseline['smallest_loc']),
                runtime_support='General standard-library arrays, collection operations, regular expressions and GMP integer primitives where used. Every benchmark algorithm, helper and FFI declaration remains in the counted language source. No native benchmark executor.')
            if row.get('upstream_source'):
                add_file(row['upstream_source'], row['upstream_sha256'])
            benchmark = next(b for b in data['benchmarks'] if b['key'] == row['benchmark'])
            group = next((g for g in benchmark['groups'] if g['language'] == language), None)
            if group is None:
                group = dict(language=language, label=metadata['label'], programs=[])
                benchmark['groups'].append(group)
            if row['key'] not in group['programs']:
                group['programs'].append(row['key'])
            admitted[language].append(row['key'])
    assert checked_reports == set(report_pins), 'Summary contains an unchecked timing report'
    assert checked_comparisons == set(comparisons), 'Summary contains an unchecked comparison'
    assert set(admitted) == set(summary['coverage']), 'Summary contains unchecked coverage'
    expected_status = 'complete' if summary['coverage'] and all(
        row['admitted_benchmarks'] == row['prepared_benchmarks'] and not row['pending']
        for row in summary['coverage'].values()) else 'partial'
    assert summary['status'] == expected_status
    data['r_apl_uiua_audit'] = dict(status=summary['status'], admitted=admitted,
        comparisons=summary['comparisons'], summary_report=summary_name,
        failures=failures, control_failures=control_failures, coverage=summary['coverage'], reports=summary['reports'],
        report_history=report_history,
        admission_rule=summary['admission_rule'], units=summary['units'],
        ratio_policy=summary.get('ratio_policy', summary['units']), runtime_specific=True)
    data['protocol']['r_apl_uiua'] = (
        'R, APL (GNU APL) and Uiua use the full canonical workloads with fresh-process elapsed time, '
        'including startup, library loading, complete input, computation, output and teardown. '
        'At least three output-validated samples are required for each plotted source. '
        'Pending or failed sources never contribute invented timings. Runtime-specific partial coverage is labelled. '
        'R bytecode compilation is enabled and included in process time. Application algorithms remain in the named '
        'language; only general libraries and primitive GMP bindings are excluded from application LOC. '
        'Exact shared samples for identical compact/optimized sources are explicitly recorded as aliases. '
        'Approved pre-repair single-process samples require exact pinned archive lineage and unchanged executions; '
        'their environment/fingerprint metadata is labelled as reconciled from the original row and runner, '
        'with the original weaker signal/group-completion guard disclosed. '
        'Fresh unchanged Mech AOT controls support seconds and × Mech comparisons; existing cohorts are preserved.')
    data['provenance']['r_apl_uiua_data.py'] = sha(Path(__file__))
