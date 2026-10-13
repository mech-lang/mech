#!/usr/bin/env python3
"""Seal only this additive qualification, called while the validator owns the mutex."""
from copy import deepcopy
from datetime import datetime, timezone
import json
from pathlib import Path
import re

from prepare import BASE, BASE_SHA, ENV, HERE, SUITE, sha


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]), conclusion=conclusion)


def main():
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed'
    assert validation['outputs_passed'] == 128 and validation['baseline_checks_passed'] == 8
    assert validation['traces_passed'] == 10 and validation['worker_gates_passed'] == 22
    assert validation['range_checks_passed'] == 36 and validation['matrix_checks_passed'] == 18
    assert validation['full_performance_runs'] == 0 and not validation['eligible_for_chart']
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    baseline_path = SUITE / 'uiua-fannkuch-v2/manifest.json'
    baseline = json.loads(baseline_path.read_text())
    original = next(row for row in baseline['programs'] if row['source'] == str(BASE))
    assert original['source_sha256'] == BASE_SHA == sha(BASE)
    runtime = deepcopy(baseline['runtime'])
    assert runtime['sha256'] == validation['runtime_sha256'] == sha(runtime['path'])
    runtime_source = Path(runtime['path']).parent / 'uiua-0.19.1'
    original_audit = json.loads((SUITE / 'uiua-fannkuch-v2/results/source-audit.json').read_text())
    proofs = deepcopy(original_audit['native_runtime_proof']) + [
        proof(runtime_source / 'src/run.rs', 1411, 1524,
              'Ordinary spawn captures the worker arguments and executes the Uiua function on one native OS thread. Explicit spawn is used; the host-sized pool branch is not used.'),
        proof(runtime_source / 'src/run.rs', 1527, 1593,
              'Waiting for the handle array returns every worker result in handle order before parent output.'),
    ]
    source_proofs = []
    for name, loc in [('parallel', 27), ('block', 35)]:
        source_proofs.append(proof(HERE / 'sources' / (name + '.ua'), 1, loc,
            'All application/helper bindings are counted. Disjoint global-rank ranges are capped at eight explicit workers; every rank is materialized and every actual prefix reversal is completed with global parity. Parent output follows wait.' +
            (' Descending factoradic widths rotate the complete bounded matrix using native broadcast indices and flat row-offset gather.' if name == 'block' else ' The v2 per-rank permutation generator and native flip recurrence are retained.')))
    audit_path = HERE / 'results/source-audit.json'
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
                 retained_source=str(BASE), retained_source_sha256=BASE_SHA,
                 retained_manifest=str(baseline_path), retained_manifest_sha256=sha(baseline_path),
                 runtime_sha256=runtime['sha256'], maximum_workers=8, maximum_block_permutations=1024,
                 full_n=12, full_permutations=479001600, full_performance_runs=0,
                 native_benchmark_kernel=False, new_host_helpers=False, permutation_cache=False,
                 flip_result_cache=False, input_specific_shortcuts=False, parent_only_output=True,
                 worker_ranges='[floor(j*Total/T), floor((j+1)*Total/T)), T=min(8,Total). Multiply integer rank boundaries before division.',
                 work_policy='Every global rank occurs exactly once, its actual prefix reversals finish, checksum uses global rank parity, and maximum is reduced from completed worker results.',
                 block_generator='For widths N,N-1,...,2, col<w selects (col+rotation[row]) mod w, otherwise col. Native flat select adds row*N and restores the matrix shape.',
                 proof_type='Pinned ordinary runtime source, complete bounded outputs, ordered worker-returned every-rank/every-flip traces, matrix/range fixtures and actual native worker gates.',
                 native_runtime_proof=proofs, application_source_proof=source_proofs,
                 reused_evidence=deepcopy(validation['reused_evidence']),
                 validation_file=str(validation_path), validation_sha256=sha(validation_path),
                 qualification_counts={key: validation[key] for key in
                     ['outputs_passed', 'baseline_checks_passed', 'traces_passed', 'worker_gates_passed', 'range_checks_passed', 'matrix_checks_passed']},
                 planning_policy='N9 pilots are planning only, never chart eligible; no extrapolated or pilot time is a performance sample.')
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(validation['retained_pins'])
    pins[str(baseline_path)] = sha(baseline_path)
    for item in proofs + source_proofs:
        assert sha(item['path']) == item['sha256'], item['path']
        pins[item['path']] = item['sha256']
    paths = [HERE / name for name in ['prepare.py', 'validate.py', 'prepare_manifest.py', 'task_threads.py', 'README.md']]
    paths += [validation_path, Path(validation['attempt_directory']) / 'validation.json', audit_path]
    for origin in validation['reused_evidence']:
        paths += [Path(origin[key]) for key in ['original_validation_file', 'original_validator_snapshot', 'original_log_file']]
    collections = ['cases', 'baseline_cases', 'traces', 'worker_gates', 'ranges', 'matrices', 'planning_pilots']
    records = [validation['parent_thread_baseline']] + [row for name in collections for row in validation[name]]
    for row in records:
        paths += [Path(row[key]) for key in ['source', 'stdout_file', 'stderr_file']]
    for path in paths:
        pins[str(path)] = sha(path)
    programs = []
    for name, expected_loc in [('parallel', 27), ('block', 35)]:
        source = HERE / 'sources' / (name + '.ua')
        lines = source.read_text().splitlines()
        loc = sum(bool(line.split('#', 1)[0].strip()) for line in lines)
        assert loc == expected_loc
        assert not any(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line)) > 1 for line in lines)
        assert validation['source_pins'][str(source)] == sha(source)
        identifier = 'fannkuchredux-uiua-' + name + '-parallel-v4'
        row = deepcopy(original)
        for key in ['execution_fingerprint', 'execution_fingerprint_inputs', 'excluded', 'exclusion_reason', 'shortlist_status',
                    'timing_alias_of', 'timing_alias_reason', 'samples_seconds', 'sample_validation', 'attempts']:
            row.pop(key, None)
        row.update(id=identifier, key='fannkuchredux:' + identifier,
                   variant='optimized', profile='optimized', source_profile=name,
                   source=str(source), source_sha256=sha(source),
                   command=[runtime['path'], 'run', '--no-format', str(source), '{n}'],
                   runtime_environment=ENV, pins=deepcopy(pins), loc=loc, physical_loc=loc, loc_expansions=[],
                   loc_review='All independent bindings and declarations, including every worker and block-generator helper, have separate lines. No hidden application helper.',
                   provenance='Locally authored isolated additive Uiua application, derived from the retained primitive-index/flat-gather v2 source.',
                   adaptations=['Explicit at-most-eight global-rank workers; unchanged actual prefix flips and global parity.',
                                'Whole-block descending factoradic native index/gather stages.' if name == 'block' else 'Retained v2 per-rank factoradic generation.'],
                   algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='integer', full_n=12,
                                  full_permutations=479001600, maximum_block_permutations=1024,
                                  source_audit=str(audit_path), source_audit_sha256=sha(audit_path)),
                   small_validation=deepcopy(next(record for record in validation['cases']
                                                  if record['family'] == name and record['n'] == 7 and record['requested_workers'] == 8)),
                   adversarial_validation=[deepcopy(record) for record in validation['cases']
                                           if record['family'] == name and (record['n'] != 7 or record['requested_workers'] != 8)],
                   validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path)),
                   maximum_workers=8, eligible_for_chart=False, measurement_status='not-measured',
                   recommended_timeout_seconds=14400)
        programs.append(row)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared-validated', language='uiua', created_at=datetime.now(timezone.utc).isoformat(),
                    scope='Two isolated additive ≤8-worker fannkuch candidates; bounded qualification only, no frozen-suite admission.',
                    runtime=runtime, pins=pins, programs=programs,
                    preparation_command=['python3', str(HERE / 'validate.py')],
                    source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), counts=audit['qualification_counts']),
                    planning_pilots=validation['planning_pilots'], full_performance_runs=0,
                    migration_policy='Root reviews this sealed manifest before any full samples, selection, frozen audit migration or chart write. Retain v2 and current main-coordinator inputs byte-for-byte.')
    path = HERE / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print('SEALED', path, sha(path), flush=True)
    for row in programs:
        print(row['id'], row['loc'], 'LOC', row['source_sha256'], flush=True)


if __name__ == '__main__':
    raise SystemExit('Run validate.py; it owns the common mutex through preparation, validation and sealing.')
