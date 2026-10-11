#!/usr/bin/env python3
"""Mutexed bounded output, exact draw/CDF/carry and generic wrapping checks."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
CLBG = SUITE.parents[1]
SOURCE = HERE / 'sources/fast.ua'
ORIGINAL = SUITE / 'sources/uiua/fasta/fast.ua'
COMPACT = SUITE / 'sources/uiua/fasta/compact.ua'
RESULTS = HERE / 'results'
LOCK = Path('/tmp/mech-clbg-timing.lock')
ENV = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
RUNTIME_META = json.loads((SUITE / 'uiua-manifest.json').read_text())['runtime']
RUNTIME = Path(RUNTIME_META['path'])
CAPACITY = 65520
CASES = (0, 1, 2, 7, 19, 20, 21, 59, 60, 61, 1000, 11999, 12000, 12001,
         13103, 13104, 13105, 19999, 20000, 20001, 21839, 21840, 21841,
         32759, 32760, 32761, 40003)
TRACE_CASES = (0, 1, 61, 13105, 21841, 40003)
HEADERS = ('>ONE Homo sapiens alu', '>TWO IUB ambiguity codes', '>THREE Homo sapiens frequency')
ALU = 'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'
LETTERS = ('acgtBDHKMNRSVWY', 'acgt')
PROBABILITIES = ([.27, .12, .12, .27] + [.02]*11,
                 [.3029549426680, .1979883004921, .1975473066391, .3015094502008])


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def cdf(probabilities):
    result = []
    total = 0.0
    for value in probabilities:
        total += value
        result.append(total)
    return result


def binary(values):
    return b''.join(struct.pack('<d', float(value)) for value in values)


def wrap(text):
    return ''.join(text[i:i+60]+'\n' for i in range(0, len(text), 60))


def classify(draws, letters, thresholds):
    return ''.join(letters[sum(value >= threshold for threshold in thresholds)]
                   for value in (draw/139968.0 for draw in draws))


def reference(n):
    seed = 42
    records = [''.join(ALU[i % len(ALU)] for i in range(2*n))]
    ends = []
    for count, letters, probabilities in zip((3*n, 5*n), LETTERS, PROBABILITIES):
        thresholds = cdf(probabilities)
        chars = []
        for _ in range(count):
            seed = (3877*seed+29573) % 139968
            value = seed/139968.0
            chars.append(letters[sum(value >= threshold for threshold in thresholds)])
        records.append(''.join(chars))
        ends.append(seed)
    return ''.join(header+'\n'+wrap(text) for header, text in zip(HEADERS, records)).encode(), ends


def execute(source, n=0, data=None):
    command = [str(RUNTIME), 'run', '--no-format', str(source), str(n)]
    done = subprocess.run(command, cwd=SUITE, env=dict(os.environ, **ENV),
                          input=data, capture_output=True, timeout=120)
    return command, done


def persist(source, n, command, done, stem):
    out = RESULTS / (stem+'.stdout')
    err = RESULTS / (stem+'.stderr')
    out.write_bytes(done.stdout)
    err.write_bytes(done.stderr)
    return dict(source=str(source), source_sha256=sha(source), n=n, command=command,
                cwd=str(SUITE), runtime_environment=ENV, returncode=done.returncode,
                bytes=len(done.stdout), sha256=sha(out), stdout_file=str(out),
                stderr_file=str(err), stderr_sha256=sha(err))


def instrument(source):
    """Keep numerical expressions; report every draw, division, CDF and carry."""
    lines = ['# Experimental!']
    context = None
    for line in source.read_text().splitlines():
        if line.startswith('Wrap '):
            lines.append('Wrap ← &p ⊂ "PACKED=" json Packed')
        elif line.startswith('Draw '):
            lines.extend(['Draw ← (', '  '+line.split('←', 1)[1].strip(),
                          '  &p ⊂ "DRAW=" json .', ')'])
        elif line.startswith('Chars '):
            lines.extend(['Chars ← (',
                          '  &p ⊂ "CDF_BYTES=" json ♭ bytes⌞ "f64" G.Cdf Chunk.State .',
                          '  &p ⊂ "RANDOM_BYTES=" json ♭ bytes⌞ "f64" ÷139968 Chunk.Draws .',
                          '  '+line.split('←', 1)[1].strip(), ')'])
        elif line.startswith('Generate ←'):
            lines.extend(['Generate ← (', '  &p ⊂ "START=" json .',
                          '  '+line.split('←', 1)[1].strip(), ')'])
        elif line in ('Advance ← (', 'AluAdvance ← ('):
            context = 'ALU' if line.startswith('Alu') else 'RNG'
            lines.append(line)
            if context == 'ALU':
                lines.append('  &p ⊂ "ALU_START=" json .')
        elif context and line.startswith(')'):
            lines.append('  &p ⊂ "'+('ALU_END=' if context == 'ALU' else 'STATE=')+'" json .')
            lines.append(line)
            context = None
        else:
            lines.append(line)
    assert context is None
    return '\n'.join(lines)+'\n'


def validate_trace(done, n, expected, ends):
    assert done.returncode == 0, done.stderr.decode()
    record = -1
    remaining = 0
    seed = 42
    alu_offset = 0
    pending = None
    output = []
    chunks = [0, 0, 0]
    states_checked = divisions_checked = cdf_checks = 0
    starts = []
    for line in done.stdout.decode().splitlines():
        if line in HEADERS:
            assert pending is None and (record < 0 or remaining == 0)
            record += 1
            assert line == HEADERS[record]
            remaining = (2, 3, 5)[record]*n
            output.append(line+'\n')
            continue
        tag, encoded = line.split('=', 1)
        value = json.loads(encoded)
        if tag == 'ALU_START':
            assert record == 0 and pending is None
            offset, left, letters, thresholds = value
            assert offset == alu_offset and left == remaining and letters == ALU and thresholds == []
            length = min(CAPACITY, remaining)
            assert length > 0
            pending = dict(length=length, packed=False,
                           expected=wrap(''.join(ALU[(offset+i) % len(ALU)] for i in range(length))))
        elif tag == 'ALU_END':
            assert record == 0 and pending and pending['packed']
            alu_offset += pending['length']
            remaining -= pending['length']
            offset, left, letters, thresholds = value
            assert offset == alu_offset and left == remaining and letters == ALU and thresholds == []
            pending = None
        elif tag == 'START':
            assert record in (1, 2) and pending is None
            incoming, left, letters, thresholds = value
            assert incoming == seed and left == remaining
            assert letters == LETTERS[record-1] and binary(thresholds) == binary(cdf(PROBABILITIES[record-1]))
            starts.append(incoming)
        elif tag == 'DRAW':
            assert record in (1, 2) and pending is None
            length = min(CAPACITY, remaining)
            draws = []
            for _ in range(length):
                seed = (3877*seed+29573) % 139968
                draws.append(seed)
            assert length > 0 and value == draws
            thresholds = cdf(PROBABILITIES[record-1])
            pending = dict(length=length, packed=False, cdf=False, divisions=False, draws=draws,
                           expected=wrap(classify(draws, LETTERS[record-1], thresholds)))
            states_checked += length
        elif tag == 'CDF_BYTES':
            assert pending and not pending['cdf']
            assert bytes(value) == binary(cdf(PROBABILITIES[record-1]))
            pending['cdf'] = True
            cdf_checks += 1
        elif tag == 'RANDOM_BYTES':
            assert pending and not pending['divisions']
            assert bytes(value) == binary([draw/139968.0 for draw in pending['draws']])
            pending['divisions'] = True
            divisions_checked += pending['length']
        elif tag == 'PACKED':
            assert pending and not pending['packed'] and value == pending['expected']
            if record:
                assert pending['cdf'] and pending['divisions']
            pending['packed'] = True
            output.append(value)
            chunks[record] += 1
        elif tag == 'STATE':
            assert record in (1, 2) and pending and pending['packed']
            remaining -= pending['length']
            outgoing, left, letters, thresholds = value
            assert outgoing == seed and left == remaining
            assert letters == LETTERS[record-1] and binary(thresholds) == binary(cdf(PROBABILITIES[record-1]))
            pending = None
        else:
            raise AssertionError(tag)
    assert record == 2 and remaining == 0 and pending is None and alu_offset == 2*n
    assert states_checked == divisions_checked == 8*n
    assert starts == [42, ends[0]] and seed == ends[1]
    assert ''.join(output).encode() == expected
    assert chunks == [(count*n+CAPACITY-1)//CAPACITY for count in (2, 3, 5)]
    return dict(actual_rng_draws=states_checked, expected_rng_draws=8*n,
                every_sequential_rng_state_verified=True, every_binary64_division_bitwise_verified=True,
                binary64_divisions=divisions_checked, cdf_vectors_bitwise_verified=cdf_checks,
                unscaled_sequential_binary64_cdf=True, record_initial_seeds=starts,
                record_final_seeds=ends, record_seed_carry_verified=True,
                alu_characters_verified=alu_offset, alu_global_phase_verified=True,
                chunk_counts=chunks, maximum_chunk_characters=CAPACITY,
                whole_output_reference_match=True, reference_match=True)


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    source_pins = {str(path): sha(path) for path in (SOURCE, ORIGINAL, COMPACT)}
    assert source_pins[str(ORIGINAL)] == 'f0dd51cda0e6dbae84afd0e95d7c4ceeb4aaac6d3892824a413926ab54c5cf05'
    assert source_pins[str(COMPACT)] == 'fba046bc5d20288547051f1fa56a80d52217edd614ed0efbbabe6ef16353a626'
    for path, digest in RUNTIME_META['pins'].items():
        assert sha(path) == digest, path
    reference_manifest = CLBG / 'd-comparison/20261011/sequence-manifest.json'
    d_row = next(row for row in json.loads(reference_manifest.read_text())['programs'] if row['id'] == 'fasta-d-compact')
    d_source, d_binary = Path(d_row['source']), Path(d_row['command'][0])
    assert sha(d_source) == d_row['source_sha256'] and sha(d_binary) == d_row['binary_sha256']
    trace = RESULTS / 'instrumented.ua'
    trace.write_text(instrument(SOURCE))
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
                  scope='Complete small outputs, all-draw/CDF/carry traces and generic wrapping only; no full n=25M timing.',
                  source_pins=source_pins, runtime=str(RUNTIME), runtime_sha256=sha(RUNTIME),
                  runtime_environment=ENV, shared_timing_lock=str(LOCK),
                  validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
                  reference_source=str(d_source), reference_source_sha256=sha(d_source),
                  reference_binary=str(d_binary), reference_binary_sha256=sha(d_binary),
                  reference_manifest=str(reference_manifest), reference_manifest_sha256=sha(reference_manifest),
                  cases=[], baseline_cases=[], instrumentation=[], primitive_cases=[], cdf_cases=[])
    print('Waiting for common mutex; no Uiua/D/reference probes before acquisition.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Locked bounded FASTA output validation.', flush=True)
        expected = {}
        ends = {}
        for n in CASES:
            golden, ends[n] = reference(n)
            expected[n] = golden
            d_command = [str(arg).replace('{n}', str(n)) for arg in d_row['command']]
            d_done = subprocess.run(d_command, cwd=SUITE, env=dict(os.environ, **ENV), capture_output=True, timeout=30)
            d_evidence = persist(d_source, n, d_command, d_done, 'reference-D-'+str(n))
            assert d_done.returncode == 0 and d_done.stdout == golden, (n, d_done.stderr.decode())
            command, done = execute(SOURCE, n)
            row = persist(SOURCE, n, command, done, 'output-'+str(n))
            assert done.returncode == 0 and done.stdout == golden, (n, done.stderr.decode(), done.stdout[:100], golden[:100])
            row.update(case='canonical-small' if n == 1000 else 'canonical-'+str(n), variant='fast',
                       reference_match=True, expected_sha256=hashlib.sha256(golden).hexdigest(),
                       independent_D_evidence=d_evidence, expected_alu_characters=2*n,
                       expected_rng_draws=8*n, record_final_seeds=ends[n], maximum_chunk_characters=CAPACITY)
            report['cases'].append(row)
            print(n, 'OUTPUT PASS', flush=True)
        for n in (1, 2, 61, 12001, 40003):
            command, done = execute(ORIGINAL, n)
            row = persist(ORIGINAL, n, command, done, 'original-output-'+str(n))
            assert done.returncode == 0 and done.stdout == expected[n], (n, done.stderr.decode())
            row.update(reference_match=True, original_preserved=True)
            report['baseline_cases'].append(row)
        for n in TRACE_CASES:
            command, done = execute(trace, n)
            row = persist(trace, n, command, done, 'work-'+str(n))
            row.update(validate_trace(done, n, expected[n], ends[n]))
            row.update(original_source=str(SOURCE), original_source_sha256=sha(SOURCE),
                       transformation='Replace raw chunk writes with serialization of the exact Packed characters; wrap unchanged Draw/Chars/Advance/Generate numerical expressions with diagnostics. Explicit float bytes preserve binary64 divisions/CDFs. Trace is never timed.')
            report['instrumentation'].append(row)
            print(n, 'ALL DRAW/CDF/CARRY/ALU WORK PASS', flush=True)
        packing_lines = [line for line in SOURCE.read_text().splitlines()
                         if line.startswith(('~Wrapped ', 'Indexed ', 'CharIndex ', 'LineEnd ', 'Packed ', 'Wrap '))]
        probe = RESULTS / 'wrap-only.ua'
        probe.write_text('\n'.join(packing_lines+['Wrap &fras "/dev/stdin"'])+'\n')
        for length in (0, 1, 59, 60, 61, 119, 120, 121, 65519, 65520, 65521, 131041):
            text = ''.join('aZ09B'[(i*7+3)%5] for i in range(length))
            command, done = execute(probe, data=text.encode())
            assert done.returncode == 0 and done.stdout == wrap(text).encode(), (length, done.stderr.decode())
            row = persist(probe, 0, command, done, 'wrap-'+str(length))
            inp = RESULTS / ('wrap-'+str(length)+'.input.txt')
            inp.write_bytes(text.encode())
            row.update(operation='generic-native-gather-LF-packing', input_file=str(inp), input_sha256=sha(inp),
                       input_characters=length, expected_lines=(length+59)//60,
                       one_raw_write=True, reference_match=True, benchmark_rng_draws=0)
            report['primitive_cases'].append(row)
        text = ('αβ🙂a'*16)[:61]
        command, done = execute(probe, data=text.encode())
        assert done.returncode == 0 and done.stdout == wrap(text).encode(), done.stderr.decode()
        row = persist(probe, 0, command, done, 'wrap-unicode-61')
        inp = RESULTS / 'wrap-unicode-61.input.txt'
        inp.write_bytes(text.encode())
        row.update(operation='generic-native-gather-LF-packing', input_file=str(inp), input_sha256=sha(inp),
                   input_characters=61, input_utf8_bytes=len(text.encode()), unicode_scalar_width=True,
                   expected_lines=2, one_raw_write=True, reference_match=True, benchmark_rng_draws=0)
        report['primitive_cases'].append(row)
        classifier_lines = [line for line in SOURCE.read_text().splitlines()
                            if line.startswith(('Iub ', 'Human ', '~G ', '~Chunk ', 'Chars '))]
        for name, letters, probabilities, cdf_name in zip(('IUB', 'Human'), LETTERS, PROBABILITIES, ('Iub', 'Human')):
            probe = RESULTS / ('all-CDF-'+name+'.ua')
            probe.write_text('\n'.join(classifier_lines+[
                '&w (Chars Chunk (⇡139968) (G 0 0 '+json.dumps(letters)+' '+cdf_name+')) 1'])+'\n')
            command, done = execute(probe)
            wanted = classify(range(139968), letters, cdf(probabilities)).encode()
            assert done.returncode == 0 and done.stdout == wanted, (name, done.stderr.decode())
            row = persist(probe, 0, command, done, 'all-CDF-'+name)
            row.update(operation='exhaustive-unscaled-binary64-CDF-classification', integer_draw_values=139968,
                       reference_match=True, all_direct_division_values=True,
                       validation_only=True, used_as_production_lookup_cache=False, benchmark_rng_draws=0)
            report['cdf_cases'].append(row)
        for path, digest in source_pins.items():
            assert sha(path) == digest, path
        assert sha(d_source) == report['reference_source_sha256'] and sha(d_binary) == report['reference_binary_sha256']
    report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                  passed=len(report['cases']), work_checks_passed=len(report['instrumentation']),
                  primitive_checks_passed=len(report['primitive_cases']), cdf_checks_passed=len(report['cdf_cases']),
                  baseline_checks_passed=len(report['baseline_cases']), full_performance_runs=0)
    (RESULTS / 'validation.json').write_text(json.dumps(report, indent=2, ensure_ascii=False)+'\n')
    print('PASS', report['passed'], 'outputs', report['work_checks_passed'], 'work traces',
          report['primitive_checks_passed'], 'generic wrappers', report['cdf_checks_passed'], 'exhaustive CDF checks', flush=True)


if __name__ == '__main__':
    main()
