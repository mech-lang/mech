#!/usr/bin/env python3
"""Freeze additive, fully small-validated Uiua array candidates; no execution."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
        excerpt='\n'.join(path.read_text().splitlines()[first-1:last]), conclusion=conclusion)


def main():
    base = json.loads((SUITE / 'uiua-manifest.json').read_text())
    original = next(row for row in base['programs']
                    if row['benchmark'] == 'fannkuchredux' and row['variant'] == 'optimized')
    old_compact = next(row for row in base['programs']
                      if row['benchmark'] == 'fannkuchredux' and row['variant'] == 'compact')
    runtime = deepcopy(base['runtime'])
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed'
    assert validation['passed'] == 16 and validation['work_checks_passed'] == 8
    assert validation['primitive_checks_passed'] == 9 and validation['full_performance_runs'] == 0
    assert validation['runtime_sha256'] == runtime['sha256'] == sha(runtime['path'])
    assert validation['runtime_environment'] == original['runtime_environment']
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    assert sha(original['source']) == original['source_sha256'] == '3727af922300cbd313ae64b4bbcb29c4ed92a6ad967291f36e19daca84ecc9a3'
    assert sha(old_compact['source']) == old_compact['source_sha256'] == '5e7ecea90ba47a00cd42508f027e00a03bec13bd6e8cf00ff0cd7088b10902c5'
    root = Path(runtime['path']).parent / 'uiua-0.19.1'
    proofs = [
        proof(Path(original['source']), 7, 11,
            'The original optimized source tables FlipIndex, a nonprimitive switch composition, per lane/column cell before row gathering.'),
        proof(root / 'src/algorithm/table.rs', 107, 121,
            'The generic table fallback pushes each pair of values and interprets the tabled function for every cell.'),
        proof(root / 'src/algorithm/table.rs', 202, 247,
            'Single-primitive and flipped-single-primitive tables dispatch to native byte/numeric array routines.'),
        proof(root / 'src/algorithm/table.rs', 378, 459,
            'Ordinary numeric comparison and subtraction tables dispatch to fast_table_list; the default opt feature also enables flipped subtraction.'),
        proof(root / 'src/algorithm/table.rs', 470, 489,
            'The native primitive table constructs its complete array with host primitive loops and no Uiua env.exec per cell. This is an existing general array primitive, not an added benchmark kernel.'),
        proof(root / 'src/algorithm/pervade.rs', 64, 104,
            'Ordinary pervasive arithmetic supports singleton broadcasting and matching leading axes for row offsets.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1268, 1373,
            'A flat ordinary select visits every supplied index and copies the selected values in native array code without executing a user function per row.'),
        proof(root / 'src/algorithm/zip.rs', 369, 400,
            'Rows Select is absent from the primitive dyadic row-fast-path set, so the compact row gather still has ordinary per-row dispatch. The optimized flat select removes this dispatch.'),
        proof(root / 'Cargo.toml', 229, 249,
            'The official source default feature set includes opt, which enables extra primitive-table specializations.'),
        proof(root / 'src/compile/pre_eval.rs', 27, 82,
            'Normal compiler pre-evaluation is bounded by purity and 1000 elements. N and ranks come from runtime argument/state expressions; no full n=12 permutation or flip-result table is supplied.')]
    source_proofs = [proof(HERE / 'sources/compact.ua', 1, 20,
        'The complete 1024-rank loop, factoradic permutation generation, actual prefix flips, rank-parity checksum and maximum remain visible Uiua code.'),
        proof(HERE / 'sources/fast.ua', 1, 21,
        'Flattened indices add each lane offset, select every requested element, and reshape to the original bounded permutation matrix. Finished first-zero lanes remain unchanged.')]
    audit_path = HERE / 'results/source-audit.json'
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
        runtime_sha256=runtime['sha256'], original_source=str(original['source']),
        original_source_sha256=original['source_sha256'], original_compact_source=str(old_compact['source']),
        original_compact_source_sha256=old_compact['source_sha256'],
        native_benchmark_kernel=False, host_helpers=False, permutation_cache=False,
        flip_result_cache=False, input_specific_shortcuts=False, maximum_block_permutations=1024,
        full_n=12, full_permutations=479001600, full_performance_runs=0,
        transformation='Replace the switched per-cell FlipIndex table with native primitive comparison/subtraction tables and pervasive arithmetic; fast additionally uses a flat native select/reshape instead of Rows Select.',
        index_identity='column + (column <= first) * (first - 2*column)',
        work_policy='Materialize each canonical rank exactly once. Flip all active prefixes until their first value is zero. Add each exact flip count using canonical rank parity and retain the maximum.',
        compact_original_review='Original scalar compact uses ordinary prefix take/reverse and contains no composite table hot spot; it remains preserved. New 20-LOC compact is an additional bounded array alternative.',
        proof_type='Pinned runtime source plus exhaustive bounded permutation/reversal traces; no native dispatch counter or full-speed claim.',
        native_runtime_proof=proofs, application_source_proof=source_proofs,
        validation_file=str(validation_path), validation_sha256=sha(validation_path),
        instrumentation=validation['instrumentation'], primitive_checks=validation['primitive_cases'])
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(runtime['pins'])
    for item in proofs + source_proofs:
        pins[item['path']] = item['sha256']
    paths = [HERE / 'prepare.py', HERE / 'validate.py', HERE / 'prepare_manifest.py', HERE / 'README.md',
        validation_path, audit_path, Path(original['source']), Path(old_compact['source']),
        SUITE / 'install_uiua_runtime.py', SUITE / 'uiua-install-pins.json']
    for collection in ('cases', 'instrumentation', 'primitive_cases'):
        for record in validation[collection]:
            for field in ('source', 'stdout_file', 'stderr_file'):
                paths.append(Path(record[field]))
    for path in paths:
        pins[str(path)] = sha(path)
    rows = []
    for variant, expected_loc in [('compact', 20), ('fast', 21)]:
        source = HERE / 'sources' / (variant + '.ua')
        lines = source.read_text().splitlines()
        physical = sum(bool(line.split('#', 1)[0].strip()) for line in lines)
        expansions = [dict(line=i, extra_lines=len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line))-1)
                      for i, line in enumerate(lines, 1)
                      if len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line)) > 1]
        assert physical == expected_loc and not expansions
        assert validation['source_pins'][str(source)] == sha(source)
        row = deepcopy(original)
        for field in ('execution_fingerprint', 'timing_alias_of', 'allocation_fixture', 'eligible_for_chart', 'measurement_status'):
            row.pop(field, None)
        identifier = 'fannkuchredux-uiua-native-' + ('indices-compact-v2' if variant == 'compact' else 'flat-gather-optimized-v2')
        small = deepcopy(next(record for record in validation['cases'] if record['variant'] == variant and record['n'] == 7))
        small.update(validation_evidence_file=str(validation_path), validation_evidence_sha256=sha(validation_path))
        row.update(id=identifier, key='fannkuchredux:' + identifier,
            variant='compact' if variant == 'compact' else 'optimized',
            profile='compact' if variant == 'compact' else 'optimized', source_profile=variant,
            source=str(source), source_sha256=sha(source),
            command=[runtime['path'], 'run', '--no-format', str(source), '{n}'],
            pins=deepcopy(pins), provenance='Locally authored additive Uiua native array rewrite of the pinned original optimized application; originals preserved.',
            adaptations=['Primitive tables and pervasive arithmetic replace per-cell switched user-function dispatch.',
                         'Flat native gather replaces per-row Select dispatch.' if variant == 'fast' else 'Original row gathers retained.',
                         'All canonical ranks, actual prefix reversals and checksum/max remain visible Uiua code.'],
            algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='integer',
                reason='Every factorial rank materializes one canonical permutation and completes its actual prefix reversals. Native general array primitives generate reversal indices and gather values; no permutation/flip cache or workload-specific host kernel.',
                full_n=12, full_permutations=479001600, maximum_block_permutations=1024,
                source_audit=str(audit_path), source_audit_sha256=sha(audit_path)),
            small_validation=small,
            adversarial_validation=[deepcopy(record) for record in validation['cases'] if record['variant'] == variant and record['n'] != 7],
            loc=physical, physical_loc=physical, loc_expansions=[],
            loc_review='Separate independent bindings and declarations; ordinary tacit pipelines are one expression, with no packed-binding discount.',
            eligible_for_chart=False, measurement_status='not-measured', maximum_workers=1,
            recommended_timeout_seconds=14400)
        rows.append(row)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared', language='uiua',
        scope='Two additive Uiua fannkuch candidates, fully bounded-validated; no full timing admission.',
        runtime=runtime, pins=pins, programs=rows,
        installation_command=[PYTHON, str(SUITE / 'install_uiua_runtime.py')],
        preparation_commands=[[PYTHON, str(HERE / name)] for name in ('prepare.py', 'validate.py', 'prepare_manifest.py')],
        source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
        validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path),
            passed=16, work_checks_passed=8, primitive_checks_passed=9),
        additive_migration_policy='Add candidates through a separate audited freeze; retain all original sources/evidence. Measure real canonical n=12 candidates and fresh unchanged Mech controls before chart admission.')
    path = HERE / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print(path, sha(path))
    for row in rows:
        print(row['key'], row['loc'], 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
