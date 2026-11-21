"""Verify the private heap/literal ABI in the actual qualified encoder object."""

from collections import Counter
from hashlib import file_digest
import json
from pathlib import Path
import re
import subprocess
import sys


EXPECTED = {
    'release_owner': 74, 'release_nullable': 25, 'release_two': 30,
    'release_three': 32, 'release_four': 31,
    'writable_scalar_backing': 31, 'append_backing_byte_consume': 10,
}
OPERANDS = {
    'release_owner': 1, 'release_nullable': 1, 'release_two': 2,
    'release_three': 3, 'release_four': 4,
    'writable_scalar_backing': 6, 'append_backing_byte_consume': 5,
}
ALLOCATION_CALLS = {(1, 1): 1, (2, 0): 1, (2, 1): 1,
                    (4, 1): 21, (8, 0): 1, (8, 1): 1}


def main():
    trial = Path(sys.argv[1]).resolve()
    receipt = json.loads((trial/'receipt.json').read_text())
    path = trial/'bound/program.o'
    with path.open('rb') as stream:
        assert file_digest(stream, 'sha256').hexdigest() == receipt['layout']['object_sha256']
    atoms = receipt['layout']['symbols']
    root = atoms['_mech_compact_closed_checked']
    assembly = subprocess.check_output(['/usr/bin/otool', '-tvV', path], text=True)
    body = {int(at, 16): (op, args or '') for at, op, args in
            re.findall(r'^([a-f0-9]+)\s+(\S+)(?:[ \t]+([^\n]+))?$', assembly, re.M)}
    relocations = subprocess.check_output(['/usr/bin/otool', '-rV', path], text=True)
    relocs = {int(at, 16): callee for at, callee in re.findall(
        r'^([a-f0-9]+) True +long +True +BR26 +False +(\S+)', relocations, re.M)}
    root_body = {at: inst for at, inst in body.items()
                 if root['start'] <= at < root['start']+root['extent']}
    loaded, actual = set(), Counter()
    checked = {}

    def finish(name, target, instructions):
        atom = atoms[name]
        start, size = atom['start'], atom['extent']
        assert size >= 4*(len(instructions)+2)
        for i, instruction in enumerate(instructions):
            assert body[start+4*i] == instruction, (name, i, body[start+4*i])
        load = start+4*len(instructions)
        assert body[load][0] == 'ldr'
        offset = re.fullmatch(r'x0, \[sp, #(0x[\da-f]+)\]', body[load][1])
        assert offset, body[load]
        loaded.add(int(offset[1], 16))
        branch = load+4
        assert body[branch][0] == 'b' and relocs[branch] == target
        assert all(body[at] == ('udf', '#0x0') for at in range(branch+4, start+size, 4))
        calls = [at for at, callee in relocs.items() if callee == name]
        assert calls and all(root_body[at][0] == 'bl' for at in calls)
        actual[name] = len(calls)
        checked[name] = {'start': start, 'extent': size, 'sha256': atom['sha256'],
                         'tail_target': target, 'redirected_calls': len(calls)}

    for operation, operands in OPERANDS.items():
        target = '_mech_compact_'+operation
        name = target+'_from_frame'
        instructions = [('mov', f'x{dst}, x{dst-1}') for dst in range(operands, 0, -1)]
        finish(name, target, instructions)
        assert actual[name] == EXPECTED[operation]
    for (stride, zeroed), count in ALLOCATION_CALLS.items():
        target = '_mech_compact_allocate_backing'
        name = f'{target}_stride_{stride}_zeroed_{zeroed}_from_frame'
        instructions = [('mov', 'x1, x0'), ('mov', f'x2, #{hex(stride)}'),
                        ('mov', f'x3, #{hex(stride)}'), ('mov', f'x4, #{hex(zeroed)}')]
        finish(name, target, instructions)
        assert actual[name] == count
    (offset,) = loaded
    stores = [at for at, (op, args) in root_body.items()
              if op == 'str' and args == f'x0, [sp, #{hex(offset)}]']
    assert len(stores) == 1 and stores[0] < min(at for at, (op, _) in root_body.items() if op == 'bl')
    all_frame_calls = Counter(callee for at, callee in relocs.items()
                              if at in root_body and callee.endswith('_from_frame'))
    assert actual == all_frame_calls and sum(actual.values()) == 259
    assert not any(callee == '_mech_compact_allocate_backing' for at, callee in relocs.items()
                   if at in root_body)
    result = {'encoder_sha256': receipt['layout']['object_sha256'],
              'root_entry_heap_slot_offset': offset, 'redirected_call_count': sum(actual.values()),
              'allocation_call_count': sum(ALLOCATION_CALLS.values()), 'stubs': checked,
              'all_original_argument_and_result_registers_preserved': True,
              'all_stubs_tail_branch_to_original_entry': True,
              'all_stubs_preserve_sp_and_lr': True,
              'heap_stored_once_before_any_root_call': True,
              'control_change': 'Final CFG cleanup removes one unreachable nullable-release call.'}
    (trial/'abi-proof.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'stubs'}, indent=2))


if __name__ == '__main__':
    main()
