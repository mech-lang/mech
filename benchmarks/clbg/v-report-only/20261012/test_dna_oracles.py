"""Literal parser/map checks for the additive V qualification oracle."""
import fcntl
from pathlib import Path
import unittest

import dna_oracles as oracle
import full_study
import references

HERE = Path(__file__).resolve().parent


class ExactTokenOracleChecks(unittest.TestCase):
    def test_lookalike_target_is_skipped(self):
        self.assertEqual(oracle.target_sequence(b'>THREEBAD\nCCCC\n>THREE target\nacgt\n'), 'ACGT')

    def test_vertical_tab_is_neither_line_break_nor_token_separator(self):
        self.assertEqual(oracle.target_sequence(b'>THREE\vnot-token\nCCCC\n>THREE\tvalid\nACGT\n'), 'ACGT')

    def test_carriage_return_and_linefeed_are_real_separators(self):
        self.assertEqual(oracle.target_sequence(b'>ONE\rGGG\r>THREE\rac\r\ngt\n>FOUR\rAAAA'), 'ACGT')

    def test_comments_cannot_change_record_selection(self):
        data = b'>ONE\n;>THREE\nGGG\n>THREE\nAC\n;>FOUR\nGT\n;comment-at-eof'
        self.assertEqual(oracle.target_sequence(data), 'ACGT')

    def test_empty_and_unterminated_headers(self):
        for data in (b'', b'>THREE', b'>THREE\n>FOUR\nACGT', b'>ONE\nACGT'):
            with self.subTest(data=data):
                self.assertEqual(oracle.target_sequence(data), '')
                self.assertEqual(oracle.knucleotide(data), b'\n\n' + ''.join(f'0\t{word}\n' for word in oracle.QUERIES).encode())

    def test_all_seven_window_totals_and_required_queries(self):
        sequence, maps = oracle.nucleotide_maps(b'>THREE\nGGTATTTTAATTTATAGT')
        self.assertEqual(sequence, 'GGTATTTTAATTTATAGT')
        self.assertEqual([sum(table.values()) for table in maps], [18, 17, 16, 15, 13, 7, 1])
        self.assertEqual([table[word] for table, word in zip(maps[2:], oracle.QUERIES)], [1] * 5)

    def test_literal_frequency_order_and_one_base_tail(self):
        suffix = ''.join(f'0\t{word}\n' for word in oracle.QUERIES).encode()
        self.assertEqual(oracle.knucleotide(b'>THREE\nACGT'),
                         b'A 25.000\nC 25.000\nG 25.000\nT 25.000\n\nAC 33.333\nCG 33.333\nGT 33.333\n\n' + suffix)
        self.assertEqual(oracle.knucleotide(b'>THREE\nA'), b'A 100.000\n\n\n' + suffix)

    def test_existing_frozen_fixture_outputs_remain_equal(self):
        for name, data in references.dna_fixtures():
            with self.subTest(case=name):
                self.assertEqual(oracle.knucleotide(data), references.knucleotide(data))


def main():
    module = full_study.configure('knucleotide')
    print('WAIT common mutex: additive DNA oracle fixtures', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(ExactTokenOracleChecks))
        destination = HERE / 'dna-oracle-fixtures.json'
        assert not destination.exists(), 'Never overwrite an actual fixture receipt'
        module.save(destination, dict(status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun, failures=len(result.failures), errors=len(result.errors),
            created_at=module.now(), report_only=True, never_chart_eligible=True,
            full_performance_runs=0, shared_timing_lock=str(module.LOCK),
            pins={str(path): module.sha(path) for path in (
                Path(__file__).resolve(), HERE / 'dna_oracles.py', HERE / 'references.py',
                HERE / 'full_study.py', full_study.HELPER)}))
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
