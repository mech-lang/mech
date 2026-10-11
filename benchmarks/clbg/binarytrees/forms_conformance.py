#!/usr/bin/env python3
"""Check canonical matches and fused comprehension sums on non-tree programs."""
import json
import conformance as c
import run


def passed(name, source, expected):
    actual = c.output(c.build(name, source))
    assert actual == expected, (name, actual, expected)
    c.checks.append([name, 'byte-exact'])
    print('Passed', name, flush=True)


def printed(expression):
    return 'result := (' + expression + ')\nio/print("{}\\n", result)\n'


TRACE = '''Trace(n<u64>) = (result<u64>) :=
  printed := io/print("trace={}\\n", n)
  result := n.
'''


def main():
    passed('match-lazy-error-arm', '''Again(n<u64>) = (result<u64>) :=
  result := Again(n).
''' + printed('true ? | true => 17 | false => Again(0)'), b'17\n')
    passed('match-once-and-local-binding', TRACE + printed('Trace(7) ? | x => x + x'), b'trace=7\n14\n')
    passed('match-wildcard-still-evaluates-input', TRACE + printed('Trace(7) ? | * => 19'), b'trace=7\n19\n')
    passed('match-guard-order', TRACE + printed('2 ? | x, Trace(x) == 0 => 1 | y => y'), b'trace=2\n2\n')
    passed('match-pattern-before-guard', TRACE + printed('2 ? | 1, Trace(99) == 0 => 1 | * => 17'), b'17\n')
    passed('match-nested', printed('true ? | true => (3 ? | 0 => 1 | x => x + 1) | false => 0'), b'4\n')
    passed('match-tuple-binding', printed('(7, true) ? | (x, true) => x | * => 0'), b'7\n')
    passed('match-arm-shadowing', 'x := 100\n' + printed('7 ? | x => x') + 'io/print("{}\\n", x)\n', b'7\n100\n')
    passed('match-pattern-recursion', '''Factorial(n<u64>) => <u64>
  | 0 => 1
  | k => k * Factorial(k - 1).
''' + printed('Factorial(10)'), b'3628800\n')
    c.fail('match-partial-function-failure', 'Only(n<u64>) => <u64>\n  | 0 => 7.\n' + printed('Only(1)'), 10)
    c.reject('match-incomplete-inline', printed('7 ? | 0 => 1'))
    c.reject('match-result-kind-mismatch', printed('true ? | true => 1 | false => 1.25'))
    c.reject('match-binding-kind-mismatch', printed('7 ? | x<f64> => 1'))
    c.reject('match-duplicate-binding', printed('(7, 8) ? | (x, x) => x'))
    c.reject('match-binding-cannot-escape', 'r := (7 ? | x => x)\nio/print("{}\\n", x)\n')
    c.reject('match-pattern-cannot-call-effects', TRACE + printed('7 ? | Trace(7) => 1 | * => 0'))
    passed('match-record-result', 'r := (true ? | true => {x: 7, y: 1.25} | false => {x: 8, y: 2.5})\nio/print("{}\\n", r.x)\n', b'7\n')
    passed('match-null-reference-pattern', printed('memory/null() ? | memory/null() => 1 | * => 0'), b'1\n')
    passed('match-unselected-memory-read', '''<row> := <{value<u64>}>
Read(n<u64>) = (result<u64>) :=
  row := memory/read(memory/null(), <row>)
  result := row.value.
''' + printed('true ? | true => 17 | false => Read(0)'), b'17\n')
    c.fail('match-layout-proof-cannot-escape', '<row> := <{value<u64>}>\nr := (false ? | true => memory/allocate({value: 1}) | false => memory/null())\nh := memory/allocate({value: 2})\nio/print("{}\\n", 1)\n', 9)
    distinct = '''<row> := <{value<u64>}>
Leaf(n<u64>) => <ref>
  | 0 => memory/allocate({value: 0})
  | k => memory/allocate({value: k}).
Test(n<u64>) = (result<u64>) :=
  a := Leaf(0)
  b := Leaf(0)
  c := Leaf(1)
  result := (a != b && b != c && a != c ? | true => 1 | false => 0).
r := memory/with-arena(3, <row>, Test, 0)
io/print("{}\\n", r)
'''
    passed('match-pattern-calls-allocate-distinct-records', distinct, b'1\n')
    inline_distinct = distinct.replace('a := Leaf(0)', 'a := (true ? | true => Leaf(0) | false => Leaf(1))').replace('b := Leaf(0)', 'b := (true ? | true => Leaf(0) | false => Leaf(1))')
    passed('match-inline-calls-allocate-distinct-records', inline_distinct, b'1\n')
    passed('sum-exclusive-range', printed('stats/sum([i | i <- 0..10])'), b'45\n')
    passed('sum-inclusive-stepped-range', printed('stats/sum([i | i <- 1..2..=5])'), b'9\n')
    passed('sum-exclusive-stepped-range', printed('stats/sum([i | i <- 1..2..5])'), b'4\n')
    passed('sum-empty-range', TRACE + printed('stats/sum([Trace(i) | i <- 5..5])'), b'0\n')
    passed('sum-reversed-range', printed('stats/sum([i | i <- 5..0])'), b'0\n')
    passed('sum-u64-maximum-endpoint', printed('stats/sum([1 | i <- 18446744073709551614..=18446744073709551615])'), b'2\n')
    passed('sum-local-invariant-capture', 'bias := 7\n' + printed('stats/sum([i + bias | i <- 0..10])'), b'115\n')
    passed('sum-bindings-and-filter', printed('stats/sum([twice | i <- 0..10, twice := i * 2, i > 5])'), b'60\n')
    passed('sum-nested-generators', printed('stats/sum([i + j | i <- 0..3, j <- 0..4])'), b'30\n')
    passed('sum-nested-comprehensions', printed('stats/sum([stats/sum([i + j | j <- 0..4]) | i <- 0..3])'), b'30\n')
    passed('sum-floating-values', 'r := stats/sum([1.25 | i <- 0..4])\nio/print("{}\\n", control/select(r == 5.0, 1, 0))\n', b'1\n')
    passed('sum-body-effects-in-order', TRACE + printed('stats/sum([Trace(i) | i <- 0..3])'), b'trace=0\ntrace=1\ntrace=2\n3\n')
    passed('sum-unused-qualifier-effects', TRACE + printed('stats/sum([i | i <- 0..3, ignored := Trace(i)])'), b'trace=0\ntrace=1\ntrace=2\n3\n')
    passed('sum-filter-skips-body-and-later-binding', TRACE + printed('stats/sum([i | i <- 0..5, i > 2, ignored := Trace(i)])'), b'trace=3\ntrace=4\n7\n')
    passed('sum-bounds-evaluated-once', TRACE + printed('stats/sum([i | i <- Trace(0)..Trace(1)..Trace(3)])'), b'trace=0\ntrace=1\ntrace=3\n3\n')
    passed('sum-separate-effects', TRACE + 'a := stats/sum([Trace(i) | i <- 0..2])\nb := stats/sum([Trace(i) | i <- 0..2])\nio/print("{}\\n", a + b)\n', b'trace=0\ntrace=1\ntrace=0\ntrace=1\n2\n')
    allocation = '''<row> := <{value<u64>}>
Allocate(n<u64>) = (result<u64>) :=
  h := memory/allocate({value: n})
  result := 1.
Test(n<u64>) = (result<u64>) :=
  result := stats/sum([Allocate(0) | i <- 0..3]).
r := memory/with-arena(2, <row>, Test, 0)
io/print("{}\\n", r)
'''
    c.fail('sum-allocates-on-every-iteration', allocation, 9)
    c.fail('sum-empty-body-proof-cannot-escape', allocation.split('Test(')[0] + 'r := stats/sum([Allocate(i) | i <- 0..0])\nh := memory/allocate({value: 2})\nio/print("{}\\n", r)\n', 9)
    c.fail('sum-zero-step', printed('stats/sum([i | i <- 0..0..10])'), 2)
    c.reject('sum-binding-cannot-escape', 'r := stats/sum([i | i <- 0..3])\nio/print("{}\\n", i)\n')
    c.reject('sum-filter-must-be-boolean', printed('stats/sum([i | i <- 0..3, 1])'))
    c.reject('sum-range-must-be-u64', printed('stats/sum([i | i <- 0.0..3.0])'))
    c.reject('sum-set-not-silently-matrix', printed('stats/sum({i | i <- 0..3})'))
    c.reject('sum-worker-transitive-io', TRACE + '''Chunk(start<u64>, end<u64>, arg<u64>) = (result<u64>) :=
  result := stats/sum([Trace(i) | i <- start..end]).
''' + printed('parallel/reduce-ranges(0, 10, 1, Chunk, 0, "sum")'))
    report = {'compiler_sha256': run.sha(c.MECH), 'passed': len(c.checks), 'checks': c.checks}
    (run.HERE / 'results/forms-conformance.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Passed', len(c.checks), 'native source-form checks', flush=True)


if __name__ == '__main__':
    main()
