"""Additive overlay for root-approved six exact APL source closer counts.

No hashes are performed here. Caller first verifies original source/provenance
pins and the supplied approval receipt pin under the common timing mutex.
"""
from copy import deepcopy


APPROVED = {
    '138dac87e5b26e8fea7a1b637468d79225a89685ee98c08b3fc4f238f86c7d61': (27,29,(16,27)),
    '34f1a097f3da19cf7b4435812c5f0256c21b6f0ae0c1982386361d114958f237': (29,31,(18,29)),
    '6a691668ec0835a4930cdc550dfceb5f04bbfbce08368e47834e26dd5be9ab76': (55,57,(12,55)),
    'bfcfeb21c2753bb6217b26e2db1cff17eed762d9ade7d27077d523e2b27401d8': (33,35,(13,33)),
    '611f8f040cd5893993d5d43830ff4c7e74c0c3bf1cc6b8f4832f616de7400d38': (51,53,(22,51)),
    'a4fcaa32c69f215bc07a9d1794bd72ceb764bd47059dfd4ec9490b0a8ab4d991': (71,75,(25,29,63,73)),
}


def apply_approved_closers(metric,approval_receipt,receipt_path,verified_receipt_sha256):
    """Return an approved copy or None; do not mutate prior metrics/records."""
    if not metric or metric.get('language')!='apl':return None
    sha=metric.get('sha256');expected=APPROVED.get(sha)
    if not expected or not verified_receipt_sha256 or len(verified_receipt_sha256)!=64:return None
    if (approval_receipt.get('status')!='approved-exact-six-source-closer-reconciliation' or
            approval_receipt.get('rule_version')!='canonical-source-units-20261012-approved-rule-v2'):
        return None
    rows=[r for r in approval_receipt.get('sources',[]) if r.get('sha256')==sha]
    if len(rows)!=1:return None
    prior,approved,lines=expected;row=rows[0]
    metric_lines=tuple(r['line'] for r in metric.get('closing_line_increments',[]))
    receipt_lines=tuple(r['line'] for r in row.get('closing_line_increments',[]))
    if (metric.get('prior_nonclosing_count_matches') is not True or
            metric.get('prior_normalized_loc')!=prior or
            metric.get('proposed_normalized_loc')!=approved or metric_lines!=lines or
            row.get('producer_loc')!=prior or row.get('approved_canonical_loc')!=approved or
            row.get('normalized_loc')!=approved or receipt_lines!=lines or
            row.get('prior_nonclosing_count_matches') is not True):
        return None
    result=deepcopy(metric)
    result.update(normalized_loc=approved,review_status='six-source-closer-reconciliation-approved',
        diagnostics=[],rule_version='canonical-source-units-20261012-approved-rule-v2')
    result['provenance']['additive_closer_approval']=dict(receipt=str(receipt_path),
        verified_receipt_sha256=verified_receipt_sha256,source_sha256=sha,
        old_producer_loc=prior,approved_canonical_loc=approved,
        written_closer_lines=list(lines),scope='Six exact source closer reconciliation only; no generic APL certification.')
    return result
