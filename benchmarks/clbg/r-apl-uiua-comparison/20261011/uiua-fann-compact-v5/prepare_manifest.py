"""Seal composed candidates after fresh complete bounded qualification."""
from copy import deepcopy
from datetime import datetime, timezone
import json
from pathlib import Path
import re
from prepare import BASE, BASE_SHA, ENV, HERE, SUITE, sha


def main():
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed'
    counts = {key: validation[key] for key in
              ('outputs_passed', 'baseline_checks_passed', 'traces_passed', 'worker_gates_passed',
               'range_checks_passed', 'matrix_checks_passed')}
    assert counts == dict(outputs_passed=128, baseline_checks_passed=8, traces_passed=10,
                          worker_gates_passed=22, range_checks_passed=36, matrix_checks_passed=18)
    v4_path = SUITE / 'uiua-fann-parallel-v4/manifest.json'
    v4 = json.loads(v4_path.read_text())
    runtime = deepcopy(v4['runtime'])
    prior_audit = json.loads(Path(v4['source_audit']).read_text())
    pins = dict(validation['retained_pins'])
    for entry in validation['input_snapshots'].values():
        assert sha(entry['path']) == entry['sha256']
        pins[entry['path']] = entry['sha256']
    for filename in ['prepare.py', 'validate.py', 'prepare_manifest.py', 'task_threads.py', 'SPEC.md']:
        path = HERE / filename
        pins[str(path)] = sha(path)
    for path in [validation_path, Path(validation['attempt_directory']) / 'validation.json']:
        pins[str(path)] = sha(path)
    for name in ['cases', 'traces', 'worker_gates', 'ranges', 'matrices', 'planning_pilots']:
        for row in validation[name]:
            for key, digest_key in [('source', 'source_sha256'), ('stdout_file', 'stdout_sha256'), ('stderr_file', 'stderr_sha256')]:
                assert sha(row[key]) == row[digest_key]
                pins[row[key]] = row[digest_key]
    for key, digest_key in [('source', 'source_sha256'), ('stdout_file', 'stdout_sha256'), ('stderr_file', 'stderr_sha256')]:
        row = validation['parent_thread_baseline']
        pins[row[key]] = row[digest_key]
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
                 maximum_workers=8, maximum_block_permutations=1024,
                 full_n=12, full_permutations=479001600, full_performance_runs=0,
                 native_benchmark_kernel=False, new_host_helpers=False, permutation_cache=False,
                 flip_result_cache=False, input_specific_shortcuts=False, parent_only_output=True,
                 source_metric='One independent definition, declaration or statement per line; pure function composition contains no packed or nested assignments.',
                 block_generator='Numeric stages [width, rotation0, ...] fold widths N..2. Native index = col + (col<width)*(((col+rotation[row]) mod width)-col); native flat select adds row*N and restores shape.',
                 worker_ranges='One Result expression composes ordinary spawn/wait over [floor(j*Total/T), floor((j+1)*Total/T)), T=min(8,Total). Multiplication precedes division.',
                 work_policy='Every global rank occurs once; each initial permutation and all actual prefix reversals are verified. Blocks clamp at End, checksums use global parity, only the joined parent prints.',
                 diagnostic_policy='Production outputs execute the exact composed source. Trace/gate diagnostics expand only Result into equivalent named range/worker/join expressions with exact worker/range string assertions; flip recorder returns ordered worker records without concurrent stdout.',
                 native_runtime_proof=prior_audit['native_runtime_proof'],
                 retained_qualification=str(v4_path), retained_qualification_sha256=sha(v4_path),
                 reused_evidence=deepcopy(validation['reused_evidence']),
                 qualification_counts=counts, validation_file=str(validation_path), validation_sha256=sha(validation_path),
                 planning_policy='N9 pilots are planning only, never chart eligible, and no extrapolation or pilot time is a full performance sample.')
    audit_path = HERE / 'results/source-audit.json'
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins[str(audit_path)] = sha(audit_path)
    programs = []
    for family, expected_loc in [('compact', 20), ('fast', 21)]:
        source = HERE / 'sources' / (family + '.ua')
        lines = source.read_text().splitlines()
        loc = sum(bool(line.split('#', 1)[0].strip()) for line in lines)
        assert loc == expected_loc
        assert not any(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line)) > 1 for line in lines)
        assert validation['source_pins'][str(source)] == sha(source)
        pins[str(source)] = sha(source)
        row = {key: deepcopy(v4['programs'][0][key]) for key in
               ('benchmark', 'language', 'source_url') if key in v4['programs'][0]}
        identifier = 'fannkuchredux-uiua-composed-' + family + '-v5'
        row.update(id=identifier, key='fannkuchredux:' + identifier, variant=family, profile=family, source_profile=family,
                   source=str(source), source_sha256=sha(source), command=[runtime['path'], 'run', '--no-format', str(source), '{n}'],
                   runtime_environment=ENV, maximum_workers=8, loc=loc, physical_loc=loc, loc_expansions=[],
                   provenance='Locally authored isolated Uiua source using pure composition of the ordinary array/block and spawn/wait operations qualified by v4.',
                   adaptations=['Whole-block numeric-stage factoradic gather; explicit at-most-eight disjoint global-rank workers.',
                                'Retained v2 row-wise primitive-index flips.' if family == 'compact' else 'Retained v2 flat row-offset gather flips.'],
                   algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='integer', full_n=12,
                                  full_permutations=479001600, maximum_block_permutations=1024,
                                  source_audit=str(audit_path), source_audit_sha256=sha(audit_path)),
                   validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), counts=counts),
                   eligible_for_chart=False, measurement_status='not-measured', recommended_timeout_seconds=14400)
        programs.append(row)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared-validated', created_at=datetime.now(timezone.utc).isoformat(), language='uiua',
                    scope='Isolated composed compact20/fast21 ordinary ≤8-worker candidates; no frozen suite admission.',
                    runtime=runtime, pins=pins, programs=programs, source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), counts=counts),
                    planning_pilots=validation['planning_pilots'], full_performance_runs=0,
                    retained_source=str(BASE), retained_source_sha256=BASE_SHA,
                    migration_policy='Root reviews this exact sealed manifest before actual N12 samples or future safe-boundary selection. No live leased audit/report or previous source is changed.')
    path = HERE / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print('SEALED', path, sha(path), flush=True)
    for row in programs:
        print(row['id'], row['loc'], 'LOC', row['source_sha256'], flush=True)


if __name__ == '__main__':
    raise SystemExit('Run validate.py; it owns the common mutex through qualification and sealing.')
