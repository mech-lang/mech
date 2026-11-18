use super::super::route_uses::Uses;
use super::optimize_with_index;
use super::test_support::fixture;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::{Block, Function, Value};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, default_libcall_names};
use std::collections::BTreeSet;

fn traversal(
    f: &Function,
    dom: &DominatorTree,
    target: Block,
    value: Value,
    owner: Block,
) -> Option<bool> {
    let mut pending = vec![target];
    let mut seen = BTreeSet::new();
    let mut used = false;
    while let Some(block) = pending.pop() {
        if block == owner || !seen.insert(block) {
            continue;
        }
        for inst in f.layout.block_insts(block) {
            let mut found = f.dfg.inst_args(inst).contains(&value);
            for call in
                f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            {
                found |= call
                    .args(&f.dfg.value_lists)
                    .any(|arg| arg.as_value() == Some(value));
                pending.push(call.block(&f.dfg.value_lists));
            }
            if found {
                if !dom.block_dominates(target, block) || !dom.block_dominates(owner, target) {
                    return None;
                }
                used = true;
            }
        }
    }
    Some(used)
}

#[test]
fn indexed_live_outs_match_stopped_traversal_for_every_fixture_value_and_target() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for variant in [
        "normal",
        "duplicate",
        "overflow-first",
        "effect",
        "load",
        "merge",
        "opaque",
    ] {
        let mut c = fixture(&module, variant);
        c.func.dfg.resolve_all_aliases();
        c.verify(module.isa()).unwrap();
        c.compute_cfg();
        c.compute_domtree();
        let uses = Uses::new(&c.func);
        let mut definitions = BTreeSet::new();
        for block in c.func.layout.blocks() {
            definitions.extend(c.func.dfg.block_params(block).iter().copied());
            for inst in c.func.layout.block_insts(block) {
                definitions.extend(c.func.dfg.inst_results(inst).iter().copied());
            }
        }
        for target in c
            .func
            .layout
            .blocks()
            .filter(|&block| c.domtree.is_reachable(block))
        {
            let mut query = uses.live_outs(target);
            for &definition in &definitions {
                let value = c.func.dfg.resolve_aliases(definition);
                let owner = super::values::owner(&c.func, value).unwrap();
                assert_eq!(
                    query.required(&c.domtree, value, owner),
                    traversal(&c.func, &c.domtree, target, value, owner),
                    "{variant}: {target}, {value}, owner {owner}"
                );
            }
        }
    }
}

#[test]
fn indexed_loop_routes_select_identical_ir_and_repairs_to_the_original_traversal() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for variant in [
        "normal",
        "duplicate",
        "overflow-first",
        "effect",
        "load",
        "merge",
        "opaque",
    ] {
        let mut before = fixture(&module, variant);
        before.func.dfg.resolve_all_aliases();
        let mut indexed = module.make_context();
        indexed.func = before.func.clone();
        before.compute_cfg();
        before.compute_domtree();
        indexed.compute_cfg();
        indexed.compute_domtree();
        let reference = optimize_with_index(&mut before.func, &before.cfg, &before.domtree, false);
        let candidate =
            optimize_with_index(&mut indexed.func, &indexed.cfg, &indexed.domtree, true);
        assert_eq!(
            (
                candidate.candidates,
                candidate.threaded_edges,
                candidate.selected_conditions,
                candidate.repair_parameters
            ),
            (
                reference.candidates,
                reference.threaded_edges,
                reference.selected_conditions,
                reference.repair_parameters
            ),
            "{variant}"
        );
        assert_eq!(
            indexed.func.display().to_string(),
            before.func.display().to_string(),
            "{variant}"
        );
        before.verify(module.isa()).unwrap();
        indexed.verify(module.isa()).unwrap();
    }
}
