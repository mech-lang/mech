"""Prepare the existing source rebuild for the checked byte-read candidate."""
import difflib
from hashlib import sha256
import json
from pathlib import Path
import subprocess

here = Path(__file__).resolve().parent
q = here.parents[1]
checkout = q / 'native-review-pr/checkout'
packet = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
parent = 'd1a9ae0c3f4f8945ddd11fd3903edd6aaab089e2'
role = 'shared-byte-read-layout'
kind = role.replace('-', '_')


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def main():
    out = here / 'recipe-overlay'
    out.mkdir(exist_ok=True)
    originals = {path: (checkout / path).read_bytes()
                 for path in (packet / 'recipe.json', packet / 'rebuild.py')}
    for path, data in originals.items():
        assert subprocess.check_output(['git', 'show', f'{parent}:{path}'], cwd=q.parent) == data
    recipe = json.loads(originals[packet / 'recipe.json'])
    result = json.loads((here / 'screen/general-byte-read/result.json').read_bytes())
    assert result['identity']['program.mecb'] == recipe['bytecode_sha256']
    assert result['encoder_text_bytes'] == 71420
    key = 'MECH_COMPACT_CONTINUATION_WINDOWS'
    retained_windows = recipe['flags'][key] + ';'
    selected = result['flags'][key]
    assert selected.startswith(retained_windows)
    new_windows = selected.removeprefix(retained_windows)
    assert len(new_windows.split(',')) == 15
    recipe[kind + '_windows'] = new_windows
    recipe[kind + '_object_sha256'] = result['identity']['program.o']
    recipe[kind + '_metadata_sha256'] = result['identity']['metadata.json']
    source = originals[packet / 'rebuild.py'].decode()
    source = replace(source, "        'frame-copies',\n", f"        'frame-copies',\n        '{role}',\n")
    source = replace(source, "frame_copies = args.variant == 'frame-copies'\nzero_branches = args.variant == 'fresh-owner-counts-cold-tails-zero-branches' or frame_copies\n",
                     f"frame_copies = args.variant == 'frame-copies'\nshare_byte_reads = args.variant == '{role}'\n"
                     "zero_branches = args.variant in ('fresh-owner-counts-cold-tails-zero-branches',\n"
                     f"                                 'frame-copies', '{role}')\n")
    source = replace(source, "if zero_branches:\n    flags['MECH_COMPACT_BRANCH_ON_ZERO'] = '1'\n    tail_profile",
                     "if zero_branches:\n    flags['MECH_COMPACT_BRANCH_ON_ZERO'] = '1'\nif zero_branches and not share_byte_reads:\n    tail_profile")
    source = replace(source, "if frame_copies:\n    flags['MECH_COMPACT_SHARE_FRAME_COPIES'] = '1'\n",
                     "if frame_copies:\n    flags['MECH_COMPACT_SHARE_FRAME_COPIES'] = '1'\n"
                     "if share_byte_reads:\n    windows = flags['MECH_COMPACT_CONTINUATION_WINDOWS']\n"
                     f"    flags['MECH_COMPACT_CONTINUATION_WINDOWS'] = windows + ';' + recipe['{kind}_windows']\n")
    compile(source, 'rebuild.py', 'exec')
    candidate = {packet / 'rebuild.py': source, packet / 'recipe.json': json.dumps(recipe, indent=2) + '\n'}
    # Keep the established recipe's existing formatting. Add only the new fields.
    recipe_source = originals[packet / 'recipe.json'].decode()
    assert recipe_source.endswith('\n}\n')
    add = {key: recipe[key] for key in recipe.keys() if key.startswith(kind)}
    fields = ',\n' + json.dumps(add, indent=2)[2:-2]
    candidate[packet / 'recipe.json'] = recipe_source[:-3] + fields + '\n}\n'
    assert json.loads(candidate[packet / 'recipe.json']) == recipe
    patch = []
    record = {}
    for path, text in candidate.items():
        target = out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        record[str(path)] = sha256(text.encode()).hexdigest()
        patch.extend(difflib.unified_diff(originals[path].decode().splitlines(keepends=True),
                                          text.splitlines(keepends=True), f'a/{path}', f'b/{path}'))
    (here / 'shared-byte-recipe.patch').write_text(''.join(patch))
    (here / 'recipe-bindings.json').write_text(json.dumps({'pin': parent, 'variant': role, 'sources': record}, indent=2)+'\n')


if __name__ == '__main__':
    main()
