"""Count graph preservation and IR work in a separate diagnostic compiler."""
from difflib import unified_diff
import os
from shutil import copy2
import subprocess
import sys

from production import CHECKOUT, HERE, OUT, REBUILD, TARGET, CODEGEN, read, save, sha

DEST = HERE / 'diagnostics'
CONTEXT = CODEGEN / 'context.rs'
SNAPSHOT = r'''
#[derive(Debug, PartialEq, Eq)]
struct FlowSnapshot(Vec<(crate::ir::Block, Option<crate::ir::Inst>,
                        Vec<(crate::ir::Inst, crate::ir::Block, bool)>)>);

impl FlowSnapshot {
    fn new(func: &Function) -> Self {
        Self(func.layout.blocks().map(|block| {
            let mut edges = Vec::new();
            crate::inst_predicates::visit_block_succs(func, block,
                |inst, target, listed| edges.push((inst, target, listed)));
            (block, func.layout.last_inst(block), edges)
        }).collect())
    }
}

#[derive(Clone, Copy)]
struct CleanupSize {
    blocks: usize,
    instructions: usize,
    parameters: usize,
}

impl CleanupSize {
    fn of(func: &Function) -> Self {
        Self {
            blocks: func.layout.blocks().count(),
            instructions: func.layout.blocks()
                .map(|block| func.layout.block_insts(block).count()).sum(),
            parameters: func.layout.blocks()
                .map(|block| func.dfg.block_params(block).len()).sum(),
        }
    }
}

'''


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def overlay(original):
    source = replace(original, 'impl Context {\n', SNAPSHOT + 'impl Context {\n')
    scope = source.index('    fn prune_and_remove_constant_phis(')
    head, body = source[:scope], source[scope:]
    body = replace(body, '        let pruned = eliminate_unreachable_code(&mut self.func, &mut self.cfg, &self.domtree);',
        '''        let old_layout = FlowSnapshot::new(&self.func);
        let before = CleanupSize::of(&self.func);
        let pruned = eliminate_unreachable_code(&mut self.func, &mut self.cfg, &self.domtree);
        let after_prune = CleanupSize::of(&self.func);
        let post_prune = FlowSnapshot::new(&self.func);
        if !pruned {
            assert_eq!(post_prune, old_layout);
        }''')
    body = replace(body, '        if do_remove_constant_phis(&mut self.func, &self.domtree) {',
        '''        let changed = do_remove_constant_phis(&mut self.func, &self.domtree);
        if changed {''')
    end = '''            verify_ir_with_dominators(&self.func, dominators, fisa)?;
        }
        Ok(())
    }

    /// Replace all redundant loads'''
    body = replace(body, end, '''            verify_ir_with_dominators(&self.func, dominators, fisa)?;
        }
        assert_eq!(FlowSnapshot::new(&self.func), post_prune);
        self.verify(fisa)?;
        let after_phi = CleanupSize::of(&self.func);
        assert_eq!(changed, after_prune.parameters > after_phi.parameters);
        eprintln!("flowgraph-reuse {} {} {} {} {} {} {} {} {} {} {} {}",
            self.func.name, u8::from(fresh_flowgraph), u8::from(pruned), u8::from(changed),
            before.blocks, before.instructions, before.parameters,
            after_prune.blocks, after_prune.instructions, after_prune.parameters,
            after_phi.instructions, after_phi.parameters);
        Ok(())
    }

    /// Replace all redundant loads''')
    return head + body


def build():
    original = CHECKOUT / CONTEXT
    data = original.read_bytes()
    receipt = read(OUT / 'build.json')
    assert sha(original) == receipt['production_source_sha256'][str(CONTEXT)]
    DEST.mkdir()
    patched = overlay(data.decode())
    (DEST / 'instrumentation.patch').write_text(''.join(unified_diff(
        data.decode().splitlines(keepends=True), patched.splitlines(keepends=True), f'a/{CONTEXT}', f'b/{CONTEXT}')))
    command = [sys.executable, '-B', REBUILD, DEST / 'rebuild', '--variant', 'frame-copies',
               '--compiler-opt-level', '3', '--direct-source', '--preselect-root-inlining',
               '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
    try:
        original.write_text(patched)
        with (DEST / 'build.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        copy2(TARGET / 'release/compile_profile', DEST / 'compile_profile')
    finally:
        original.write_bytes(data)
    for path, key in [('frontend/program.mecb', 'bytecode'), ('native/program.o', 'encoder'),
                      ('native/metadata.json', 'metadata')]:
        assert sha(DEST / 'rebuild' / path) == receipt['artifact_sha256'][key]
    save(DEST / 'plan.json', {'base': receipt['base'], 'production_sources': receipt['production_source_sha256'],
         'command': list(map(str, command)), 'compiler_sha256': sha(DEST / 'compile_profile'),
         'patch_sha256': sha(DEST / 'instrumentation.patch'), 'artifact_sha256': receipt['artifact_sha256'],
         'ordinary_plan_sha256': sha(OUT / 'matched-repeat/plan.json')})


def count():
    columns = ('fresh', 'pruned', 'changed_phi', 'blocks_before', 'instructions_before', 'parameters_before',
               'blocks_after_prune', 'instructions_after_prune', 'parameters_after_prune', 'instructions_after_phi',
               'parameters_after_phi')
    path = DEST / 'rebuild/direct-source.log'
    rows = []
    root_cleanup = False
    for line in path.read_text().splitlines():
        if line == 'compact-stage: CFG cleanup':
            root_cleanup = True
        elif line == 'compact-stage: machine code':
            root_cleanup = False
        elif line.startswith('flowgraph-reuse '):
            prefix, name, *fields = line.split()
            rows.append({'name': name, 'inside_root_cleanup': root_cleanup,
                         **dict(zip(columns, map(int, fields), strict=True))})
    assert rows
    root_rows = [row for row in rows if row['inside_root_cleanup']]
    assert 0 < len(root_rows) < len(rows)
    groups = {'root_cleanup': root_rows, 'all_functions': rows}
    totals = {}
    for scope, values in groups.items():
        totals[scope] = {
            'cleanup_calls': len(values),
            'prune_sweep_block_visits': sum(r['blocks_before'] for r in values),
            'blocks_removed': sum(r['blocks_before'] - r['blocks_after_prune'] for r in values),
            'instructions_pruned': sum(r['instructions_before'] - r['instructions_after_prune'] for r in values),
            'phi_sweep_instruction_visits': sum(r['instructions_after_prune'] for r in values),
            'phi_parameters_removed': sum(r['parameters_after_prune'] - r['parameters_after_phi'] for r in values),
            'first_verifications_reusing_fresh_graphs': sum(r['fresh'] and not r['pruned'] for r in values),
            'post_phi_verifications_reusing_graphs': sum(r['changed_phi'] for r in values),
            'full_verifier_instruction_visits': sum(r['instructions_after_prune'] +
                                                    r['changed_phi'] * r['instructions_after_phi'] for r in values),
        }
    save(DEST / 'summary.json', {'plan_sha256': sha(DEST / 'plan.json'), 'log_sha256': sha(path),
         'totals': totals, 'calls': rows,
         'scope': 'Verifier-enabled combined cleanup calls in one fresh diagnostic compiler execution. Root cleanup includes nested callees between the stage markers. Times excluded.'})
    print(totals, flush=True)


if __name__ == '__main__':
    if sys.argv[1:] != ['count']:
        build()
    count()
