#!/usr/bin/env python3
"""Package measured CLBG source and timing samples for the static explorer."""
import argparse, hashlib, json, statistics, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def build(root, array_audit_dir):
    sys.path.insert(0, str(root / 'overview'))
    from plot_ten import data_model
    from language_colors import COLORS
    model = data_model()
    workloads = {s['key']: s['workload'] for s in json.loads((root / 'overview' / 'normalized-results.json').read_text())}
    for key in ('regexredux', 'pidigits'):
        report = json.loads((root / key / 'results' / 'apple-m3-ac-normal-20261009.json').read_text())
        workloads[key] = report['protocol']['workload']
    report_specs = [
        ('retime/results/apple-m3-ac-normal-20261008.json', None),
        ('fast-scripting/results/apple-m3-ac-normal-20261009.json', None),
        ('fast-scripting/results/runtime-policies.json', None),
        ('nbody/experiments/20261009-local-simd/final-confirmation-50m.json', 'nbody'),
        ('regexredux/results/apple-m3-ac-normal-20261009.json', 'regexredux'),
        ('pidigits/results/apple-m3-ac-normal-20261009.json', 'pidigits'),
        ('remaining-conformance/signed-division-confirmation.json', None),
    ]
    records = {}
    reports = {}
    for relative, default in report_specs:
        path = root / relative
        report = json.loads(path.read_text())
        reports[relative] = digest(path)
        for row in report['programs']:
            benchmark = row.get('benchmark', default)
            if benchmark and row.get('samples_seconds'):
                records.setdefault((benchmark, row['id']), []).append((row, relative))

    source_index = None
    def find_file(benchmark, relative, expected):
        nonlocal source_index
        path = Path(relative)
        candidates = [path] if path.is_absolute() else [root / benchmark / path, root / 'compact' / path, root / path]
        for candidate in candidates:
            if candidate.is_file() and (not expected or digest(candidate) == expected):
                return candidate
        if expected:
            if source_index is None:
                paths = subprocess.check_output([
                    'rg', '--files', '--hidden', '-g', '!**/target/**', '-g', '!**/.build/**',
                    '-g', '!**/.git/**', str(root)
                ], text=True).splitlines()
                extensions = {'.mec', '.rs', '.c', '.cpp', '.h', '.hpp', '.chpl', '.java', '.go', '.jl', '.py', '.rb', '.pl', '.lua', '.php', '.js'}
                source_index = {}
                for item in paths:
                    candidate = Path(item)
                    if candidate.suffix in extensions and candidate.is_file():
                        source_index.setdefault(digest(candidate), candidate)
            if expected in source_index:
                return source_index[expected]
        raise ValueError(f'No measured source for {benchmark}/{relative} (expected {expected})')

    programs = {}
    files = {}
    benchmarks = []
    exclusions = []
    for study in model['studies']:
        groups = []
        for group in study['groups']:
            candidates = []
            for point in group['builds']:
                algorithm = point.get('algorithm') or {}
                if algorithm.get('strict') is False:
                    exclusions.append(dict(key=study['key'] + ':' + point['id'],
                                           benchmark=study['key'], language=group['lang'],
                                           reason=algorithm.get('reason', 'Required work mismatch.')))
                else:
                    candidates.append(point)
            if not candidates:
                continue
            fast = min(candidates, key=lambda p:(p['seconds'], p['loc']))
            small = min(candidates, key=lambda p:(p['loc'], p['seconds']))
            builds = []
            endpoint_ids = {fast['id'], small['id']}
            for point in candidates:
                if point['id'] not in endpoint_ids:
                    continue
                matches = records.get((study['key'], point['id']), [])
                matches = [(row, report) for row, report in matches
                           if abs(statistics.median(row['samples_seconds']) - point['seconds']) < 1e-10]
                if not matches:
                    raise ValueError(f'No timing record: {study["key"]}/{point["id"]}')
                row, report = matches[-1]
                source_report = report
                # Confirmation runs reuse the existing compiled native program;
                # their shorter records keep timings/binary hashes but omit source metadata.
                if not row.get('path') and not row.get('source'):
                    earlier = [(r, provenance) for r, provenance in records[(study['key'], point['id'])]
                               if r.get('path') or r.get('source')]
                    if earlier:
                        metadata, source_report = earlier[-1]
                        row = dict(metadata, **row)
                source = row.get('source', {})
                paths = source.get('paths', []) if isinstance(source, dict) else [source]
                if not paths and row.get('path'):
                    paths = [row['path']]
                if not paths:
                    raise ValueError(f'No source path: {study["key"]}/{point["id"]}')
                hashes = source.get('sha256') if isinstance(source, dict) else row.get('source_sha256')
                file_ids = []
                for path in paths:
                    if group['lang'] == 'mech' and not str(path).endswith('.mec'):
                        continue
                    expected = hashes.get(path) if isinstance(hashes, dict) else hashes or row.get('sha256')
                    resolved = find_file(study['key'], path, expected)
                    sha = digest(resolved)
                    assert not expected or sha == expected
                    file_ids.append(sha)
                    if sha not in files:
                        files[sha] = dict(name=resolved.name, path=str(resolved.relative_to(root)), sha256=sha,
                                         code=resolved.read_text(errors='strict'))
                assert file_ids, (study['key'], point['id'])
                key = study['key'] + ':' + point['id']
                samples = row['samples_seconds']
                assert len(samples) >= 3
                programs[key] = dict(
                    key=key, id=point['id'], benchmark=study['key'], language=group['lang'],
                    loc=point['loc'], seconds=point['seconds'], x=point['x'], y=point['y'],
                    samples=samples, timing_sd=statistics.stdev(samples), files=file_ids,
                    source_url=row.get('url'), report=report, source_report=source_report, algorithm=point.get('algorithm'),
                    runtime_policy=point.get('runtime_policy'),
                )
                builds.append(key)
            groups.append(dict(language=group['lang'], label=group['label'], programs=builds,
                               fast=study['key'] + ':' + fast['id'],
                               small=study['key'] + ':' + small['id']))
        # Keep the original measured cohort's baselines even when visibility changes.
        benchmarks.append(dict(key=study['key'], title=study['title'],
                               workload=workloads[study['key']],
                               baseline_loc=study['size_baseline_loc'],
                               baseline_seconds=study['time_baseline_seconds'], groups=groups))
    languages = [dict(key=row['lang'], label=row['label'], color=COLORS[row['lang']])
                 for row in model['summary']]
    # The original summary admits complete suites only. Keep measured partial
    # suites too, with explicit coverage; never manufacture missing timings.
    for benchmark in benchmarks:
        for group in benchmark['groups']:
            if not any(l['key'] == group['language'] for l in languages):
                languages.append(dict(key=group['language'], label=group['label'], color=COLORS[group['language']]))
    payload = dict(schema_version=1, languages=languages, benchmarks=benchmarks, programs=programs, files=files,
                   exclusions=exclusions,
                   reference_summary=model['summary'], provenance=reports,
                   protocol=dict(hardware='Apple M3', power='AC power; Low Power Mode off',
                                 programs='Fastest and fewest-LOC endpoints only',
                                 measured='October 8–9, 2026', metric='Nonblank, noncomment source lines',
                                 timing='Median elapsed time of fresh processes, including startup and full output',
                                 spread='One sample standard deviation in log-ratio space, retaining covariance; not timing uncertainty'))
    from augment_data import enrich_payload
    enrichment = enrich_payload(root, payload, array_audit_dir)
    programs, files = payload['programs'], payload['files']
    (HERE / 'dist' / 'data.json').write_text(json.dumps(payload, separators=(',', ':'), ensure_ascii=False) + '\n')
    audit = dict(benchmarks=len(benchmarks), languages=len(languages), programs=len(programs),
                 program_scope='fastest_and_fewest_loc',
                 unique_sources=len(files), samples=sum(len(p['samples']) for p in programs.values()),
                 all_source_hashes_verified=True, all_medians_match_chart=True,
                 reference_sha256=digest(root / 'overview' / 'ten-loc-endpoints.json'), provenance=payload['provenance'], enrichment=enrichment)
    (HERE / 'data-audit.json').write_text(json.dumps(audit, indent=2) + '\n')
    print(json.dumps(audit))

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--clbg-root', required=True, type=Path)
    parser.add_argument('--array-audit-dir', required=True, type=Path)
    args = parser.parse_args()
    build(args.clbg_root.resolve(), args.array_audit_dir.resolve())
