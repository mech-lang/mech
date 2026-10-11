#!/usr/bin/env python3
"""Pin the additive, small-validated vectorized candidates; run no workloads."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
PY = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def proof(path, first, last, conclusion):
    return dict(path=str(path), sha256=sha(path), first_line=first, last_line=last,
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]),
                conclusion=conclusion)


def loc(source):
    lines = source.splitlines()
    physical = sum(bool(line.split('#', 1)[0].strip()) for line in lines)
    expansions = []
    for number, line in enumerate(lines, 1):
        count = len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line.split('#', 1)[0]))
        if count > 1:
            expansions.append(dict(line=number, extra_lines=count-1,
                                   reason='Independent bindings packed on one physical line.'))
    return physical + sum(row['extra_lines'] for row in expansions), physical, expansions


def main():
    base = json.loads((SUITE / 'uiua-manifest.json').read_text())
    runtime = deepcopy(base['runtime'])
    original = next(row for row in base['programs']
                    if row['benchmark'] == 'spectralnorm' and row['variant'] == 'compact')
    environment = original['runtime_environment']
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed' and validation['passed'] == 21
    assert validation['work_checks_passed'] == 9
    assert validation['runtime_environment'] == environment
    assert validation['runtime_sha256'] == runtime['sha256'] == sha(runtime['path'])
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    source_root = Path(runtime['path']).parent / 'uiua-0.19.1'
    native_proof = [
        proof(source_root / 'src/algorithm/table.rs', 205, 213,
              'A directly tabled primitive and numeric arrays dispatch to native table_nums, rather than interpreting a composite coefficient function at each pair.'),
        proof(source_root / 'src/algorithm/table.rs', 413, 415,
              'The Add primitive selects fast_table_list with ordinary numeric addition.'),
        proof(source_root / 'src/algorithm/table.rs', 297, 311,
              'Mixed compact-byte and numeric index arrays convert to the same native f64 table_nums path; this includes early bounded row blocks with all 5500 input indices.'),
        proof(source_root / 'src/algorithm/table.rs', 449, 489,
              'Numeric arrays use f64; the fast table allocates one result array and writes every row/column pair in native loops.'),
        proof(source_root / 'src/algorithm/pervade.rs', 78, 126,
              'Pervasive shapes align from the first dimension. Forward adds the output-index vector to each row; transpose adds the input-index vector after transposing the triangular array.'),
        proof(source_root / 'src/algorithm/pervade.rs', 164, 210,
              'Ordinary pervasive arithmetic traverses scalar/array and aligned-array elements in native loops, rather than invoking a Uiua coefficient function per cell.'),
        proof(source_root / 'src/compile/pre_eval.rs', 89, 111,
              'Compiler pre-evaluation requires a zero-argument section. Compact coefficient construction consumes the live vector through length; bounded construction consumes the live Product.Offset. No coefficient array has a zero-argument global binding.'),
        proof(source_root / 'src/compile/pre_eval.rs', 27, 28,
              'Pre-evaluated values are limited to 1000 elements. A full 5500-element working vector and either 5500-by-5500 or 128-by-5500 coefficient arrays exceed this limit.'),
        proof(source_root / 'src/compile/pre_eval.rs', 151, 194,
              'Only successful bounded pre-evaluation results become cached constants; results over the element limit and timeouts produce None. This cannot replace full-workload V/U vectors or coefficient blocks with cached constant values.'),
    ]
    audit_path = HERE / 'results/source-audit.json'
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
        scope='Visible Uiua application source and pinned official runtime source; diagnostic instrumentation is separate from the timed source.',
        source_pins=validation['source_pins'], runtime_sha256=runtime['sha256'],
        native_array_proof=native_proof, coefficient_formula='1 / ((i+j)*(i+j+1)/2 + i + 1)',
        forward_products=20, transpose_products=20, matrix_vector_products=40,
        coefficient_evaluations_per_workload='40*N*N',
        work='19 Step applications followed by U=Step V; every Step evaluates A then A-transpose. Each product visits every input index for every output index.',
        cache_policy='Reusable coefficient-construction data are scalar N and bounded block start offsets. V/U are ordinary working vectors. Compact index vectors come from the current live vector length; bounded row indices come from the live Product.Offset. Sums, Triangle and coefficient expressions require live input; no coefficient or triangular matrix survives between products.',
        block_policy='Optimized output rows are partitioned at starts 0,128,... with min(128,N-start) rows; partial final blocks retained. Each block includes every one of the N input indices.',
        instrumentation=validation['instrumentation'],
        instrumentation_limit='Trace-only diagnostics retain every original expression. Source/runtime pre-evaluation proof independently establishes live-input dependency; instrumentation itself is not used for timing.',
        validation_file=str(validation_path), validation_sha256=sha(validation_path),
        full_workload_measured=False)
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(runtime['pins'])
    for item in native_proof:
        pins[item['path']] = item['sha256']
    for path in (HERE / 'prepare.py', HERE / 'prepare_manifest.py', HERE / 'validate.py',
                 HERE / 'README.md', validation_path, audit_path, SUITE / 'install_uiua_runtime.py',
                 SUITE / 'uiua-install-pins.json'):
        pins[str(path)] = sha(path)
    for row in validation['cases']:
        pins[row['stdout_file']] = sha(row['stdout_file'])
    for row in validation['instrumentation']:
        pins[row['instrumented_source']] = row['instrumented_source_sha256']
        pins[row['stdout_file']] = row['stdout_sha256']
    rows = []
    for name, variant in (('compact9', 'compact'), ('compact', 'compact'), ('fast', 'optimized')):
        source = HERE / 'sources' / (name + '.ua')
        digest = sha(source)
        assert validation['source_pins'][str(source)] == digest
        count, physical, expansions = loc(source.read_text())
        pins[str(source)] = digest
        cases = [deepcopy(row) for row in validation['cases'] if row['variant'] == name]
        small = next(row for row in cases if row['n'] == 100)
        assert small['reference_match'] is True
        small.update(validation_evidence_file=str(validation_path),
                     validation_evidence_sha256=sha(validation_path))
        sid = 'spectralnorm-uiua-vectorized-' + ('compact9' if name == 'compact9' else variant) + '-v3'
        command = [runtime['path'], 'run', '--no-format', str(source), '{n}']
        rows.append(dict(id=sid, key='spectralnorm:' + sid, benchmark='spectralnorm',
            language='uiua', variant=variant, profile=variant, source_profile=name,
            source=str(source), source_sha256=digest, command=command, cwd=str(SUITE),
            input_mode='argument', build_command=[], build_status='not-required',
            execution_mode='official-compiled-interpreter-runtime', runtime=runtime,
            runtime_sha256=runtime['sha256'], runtime_environment=environment,
            library_support=[], pins=deepcopy(pins), source_url=None,
            provenance='Locally authored additive Uiua vectorization; original frozen sources and measurements preserved.',
            adaptations=['Primitive outer addition creates each index-sum array; ordinary pervasive arithmetic computes every coefficient on demand. Optimized form uses at most 128 output rows.'],
            algorithm=dict(strict=True, native_benchmark_kernel=False, arithmetic='binary64',
                reason='Every one of all 40 matrix-vector products recomputes all N squared coefficients, using native general-purpose Uiua outer addition and pervasive array arithmetic. No coefficient or triangular matrix is cached. Ascending input-order sums and complete partial blocks are retained.',
                source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                full_work='20 A products + 20 A-transpose products; 40*N*N coefficient evaluations.',
                maximum_coefficient_block_rows='N' if name != 'fast' else 128),
            small_validation=small, adversarial_validation=[row for row in cases if row['n'] != 100],
            loc=count, physical_loc=physical, loc_expansions=expansions,
            loc_review='Each independent binding/data declaration is a separate nonblank noncomment line. Ordinary tacit pipelines remain one expression; no source packing discount.',
            eligible_for_chart=False, measurement_status='not-measured',
            recommended_timeout_seconds=14400, maximum_workers=1))
    for path, pin in pins.items():
        assert sha(path) == pin, path
    manifest = dict(status='prepared', language='uiua', scope='Additive spectralnorm candidates only; no full timing admission.',
        runtime=runtime, pins=pins, programs=rows,
        installation_command=[PY, str(SUITE / 'install_uiua_runtime.py')],
        preparation_commands=[[PY, str(HERE / 'prepare.py')],
                              [PY, str(HERE / 'validate.py')],
                              [PY, str(HERE / 'prepare_manifest.py')]],
        source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
        validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), passed=21,
                                 work_checks_passed=9),
        additive_migration_policy='Append or explicitly replace candidates through a new frozen audit; do not mutate old source/timing evidence. Candidate and fresh control require separate qualifying full measurements before chart admission.')
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print('Prepared additive manifest:', HERE / 'manifest.json')
    for row in rows:
        print(row['key'], row['loc'], 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
