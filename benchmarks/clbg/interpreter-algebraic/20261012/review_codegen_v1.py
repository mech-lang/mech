"""Locked extraction of saved FP experiment evidence; never approves performance.

Reads the completed v5 preparation only. No subprocess, native probe, compiler,
timing, production edit or chart admission. Missing/inlined row symbols are an
explicit manual-review limitation, never evidence of equivalent code generation.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import fcntl
import hashlib
from itertools import product
import json
import math
from pathlib import Path
import re
import struct
import sys
import traceback

HERE = Path(__file__).resolve().parent
PREPARATION = HERE / 'revisions/explicit-f64-v5/preparation/report.json'
EXPECTED_PROPOSAL = '447300b77705b3467ced2701af8de2b42073998c00e98b62d1dd12fe5539f3b6'
LOCK = Path('/tmp/mech-clbg-timing.lock')
MODES = ('strict', 'reduction', 'broad')
SIZES = (1, 2, 3, 7, 8, 9, 31, 32, 33, 65, 129)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, payload):
    with Path(path).open('x') as stream:
        json.dump(payload, stream, indent=2, allow_nan=False)
        stream.write('\n')


def float_of(bits):
    assert re.fullmatch('[0-9a-f]{16}', bits)
    return struct.unpack('>d', bytes.fromhex(bits))[0]


def category(bits):
    value = float_of(bits)
    if math.isnan(value): return 'nan'
    if math.isinf(value): return 'infinite'
    if value == 0: return 'negative-zero' if bits.startswith('8') else 'positive-zero'
    if abs(value) < sys.float_info.min: return 'subnormal'
    return 'normal'


def ordered(bits):
    value = int(bits, 16)
    return (~value & ((1 << 64) - 1)) if value >> 63 else value | (1 << 63)


def numeric_rows(path, mode):
    rows = {}
    for line in Path(path).read_text().splitlines():
        if not line.startswith(('FP_AXES|', 'FP_CONFORMANCE|')): continue
        fields = line.split('|')
        assert fields[1] == mode
        if fields[0] == 'FP_AXES':
            assert len(fields) == 9
            key, expected, actual = tuple([fields[0]] + fields[2:7]), fields[7], fields[8]
        else:
            assert len(fields) == 11
            key, expected, actual = tuple([fields[0]] + fields[2:7]), fields[7], fields[8]
            assert category(actual) == fields[9]
            ulp = str(abs(ordered(expected) - ordered(actual))) if all(
                math.isfinite(float_of(value)) for value in (expected, actual)) else 'nonfinite'
            assert fields[10] == ulp
        assert key not in rows
        float_of(expected); float_of(actual)
        rows[key] = (expected, actual)
    axes = {('FP_AXES', str(length), str(row), transpose, divide, tiled)
        for length, row, transpose, divide, tiled in product(
            (0, 1, 7, 31, 32, 33, 65), range(3), ('false', 'true'), ('false', 'true'), ('false', 'true'))}
    constants = {'3ff0000000000000', '0000000000000000', '8000000000000000',
                 '4008000000000000', '0010000000000000', '7ff0000000000000'}
    nan_constants = {key[3] for key in rows if key[0] == 'FP_CONFORMANCE' and category(key[3]) == 'nan'}
    assert len(nan_constants) == 1
    constants |= nan_constants
    conform = {('FP_CONFORMANCE', case, str(length), coefficient, divide, tiled)
        for case, length, coefficient, divide, tiled in product(
            ('positive', 'mixed-sign', 'cancellation', 'signed-zero', 'subnormal', 'overflow', 'nan', 'infinity'),
            (0, 1, 2, 7, 31, 32, 33, 65), constants, ('false', 'true'), ('false', 'true'))}
    assert set(rows) == axes | conform and len(rows) == 1960
    if mode == 'strict':
        assert all(expected == actual or category(expected) == category(actual) == 'nan'
                   for expected, actual in rows.values())
    return rows


def numerical_summary(rows, strict):
    assert rows.keys() == strict.keys()
    counts, changes = Counter(), []
    for key, (expected, actual) in rows.items():
        strict_expected, strict_actual = strict[key]
        assert expected == strict_expected or category(expected) == category(strict_expected) == 'nan'
        counts['total_rows'] += 1
        if actual == strict_actual: continue
        counts['changed_bits'] += 1
        counts['class:' + category(strict_actual) + '->' + category(actual)] += 1
        reference_value, value = float_of(strict_actual), float_of(actual)
        change = dict(key=key, strict_bits=strict_actual, actual_bits=actual,
                      strict_class=category(strict_actual), actual_class=category(actual))
        if math.isfinite(reference_value) and math.isfinite(value):
            absolute = abs(value - reference_value)
            relative = absolute / abs(reference_value) if reference_value != 0 else None
            # Extremely large finite differences can overflow the report scalar.
            change.update(ulp=abs(ordered(strict_actual) - ordered(actual)),
                absolute_error=absolute if math.isfinite(absolute) else 'overflow',
                relative_error=relative if relative is None or math.isfinite(relative) else 'overflow')
        changes.append(change)
    return dict(counts=dict(counts), changes=changes, algebraic_semantics_accepted=False,
                canonical_output_acceptance_is_separate=True)


def row_symbol(name):
    return 'scalar_row' in name or ('ScalarTile' in name and ('3row' in name or '::row' in name))


def ir_functions(path):
    # LLVM textual definitions have one unindented closing brace. Keep the
    # complete function; operand/call matches alone never become loop evidence.
    with Path(path).open() as stream:
        current, symbol = None, None
        for line in stream:
            if current is None and line.startswith('define '):
                match = re.search(r'@(?:"([^"]+)"|([^ (]+))\(', line)
                assert match, ('Cannot identify LLVM definition', str(path), line[:180])
                symbol = match[1] or match[2]
                current = [line]
            elif current is not None:
                current.append(line)
                if line.rstrip() == '}':
                    if row_symbol(symbol): yield symbol, ''.join(current)
                    current, symbol = None, None
        assert current is None, ('Incomplete LLVM definition', str(path))


def bind_receipt(path, expected_command, preparation, pins):
    """Newly pin saved producer receipts without inventing historical pins."""
    digest = sha(path)
    historical = preparation['pins'].get(str(path))
    if historical is not None: assert historical == digest
    pins[str(path)] = digest
    receipt = json.loads(path.read_text())
    assert receipt['command'] == expected_command
    assert receipt['exit_status'] == receipt['final_exit_status'] == 0
    assert receipt['inherited_common_mutex'] is True
    assert receipt['preparation_only'] is True and receipt['never_chart_eligible'] is True
    assert not any(receipt.get(key) for key in ('error', 'cleanup_error', 'cleanup_errors', 'remaining_group_members'))
    tool = Path(expected_command[0])
    assert sha(tool) == preparation['pins'][str(tool)]
    pins[str(tool)] = preparation['pins'][str(tool)]
    return dict(path=str(path), sha256=digest, command=expected_command,
        recorded_in_completed_preparation_pins=historical is not None,
        newly_captured_saved_receipt=historical is None)


def bind_codegen_receipts(row, preparation, pins):
    directory = Path(row['conformance_receipt']).parent.parent / 'codegen'
    saved = [(path, json.loads(path.read_text())) for path in sorted(directory.glob('ir-*.json'))
             if not path.name.endswith('.active.json')]
    records = []
    for filename in row['codegen_files']:
        if not filename.endswith('thin-lto-after-pm.ll'): continue
        llvm_ir = Path(filename); bitcode = llvm_ir.with_suffix('.bc')
        expected_tail = [str(bitcode), '-o', str(llvm_ir)]
        matches = [(path, receipt) for path, receipt in saved
                   if receipt.get('command', [])[1:] == expected_tail]
        assert len(matches) == 1, ('Missing/ambiguous actual IR producer', filename)
        path, receipt = matches[0]
        assert Path(receipt['command'][0]).name == 'llvm-dis'
        assert sha(bitcode) == preparation['pins'][str(bitcode)]
        pins[str(bitcode)] = preparation['pins'][str(bitcode)]
        records.append(bind_receipt(path, receipt['command'], preparation, pins))
    arm_receipt = directory / 'arm-disassembly.json'
    command = json.loads(arm_receipt.read_text())['command']
    assert Path(command[0]).name == 'llvm-objdump'
    assert command[1:] == ['--disassemble', '--demangle', '--no-show-raw-insn', row['binary']]
    records.append(bind_receipt(arm_receipt, command, preparation, pins))
    return records


def extract_codegen(row, output):
    records, pins = [], {}
    files = [Path(path) for path in row['codegen_files'] if path.endswith('thin-lto-after-pm.ll')]
    assert files and len(files) == len(set(files))
    destination = output / row['variant']; destination.mkdir()
    for path in files:
        pins[str(path)] = sha(path)
        for symbol, body in ir_functions(path):
            target = destination / ('row-' + str(len(records)).zfill(3) + '.ll')
            with target.open('x') as stream: stream.write(body)
            operations = Counter()
            for line in body.splitlines():
                match = re.search(r'\b(fadd|fmul|fdiv)\b', line)
                if match:
                    operation = match[1]
                    operations[operation] += 1
                    if re.search(r'<\d+ x double>', line): operations['vector_' + operation] += 1
                    if re.search(r'\b(fast|reassoc|nnan|ninf|nsz|arcp|contract|afn)\b', line):
                        operations['relaxed_' + operation] += 1
            records.append(dict(input=str(path), symbol=symbol, extracted=str(target),
                                operations=dict(operations), extracted_sha256=sha(target)))
    assembly = [Path(path) for path in row['codegen_files'] if path.endswith('arm-disassembly.stdout')]
    assert len(assembly) == 1
    pins[str(assembly[0])] = sha(assembly[0])
    # Preserve a bounded navigation index. Complete original assembly remains
    # pinned; context excerpts do not establish complete inlined row coverage.
    matches = []
    with assembly[0].open() as stream:
        for number, line in enumerate(stream, 1):
            if row_symbol(line): matches.append(dict(line=number, text=line.rstrip()))
    return dict(final_ir_functions=records, assembly=str(assembly[0]), assembly_matches=matches,
        missing_row_symbol=not bool(records), relevant_codegen_gain_proven=False,
        complete_inlined_coverage_proven=False, manual_IR_and_ARM_review_required=True), pins


def run(output):
    print('WAIT common mutex: saved FP numerical/code-generation evidence', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        observed, named = __import__('os').fstat(lock.fileno()), LOCK.stat()
        assert (observed.st_dev, observed.st_ino) == (named.st_dev, named.st_ino)
        assert not output.exists(), 'Preserve prior jobs; no implicit retry or overwrite'
        output.mkdir(parents=True)
        pins = {str(Path(__file__).resolve()): sha(__file__), str(PREPARATION): sha(PREPARATION),
                str(Path(sys.executable).resolve()): sha(Path(sys.executable).resolve())}
        report = dict(status='extracting-saved-evidence', started_at=datetime.now(timezone.utc).isoformat(),
            preparation=str(PREPARATION), common_mutex_held=True, common_mutex=str(LOCK),
            benchmark_processes_started=0, full_performance_runs=0, chart_admission=False,
            automatic_retry=False, performance_authorized=False, variants=[], pins=pins)
        try:
            preparation = json.loads(PREPARATION.read_text())
            assert preparation['status'] == 'bounded-three-copy-preparation-complete-pending-independent-review'
            assert preparation['proposal_sha256'] == EXPECTED_PROPOSAL and preparation['full_performance_runs'] == 0
            assert preparation['active'] is None and preparation['never_chart_eligible'] is True
            rows = preparation['variants']
            assert [row['variant'] for row in rows] == list(MODES)
            assert len(preparation['attempts']) == 33
            for mode in MODES:
                attempts = [row for row in preparation['attempts'] if row['id'] == 'mech-algebraic-' + mode]
                assert sorted(row['workload_n'] for row in attempts) == sorted(SIZES)
                assert all(row['qualified'] is True and row['validation']['reference_match'] is True and
                    row['preparation_only'] is True and row['full_performance_run'] is False for row in attempts)
            observed_rows = {}
            for row in rows:
                assert row['status'] == 'bounded-output-passed-pending-codegen-work-and-performance-review'
                inputs = [Path(row[key]) for key in ('binary', 'source_copy', 'conformance_binary',
                    'conformance_receipt', 'conformance_stdout', 'conformance_stderr')]
                inputs += [Path(path) for path in row['codegen_files'] if path.endswith('thin-lto-after-pm.ll') or path.endswith('arm-disassembly.stdout')]
                for path in inputs:
                    expected = preparation['pins'][str(path)]
                    assert sha(path) == expected, ('Saved evidence changed', str(path))
                    pins[str(path)] = expected
                assert pins[row['binary']] == row['binary_sha256']
                assert pins[row['conformance_binary']] == row['conformance_binary_sha256']
                assert row['conformance_command'] == [row['conformance_binary']]
                conformance = bind_receipt(Path(row['conformance_receipt']), row['conformance_command'], preparation, pins)
                probe_sources = [HERE / 'conformance.rs',
                    Path(row['source_copy']).parents[1] / 'bin/mech-algebraic-conformance.rs']
                for path in probe_sources:
                    assert sha(path) == preparation['pins'][str(path)]
                    pins[str(path)] = preparation['pins'][str(path)]
                producers = bind_codegen_receipts(row, preparation, pins)
                observed_rows[row['variant']] = numeric_rows(row['conformance_stdout'], row['variant'])
                codegen, scanned_pins = extract_codegen(row, output)
                assert all(pins[path] == digest for path, digest in scanned_pins.items())
                report['variants'].append(dict(variant=row['variant'], codegen=codegen,
                    conformance_producer=conformance, codegen_producers=producers,
                    actual_probe_sources=list(map(str, probe_sources))))
            for row in report['variants']:
                row['numerical_evidence'] = numerical_summary(observed_rows[row['variant']], observed_rows['strict'])
            for path, digest in pins.items(): assert sha(path) == digest, ('Evidence changed during review', path)
            report.update(status='evidence-extracted-awaiting-independent-loop-and-work-review',
                relevant_codegen_gain_proven=False, source_work_qualified=False,
                numeric_classification_is_not_algebraic_acceptance=True)
        except BaseException as error:
            report.update(status='failed-stop', error=str(error), error_type=type(error).__name__)
            with (output / 'error.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            report['finished_at'] = datetime.now(timezone.utc).isoformat()
            new_json(output / 'report.json', report)
            new_json(output / 'terminal-pins.json', {str(path): sha(path)
                for path in output.rglob('*') if path.is_file()})
        print(json.dumps(dict(status=report['status'], variants=3, numerical_rows_per_variant=1960,
            performance_authorized=False, report=str(output / 'report.json'))), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.output.resolve())
