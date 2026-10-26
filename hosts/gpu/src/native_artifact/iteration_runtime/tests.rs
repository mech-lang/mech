use crate::native_artifact::iteration_runtime::*;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn plan(count: u64) -> mech_engine::native_artifact::NativeIterationArtifactPlan {
    let source =
        format!("identity(x<u64>) => (next<u64>) := next := x.\nseed<u64> |{count}> identity\n");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x49545254), Revision(1), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    mech_engine::native_artifact::lower_scalar_iteration_artifact(&artifact, &catalog).unwrap()
}
#[test]
fn callback_progress_rejects_early_exit_and_body_after_exact_count() {
    for count in [0, 2] {
        let plan = plan(count);
        let descriptor = NativeIterationDescriptor::from_plan(&plan, 32, 1).unwrap();
        let iteration = descriptor.iterations[0].clone();
        let mut progress = [0; 2];
        let mut control_state = ControlState::new(NativeWorkMeter::new(100_000).unwrap());
        let mut context = ControlView {
            state: &mut control_state,
            descriptor: &descriptor,
            progress: &mut progress,
            error: None,
        };
        context.visit(descriptor.entry as u64).unwrap();
        context.visit(iteration.initialize() as u64).unwrap();
        context.visit(iteration.header() as u64).unwrap();
        let denied = if count == 0 {
            iteration.body()
        } else {
            iteration.exit()
        };
        let before = (
            context.state.work.used(),
            context.state.visits,
            context.progress[0],
        );
        assert!(matches!(
            context.visit(denied as u64),
            Err(NativeIterationExecutionError::Visit { .. })
        ));
        assert_eq!(
            (
                context.state.work.used(),
                context.state.visits,
                context.progress[0]
            ),
            before
        );
        // Identity's body has one no-op block. No hidden execution or
        // inferred effect is used to move this private progress counter.
        for _ in 0..count {
            context.visit(iteration.body() as u64).unwrap();
            context.visit(iteration.latch() as u64).unwrap();
            context.visit(iteration.header() as u64).unwrap();
        }
        context.visit(iteration.exit() as u64).unwrap();
        assert_eq!(context.state.current, None);
        assert_eq!(&*context.progress, &[0, 0]);
        assert!(context.visit(iteration.header() as u64).is_err());
    }
}
#[test]
fn setup_transport_and_shared_budget_deny_before_input_scan_or_entry() {
    let plan = plan(2);
    let descriptor = NativeIterationDescriptor::from_plan(&plan, 32, 1).unwrap();
    for (limits, shared, expected) in [
        (
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
            None,
            "work",
        ),
        (
            NativeExecutionLimits {
                max_transport_bytes: 0,
                ..NativeExecutionLimits::default()
            },
            None,
            "transport",
        ),
        (
            NativeExecutionLimits::default(),
            Some(ManagedMemoryBudget::new(0)),
            "memory",
        ),
    ] {
        let called = std::cell::Cell::new(false);
        let mut output = [NativeScalarValue::U64(99)];
        // Wrong input type would fail the payload-type pass; denial must
        // win first without invoking the injected entry.
        let error = execute(
            &descriptor,
            |_| {
                called.set(true);
                STATUS_ABI
            },
            &[NativeScalarValue::Bool(false)],
            &mut output,
            limits,
            shared.as_ref(),
        )
        .unwrap_err();
        assert!(!called.get());
        assert_eq!(output, [NativeScalarValue::U64(99)]);
        match expected {
            "work" => assert!(matches!(
                error,
                NativeIterationExecutionError::Work { origin: None, .. }
            )),
            "transport" => assert!(matches!(
                error,
                NativeIterationExecutionError::Base(NativeExecutionError::TransportLimit { .. })
            )),
            "memory" => assert!(matches!(
                error,
                NativeIterationExecutionError::Base(NativeExecutionError::Memory { .. })
            )),
            _ => unreachable!(),
        }
        if let Some(shared) = shared {
            assert_eq!(shared.used_bytes(), 0);
        }
    }
}
