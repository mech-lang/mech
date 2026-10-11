"""Locked small/adversarial checks for v2 regex and its generic substitution API."""
import fcntl
import hashlib
import importlib.util
import json
import os
import subprocess
from pathlib import Path

G = Path(__file__).resolve().parent
C = G.parents[1]
OUT = G / 'results/apl/regex-v2'
OUT.mkdir(parents=True, exist_ok=True)
spec = importlib.util.spec_from_file_location('apl_reference', G / 'validate_apl.py')
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(source, variant, data=b''):
    command = [str(G / 'run-apl.sh'), '--cc', '8' if variant == 'optimized' else '1',
               '-f', str(source)]
    process = subprocess.run(command, cwd=G, input=data, capture_output=True,
                             env=os.environ | {'LC_ALL': 'C'}, timeout=90)
    return process, dict(command=command, source=str(source), source_sha256=sha(source),
        returncode=process.returncode, stdout=process.stdout.decode(), stderr=process.stderr.decode(),
        stdout_bytes=len(process.stdout), stdout_sha256=hashlib.sha256(process.stdout).hexdigest())


def apl_string(value):
    return "'" + value.replace("'", "''") + "'"


def successful(process, expected):
    # GNU APL emits this internal shared-variable-pipe cleanup notice at exit.
    # Preserve it in evidence; every other diagnostic, including WS FULL+, fails.
    diagnostic = process.stderr.decode().strip()
    benign = ('', 'WARNING: File pipe-to_client still open - closing it')
    return process.returncode == 0 and process.stdout == expected and diagnostic in benign


def main():
    cases = [
        ('canonical-small', (C / 'regexredux/inputs/regexredux-input-small.txt').read_bytes()),
        ('case-and-replacements', b'>one\nagggtaaatttaccct\nAGGGTAAA\ntHaNtaNDcaNHaDWaSaNaSaNBY<x>|hello|\n'),
        ('empty-input', b''),
        ('header-only', b'>one\n>two\n'),
        ('no-header-no-final-newline', b'agggtaaatttacccttHaNtBY<tag>|hello|'),
        ('crlf-and-ungreedy-bars', b'>one\r\naggGtaaatttaccct\r\n|one||two|<a><b>WaS\r\n'),
    ]
    generic = [
        ('literal-dollar', '(a)', '$1', 'ababa', '$1b$1b$1'),
        ('delete', 'x+', '', 'aaxxbbb', 'aabbb'),
        ('empty-match', '(?=b)', 'X', 'abc', 'aXbc'),
        ('empty-pattern', '', 'X', 'ab', 'XaXbX'),
        ('empty-subject', 'a', 'b', '', ''),
        ('delete-all', '.+', '', 'all', ''),
        ('expanding-buffer', 'a', 'abcdefghijklmnop', 'aaaaa', 'abcdefghijklmnop' * 5),
        ('unicode', 'λ+', 'Ω', 'xλλyλ', 'xΩyΩ'),
    ]
    rows = []
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise SystemExit('Shared timing lock busy; no validation process started.')
        for variant in ('compact', 'optimized'):
            source = G / 'sources/apl-v2' / f'regexredux.{variant}.apl'
            for name, data in cases:
                p, detail = run(source, variant, data)
                expected = reference.regex(data)
                passed = successful(p, expected)
                rows.append(dict(case=name, variant=variant, scope='benchmark', passed=passed,
                    input_bytes=len(data), input_sha256=hashlib.sha256(data).hexdigest(),
                    expected_stdout_sha256=hashlib.sha256(expected).hexdigest(), **detail))
                print(variant, name, 'PASS' if passed else 'FAIL', flush=True)
            fixture = OUT / f'generic-substitution.{variant}.apl'
            mode = '[1]' if variant == 'optimized' else ''
            lines = ['⎕IO←0', "OUT←'w'⎕FIO[3]'/dev/fd/3'",
                     "⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'"]
            for _, pattern, replacement, subject, expected in generic:
                lines.append(f'Result←({apl_string(pattern)} {apl_string(replacement)}) Substitute{mode} {apl_string(subject)}')
                lines.append(f"⊣(('%d',⎕UCS 10)({apl_string(expected)}≡Result))⎕FIO[22]OUT")
            lines.extend(['⊣⎕FIO[4]OUT', ')OFF'])
            fixture.write_text('\n'.join(lines) + '\n')
            p, detail = run(fixture, variant)
            passed = successful(p, b'1\n' * len(generic))
            rows.append(dict(case='general-substitution-api', variant=variant, scope='generic-library',
                cases=[g[0] for g in generic], passed=passed, **detail))
            print(variant, 'general-substitution-api', 'PASS' if passed else 'FAIL', flush=True)
        fcntl.flock(lock, fcntl.LOCK_UN)
    report = dict(schema_version=1, purpose='Small correctness only; no full workloads or timing samples.',
        lock='/tmp/mech-clbg-timing.lock', runtime_sha256=sha(G / '.build/toolchains/gnu-apl-local/bin/apl'),
        support_source_sha256=sha(G / 'support/apl/regex-native.cpp'),
        support_binary_sha256=sha(G / '.build/apl-support/libregex-apl.dylib'),
        rows=rows, passed=all(r['passed'] for r in rows))
    (OUT / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
    assert report['passed'], 'v2 not admitted: a correctness case failed.'


if __name__ == '__main__':
    main()
