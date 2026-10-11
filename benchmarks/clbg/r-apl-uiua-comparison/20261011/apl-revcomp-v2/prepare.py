#!/usr/bin/env python3
"""Prepare additive revcomp v2 metadata without editing the active frozen cohort."""
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
    original = json.loads((G / 'apl-manifest.json').read_text())
    report_path = HERE / 'results/validation.json'
    report = json.loads(report_path.read_text())
    assert report['passed'] and len(report['rows']) == 44
    assert all(row['reference_match'] for row in report['rows'])
    assert all(sha(path) == digest for path, digest in report['pins'].items())
    launcher = HERE / 'run-apl-scratch.sh'
    helpers = [launcher, G / 'run-apl.sh', Path('/bin/sh'), Path('/usr/bin/mktemp'),
               Path('/bin/rm'), Path('/bin/rmdir')]
    helper_pins = {str(path): sha(path) for path in helpers}
    support = dict(name='General private scratch-path launcher', source=str(launcher),
        source_sha256=sha(launcher), binary=None, build_command=[], pins=helper_pins,
        native_benchmark_kernel=False,
        operations=['Create unique private temporary directory', 'Pass scratch pathname to interpreter',
                    'Remove scratch file and directory on exit'],
        reason='Standard process/file I/O only. The launcher does not read benchmark input, parse records, '
               'select or transform sequence bytes, reverse data, calculate offsets, wrap output, '
               'or implement any benchmark algorithm.',
        provenance=['GNU APL2.0 Quad_FIO.cc ordinary fopen/fread/fwrite/ftell/fseek bindings',
                    'macOS standard mktemp(1), rm(1), rmdir(1) and sh(1)'],
        documentation='https://www.gnu.org/software/apl/apl.html',
        api_source=str(G / '.build/toolchains/apl-2.0/src/Quad_FIO.cc'))
    rows = []
    preserved = []
    for variant in ('compact', 'optimized'):
        old = next(row for row in original['programs']
                   if row['benchmark'] == 'revcomp' and row['variant'] == variant)
        preserved.append(dict(key=old['key'], source=old['source'], source_sha256=old['source_sha256']))
        row = copy.deepcopy(old)
        path = HERE / 'sources' / f'revcomp.{variant}.apl'
        small = copy.deepcopy(next(item for item in report['rows']
                                   if item['case'] == 'canonical-small' and item['variant'] == variant))
        row.update(id=f'revcomp-apl-v2-{variant}', key=f'revcomp:revcomp-apl-v2-{variant}',
            source=str(path), source_sha256=sha(path), loc=loc(path), normalized_loc=loc(path),
            physical_loc=loc(path), loc_expansions=[], library_support=[support],
            command=[str(launcher), '--cc', '8' if variant == 'optimized' else '1', '-f', str(path)],
            cwd=str(G), runtime_environment={'LC_ALL': 'C'},
            supersedes_keys=[old['key']], supersedes_source_sha256=old['source_sha256'],
            build_status='not-required', build_command=[], small_validation=small,
            adversarial_validation_status='all-v2-cases-passed',
            adversarial_validation=dict(report=str(report_path), sha256=sha(report_path),
                cases_per_variant=22, passed=True),
            source_provenance='Newly authored counted APL bounded-memory canonical implementation. '
                'The original cross-language reference and failed APL sources are retained.',
            measurement_status='prepared-not-fully-timed', eligible_for_chart=False,
            samples_seconds=[], sample_validation=[], attempts=[])
        row['runtime']['pins'].update(helper_pins)
        row['adaptations'] = [
            'Standard scratch-path launcher creates one private path and cleans it up; all algorithm and offsets remain counted APL.',
            'Spool one current record through ordinary GNUAPL ⎕FIO fopen/fwrite, with no whole-input or whole-sequence APL array.',
            'Actually read every stored sequence byte in bounded slices from the end, apply the full upper/lower IUPAC mapping and reverse each slice in APL.',
            'Write complete headers and 60-column sequence output; normalize CRLF and skip semicolon comment lines, including across input chunk boundaries.',
            'Compact uses standard fgets fragments and records header/comment/line state.' if variant == 'compact' else
            'Optimized uses 245,760-byte fread chunks; general regex boundary positions and bounded APL slices replace per-line input dispatch.']
        row['algorithm'] = dict(strict=True, native_benchmark_kernel=False,
            reason='All record parsing, comment handling, sequence spooling, backward slice offsets, '
                   'full IUPAC complement lookup, actual reversal, header preservation and 60-column '
                   'complete output remain counted APL. Support provides only ordinary temporary-path '
                   'lifecycle; no native transformation or benchmark kernel.',
            maximum_input_chunk=262143 if variant == 'compact' else 245760,
            maximum_reverse_chunk=245760,
            bounded_memory='All APL input/transform/output arrays are bounded by chunk size. '
                'The largest individual record occupies an ordinary temporary disk file; '
                'no entire input, line, header or sequence is retained in APL memory.')
        row.pop('full_failure', None)
        row.pop('timing_alias_of', None)
        rows.append(row)
    result = dict(schema_version=1, status='prepared-validated-pending-guarded-full-timing',
        created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(), language='apl',
        programs=rows, native_support=support,
        preserved_v1=dict(sources=preserved, failures='Retained by the parent in the active frozen cohort and evidence archives; no old source or report edited.'),
        validation=dict(path=str(report_path), sha256=sha(report_path), passed=True, checks=44),
        capacity_review=dict(runtime_changed=False,
            full_input_bytes=254166745,
            diagnosis='The full input alone needs approximately4GiB in GNUAPL16-byte array cells, '
                      'above its macOS workspace fallback before whole-record copies.',
            correction='Replace whole-input/whole-record arrays with application-owned bounded '
                       'streaming and ordinary seekable temporary-file storage.',
            code_evidence=['Quad_FIO.cc:2427–2546 (standard fopen/fread/fwrite/fseek)',
                           'Quad_FIO.cc:1633–1637 (standard ftell)',
                           'Quad_WA.cc:172–187 (macOS /proc fallback)',
                           'Value.cc:196–215 (workspace check)'],
            guards='Unchanged parent memory/power/workload/output/timing guards. No full-size run has been performed by this preparation.'))
    path = HERE / 'manifest.json'
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(dict(manifest=str(path), validation_checks=44,
        programs=[dict(key=row['key'], loc=row['loc'], source_sha256=row['source_sha256']) for row in rows],
        support_pins=helper_pins), indent=2))


if __name__ == '__main__':
    main()
