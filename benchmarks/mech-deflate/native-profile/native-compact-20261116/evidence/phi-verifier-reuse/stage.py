"""Stage a verified adjacent prune and constant-phi pass."""
import difflib
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess

here = Path(__file__).resolve().parent
q = here.parents[1]
repo = q.parent
checkout = q / 'native-review-pr/checkout'
head = 'd1a9ae0c3f4f8945ddd11fd3903edd6aaab089e2'
context = Path('vendor/cranelift-codegen/src/context.rs')
phis = context.with_name('remove_constant_phis.rs')
gpu = Path('hosts/gpu/src/native_artifact/program')


def repo_blob(path):
    return subprocess.check_output(['git', 'show', f'{head}:{path}'], cwd=repo)


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def phi_removal(text):
    text = replace(text, '/// classic forward dataflow analysis.  Then remove them.\n',
                   '/// classic forward dataflow analysis. Then remove them and report whether the function changed.\n')
    text = replace(text, 'pub fn do_remove_constant_phis(func: &mut Function, domtree: &mut DominatorTree) {\n',
                   'pub fn do_remove_constant_phis(func: &mut Function, domtree: &DominatorTree) -> bool {\n')
    return replace(text, '        n_consts\n    );\n}\n', '        n_consts\n    );\n    !need_editing.is_empty()\n}\n')


def combine_context(text):
    text = replace(text, '        self.eliminate_unreachable_code(isa)?;\n        self.remove_constant_phis(isa)?;\n',
                   '        self.prune_unreachable_and_remove_constant_phis(isa)?;\n')
    text = replace(text, '        do_remove_constant_phis(&mut self.func, &mut self.domtree);\n',
                   '        do_remove_constant_phis(&mut self.func, &self.domtree);\n')
    anchor = '    /// Replace all redundant loads with the known values in\n'
    return replace(text, anchor, '''    /// Prune unreachable blocks and then remove constant phis.
    ///
    /// Verification of the pruned function also covers the final state when
    /// the phi pass leaves the function intact. The standalone pass still
    /// verifies its result, since its caller may pass an unverified function.
    pub fn prune_unreachable_and_remove_constant_phis(
        &mut self,
        isa: &dyn TargetIsa,
    ) -> CodegenResult<()> {
        self.eliminate_unreachable_code(isa)?;
        if do_remove_constant_phis(&mut self.func, &self.domtree) {
            self.verify_if(isa)?;
        }
        Ok(())
    }

''' + anchor)


def adjacent_pairs(text, count):
    old = (r'(?m)^( +)context\.eliminate_unreachable_code\(isa\)\.map_err\(backend\)\?;\n'
           r'\1context\.remove_constant_phis\(isa\)\.map_err\(backend\)\?;')
    def replacement(match):
        indent = match[1]
        return (f'{indent}context\n'
                f'{indent}    .prune_unreachable_and_remove_constant_phis(isa)\n'
                f'{indent}    .map_err(backend)?;')
    source, actual = re.subn(old, replacement, text)
    assert count == actual, (count, actual)
    return source


def main():
    files = {
        context: combine_context,
        phis: phi_removal,
        gpu / 'codegen/cfg_cleanup/mod.rs': lambda text: adjacent_pairs(text, 8),
        gpu / 'compact/phase_function.rs': lambda text: adjacent_pairs(text, 1),
        gpu / 'compact/emit.rs': lambda text: adjacent_pairs(text, 1),
    }
    out = here / 'overlay'
    out.mkdir()
    bindings = {}
    patch = []
    for path, change in files.items():
        old = repo_blob(path)
        assert old == (checkout / path).read_bytes(), path
        candidate = change(old.decode()).encode()
        file = out / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(candidate)
        bindings[str(path)] = sha256(candidate).hexdigest()
        patch.extend(difflib.unified_diff(old.decode().splitlines(keepends=True),
                                          candidate.decode().splitlines(keepends=True), f'a/{path}', f'b/{path}'))
    patch = ''.join(patch).encode()
    (here / 'phi-verifier.patch').write_bytes(patch)
    (here / 'source-bindings.json').write_text(json.dumps({'pin': head, 'patch': sha256(patch).hexdigest(),
                                                        'production_sources': bindings}, indent=2) + '\n')


if __name__ == '__main__':
    main()
