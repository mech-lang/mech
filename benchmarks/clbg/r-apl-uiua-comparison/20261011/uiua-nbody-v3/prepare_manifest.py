#!/usr/bin/env python3
"""Freeze bounded denominator-placement evidence without executing workloads."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
BASE_DIR = SUITE / 'nbody-canonical-v2'
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'
CASES = {0, 1, 2, 7, 1000}
OLD = '(≡≡(¤√ⁿ3)+Diagonal Norm)'
NEW = '(≡≡¤ √ⁿ3+Diagonal Norm)'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]),
                conclusion=conclusion)


def evidence_paths(value):
    """Include diagnostic baseline evidence nested in each state record."""
    if isinstance(value, dict):
        for field, item in value.items():
            if field in ('source', 'stdout_file', 'stderr_file', 'input_file'):
                yield Path(item)
            elif isinstance(item, (dict, list)):
                yield from evidence_paths(item)
    elif isinstance(value, list):
        for item in value:
            yield from evidence_paths(item)


def main():
    baseline_path = BASE_DIR / 'uiua-manifest.json'
    baseline = json.loads(baseline_path.read_text())
    assert baseline['status'] == 'prepared-validated', 'Canonical v2 must pass its locked checks first.'
    runtime = deepcopy(baseline['runtime'])
    old_rows = {row['source_profile']: row for row in baseline['programs']}
    assert set(old_rows) == {'compact', 'fast'}
    baseline_validation = baseline['validation']
    assert baseline_validation['passed'] is True
    assert sha(baseline_validation['path']) == baseline_validation['sha256']
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed'
    assert validation['passed'] == validation['state_checks_passed'] == 10
    assert validation['primitive_checks_passed'] == 7
    assert validation['full_performance_runs'] == 0
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    assert validation['runtime_sha256'] == runtime['sha256'] == sha(runtime['path'])
    assert validation['shared_timing_lock'] == '/tmp/mech-clbg-timing.lock'
    assert validation['canonical_correction_ulps'] == 2
    assert validation['canonical_jupiter_z_hex'] == '-0x1.a86f96c25ebf0p-4'
    assert validation['reference_source_sha256'] == sha(validation['reference_source'])
    for path, digest in validation['source_pins'].items():
        assert sha(path) == digest, path
    combinations = {(variant, n) for variant in old_rows for n in CASES}
    for collection in ('cases', 'state_cases'):
        assert len(validation[collection]) == 10
        assert {(row['variant'], row['n']) for row in validation[collection]} == combinations
        assert all(row['returncode'] == 0 and row['reference_match'] for row in validation[collection])
    for row in validation['state_cases']:
        assert row['bodies'] == 5 and row['state_shape'] == [5, 6] and row['state_components'] == 30
        assert row['verified_steps'] == row['n']
        assert row['final_state_bitwise_equal'] and row['every_step_bitwise_equal']
        assert row['canonical_raw_constants_bitwise_equal']
        assert row['canonical_zero_jupiter_z_ulp_difference'] == 0
        assert row['canonical_initial_state_bitwise_equal'] == (row['n'] == 0)
        assert row['maximum_final_reference_absolute_error'] <= 5e-11
        assert row['maximum_step_reference_absolute_error'] <= 5e-11
    expected_shapes = {(1, 1), (2, 3), (3, 2), (5, 5), (7, 4), (1, 8), (9, 1)}
    assert {tuple(row['input_shape']) for row in validation['primitive_cases']} == expected_shapes
    assert all(row['returncode'] == 0 and row['same_values_and_order'] and row['reference_match']
               and row['output_shape'] == [*row['input_shape'], 1]
               and row['benchmark_steps'] == 0 for row in validation['primitive_cases'])
    root = Path(runtime['path']).parent / 'uiua-0.19.1'
    proofs = [
        proof(root / 'parser/src/parse.rs', 1133, 1193,
              'Modifiers consume their recursive strand operand. Unparenthesized nested Rows here binds only Fix; Norm already binds nested Rows to Reduce Add and leaves Self Mul on the whole array.'),
        proof(root / 'src/algorithm/zip.rs', 47, 81,
              'Fix is a recognized general monadic native array function, applying fix_depth at the requested depth.'),
        proof(root / 'src/algorithm/zip.rs', 150, 188,
              'Nested Rows over a recognized function increments its depth instead of executing a user function for each row.'),
        proof(root / 'src/algorithm/zip.rs', 632, 662,
              'Rows dispatches recognized native functions once over the array; unmatched composite functions use the row-by-row env.exec fallback.'),
        proof(root / 'src/algorithm/monadic/mod.rs', 703, 712,
              'General depth-aware Fix changes the shape; the benchmark matrix has no map-key metadata.'),
        proof(root / 'src/shape.rs', 108, 115,
              'fix_depth inserts a singleton dimension at the requested depth, without copying numeric data.'),
        proof(root / 'src/compile/optimize.rs', 128, 130,
              'The same ordinary power-three operation expands to Dup Dup Mul Mul in both placements.'),
        proof(root / 'src/algorithm/encode.rs', 348, 410,
              'Diagnostic f64 byte encoding serializes explicit little-endian float bits; diagnostic mode is separate from production.'),
    ]
    transformations = []
    for variant, old in old_rows.items():
        original = Path(old['source'])
        source = HERE / 'sources' / (variant + '.ua')
        assert sha(original) == old['source_sha256'] == validation['source_pins'][str(original)]
        assert sha(source) == validation['source_pins'][str(source)]
        before, after = original.read_text(), source.read_text()
        assert before.count(OLD) == 1 and after == before.replace(OLD, NEW)
        assert [i+1 for i, (a, b) in enumerate(zip(before.splitlines(), after.splitlines())) if a != b] == [13]
        assert before.splitlines()[9] == after.splitlines()[9] == 'Norm         ← ≡≡/+˙×'
        lines = after.splitlines()
        physical = sum(bool(line.split('#', 1)[0].strip()) for line in lines)
        assert physical == 21
        assert all(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line.split('#', 1)[0])) <= 1 for line in lines)
        transformations.append(dict(variant=variant, original_source=str(original),
                                    original_source_sha256=sha(original), source=str(source),
                                    source_sha256=sha(source), changed_lines=[13], physical_loc=physical,
                                    old_expression=OLD, new_expression=NEW, norm_unchanged=True))
        proofs.append(proof(original, 10, 15,
                            'Canonical v2 already uses native Norm; its denominator has the composite Rows body. All pair/diagonal/update operations are retained.'))
        proofs.append(proof(source, 1, 21,
                            'Runtime N controls every update. Only denominator placement differs from canonical v2; canonical constants, Norm, pair arrays, dt, energies and all steps remain unchanged.'))
    audit_path = HERE / 'results/source-audit.json'
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
                 transformation='Apply cube and square root to the complete denominator matrix, then native nested Rows Fix inserts the trailing singleton dimension.',
                 transformations=transformations, runtime_sha256=runtime['sha256'],
                 canonical_baseline_manifest=str(baseline_path), canonical_baseline_manifest_sha256=sha(baseline_path),
                 norm_already_native_and_unchanged=True, reduction_order_unchanged=True,
                 native_benchmark_kernel=False, host_helpers=False, input_specific_shortcuts=False,
                 full_n=50000000, bodies=5, state_components=30, dt=0.01,
                 directed_pair_array_shape=[5, 5, 3], full_performance_runs=0,
                 work_policy='Preserve every requested update over the same five bodies, directed pair matrix and diagonal treatment, using the same binary64 operations and reduction order.',
                 proof_type='Pinned parser/runtime source plus bounded complete-state and generic denominator checks; no dispatch counter or performance claim.',
                 native_runtime_proof=proofs,
                 validation_file=str(validation_path), validation_sha256=sha(validation_path),
                 state_checks=validation['state_cases'], primitive_checks=validation['primitive_cases'])
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(runtime['pins'])
    for item in proofs:
        pins[item['path']] = item['sha256']
    paths = [HERE / name for name in ('prepare.py', 'validate.py', 'prepare_manifest.py', 'README.md')]
    paths += [baseline_path, Path(baseline_validation['path']), BASE_DIR / 'prepare.py',
              BASE_DIR / 'validate.py', validation_path, audit_path,
              Path(validation['reference_source']), Path(validation['original_wrong_source']),
              SUITE / 'install_uiua_runtime.py', SUITE / 'uiua-install-pins.json']
    paths += list(evidence_paths({name: validation[name] for name in ('cases', 'state_cases', 'primitive_cases')}))
    for path in paths:
        pins[str(path)] = sha(path)
    programs = []
    for item in transformations:
        variant = item['variant']
        old = old_rows[variant]
        row = deepcopy(old)
        correction = row.pop('canonical_constant_correction', None)
        for field in ('execution_fingerprint', 'execution_fingerprint_inputs', 'timing_alias_of',
                      'timing_alias_reason', 'allocation_fixture', 'small_validation', 'adversarial_validation',
                      'samples_seconds', 'sample_validation', 'attempts', 'validation_evidence'):
            row.pop(field, None)
        source = Path(item['source'])
        profile = 'compact' if variant == 'compact' else 'optimized'
        identifier = 'nbody-uiua-native-denominator-' + profile + '-v3'
        cases = [deepcopy(case) for case in validation['cases'] if case['variant'] == variant]
        small = next(case for case in cases if case['n'] == 1000)
        small.update(validation_evidence_file=str(validation_path), validation_evidence_sha256=sha(validation_path))
        execution = dict(source_sha256=sha(source), runtime_sha256=runtime['sha256'],
                         command=[runtime['path'], 'run', '--no-format', '<source>', '{n}'],
                         cwd=str(SUITE), runtime_environment=row['runtime_environment'], library_pins={}, input_mode='argument')
        assert validation['runtime_environment'] == row['runtime_environment']
        row.update(id=identifier, key='nbody:' + identifier, source_profile=variant,
                   source=str(source), source_sha256=sha(source),
                   command=[runtime['path'], 'run', '--no-format', str(source), '{n}'], pins=deepcopy(pins),
                   provenance='Locally authored additive general-array denominator rewrite of canonical Uiua v2; original and canonical sources and evidence preserved.',
                   adaptations=deepcopy(old['adaptations']) + ['Move cube/square root outside nested Rows; apply only native depth-aware Fix. Norm and every other source byte remain unchanged.'],
                   algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='binary64',
                                  reason='The same five-body binary64 simulation performs all requested updates with unchanged pair matrices, reduction order, diagonal handling and dt=0.01; only generic denominator array placement changes.',
                                  full_n=50000000, bodies=5, dt=0.01,
                                  source_audit=str(audit_path), source_audit_sha256=sha(audit_path)),
                   small_validation=small, adversarial_validation=[case for case in cases if case['n'] != 1000],
                   state_validation=[deepcopy(case) for case in validation['state_cases'] if case['variant'] == variant],
                   validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path),
                                            output_checks=5, state_checks=5, primitive_checks=7),
                   original_source=old['source'], original_source_sha256=old['source_sha256'],
                   canonical_initial_data_baseline=dict(source=old['source'], source_sha256=old['source_sha256'],
                                                        manifest=str(baseline_path), manifest_sha256=sha(baseline_path),
                                                        predecessor_constant_correction=correction),
                   supersedes_keys=[old['key']], supersedes_source_sha256=[old['source_sha256']],
                   loc=21, physical_loc=21, loc_expansions=[],
                   execution_fingerprint_inputs=execution,
                   execution_fingerprint=hashlib.sha256(json.dumps(execution, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
                   eligible_for_chart=False, measurement_status='not-measured', maximum_workers=1,
                   samples_seconds=[], sample_validation=[], attempts=[])
        programs.append(row)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared', language='uiua', runtime=runtime, pins=pins, programs=programs,
                    scope='Two additive Uiua nbody native-denominator candidates, bounded-validated; no full timing admission.',
                    canonical_baseline_manifest=str(baseline_path), canonical_baseline_manifest_sha256=sha(baseline_path),
                    source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path),
                                             passed=10, state_checks_passed=10, primitive_checks_passed=7),
                    installation_command=[PYTHON, str(SUITE / 'install_uiua_runtime.py')],
                    preparation_commands=[[PYTHON, str(HERE / name)] for name in ('prepare.py', 'validate.py', 'prepare_manifest.py')],
                    additive_migration_policy='Freeze separately without modifying original or canonical evidence. Select explicitly through a new audit; three fresh full 50M-step trials and unchanged Mech controls are required before chart admission.')
    destination = HERE / 'manifest.json'
    destination.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print(destination, sha(destination))
    for row in programs:
        print(row['key'], row['loc'], 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
