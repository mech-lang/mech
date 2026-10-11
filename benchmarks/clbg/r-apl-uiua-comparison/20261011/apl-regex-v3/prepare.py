"""Create additive v3 metadata without changing any frozen v2 file."""
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
    old_path = G / 'apl-regex-v2-manifest.json'
    original = json.loads(old_path.read_text())
    report_path = HERE / 'results/validation.json'
    validation = json.loads(report_path.read_text())
    build_path = HERE / 'results/build.json'
    build = json.loads(build_path.read_text())
    assert validation['passed'] and all(row['passed'] for row in validation['rows'])
    assert build['returncode'] == 0
    for path, digest in validation['preserved_v2_pins'].items():
        assert sha(path) == digest, path
    source = HERE / 'support/regex-count.cpp'
    binary = G / '.build/apl-support/libregex-count-v3-apl.dylib'
    assert sha(source) == validation['support_source_sha256'] == build['source_sha256']
    assert sha(binary) == validation['support_binary_sha256'] == build['binary_sha256']
    pcre = Path('/opt/homebrew/opt/pcre2/lib/libpcre2-32.0.dylib')
    support_files = [source, binary, pcre, Path('/opt/homebrew/opt/pcre2/include/pcre2.h'),
                     Path('/usr/bin/clang++'), G / '.build/toolchains/apl-2.0/src/Native_interface.hh',
                     G / '.build/toolchains/apl-2.0/src/Value.hh',
                     G / '.build/toolchains/apl-2.0/src/native/template.hh']
    count_support = dict(name='General GNUAPL/PCRE2 nonoverlapping regex count',
        source=str(source), source_sha256=sha(source), binary=str(binary), binary_sha256=sha(binary),
        build_command=build['command'], build_report=str(build_path), build_report_sha256=sha(build_path),
        compiler=build['compiler'], library='PCRE2 10.49, 32-bit Unicode code-unit library',
        operations=['Count all nonoverlapping regex matches', 'Arbitrary compile-option bit flags',
                    'Optional ordinary PCRE2 JIT compilation'],
        interface='Pattern Count Subject; Pattern Count[1] Subject requests JIT; '
                  'Pattern Count[compileFlags jit] Subject supplies ordinary PCRE2 compile-option bits and JIT0/1.',
        native_benchmark_kernel=False,
        reason='Arbitrary strings/options only. Standard pcre2_compile/pcre2_match/pcre2_next_match calls '
               'produce a scalar count. One reusable match_data frame; no all-match collection or '
               'capture strings. It contains no benchmark patterns, headers, sequence, output logic, '
               'replacement rules, workload selection, or I/O.',
        source_url='https://www.gnu.org/software/apl/apl.html#Native-Functions',
        library_documentation=['https://www.pcre.org/current/doc/html/pcre2_next_match.html',
                               'https://www.pcre.org/current/doc/html/pcre2_match.html'],
        empty_match_semantics='Uses the standard PCRE2 global-match progression API, including '
                             'nonempty matches after empty matches and Unicode/newline boundaries.',
        pins={str(path): sha(path) for path in support_files})
    rows = []
    for variant in ['compact', 'optimized']:
        old = next(row for row in original['programs'] if row['variant'] == variant)
        row = copy.deepcopy(old)
        path = HERE / 'sources' / f'regexredux.{variant}.apl'
        small = copy.deepcopy(next(item for item in validation['rows']
                                   if item['case'] == 'canonical-small' and item['variant'] == variant))
        assert small['reference_match']
        support = copy.deepcopy(old['library_support']) + [count_support]
        row.update(id=f'regexredux-apl-v3-{variant}', key=f'regexredux:regexredux-apl-v3-{variant}',
            source=str(path), source_sha256=sha(path), loc=loc(path), normalized_loc=loc(path),
            physical_loc=loc(path), loc_expansions=[], library_support=support,
            supersedes_keys=[old['key']], supersedes_source_sha256=old['source_sha256'],
            small_validation=small, adversarial_validation_status='all-v3-cases-passed',
            adversarial_validation=dict(report=str(report_path), sha256=sha(report_path),
                benchmark_cases=validation['benchmark_cases_per_variant'],
                generic_valid_assertions=validation['generic_valid_assertions_per_variant'],
                generic_error_cases=validation['generic_error_cases_per_variant'], passed=True),
            command=[str(G / 'run-apl.sh'), '--cc', '8' if variant == 'optimized' else '1', '-f', str(path)],
            cwd=str(G), runtime_environment={'LC_ALL': 'C'},
            measurement_status='prepared-not-fully-timed', eligible_for_chart=False,
            samples_seconds=[], sample_validation=[], attempts=[])
        row['runtime']['pins'].update(count_support['pins'])
        row['source_provenance'] = ('New counted APL application retaining canonical regex-redux '
            'orchestration. Uses separately documented general PCRE2 count/substitution primitives '
            'through the published GNUAPL native-library interface.')
        row['adaptations'] = [
            'All nine literal patterns remain in one counted APL vector; the counted APL loop invokes a general matcher for each pattern and writes its actual scalar match count.',
            'Use standard PCRE2 global matching without constructing every captured match as an APL array.',
            'Retain v2 literal global substitution module/source/binary unchanged; APL owns cleanup and all five ordered complete replacements.',
            'Initial, cleaned and final lengths come from actual strings; clear initial input after cleanup.',
            'Ordinary PCRE2 interpreter matching.' if variant == 'compact' else
            'Request ordinary PCRE2 JIT for counting and substitution; library fallback preserves semantics if JIT is unavailable.']
        row['algorithm'] = dict(strict=True, native_benchmark_kernel=False,
            reason='Count all nine case-sensitive regexes with standard general matching, then '
                   'perform all five ordered complete substitutions. All patterns, sequence, '
                   'cleanup, length tracking, looping and output remain counted APL. Native '
                   'support performs only arbitrary-pattern library counting/substitution; '
                   'no benchmark-specific executor or precomputed results.')
        row.pop('full_failure', None)
        row.pop('timing_alias_of', None)
        rows.append(row)
    checks_per_variant = (validation['benchmark_cases_per_variant']
                          + validation['generic_valid_assertions_per_variant']
                          + validation['generic_error_cases_per_variant'])
    manifest = dict(schema_version=1, status='prepared-validated', language='apl',
        created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        programs=rows, native_support=count_support,
        validation=dict(path=str(report_path), sha256=sha(report_path), passed=True,
                        total_checks=2 * checks_per_variant),
        preserved_v2=dict(manifest=str(old_path), sha256=sha(old_path),
                          pins=validation['preserved_v2_pins'],
                          reason='All frozen v2 application/support files and evidence are unchanged.'),
        policy='General standard-library primitive only. No full benchmark timings in preparation; '
               'parent will merge/refreeze additively and measure all samples under existing guards.')
    out = HERE / 'manifest.json'
    out.write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(dict(manifest=str(out), validation_checks=2 * checks_per_variant,
        programs=[dict(key=row['key'], loc=row['loc'], source_sha256=row['source_sha256']) for row in rows],
        count_source_sha256=sha(source), count_binary_sha256=sha(binary)), indent=2))


if __name__ == '__main__':
    main()
