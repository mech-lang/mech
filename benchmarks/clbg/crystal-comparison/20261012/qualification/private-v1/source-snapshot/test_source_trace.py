"""Twelve authored unexecuted Crystal work-trace admission/rejection fixtures."""
from collections import Counter
import fcntl
import io
import json
from pathlib import Path
import unittest

import guarded_prepare as preparation
import source_trace as trace

FD = None
WIDTHS = (1, 2, 3, 4, 6, 12, 18)
SEQUENCE = 'GGTATTTTAATTTATAGTACGT'
DATA = ('>THREEBAD\r\nAAAA\r\n;>THREE\r\n>THREE\tselected\r\n' + SEQUENCE.lower() + '\r\n>FOUR\r\nAAAA').encode()
PI_CANDIDATES = ((3, 4, 1, None), (9, 10, 3, 3), (0, 10, 3, None), (10, 30, 15, None), (110, 170, 105, 1))


def maps(variant):
    tables = [Counter(SEQUENCE[i:i + k] for i in range(max(0, len(SEQUENCE) - k + 1))) for k in WIDTHS]
    lines = [f'TRACE_C1|knucleotide|{variant}|7|{len(SEQUENCE)}']
    for k, table in zip(WIDTHS, tables):
        for word, count in sorted(table.items()):
            key = word if variant == 'compact' else str(sum('ACGT'.index(base) << (2 * (k - i - 1)) for i, base in enumerate(word)))
            lines.append(f'MAP|{k}|{key}|{count}')
        lines.append(f'TABLE|{k}|{len(table)}|{sum(table.values())}')
    lines.append('END_C1')
    return ('\n'.join(lines) + '\n').encode(), trace.independent()._nucleotide_output(tables)


def pi(variant):
    lines = [f'TRACE_C1|pidigits|{variant}|2']
    count = 0
    roles = ('lower', 'upper') if variant == 'compact' else ('upper', 'lower')
    for candidate, (lower, upper, denominator, accepted) in enumerate(PI_CANDIDATES, 1):
        values = dict(lower=lower, upper=upper)
        for role in roles:
            numerator = values[role]
            lines.append(f'DIV|{candidate}|{role}|{numerator}|{denominator}|{numerator // denominator}')
        if accepted is not None:
            count += 1
            lines.append(f'ACCEPT|{candidate}|{accepted}|{count}')
    lines.append('END_C1')
    return ('\n'.join(lines) + '\n').encode()


def forest(variant='compact', n=0):
    lines = [f'TRACE_C1|binarytrees|{variant}|{n}']
    next_id = 1

    def tree(depth):
        nonlocal next_id
        left, right = (tree(depth - 1), tree(depth - 1)) if depth else (0, 0)
        identity = next_id
        next_id += 1
        lines.append(f'NODE|{identity}|{left}|{right}')
        return identity

    maximum = max(6, n)
    root = tree(maximum + 1)
    lines.append(f'STRETCH|{root}|{maximum + 1}|{(1 << (maximum + 2)) - 1}')
    long_lived = tree(maximum)
    lines.append(f'LONG_START|{long_lived}|{maximum}')
    for depth in range(4, maximum + 1, 2):
        for index in range(1 << (maximum - depth + 4)):
            root = tree(depth)
            lines.append(f'TREE|{depth}|{index}|{root}|{(1 << (depth + 1)) - 1}')
    lines.append(f'LONG_END|{long_lived}|{maximum}|{(1 << (maximum + 1)) - 1}')
    lines.append('END_C1')
    references = preparation.load_module('crystal_fixture_tree_reference', preparation.V / 'references.py')
    return ('\n'.join(lines) + '\n').encode(), references.binarytrees(n)


class WorkChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Use the locked standalone fixture entry point')

    def check_maps(self, variant='compact', payload=None):
        raw, stdout = maps(variant)
        return trace.reconcile_maps(payload or raw, DATA, stdout, variant, common_timing_mutex_fd=FD)

    def check_pi(self, variant='compact', payload=None):
        return trace.reconcile_pi(payload or pi(variant), b'31        \t:2\n', 2, variant, common_timing_mutex_fd=FD)

    def check_trees(self, payload=None):
        raw, stdout = forest()
        return trace.reconcile_trees(payload or raw, stdout, 0, 'compact', common_timing_mutex_fd=FD)

    def test_all_six_actual_source_anchors(self):
        for benchmark in ('knucleotide', 'pidigits', 'binarytrees'):
            for variant in ('compact', 'optimized'):
                with self.subTest(benchmark=benchmark, variant=variant):
                    original = (preparation.HERE / 'sources' / benchmark / (variant + '.cr')).read_text()
                    amended, plan = trace.instrument_text(original, benchmark, variant)
                    self.assertIn('TRACE_C1', amended)
                    self.assertTrue(plan['never_chart_eligible'])
                    if benchmark != 'binarytrees':
                        position = 0
                        for line in original.splitlines():
                            if line:
                                position = amended.index(line, position) + len(line)

    def test_all_seven_actual_maps_both_forms(self):
        for variant in ('compact', 'optimized'):
            self.assertTrue(self.check_maps(variant)['all_seven_maps_reconciled'])

    def test_wrong_map_count_rejected(self):
        payload, _ = maps('compact')
        with self.assertRaises(AssertionError):
            self.check_maps(payload=payload.replace(b'MAP|1|A|', b'MAP|1|X|', 1))

    def test_missing_map_rejected(self):
        payload, _ = maps('compact')
        lines = payload.splitlines()
        lines = [line for line in lines if not line.startswith(b'TABLE|18|')]
        with self.assertRaises(AssertionError):
            self.check_maps(payload=b'\n'.join(lines) + b'\n')

    def test_unused_packed_high_bits_rejected(self):
        payload, _ = maps('optimized')
        with self.assertRaises(AssertionError):
            self.check_maps('optimized', payload.replace(b'MAP|1|0|', b'MAP|1|4|', 1))

    def test_both_actual_pi_divisions_both_forms(self):
        for variant in ('compact', 'optimized'):
            self.assertEqual(self.check_pi(variant)['actual_divisions'], 10)

    def test_wrong_second_pi_quotient_rejected(self):
        with self.assertRaises(AssertionError):
            self.check_pi(payload=pi('compact').replace(b'DIV|1|upper|4|1|4', b'DIV|1|upper|4|1|3'))

    def test_missing_second_pi_division_rejected(self):
        raw = pi('compact').replace(b'DIV|1|upper|4|1|4\n', b'')
        with self.assertRaises(AssertionError):
            self.check_pi(payload=raw)

    def test_exact_parser_comment_crlf_eof(self):
        helper = trace.independent()
        data = b'>THREE\vwrong\rAAAA\r;>THREE\r>THREE\tselected\rACGT\r;>FOUR\rGGT'
        self.assertEqual(helper._sequence(data), 'ACGTGGT')

    def test_actual_complete_distinct_tree_forest(self):
        self.assertTrue(self.check_trees()['distinct_leaves'])
        self.assertTrue(self.check_trees()['long_lived_root_retained'])

    def test_shared_leaf_and_missing_temporary_root_rejected(self):
        raw, _ = forest()
        with self.assertRaises(AssertionError):
            self.check_trees(raw.replace(b'NODE|3|1|2', b'NODE|3|1|1', 1))
        lines = [line for line in raw.splitlines() if not line.startswith(b'TREE|4|0|')]
        with self.assertRaises(AssertionError):
            self.check_trees(b'\n'.join(lines) + b'\n')

    def test_changed_long_lived_root_and_incomplete_trace_rejected(self):
        raw, _ = forest()
        line = next(line for line in raw.splitlines() if line.startswith(b'LONG_END|'))
        fields = line.split(b'|')
        fields[1] = b'1'
        with self.assertRaises(AssertionError):
            self.check_trees(raw.replace(line, b'|'.join(fields)))
        with self.assertRaises(AssertionError):
            self.check_trees(raw.removesuffix(b'END_C1\n'))


def main():
    global FD
    print('WAIT LOCK: Crystal work-trace fixtures', flush=True)
    with preparation.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        FD = lock.fileno()
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(WorkChecks))
        text = output.getvalue()
        print(text, end='', flush=True)
        log = preparation.HERE / 'work-trace-fixtures.log'
        assert not log.exists(), 'Never overwrite a fixture attempt'
        with log.open('x') as stream:
            stream.write(text)
        receipt = dict(status='passed' if result.wasSuccessful() else 'failed', tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors), native_probes=0,
            actual_signals_sent=0, never_chart_eligible=True, shared_timing_lock=str(preparation.LOCK),
            pins={str(p): preparation.sha(p) for p in (Path(__file__).resolve(),
                preparation.HERE / 'guarded_prepare.py', preparation.HERE / 'source_trace.py',
                preparation.V / 'source_trace.py', preparation.V / 'references.py', log)})
        preparation.new_json(preparation.HERE / 'work-trace-fixtures.json', receipt)
        if not result.wasSuccessful() or result.testsRun != 12:
            raise SystemExit(1)


if __name__ == '__main__':
    main()
