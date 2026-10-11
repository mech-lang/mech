#!/usr/bin/env python3
"""Prepare pending or bounded-validated output-batching metadata; never execute Uiua."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'
IDENTIFIER = 'fasta-uiua-batched-output-optimized-v2'
COUNTS = dict(passed=27, work_checks_passed=6, primitive_checks_passed=13, cdf_checks_passed=2)
COLLECTIONS = dict(cases='passed', instrumentation='work_checks_passed',
                   primitive_cases='primitive_checks_passed', cdf_cases='cdf_checks_passed')
ORIGINAL_SHA = 'f0dd51cda0e6dbae84afd0e95d7c4ceeb4aaac6d3892824a413926ab54c5cf05'
COMPACT_SHA = 'fba046bc5d20288547051f1fa56a80d52217edd614ed0efbbabe6ef16353a626'
PATH_FIELDS = {'source', 'source_file', 'fixture', 'fixture_file', 'fixture_path',
               'stdout_file', 'stderr_file', 'input_file', 'input_path',
               'reference_source', 'reference_file', 'original_source',
               'reference_binary', 'reference_manifest',
               'diagnostic_source', 'baseline_source', 'candidate_source',
               'validator', 'reference_validator'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]),
                conclusion=conclusion)


def evidence_paths(value):
    """Recursively collect fixture and nested baseline artifact references."""
    if isinstance(value, dict):
        for field, item in value.items():
            if isinstance(item, str) and (field in PATH_FIELDS or field.endswith('_file')):
                path = Path(item)
                if not path.is_absolute():
                    path = SUITE / path
                assert path.is_file(), (field, item)
                yield path
            elif isinstance(item, (dict, list)):
                yield from evidence_paths(item)
    elif isinstance(value, list):
        for item in value:
            yield from evidence_paths(item)


def main():
    base = json.loads((SUITE / 'uiua-manifest.json').read_text())
    originals = {row['source_profile']: row for row in base['programs'] if row['benchmark'] == 'fasta'}
    assert set(originals) == {'compact', 'fast'}
    old = originals['fast']
    original = Path(old['source'])
    compact = Path(originals['compact']['source'])
    source = HERE / 'sources/fast.ua'
    assert sha(original) == old['source_sha256'] == ORIGINAL_SHA
    assert sha(compact) == originals['compact']['source_sha256'] == COMPACT_SHA
    runtime = deepcopy(base['runtime'])
    assert sha(runtime['path']) == runtime['sha256']
    before, after = original.read_text().splitlines(), source.read_text().splitlines()
    assert len(before) == 24 and len(after) == 34
    assert before[:4] == after[:4]
    assert before[5:18] == after[10:23], 'Random generation must remain byte-identical.'
    assert before[18] == after[28] and before[20:] == after[30:]
    physical = sum(bool(line.split('#', 1)[0].strip()) for line in after)
    assert physical == 34
    assert all(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line.split('#', 1)[0])) <= 1 for line in after)
    assert sum(bool(line.split('#', 1)[0].strip()) for line in before) == 24
    assert sum(bool(line.split('#', 1)[0].strip()) for line in compact.read_text().splitlines()) == 22
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text()) if validation_path.exists() else None
    ready = validation is not None and validation.get('status') == 'passed'
    if ready:
        for field, expected in COUNTS.items():
            assert validation[field] == expected, field
        for collection, field in COLLECTIONS.items():
            assert len(validation[collection]) == COUNTS[field], collection
            assert all(record['returncode'] == 0 for record in validation[collection]), collection
        assert validation['baseline_checks_passed'] == len(validation['baseline_cases']) == 5
        assert all(record['returncode'] == 0 and record['reference_match']
                   for record in validation['baseline_cases'])
        assert all(record['independent_D_evidence']['returncode'] == 0
                   for record in validation['cases'])
        for field in ('reference_source', 'reference_binary', 'reference_manifest'):
            assert sha(validation[field]) == validation[field + '_sha256'], field
        assert validation['full_performance_runs'] == 0
        assert validation['runtime_sha256'] == runtime['sha256']
        assert validation['runtime_environment'] == old['runtime_environment']
        assert validation['validator_sha256'] == sha(HERE / 'validate.py')
        assert validation['shared_timing_lock'] == '/tmp/mech-clbg-timing.lock'
        for path, digest in validation['source_pins'].items():
            assert sha(path) == digest, path
        assert validation['source_pins'][str(source)] == sha(source)
        assert all(record['reference_match'] for record in validation['cases'])
    root = Path(runtime['path']).parent / 'uiua-0.19.1'
    proofs = [
        proof(original, 1, 24,
              'The original canonical optimized application prints each grouped sequence line; generic affine RNG, binary64 unscaled CDFs, headers and record lengths are the preserved baseline.'),
        proof(compact, 1, 22,
              'The original compact and optimized applications remain unchanged, separately pinned, measured and selected. This is an additional optimized candidate; actual measurements determine the eventual endpoints.'),
        proof(source, 1, 34,
              'All helpers occupy counted physical lines. Native gather constructs all output characters/LFs, one Write emits each bounded chunk, ALU retains its global offset, and the random-generation block is unchanged.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1138, 1145,
              'Ordinary Select converts the complete index array and dispatches to the selected array datatype, including characters.'),
        proof(root / 'src/algorithm/dyadic/structure.rs', 1268, 1358,
              'General native array selection allocates output storage and gathers every supplied index in order; no user-function call is required per character.'),
        proof(root / 'src/algorithm/pervade.rs', 64, 104,
              'Ordinary pervasive arithmetic applies singleton broadcasting over complete arrays, used to construct character and LF indices.'),
        proof(root / 'parser/src/defs.rs', 4075, 4082,
              'Write is an ordinary stream operation and stdout is stream handle 1.'),
        proof(root / 'src/sys/mod.rs', 1002, 1019,
              'Write consumes unboxed numeric, byte or character arrays; characters encode to UTF-8, backing-data order is preserved, and no newline is added.'),
        proof(root / 'src/sys/native.rs', 354, 361,
              'The pinned native stdout backend writes all provided bytes and flushes for one runtime invocation; this is not a claim about OS syscall count.'),
        proof(root / 'src/algorithm/monadic/mod.rs', 2128, 2142,
              'Ordinary UTF-8 conversion serializes characters to bytes and rejects invalid UTF-8 on inverse conversion; production Write directly encodes the gathered character array.'),
        proof(root / 'src/value.rs', 545, 587,
              'Print formatting is raw only for rank0/1 character arrays; direct Write avoids formatting rank2 arrays with brackets, quoted rows or spaces.'),
        proof(root / 'src/algorithm/reduce.rs', 849, 867,
              'The existing ordinary Add scan uses its general numeric/byte scan paths to construct binary64 cumulative probabilities.'),
        proof(root / 'src/algorithm/reduce.rs', 974, 992,
              'The numeric scan folds each input in sequence, preserving the original binary64 cumulative addition order.'),
    ]
    results = HERE / 'results'
    results.mkdir(exist_ok=True)
    audit_path = results / 'source-audit.json'
    audit = dict(status='passed-source-audit' if ready else 'source-audit-pending-runtime-validation',
                 created_at=datetime.now(timezone.utc).isoformat(),
                 source=str(source), source_sha256=sha(source), physical_loc=physical,
                 original_sources={name: dict(source=row['source'], sha256=row['source_sha256'], loc=row['loc'])
                                   for name, row in originals.items()},
                 runtime_sha256=runtime['sha256'], random_generation_block_byte_identical=True,
                 transformation='Construct complete character/LF gather indices and use one ordinary stdout Write per bounded chunk; ALU chunks carry their global character offset.',
                 maximum_chunk_characters=65520, line_width=60, chunks_line_aligned=True,
                 formatter_recipe='For L input characters and B=L+ceil(L/60), indices j=0..B-1 select j-floor(j/61), except j%61=60 or j=B-1 selects index L from the input joined with one LF.',
                 native_benchmark_kernel=False, host_helpers=False, input_specific_shortcuts=False,
                 rng_period_shortcut=False, exhaustive_cdf_cache_in_production=False,
                 arithmetic='binary64', full_n=25000000, full_random_draws=200000000,
                 full_sequence_characters=250000000, record_length_multipliers=[2, 3, 5],
                 work_policy='Materialize every requested random draw using the unchanged generic affine scan, compare direct state/139968 values against unscaled binary64 CDFs, emit every requested ALU/random character and exact line ending.',
                 proof_type='Pinned source/runtime code plus pending or completed bounded output/work/wrapping/CDF checks; no timing or native dispatch-counter claim.',
                 native_runtime_proof=proofs, full_performance_runs=0,
                 validation_status='passed' if ready else 'pending',
                 validation_file=str(validation_path),
                 validation_sha256=sha(validation_path) if validation else None,
                 expected_validation_counts=COUNTS)
    if ready:
        audit.update(instrumentation=validation['instrumentation'], primitive_checks=validation['primitive_cases'],
                     cdf_checks=validation['cdf_cases'], exhaustive_cdf_checks_probe_only=True,
                     exhaustive_cdf_states_per_distribution=139968)
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(runtime['pins'])
    for item in proofs:
        pins[item['path']] = item['sha256']
    paths = [HERE / name for name in ('prepare.py', 'prepare_manifest.py', 'README.md')]
    paths += [source, original, compact, audit_path,
              SUITE / 'install_uiua_runtime.py', SUITE / 'uiua-install-pins.json']
    if (HERE / 'validate.py').exists():
        paths.append(HERE / 'validate.py')
    if validation:
        paths.append(validation_path)
        paths.extend(evidence_paths(validation))
        pins.update(validation.get('source_pins', {}))
    for path in paths:
        pins[str(path)] = sha(path)
    row = deepcopy(old)
    for field in ('execution_fingerprint', 'execution_fingerprint_inputs', 'timing_alias_of',
                  'timing_alias_reason', 'allocation_fixture', 'small_validation', 'adversarial_validation',
                  'validation_evidence', 'samples_seconds', 'sample_validation', 'attempts'):
        row.pop(field, None)
    execution = dict(source_sha256=sha(source), runtime_sha256=runtime['sha256'],
                     command=[runtime['path'], 'run', '--no-format', '<source>', '{n}'],
                     cwd=str(SUITE), runtime_environment=row['runtime_environment'],
                     library_pins={}, input_mode='argument')
    row.update(id=IDENTIFIER, key='fasta:' + IDENTIFIER, variant='optimized', profile='optimized',
               source_profile='fast', source=str(source), source_sha256=sha(source),
               command=[runtime['path'], 'run', '--no-format', str(source), '{n}'], pins=deepcopy(pins),
               provenance='Locally authored additive generic-array output batching and bounded ALU traversal of the pinned optimized source; original compact/optimized sources and evidence preserved.',
               adaptations=deepcopy(old['adaptations']) + [
                   'Construct all 60-character line endings with native array arithmetic and gather, then issue one ordinary stdout Write per bounded chunk.',
                   'Traverse ALU in the same bounded line-aligned chunks while carrying the global character offset; retain every requested character and all headers.',
                   'Preserve the random-generation block byte for byte; exhaustive CDF validation is probe-only and supplies no production lookup table.'],
               algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='binary64',
                              reason='The unchanged generic affine scan generates every LCG draw and unscaled binary64 CDF comparison; native gather batches exact output and retains global ALU phase.',
                              full_n=25000000, full_random_draws=200000000,
                              record_length_multipliers=[2, 3, 5], maximum_chunk_characters=65520,
                              source_audit=str(audit_path), source_audit_sha256=sha(audit_path)),
               source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
               original_source=old['source'], original_source_sha256=old['source_sha256'],
               based_on_keys=[old['key']], based_on_source_sha256=[old['source_sha256']],
               loc=34, physical_loc=34, loc_expansions=[],
               loc_review='Every independent binding, datatype declaration and function-body line is counted; all formatter and ALU chunk helpers are included. No packed-binding discount.',
               execution_fingerprint_inputs=execution,
               execution_fingerprint=hashlib.sha256(json.dumps(execution, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
               validation_status='passed' if ready else 'pending',
               eligible_for_chart=False, measurement_status='not-measured', maximum_workers=1,
               samples_seconds=[], sample_validation=[], attempts=[])
    if ready:
        candidate_cases = [deepcopy(case) for case in validation['cases'] if Path(case['source']) == source]
        assert candidate_cases, 'The passed report must contain production runs of this candidate.'
        assert all(case['source_sha256'] == sha(source) for case in candidate_cases)
        small = next(case for case in candidate_cases if case['n'] == 1000)
        small.update(validation_evidence_file=str(validation_path), validation_evidence_sha256=sha(validation_path))
        row.update(small_validation=small,
                   adversarial_validation=[case for case in candidate_cases if case['n'] != 1000],
                   validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), **COUNTS))
    else:
        row['pending_validation'] = dict(expected_report=str(validation_path), expected_counts=COUNTS,
                                        report_status=validation.get('status') if validation else 'not-created',
                                        full_performance_runs_required_here=0)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared' if ready else 'prepared-pending-locked-validation',
                    language='uiua', created_at=datetime.now(timezone.utc).isoformat(),
                    scope='One additive optimized Uiua FASTA output-batching candidate; no full timing or refreeze admission.',
                    runtime=runtime, pins=pins, programs=[row],
                    original_sources=deepcopy(audit['original_sources']),
                    source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    validation_status='passed' if ready else 'pending', expected_validation_counts=COUNTS,
                    validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), **COUNTS) if ready else None,
                    installation_command=[PYTHON, str(SUITE / 'install_uiua_runtime.py')],
                    preparation_commands=[[PYTHON, str(HERE / name)] for name in ('prepare.py', 'validate.py', 'prepare_manifest.py')],
                    full_performance_runs=0,
                    additive_migration_policy='Preserve original/frozen evidence and retain both original compact and optimized candidates and measurements. This additional candidate does not replace either endpoint or select/refreeze programs; explicit admission and fresh full candidate/control measurements are required, and actual results determine eventual endpoints.')
    destination = HERE / 'manifest.json'
    destination.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print(destination, manifest['status'], sha(destination))
    print(row['key'], physical, 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
