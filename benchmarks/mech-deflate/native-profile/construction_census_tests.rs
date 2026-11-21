//! Exercise observer accounting on actual graphs, copies and compaction.
use cranelift_codegen::{
    construction_census::{self as census, ArenaSnapshot},
    ir::{DataFlowGraph, Function, InstBuilder, UserFuncName, Value, types},
    settings, verifier,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn empty(a: ArenaSnapshot) {
    assert_eq!(a.resident, 0);
}

#[test]
fn clone_clear_compact_and_cross_thread_drop_balance_actual_graph_storage() {
    census::reset();
    let mut f = Function::new();
    f.name = UserFuncName::user(0, 7);
    let mut builder = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut f, &mut builder);
        let entry = b.create_block();
        b.switch_to_block(entry);
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().iadd(zero, one);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    verifier::verify_function(&f, &settings::Flags::new(settings::builder())).unwrap();
    let initial = census::snapshot();
    assert_eq!(initial.cfg_builds, 1);
    assert_eq!(initial.dominator_builds, 1);
    assert_eq!(initial.full_verifications, 1);
    assert_eq!(initial.values.created, f.dfg.num_values() as u64);
    assert_eq!(initial.instructions.created, f.dfg.num_insts() as u64);
    assert_eq!(initial.blocks.created, f.dfg.num_blocks() as u64);
    assert_eq!(initial.values.resident, initial.values.created);
    let mut copy = f.clone();
    let copied = census::snapshot();
    assert_eq!(copied.values.created, 2 * initial.values.created);
    assert_eq!(copied.values.resident, copied.values.created);
    assert_eq!(copied.blocks.maximum_unit, initial.blocks.created);
    assert_eq!(
        copied.instructions.maximum_unit,
        initial.instructions.created
    );
    copy.dfg.clear();
    let cleared = census::snapshot();
    assert_eq!(cleared.values.resident, initial.values.resident);
    assert_eq!(cleared.instructions.resident, initial.instructions.resident);
    assert_eq!(cleared.blocks.resident, initial.blocks.resident);
    drop(copy);
    // Make a retired parser alias with no live instruction or parameter handle.
    f.dfg.make_invalid_value_for_parser();
    let alias = Value::from_u32((f.dfg.num_values() - 1) as u32);
    let first = f.dfg.values().next().unwrap();
    f.dfg.change_to_alias(alias, first);
    let before = census::snapshot();
    let original = f.dfg.num_values();
    let removed = f.dfg.compact_values();
    assert_eq!(removed, 1);
    let retained = f.dfg.num_values() as u64;
    let compacted = census::snapshot();
    assert_eq!(compacted.values.created, before.values.created + retained);
    assert_eq!(
        compacted.values.resident,
        before.values.resident - original as u64 + retained
    );
    assert!(compacted.values.peak_resident >= before.values.resident + retained);
    // Graphs can be created by the producer and destroyed by compilation jobs.
    std::thread::spawn(move || drop(f)).join().unwrap();
    let after = census::snapshot();
    empty(after.blocks);
    empty(after.instructions);
    empty(after.values);
    let mut empty_graph = DataFlowGraph::new();
    empty_graph.clear();
    drop(empty_graph);
    assert_eq!(census::snapshot(), after);
    census::reset();
    assert_eq!(census::snapshot().values.created, 0);
}
