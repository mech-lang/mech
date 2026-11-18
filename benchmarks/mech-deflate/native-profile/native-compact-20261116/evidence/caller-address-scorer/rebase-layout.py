"""Rebase the scorer profile when its ordered branch graph remains identical."""
from struct import unpack_from
import sys

from emit import HERE, HORIZONTAL, REVERSE, SCORER, emit, macho, read, save, sha


def signed_field(word, shift, bits):
    value = (word >> shift) & ((1 << bits) - 1)
    return value - (1 << bits) if value & (1 << (bits - 1)) else value


def control(word, at):
    if word & 0xFC00_0000 == 0x1400_0000:
        return 'jump', at + 4 * signed_field(word, 0, 26)
    if word & 0xFF00_0010 == 0x5400_0000:
        return ('condition', word & 15), at + 4 * signed_field(word, 5, 19)
    if word & 0x7E00_0000 == 0x3400_0000:
        return ('zero', (word >> 24) & 1), at + 4 * signed_field(word, 5, 19)
    if word & 0x7E00_0000 == 0x3600_0000:
        return ('bit', (word >> 24) & 1), at + 4 * signed_field(word, 5, 14)
    if word & 0xFFFF_FC1F == 0xD65F_0000 or word in (0xD65F_0BFF, 0xD65F_0FFF):
        return 'return', None
    if word & 0xFFE0_001F == 0xD420_0000 or word & 0xFFFF_0000 == 0:
        return 'trap', None
    return 'next', None


def parse(role):
    folder = HERE / role
    receipt = read(folder / 'receipt.json')
    atom = receipt['layout']['symbols'][SCORER]
    source = folder / 'frontend/program.o'
    assert sha(source) == receipt['layout']['object_sha256']
    current = read(folder / 'frontend/metadata.json')
    composed, = [row['composed_variant'] for row in current['numeric_phases']
                 if row.get('composed_variant', {}).get('symbol') == SCORER[1:]]
    image, sections, _ = macho.load(source)
    _, base, _, offset, *_ = sections[macho.TEXT]
    begin = offset + atom['start'] - base
    words = {at: unpack_from('<I', image, begin + at)[0] for at in range(0, composed['code_bytes'], 4)}
    references = []
    for at, word in words.items():
        if references and at >= min(references):
            break
        if word & 0x3B00_0000 == 0x1800_0000:
            target = at + 4 * signed_field(word, 5, 19)
            assert at < target < composed['code_bytes'] and target % 16 == 0
            references.append(target)
    assert references
    pool = min(references)
    code = {at: word for at, word in words.items() if at < pool}
    while code[max(code)] == 0xD503_201F:
        code.pop(max(code))
    stop = max(code) + 4
    transfers = {at: control(word, at) for at, word in code.items()}
    heads = {0}
    for at, (kind, target) in transfers.items():
        if target is not None:
            assert target in code
            heads.add(target)
        if kind != 'next' and at + 4 < stop:
            heads.add(at + 4)
    order = sorted(heads)
    ids = {at: index for index, at in enumerate(order)}
    blocks = []
    for end in [*order[1:], stop]:
        kind, target = transfers[end - 4]
        destination = None if target is None else ids[target]
        fallthrough = ids[end] if kind not in ('return', 'trap', 'jump') else None
        blocks.append((kind, destination, fallthrough))
    return {'object_sha256': sha(source), 'atom': atom, 'pool': pool, 'heads': order, 'blocks': blocks}


def main():
    addresses = 'addresses' in sys.argv[1:]
    role = 'caller-addresses' if addresses else 'horizontal-reverse'
    before, after = parse('horizontal'), parse(role)
    historical_file = HERE / 'scorer-block-profile.json'
    historical = read(historical_file)
    assert set(historical['order']) == set(before['heads']) and historical['literal_offset'] == before['pool']
    study = {'before': before, 'after': after, 'historical_sha256': sha(historical_file)}
    prefix = 'addresses' if addresses else 'combined'
    if before['blocks'] != after['blocks']:
        save(HERE / f'{prefix}-layout-rejected.json', study)
        print('ordered branches differ', len(before['blocks']), len(after['blocks']), flush=True)
        return
    table = dict(zip(before['heads'], after['heads'], strict=True))
    path = HERE / f'{prefix}-block-profile.json'
    save(path, {'order': [table[head] for head in historical['order']], 'literal_offset': after['pool']})
    save(HERE / f'{prefix}-layout-proof.json', study | {'profile_sha256': sha(path)})
    print('matched ordered branch graph', len(table), 'blocks', flush=True)
    if 'emit' in sys.argv[1:]:
        if addresses:
            from addresses import FLAG, NO_STALE_PROFILE, OUT
            compiler = OUT / 'compile_profile'
            assert sha(compiler) == read(OUT / 'build.json')['compiler_sha256']
            emit('caller-addresses-layout', FLAG | HORIZONTAL | REVERSE, compiler,
                 profile_path=path, drop_flags=NO_STALE_PROFILE)
        else:
            emit('horizontal-reverse-layout', HORIZONTAL | REVERSE, profile_path=path)


if __name__ == '__main__':
    main()
