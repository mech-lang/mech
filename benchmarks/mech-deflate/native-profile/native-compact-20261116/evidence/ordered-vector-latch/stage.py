"""Build a bottom-tested version of the already-admitted nonempty vector prefix."""

from difflib import unified_diff
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'ordered-vector-cursors'))
from build import read, sha
from stage import BASE, ORDERED, Q, ROOT, replace

# Keep the same established source and artifact pins as the cursor study.
CURSORS = HERE.parent / 'ordered-vector-cursors/single-address'
MOVES = 'MECH_ORDERED_VECTOR_LATCH_TEST'
FORM = 'MECH_ORDERED_VECTOR_WINDOW_CURSORS'
FLAGS = {
    'latch-control': {},
    'latch-test': {MOVES: '1'},
    'latch-test-deferred': {MOVES: '1', FORM: 'lanes-only'},
}


def change(text):
    text = replace(text, '''        let header = b.create_block();
        let body = b.create_block();
        let finish = b.create_block();
''', '''        let bottom_test = std::env::var_os("MECH_ORDERED_VECTOR_LATCH_TEST").is_some();
        let header = b.create_block();
        let body = (!bottom_test).then(|| b.create_block());
        let finish = b.create_block();
        if bottom_test {
            b.append_block_param(finish, types::I32X4);
            b.append_block_param(finish, types::I32X4);
        }
''')
    text = replace(text, '''        let more = b.ins().icmp(IntCC::UnsignedLessThan, k, end);
        b.ins().brif(more, body, &[], finish, &[]);
        b.switch_to_block(body);
''', '''        if let Some(body) = body {
            let more = b.ins().icmp(IntCC::UnsignedLessThan, k, end);
            b.ins().brif(more, body, &[], finish, &[]);
            b.switch_to_block(body);
        }
''')
    text = replace(text, '''        b.ins().jump(header, &ready);
        b.switch_to_block(finish);
''', '''        if bottom_test {
            // The setup selected a remaining length of at least 16, so
            // the rounded prefix contains at least one complete group.
            // Compare the following index before entering another group.
            let more = b.ins().icmp(IntCC::UnsignedLessThan, next, end);
            b.ins().brif(more, header, &ready, finish, &[costs.into(), indices.into()]);
        } else {
            b.ins().jump(header, &ready);
        }
        b.switch_to_block(finish);
        let (final_costs, final_indices) = if bottom_test {
            let final_values = b.block_params(finish);
            (final_values[0], final_values[1])
        } else {
            (params[1], params[2])
        };
''')
    text = replace(text, '''        let minima = minimum_splat(b, params[1]);
        let equal = b.ins().icmp(IntCC::Equal, params[1], minima);
''', '''        let minima = minimum_splat(b, final_costs);
        let equal = b.ins().icmp(IntCC::Equal, final_costs, minima);
''')
    text = replace(text, 'b.ins().icmp(IntCC::NotEqual, params[2], missing_indices)',
                         'b.ins().icmp(IntCC::NotEqual, final_indices, missing_indices)')
    text = replace(text, 'b.ins().iadd(params[2], lanes)', 'b.ins().iadd(final_indices, lanes)')
    return replace(text, '''        } else {
            (equal, params[2])
        };
''', '''        } else {
            (equal, final_indices)
        };
''')


def main():
    pins = read(CURSORS / 'source.json')
    assert BASE == pins['base']
    patch = []
    source_shas = {}
    for name, digest in pins['source_sha256'].items():
        source = CURSORS / 'source' / name
        assert sha(source) == digest
        old = subprocess.check_output(['git', 'show', f'{BASE}:{name}'], cwd=ROOT).decode()
        text = source.read_text()
        if Path(name).name == 'emit.rs':
            text = change(text)
        target = HERE / 'source' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        source_shas[name] = sha(target)
        patch.extend(unified_diff(old.splitlines(keepends=True), text.splitlines(keepends=True),
                                  f'a/{name}', f'b/{name}'))
    (HERE / 'cursor.patch').write_text(''.join(patch))
    (HERE / 'source.json').write_text(json.dumps(pins | {'source_sha256': source_shas,
        'modes': FLAGS, 'validated': False}, indent=2) + '\n')


if __name__ == '__main__':
    main()
