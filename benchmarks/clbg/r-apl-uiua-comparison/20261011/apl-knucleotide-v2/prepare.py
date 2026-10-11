"""Pin additive streaming applications after their independent checks pass."""
from pathlib import Path
import copy
import datetime
import hashlib
import json

HERE = Path(__file__).resolve().parent
G = HERE.parent


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def loc(path):
    lines = Path(path).read_text().splitlines()
    assert all('⋄' not in line and '◊' not in line for line in lines)
    return sum(bool(line.strip()) and not line.lstrip().startswith('⍝')
               and line.strip() != '∇' for line in lines)


def main():
    report_path = HERE / 'results/validation.json'
    report = json.loads(report_path.read_text())
    assert report['passed'] and all(row['passed'] for row in report['rows'])
    for path, digest in report['preserved_pins'].items():
        assert sha(path) == digest, path
    old_path = G / 'apl-manifest.json'
    old = json.loads(old_path.read_text())
    rows = []
    for variant in ['compact', 'optimized']:
        prior = next(row for row in old['programs']
                     if row['benchmark'] == 'knucleotide' and row['variant'] == variant)
        row = copy.deepcopy(prior)
        source = HERE / 'sources' / f'knucleotide.{variant}.apl'
        small = copy.deepcopy(next(item for item in report['rows']
            if item['variant'] == variant and item['case'] == 'canonical-small'))
        assert small['reference_match']
        validations = [item for item in report['rows'] if item['variant'] == variant]
        row.update(id=f'knucleotide-apl-v2-{variant}',
            key=f'knucleotide:knucleotide-apl-v2-{variant}', source=str(source), source_sha256=sha(source),
            loc=loc(source), normalized_loc=loc(source), physical_loc=loc(source), loc_expansions=[],
            source_provenance='New counted GNU APL application with bounded record parsing and seven growing maps; no native support added.',
            command=[str(G / 'run-apl.sh'), '--cc', '8' if variant == 'optimized' else '1', '-f', str(source)],
            cwd=str(G), runtime_environment={'LC_ALL': 'C'},
            small_validation=small, adversarial_validation_status='all-v2-cases-passed',
            adversarial_validation=dict(report=str(report_path), sha256=sha(report_path),
                passed=True, case_count=len(validations),
                scopes=['benchmark-output', 'generic-map', 'all-seven-complete-maps']),
            supersedes_keys=[prior['key']], supersedes_source_sha256=prior['source_sha256'],
            measurement_status='prepared-not-fully-timed', eligible_for_chart=False,
            samples_seconds=[], sample_validation=[], attempts=[], library_support=[])
        row['adaptations'] = [
            'Read bounded 65,536-byte buffers, select the THREE record across arbitrary header/comment boundaries, normalize case and remove CR/LF in APL.',
            'Maintain seven live, independently growing open-addressed maps for widths 1,2,3,4,6,12,18; carry at most k−1 previous codes per width, count each overlapping window once.',
            'Drop temporary aliases before table updates; each map persists between input chunks, and all seven persist through final output.',
            'Original rolling integer-key scalar map updates retained.' if variant == 'compact' else
            'Encode every window through ordinary bounded array operations; grade and combine adjacent equal keys, then batch-probe and scatter map updates. Explicitly resolve collisions against both existing and simultaneous incoming keys; rehash all entries on growth.',
            'Every application helper is counted; only unchanged GNU APL array primitives and standard IO are used. No native/library benchmark executor, fixed key universe, RNG-period reuse or query-only count shortcut.']
        row['algorithm'] = dict(strict=True, native_benchmark_kernel=False,
            reason='Seven generic growing maps retain all keys and counts from all overlapping windows. '
                   'APL owns record parsing, full-window encoding/counting, equality/probing/growth, '
                   'frequency sorting and every output. Fast batching uses ordinary grade/gather/scatter '
                   'and counts duplicate input keys by their exact amounts; it makes no assumptions '
                   'about canonical input repetition, known answers or the set of encountered keys.')
        for field in ['timing_alias_of', 'full_failure', 'full_validation', 'median_seconds',
                      'sample_stdev_seconds', 'aliased_sample_source']:
            row.pop(field, None)
        assert sha(source) == next(item['source_sha256'] for item in report['rows']
                                   if item['variant'] == variant and item['case'] == 'canonical-small')
        rows.append(row)
    manifest = dict(schema_version=1, status='prepared-validated', language='apl',
        created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(), programs=rows,
        validation=dict(path=str(report_path), sha256=sha(report_path), passed=True, total_checks=report['checks']),
        memory_analysis=report['memory_analysis'], preserved_pins=report['preserved_pins'],
        source_recipe=dict(path=str(HERE / 'prepare_sources.py'), sha256=sha(HERE / 'prepare_sources.py')),
        old_manifest=dict(path=str(old_path), sha256=sha(old_path)),
        policy='Additive candidates only; frozen original sources/evidence unchanged. No full timings in preparation. Parent measures under common guards.')
    out = HERE / 'manifest.json'
    out.write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(dict(manifest=str(out), validation_checks=report['checks'],
        programs=[dict(key=row['key'], loc=row['loc'], source_sha256=row['source_sha256']) for row in rows]), indent=2))


if __name__ == '__main__':
    main()
