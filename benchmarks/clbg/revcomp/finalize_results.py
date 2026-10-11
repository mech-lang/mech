#!/usr/bin/env python3
"""Preserve the two measured batches and assemble their explicitly labeled chart."""
import copy
import json
import os
from datetime import datetime, timezone

import run


def main():
    directory = run.HERE / 'results'
    cohort_name = 'apple-m3-precise-25000000.json'
    paired_name = 'paired-optimizations.json'
    report = json.loads((directory / cohort_name).read_text())
    paired = json.loads((directory / paired_name).read_text())
    old_snapshot = copy.deepcopy(report['compiler_snapshot'])
    replacement = {row['id']: row for row in paired['programs']}
    small, _, expected, _, _ = run.references()
    for row in report['programs']:
        row['measurement_batch'] = cohort_name
        row['sample_count'] = len(row['samples_seconds'])
        if row['id'] not in replacement:
            continue
        measured = copy.deepcopy(replacement[row['id']])
        assert row['source']['sha256'] == measured['source']['sha256']
        row.update(measured)
        # The current executable must independently pass the official small case.
        _, digest, size, _ = run.execute(row['command'], os.environ | row['runtime_environment'], small, 30)
        assert digest == expected
        row['correctness_1000'] = dict(exact_reference_match=True, sha256=digest, bytes=size)
        row['correctness_full']['all_samples_match'] = paired['all_full_outputs_passed']
        row['measurement_batch'] = paired_name
        row['sample_count'] = len(row['samples_seconds'])
        row['min_seconds'] = min(row['samples_seconds'])
        row['max_seconds'] = max(row['samples_seconds'])
    report['created_at'] = datetime.now(timezone.utc).isoformat()
    report['compiler_snapshot'] = paired['compiler_snapshot']
    report['measurement_batches'] = {
        cohort_name: {'samples_per_build': 5, 'compiler_snapshot': old_snapshot},
        paired_name: {'samples_per_build': 7, 'compiler_snapshot': paired['compiler_snapshot']},
    }
    report['source_reports'] = [cohort_name, paired_name]
    report['full_source_coverage_report'] = 'apple-m3-25000000.json'
    report['protocol']['samples'] = '5–7'
    report['protocol']['sample_order_report'] = cohort_name
    del report['protocol']['sample_order']
    report['protocol']['batch_description'] = (
        'Two independent sequential shuffled batches: 42 competitive builds with five samples, '
        'then six current Mech policies and fastest C/Rust controls with seven samples. '
        'Per-row measurement_batch and sample_count identify provenance; this is not one combined run. '
        'The second batch includes an encoding-domain metadata correction, which does not change '
        'the revcomp native transform. Four dominated C++ builds were measured in the preserved '
        'full-coverage report using the older polling wait, and are omitted here. '
        'That historical report establishes source coverage, not precise timing.'
    )
    report['endpoints'] = run.common.endpoint_summary(report['programs'])
    report['nonheroic_endpoints'] = run.common.endpoint_summary(
        [row for row in report['programs'] if not row['upstream_starred']])
    target = directory / 'apple-m3-final-25000000.json'
    target.write_text(json.dumps(report, indent=2) + '\n')
    for language, endpoint in report['endpoints']['languages'].items():
        row = next(row for row in report['programs'] if row['id'] == endpoint['fastest'])
        print(f"{language}: {row['median_seconds']:.6f}s; "
              f"{row['source']['nonblank_noncomment_lines']} LOC; {row['source']['gzip1_bytes']} gzip bytes")


if __name__ == '__main__':
    main()
