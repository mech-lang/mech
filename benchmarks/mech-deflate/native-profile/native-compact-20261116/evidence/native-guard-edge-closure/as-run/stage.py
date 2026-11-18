"""Stage a dependency-directed closure for one new branch truth."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
BASE = 'b05241855264df7df861994ef125e5dfb678bc81'
COMPACT = Path('hosts/gpu/src/native_artifact/program/compact')
FLAG = 'MECH_INCREMENTAL_GUARD_EDGE_CLOSURE'


def change(body, old, replacement):
    assert body.count(old) == 1, old
    return body.replace(old, replacement)


def state_code(text):
    text = change(text, "    groups: Vec<Vec<u32>>,\n}",
                  "    groups: Vec<Vec<u32>>,\n    dependents: OnceLock<BTreeMap<u32, Vec<usize>>>,\n}")
    text = change(text, '''            projection_group,
            groups,
        }
    }
}''', '''            projection_group,
            groups,
            dependents: OnceLock::new(),
        }
    }

    fn dependents(&self) -> &BTreeMap<u32, Vec<usize>> {
        self.dependents.get_or_init(|| {
            let mut index = BTreeMap::<u32, Vec<usize>>::new();
            for (position, (out, expression)) in self.rows.iter().enumerate() {
                let inputs = match *expression {
                    Expression::Copy(a) | Expression::Not(a) => [Some(a), None],
                    Expression::And(a, b) | Expression::Or(a, b) => [Some(a), Some(b)],
                    Expression::Project { .. } => [None, None],
                };
                // A projection row reads its output truth. Rows for the
                // other outputs of this group propose their own peer facts.
                for value in [Some(*out), inputs[0], inputs[1]].into_iter().flatten() {
                    let dependent = index.entry(value).or_default();
                    if dependent.last() != Some(&position) {
                        dependent.push(position);
                    }
                }
            }
            index
        })
    }
}''')
    text = change(text, '''    pub(super) fn close(&mut self) {
        if self.closed {''',
                  '''    pub(super) fn assume_on_closed_edge(&mut self, value: u32, truth: bool) {
        let was_closed = self.closed;
        self.assume(value, truth);
        // In a previously closed state, only relations that read this
        // changed truth can add an inference in the initial round.
        self.close_from(was_closed.then_some(value));
    }

    pub(super) fn close(&mut self) {
        self.close_from(None);
    }

    fn close_from(&mut self, changed_truth: Option<u32>) {
        if self.closed {''')
    text = change(text, '''            .get_or_init(|| RelationIndex::new(&self.expressions));
        loop {
            let mut inferred = Vec::new();''',
                  '''            .get_or_init(|| RelationIndex::new(&self.expressions));
        let dependents = changed_truth.map(|_| index.dependents());
        let mut visit = changed_truth.map(|value| {
            dependents.and_then(|map| map.get(&value)).cloned().unwrap_or_default()
        });
        loop {
            let mut inferred = Vec::new();''')
    text = change(text, '''            for (position, (out, e)) in index.rows.iter().enumerate() {
                let out = *out;''',
                  '''            let mut infer_row = |position: usize| {
                let (out, e) = &index.rows[position];
                let out = *out;''')
    text = change(text, '''            }
            let before = self.known.len();
            for (value, truth) in inferred {
                if !self.known.contains_key(&value) {
                    self.known.insert(value, truth);
                }
            }
            if self.known.len() == before {
                break;
            }
        }
        self.closed = true;''',
                  '''            };
            if let Some(visit) = &visit {
                visit.iter().copied().for_each(&mut infer_row);
            } else {
                (0..index.rows.len()).for_each(&mut infer_row);
            }
            let before = self.known.len();
            let mut next = Vec::new();
            for (value, truth) in inferred {
                if !self.known.contains_key(&value) {
                    self.known.insert(value, truth);
                    if let Some(rows) = dependents.and_then(|map| map.get(&value)) {
                        next.extend_from_slice(rows);
                    }
                }
            }
            if self.known.len() == before {
                break;
            }
            if let Some(visit) = &mut visit {
                // Commit only between rounds. Restore the original row order
                // so conflicting first-inference priorities remain identical.
                next.sort_unstable();
                next.dedup();
                *visit = next;
            }
        }
        self.closed = true;''')
    return change(text, '''                        actual.close();
                        expected.close();
                        assert_eq!(
                            actual.known.to_map(),
                            expected.known,
                            "network={network} pattern={pattern} assumption=({value},{truth})"
                        );''',
                  '''                        actual.close();
                        expected.close();
                        let mut indexed = state.clone();
                        indexed.assume_on_closed_edge(value, truth);
                        assert_eq!(
                            indexed.known.to_map(), expected.known,
                            "indexed network={network} pattern={pattern} assumption=({value},{truth})"
                        );
                        assert_eq!(
                            actual.known.to_map(),
                            expected.known,
                            "network={network} pattern={pattern} assumption=({value},{truth})"
                        );''')


def guards_code(text):
    text = change(text, '''    expressions: Vec<Option<(u32, Expression)>>,
}''',
                  '''    expressions: Vec<Option<(u32, Expression)>>,
    incremental_edges: bool,
}''')
    text = change(text, '''            branches,
            expressions,
        };''',
                  '''            branches,
            expressions,
            incremental_edges: std::env::var_os("MECH_INCREMENTAL_GUARD_EDGE_CLOSURE").is_some(),
        };''')
    return change(text, '''        if let Some((value, no, yes)) = self.branches[site] {
            if next == no {
                edge.assume(value, false);
            }
            if next == yes {
                edge.assume(value, true);
            }
        }''',
                  '''        if let Some((value, no, yes)) = self.branches[site]
            && let Some(truth) = if next == yes {
                Some(true)
            } else if next == no {
                Some(false)
            } else {
                None
            }
        {
            if self.incremental_edges {
                edge.assume_on_closed_edge(value, truth);
            } else {
                edge.assume(value, truth);
            }
        }''')


def main():
    dest = HERE / 'source'
    dest.mkdir()
    files = {}
    diff = []
    for name, edit in (('guard_state.rs', state_code), ('guards.rs', guards_code)):
        path = COMPACT / name
        before = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT).decode()
        after = edit(before)
        out = dest / path
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(after)
        files[str(path)] = sha256(after.encode()).hexdigest()
        diff.extend(unified_diff(before.splitlines(keepends=True), after.splitlines(keepends=True),
                                 f'a/{path}', f'b/{path}'))
    (HERE / 'guard-index.patch').write_text(''.join(diff))
    (HERE / 'source.json').write_text(json.dumps({
        'base': BASE, 'source_sha256': files, 'flag': FLAG,
        'profile': str(HERE.parent / 'ordered-vector-cursors/compiler-sample/result.json'),
    }, indent=2) + '\n')
    harness = HERE / 'guard-harness.rs'
    harness.write_text(f'#[path = "{dest / COMPACT / "guard_state.rs"}"]\nmod state;\n')


if __name__ == '__main__':
    main()
