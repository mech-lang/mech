"""Recheck reader-aware normalized LOC without changing measured programs."""
import argparse
import json
from pathlib import Path
from loc import normalize as initial_normalize
from loc_final import normalize
from prepare import sha

D = Path(__file__).resolve().parent


def fixtures():
    cases = [
        ('racket', '(define (f x) (g x) (h x))', 2),
        ('lisp', '(setq a 1 b 2)', 2),
        ('lisp', '(do ((a 0 (1+ a)) (b 1 (1+ b)))\n  ((done a) b)\n  (f a b))', 4),
        ('lisp', '(let ((a 0) (b 1))\n  (+ a b))', 3),
        ('racket', '#lang racket/base\n#;(struct hidden (a b))\n(define x "; not a comment")', 2),
        ('lisp', '(f a b c)', 1),
        ('lisp', "(define x '(a b c))", 1),
        ('lisp', '(f) (g)', 2),
        ('lisp', '(when x (f) (g))', 2),
        ('racket', '(define xs #(1 2 3))', 1),
        ('lisp', '(format t "hello")', 1),
        ('racket', '(loop n (f) (g))', 1),
        ('racket', '(gmp _x = _y * 2, _r = _r + _x, get_r)', 3),
        ('racket', '(define-syntax-rule (for/par ([i range] [j range2]) body ...)\n  (for ([i range]) body ...))', 2),
        ('racket', '(cond [x => (lambda (y) (f y))])', 1),
        ('lisp', "#'(lambda (x) (f x) (g x))", 2),
        ('lisp', '(defun f (x y) (+ x y)) ; trailing comment', 1),
        ('lisp', '#| a nested #| comment |# here |#\n(define x #\\;)', 1),
    ]
    for language, source, expected in cases:
        result = normalize(source, language)
        assert result['normalized_loc'] == expected, (language, source, expected, result)
    return len(cases)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixtures-only', action='store_true')
    args = parser.parse_args()
    count = fixtures()
    if args.fixtures_only:
        print(f'{count} reader/statement fixtures passed')
        return
    report_path = D / 'results/timings.json'
    report = json.loads(report_path.read_text())
    assert report['status'] == 'complete'
    rows = {}
    for row in report['programs'].values():
        if row['language'] not in ('lisp', 'racket') or not row['eligible_for_chart']:
            continue
        source = Path(row['source'])
        assert sha(source) == row['source_sha256']
        assert initial_normalize(source.read_text(), row['language'])['normalized_loc'] == row['loc'], row['key']
        metric = normalize(source.read_text(), row['language'])
        rows[row['key']] = dict(source_sha256=row['source_sha256'], initial_normalized_loc=row['loc'], **metric)
    assert len(rows) == 59
    output = dict(status='passed', rows=rows, fixtures_passed=count,
                  timing_report_sha256=sha(report_path), source_audit_sha256=sha(D / 'source-audit.json'),
                  audit_script_sha256=sha(Path(__file__)), loc_tool_sha256=sha(D / 'loc_final.py'),
                  initial_loc_tool_sha256=sha(D / 'loc.py'),
                  corrections=[dict(key=key, before=row['initial_normalized_loc'], after=row['normalized_loc'],
                                    reason=('Packed independent Common Lisp DO initializers count separately, like LET initializers.'
                                            if row['normalized_loc'] > row['initial_normalized_loc'] else
                                            'Racket syntax-template ellipses are structural repetition markers, not separate body expressions.'))
                               for key,row in rows.items() if row['initial_normalized_loc'] != row['normalized_loc']],
                  policy='Every application helper, macro, declaration and FFI binding is included. Packed independent body expressions, initialized bindings and assignments are expanded; structural parameters, literal data and argument separators are not. Reader comments are removed. Original sources, binaries and timings are unchanged.')
    (D / 'results/final-loc-audit.json').write_text(json.dumps(output, indent=2) + '\n')
    print('Final statement LOC verified for all 59 measured Lisp/Racket sources')


if __name__ == '__main__':
    main()
