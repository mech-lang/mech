//! Observe the actual generated program and unchanged production helper.
use super::super::*;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug)]
struct Row {
    snapshot_nodes: u64,
    nodes: u64,
    bytes: u64,
    work: u64,
    activation: bool,
    builders: u8,
    wrong_site_denied: bool,
}
thread_local! {
    static ROWS: RefCell<Vec<Result<Row, String>>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "C" fn observe(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    if matches!(operation, LIBRARY | MOVE | COLLECTION) {
        // SAFETY: native code is suspended at the real helper boundary. The
        // exclusive diagnostic borrow ends before delegating unchanged args.
        let ctx = unsafe { &mut *context.cast::<Context<'_, '_>>() };
        let memory = ctx.library.memory_budget();
        let snapshots = memory.retained_snapshot_nodes();
        let before = ctx.library.used().0;
        let wrong_site_denied = ctx.retained_caller_geometry(site as usize + 1).is_err();
        let outcome = ctx.retained_caller_geometry(site as usize);
        if matches!(outcome, Ok(Some(_))) {
            // Independent scan checks the maintained total against all live
            // cache owners, including selected String-span index mutations.
            let mut scanned = 0u64;
            for entry in &ctx.call_caches.entries {
                if let session::CallCacheEntry::Repeated(cache, tracked) = entry {
                    let actual = cache
                        .retained_metadata_demand(&ctx.library)
                        .unwrap()
                        .retained_nodes;
                    assert_eq!(*tracked, actual);
                    scanned += actual;
                }
            }
            assert_eq!(
                ctx.call_caches.caller_retained_nodes(&ctx.library).unwrap(),
                session::CallCacheLane::metadata_bytes(ctx.call_caches.entries.len()).unwrap()
                    + scanned
            );
        }
        if let Some(row) = match outcome {
            Ok(Some(geometry)) => Some(Ok(Row {
                snapshot_nodes: snapshots,
                nodes: geometry.live_nodes,
                bytes: geometry.live_bytes,
                work: ctx.library.used().0 - before,
                activation: ctx.activation_phase,
                builders: ctx
                    .collections
                    .as_ref()
                    .map_or(0, |state| state.test_builder_kinds()),
                wrong_site_denied,
            })),
            Ok(None) => None,
            Err(error) => Some(Err(format!("site {site} op {operation}: {error:?}"))),
        } {
            ROWS.with(|rows| rows.borrow_mut().push(row));
        }
    }
    // SAFETY: the diagnostic borrow has ended; delegate the actual callback
    // without replacing any source/body/library semantics or word operands.
    unsafe { helper(context, site, operation, first, second, third, result) }
}

fn run(
    jit: &crate::native_artifact::JitProgramArtifact,
    outputs: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Vec<Row>,
) {
    ROWS.with(|rows| rows.borrow_mut().clear());
    let entry = jit.test_checked_entry();
    let result = execute(
        jit.descriptor(),
        |raw| {
            // SAFETY: same owned frame and actual JIT. Restore the helper before
            // production return checks inspect immutable transport fields.
            let previous = unsafe { (*raw).helper };
            unsafe { (*raw).helper = observe };
            let status = unsafe { entry(raw) };
            unsafe { (*raw).helper = previous };
            status
        },
        &[],
        outputs,
        tests::limits(),
        memory,
    );
    let rows = ROWS
        .with(|rows| core::mem::take(&mut *rows.borrow_mut()))
        .into_iter()
        .map(Result::unwrap)
        .collect();
    (result, rows)
}

fn variants(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let original = tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

fn external_matrix(count: usize, memory: &ManagedMemoryBudget) -> Value {
    let mut builder = mech_core::SchemaTableBuilder::new();
    let handle = builder
        .insert(
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body: mech_core::SchemaBody::Matrix {
                    element: Box::new(mech_core::SchemaBody::String),
                    dimensions: vec![mech_core::DimensionExpr::Constant(count as u64)]
                        .into_boxed_slice(),
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let schema = built.resolve(handle).unwrap();
    let schemas = std::sync::Arc::new(built.table);
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            (0..count)
                .map(|_| ValueDataDraft::String("é".into()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
    .unwrap();
    let mut claim = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut claim).unwrap()
}

#[test]
fn generated_caller_geometry_includes_prior_roots_without_payload_dependent_work() {
    let source = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec");
    for artifact in variants(source) {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let mut first_price = None;
        for count in [1usize, 4096] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let prior = external_matrix(count, &memory);
            let prior_nodes = memory.retained_snapshot_nodes();
            let oracle = prior
                .retained_footprint(&prior.schemas().unwrap())
                .unwrap()
                .node_count;
            assert_eq!(prior_nodes, oracle);
            assert!(prior_nodes >= count as u64);
            let identity = prior.frozen_storage_identity();
            let mut outputs = vec![Some(prior)];
            let (result, rows) = run(&jit, &mut outputs, &memory);
            // The deliberately wrong old-output type remains unexamined until
            // closed activation completes. Its census must already be present.
            assert!(matches!(
                result,
                Err(NativeProgramExecutionError::Value(
                    NativeValueError::TypeMismatch
                ))
            ));
            let first = rows.first().expect("actual activation helper");
            assert!(first.activation && first.wrong_site_denied);
            assert!(first.snapshot_nodes >= prior_nodes && first.nodes >= first.snapshot_nodes);
            if let Some(price) = first_price {
                assert_eq!(price, first.work);
            } else {
                first_price = Some(first.work);
            }
            assert!(rows.iter().all(|row| row.nodes >= row.snapshot_nodes
                && row.bytes > 0
                && row.wrong_site_denied));
            assert!(identity.matches(outputs[0].as_ref().unwrap()));
            drop((outputs, identity));
            assert_eq!(memory.retained_snapshot_nodes(), 0);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn generated_caller_geometry_covers_live_collection_builder_families() {
    let prelude = "+> sequence\n+> text\n+> map\nwords := [\"é\", \"ββ\", \"é\"]\n";
    let mut kinds = 0;
    for (expression, expected_kind) in [
        ("[sequence/length(word) | word <- words]", 1),
        ("text/join([word + \"!\" | word <- words], \",\")", 2),
        ("map/histogram([word + \"!\" | word <- words])", 4),
        (
            "hist := map/histogram([word + \"!\" | word <- words])\nquery := \"é!\"\nkeys<{string}> := map/keys(hist)\n(query ∈ keys)? | true => hist{query} | false => 0u64.",
            8,
        ),
    ] {
        for artifact in variants(&format!("{prelude}{expression}\n")) {
            let jit = crate::native_artifact::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None; artifact.outputs().len()];
            let (result, rows) = run(&jit, &mut outputs, &memory);
            result.unwrap();
            let node_limit = tests::limits().library.max_retained_nodes.unwrap();
            assert!(
                rows.iter().all(|row| row.nodes <= node_limit),
                "small live callers must fit the unchanged node cap {node_limit}: {rows:?}"
            );
            let seen = rows.iter().fold(0, |bits, row| bits | row.builders);
            assert!(
                seen & expected_kind != 0,
                "missing builder {expected_kind} for {expression}: {rows:?}"
            );
            kinds |= seen;
            let output = outputs.last().unwrap().as_ref().unwrap();
            match expected_kind {
                1 => {
                    let ValueData::Matrix(matrix) = output.data() else {
                        panic!("matrix");
                    };
                    let mech_core::snapshot::SequenceView::U64(values) = matrix.elements() else {
                        panic!("u64");
                    };
                    assert_eq!(values, &[1, 2, 1]);
                }
                2 => assert!(
                    matches!(output.data(), ValueData::String(text) if text.as_ref() == "é!,ββ!,é!")
                ),
                4 => {
                    let ValueData::Map(map) = output.data() else {
                        panic!("map");
                    };
                    assert_eq!(map.entries().len(), 2);
                    assert!(matches!(map.entries()[0].value(), ValueData::U64(2)));
                    assert!(matches!(map.entries()[1].value(), ValueData::U64(1)));
                }
                8 => assert!(matches!(output.data(), ValueData::U64(2))),
                _ => unreachable!(),
            }
            drop(outputs);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    assert_eq!(kinds & 15, 15);
}
