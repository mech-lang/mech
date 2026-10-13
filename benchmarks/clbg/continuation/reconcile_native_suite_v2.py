"""Reconcile every requested native workload before canonical LOC/publication.

Only read completed immutable studies; do not invoke a benchmark or alter its
source, manifest, raw result, or summary. All hashing and evidence checks hold
the common timing mutex. The original owner must have released its lease.
"""
import argparse
from contextlib import ExitStack
from copy import deepcopy
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path

import reconcile_native_records_v2 as records

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
GUARD = CLBG / 'r-apl-uiua-comparison/20261011/uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
GUARD_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
BENCHMARKS = ('regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
              'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def load_guard():
    records.require(sha(GUARD) == GUARD_SHA, 'Frozen native sampler changed')
    spec = importlib.util.spec_from_file_location('native_suite_frozen_sampler', GUARD)
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    return guard


def read_json(path):
    return json.loads(Path(path).read_text())


def canonical_reference(harness, benchmark):
    if benchmark == 'nbody':
        return dict(energies=[-0.169075164, -0.169059907], tolerance=1e-8)
    if benchmark in ('pidigits', 'regexredux'):
        path = CLBG / benchmark / 'inputs/output-full.txt'
        return dict(bytes=path.stat().st_size, sha256=sha(path))
    if benchmark in ('spectralnorm', 'fannkuchredux'):
        data = (b'1.274224153\n' if benchmark == 'spectralnorm'
                else b'3968050\nPfannkuchen(12) = 65\n')
        return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
    size, digest = harness.compact.HASHES[benchmark][1]
    return dict(bytes=size, sha256=digest)


def frozen_manifest(root, benchmark, declared):
    filename = declared.get('manifest', declared.get('path'))
    path = Path(filename).resolve(strict=True)
    records.require(path == root / benchmark / 'manifest.json', 'Manifest escaped its named benchmark')
    records.require(sha(path) == declared['sha256'], 'Reviewed benchmark manifest changed')
    manifest = read_json(path)
    records.require(manifest['status'] == 'prepared-awaiting-root-review', 'Unexpected sealed source state')
    records.require(manifest['benchmark'] == benchmark, 'Manifest names another benchmark')
    records.require(manifest['pins'][str(GUARD)] == GUARD_SHA, 'Different sampler pin')
    return path, manifest


def lease_finished(stack, path):
    records.require(path.is_file(), 'Missing authoritative original owner lease')
    lease = stack.enter_context(path.open('a+'))
    # Never wait for an owner while holding the timing mutex: an active owner
    # may need that mutex for its final pin check. A live owner is a verified
    # wait, not a reason to resume or duplicate its measurement.
    fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)


def reconcile(language, study_root, reviewed_digest):
    root = Path(study_root).resolve(strict=True)
    cohort = CLBG / (language + '-comparison/20261012')
    records.require(root.is_relative_to(cohort) and root != cohort, 'Study root outside requested cohort')
    index_path = root / ('manifest.json' if language == 'pony' else 'index.json')
    records.require(sha(index_path) == reviewed_digest, 'Reviewed suite index changed')
    index = read_json(index_path)
    records.require(set(index['benchmarks']) == set(BENCHMARKS), 'Every one of ten workloads required')
    guard = load_guard()
    with ExitStack() as owners:
        global_report, report_paths = None, {}
        if language == 'pony':
            lease_finished(owners, root / 'owner.lock')
            guard.verify(index['pins'])
            global_report = read_json(root / 'report.json')
            records.require(global_report['status'] == 'complete' and global_report.get('active') is None and
                            global_report['manifest_sha256'] == reviewed_digest and
                            global_report['pins_unchanged_at_end'] is True, 'Pony suite is incomplete or changed')
            records.require(global_report['completed_benchmarks'] == index['benchmark_order'] and
                            len(index['benchmark_order']) == len(set(index['benchmark_order'])) == 10 and
                            set(index['benchmark_order']) == set(BENCHMARKS), 'Pony global work inventory changed')
            records.require(records.exact_integer(global_report['full_performance_runs'], 90) and
                            len(global_report['attempts']) == 90, 'Ninety actual global executions required')
            records.require(not global_report.get('correctable_source_failures'), 'Unqualified Pony source remains')
            records.require(len(index['profiles']) == len(global_report['programs']) == 30 and
                            {r['key'] for r in index['profiles']} == set(global_report['programs']),
                            'Pony source/control inventory changed')
            order = index['benchmark_order']
        else:
            for benchmark in BENCHMARKS:
                lease_finished(owners, root / benchmark / 'owner.lock')
            order = BENCHMARKS
        checked, provenance = {}, {str(index_path): reviewed_digest, str(Path(records.__file__).resolve()): sha(records.__file__), str(Path(records.original.__file__).resolve()): sha(records.original.__file__), str(Path(__file__).resolve()): sha(__file__)}
        for index_number, benchmark in enumerate(order):
            manifest_path, manifest = frozen_manifest(root, benchmark, index['benchmarks'][benchmark])
            guard.verify(manifest['pins'])
            harness = guard.load_harness()
            profiles = manifest['profiles']
            for row in profiles:
                records.require(manifest['pins'][row['source']] == row['source_sha256'] and
                                sha(row['source']) == row['source_sha256'], 'Exact compiled source pin missing')
                binary = row.get('binary', row['command'][0])
                records.require(binary == row['command'][0] and
                                manifest['pins'][binary] == row['binary_sha256'] and
                                sha(binary) == row['binary_sha256'], 'Exact measured native binary pin missing')
                workers = row.get('maximum_workers', row.get('maximum_compute_actors',
                              row.get('maximum_compute_workers')))
                # Older unchanged Mech controls carry the cohort worker policy.
                if workers is None and row['language'] == 'mech':
                    workers = manifest['maximum_compute_workers']
                records.require(type(workers) is int and 1 <= workers <= 8, 'Source worker bound missing or changed')
            if global_report is not None:
                offset = index_number * 9
                attempts = global_report['attempts'][offset:offset+9]
                records.require(all(a['benchmark'] == benchmark for a in attempts), 'Pony global attempt order changed')
                report_path = root / 'report.json'
                report = dict(status=global_report['status'], active=global_report.get('active'),
                    manifest_sha256=index['benchmarks'][benchmark]['sha256'],
                    pins_unchanged_at_end=global_report['pins_unchanged_at_end'],
                    orders=manifest['orders'], attempts=attempts,
                    profiles={row['id']: global_report['programs'][row['key']] for row in profiles})
                evidence_root = root / 'attempts'
            else:
                offset = 0
                report_path = root / benchmark / 'report.json'
                report = read_json(report_path)
                records.require(Path(report['manifest']).resolve() == manifest_path,
                                'Report names another reviewed manifest')
                evidence_root = root / benchmark / 'attempts'
            evidence_root = evidence_root.resolve(strict=True)

            def load_evidence(filename):
                path = Path(filename).resolve(strict=True)
                records.require(path.is_relative_to(evidence_root), 'Native evidence escaped its study')
                provenance[str(path)] = sha(path)
                return read_json(path)

            rows = records.validate_benchmark(manifest, report,
                index['benchmarks'][benchmark]['sha256'], language=language,
                canonical_n=harness.WORK[benchmark][1],
                reference=canonical_reference(harness, benchmark),
                fingerprint=lambda row: json.loads(harness.execution_fingerprint(row)),
                load_evidence=load_evidence, attempt_offset=offset)
            checked[benchmark] = dict(profiles=rows, manifest=str(manifest_path),
                manifest_sha256=index['benchmarks'][benchmark]['sha256'], report=str(report_path),
                report_sha256=sha(report_path), canonical_n=manifest['size'])
            provenance[str(manifest_path)] = index['benchmarks'][benchmark]['sha256']
            provenance[str(report_path)] = sha(report_path)
            report_paths[str(report_path)] = provenance[str(report_path)]
        for path, digest in report_paths.items():
            records.require(sha(path) == digest, 'Completed report changed while reconciling')
        result = dict(language=language,
            status='ten-native-workloads-reconciled-awaiting-canonical-LOC-and-publication',
            benchmarks=checked, canonical_full_native_processes=90, candidate_sources=20,
            actual_samples_per_source=3, provenance=provenance,
            records_reconciler_sha256=sha(records.__file__), base_records_reconciler_sha256=sha(records.original.__file__), suite_reconciler_sha256=sha(__file__),
            common_timing_mutex=str(LOCK), source_timing_relabels=0,
            source_or_report_mutations=0, canonical_LOC_admitted=False, chart_admitted=False,
            completed_at=datetime.now(timezone.utc).isoformat())
        return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--language', choices=sorted(records.LANGUAGES), required=True)
    parser.add_argument('--study-root', type=Path, required=True)
    parser.add_argument('--reviewed-index-sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print('WAIT common timing mutex: independent complete native suite', args.language, flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        records.require(not args.output.exists(), 'Never overwrite a reconciliation')
        result = reconcile(args.language, args.study_root, args.reviewed_index_sha256)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open('x') as stream:
            json.dump(result, stream, indent=2)
            stream.write('\n')
        print(json.dumps(dict(language=args.language, status=result['status'],
            full_processes=result['canonical_full_native_processes'], output=str(args.output))), flush=True)


if __name__ == '__main__':
    main()
