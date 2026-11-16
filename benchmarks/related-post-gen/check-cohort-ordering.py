#!/usr/bin/env python3
"""Check cohort permutations and reproducibility without running benchmarks."""
from importlib.machinery import SourceFileLoader
from pathlib import Path
import random
import unittest

comparison = SourceFileLoader(
    'related_comparison_ordering',
    str(Path(__file__).with_name('compare-implementations.py')),
).load_module()


class CohortOrdering(unittest.TestCase):
    def setUp(self):
        self.variants = [dict(name=f'variant-{i}') for i in range(45)]

    def test_default_preserves_existing_alternating_order(self):
        orders = comparison.cohort_orders(self.variants, 15)
        for sample, order in enumerate(orders):
            expected = self.variants[::1 if sample % 2 == 0 else -1]
            self.assertEqual(order, expected)

    def test_every_randomized_round_contains_each_variant_once(self):
        orders = comparison.cohort_orders(self.variants, 31, 'randomized', 990)
        self.assertEqual(len(orders), 31)
        expected = sorted(v['name'] for v in self.variants)
        for order in orders:
            self.assertEqual(sorted(v['name'] for v in order), expected)
        self.assertGreater(len({tuple(v['name'] for v in order) for order in orders}), 1)

    def test_seed_is_reproducible_and_other_seeds_change_the_schedule(self):
        first = comparison.cohort_orders(self.variants, 15, 'randomized', 0)
        self.assertEqual(first, comparison.cohort_orders(self.variants, 15, 'randomized', 0))
        self.assertNotEqual(first, comparison.cohort_orders(self.variants, 15, 'randomized', 1))

    def test_generator_does_not_mutate_input_or_global_random_state(self):
        before = list(self.variants)
        state = random.getstate()
        comparison.cohort_orders(self.variants, 15, 'randomized', 20261106)
        self.assertEqual(self.variants, before)
        self.assertEqual(random.getstate(), state)

    def test_ambiguous_or_missing_ordering_settings_are_rejected(self):
        for mode, seed in [('randomized', None), ('alternating', 0), ('unknown', 123)]:
            with self.assertRaises(ValueError):
                comparison.cohort_orders(self.variants, 1, mode, seed)


if __name__ == '__main__':
    unittest.main()
