"""Repeat the borderline C comparison with longer, fully balanced blocks."""
from importlib import import_module
from pathlib import Path
import json
import sys

q = Path(__file__).resolve().parent
driver = q / 'compare-library.py'
source = driver.read_text()
source = source.replace('orders = list(permutations(commands))',
                        'orders = list(permutations(commands)) * 4')
source = source.replace("selected = {case['name'] for case in cases}",
                        "selected = {'320-overlap'}")
before = "block_size = 512 if name in ('empty','one-stored','seven-stored-prefix') else (\n                     64 if case['input_bytes']<=1024 else 1)"
assert source.count(before) == 1
source = source.replace(before, 'block_size = 1024')
control = q / 'cranelift-continuation-registers-word-pages-native/final-candidate'
candidate = q / 'cranelift-owner-ranges-native/final-candidate'
results = []
for reverse in (False, True):
    label = 'cranelift-owner-ranges-close-win' + ('-reverse' if reverse else '')
    sys.argv = [str(driver), label, str(control), str(candidate)] + (['reverse'] if reverse else [])
    exec(compile(source, str(driver), 'exec'), {'__file__': str(driver), '__name__': '__main__'})
    results.append(import_module('checked-controller-results').compare(label))
(q / 'cranelift-owner-ranges-close-win.json').write_text(json.dumps({
    'reason': 'The retained control and candidate both crossed the C threshold in the second full order.',
    'case': '320-overlap', 'calls_per_actor_per_block': 1024,
    'balanced_blocks_per_order': 96, 'orders': results,
    'scope': 'Targeted follow-up; original full-run counts remain 61 and 60.'
}, indent=2) + '\n')
