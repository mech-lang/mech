"""Unexecuted exact-text insertion plan; original creators/checks/loops retained."""
from pathlib import Path

HOOKS = {
    'binarytrees': [
        ('    make(Depth, Tree),\n    check(Tree, Value),',
         '    clbg_observer:temporary_begin(Iterations, Depth),\n'
         '    make(Depth, Tree),\n    check(Tree, Value),\n'
         '    clbg_observer:temporary_checked(Iterations, Depth, Tree, Value),'),
        ('    check(Tree, Count),\n    format(\'stretch tree',
         '    check(Tree, Count),\n    clbg_observer:stretch_checked(Depth, Tree, Count),\n'
         '    format(\'stretch tree'),
        ('    make(Maximum, LongLived),\n    depths(4, Maximum),',
         '    make(Maximum, LongLived),\n    clbg_observer:retain(Maximum, LongLived),\n'
         '    depths(4, Maximum),'),
        ('    check(LongLived, Count),\n    format(\'long lived',
         '    check(LongLived, Count),\n    clbg_observer:retained_checked(Maximum, LongLived, Count),\n'
         '    format(\'long lived'),
    ],
    'knucleotide': [
        ('    ht_new(Table),\n    Mask is',
         '    ht_new(Table),\n    clbg_observer:map_begin(Width, Table),\n    Mask is'),
        ('    windows(1, Length, Width, Mask, Sequence, 0, Table),\n    all_maps',
         '    windows(1, Length, Width, Mask, Sequence, 0, Table),\n'
         '    clbg_observer:map_completed(Width, Table, Sequence, Length),\n    all_maps'),
        ('    all_maps([1,2,3,4,6,12,18], Sequence, Length, Tables),\n    Tables =',
         '    all_maps([1,2,3,4,6,12,18], Sequence, Length, Tables),\n'
         '    clbg_observer:maps_returned(Tables),\n    Tables ='),
    ],
    'pidigits': [
        ('    Digit4 is Numerator4 div T,\n    (   Digit3 =:= Digit4',
         '    Digit4 is Numerator4 div T,\n'
         '    clbg_observer:pi_candidate(Count, Q, R, T, K, Numerator3, Numerator4, Digit3, Digit4),\n'
         '    (   Digit3 =:= Digit4'),
        ('    current_prolog_flag(bounded, false),\n    digits(N, 0, 0, 1, 0, 1, 1).',
         '    current_prolog_flag(bounded, false),\n    clbg_observer:pi_begin(N),\n'
         '    digits(N, 0, 0, 1, 0, 1, 1),\n    clbg_observer:pi_finished(N).'),
    ],
}


def transform(benchmark, original, observer_path):
    assert benchmark in HOOKS
    assert 'clbg_observer' not in original
    path = str(Path(observer_path))
    assert not any(character in path for character in '\r\n\0')
    quoted = "'" + path.replace('\\', '\\\\').replace("'", "\\'") + "'"
    prefix = ':- use_module(' + quoted + ', []).\n'
    lines = original.splitlines(keepends=True)
    assert lines[0].startswith(':- module(')
    result = lines[0] + prefix + ''.join(lines[1:])
    edits = []
    for before, after in HOOKS[benchmark]:
        assert result.count(before) == 1, ('Expected unique actual source hook', benchmark, before)
        result = result.replace(before, after, 1)
        edits.append(dict(before=before, after=after))
    # Every other byte is unchanged; reverse projection must equal the exact
    # selected original, and the real make/check definitions are untouched.
    reconstructed = result
    for edit in reversed(edits):
        assert reconstructed.count(edit['after']) == 1
        reconstructed = reconstructed.replace(edit['after'], edit['before'], 1)
    reconstructed = reconstructed.replace(prefix, '', 1)
    assert reconstructed == original
    return result, edits
