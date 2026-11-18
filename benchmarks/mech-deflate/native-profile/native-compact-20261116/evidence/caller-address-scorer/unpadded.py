"""Isolate historical root-tail padding from the caller-address experiment."""
from emit import HERE, HORIZONTAL, REVERSE, emit, read, save

ROLE = 'horizontal-reverse-layout-unpadded'


if __name__ == '__main__':
    emit(ROLE, HORIZONTAL | REVERSE, profile_path=HERE / 'combined-block-profile.json',
         drop_flags=('MECH_COMPACT_ROOT_TAIL_PROFILE',))
    root = HERE / ROLE
    padded = HERE / 'horizontal-reverse-layout'
    addresses = HERE / 'caller-addresses-refined'
    reference = read(padded / 'receipt.json')
    ablation = read(root / 'receipt.json')
    candidate = read(addresses / 'receipt.json')
    metadata = read(root / 'bound/metadata.json')
    before = read(padded / 'bound/metadata.json')
    after = read(addresses / 'bound/metadata.json')
    assert ablation['text_bytes'] == candidate['text_bytes'] == 71048
    assert reference['text_bytes'] == ablation['text_bytes'] + 48
    assert metadata['code_bytes'] == before['code_bytes'] == 50632
    assert after['code_bytes'] == 50628
    assert before['stack_bytes'] == metadata['stack_bytes'] == 3728
    assert after['stack_bytes'] == 3744
    profiles = [x for x in before.get('numeric_phases', []) if x['header'] == 3009]
    same_composed = profiles == [x for x in metadata.get('numeric_phases', []) if x['header'] == 3009]
    assert same_composed
    save(root / 'root-tail-ablation.json', {
        'padded_text_bytes': reference['text_bytes'],
        'unpadded_text_bytes': ablation['text_bytes'],
        'caller_address_text_bytes': candidate['text_bytes'],
        'root_padding_bytes_removed': 48,
        'padded_root_emitted_bytes_before_alignment': before['code_bytes'],
        'unpadded_root_emitted_bytes_before_alignment': metadata['code_bytes'],
        'caller_address_root_emitted_bytes_before_alignment': after['code_bytes'],
        'unpadded_root_frame_bytes': metadata['stack_bytes'],
        'caller_address_root_frame_bytes': after['stack_bytes'],
        'exact_checks': ablation['exact_checks'],
    })
