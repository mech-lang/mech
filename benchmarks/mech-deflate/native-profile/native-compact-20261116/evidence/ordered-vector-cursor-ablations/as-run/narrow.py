"""Separate lane deferral from carrying both addresses."""

from difflib import unified_diff
from pathlib import Path
from shutil import copy2
import subprocess

from build import read, save, sha
from stage import BASE, HERE, ROOT

SOURCE = HERE / 'single-address'
MODES = {
    'vec-reverse-cursor': 'reverse',
    'vec-lane-only': 'lanes-only',
    'vec-reverse-and-lanes': 'reverse-with-lanes',
}


def main():
    pins = read(HERE / 'source.json')
    assert pins['base'] == BASE
    SOURCE.mkdir()
    new_pins = {}
    patch = []
    for name, digest in pins['source_sha256'].items():
        relative = Path(name)
        original = HERE / 'source' / relative
        assert sha(original) == digest
        next_source = SOURCE / 'source' / relative
        next_source.parent.mkdir(parents=True, exist_ok=True)
        copy2(original, next_source)
        if relative.name == 'emit.rs':
            text = next_source.read_text()
            before = '''        let windows = if cursor_mode.is_some() {
            self.reads.as_slice()
        } else {
            &[]
        };
        let deferred_lanes = cursor_mode.as_deref().is_some_and(|mode| mode == "defer-lanes")
            && !super::cost::Inputs::uses_index(&self.cost);'''
            after = '''        let mode = cursor_mode.as_deref().and_then(std::ffi::OsStr::to_str);
        let windows = self.reads.iter().copied().filter(|window| match mode {
            Some("reverse" | "reverse-with-lanes") => window.reverse,
            Some("1" | "defer-lanes") => true,
            _ => false,
        }).collect::<Vec<_>>();
        let deferred_lanes = matches!(mode, Some("defer-lanes" | "lanes-only" | "reverse-with-lanes"))
            && !super::cost::Inputs::uses_index(&self.cost);'''
            assert text.count(before) == 1 and text.count('        for &window in windows {') == 1
            next_source.write_text(text.replace(before, after).replace('        for &window in windows {',
                                                                      '        for &window in &windows {'))
        new_pins[name] = sha(next_source)
        old = subprocess.check_output(['git', 'show', f'{BASE}:{relative}'], cwd=ROOT).decode()
        patch.extend(unified_diff(old.splitlines(keepends=True), next_source.read_text().splitlines(keepends=True),
                                  f'a/{relative}', f'b/{relative}'))
    (SOURCE / 'cursor.patch').write_text(''.join(patch))
    save(SOURCE / 'source.json', pins | {'source_sha256': new_pins, 'modes': MODES,
                                         'reason': 'Both-address variants add one scorer stack reload per outer loop.'})


if __name__ == '__main__':
    main()
