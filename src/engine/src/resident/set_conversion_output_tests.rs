//! Observe the real prepared kernel estimate before admission; no fake plan.
use crate::native_artifact::{
    NativeCollectionInvocationError, NativeCollectionPrimitive, NativeIterationAction,
    NativeLibraryWorkBudget, lower_native_program_artifact, prepare_native_program_library_call,
};
use mech_core::snapshot::{CanonicalKeyValue, OptionDraft, SnapshotValidationContext, ValueDraft};
use mech_core::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::{cell::Cell, mem::size_of, sync::Arc};

#[path = "set_conversion_shape_output_tests.rs"]
mod shape;

#[derive(Clone, Copy)]
struct Seen {
    bytes: u64,
    wrappers: u64,
    calls: u64,
    dense: bool,
}
thread_local! { static SEEN: Cell<Option<Seen>> = const { Cell::new(None) }; }
pub(super) fn record(bytes: u64, wrappers: u64, dense: bool) {
    SEEN.with(|slot| {
        let calls = slot.get().map_or(1, |seen| seen.calls.saturating_add(1));
        slot.set(Some(Seen {
            bytes,
            wrappers,
            calls,
            dense,
        }));
    });
}
fn reset() {
    SEEN.with(|slot| slot.set(None));
}
fn seen(wrappers: u64, dense: bool) -> Seen {
    let seen = SEEN
        .with(Cell::get)
        .expect("actual maintained cost::prepare must be reached");
    assert_eq!(seen.calls, 1);
    assert_eq!(seen.wrappers, wrappers);
    assert_eq!(seen.dense, dense);
    seen
}
fn limits() -> MemoryBudgetLimits {
    let mut limits = TargetMemoryProfile::current_resident_cpu().unwrap().limits;
    // Existing finite native qualification policy; no default/product change.
    limits.max_compute_work = Some(134_217_728);
    limits.max_comparison_work = Some(1_000_000);
    limits.max_temporary_bytes = Some(128 * 1024 * 1024);
    // Include the four new wrappers' recursive schema evidence in this
    // explicit finite test policy; the production default stays unchanged.
    limits.max_retained_nodes = Some(131_072);
    limits
}

#[test]
fn new_option_headers_are_in_actual_prepared_output_estimate_before_allocation() {
    // An already-optional source selects ONE ordinary recursive convert/set:
    // OptionPayload then four NEW OptionPresent boxes. A bare Matrix source
    // instead gets separate frontend option/some operations, not this plan.
    let source = "<maybe0> := <{u8}?>\n<maybe1> := <maybe0?>\n<maybe2> := <maybe1?>\n<maybe3> := <maybe2?>\nunique<maybe3?> := values<[u8]?>\nunique\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4f5054), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let mut builder = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let original = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive() == NativeCollectionPrimitive::MatrixToSetConversion =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let conversion = super::conversion_plan(&template.inputs()[0], template.output()).unwrap();
        assert!(matches!(&conversion.step, ConversionStep::OptionPayload(_)));
        assert_eq!(conversion_option_present_depth(&conversion.step), 4);
        let prepared = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let input = &artifact.inputs()[0];
        let schema = artifact.schemas().get(input.schema).unwrap();
        let actual_body = SchemaBody::Option(Box::new(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                .into_boxed_slice(),
        }));
        let shape =
            shape_for_schema_components(schema, &[(schema.body(), actual_body)], None).unwrap();
        let value = ValueDraft {
            schema: input.schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data: ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(ValueDataDraft::Matrix(
                    vec![ValueDataDraft::U8(7)].into_boxed_slice(),
                ))),
            }),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        assert_eq!(
            conversion_draft_population(value.data(), |_| Ok::<(), ()>(())).unwrap(),
            3
        );
        assert_eq!(schema.dimension_parameters().len(), 2);
        assert_eq!(template.output().dimension_parameters().len(), 1);
        let input_bytes = value
            .retained_footprint(artifact.schemas())
            .unwrap()
            .retained_bytes;
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let input = vec![Some(value.into_memory_budget(&mut reservation).unwrap())];
        drop(reservation);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let mut output = None;
        reset();
        prepared
            .invoke_current(&input, &mut output, limits(), &work)
            .unwrap();
        let estimate = seen(4, false).bytes;
        let actual = output
            .as_ref()
            .unwrap()
            .retained_footprint(artifact.schemas())
            .unwrap()
            .retained_bytes;
        assert!(
            estimate >= actual,
            "actual output {actual} exceeds pre-admitted estimate {estimate}"
        );
        let expected = input_bytes.max(3 * 32)
            + (size_of::<ValueData>() as u64).saturating_sub(size_of::<u8>() as u64)
            + 4 * size_of::<ValueData>() as u64
            + size_of::<CanonicalKeyValue>() as u64;
        assert_eq!(
            estimate, expected,
            "actual optional source plus four NEW boxes, packed growth and key carrier"
        );
        let full_reference = output.as_ref().unwrap().clone();

        // Produce a genuinely smaller prior with the SAME prepared call and
        // an actually absent optional input. An existing output reserves its
        // Option<Value> carrier plus full retention before kernel admission;
        // the full result's eight spare tape bytes cannot fund that carrier.
        let absent = ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: input[0]
                .as_ref()
                .unwrap()
                .shape()
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
            data: ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None,
            }),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut reservation = memory
            .reserve_capacity(absent.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let absent_input = vec![Some(absent.into_memory_budget(&mut reservation).unwrap())];
        drop(reservation);
        reset();
        prepared
            .invoke_current(&absent_input, &mut output, limits(), &work)
            .unwrap();
        seen(0, false);
        assert_eq!(
            output.as_ref().unwrap().canonical_data_draft().unwrap(),
            ValueDataDraft::Option(OptionDraft {
                present: false,
                value: None
            })
        );
        let prior = output.as_ref().unwrap().clone();
        let prior_envelope = prior
            .retained_footprint(artifact.schemas())
            .unwrap()
            .retained_bytes
            .checked_add(size_of::<Option<Value>>() as u64)
            .unwrap();
        assert!(
            prior_envelope <= estimate - 1,
            "real smaller prior plus its actual carrier must fit one-short control"
        );

        // Exact SAME per-invocation OutputBytes policy. The estimate observer
        // proves the one-short refusal is at the maintained kernel admission,
        // not an earlier prefix or a later post-allocation owner safeguard.
        let mut short = limits();
        short.max_output_bytes = Some(estimate - 1);
        let before = memory.used_bytes();
        reset();
        let error = prepared
            .invoke_current(&input, &mut output, short, &work)
            .unwrap_err();
        assert_eq!(seen(4, false).bytes, estimate);
        assert!(
            matches!(error, NativeCollectionInvocationError::Kernel(
            ResidentKernelError::ResourceLimit { dimension: MemoryBudgetDimension::OutputBytes,
                required, limit, .. }) if required == estimate && limit == estimate - 1),
            "{error:?}"
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), before);

        let mut exact = limits();
        exact.max_output_bytes = Some(estimate);
        reset();
        prepared
            .invoke_current(&input, &mut output, exact, &work)
            .unwrap();
        assert_eq!(seen(4, false).bytes, estimate);
        assert!(
            output
                .as_ref()
                .unwrap()
                .snapshot_eq(artifact.schemas(), &full_reference, artifact.schemas())
                .unwrap()
        );
        reset();
        prepared
            .invoke_current(&input, &mut output, limits(), &work)
            .unwrap();
        assert_eq!(seen(4, false).bytes, estimate);
        drop((output, prior, full_reference, input, absent_input, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
