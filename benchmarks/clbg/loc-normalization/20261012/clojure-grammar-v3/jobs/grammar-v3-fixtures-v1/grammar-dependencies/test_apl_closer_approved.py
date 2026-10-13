"""Exact six-source additive approval metadata fixtures; no source mutation."""
from copy import deepcopy
import unittest
from apl_closer_approved import APPROVED, apply_approved_closers


class ApprovedAPLClosers(unittest.TestCase):
    def pair(self, sha):
        prior, approved, lines = APPROVED[sha]
        increments = [dict(line=line, original='∇', extra_lines=1) for line in lines]
        metric = dict(language='apl', sha256=sha, prior_normalized_loc=prior,
                      proposed_normalized_loc=approved, normalized_loc=None,
                      prior_nonclosing_count_matches=True,
                      closing_line_increments=increments, diagnostics=[dict(kind='apl-review')],
                      provenance=dict(existing_pin='preserved'))
        receipt = dict(status='approved-exact-six-source-closer-reconciliation',
                       rule_version='canonical-source-units-20261012-approved-rule-v2',
                       sources=[dict(sha256=sha, producer_loc=prior,
                                     approved_canonical_loc=approved, normalized_loc=approved,
                                     prior_nonclosing_count_matches=True,
                                     closing_line_increments=deepcopy(increments))])
        return metric, receipt

    def apply(self, metric, receipt, pin='f'*64):
        return apply_approved_closers(metric, receipt, '/frozen/approval.json', pin)

    def test_each_exact_approved_source_retains_prior_metadata(self):
        for sha, (_, approved, lines) in APPROVED.items():
            metric, receipt = self.pair(sha)
            before = deepcopy(metric)
            row = self.apply(metric, receipt)
            self.assertEqual(row['normalized_loc'], approved)
            self.assertEqual(row['diagnostics'], [])
            self.assertEqual(row['provenance']['existing_pin'], 'preserved')
            self.assertEqual(row['provenance']['additive_closer_approval']['written_closer_lines'], list(lines))
            self.assertEqual(metric, before)

    def test_unknown_identity_language_and_unverified_receipt_are_rejected(self):
        sha = next(iter(APPROVED))
        for changed in ('identity', 'language', 'pin'):
            metric, receipt = self.pair(sha)
            if changed == 'identity': metric['sha256'] = '0'*64
            if changed == 'language': metric['language'] = 'uiua'
            self.assertIsNone(self.apply(metric, receipt, '' if changed == 'pin' else 'f'*64))

    def test_original_nonclosing_count_and_written_closer_lines_must_match(self):
        for target in ('prior', 'proposal', 'closer', 'nonclosing', 'authority'):
            metric, receipt = self.pair(next(iter(APPROVED)))
            if target == 'prior': metric['prior_normalized_loc'] += 1
            if target == 'proposal': metric['proposed_normalized_loc'] += 1
            if target == 'closer': receipt['sources'][0]['closing_line_increments'][0]['line'] += 1
            if target == 'nonclosing': metric['prior_nonclosing_count_matches'] = False
            if target == 'authority': receipt['status'] = 'proposal-only'
            self.assertIsNone(self.apply(metric, receipt))

    def test_duplicate_approval_rows_do_not_supply_a_count(self):
        metric, receipt = self.pair(next(iter(APPROVED)))
        receipt['sources'].append(deepcopy(receipt['sources'][0]))
        self.assertIsNone(self.apply(metric, receipt))


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
