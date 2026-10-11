"""Build only a new generic regex-count module and validate under the shared mutex."""
from pathlib import Path
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import subprocess

HERE = Path(__file__).resolve().parent
G = HERE.parent
C = G.parents[1]
OUT = HERE / 'results'
COUNT_SOURCE = HERE / 'support/regex-count.cpp'
COUNT_BINARY = G / '.build/apl-support/libregex-count-v3-apl.dylib'
SUBSTITUTE_SOURCE = G / 'support/apl/regex-native.cpp'
SUBSTITUTE_BINARY = G / '.build/apl-support/libregex-apl.dylib'
APL_ROOT = G / '.build/toolchains/apl-2.0'
RUNTIME = G / '.build/toolchains/gnu-apl-local/bin/apl'
PCRE = Path('/opt/homebrew/opt/pcre2/lib/libpcre2-32.0.dylib')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def text(value):
    if any(character in value for character in '\r\n\x00'):
        return '(⎕UCS ' + ' '.join(str(ord(character)) for character in value) + ')'
    return "'" + value.replace("'", "''") + "'"


def successful(process, expected):
    return (process.returncode == 0 and process.stdout == expected
            and process.stderr.decode().strip() in ('', 'WARNING: File pipe-to_client still open - closing it'))


def run(source, variant, data=b''):
    command = [str(G / 'run-apl.sh'), '--cc', '8' if variant == 'optimized' else '1', '-f', str(source)]
    process = subprocess.run(command, input=data, capture_output=True, cwd=G,
                             env=os.environ | {'LC_ALL': 'C'}, timeout=90)
    return process, dict(command=command, source=str(source), source_sha256=sha(source),
        returncode=process.returncode, stdout=process.stdout.decode(), stderr=process.stderr.decode(),
        stdout_bytes=len(process.stdout), stdout_sha256=hashlib.sha256(process.stdout).hexdigest())


def fixture_prefix():
    return ['⎕IO←0', "OUT←'w'⎕FIO[3]'/dev/fd/3'",
            "⊣'.build/apl-support/libregex-count-v3-apl.dylib'⎕FX'Count'",
            "⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'"]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    original_pins = {str(path): sha(path) for path in [SUBSTITUTE_SOURCE, SUBSTITUTE_BINARY,
                     RUNTIME, G / 'run-apl.sh', PCRE, G / 'sources/apl-v2/regexredux.compact.apl',
                     G / 'sources/apl-v2/regexredux.optimized.apl', G / 'apl-regex-v2-manifest.json']}
    command = ['/usr/bin/clang++', '-O3', '-std=c++11', '-dynamiclib', '-fPIC',
               '-undefined', 'dynamic_lookup', '-DHAVE_CONFIG_H', '-I' + str(APL_ROOT),
               '-I' + str(APL_ROOT / 'src'), '-I/opt/homebrew/opt/pcre2/include', str(COUNT_SOURCE),
               '-L/opt/homebrew/opt/pcre2/lib', '-lpcre2-32', '-o', str(COUNT_BINARY)]
    build = subprocess.run(command, cwd=G, capture_output=True, text=True)
    build_report = dict(command=command, returncode=build.returncode, stdout=build.stdout,
        stderr=build.stderr, source=str(COUNT_SOURCE), source_sha256=sha(COUNT_SOURCE),
        binary=str(COUNT_BINARY), binary_sha256=sha(COUNT_BINARY) if build.returncode == 0 else None,
        compiler=dict(path='/usr/bin/clang++', sha256=sha('/usr/bin/clang++'),
                      version=subprocess.check_output(['/usr/bin/clang++', '--version'], text=True)),
        pcre_header=dict(path='/opt/homebrew/opt/pcre2/include/pcre2.h', sha256=sha('/opt/homebrew/opt/pcre2/include/pcre2.h')))
    (OUT / 'build.json').write_text(json.dumps(build_report, indent=2) + '\n')
    assert build.returncode == 0, build.stderr
    spec = importlib.util.spec_from_file_location('apl_reference_v3', G / 'validate_apl.py')
    reference = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(reference)
    cases = [
        ('canonical-small', (C / 'regexredux/inputs/regexredux-input-small.txt').read_bytes()),
        ('case-and-replacements', b'>one\nagggtaaatttaccct\nAGGGTAAA\ntHaNtaNDcaNHaDWaSaNaSaNBY<x>|hello|\n'),
        ('empty-input', b''),
        ('header-only', b'>one\n>two\n'),
        ('no-header-no-final-newline', b'agggtaaatttacccttHaNtBY<tag>|hello|'),
        ('crlf-and-ungreedy-bars', b'>one\r\naggGtaaatttaccct\r\n|one||two|<a><b>WaS\r\n'),
        ('all-nine-positive-and-overlaps', b'>one\n' + b'\n'.join([
            b'agggtaaa', b'cgggtaaa', b'aaggtaaa', b'agagtaaa', b'aggataaa',
            b'agggcaaa', b'agggtgaa', b'agggtaca', b'agggtaac', b'tttaccct']) + b'\n'),
        ('read-chunk-split-pattern', b'>one\n' + b'A' * (1048576 - 10) + b'agggtaaatttacccttHaNtBY\n'),
    ]
    counts = [
        ('no-match', 'x', 'abc', 0, 0),
        ('nonoverlap', 'aa', 'aaaaa', 2, 0),
        ('alternation-order', 'a|aa', 'aaa', 3, 0),
        ('escaped-literal', r'a\+', 'a+ a+ aa', 2, 0),
        ('captures', '(a)(b)', 'ababab', 3, 0),
        ('backreferences', r'(.)\1', 'aabbcc', 3, 0),
        ('empty-pattern', '', 'ab', 3, 0),
        ('empty-pattern-and-subject', '', '', 1, 0),
        ('empty-subject-no-match', 'a', '', 0, 0),
        ('lookahead-empty', '(?=a)', 'aaa', 3, 0),
        ('non-greedy-empty-and-nonempty', 'a*?', 'aa', 5, 0),
        ('empty-alternation', '(?:|a)', 'a', 3, 0),
        ('greedy-plus-terminal-empty', 'a*', 'aa', 2, 0),
        ('unicode-literals', 'λ+', 'xλλyλ', 2, 0),
        ('unicode-codepoints', '.', '💡a', 2, 0),
        ('unicode-properties', r'\p{L}+', 'λΩ 123 A', 2, 0),
        ('caseless-flags', 'a', 'AaA', 3, 8),
        ('multiline-and-caseless-flags', '^a', 'a\nA\na', 3, 1032),
        ('UTF-UCP-flags', r'\w+', 'λΩ ! A', 2, 655360),
        ('inline-flags', '(?i)a', 'AaA', 3, 0),
        ('CRLF-newline-empty-progress', '(*CRLF)', '\r\n', 2, 0),
    ]
    substitutions = [
        ('literal-dollar', '(a)', '$1', 'ababa', '$1b$1b$1'),
        ('delete', 'x+', '', 'aaxxbbb', 'aabbb'),
        ('empty-match', '(?=b)', 'X', 'abc', 'aXbc'),
        ('empty-pattern', '', 'X', 'ab', 'XaXbX'),
        ('empty-subject', 'a', 'b', '', ''),
        ('delete-all', '.+', '', 'all', ''),
        ('expanding-buffer', 'a', 'abcdefghijklmnop', 'aaaaa', 'abcdefghijklmnop' * 5),
        ('unicode', 'λ+', 'Ω', 'xλλyλ', 'xΩyΩ'),
        ('nonoverlap', 'aa', 'X', 'aaaaa', 'XXa'),
        ('escaped-literal', r'a\+', 'X', 'a+ a+ aa', 'X X aa'),
        ('alternation', 'a|bb', 'X', 'abbabba', 'XXXXX'),
    ]
    rows = []
    for variant in ['compact', 'optimized']:
        source = HERE / 'sources' / f'regexredux.{variant}.apl'
        for name, data in cases:
            process, detail = run(source, variant, data)
            expected = reference.regex(data)
            passed = successful(process, expected)
            rows.append(dict(case=name, variant=variant, scope='benchmark', passed=passed,
                reference_match=passed, input_bytes=len(data), input_sha256=hashlib.sha256(data).hexdigest(),
                expected_stdout_sha256=hashlib.sha256(expected).hexdigest(), **detail))
            print(variant, name, 'PASS' if passed else 'FAIL', flush=True)
        fixture = OUT / f'generic-valid.{variant}.apl'
        lines = fixture_prefix()
        mode = 1 if variant == 'optimized' else 0
        for _, pattern, subject, expected, flags in counts:
            axis = f'[{flags} {mode}]'
            lines.append(f'Result←{text(pattern)} Count{axis} {text(subject)}')
            lines.append(f"⊣(('%d',⎕UCS 10)({expected}=Result))⎕FIO[22]OUT")
        lines.append(f"Result←'\\x00' Count[{mode}] ('a',(⎕UCS 0),'a')")
        lines.append("⊣(('%d',⎕UCS 10)(1=Result))⎕FIO[22]OUT")
        for _, pattern, replacement, subject, expected in substitutions:
            lines.append(f'Result←({text(pattern)} {text(replacement)}) Substitute[{mode}] {text(subject)}')
            lines.append(f"⊣(('%d',⎕UCS 10)({text(expected)}≡Result))⎕FIO[22]OUT")
        lines.extend(['⊣⎕FIO[4]OUT', ')OFF'])
        fixture.write_text('\n'.join(lines) + '\n')
        process, detail = run(fixture, variant)
        assertions = len(counts) + 1 + len(substitutions)
        passed = successful(process, b'1\n' * assertions)
        rows.append(dict(case='general-count-and-substitution-api', variant=variant, scope='generic-library',
            cases=[row[0] for row in counts] + ['embedded-NUL'] + [row[0] for row in substitutions],
            assertions=assertions, passed=passed, reference_match=passed, **detail))
        print(variant, 'general-count-and-substitution-api', 'PASS' if passed else 'FAIL', flush=True)
        for name, expression, diagnostic in [
            ('invalid-pattern', "'[' Count 'abc'", 'DOMAIN ERROR'),
            ('wrong-pattern-kind', "3 Count 'abc'", 'DOMAIN ERROR'),
            ('negative-flags', "'a' Count[¯1 0] 'abc'", 'DOMAIN ERROR'),
            ('invalid-JIT-option', "'a' Count[2] 'abc'", 'DOMAIN ERROR'),
            ('wrong-axis-length', "'a' Count[0 0 0] 'abc'", 'LENGTH ERROR'),
        ]:
            fixture = OUT / f'{name}.{variant}.apl'
            fixture.write_text('\n'.join(fixture_prefix() + [f'Result←{expression}', ')OFF']) + '\n')
            process, detail = run(fixture, variant)
            passed = process.stdout == b'' and diagnostic in process.stderr.decode()
            rows.append(dict(case=name, variant=variant, scope='generic-invalid-argument',
                expected_diagnostic=diagnostic, passed=passed, reference_match=passed, **detail))
            print(variant, name, 'PASS' if passed else 'FAIL', flush=True)
    assert all(sha(path) == digest for path, digest in original_pins.items())
    report = dict(schema_version=1, purpose='Canonical small/adversarial correctness; no full timings.',
        runtime_sha256=sha(RUNTIME), support_source_sha256=sha(COUNT_SOURCE),
        support_binary_sha256=sha(COUNT_BINARY), preserved_v2_pins=original_pins,
        lock='/tmp/mech-clbg-timing.lock', rows=rows, passed=all(row['passed'] for row in rows),
        benchmark_cases_per_variant=len(cases), generic_valid_assertions_per_variant=len(counts) + 1 + len(substitutions),
        generic_error_cases_per_variant=5,
        primary_api_documentation=['https://www.pcre.org/current/doc/html/pcre2_next_match.html',
                                   'https://www.pcre.org/current/doc/html/pcre2_match.html'])
    (OUT / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
    assert report['passed'], 'New v3 candidate has not passed correctness.'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--wait-for-lock', action='store_true')
    args = parser.parse_args()
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | (0 if args.wait_for_lock else fcntl.LOCK_NB))
        except BlockingIOError:
            raise SystemExit('Shared timing lock is busy; no compiler or validation process started.')
        print('Acquired timing mutex; building new generic count module and validating.', flush=True)
        main()
