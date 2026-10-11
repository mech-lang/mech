"""Recheck statement LOC against pinned sources without changing measured programs."""
import json
import tempfile
from pathlib import Path
from build import loc
from prepare import sha

D = Path(__file__).resolve().parent


def main():
    # Clause/block sequencing is counted; data/arguments and comments are not.
    cases = [('f(X) -> Y = X, Z = X, Y + Z.', 3),
             ('f(X) -> {X, X, X}.', 1),
             ('f(X) -> g(X, X, X).', 1),
             ('f(X) -> [X, X, X]. % ignored, comment', 1),
             ('f(X) -> begin Y = X, Y end.', 2)]
    with tempfile.TemporaryDirectory(prefix='erlang-loc-audit-') as scratch:
        source = Path(scratch) / 'loc_fixture.erl'
        for body, expected in cases:
            source.write_text('-module(loc_fixture).\n-export([f/1]).\n' + body + '\n')
            assert loc(source)['normalized_loc'] == expected + 2, body
    report_path = D / 'results/timings.json'
    report = json.loads(report_path.read_text())
    assert report['status'] == 'complete'
    rows = {}
    for row in report['programs'].values():
        if row['language'] != 'erlang' or not row['eligible_for_chart']:
            continue
        source = Path(row['source'])
        assert sha(source) == row['source_sha256']
        metric = loc(source)
        assert metric['normalized_loc'] == row['loc']
        rows[row['key']] = dict(source_sha256=row['source_sha256'], initial_normalized_loc=row['loc'], **metric)
    assert len(rows) == 21
    output = dict(status='passed', rows=rows, timing_report_sha256=sha(report_path), source_audit_sha256=sha(D / 'source-audit.json'),
        audit_script_sha256=sha(Path(__file__)), loc_tool_sha256=sha(D / 'support/loc_probe.erl'),
        policy='Nonblank/noncomment physical LOC including semantic directives and every application helper. Packed independent expressions in clause/block bodies count separately; tuple/list/argument/guard separators do not. Source bytes and timings are unchanged.')
    (D / 'results/final-loc-audit.json').write_text(json.dumps(output, indent=2) + '\n')
    print('Final statement LOC verified for all 21 measured Erlang sources')


if __name__ == '__main__':
    main()
