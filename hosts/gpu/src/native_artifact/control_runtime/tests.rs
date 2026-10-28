use crate::native_artifact::JitMatchArtifact;
use crate::native_artifact::control_runtime::*;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn descriptor() -> NativeMatchDescriptor {
    let parsed = parse_canonical_document(
        TextSnapshot::new(
            DocumentId(0x4d564953),
            Revision(1),
            "(flag<bool>? | true => value<u64> + 1u64 | false => 0u64)\n",
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let catalog = mech_stdlib::source_catalog();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let plan =
        mech_engine::native_artifact::lower_scalar_match_artifact(&artifact, &catalog).unwrap();
    JitMatchArtifact::compile(&plan)
        .unwrap()
        .descriptor()
        .clone()
}
#[test]
fn invalid_cfg_visit_is_rejected_without_publication() {
    let descriptor = descriptor();
    let inputs = descriptor
        .inputs()
        .iter()
        .map(|ty| match ty.kind() {
            NativeScalarKind::Bool => NativeScalarValue::Bool(true),
            NativeScalarKind::U8 => NativeScalarValue::U8(8),
            NativeScalarKind::U16 => NativeScalarValue::U16(8),
            NativeScalarKind::U32 => NativeScalarValue::U32(8),
            NativeScalarKind::U64 => NativeScalarValue::U64(8),
            NativeScalarKind::I64 => NativeScalarValue::I64(8),
            NativeScalarKind::F64 => {
                NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_f64(8.0))
            }
        })
        .collect::<Vec<_>>();
    let mut outputs = vec![NativeScalarValue::U64(77); descriptor.outputs().len()];
    let error = execute(
        &descriptor,
        |frame| {
            // SAFETY: the injected entry uses only the live callback and
            // context supplied by execute; it never accesses a word arena.
            unsafe { ((*frame).admit_next)((*frame).context, u64::MAX) }
        },
        &inputs,
        &mut outputs,
        NativeExecutionLimits::default(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        NativeMatchExecutionError::Visit {
            previous: None,
            actual: u64::MAX
        }
    ));
    assert!(
        outputs
            .iter()
            .all(|value| *value == NativeScalarValue::U64(77))
    );
    let mut context = Context {
        descriptor: &descriptor,
        previous: None,
        completed: false,
        work: NativeWorkMeter::new(u64::MAX).unwrap(),
        visits: 0,
        error: None,
    };
    context.visit(descriptor.entry as u64).unwrap();
    let before = context.work.used();
    assert!(matches!(
        context.visit(descriptor.entry as u64),
        Err(NativeMatchExecutionError::Visit { .. })
    ));
    assert_eq!(context.work.used(), before);
    assert_eq!(context.visits, 1);
}
#[test]
fn setup_and_memory_denial_precede_injected_entry_and_preserve_outputs() {
    let descriptor = descriptor();
    let inputs = descriptor
        .inputs()
        .iter()
        .map(|ty| match ty.kind() {
            NativeScalarKind::Bool => NativeScalarValue::Bool(true),
            NativeScalarKind::U8 => NativeScalarValue::U8(8),
            NativeScalarKind::U16 => NativeScalarValue::U16(8),
            NativeScalarKind::U32 => NativeScalarValue::U32(8),
            NativeScalarKind::U64 => NativeScalarValue::U64(8),
            NativeScalarKind::I64 => NativeScalarValue::I64(8),
            NativeScalarKind::F64 => {
                NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_f64(8.0))
            }
        })
        .collect::<Vec<_>>();
    for (limits, budget) in [
        (
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
            None,
        ),
        (
            NativeExecutionLimits::default(),
            Some(ManagedMemoryBudget::new(0)),
        ),
    ] {
        let entered = std::cell::Cell::new(false);
        let mut outputs = vec![NativeScalarValue::U64(77); descriptor.outputs().len()];
        assert!(
            execute(
                &descriptor,
                |_| {
                    entered.set(true);
                    STATUS_OK
                },
                &inputs,
                &mut outputs,
                limits,
                budget.as_ref()
            )
            .is_err()
        );
        assert!(!entered.get());
        assert!(
            outputs
                .iter()
                .all(|value| *value == NativeScalarValue::U64(77))
        );
    }
}
