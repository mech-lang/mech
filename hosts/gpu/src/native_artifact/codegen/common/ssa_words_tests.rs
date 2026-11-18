//! SSA boundary transport must keep the home's actual alias classification.
use super::*;
use cranelift_codegen::ir::{AbiParam, AliasRegion, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;

#[test]
fn ssa_home_spills_and_reloads_keep_the_boundary_memory_region() {
    for flags in [
        MemFlags::new(),
        MemFlags::new()
            .with_alias_region(Some(AliasRegion::Table))
            .with_notrap(),
        MemFlags::new()
            .with_alias_region(Some(AliasRegion::Heap))
            .with_notrap(),
    ] {
        let mut function = Function::new();
        function.signature.params.push(AbiParam::new(types::I64));
        function.signature.returns.push(AbiParam::new(types::I64));
        let mut context = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut function, &mut context);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let locals = b.block_params(entry)[0];
            let variable = b.declare_var(types::I64);
            let initial = b.ins().iconst(types::I64, 41);
            b.def_var(variable, initial);
            let words = Words {
                variables: BTreeMap::from([(3, (Home::Variable(variable), flags))]),
                literal_write: Cell::new(false),
                native_extent_slots: BTreeMap::new(),
                native_extent_values: RefCell::new(BTreeMap::new()),
            };
            let updated = b.ins().iadd_imm(initial, 1);
            words.write(&mut b, 3, updated);
            words.spill_all(&mut b, locals).unwrap();
            words
                .reload_live(&mut b, locals, &BTreeSet::from([3]))
                .unwrap();
            let value = words.read(&mut b, 3);
            b.ins().return_(&[value]);
            b.seal_all_blocks();
            b.finalize();
        }
        let operations = function
            .layout
            .blocks()
            .flat_map(|block| function.layout.block_insts(block))
            .filter(|&inst| {
                matches!(
                    function.dfg.insts[inst].opcode(),
                    Opcode::Load | Opcode::Store
                )
            })
            .map(|inst| function.dfg.insts[inst].memflags().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            operations,
            vec![flags, flags],
            "a spill in another region cannot invalidate the boundary's cached load"
        );
    }
}

#[test]
fn entry_literal_needs_no_merge_parameter_and_flags_unexpected_writes() {
    let mut function = Function::new();
    let mut context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut function, &mut context);
    let entry = builder.create_block();
    let alternative = builder.create_block();
    let merge = builder.create_block();
    builder.switch_to_block(entry);
    let literal = builder.ins().iconst(types::I64, 41);
    let condition =
        builder
            .ins()
            .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, literal, 0);
    let words = Words {
        variables: BTreeMap::from([(0, (Home::Literal(literal), MemFlags::new()))]),
        literal_write: Cell::new(false),
        native_extent_slots: BTreeMap::new(),
        native_extent_values: RefCell::new(BTreeMap::new()),
    };
    builder.ins().brif(condition, alternative, &[], merge, &[]);
    builder.switch_to_block(alternative);
    builder.ins().jump(merge, &[]);
    builder.switch_to_block(merge);

    assert_eq!(words.read(&mut builder, 0), literal);
    assert!(builder.block_params(merge).is_empty());
    assert!(!words.attempted_literal_write());
    let changed = builder.ins().iconst(types::I64, 42);
    words.write(&mut builder, 0, changed);
    assert!(words.attempted_literal_write());

    builder.ins().return_(&[]);
    builder.seal_all_blocks();
    builder.finalize();
}
