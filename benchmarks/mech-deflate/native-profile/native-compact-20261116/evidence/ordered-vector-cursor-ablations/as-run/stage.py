"""Prepare a separate four-lane ordered reduction cursor prototype."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
BASE = 'b05241855264df7df861994ef125e5dfb678bc81'
ORDERED = Path('hosts/gpu/src/native_artifact/program/codegen/ordered_reduction')
SWITCH = 'MECH_ORDERED_VECTOR_WINDOW_CURSORS'


def replace(text, before, after):
    assert text.count(before) == 1, before
    return text.replace(before, after)


def cost(text):
    text = replace(text, '        indices: Value,\n    ) -> Value {',
                   '        indices: Value,\n        cursors: &[(Window, Value)],\n    ) -> Value {')
    text = replace(text, '            Cost::Read(w) => self.read(b, proof, *w, k),',
                   '''            Cost::Read(window) => {
                let pointer = cursors
                    .iter()
                    .find_map(|(source, ptr)| (source == window).then_some(*ptr))
                    .unwrap_or_else(|| self.pointer(b, proof, *window, k));
                self.read(b, proof, *window, pointer)
            }''')
    text = replace(text, '''                let a = self.emit(b, proof, a, k, indices);
                let c = self.emit(b, proof, c, k, indices);''',
                   '''                let a = self.emit(b, proof, a, k, indices, cursors);
                let c = self.emit(b, proof, c, k, indices, cursors);''')
    return replace(text, '''    fn read(&self, b: &mut FunctionBuilder<'_>, proof: &Proof, w: Window, k: Value) -> Value {
        let pointer = self.pointer(b, proof, w, k);''',
                   '''    fn read(
        &self,
        b: &mut FunctionBuilder<'_>,
        proof: &Proof,
        w: Window,
        pointer: Value,
    ) -> Value {''')


def emit(text):
    text = replace(text, '''        b.ins()
            .jump(header, &[start.into(), costs.into(), indices.into()]);
        b.switch_to_block(header);''',
                   '''        let cursor_mode = cfg!(target_arch = "aarch64")
            .then(|| std::env::var_os("MECH_ORDERED_VECTOR_WINDOW_CURSORS"))
            .flatten();
        let windows = if cursor_mode.is_some() {
            self.reads.as_slice()
        } else {
            &[]
        };
        let deferred_lanes = cursor_mode.as_deref().is_some_and(|mode| mode == "defer-lanes")
            && !super::cost::Inputs::uses_index(&self.cost);
        let mut initial = vec![start.into(), costs.into(), indices.into()];
        for &window in windows {
            b.append_block_param(header, types::I64);
            let address = inputs.pointer(b, proof, window, start);
            initial.push(address.into());
        }
        b.ins().jump(header, &initial);
        b.switch_to_block(header);''')
    text = replace(text, '        let index = b.ins().iadd(index, lanes);',
                   '''        let index = if deferred_lanes {
            index
        } else {
            b.ins().iadd(index, lanes)
        };''')
    text = replace(text, '''        let missing_indices = b.ins().splat(types::I32X4, missing);
        let indices = b.ins().bitselect(equal, params[2], missing_indices);''',
                   '''        let missing_indices = b.ins().splat(types::I32X4, missing);
        let (equal, indices) = if deferred_lanes {
            // Every lane kept its four-wide group's starting index. The
            // original eligible index is that start plus the lane number.
            // Mask out the unelected lanes before reducing their positions.
            let present = b.ins().icmp(IntCC::NotEqual, params[2], missing_indices);
            let equal = b.ins().band(equal, present);
            (equal, b.ins().iadd(params[2], lanes))
        } else {
            (equal, params[2])
        };
        let indices = b.ins().bitselect(equal, indices, missing_indices);''')
    text = replace(text, '''        let cost = inputs.emit(b, proof, &self.cost, k, index);''',
                   '''        // Each cursor denotes the original checked address at this k.
        // Reverse reads still project precisely the same four low words.
        let cursors = windows.iter().copied().zip(params[3..].iter().copied()).collect::<Vec<_>>();
        let cost = inputs.emit(b, proof, &self.cost, k, index, &cursors);''')
    return replace(text, '''        b.ins()
            .jump(header, &[next.into(), costs.into(), indices.into()]);
        b.switch_to_block(finish);''',
                   '''        let mut ready = vec![next.into(), costs.into(), indices.into()];
        for (window, pointer) in cursors {
            let step = 4 * i64::from(proof.arrays[window.array].width);
            let step = if window.reverse { -step } else { step };
            let following = b.ins().iadd_imm(pointer, step);
            ready.push(following.into());
        }
        b.ins().jump(header, &ready);
        b.switch_to_block(finish);''')


def main():
    observed = Q / 'current-roadmap/ordered-scorer/vector-census.json'
    counts = json.loads(observed.read_bytes())
    assert counts['cases']['prefix-natural-4096']['groups'] == 24210
    assert counts['cases']['prefix-natural-8192']['groups'] == 40728
    output = HERE / 'source'
    output.mkdir()
    patches = []
    pins = {}
    for name, change in (('cost.rs', cost), ('emit.rs', emit)):
        path = ORDERED / name
        original = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT).decode()
        current = change(original)
        target = output / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(current)
        pins[str(path)] = sha256(current.encode()).hexdigest()
        patches.extend(unified_diff(original.splitlines(keepends=True), current.splitlines(keepends=True),
                                    f'a/{path}', f'b/{path}'))
    (HERE / 'cursor.patch').write_text(''.join(patches))
    (HERE / 'source.json').write_text(json.dumps({'base': BASE, 'source_sha256': pins, 'flag': SWITCH,
         'census_sha256': sha256(observed.read_bytes()).hexdigest(), 'validated': False}, indent=2) + '\n')


if __name__ == '__main__':
    main()
