"""Separate the ordinary-Mech collector's search loop and bounded skip run.

Keep the original codec untouched. This experiment changes the collector's
state lifetimes; it does not change matching, observations or block policy.
"""
import argparse
from hashlib import sha256
from pathlib import Path


ORIGINAL_SHA256 = '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
HYBRID_SHA256 = 'ecf98df0c4f0ea9bfd2b5ae548a76fdfba178703c224a6e33e987848711a4b22'


def replace_once(text, before, after):
    assert text.count(before) == 1, before
    return text.replace(before, after)


SKIP_RUN = '''<optimal-skipped-position> := <{tables<[u16]> cache<[u32]> next-hash3<u64> next-hash4<u64> next-position<u64> cache-ptr<u64> base<u64> next-slide<u64>}>

// A skip run does not sample observations or decide block boundaries. Keep
// that state in the search loop, and retain the verified prefix in this loop.
optimal-skip-run(tables<[u16]>, cache<[u32]>, hash3<u64>, hash4<u64>, input<[u8]>, position<u64>, base<u64>, next-slide<u64>, pointer<u64>, count<u64>, previous-match<u64>) => (out<optimal-skipped-position>) :=
  remaining := bits/wrapping-sub(sequence/length(input), position)
  slide := position == next-slide
  mf := (slide? | true => rebase-nodes(tables) | false => tables)
  current-base := (slide? | true => position | false => base)
  slide-at := (slide? | true => bits/wrapping-add(position, bt-minimum(remaining, 32768u64)) | false => next-slide)
  max-length := bt-minimum(remaining, 258u64)
  advanced := ((max-length >= 5u64)?
    | true => bt-advance(mf, hash3, hash4, input, current-base, bits/wrapping-sub(position, current-base), max-length, false, 300u64, cache, pointer, previous-match)
    | false => {tables: mf, next-hash3: hash3, next-hash4: hash4, cache: cache, cache-ptr: pointer, previous-match: 0u64})
  next-tables := advanced.tables
  next-cache := advanced.cache
  next-hash3 := advanced.next-hash3
  next-hash4 := advanced.next-hash4
  next-pointer := advanced.cache-ptr
  with-header := bt-put-entry(next-cache, next-pointer, bits/wrapping-sub(next-pointer, pointer), byte(input, position))
  next-position := bits/wrapping-add(position, 1u64)
  after-pointer := bits/wrapping-add(next-pointer, 1u64)
  next-count := bits/wrapping-sub(count, 1u64)
  verified := advanced.previous-match
  out := ((next-count != 0u64)?
    | true => optimal-skip-run(next-tables, with-header, next-hash3, next-hash4, input, next-position, current-base, slide-at, after-pointer, next-count, verified)
    | false => {tables: next-tables, cache: with-header, next-hash3: next-hash3, next-hash4: next-hash4, next-position: next-position, cache-ptr: after-pointer, base: current-base, next-slide: slide-at}).

'''


def generate(source, mode='split'):
    assert mode in ('split', 'project-first', 'count-bound', 'project-and-bound')
    digest = sha256(source.encode()).hexdigest()
    assert digest in (ORIGINAL_SHA256, HYBRID_SHA256), digest
    begin = source.index('optimal-advance-position(')
    end = source.index('optimal-merge-checkpoint(', begin)
    advance = source[begin:end]
    advance = replace_once(advance, ', skip<u64>', '')
    advance = replace_once(advance, '''  skip-count := bits/shift-right-wrap(skip, 32u64)
  previous-match := bits/and(skip, 4294967295u64)
  searching := skip-count == 0u64
''', '')
    advance = replace_once(advance,
        'bt-advance(mf, hash3, hash4, input, current-base, bits/wrapping-sub(position, current-base), max-length, searching, 300u64, cache, pointer, previous-match)',
        'bt-without-prefix(bt-get-matches(mf, hash3, hash4, input, current-base, bits/wrapping-sub(position, current-base), max-length, max-length, 300u64, cache, pointer))')
    advance = replace_once(advance, '''  sampled := (searching?
    | true => optimal-sample-position(frequencies, observations, number, next-observation, position, best, minimum, input)
    | false => {fresh-frequencies: frequencies, fresh-observations: observations, number-fresh: number, next-observation: next-observation})''',
        '  sampled := optimal-sample-position(frequencies, observations, number, next-observation, position, best, minimum, input)')
    advance = replace_once(advance, '''  next-skip := (searching?
    | true => (((best >= 3u64) && (best >= max-length))? | true => bits/wrapping-sub(best, 1u64) | false => 0u64)
    | false => bits/wrapping-sub(skip-count, 1u64))''',
        '  next-skip := (((best >= 3u64) && (best >= max-length))? | true => bits/wrapping-sub(best, 1u64) | false => 0u64)')
    advance = replace_once(advance, '  verified := (searching? | true => searched-match | false => advanced.previous-match)',
        '  verified := searched-match')
    source = source[:begin] + advance + SKIP_RUN + source[end:]
    begin = source.index('optimal-block-loop(')
    end = source.index('optimal-collect-block(', begin)
    loop = source[begin:end]
    loop = replace_once(loop, ', skip<u64>', '')
    loop = replace_once(loop,
        '  advanced := optimal-advance-position(tables, cache, hash3, hash4, input, position, base, next-slide, skip, pointer, fresh-frequencies, fresh-observations, number-fresh, next-observation, minimum)',
        '''  advanced := optimal-advance-position(tables, cache, hash3, hash4, input, position, base, next-slide, pointer, fresh-frequencies, fresh-observations, number-fresh, next-observation, minimum)
  next-skip := bits/shift-right-wrap(advanced.skip, 32u64)
  after-skips := ((next-skip != 0u64)?
    | true => optimal-skip-run(advanced.tables, advanced.cache, advanced.next-hash3, advanced.next-hash4, input, advanced.next-position, advanced.base, advanced.next-slide, advanced.cache-ptr, next-skip, bits/and(advanced.skip, 4294967295u64))
    | false => {tables: advanced.tables, cache: advanced.cache, next-hash3: advanced.next-hash3, next-hash4: advanced.next-hash4, next-position: advanced.next-position, cache-ptr: advanced.cache-ptr, base: advanced.base, next-slide: advanced.next-slide})''')
    for field in ('tables', 'cache', 'next-hash3', 'next-hash4', 'next-position', 'base', 'next-slide', 'cache-ptr'):
        target = ('next-' + field if field in ('tables', 'cache', 'base') else
                  'next-slide-at' if field == 'next-slide' else
                  'next-pointer' if field == 'cache-ptr' else field)
        loop = replace_once(loop, f'  {target} := advanced.{field}', f'  {target} := after-skips.{field}')
    loop = replace_once(loop, '''  next-skip := advanced.skip
  decision := ((next-skip == 0u64)?
    | true => optimal-block-decision(frequencies, new-frequencies, observations, new-observations, number-observations, new-number, next-position, begin, maximum-end, next-pointer, sequence/length(input), previous-check, have-previous-check)
    | false => {frequencies: frequencies, fresh-frequencies: new-frequencies, observations: observations, fresh-observations: new-observations, number-observations: number-observations, number-fresh: new-number, previous-check: previous-check, have-previous-check: have-previous-check, changed: 0u64, keep-going: true})''',
        '  decision := optimal-block-decision(frequencies, new-frequencies, observations, new-observations, number-observations, new-number, next-position, begin, maximum-end, next-pointer, sequence/length(input), previous-check, have-previous-check)')
    loop = replace_once(loop, ', observe-at, next-skip, begin,', ', observe-at, begin,')
    source = source[:begin] + loop + source[end:]
    source = replace_once(source, ', number-observations, number-fresh, position, 0u64, begin, maximum-end, minimum, 0u64, 0u64)',
        ', number-observations, number-fresh, position, begin, maximum-end, minimum, 0u64, 0u64)')
    if mode in ('project-first', 'project-and-bound'):
        begin = source.index('optimal-block-loop(')
        end = source.index('optimal-collect-block(', begin)
        loop = source[begin:end]
        projections = '''  new-frequencies := advanced.fresh-frequencies
  new-observations := advanced.fresh-observations
  new-number := advanced.number-fresh
  observe-at := advanced.next-observation
'''
        loop = replace_once(loop, projections, '')
        line = '  next-skip := bits/shift-right-wrap(advanced.skip, 32u64)\n'
        loop = replace_once(loop, line, projections + line)
        source = source[:begin] + loop + source[end:]
    if mode in ('count-bound', 'project-and-bound'):
        line = 'count<u64>, previous-match<u64>) => (out<optimal-skipped-position>) :=\n'
        source = replace_once(source, line,
            line + '  skip-range! := (count > 0u64) && (count <= 257u64)\n')
    return source


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--mode', choices=['split', 'project-first', 'count-bound', 'project-and-bound'], default='split')
    args = parser.parse_args()
    args.output.write_text(generate(args.source.read_text(), args.mode))


if __name__ == '__main__':
    main()
