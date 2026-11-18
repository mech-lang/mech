"""Stage the owned retained-child traversal and exercise its spill path."""

from difflib import unified_diff
from hashlib import file_digest
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
REPO = Q.parent
BASE = 'cbcc3a803e59d506bb3d582f885e332e71ee1f53'
BRANCH = 'refs/heads/codex/native-compact-source-handoff'
CHECKOUT = Q / 'native-review-pr/checkout'
PACKET = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
SOURCE = Path('src/syntax/src/document/retained_sequence.rs')
PATCH = HERE / 'source.patch'
TARGET = ROADMAP / 'definition-compiler/target'
PLAN = ROADMAP / 'immutable-root-literals/matched/plan.json'
INCUMBENT = 'mech-literals-enabled'


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(data if isinstance(data, str) else json.dumps(data, indent=2) + '\n')


def git(*args):
    return subprocess.check_output(['git', *args], cwd=REPO)


def stage():
    assert git('rev-parse', BRANCH).decode().strip() == BASE
    current = git('show', f'{BASE}:{SOURCE}').decode()
    assert (CHECKOUT / SOURCE).read_text() == current
    begin = current.index('        struct Owned<T> {')
    end = current.index('\n    }\n    pub(crate) fn iter_from', begin)
    change = '''
        struct Owned<T> {
            pending: smallvec::SmallVec<[Arc<Node<T>>; 4]>,
        }
        impl<T: Clone> Iterator for Owned<T> {
            type Item = T;
            fn next(&mut self) -> Option<T> {
                let root = self.pending.pop()?;
                let mut node = root.as_ref();
                loop {
                    match node {
                        Node::Leaf(value) => return Some(value.clone()),
                        Node::Branch { left, right, .. } => {
                            self.pending.push(right.clone());
                            node = left;
                        }
                    }
                }
            }
        }
        let mut pending = smallvec::SmallVec::new();
        if let Some(root) = self.root {
            pending.push(root);
        }
        Owned { pending }'''
    candidate = current[:begin] + change.lstrip('\n') + current[end:]
    test = '''
    #[test]
    fn owned_traversal_keeps_order_after_the_parent_is_dropped() {
        for len in [0, 1, 2, 8, 16, 17, 32, 33, 129] {
            let expected = (0..len).map(Weighted).collect::<Vec<_>>();
            let retained = expected.iter().cloned().collect::<RetainedSequence<_>>();
            let mut owned = retained.clone().into_values();
            drop(retained);
            for item in expected {
                assert_eq!(owned.next(), Some(item));
            }
            assert_eq!(owned.next(), None);
            assert_eq!(owned.next(), None);
        }
    }

'''
    point = '    #[test]\n    fn unpublished_appends_allocate_linear_storage_and_rewinds_reuse_it()'
    assert candidate.count(point) == 1
    candidate = candidate.replace(point, test.lstrip('\n') + point)
    name = HERE / 'source' / SOURCE
    write(name, candidate)
    subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024', '--check',
                    '--config', 'skip_children=true', name], check=True)
    patch = ''.join(unified_diff(current.splitlines(keepends=True), name.read_text().splitlines(keepends=True),
                                 f'a/{SOURCE}', f'b/{SOURCE}'))
    write(PATCH, patch)
    write(HERE / 'source.json', {'base': BASE, 'source': str(SOURCE),
                                'control_sha256': sha(CHECKOUT / SOURCE),
                                'candidate_sha256': sha(name), 'patch_sha256': sha(PATCH)})


def focused_tests():
    tests = HERE / 'tests'
    tests.mkdir()
    crate = tests / 'sequence-tests.rs'
    code = '#![allow(dead_code)]\nextern crate alloc;\n'
    path = HERE / 'source' / SOURCE
    write(crate, code + f'#[path = {json.dumps(str(path))}]\nmod retained_sequence;\n')
    artifact_inventory = ROADMAP / 'scorer-cost-length/compiler/disabled/compiler-build.jsonl'
    smallvec = None
    for line in artifact_inventory.read_text().splitlines():
        if line.startswith('{'):
            item = json.loads(line)
            if item.get('reason') == 'compiler-artifact' and item['target']['name'] == 'smallvec':
                (smallvec,) = [Path(name) for name in item['filenames'] if name.endswith('.rlib')]
    assert smallvec and smallvec.is_file()
    binary = tests / 'test-sequence'
    command = ['rustc', '+nightly-2026-03-03', '--edition', '2024', '--test',
               '-D', 'warnings', '-C', 'opt-level=2', '-L', f'dependency={smallvec.parent}',
               '--extern', f'smallvec={smallvec}', crate, '-o', binary]
    log = tests / 'results.log'
    with log.open('w') as stream:
        subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, check=True)
        subprocess.run([binary], stdout=stream, stderr=subprocess.STDOUT, check=True)
    assert '3 passed; 0 failed' in log.read_text()
    write(tests / 'result.json', {'command': list(map(str, command)), 'passed': 3,
                                 'log_sha256': sha(log), 'smallvec_sha256': sha(smallvec),
                                 'source_sha256': sha(path), 'harness_sha256': sha(crate)})
    print(log.read_text(), flush=True)


if __name__ == '__main__':
    stage()
    focused_tests()
