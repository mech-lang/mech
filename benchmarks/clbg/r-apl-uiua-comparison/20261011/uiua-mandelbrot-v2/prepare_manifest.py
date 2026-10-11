#!/usr/bin/env python3
"""Prepare the separately validated native Pack candidate without executing it."""
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
    old = next(row for row in base['programs']
               if row['benchmark'] == 'mandelbrot' and row['variant'] == 'optimized')
    runtime = deepcopy(base['runtime'])
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed'
    assert validation['passed'] == 15 and validation['work_checks_passed'] == 5
    assert validation['primitive_checks_passed'] == 14 and validation['full_performance_runs'] == 0
    assert validation['runtime_sha256'] == runtime['sha256'] == sha(runtime['path'])
    assert validation['runtime_environment'] == old['runtime_environment']
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    assert sha(old['source']) == old['source_sha256'] == '3f3f1a02e405e93ac2e3e90298d45e1a34d6ab21714b96ab2bb3b779c62dcd57'
    source = HERE / 'sources/fast.ua'
    assert validation['source_sha256'] == sha(source)
    lines = source.read_text().splitlines()
    physical = sum(bool(line.split('#', 1)[0].strip()) for line in lines)
    expansions = [dict(line=i, extra_lines=len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line))-1)
                  for i, line in enumerate(lines, 1)
                  if len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line)) > 1]
    assert physical == 11 and not expansions
    root = Path(runtime['path']).parent / 'uiua-0.19.1'
    proofs = [
        proof(Path(old['source']), 7, 8,
              'Original Pack applies a composite multiplication/reduction user function per byte; Row performs the unchanged ordinary recurrence.'),
        proof(root / 'src/compile/optimize.rs', 213, 233,
              'The general compiler replaces plain Rows Reduce with ReduceDepth(1). The original composite Pack body does not match this pattern.'),
        proof(root / 'src/algorithm/pervade.rs', 64, 104,
              'General pervasive arithmetic supports singleton broadcasting; multiplication applies the eight weights to all bit groups.'),
        proof(root / 'src/algorithm/reduce.rs', 18, 102,
              'A reduction of a single ordinary primitive dispatches to numeric/byte reduction rather than a user-function fold.'),
        proof(root / 'src/algorithm/reduce.rs', 532, 642,
              'Depth-aware native reduction visits each group and folds every supplied value in native array code.'),
        proof(source, 1, 11,
              'Runtime N controls all pixels. The exact original recurrence and row loop retain the ordinary 50-iteration/escape condition and PBM dimensions; only byte packing changes.')]
    audit_path = HERE / 'results/source-audit.json'
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
                 original_source=old['source'], original_source_sha256=old['source_sha256'],
                 source=str(source), source_sha256=sha(source), runtime_sha256=runtime['sha256'],
                 native_benchmark_kernel=False, host_helpers=False, input_specific_shortcuts=False,
                 analytic_interior_shortcut=False, full_n=16000, full_pixels=256000000,
                 maximum_pixel_iterations=50, full_performance_runs=0,
                 transformation='Broadcast the eight weights over the complete bit matrix, followed by plain native Rows Reduce Add.',
                 work_policy='All N*N pixels undergo the unchanged recurrence and ordinary escape tests. Every output bit is packed, including zero-filled partial final bytes.',
                 proof_type='Pinned compiler/runtime source and bounded bitmap/recurrence/packing checks; no native dispatch counter or full-speed claim.',
                 native_runtime_proof=proofs,
                 validation_file=str(validation_path), validation_sha256=sha(validation_path),
                 instrumentation=validation['instrumentation'], primitive_checks=validation['primitive_cases'])
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(runtime['pins'])
    for item in proofs:
        pins[item['path']] = item['sha256']
    paths = [HERE / name for name in ('prepare.py', 'validate.py', 'prepare_manifest.py', 'README.md')]
    paths += [validation_path, audit_path, Path(old['source']),
              SUITE / 'install_uiua_runtime.py', SUITE / 'uiua-install-pins.json']
    for collection in ('cases', 'instrumentation', 'primitive_cases'):
        for record in validation[collection]:
            for field in ('source', 'stdout_file', 'stderr_file', 'input_file'):
                if field in record:
                    paths.append(Path(record[field]))
    for path in paths:
        pins[str(path)] = sha(path)
    row = deepcopy(old)
    for field in ('execution_fingerprint', 'timing_alias_of', 'allocation_fixture', 'eligible_for_chart', 'measurement_status'):
        row.pop(field, None)
    small = deepcopy(next(record for record in validation['cases'] if record['n'] == 200))
    small.update(validation_evidence_file=str(validation_path), validation_evidence_sha256=sha(validation_path))
    identifier = 'mandelbrot-uiua-native-pack-optimized-v2'
    row.update(id=identifier, key='mandelbrot:' + identifier, variant='optimized', profile='optimized',
               source_profile='fast', source=str(source), source_sha256=sha(source),
               command=[runtime['path'], 'run', '--no-format', str(source), '{n}'], pins=deepcopy(pins),
               provenance='Locally authored additive general-array Pack rewrite of the pinned original Uiua application; original sources and measurements preserved.',
               adaptations=['Broadcast weight multiplication over all bit groups before plain Rows Reduce Add.',
                            'All pixel recurrences, ordinary escape tests, dimensions and zero-filled PBM tail padding retained.'],
               algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='binary64',
                              reason='Every runtime pixel undergoes the unchanged ordinary complex recurrence and escape test. Existing standard array multiplication and reduction pack all output bits.',
                              full_n=16000, full_pixels=256000000, maximum_pixel_iterations=50,
                              source_audit=str(audit_path), source_audit_sha256=sha(audit_path)),
               small_validation=small,
               adversarial_validation=[deepcopy(record) for record in validation['cases'] if record['n'] != 200],
               loc=physical, physical_loc=physical, loc_expansions=[],
               loc_review='Independent bindings and datatype declaration each occupy separate lines; no packed-binding discount.',
               eligible_for_chart=False, measurement_status='not-measured', maximum_workers=1,
               recommended_timeout_seconds=14400)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared', language='uiua',
                    scope='One additive Uiua Mandelbrot native-packing candidate, bounded-validated; no full timing admission.',
                    runtime=runtime, pins=pins, programs=[row],
                    installation_command=[PYTHON, str(SUITE / 'install_uiua_runtime.py')],
                    preparation_commands=[[PYTHON, str(HERE / name)] for name in ('prepare.py', 'validate.py', 'prepare_manifest.py')],
                    source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path),
                                             passed=15, work_checks_passed=5, primitive_checks_passed=14),
                    additive_migration_policy='Freeze separately without modifying original evidence. Measure three real full n=16000 fresh-process trials and unchanged Mech controls before chart admission.')
    path = HERE / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print(path, sha(path))
    print(row['key'], physical, 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
