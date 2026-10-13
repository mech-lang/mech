"""Concrete semantic review landmarks for the immutable Pony production sources.

Landmarks supplement source reading and complete native outputs; they do not
claim a text match is execution proof. Source/binary native evidence stays pinned.
"""
LANDMARKS = {
 'binarytrees': ['left = Node(depth - 1)', 'right = Node(depth - 1)', 'left = None', 'right = None', 'let tree = Node(_depth)', '_sum = _sum + tree.check()', '_long = Node(_maximum)', '_long.check()', '_remaining = _remaining - 1', '_one()'],
 'knucleotide': ['values.push(1)', 'values.push(2)', 'values.push(3)', 'values.push(4)', 'values.push(6)', 'values.push(12)', 'values.push(18)', '_parent.done(_slot, consume counts)', '_maps(slot)? = consume counts', '_completed == 7', 'DNA(input)?'],
 'pidigits': ['let numerator3 = (_q * _three) + _r', 'let numerator4 = (_q * _four) + _r', 'let digit3 = numerator3 / _t', 'let digit4 = numerator4 / _t', 'if digit3 == digit4 then', '_q = _q * _ten', '_k = _k + 1'],
 'fannkuchredux': ['let shift = (first / factorial(width - 1)?) % width', 'while _rank < stop do', 'while work(0)? != 0 do', 'while left < right do', 'work(left)? = work(right)?', 'work(right)? = value', 'flips = flips + 1', 'if (_rank and 1) == 0 then flips.i64() else -flips.i64() end', '_rank = _rank + 1', '_maximum = _maximum.max(flips)', '_pending = _pending - 1'],
 'fasta': ['total = total + entry._2', 'entries(index)? = (entry._1, total)', '_seed = ((_seed * 3877) + 29573) % 139968', 'let draw = _seed.f64() / 139968.0', 'if draw < entry._2 then', '_length = 3 * _n', '_length = 5 * _n', '_column == 60'],
 'spectralnorm': ['let denominator = (((i + j) * (i + j + 1)) / 2) + i + 1', 'value = value + (_input(column)? / denominator.f64())', '_output(first + index)? = output(index)?', '_pending = _pending - 1', '_phase = (_phase + 1) % 4', '_round < 10', '(uv / vv).sqrt()'],
 'mandelbrot': ['while iteration < 50 do', 'zi = ((zr + zr) * zi) + ci', 'zr = (tr - ti) + cr', 'tr = zr * zr', 'ti = zi * zi', '(tr + ti) > 4.0', '(_n + 7) / 8', 'USize(8).min(_n - (column * 8))', '_blocks(index)? = consume bytes', '_pending = _pending - 1', 'for block in _blocks.values() do'],
 'nbody': ['let d2 = (dx * dx) + (dy * dy) + (dz * dz)', 'let mag = dt / (d2 * d2.sqrt())', 'body.attract(system(j)?, dt)', 'body.integrate(dt)', 'advance(0.01)', 'sun.vx = px / sun.mass', 'Format.float[F64](energy() where fmt = FormatFix, prec = 9)'],
 'regexredux': ['let iterator = _iterator as MatchIterator', 'iterator.next()?', '_count = _count + 1', '_pending = 9', '_counts(index)? = count', '_pending = _pending - 1', 'expression.replace(_sequence, substitution._2 where global = true)?', '_replace(index + 1)', 'env.input(recover Input(this) end, 65536)'],
 'revcomp': ['_remaining = _remaining - 1', '_output.push(_complement(_sequence(_remaining)?.usize())?)', '_parent.done(consume output)', '_records.push((header, sequence.clone()))', '_env.out.write(consume output)', '_index = _index + 1', 'env.input(recover Input(this) end, 65536)'],
}
REVIEWS = {
 'binarytrees': 'Every interior node and leaf is a separate Node. One bounded behavior allocates/checks each transient tree; Main retains its long root to final output. Real allocation/check/finalizer traces required.',
 'knucleotide': 'All seven growable standard Map tables count every width-window. Isolation ownership moves each complete table to Main and preserves it through output. Real before/after complete-map and window traces required.',
 'pidigits': 'Each candidate independently forms and divides both numerator3 and numerator4 using ordinary GMP operations. No simplified one-numerator shortcut. Real complete candidate-state traces required.',
 'fannkuchredux': 'Workers initialize each global factorial rank range, visit every rank once, perform each prefix reversal in Pony and use global rank parity. Native N1..N8 checksum/max outputs and canonical N12 full output gate actual enumeration.',
 'fasta': 'CDFs accumulate F64 probabilities and each actual RNG draw uses the exact LCG and floating comparison; lines and record lengths are explicit. Native N0/tails/N20000 exact output and canonical full hash gate the floating algorithm.',
 'spectralnorm': 'Every row/column term performs strict F64 division in original accumulation order. Forty joined matrix phases run through ten rounds; joined owned arrays survive in Main. Native small/N100 and full N5500 precise complete output gates joins and numerics.',
 'mandelbrot': 'Pony evaluates each pixel recurrence up to fifty steps with strict floating operators, short-circuits escaped pixels, packs each tail lane correctly and parent orders blocks. Native odd/tail widths and canonical N16000 complete PBM gate exact recurrence and output.',
 'nbody': 'Five ordinary bodies retain the canonical constants. Ten pairs per iteration update both bodies before the separate position loop. Strict sqrt/division/energy code is Pony; bounded and full complete energy pairs checked with the frozen tolerance.',
 'regexredux': 'Published PCRE2-backed generic matching/replacement only. All nine pattern iterators and ordered five replacements are explicit application work in Pony; parent waits for nine results and orders all outputs. Native boundary/EOF and canonical full output gate work.',
 'revcomp': 'Each source byte is complemented at its real reversed offset; one record at a time preserves order. EOF, CRLF, IUPAC, blank records and long tails are native tested. Complete canonical output hash gates byte order and joins.',
}


def review(source, benchmark, variant):
    assert all(landmark in source for landmark in LANDMARKS[benchmark]), (benchmark, variant, 'Missing reviewed operation')
    assert 'use @' not in source, 'Production workloads have no benchmark-specific native helper'
    if benchmark in {'knucleotide', 'revcomp', 'regexredux'}:
        assert 'fun ref read_failed() =>' in source and '_parent.failed()' in source
        assert 'fun ref dispose() =>' in source and '_parent.finish()' in source
    return dict(semantic_review=REVIEWS[benchmark], all_landmarks_present=True, no_native_workload_kernel=True,
                native_output_evidence_required=True, variant=variant,
                scheduler_policy='Pony 0.74 maxthreads8 at every actual native command; declared actor count<=8; ordered parent joins retained.')
