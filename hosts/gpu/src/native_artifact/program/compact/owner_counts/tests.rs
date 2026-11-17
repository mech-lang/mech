use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, StackSlotData, StackSlotKind};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::default_libcall_names;

#[repr(C)]
struct Owner {
    references: u64,
    other: u64,
}

#[derive(Default)]
struct Arena {
    // Each returned address must stay live across later arena growth.
    #[allow(
        clippy::vec_box,
        reason = "JIT code retains stable owner addresses across allocations"
    )]
    owners: Vec<Box<Owner>>,
    calls: usize,
    fail_at: usize,
}

#[repr(C)]
struct Backing {
    data: u64,
    owner_or_status: u64,
}

fn backing(arena: &mut Arena, capacity: u64, stride: u64, references: u64) -> Backing {
    arena.calls += 1;
    if arena.calls == arena.fail_at || stride == 0 {
        return Backing {
            data: 0,
            owner_or_status: 17,
        };
    }
    if capacity == 0 {
        return Backing {
            data: 1,
            owner_or_status: 0,
        };
    }
    let mut owner = Box::new(Owner {
        references,
        other: 0xabcd,
    });
    let address = (&mut *owner as *mut Owner) as u64;
    arena.owners.push(owner);
    Backing {
        data: 1,
        owner_or_status: address,
    }
}

extern "C" fn allocate(heap: *mut Arena, capacity: u64, stride: u64, _: u64, _: u64) -> Backing {
    // SAFETY: every JIT entry below receives a unique, live Arena.
    backing(unsafe { &mut *heap }, capacity, stride, 0)
}

extern "C" fn writable(
    heap: *mut Arena,
    _: u64,
    _: u64,
    capacity: u64,
    _: u64,
    _: u64,
    _: u64,
) -> Backing {
    // A writable result can be a reused owner with one live reference.
    backing(unsafe { &mut *heap }, capacity, 1, 1)
}

extern "C" fn touch(owner: *mut Owner, references: u64) {
    // SAFETY: only a successful nonempty backing reaches this helper.
    unsafe { (*owner).references = references };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Fresh,
    Retained,
    AliasingStore,
    NarrowStore,
    UnknownCall,
    IndirectCall,
    SecondAllocation,
    ZeroCapacity,
    ZeroStride,
    UnknownCapacity,
    Writable,
    SharedSuccess,
    EqualJoin,
    DifferentJoin,
    Loop,
    UnchangedLoop,
    StackStore,
    OtherField,
}

type Entry = unsafe extern "C" fn(*mut Arena, u64, u64, u64, u64) -> u64;

fn compile(shape: Shape, optimize: bool) -> (JITModule, Entry, usize) {
    let mut builder = JITBuilder::with_flags(
        &[
            ("opt_level", "speed"),
            ("enable_verifier", "true"),
            ("regalloc_checker", "true"),
        ],
        default_libcall_names(),
    )
    .unwrap();
    builder.symbol("mech_compact_allocate_backing", allocate as *const u8);
    builder.symbol(
        "mech_compact_writable_scalar_backing",
        writable as *const u8,
    );
    builder.symbol("unknown_owner_mutation", touch as *const u8);
    let mut module = JITModule::new(builder);
    let word = AbiParam::new(types::I64);
    let mut allocation_signature = module.make_signature();
    allocation_signature.params = vec![word; 5];
    allocation_signature.returns = vec![word; 2];
    let allocation_id = module
        .declare_function(
            "mech_compact_allocate_backing",
            Linkage::Import,
            &allocation_signature,
        )
        .unwrap();
    let mut writable_signature = allocation_signature.clone();
    writable_signature.params = vec![word; 7];
    let writable_id = module
        .declare_function(
            "mech_compact_writable_scalar_backing",
            Linkage::Import,
            &writable_signature,
        )
        .unwrap();
    let mut touch_signature = module.make_signature();
    touch_signature.params = vec![word; 2];
    let touch_id = module
        .declare_function("unknown_owner_mutation", Linkage::Import, &touch_signature)
        .unwrap();

    let mut context = module.make_context();
    context.func.signature.params = vec![word; 5];
    context.func.signature.returns = vec![word];
    let slot =
        context
            .func
            .create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let allocation = module.declare_func_in_func(allocation_id, &mut context.func);
    let writable = module.declare_func_in_func(writable_id, &mut context.func);
    let touch = module.declare_func_in_func(touch_id, &mut context.func);
    let indirect_touch = context.func.import_signature(touch_signature);
    let mut storage = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut storage);
    let entry = b.create_block();
    let accepted = b.create_block();
    let failed = b.create_block();
    let nonempty = b.create_block();
    let empty = b.create_block();
    b.append_block_params_for_function_params(entry);
    let failure = b.append_block_param(failed, types::I64);
    b.switch_to_block(entry);
    let [heap, selector, offset, steps, dynamic_capacity] = b.block_params(entry)[..] else {
        unreachable!()
    };
    let capacity = if shape == Shape::UnknownCapacity {
        dynamic_capacity
    } else {
        b.ins()
            .iconst(types::I64, if shape == Shape::ZeroCapacity { 0 } else { 8 })
    };
    let one = b.ins().iconst(types::I64, 1);
    let stride = if shape == Shape::ZeroStride {
        b.ins().iconst(types::I64, 0)
    } else {
        one
    };
    let call = if shape == Shape::Writable {
        b.ins()
            .call(writable, &[heap, one, one, capacity, one, capacity, one])
    } else {
        b.ins()
            .call(allocation, &[heap, capacity, stride, one, one])
    };
    let [data, owner] = b.inst_results(call)[..] else {
        unreachable!()
    };
    if shape == Shape::SharedSuccess {
        let admission = b.create_block();
        let bypass = b.create_block();
        let mutate = b.create_block();
        b.ins().brif(selector, bypass, &[], admission, &[]);
        b.switch_to_block(admission);
        b.ins().brif(data, accepted, &[], failed, &[owner.into()]);
        b.switch_to_block(bypass);
        b.ins().brif(data, mutate, &[], failed, &[owner.into()]);
        b.switch_to_block(mutate);
        let value = b.ins().iconst(types::I64, 5);
        b.ins().call(touch, &[owner, value]);
        b.ins().jump(accepted, &[]);
    } else {
        b.ins().brif(data, accepted, &[], failed, &[owner.into()]);
    }
    b.switch_to_block(failed);
    b.ins().return_(&[failure]);
    b.switch_to_block(accepted);
    b.ins().brif(owner, nonempty, &[], empty, &[]);
    b.switch_to_block(empty);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero]);
    b.switch_to_block(nonempty);

    match shape {
        Shape::Retained => {
            for _ in 0..2 {
                let count = b.ins().load(types::I64, MemFlags::new(), owner, 0);
                let next = b.ins().iadd_imm(count, 1);
                b.ins().store(MemFlags::new(), next, owner, 0);
            }
        }
        Shape::AliasingStore | Shape::NarrowStore => {
            let address = b.ins().iadd(owner, offset);
            let value = b.ins().iconst(types::I64, 9);
            if shape == Shape::NarrowStore {
                b.ins().istore32(MemFlags::new(), value, address, 0);
            } else {
                b.ins().store(MemFlags::new(), value, address, 0);
            }
        }
        Shape::UnknownCall | Shape::IndirectCall => {
            let value = b.ins().iconst(types::I64, 9);
            if shape == Shape::IndirectCall {
                let address = b.ins().func_addr(types::I64, touch);
                b.ins()
                    .call_indirect(indirect_touch, address, &[owner, value]);
            } else {
                b.ins().call(touch, &[owner, value]);
            }
        }
        Shape::SecondAllocation => {
            let call = b.ins().call(allocation, &[heap, capacity, one, one, one]);
            let [data, status] = b.inst_results(call)[..] else {
                unreachable!()
            };
            let next = b.create_block();
            b.ins().brif(data, next, &[], failed, &[status.into()]);
            b.switch_to_block(next);
        }
        Shape::EqualJoin | Shape::DifferentJoin => {
            let left = b.create_block();
            let right = b.create_block();
            let join = b.create_block();
            b.ins().brif(selector, left, &[], right, &[]);
            b.switch_to_block(left);
            b.ins().store(MemFlags::new(), one, owner, 0);
            b.ins().jump(join, &[]);
            b.switch_to_block(right);
            let value = b
                .ins()
                .iconst(types::I64, if shape == Shape::EqualJoin { 1 } else { 2 });
            b.ins().store(MemFlags::new(), value, owner, 0);
            b.ins().jump(join, &[]);
            b.switch_to_block(join);
        }
        Shape::Loop | Shape::UnchangedLoop => {
            let header = b.create_block();
            let latch = b.create_block();
            let done = b.create_block();
            let remaining = b.append_block_param(header, types::I64);
            b.ins().jump(header, &[steps.into()]);
            b.switch_to_block(header);
            let count = b.ins().load(types::I64, MemFlags::new(), owner, 0);
            b.ins().brif(remaining, latch, &[], done, &[]);
            b.switch_to_block(latch);
            if shape == Shape::Loop {
                let next = b.ins().iadd_imm(count, 1);
                b.ins().store(MemFlags::new(), next, owner, 0);
            }
            let rest = b.ins().iadd_imm(remaining, -1);
            b.ins().jump(header, &[rest.into()]);
            b.switch_to_block(done);
        }
        Shape::StackStore => {
            b.ins().stack_store(selector, slot, 0);
        }
        _ => {}
    }
    let count = b.ins().load(
        types::I64,
        MemFlags::new(),
        owner,
        if shape == Shape::OtherField { 8 } else { 0 },
    );
    b.ins().return_(&[count]);
    b.seal_all_blocks();
    b.finalize();
    context.verify(module.isa()).unwrap();
    let folded = if optimize {
        fold(&module, &mut context).1
    } else {
        0
    };
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "owner_count_control",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the finalized signature is exactly Entry and the module stays live.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry, folded)
}

fn compare(shape: Shape, expected_folds: usize) {
    let (_original, original, _) = compile(shape, false);
    let (_optimized, optimized, folds) = compile(shape, true);
    assert_eq!(folds, expected_folds, "{shape:?}");
    for fail_at in [0, 1, 2] {
        for selector in [0, 1] {
            for offset in [0, 8] {
                for steps in [0, 1, 3] {
                    for capacity in [0, 8] {
                        let mut a = Arena {
                            fail_at,
                            ..Arena::default()
                        };
                        let mut b = Arena {
                            fail_at,
                            ..Arena::default()
                        };
                        let before = unsafe { original(&mut a, selector, offset, steps, capacity) };
                        let after = unsafe { optimized(&mut b, selector, offset, steps, capacity) };
                        assert_eq!(
                            after, before,
                            "{shape:?}, failure {fail_at}, selector {selector}, offset {offset}, steps {steps}, capacity {capacity}"
                        );
                        assert_eq!(a.calls, b.calls, "allocation/failure order changed");
                        let snapshot = |a: &Arena| {
                            a.owners
                                .iter()
                                .map(|o| (o.references, o.other))
                                .collect::<Vec<_>>()
                        };
                        assert_eq!(
                            snapshot(&a),
                            snapshot(&b),
                            "{shape:?}: memory effects changed"
                        );
                        if fail_at == 1 {
                            assert_eq!(after, 17)
                        }
                        if fail_at == 0 {
                            match shape {
                                Shape::Fresh | Shape::SecondAllocation | Shape::UnchangedLoop => {
                                    assert_eq!(after, 0)
                                }
                                Shape::Retained => assert_eq!(after, 2),
                                Shape::Writable => assert_eq!(after, 1),
                                Shape::Loop => assert_eq!(after, steps),
                                Shape::OtherField => assert_eq!(after, 0xabcd),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn fresh_counts_and_exact_retains_keep_their_memory_effects() {
    compare(Shape::Fresh, 1);
    compare(Shape::Retained, 3);
    compare(Shape::SecondAllocation, 1);
}

#[test]
fn possible_aliases_and_unknown_calls_stop_count_propagation() {
    for shape in [
        Shape::AliasingStore,
        Shape::NarrowStore,
        Shape::UnknownCall,
        Shape::IndirectCall,
        Shape::StackStore,
    ] {
        compare(shape, 0);
    }
}

#[test]
fn nonempty_admission_does_not_generalize_to_reused_or_unknown_backings() {
    for shape in [
        Shape::ZeroCapacity,
        Shape::ZeroStride,
        Shape::UnknownCapacity,
        Shape::Writable,
        Shape::SharedSuccess,
    ] {
        compare(shape, 0);
    }
}

#[test]
fn all_join_paths_must_agree_and_cycles_must_not_assume_initial_counts() {
    compare(Shape::EqualJoin, 1);
    compare(Shape::DifferentJoin, 0);
    compare(Shape::Loop, 0);
    compare(Shape::UnchangedLoop, 0);
}

#[test]
fn only_the_reference_count_word_is_a_seed() {
    compare(Shape::OtherField, 0);
}
