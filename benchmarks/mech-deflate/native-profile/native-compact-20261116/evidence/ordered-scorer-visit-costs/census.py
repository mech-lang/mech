"""Count ordered scorer lengths and strict improvements in a pinned arm64 atom."""

from hashlib import sha256
from pathlib import Path
import json
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ORIGINAL = HERE.parent / 'scorer-runtime/caller-addresses'
BOUND = ORIGINAL / 'bound'
COUNTER = Q / 'count-matcher-instructions.py'
OUT = HERE / 'trip-census'
ROLE = '_mech_compact_numeric_3009_no_growth_composed'
CASES = ('prefix-natural-4096', 'prefix-natural-8192', 'prefix-natural-16384', 'reymont-prefix-32769')


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def replace(code, old, new):
    assert code.count(old) == 1, old
    return code.replace(old, new)


def main():
    receipt = json.loads((ORIGINAL / 'receipt.json').read_bytes())
    object_sha = '5ce90c8ce5f1702e2f061231d4c9821627f8d072a36e753c2d249c459648f4c5'
    assert digest(BOUND / 'program.o') == receipt['layout']['object_sha256'] == object_sha
    assert digest(BOUND / 'mech-worker') == receipt['worker_sha256']
    native = {
        0x2214: ('cmp', 'x7, #0x10'),
        0x2234: ('cmp', 'w27, w23'),
        0x2238: ('csel', 'x25, x27, x23, lo'),
        0x22f8: ('cmp', 'w14, w25'),
        0x22fc: ('csel', 'x25, x14, x25, lo'),
    }
    source = COUNTER.read_text()
    source = replace(source, '    root = q / f"{label}-native/final-candidate"', f'    root = Path({str(BOUND)!r})')
    source = replace(source, '    directory = q / "matcher-dynamic-blocks" / actor', f'    directory = Path({str(OUT)!r})')
    source = replace(source, '    leaders = {start}', f'    assert all(assembly[pc] == op for pc, op in {native!r}.items())\n    leaders = {{start}}')
    source = replace(source, '        mapped[pc] = start + len(output) * 4', '''
        if pc == 0x2214:
            # This edge has 0 < remaining <= match_length <= 258. Bin by
            # that value directly. The inserted instructions preserve flags.
            loc = start + len(output) * 4
            new_relocs.extend(((loc + 4, 0x5D000000 | n), (loc + 8, 0x6C000000 | n)))
            output.extend((0xa9bf47f0, 0x90000010, 0xf9400210, 0x91400610, 0x8b070e10,
                           0xf9400211, 0x91000631, 0xf9000211, 0xa8c147f0))
        if pc in {0x2238, 0x22fc}:
            # B.HS skips the counter when the original strict unsigned test
            # did not improve the minimum. Preserve that test's NZCV.
            ix = 1000 if pc == 0x2238 else 1001
            loc = start + len(output) * 4
            new_relocs.extend(((loc + 8, 0x5D000000 | n), (loc + 12, 0x6C000000 | n)))
            output.extend((0x54000102, 0xa9bf47f0, 0x90000010, 0xf9400210,
                           0xf9400211 | (ix << 10), 0x91000631,
                           0xf9000211 | (ix << 10), 0xa8c147f0))
        mapped[pc] = start + len(output) * 4'''.lstrip('\n'))
    source = replace(source, '        assert visits == other and not any(visits[len(leaders):])', '''
        assert visits == other and not any(visits[len(leaders):512])
        assert not visits[512] and not any(visits[771:1000]) and not any(visits[1002:])
        lengths = {i: ct for i, ct in enumerate(visits[512:771]) if ct}
        assert sum(lengths.values()) == visits[leaders.index(0x2214)]
        assert sum(ct for n, ct in lengths.items() if n < 16) == visits[leaders.index(0x221c)]
        first = {"visits": visits[leaders.index(0x221c)], "improvements": visits[1000]}
        continued = {"visits": visits[leaders.index(0x22e8)], "improvements": visits[1001]}
        assert all(v["improvements"] <= v["visits"] for v in (first, continued))'''.lstrip('\n'))
    source = replace(source, '"blocks": sorted(details, key=lambda row: -row["visits"])}',
                     '"blocks": sorted(details, key=lambda row: -row["visits"]),\n'
                     '                        "trip_histogram": lengths, "first_scalar": first, "continued_scalar": continued}')
    sys.path.insert(0, str(Q))
    runner = {'__file__': str(COUNTER), '__name__': 'pinned_trip_census'}
    exec(compile(source, str(COUNTER), 'exec'), runner)
    runner['count']('trip-census', 'exact', CASES, ROLE, 1104)
    report = json.loads((OUT / 'counts.json').read_bytes())
    assert report['object_sha256'] == object_sha
    write(OUT / 'plan.json', {'object_sha256': object_sha, 'worker_sha256': receipt['worker_sha256'],
                            'counter_script_sha256': digest(COUNTER), 'patched_source': source,
                            'fixture_sha256': digest(Q / 'fixtures.json'), 'cases': CASES,
                            'counts_sha256': digest(OUT / 'counts.json'), 'timed': False})
    for batch, item in report['cases'].items():
        lengths = {int(n): v for n, v in item['trip_histogram'].items()}
        print(batch, sum(lengths.values()), item['first_scalar'], item['continued_scalar'],
              {k: lengths.get(k, 0) for k in range(1, 17)}, flush=True)


if __name__ == '__main__':
    main()
