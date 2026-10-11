#!/usr/bin/env python3
"""Prepare additive source/runtime/evidence pins; no runtime execution or refreeze."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
SOURCE = HERE / 'sources/fast.ua'
ORIGINAL_MANIFEST = SUITE / 'uiua-mandelbrot-v2/manifest.json'
ORIGINAL = SUITE / 'uiua-mandelbrot-v2/sources/fast.ua'
COUNTS = dict(passed=20, baseline_checks_passed=4, work_checks_passed=6,
              prune_checks_passed=8, scatter_checks_passed=7, primitive_checks_passed=14)
COLLECTIONS = dict(cases='passed', baseline_cases='baseline_checks_passed', instrumentation='work_checks_passed',
                   prune_cases='prune_checks_passed', scatter_cases='scatter_checks_passed', primitive_cases='primitive_checks_passed')
PATH_FIELDS = {'source', 'original_source', 'validator', 'stdout_file', 'stderr_file', 'input_file', 'fixture_file'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]), conclusion=conclusion)


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


def main():
    original_manifest = json.loads(ORIGINAL_MANIFEST.read_text())
    original_row = next(row for row in original_manifest['programs'] if row['benchmark'] == 'mandelbrot')
    assert sha(ORIGINAL) == original_row['source_sha256'] == '64f2913ec83009b70cc3ac8c4064438732bc1faf4448af198b7052e4cb0aaa4e'
    before, after = ORIGINAL.read_text().splitlines(), SOURCE.read_text().splitlines()
    assert len(before) == 11 and len(after) == 14
    assert before[:2] == after[:2]
    assert before[3].replace('M.Live', 'M.Index') == after[3], 'Recurrence expression changed'
    assert before[6].split('←', 1)[1].strip() == after[9].split('←', 1)[1].strip(), 'Byte packer changed'
    assert before[8:] == after[11:]
    assert all(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line)) <= 1 for line in after)
    assert '⊞' not in SOURCE.read_text(), 'No pairwise membership array'
    runtime = deepcopy(original_manifest['runtime'])
    for path, digest in runtime['pins'].items():
        assert sha(path) == digest, path
    assert sha(runtime['path']) == runtime['sha256']
    root = Path(runtime['path']).parent / 'uiua-0.19.1'
    proofs = [
        proof(SOURCE, 1, 14, 'All 14 application/helper lines are counted. Each exact recurrence updates active state; native Where/Select retain states, coordinates and original pixel identities; one final Under Select restores exact-width bits.'),
        proof(ORIGINAL, 1, 11, 'The qualified 11 LOC application and its existing actual full measurements remain retained and selected. The recurrence arithmetic, coordinates, byte packing, headers and row traversal are preserved by this additional candidate.'),
        proof(root / 'parser/src/defs.rs', 1201, 1226, 'Where turns the ordinary zero/one inside mask into stable surviving row indices.'),
        proof(root / 'src/algorithm/monadic/mod.rs', 1837, 1883, 'Native Where scans every mask entry and directly emits its surviving index vector; storage is proportional to selected entries, without a user-function callback per pixel.'),
        proof(root / 'src/run_prim.rs', 137, 138, 'Select and Where dispatch directly to the general native array implementations.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1138, 1145, 'Ordinary Select converts the complete selector and dispatches by array datatype.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1268, 1358, 'Native selection gathers all supplied rows in order; each of four state/coordinate vectors is selected with the same stable index vector.'),
        proof(root / 'parser/src/defs.rs', 1480, 1494, 'Under Select supports replacing selected rows with a scalar; the documented generic operation writes ones at surviving original pixel indices.'),
        proof(root / 'src/run_prim.rs', 919, 924, 'UndoSelect pops replacement, original selector and target then dispatches to the ordinary native indexed update.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1147, 1217, 'UndoSelect validates the ordinary selector, handles duplicate semantics and dispatches the replacement and target by datatype.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1466, 1486, 'The lower-rank scalar replacement path writes the repeated scalar directly at every supplied target index; no pixel-pair membership matrix is built.'),
        proof(root / 'src/algorithm/pervade.rs', 64, 104, 'Binary64 array arithmetic and scalar broadcasting use the general pervasive implementation; arithmetic is unmodified.'),
    ]
    RESULTS = HERE / 'results'
    RESULTS.mkdir(exist_ok=True)
    report_path = RESULTS / 'validation.json'
    report = json.loads(report_path.read_text()) if report_path.exists() else None
    ready = report is not None and report.get('status') == 'passed'
    if ready:
        for field, expected in COUNTS.items():
            assert report[field] == expected, field
        for collection, field in COLLECTIONS.items():
            assert len(report[collection]) == COUNTS[field]
            assert all(row['returncode'] == 0 and row['reference_match'] for row in report[collection]), collection
        assert report['full_performance_runs'] == 0
        assert report['runtime_sha256'] == runtime['sha256']
        assert report['runtime_environment'] == original_row['runtime_environment']
        assert report['shared_timing_lock'] == '/tmp/mech-clbg-timing.lock'
        assert report['validator_sha256'] == sha(HERE / 'validate.py')
        for path, digest in report['source_pins'].items():
            assert sha(path) == digest, path
        assert report['source_pins'][str(SOURCE)] == sha(SOURCE)
        assert all(row['every_recurrence_state_bitwise_verified'] and row['no_escaped_pixel_recurrences']
                   and row['actual_pixel_recurrences'] == row['expected_pixel_recurrences']
                   for row in report['instrumentation'])
    audit = dict(status='passed-source-audit' if ready else 'pending-locked-runtime-validation',
                 created_at=datetime.now(timezone.utc).isoformat(), source=str(SOURCE), source_sha256=sha(SOURCE),
                 physical_loc=14, original_source=str(ORIGINAL), original_source_sha256=sha(ORIGINAL), original_loc=11,
                 additional_candidate=True, original_candidates_retained_selected=True,
                 recurrence_arithmetic_preserved=True, coordinate_expressions_preserved=True,
                 native_byte_packer_preserved=True, maximum_pixel_iterations=50, full_n=16000,
                 every_requested_pixel=True, only_ordinary_escape_termination=True,
                 analytic_interior_shortcut=False, host_helpers=False, native_benchmark_kernel=False,
                 cache=False, pairwise_membership_matrix=False, storage='One row of linear state/index/coordinate vectors and final bits.',
                 arithmetic='binary64', native_runtime_proof=proofs,
                 expected_validation_counts=COUNTS, validation_status='passed' if ready else 'pending',
                 validation_file=str(report_path), validation_sha256=sha(report_path) if report else None,
                 full_performance_runs=0, measured_speedup=None)
    audit_path = RESULTS / 'source-audit.json'
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False)+'\n')
    pins = dict(runtime['pins'])
    for item in proofs:
        pins[item['path']] = item['sha256']
    paths = [HERE / name for name in ('prepare.py', 'validate.py', 'prepare_manifest.py', 'README.md')]
    paths += [SOURCE, ORIGINAL, ORIGINAL_MANIFEST, audit_path]
    if report:
        paths += [report_path] + list(evidence_paths(report))
    for path in paths:
        pins[str(path)] = sha(path)
    row = deepcopy(original_row)
    for field in ('small_validation', 'adversarial_validation', 'validation_evidence', 'allocation_fixture',
                  'timing_alias_of', 'timing_alias_reason', 'execution_fingerprint', 'execution_fingerprint_inputs',
                  'samples_seconds', 'sample_validation', 'attempts', 'supersedes_keys', 'supersedes_source_sha256'):
        row.pop(field, None)
    identifier = 'mandelbrot-uiua-active-pixels-optimized-v3'
    row.update(id=identifier, key='mandelbrot:'+identifier, source=str(SOURCE), source_sha256=sha(SOURCE),
               command=[runtime['path'], 'run', '--no-format', str(SOURCE), '{n}'],
               loc=14, physical_loc=14, loc_expansions=[], source_profile='fast', variant='optimized', profile='optimized',
               loc_review='All independent bindings, record declarations and application lines counted; selection/scatter helpers included. No packed-assignment discount.',
               provenance='Locally authored additional active-pixel application using unmodified general native Where, Select and Under Select primitives.',
               based_on_keys=[original_row['key']], based_on_source_sha256=[original_row['source_sha256']],
               original_source=str(ORIGINAL), original_source_sha256=sha(ORIGINAL),
               adaptations=deepcopy(original_row.get('adaptations', []))+[
                   'Compute recurrence only for currently active pixels; stable Where/Select preserves each original index and coordinate.',
                   'Reconstruct every exact-width bitmap with general Under Select into zeros; preserve original native byte packer and all PBM padding.',
                   'Retain original qualified source and measured candidates; no superiority assumed before actual fresh full measurements.'],
               algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='binary64', maximum_pixel_iterations=50,
                              analytic_shortcuts=False, full_n=16000,
                              reason='Every requested pixel performs each exact recurrence until first ordinary escape or 50 iterations, then native indexed update reconstructs its canonical bit.'),
               source_audit=str(audit_path), source_audit_sha256=sha(audit_path), pins=deepcopy(pins),
               validation_status='passed' if ready else 'pending', eligible_for_chart=False,
               measurement_status='not-measured', maximum_workers=1, samples_seconds=[], sample_validation=[], attempts=[])
    if ready:
        row['small_validation'] = next(deepcopy(case) for case in report['cases'] if case['n'] == 200)
        row['adversarial_validation'] = [deepcopy(case) for case in report['cases'] if case['n'] != 200]
        row['validation_evidence'] = dict(path=str(report_path), sha256=sha(report_path), **COUNTS)
    manifest = dict(status='prepared' if ready else 'prepared-pending-locked-validation', language='uiua',
                    created_at=datetime.now(timezone.utc).isoformat(), runtime=runtime, pins=pins, programs=[row],
                    scope='One additional active-pixel candidate; original qualified source/results retained; no full timing or refreeze.',
                    source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    expected_validation_counts=COUNTS, validation_status='passed' if ready else 'pending',
                    full_performance_runs=0, admission_policy='Root approval and fresh full candidate/control measurements required; actual results determine endpoints.')
    for path, digest in pins.items():
        assert sha(path) == digest, path
    destination = HERE / 'manifest.json'
    destination.write_text(json.dumps(manifest, indent=2, ensure_ascii=False)+'\n')
    print(destination, manifest['status'], sha(destination))
    print(row['key'], '14 LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
