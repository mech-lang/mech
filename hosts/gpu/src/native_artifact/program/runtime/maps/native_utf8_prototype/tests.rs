use super::*;
use crate::native_artifact::NativeValueSlot;
use cranelift_codegen::ir::{AbiParam, Opcode, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::snapshot::{MapEntryDraft, SnapshotValidationContext};
use mech_core::{
    CardinalitySpec, DimensionExpr, ManagedMemoryBudget, SchemaDraft, SchemaTableBuilder,
    ValueDataDraft as Data, ValueDraft,
};

const LARGE: u64 = 1_000_000_000_000;
static TEST_EPOCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Compiled {
    entry: unsafe extern "C" fn(*mut Frame) -> u64,
    instructions: usize,
    _module: cranelift_jit::JITModule,
}
impl Compiled {
    fn new() -> Self {
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let mut context = module.make_context();
        context
            .func
            .signature
            .params
            .push(AbiParam::new(types::I64));
        context
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));
        let function = module
            .declare_function("native_utf8_batch", Linkage::Local, &context.func.signature)
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let frame = b.block_params(entry)[0];
            emit::emit(&mut b, frame);
            b.seal_all_blocks();
            b.finalize();
        }
        let instructions = context
            .func
            .layout
            .blocks()
            .map(|block| context.func.layout.block_insts(block).count())
            .sum();
        assert!(instructions > 100, "the lookup body must be emitted");
        for block in context.func.layout.blocks() {
            for inst in context.func.layout.block_insts(block) {
                assert!(!matches!(
                    context.func.dfg.insts[inst].opcode(),
                    Opcode::Call
                        | Opcode::CallIndirect
                        | Opcode::ReturnCall
                        | Opcode::ReturnCallIndirect
                ));
            }
        }
        module.define_function(function, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this private emitted function has exactly the one-pointer
        // signature; the owner keeps its code alive through every invocation.
        let entry = unsafe { std::mem::transmute(module.get_finalized_function(function)) };
        Self {
            entry,
            instructions,
            _module: module,
        }
    }
    fn run(&self, frame: *mut Frame) -> u64 {
        // SAFETY: callers own the initialized private frame and all extents
        // for the synchronous call. Native code has no callback or escape.
        unsafe { (self.entry)(frame) }
    }
}
fn value(body: SchemaBody, data: Data) -> (NativeValueSlot, Value) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema: id,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    (
        NativeValueSlot {
            schema: id,
            key: value.schema_key(),
        },
        value,
    )
}
fn map(entries: &[(String, u64)]) -> (NativeValueSlot, Value) {
    value(
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        Data::Map(
            entries
                .iter()
                .map(|(key, value)| MapEntryDraft {
                    items: vec![Data::String(key.clone().into()), Data::U64(*value)]
                        .into_boxed_slice(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    )
}
fn matrix(keys: &[String], rows: u64, columns: u64) -> (NativeValueSlot, Value) {
    value(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: vec![
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        },
        Data::Matrix(
            keys.iter()
                .map(|key| Data::String(key.clone().into()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    )
}
fn admit(value: &Value, memory: &ManagedMemoryBudget) -> Value {
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.clone().into_memory_budget(&mut reservation).unwrap()
}
fn fixture(
    entries: &[(String, u64)],
    keys: &[String],
    rows: u64,
    columns: u64,
    memory: &ManagedMemoryBudget,
) -> ([NativeValueSlot; 2], [Value; 2]) {
    let (map_type, map) = map(entries);
    let (key_type, keys) = matrix(keys, rows, columns);
    let map = admit(&map, memory);
    let keys = admit(&keys, memory);
    ([map_type, key_type], [map, keys])
}
fn arena<'a>(
    types: &'a [NativeValueSlot],
    values: &[Value; 2],
    memory: &ManagedMemoryBudget,
    work: &mut NativeWorkMeter,
) -> NativeValueArena<'a> {
    let mut arena = NativeValueArena::new(types, 1_000_000, memory, work).unwrap();
    arena.put(0, &values[0], work).unwrap();
    arena.put(1, &values[1], work).unwrap();
    arena
}
fn expected(
    entries: &[(String, u64)],
    keys: &[String],
    rows: usize,
    columns: usize,
) -> Vec<ResultCell> {
    (0..keys.len())
        .map(|ordinal| {
            let key = &keys[(ordinal % rows) * columns + ordinal / rows];
            let found = entries.iter().find(|(candidate, _)| candidate == key);
            ResultCell {
                value: found.map_or(0, |(_, value)| *value),
                found: u64::from(found.is_some()),
            }
        })
        .collect()
}
#[test]
fn native_utf8_query_batch_emits_hash_probe_equality_and_preserves_current_roots() {
    let _epoch_serial = TEST_EPOCH_LOCK.lock().unwrap();
    let compiled = Compiled::new();
    let entries = vec![
        ("".into(), 0),
        ("λ\0🦀".into(), u64::MAX),
        ("prefix".into(), 17),
        ("prefixλ".into(), 19),
        ("x".repeat(4097), 23),
    ];
    let keys = vec![
        "λ\0🦀".into(),
        "missing".into(),
        "prefix".into(),
        "".into(),
        "x".repeat(4097),
        "prefixλ".into(),
    ];
    let memory = ManagedMemoryBudget::new(32_000_000);
    let mut work = NativeWorkMeter::new(LARGE).unwrap();
    let (types, aliases) = fixture(&entries, &keys, 2, 3, &memory);
    let arena = arena(&types, &aliases, &memory, &mut work);
    let mut loan = Loan::new(&arena, 0, 1, &mut work, &memory)
        .unwrap()
        .unwrap();
    let remaining = loan.work.unscoped_remaining().unwrap();
    let used = loan.work.used();
    let compute = keys
        .iter()
        .map(|key| {
            price(
                loan.frame.capacity,
                loan.frame.stored_bytes,
                key.len() as u64,
            )
            .unwrap()
            .0
        })
        .sum::<u64>();
    let comparison = keys
        .iter()
        .map(|key| {
            price(
                loan.frame.capacity,
                loan.frame.stored_bytes,
                key.len() as u64,
            )
            .unwrap()
            .1
        })
        .sum::<u64>();
    assert_eq!(
        loan.run(&arena, &mut work, |frame| compiled.run(frame))
            .unwrap(),
        OK
    );
    assert_eq!(&*loan.output, expected(&entries, &keys, 2, 3));
    assert_eq!(
        loan.work.used(),
        (used.0 + 40 + compute, used.1 + comparison)
    );
    assert_eq!(
        loan.work.unscoped_remaining(),
        Some((remaining.0 - 40 - compute, remaining.1 - comparison))
    );
    assert_eq!(loan.frame.next, keys.len() as u64);
    assert!(
        arena
            .copy_current(0, &mut work)
            .unwrap()
            .shares_frozen_storage(&aliases[0])
    );
    assert!(
        arena
            .copy_current(1, &mut work)
            .unwrap()
            .shares_frozen_storage(&aliases[1])
    );
    assert_eq!(loan.slots.len() as u64, loan.frame.capacity);
    assert_eq!(loan.cells.len(), keys.len());
    eprintln!(
        "native_utf8_prototype instructions={} hot_calls=0 queries={} compute={} comparison={}",
        compiled.instructions,
        keys.len(),
        compute,
        comparison
    );
    drop((loan, arena, aliases, work));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}
#[test]
fn native_utf8_probe_handles_collision_chains_and_empty_maps() {
    let _epoch_serial = TEST_EPOCH_LOCK.lock().unwrap();
    let compiled = Compiled::new();
    let mut colliding = Vec::new();
    for n in 0..100_000 {
        let key = format!("clé-{n}🦀");
        if hash(key.as_bytes()) & 31 == 7 {
            colliding.push((key, n));
            if colliding.len() == 8 {
                break;
            }
        }
    }
    assert_eq!(colliding.len(), 8);
    let mut keys = colliding
        .iter()
        .rev()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    keys.push("not present".into());
    for entries in [&colliding[..], &[]] {
        let memory = ManagedMemoryBudget::new(8_000_000);
        let mut work = NativeWorkMeter::new(LARGE).unwrap();
        let (types, aliases) = fixture(entries, &keys, 1, keys.len() as u64, &memory);
        let arena = arena(&types, &aliases, &memory, &mut work);
        let mut loan = Loan::new(&arena, 0, 1, &mut work, &memory)
            .unwrap()
            .unwrap();
        assert_eq!(
            loan.run(&arena, &mut work, |frame| compiled.run(frame))
                .unwrap(),
            OK
        );
        assert_eq!(&*loan.output, expected(entries, &keys, 1, keys.len()));
        drop((loan, arena, aliases, work));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn native_utf8_finite_native_debits_resume_without_partial_current_query_publication() {
    let _epoch_serial = TEST_EPOCH_LOCK.lock().unwrap();
    let compiled = Compiled::new();
    let entries = vec![("a".into(), 4), ("🦀".into(), 8)];
    let keys = vec!["a".into(), "🦀".into(), "missing".into()];
    let memory = ManagedMemoryBudget::new(8_000_000);
    let mut work = NativeWorkMeter::new(LARGE).unwrap();
    let (types, aliases) = fixture(&entries, &keys, 1, 3, &memory);
    let arena = arena(&types, &aliases, &memory, &mut work);
    let mut loan = Loan::new(&arena, 0, 1, &mut work, &memory)
        .unwrap()
        .unwrap();
    let first = price(
        loan.frame.capacity,
        loan.frame.stored_bytes,
        keys[0].len() as u64,
    )
    .unwrap();
    // This directly probes generated finite debit semantics, not an authority
    // to replenish a live invocation. Each trial begins with fresh outputs.
    for (compute, comparison, completed) in [
        (first.0 - 1, LARGE, 0),
        (LARGE, first.1 - 1, 0),
        (first.0, first.1, 1),
    ] {
        loan.output.fill(ResultCell::default());
        loan.frame.next = 0;
        loan.frame.issued = loan.frame.epoch;
        loan.frame.fuel = compute;
        loan.frame.comparison = comparison;
        assert_eq!(compiled.run(&mut *loan.frame), COLD);
        assert_eq!(loan.frame.next, completed);
        assert_eq!(
            &loan.output[completed as usize..],
            &vec![ResultCell::default(); keys.len() - completed as usize]
        );
        if completed == 1 {
            assert_eq!(loan.output[0], ResultCell { value: 4, found: 1 });
            assert_eq!((loan.frame.fuel, loan.frame.comparison), (0, 0))
        }
    }
    loan.output.fill(ResultCell::default());
    loan.frame.next = 0;
    assert_eq!(
        loan.run(&arena, &mut work, |frame| compiled.run(frame))
            .unwrap(),
        OK
    );
    assert_eq!(&*loan.output, expected(&entries, &keys, 1, 3));
    drop((loan, arena, aliases, work));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}
#[test]
fn native_utf8_bad_epoch_geometry_and_current_owner_decline_before_native_payload_read() {
    let _epoch_serial = TEST_EPOCH_LOCK.lock().unwrap();
    let compiled = Compiled::new();
    let entries = vec![("a".into(), 4)];
    let keys = vec!["a".into()];
    let memory = ManagedMemoryBudget::new(8_000_000);
    let mut work = NativeWorkMeter::new(LARGE).unwrap();
    let (types, aliases) = fixture(&entries, &keys, 1, 1, &memory);
    let mut arena = arena(&types, &aliases, &memory, &mut work);
    let mut loan = Loan::new(&arena, 0, 1, &mut work, &memory)
        .unwrap()
        .unwrap();
    loan.frame.issued = loan.frame.epoch + 1;
    loan.frame.fuel = LARGE;
    loan.frame.comparison = LARGE;
    assert_eq!(compiled.run(&mut *loan.frame), COLD);
    assert_eq!(&*loan.output, [ResultCell::default()]);
    loan.frame.issued = loan.frame.epoch;
    loan.frame.rows = 0;
    assert_eq!(compiled.run(&mut *loan.frame), COLD);
    assert_eq!(&*loan.output, [ResultCell::default()]);
    loan.frame.rows = 1;
    loan.frame.capacity = 3;
    assert_eq!(compiled.run(&mut *loan.frame), COLD);
    assert_eq!(&*loan.output, [ResultCell::default()]);
    loan.frame.capacity = loan.slots.len() as u64;
    let mut foreign_work = NativeWorkMeter::new(LARGE).unwrap();
    assert!(
        loan.run(&arena, &mut foreign_work, |_| panic!(
            "foreign ledger entered native code"
        ))
        .is_err()
    );
    let (foreign_types, foreign_aliases) = fixture(&entries, &keys, 1, 1, &memory);
    let foreign_arena = self::arena(&foreign_types, &foreign_aliases, &memory, &mut work);
    assert!(
        loan.run(&foreign_arena, &mut work, |_| panic!(
            "foreign arena entered native code"
        ))
        .is_err()
    );
    arena.put(1, &aliases[1], &mut work).unwrap();
    assert!(
        loan.run(&arena, &mut work, |_| panic!(
            "rebound generation entered native code"
        ))
        .is_err()
    );
    drop((
        loan,
        arena,
        aliases,
        foreign_arena,
        foreign_aliases,
        work,
        foreign_work,
    ));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

#[test]
fn native_utf8_same_account_exact_and_one_short_limits_do_not_renew_or_refund() {
    let _epoch_serial = TEST_EPOCH_LOCK.lock().unwrap();
    let compiled = Compiled::new();
    let entries = vec![("a".into(), 4), ("🦀".into(), 8)];
    let keys = vec!["a".into(), "🦀".into(), "missing".into()];
    let run_case = |compute_limit: u64, comparison_limit: u64| {
        let memory = ManagedMemoryBudget::new(8_000_000);
        let mut work = NativeWorkMeter::new(compute_limit).unwrap();
        let (types, aliases) = fixture(&entries, &keys, 1, 3, &memory);
        let arena = arena(&types, &aliases, &memory, &mut work);
        let mut loan = Loan::new(&arena, 0, 1, &mut work, &memory)
            .unwrap()
            .unwrap();
        let prices = keys
            .iter()
            .map(|key| {
                price(
                    loan.frame.capacity,
                    loan.frame.stored_bytes,
                    key.len() as u64,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let total = prices
            .iter()
            .fold((0, 0), |sum, price| (sum.0 + price.0, sum.1 + price.1));
        let before = loan.work.used();
        // Compute authority was finite from WorkMeter construction, not
        // secretly narrowed behind its back. Tighten only comparison on this
        // existing shared account, then lend precisely its current allowance.
        loan.work
            .restrict_and_charge(work.limit(), comparison_limit, 0, 0)
            .unwrap();
        let status = loan
            .run(&arena, &mut work, |frame| compiled.run(frame))
            .unwrap();
        let complete = loan.frame.next as usize;
        let expected = expected(&entries, &keys, 1, 3);
        assert_eq!(&loan.output[..complete], &expected[..complete]);
        assert!(
            loan.output[complete..]
                .iter()
                .all(|cell| *cell == ResultCell::default())
        );
        let paid = prices[..complete]
            .iter()
            .fold((0, 0), |sum, price| (sum.0 + price.0, sum.1 + price.1));
        assert_eq!(
            loan.work.used(),
            (before.0 + 40 + paid.0, before.1 + paid.1)
        );
        assert_eq!(
            loan.work.unscoped_remaining(),
            Some((
                compute_limit - before.0 - 40 - paid.0,
                comparison_limit - before.1 - paid.1
            ))
        );
        drop((loan, arena, aliases, work));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
        (before, total, status, complete)
    };
    // Fixture/nonce issuance is serialized only in test scaffolding so the
    // calibration and new finite-ledger trials have the same paid setup.
    let (before, total, status, complete) = run_case(LARGE, LARGE);
    assert_eq!((status, complete), (OK, 3));
    for dimension in [0, 1] {
        for short in [0, 1] {
            let compute = before.0 + 40 + total.0 - if dimension == 0 { short } else { 0 };
            let comparison = before.1 + total.1 - if dimension == 1 { short } else { 0 };
            let (again, _, status, complete) = run_case(compute, comparison);
            assert_eq!(again, before, "identical current roots retain paid setup");
            assert_eq!(
                (status, complete),
                if short == 0 { (OK, 3) } else { (COLD, 2) }
            );
        }
    }
}
