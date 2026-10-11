#!/usr/bin/env python3
"""Pin the additive small-validated bounded native-count candidate; no runs."""
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
                excerpt='\n'.join(path.read_text().splitlines()[first-1:last]), conclusion=conclusion)


def main():
    base = json.loads((SUITE / 'uiua-manifest.json').read_text())
    runtime = deepcopy(base['runtime'])
    original = next(row for row in base['programs']
                    if row['benchmark'] == 'knucleotide' and row['variant'] == 'optimized')
    source = HERE / 'sources/fast.ua'
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed' and validation['passed'] == 14
    assert validation['work_checks_passed'] == 14
    assert validation['runtime_environment'] == original['runtime_environment']
    assert sha(source) == validation['source_sha256']
    assert validation['runtime_sha256'] == runtime['sha256'] == sha(runtime['path'])
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    assert sha(original['source']) == original['source_sha256'] == 'b3606cf00816ea8152bc2a05f1d1cd2a89bf689a60c75f106e2e279e6d9dad38'
    root = Path(runtime['path']).parent / 'uiua-0.19.1'
    native_proof = [
        proof(root / 'src/algorithm/stencil.rs', 149, 157,
              'Direct identity stencil selects the ordinary native array window path.'),
        proof(root / 'src/algorithm/stencil.rs', 210, 232,
              'The native identity stencil copies every overlapping window into the bounded chunk array.'),
        proof(root / 'src/algorithm/monadic/mod.rs', 1415, 1442,
              'Ordinary classify visits every complete row, assigns exact classes via HashMap row equality, and returns one index per window; it does not call a Uiua function per window.'),
        proof(root / 'src/algorithm/groups.rs', 543, 565,
              'Group length increments an ordinary native count for every nonnegative class index.'),
        proof(root / 'src/algorithm/groups.rs', 810, 839,
              'Primitive group First and Len dispatch directly to native firsts/lens rather than interpreting a function for every group.'),
        proof(root / 'src/algorithm/groups.rs', 607, 633,
              'Native group first copies one entire window key per class in classification order.'),
        proof(root / 'src/algorithm/map.rs', 274, 311,
              'Generic map insertion updates an existing count or appends a new value; all prior keys/values remain in the growing map.'),
        proof(root / 'src/algorithm/map.rs', 447, 505,
              'Generic map capacity grows at its load threshold; rehashing copies all prior keys, so completed chunk buffers are not retained by map keys.'),
        proof(root / 'src/algorithm/map.rs', 506, 591,
              'Generic map insertion probes colliding buckets, compares complete key rows, copies the key, and advances linearly until an equal or empty bucket is found.'),
        proof(root / 'src/algorithm/map.rs', 593, 625,
              'Generic map lookup probes with exact full-key equality, retaining correct counts across hash collisions.'),
        proof(root / 'src/algorithm/map.rs', 887, 908,
              'The generic map uses a 0.75 load threshold and DefaultHasher over each full key, followed by modulo-capacity bucket selection.'),
    ]
    audit_path = HERE / 'results/source-audit.json'
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
        source=str(source), source_sha256=sha(source), runtime_sha256=runtime['sha256'],
        original_source=str(original['source']), original_source_sha256=original['source_sha256'],
        transformation='Retain the original bounded chunk parser/loop/formatting; replace each window-level map increment with native class/first/length and a weighted generic merge per distinct chunk key.',
        widths=[1, 2, 3, 4, 6, 12, 18], maximum_chunk_starts=65536,
        chunk_partition='WindowCount=max(0,L-width+1); Start begins at 0; ChunkSize=min(65536,WindowCount-Start). ChunkText contains ChunkSize+width-1 characters beginning at Start. Identity stencil emits exactly ChunkSize overlapping windows. Advance adds ChunkSize to Start and carries the complete Counts map.',
        weighted_merge='ClassCounts produces every distinct complete key and its native group length. Each Weighted entry adds Weight to get(Key,Counts), with missing values filled by zero, then inserts the sum into the same complete growing map.',
        native_benchmark_kernel=False, host_helpers=False, input_period_assumptions=False,
        fixed_bucket_tables=False, query_only_maps=False, complete_growing_maps=7,
        full_sequence_length=125000000, full_total_windows=874999961,
        full_windows_by_width={str(width): 125000000-width+1 for width in [1,2,3,4,6,12,18]},
        numeric_policy='Counts remain exact binary64 integers below 2^53; canonical full counts are at most 125000000. Classification compares complete character-array keys.',
        native_runtime_proof=native_proof,
        instrumentation=validation['instrumentation'],
        validation_file=str(validation_path), validation_sha256=sha(validation_path),
        collision_evidence='Pinned native equality/linear-probe/growth code and exhaustive six-base/high-diversity full-map correctness fixtures. The official runtime exposes no native collision counter; observed collision counts are not claimed.',
        full_workload_measured=False)
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    pins = dict(runtime['pins'])
    pins.update({item['path']: item['sha256'] for item in native_proof})
    for path in (source, HERE / 'prepare.py', HERE / 'validate.py', HERE / 'prepare_manifest.py',
                 HERE / 'README.md', validation_path, audit_path, Path(original['source']),
                 SUITE / 'install_uiua_runtime.py', SUITE / 'uiua-install-pins.json',
                 Path(validation['instrumented_source'])):
        pins[str(path)] = sha(path)
    for case in validation['cases']:
        for field in ('input_file', 'stdout_file', 'stderr_file'):
            pins[case[field]] = sha(case[field])
    for case in validation['instrumentation']:
        for field in ('stdout_file', 'stderr_file'):
            pins[case[field]] = sha(case[field])
    physical = sum(bool(line.split('#', 1)[0].strip()) for line in source.read_text().splitlines())
    expansions = []
    for number, line in enumerate(source.read_text().splitlines(), 1):
        count = len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←', line.split('#', 1)[0]))
        if count > 1:
            expansions.append(dict(line=number, extra_lines=count-1, reason='Independent bindings packed on one physical line.'))
    normalized = physical + sum(row['extra_lines'] for row in expansions)
    assert normalized == physical == 29 and expansions == []
    small = deepcopy(next(case for case in validation['cases'] if case['case'] == 'canonical-small'))
    assert small['reference_match'] is True
    small.update(validation_evidence_file=str(validation_path), validation_evidence_sha256=sha(validation_path))
    row = dict(id='knucleotide-uiua-native-group-optimized-v2',
        key='knucleotide:knucleotide-uiua-native-group-optimized-v2', benchmark='knucleotide',
        language='uiua', variant='optimized', profile='optimized',
        source=str(source), source_sha256=sha(source),
        command=[runtime['path'], 'run', '--no-format', str(source)], cwd=str(SUITE),
        input_mode='stdin', build_command=[], build_status='not-required',
        execution_mode='official-compiled-interpreter-runtime', runtime=runtime,
        runtime_sha256=runtime['sha256'], runtime_environment=original['runtime_environment'],
        library_support=[], pins=deepcopy(pins), source_url=None,
        provenance='Locally authored additive Uiua native class/count optimization of the pinned original bounded source; all original source/timing evidence preserved.',
        adaptations=['Native classify and group first/length visit all chunk windows; weighted generic insertion retains all seven complete growing maps.'],
        algorithm=dict(strict=True, native_benchmark_kernel=False,
            reason='Every overlapping window of all seven widths is generated and natively classified/counted. Complete full keys and weights merge into the retained generic map across bounded chunks. No input-period assumptions, fixed bucket tables, query-only counting, or host helper.',
            widths=[1,2,3,4,6,12,18], complete_growing_maps=7, maximum_chunk_starts=65536,
            source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
            full_work='THREE length 125000000 at n25000000; all 874999961 overlapping windows counted.'),
        small_validation=small,
        adversarial_validation=[deepcopy(case) for case in validation['cases'] if case['case'] != 'canonical-small'],
        loc=normalized, physical_loc=physical, loc_expansions=expansions,
        loc_review='Every independent binding and data declaration occupies one separate line. The unchanged multiline formatter retains its physical lines; ordinary pipelines are single expressions.',
        eligible_for_chart=False, measurement_status='not-measured', recommended_timeout_seconds=14400,
        maximum_workers=1)
    for path, digest in pins.items():
        assert sha(path) == digest, path
    manifest = dict(status='prepared', language='uiua',
        scope='One additive optimized knucleotide candidate only; no full timing admission.',
        runtime=runtime, pins=pins, programs=[row],
        installation_command=[PY, str(SUITE / 'install_uiua_runtime.py')],
        preparation_commands=[[PY, str(HERE / 'prepare.py')], [PY, str(HERE / 'validate.py')],
                              [PY, str(HERE / 'prepare_manifest.py')]],
        source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
        validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path), passed=14,
                                 work_checks_passed=14, complete_seven_map_checks=14),
        additive_migration_policy='Append or explicitly supersede the original optimized candidate through a new frozen audit; preserve the original compact/source/timing evidence. Fresh full candidate/control measurements are required before chart admission.')
    path = HERE / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print(path, sha(path))
    print(row['key'], row['loc'], 'LOC', row['source_sha256'])


if __name__ == '__main__':
    main()
