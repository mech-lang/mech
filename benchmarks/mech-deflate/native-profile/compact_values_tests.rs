//! Verify the production DFG compactor, including shared pooled argument storage.
use cranelift_codegen::{
    Context,
    ir::*,
    isa,
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn aliases(dfg: &mut DataFlowGraph, mut root: Value, count: usize) -> Value {
    for _ in 0..count {
        let next = Value::from_u32(dfg.num_values() as u32);
        dfg.make_invalid_value_for_parser();
        dfg.make_value_alias_for_serialization(root, next);
        root = next;
    }
    root
}
fn diamond(seed: usize) -> Function {
    let mut f = Function::new();
    f.signature.params = vec![AbiParam::new(types::I64); 2];
    f.signature.returns = vec![AbiParam::new(types::I64)];
    let mut bc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut f, &mut bc);
    let entry = b.create_block();
    let yes = b.create_block();
    let no = b.create_block();
    let join = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let left = b.block_params(entry)[0];
    let right = b.block_params(entry)[1];
    let left = aliases(&mut b.func.dfg, left, 17 + seed * 3);
    let cond = b
        .ins()
        .icmp(condcodes::IntCC::UnsignedLessThan, left, right);
    b.ins().brif(cond, yes, &[], no, &[]);
    b.switch_to_block(yes);
    let plus = b.ins().iadd(left, right);
    let plus = aliases(&mut b.func.dfg, plus, 13 + seed);
    b.ins().jump(join, &[plus.into()]);
    b.switch_to_block(no);
    for _ in 0..seed % 7 {
        b.func.dfg.make_invalid_value_for_parser();
    }
    let minus = b.ins().isub(left, right);
    b.ins().jump(join, &[minus.into()]);
    b.switch_to_block(join);
    let result = b.block_params(join)[0];
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    f
}
fn arm_results(f: &Function) -> (Inst, Inst) {
    let yes = f.layout.blocks().nth(1).unwrap();
    let no = f.layout.blocks().nth(2).unwrap();
    (
        f.layout.block_insts(yes).next().unwrap(),
        f.layout.block_insts(no).next().unwrap(),
    )
}
fn machine(f: Function) -> Vec<u8> {
    let mut flags = settings::builder();
    flags.set("opt_level", "speed").unwrap();
    let isa = isa::lookup_by_name("aarch64")
        .unwrap()
        .finish(settings::Flags::new(flags))
        .unwrap();
    let mut cx = Context::for_function(f);
    cx.compile(&*isa, &mut Default::default())
        .unwrap()
        .buffer
        .data()
        .to_vec()
}
// Definition identities survive renumbering. This reference observes the public
// IR's actual inputs/targets, independent of the compactor's value-ID mapping.
fn graph(f: &Function) -> Vec<String> {
    let mut rows = Vec::new();
    for block in f.layout.blocks() {
        rows.push(format!(
            "{block}: {:?}",
            f.dfg.block_param_types(block).collect::<Vec<_>>()
        ));
        for inst in f.layout.block_insts(block) {
            let data = &f.dfg.insts[inst];
            let args = f
                .dfg
                .inst_args(inst)
                .iter()
                .map(|&v| (f.dfg.value_type(v), f.dfg.value_def(v)))
                .collect::<Vec<_>>();
            let branches = data
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                .iter()
                .map(|call| {
                    (
                        call.block(&f.dfg.value_lists),
                        call.args(&f.dfg.value_lists)
                            .map(|arg| match arg {
                                BlockArg::Value(v) => {
                                    format!("{:?}:{:?}", f.dfg.value_type(v), f.dfg.value_def(v))
                                }
                                other => format!("{other:?}"),
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>();
            rows.push(format!("{inst}: {:?} {args:?} {branches:?}", data.opcode()));
        }
    }
    rows
}
#[test]
fn varied_alias_chains_and_holes_preserve_graph_and_exact_machine_code() {
    let flags = settings::Flags::new(settings::builder());
    for seed in 0..64 {
        let original = diamond(seed);
        cranelift_codegen::verify_function(&original, &flags).unwrap();
        let expected = graph(&original);
        let mut compact = original.clone();
        let before = compact.dfg.num_values();
        let retired = compact.dfg.compact_values();
        assert!(retired > 0);
        assert_eq!(before - retired, compact.dfg.num_values());
        assert_eq!(graph(&compact), expected);
        cranelift_codegen::verify_function(&compact, &flags).unwrap();
        assert_eq!(machine(original), machine(compact.clone()), "seed={seed}");
        assert_eq!(compact.dfg.compact_values(), 0);
        assert_eq!(graph(&compact), expected);
    }
}
#[test]
fn a_million_aliases_release_id_space_and_keep_debug_label_targets() {
    let mut f = diamond(0);
    let (plus, minus) = arm_results(&f);
    let root = f.dfg.inst_results(plus)[0];
    let target = f.dfg.inst_results(minus)[0];
    aliases(&mut f.dfg, root, 1_000_003);
    f.dfg.collect_debug_info();
    f.dfg.values_labels.as_mut().unwrap().insert(
        root,
        ValueLabelAssignments::Starts(vec![ValueLabelStart {
            from: RelSourceLoc::default(),
            label: ValueLabel::from_u32(9),
        }]),
    );
    f.dfg
        .add_value_label_alias(target, RelSourceLoc::default(), root);
    let expected = graph(&f);
    assert!(f.dfg.compact_values() >= 1_000_003);
    assert!(f.dfg.num_values() < 32);
    assert_eq!(graph(&f), expected);
    let root = f.dfg.inst_results(plus)[0];
    let target = f.dfg.inst_results(minus)[0];
    assert!(matches!(
        f.dfg.values_labels.as_ref().unwrap()[&root],
        ValueLabelAssignments::Starts(_)
    ));
    match f.dfg.values_labels.as_ref().unwrap()[&target] {
        ValueLabelAssignments::Alias { value, .. } => assert_eq!(value, root),
        _ => panic!("expected remapped debug alias"),
    }
}
#[test]
fn shared_block_calls_and_unreferenced_tables_are_mapped_once() {
    let mut f = diamond(3);
    let (plus, _) = arm_results(&f);
    let root = f.dfg.inst_results(plus)[0];
    let root = aliases(&mut f.dfg, root, 7);
    let join = f.layout.blocks().last().unwrap();
    let call = BlockCall::new(join, [BlockArg::Value(root)], &mut f.dfg.value_lists);
    // These copies intentionally share the same underlying pooled argument list.
    let table = f.create_jump_table(JumpTableData::new(call, &[call, call]));
    f.dfg.compact_values();
    let root = f.dfg.inst_results(plus)[0];
    for branch in f.dfg.jump_tables[table].all_branches() {
        assert_eq!(branch.block(&f.dfg.value_lists), join);
        assert_eq!(
            branch.args(&f.dfg.value_lists).collect::<Vec<_>>(),
            vec![BlockArg::Value(root)]
        );
    }
}
#[test]
fn exception_contexts_and_pseudo_arguments_and_union_inputs_survive() {
    let mut f = diamond(5);
    let entry = f.layout.entry_block().unwrap();
    let (plus, _) = arm_results(&f);
    let root = f.dfg.inst_results(plus)[0];
    let root = aliases(&mut f.dfg, root, 9);
    let left = f.dfg.block_params(entry)[0];
    let join = f.layout.blocks().last().unwrap();
    let call = BlockCall::new(
        join,
        [
            BlockArg::Value(root),
            BlockArg::TryCallRet(2),
            BlockArg::TryCallExn(3),
        ],
        &mut f.dfg.value_lists,
    );
    let sig = f
        .dfg
        .signatures
        .push(Signature::new(isa::CallConv::SystemV));
    let exception = f.dfg.exception_tables.push(ExceptionTableData::new(
        sig,
        call,
        [
            ExceptionTableItem::Context(root),
            ExceptionTableItem::Default(call),
        ],
    ));
    f.dfg.union(left, root);
    f.dfg.compact_values();
    let root = f.dfg.inst_results(plus)[0];
    let table = &f.dfg.exception_tables[exception];
    for branch in table.all_branches() {
        assert_eq!(
            branch.args(&f.dfg.value_lists).collect::<Vec<_>>(),
            vec![
                BlockArg::Value(root),
                BlockArg::TryCallRet(2),
                BlockArg::TryCallExn(3)
            ]
        );
    }
    assert!(
        table
            .display(&f.dfg.value_lists)
            .to_string()
            .contains(&format!("context {root}"))
    );
    let union = f.dfg.values().last().unwrap();
    assert_eq!(
        f.dfg.value_def(union),
        ValueDef::Union(f.dfg.block_params(entry)[0], root)
    );
}
#[test]
fn empty_and_already_dense_graphs_are_stable() {
    let mut empty = DataFlowGraph::new();
    assert_eq!(empty.compact_values(), 0);
    let block = empty.make_block();
    empty.append_block_param(block, types::F64);
    empty.append_block_param(block, types::I128);
    assert_eq!(empty.compact_values(), 0);
    assert_eq!(
        empty.block_param_types(block).collect::<Vec<_>>(),
        vec![types::F64, types::I128]
    );
}

#[test]
fn forward_aliases_and_multiple_instruction_results_keep_their_positions() {
    let mut f = Function::new();
    f.signature.params.push(AbiParam::new(types::I64));
    f.signature.returns.push(AbiParam::new(types::I32));
    let mut bc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut f, &mut bc);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    for _ in 0..64 {
        b.func.dfg.make_invalid_value_for_parser();
    }
    let wide = b.block_params(entry)[0];
    let (lo, hi) = b.ins().isplit(wide);
    let split = b.func.dfg.value_def(lo).unwrap_inst();
    let mut root = hi;
    for index in (1..=64).rev() {
        let dest = Value::from_u32(index);
        b.func.dfg.make_value_alias_for_serialization(root, dest);
        root = dest;
    }
    let sum = b.ins().iadd(lo, root);
    let add = b.func.dfg.value_def(sum).unwrap_inst();
    b.ins().return_(&[sum]);
    b.seal_all_blocks();
    b.finalize();
    let expected = graph(&f);
    let original = f.clone();
    assert_eq!(f.dfg.compact_values(), 64);
    assert_eq!(f.dfg.num_values(), 4);
    assert_eq!(graph(&f), expected);
    assert_eq!(f.dfg.inst_args(add), f.dfg.inst_results(split));
    let flags = settings::Flags::new(settings::builder());
    cranelift_codegen::verify_function(&original, &flags).unwrap();
    cranelift_codegen::verify_function(&f, &flags).unwrap();
    // This is valid multi-result IR; I32 isplit lowering is not supported by
    // this backend. Machine-byte comparisons use supported instructions above.
}

#[test]
fn forward_alias_chains_preserve_exact_supported_machine_code() {
    let mut f = Function::new();
    f.signature.params.push(AbiParam::new(types::I64));
    f.signature.returns.push(AbiParam::new(types::I64));
    let mut bc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut f, &mut bc);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    for _ in 0..64 {
        b.func.dfg.make_invalid_value_for_parser();
    }
    let input = b.block_params(entry)[0];
    let mut root = b.ins().iconst(types::I64, 17);
    for index in (1..=64).rev() {
        let dest = Value::from_u32(index);
        b.func.dfg.make_value_alias_for_serialization(root, dest);
        root = dest;
    }
    let sum = b.ins().iadd(input, root);
    b.ins().return_(&[sum]);
    b.seal_all_blocks();
    b.finalize();
    let expected = graph(&f);
    let original = f.clone();
    assert_eq!(f.dfg.compact_values(), 64);
    assert_eq!(f.dfg.num_values(), 3);
    assert_eq!(graph(&f), expected);
    assert_eq!(machine(f), machine(original));
}
