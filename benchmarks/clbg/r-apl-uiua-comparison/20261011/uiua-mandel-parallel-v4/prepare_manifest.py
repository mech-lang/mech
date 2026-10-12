#!/usr/bin/env python3
"""Seal additive ordinary-thread candidates and their preserved validation evidence."""
from copy import deepcopy
from datetime import datetime, timezone
import fcntl
import json
from pathlib import Path
import re

from prepare import BASES, HERE, LOCK, SUITE, sha
from validate import CASES, ENV, GATE_CASES, TRACE_CASES, save

SEALED = {
    SUITE / 'uiua-mandelbrot-v2/manifest.json': '7ef4271311b62a7dda9c75403192b6b4f38e57c674321c636795872fabd9a8a0',
    SUITE / 'uiua-mandel-active-v3/manifest.json': 'd62ac00b8eb2f3311fe5155447f1934bef889a1e04201d3e4127ab20b99c2dbc',
}
PATH_FIELDS = {'source', 'original_source', 'validator', 'reference_validator', 'stdout_file', 'stderr_file'}


def evidence_paths(value):
    if isinstance(value, dict):
        for field, item in value.items():
            if field in PATH_FIELDS and isinstance(item, str):
                path = Path(item)
                assert path.is_file(), path
                yield path
            elif isinstance(item, (dict, list)):
                yield from evidence_paths(item)
    elif isinstance(value, list):
        for item in value:
            yield from evidence_paths(item)


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]), conclusion=conclusion)


def main():
    for path, digest in SEALED.items():
        assert sha(path) == digest, path
    bases = {family: json.loads((base.parent.parent / 'manifest.json').read_text())
             for family, (base, _, _) in BASES.items()}
    runtime = deepcopy(bases['active']['runtime'])
    for path, digest in runtime['pins'].items():
        assert sha(path) == digest, path
    report_path = HERE / 'results/validation.json'
    report = json.loads(report_path.read_text())
    assert report['status'] == 'passed'
    counts = dict(outputs_passed=2*len(CASES), baseline_checks_passed=4,
                  traces_passed=2*len(TRACE_CASES), worker_gates_passed=2*len(GATE_CASES))
    for field, count in counts.items():
        assert report[field] == count, field
    for collection in ('cases', 'baseline_cases', 'traces', 'worker_gates'):
        assert all(row['returncode'] == 0 and row['reference_match'] for row in report[collection])
    assert report['full_performance_runs'] == 0
    assert report['runtime_environment'] == ENV
    assert report['shared_timing_lock'] == str(LOCK)
    assert report['runtime_sha256'] == runtime['sha256']
    assert report['validator_sha256'] == sha(HERE / 'validate.py')
    assert all(row['every_required_recurrence_bitwise_verified'] and row['every_row_order_verified']
               and row['worker_returned_trace_only'] and not row['concurrent_worker_stdout']
               for row in report['traces'])
    assert all(row['actual_worker_bound_verified'] and row['expected_spawn_workers'] == min(8, row['n'])
               for row in report['worker_gates'])
    for path, digest in report['source_pins'].items():
        assert sha(path) == digest, path
    native = Path(runtime['path']).parent / 'uiua-0.19.1'
    proofs = [
        proof(native / 'parser/src/defs.rs', 2902, 2963,
              'Ordinary Spawn creates one native OS thread per call. Wait collects returned values; Send/Recv diagnostics use ordinary channels. Pool is not used.'),
        proof(native / 'src/run_prim.rs', 548, 554,
              'Spawn dispatches env.spawn(false), distinct from the unused automatic Pool path.'),
        proof(native / 'src/run.rs', 1411, 1528,
              'The native false-pool branch launches one standard OS thread and gives it ordinary Uiua stack and runtime state. No benchmark kernel executes.'),
        proof(native / 'src/run.rs', 1529, 1618,
              'Wait receives every worker result and preserves the supplied handle order. Channel Send/Recv supports the bounded diagnostic gate.'),
        proof(native / 'parser/src/defs.rs', 2284, 2324,
              'General Group buckets the row-index array. Boxing permits groups of unequal lengths, preserving odd dimensions.'),
        proof(native / 'src/algorithm/groups.rs', 543, 605,
              'Native Group scans ordinary row indices and stores each row in one bucket; no pairwise membership matrix is formed.'),
        proof(native / 'parser/src/defs.rs', 1167, 1183,
              'Rise computes the ordinary row-index permutation used by Select to restore ascending output row order.'),
        proof(native / 'src/algorithm/monadic/sort.rs', 93, 128,
              'General Rise sorts index values with Rayon. RAYON_NUM_THREADS=1 caps its helper pool independently of the eight ordinary Mandelbrot workers.'),
        proof(native / 'src/run_prim.rs', 134, 138,
              'Rise, Select, and Where dispatch to general native array operations.'),
    ]
    pins = dict(runtime['pins'])
    pins.update({str(path): digest for path, digest in SEALED.items()})
    for item in proofs:
        pins[item['path']] = item['sha256']
    retained = dict(manifests={str(path): digest for path, digest in SEALED.items()},
                    sources={str(path): digest for path, digest, _ in BASES.values()},
                    existing_sources_and_manifests_unchanged=True)
    programs = []
    for family, (base, digest, base_loc) in BASES.items():
        assert sha(base) == digest
        source = HERE / 'sources' / (family+'.ua')
        lines = source.read_text().splitlines()
        before = base.read_text().splitlines()
        loc = base_loc+5
        assert len(lines) == loc
        assert lines[:base_loc-3] == [line.replace('&fwa "/dev/stdout" Pack', 'Pack') for line in before[:-3]]
        assert all(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line)) <= 1 for line in lines)
        assert source.read_text().count('spawn') == 1
        assert all(word not in source.read_text() for word in ('pool', 'memo', 'comptime', '⊞'))
        assert sum(line.startswith('&fwa ') for line in lines) == 1
        application_proof = proof(source, 1, loc,
                                  'All bindings, record declarations, helpers and output statements count. Exactly min(8,N) Spawn calls operate on disjoint strided row groups. Workers return packed rows; the parent joins and restores exact order after Wait.')
        proofs_for_row = [application_proof]+proofs
        audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
                     family=family, source=str(source), source_sha256=sha(source), loc=loc,
                     base_source=str(base), base_source_sha256=digest, base_loc=base_loc,
                     all_independent_statements_counted=True, native_runtime_unmodified=True,
                     recurrence_arithmetic_preserved=True, maximum_pixel_iterations=50,
                     maximum_compute_workers=8, spawned_worker_count='min(8,N)', nested_spawns=False,
                     automatic_spawn_pool=False, rayon_helper_pool_cap=1,
                     worker_standard_output=False, parent_binary_payload_after_all_waits=True,
                     parent_header_may_precede_worker_completion=True,
                     row_subdivision='Stable original indices grouped by remainder modulo worker count.',
                     result_ordering='Join boxed worker blocks, Rise original indices, Select packed rows.',
                     every_requested_pixel=True, arithmetic='binary64', analytic_shortcut=False,
                     pairwise_membership_matrix=False, output_or_iteration_cache=False,
                     host_workload_helper=False, native_benchmark_kernel=False,
                     storage='One numeric row per worker and linear complete packed output blocks.',
                     full_n=16000, full_performance_runs=0, measured_speedup=None,
                     validation_status='passed', validation_file=str(report_path), validation_sha256=sha(report_path),
                     native_runtime_proof=proofs_for_row, retained=retained)
        audit_path = HERE / 'results' / ('source-audit-'+family+'.json')
        save(audit_path, audit)
        for path in (source, base, audit_path):
            pins[str(path)] = sha(path)
        row = deepcopy(next(row for row in bases[family]['programs'] if row['benchmark'] == 'mandelbrot'))
        for field in ('small_validation', 'adversarial_validation', 'validation_evidence', 'timing_alias_of',
                      'timing_alias_reason', 'execution_fingerprint', 'execution_fingerprint_inputs',
                      'samples_seconds', 'sample_validation', 'attempts', 'supersedes_keys', 'supersedes_source_sha256'):
            row.pop(field, None)
        identifier = 'mandelbrot-uiua-parallel-'+family+'-v4'
        row.update(id=identifier, key='mandelbrot:'+identifier, source=str(source), source_sha256=sha(source),
                   command=[runtime['path'], 'run', '--no-format', str(source), '{n}'], cwd=str(SUITE),
                   loc=loc, physical_loc=loc, loc_expansions=[], runtime_environment=ENV,
                   loc_review='Every independent binding, record declaration, helper and output statement is on its own counted line.',
                   provenance='Locally authored ordinary Uiua Spawn/Wait composition of the unchanged '+family+' recurrence.',
                   based_on_source_sha256=[digest], original_source=str(base), original_source_sha256=digest,
                   maximum_workers=8, source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                   validation_status='passed', eligible_for_chart=False, measurement_status='not-measured',
                   samples_seconds=[], sample_validation=[], attempts=[],
                   algorithm=dict(strict=True, arithmetic='binary64', maximum_pixel_iterations=50,
                                  native_benchmark_kernel=False, analytic_shortcuts=False, full_n=16000,
                                  maximum_compute_workers=8),
                   small_validation=next(deepcopy(item) for item in report['cases'] if item['family'] == family and item['n'] == 200),
                   adversarial_validation=[deepcopy(item) for item in report['cases'] if item['family'] == family and item['n'] != 200],
                   validation_evidence=dict(path=str(report_path), sha256=sha(report_path), **counts))
        programs.append(row)
    paths = [HERE / name for name in ('prepare.py', 'validate.py', 'prepare_manifest.py', 'task_threads.py', 'README.md')]
    paths += [report_path, Path(report['attempt_directory']) / 'validation.json'] + list(evidence_paths(report))
    for path in paths:
        pins[str(path)] = sha(path)
    for row in programs:
        row['pins'] = deepcopy(pins)
    manifest = dict(status='prepared', language='uiua', created_at=datetime.now(timezone.utc).isoformat(),
                    runtime=runtime, pins=pins, programs=programs, validation_status='passed',
                    expected_validation_counts=counts, retained=retained, full_performance_runs=0,
                    admission_policy='Root review of isolated full study, three fresh full samples and fresh Mech AOT controls required before any chart admission.',
                    scope='Two additional parallel candidates; active suite reports, frozen manifests and original candidates remain untouched.')
    for path, digest in pins.items():
        assert sha(path) == digest, path
    destination = HERE / 'manifest.json'
    save(destination, manifest)
    print(destination, 'prepared', sha(destination), flush=True)
    for row in programs:
        print(row['key'], row['loc'], 'LOC', row['source_sha256'], flush=True)


if __name__ == '__main__':
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        main()
