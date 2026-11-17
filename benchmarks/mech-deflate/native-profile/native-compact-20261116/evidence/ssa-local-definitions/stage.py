"""Stage two SSA-builder probes for the profiled nonlocal lookup boundary."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
BASE = 'b05241855264df7df861994ef125e5dfb678bc81'
PATH = Path('vendor/cranelift-frontend/src/ssa.rs')
PROFILE = HERE.parent / 'ordered-vector-cursors/compiler-sample/result.json'


def replace(body, old, new):
    assert body.count(old) == 1, old
    return body.replace(old, new)


def local_hit(body):
    body = replace(body, '''        // Prepare the 'calls' and 'results' stacks for the state machine.
        self.use_var_nonlocal(func, var, ty, block);
        let value = self.run_state_machine(func, var, ty);''',
                   '''        // A local definition needs neither a pending lookup nor a result
        // stack round trip. Predecessor lookups still use the state machine.
        if let Some(value) = self.local_definition(var, block) {
            return (value, mem::take(&mut self.side_effects));
        }
        self.use_var_from_predecessors(func, var, ty, block);
        let value = self.run_state_machine(func, var, ty);''')
    return replace(body, '''        if let Some(val) = self.variables[var]
            .get(&block)
            .copied()
            .unwrap_or_default()
            .expand()
        {
            self.results.push(val);
            return;
        }

        // Otherwise, use Global Value Numbering (Algorithm 2 in the paper).''',
                   '''        if let Some(val) = self.local_definition(var, block) {
            self.results.push(val);
        } else {
            self.use_var_from_predecessors(func, var, ty, block);
        }
    }

    fn local_definition(&self, var: Variable, block: Block) -> Option<Value> {
        self.variables[var]
            .get(&block)
            .copied()
            .unwrap_or_default()
            .expand()
    }

    fn use_var_from_predecessors(&mut self, func: &mut Function, var: Variable, ty: Type, block: Block) {
        // Use Global Value Numbering (Algorithm 2 in the paper).''')


def inline_first(body):
    body = replace(body, '// Most variables have very few queried blocks.',
                   '''// Delay the per-variable hash allocation until a second sparse block
// needs a cached definition.
#[derive(Clone, Default)]
enum SparseDefs {
    #[default]
    Empty,
    One(Block, PackedOption<Value>),
    Many(FxHashMap<Block, PackedOption<Value>>),
}

impl SparseDefs {
    fn get(&self, block: &Block) -> Option<&PackedOption<Value>> {
        match self {
            Self::One(at, value) if at == block => Some(value),
            Self::Many(map) => map.get(block),
            _ => None,
        }
    }

    fn insert(&mut self, block: Block, value: PackedOption<Value>) -> Option<PackedOption<Value>> {
        match self {
            Self::Empty => *self = Self::One(block, value),
            Self::One(at, prior) if *at == block => return Some(mem::replace(prior, value)),
            Self::One(at, prior) => {
                let mut map = FxHashMap::default();
                map.insert(*at, *prior);
                map.insert(block, value);
                *self = Self::Many(map);
            }
            Self::Many(map) => return map.insert(block, value),
        }
        None
    }

    fn retain(&mut self, mut keep: impl FnMut(&Block, &mut PackedOption<Value>) -> bool) {
        match self {
            Self::One(block, value) => {
                if !keep(block, value) {
                    *self = Self::Empty;
                }
            }
            Self::Many(map) => map.retain(keep),
            Self::Empty => {}
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&Block, &PackedOption<Value>)> {
        let one = if let Self::One(block, value) = self {
            Some((block, value))
        } else {
            None
        };
        let many = if let Self::Many(map) = self { Some(map.iter()) } else { None };
        one.into_iter().chain(many.into_iter().flatten())
    }

    #[cfg(test)]
    fn keys(&self) -> impl Iterator<Item = &Block> {
        self.iter().map(|(block, _)| block)
    }
}

// Most variables have very few queried blocks.''')
    return replace(body, '    sparse: FxHashMap<Block, PackedOption<Value>>,', '    sparse: SparseDefs,')


def main():
    original = subprocess.check_output(['git', 'show', f'{BASE}:{PATH}'], cwd=ROOT).decode()
    for role, update in (('local-hit', local_hit), ('first-definition', inline_first)):
        current = update(original)
        folder = HERE / role
        target = folder / 'source' / PATH
        target.parent.mkdir(parents=True)
        target.write_text(current)
        patch = ''.join(unified_diff(original.splitlines(keepends=True), current.splitlines(keepends=True),
                                     f'a/{PATH}', f'b/{PATH}'))
        (folder / 'candidate.patch').write_text(patch)
        (folder / 'source.json').write_text(json.dumps({
            'base': BASE, 'path': str(PATH), 'source_sha256': sha256(current.encode()).hexdigest(),
            'sample_sha256': sha256(PROFILE.read_bytes()).hexdigest(), 'compiled': False,
        }, indent=2) + '\n')


if __name__ == '__main__':
    main()
