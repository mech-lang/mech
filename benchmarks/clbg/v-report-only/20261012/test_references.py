"""Qualify bounded independent oracles against retained benchmark references."""
import fcntl
import io
from pathlib import Path
import unittest

import full_study
import references

HERE = Path(__file__).resolve().parent
HARNESS = None


class ReferenceChecks(unittest.TestCase):
    def canonical(self, benchmark):
        n = HARNESS.WORK[benchmark][0]
        if benchmark in HARNESS.INPUTS:
            argument = HARNESS.INPUTS[benchmark][0].read_bytes()
        else:
            argument = n
        actual = getattr(references, benchmark)(argument)
        result = HARNESS.validate(benchmark, False, io.BytesIO(actual))
        self.assertTrue(result['reference_match'])

    def test_canonical_nbody(self):
        self.canonical('nbody')

    def test_canonical_mandelbrot(self):
        self.canonical('mandelbrot')

    def test_canonical_fannkuch(self):
        self.canonical('fannkuchredux')

    def test_canonical_fasta(self):
        self.canonical('fasta')

    def test_canonical_trees(self):
        self.canonical('binarytrees')

    def test_canonical_nucleotides(self):
        self.canonical('knucleotide')

    def test_canonical_reverse_complement(self):
        self.canonical('revcomp')

    def test_canonical_regex(self):
        self.canonical('regexredux')

    def test_pi_partial_and_zero(self):
        self.assertEqual(references.pidigits(0), b'')
        self.assertEqual(references.pidigits(27),
                         b'3141592653\t:10\n5897932384\t:20\n6264338   \t:27\n')

    def test_spectral_scalar(self):
        self.assertEqual(references.spectralnorm(1), b'1.000000000\n')

    def test_signed_fannkuch_two(self):
        self.assertEqual(references.fannkuchredux(2), b'-1\nPfannkuchen(2) = 1\n')

    def test_nucleotide_maps_include_every_width(self):
        for _, data in references.dna_fixtures():
            sequence, maps = references.nucleotide_maps(data)
            self.assertEqual(len(maps), 7)
            for width, counts in zip(references.WIDTHS, maps):
                self.assertEqual(sum(counts.values()), max(0, len(sequence) - width + 1))

    def test_mandelbrot_odd_width_tail(self):
        n = 9
        actual = references.mandelbrot(n)
        header = b'P4\n9 9\n'
        self.assertTrue(actual.startswith(header))
        bitmap = actual[len(header):]
        self.assertEqual(len(bitmap), 18)
        self.assertTrue(all(byte & 0x7f == 0 for byte in bitmap[1::2]))

    def test_reverse_complement_all_ambiguity_symbols(self):
        data = b'>one\r\nACGTUMRWSYKVHDBNacgtumrwsykvhdbn\r\n;ignored\r\n>empty\n'
        actual = references.revcomp(data)
        self.assertEqual(actual, b'>one\nNVHDBMRSWYKAACGTNVHDBMRSWYKAACGT\n>empty\n')


def main():
    global HARNESS
    module = full_study.configure('nbody')
    print('WAIT LOCK: V bounded-oracle fixtures', flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('LOCKED: V bounded-oracle fixtures', flush=True)
        HARNESS = module.load_harness()
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ReferenceChecks)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        receipt = dict(
            status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun,
            failures=len(result.failures), errors=len(result.errors),
            full_performance_runs=0, report_only=True,
            never_chart_eligible=True, shared_timing_lock=str(module.LOCK),
            created_at=module.now(),
            pins={str(path): module.sha(path) for path in (
                Path(__file__).resolve(), HERE / 'references.py', HERE / 'full_study.py',
                full_study.HELPER, full_study.SUITE / 'run.py',
            )},
        )
        destination = HERE / 'oracle-fixtures.json'
        assert not destination.exists(), 'Never overwrite a retained fixture receipt'
        module.save(destination, receipt)
        if not result.wasSuccessful():
            raise SystemExit(1)


if __name__ == '__main__':
    main()
