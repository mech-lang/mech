use super::super::{
    MechRuntime, MechSourceCode, ObjectId, ObjectRecord, ResourceBudgetExceededError,
    RuntimeConfig, RuntimeEventKind,
};

#[test]
fn run_string_with_context_emits_profile_event_when_enabled() {
    let mut config = RuntimeConfig::default();
    config.diagnostics.profile_enabled = true;
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .config(config)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .run_string_with_context(&mut context, "profiled := 1")
        .unwrap();

    assert!(context.events.iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::ProgramProfiled { duration_ns, .. } if duration_ns > 0
        )
    }));
}

#[test]
fn run_string_with_context_emits_profile_event_on_failure_when_enabled() {
    let mut config = RuntimeConfig::default();
    config.diagnostics.profile_enabled = true;
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    assert!(
        runtime
            .run_string_with_context(&mut context, "1 +")
            .is_err()
    );

    assert!(context.events.iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::ProgramProfiled { duration_ns, .. } if duration_ns > 0
        )
    }));
}

#[test]
fn max_source_bytes_rejects_string_source() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(&mut context, "1234")
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.used, 0);
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
}

#[test]
fn direct_string_source_limit_uses_borrowed_length() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let source = String::from("1234");

    let error = runtime
        .run_string_with_context(&mut context, &source)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(source, "1234");
}

#[test]
fn direct_bytecode_source_limit_uses_borrowed_length() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let bytecode = vec![1, 2, 3, 4];

    let error = runtime
        .run_bytecode_with_context(&mut context, &bytecode)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(bytecode, vec![1, 2, 3, 4]);
}

#[test]
fn context_event_retention_is_bounded() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(2);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .put_object_with_context(&mut context, ObjectRecord::text(ObjectId(1), "text", "one"))
        .unwrap();
    runtime
        .put_object_with_context(&mut context, ObjectRecord::text(ObjectId(2), "text", "two"))
        .unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(3), "text", "three"),
        )
        .unwrap();
    let object_ids = context
        .events
        .iter()
        .filter_map(|event| match event.kind {
            RuntimeEventKind::ObjectCreated { object_id } => Some(object_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(object_ids, vec![ObjectId(2), ObjectId(3)]);
}

#[test]
fn max_source_bytes_rejects_program_aggregate() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let source = MechSourceCode::Program(vec![
        MechSourceCode::String("1".to_string()),
        MechSourceCode::String("22".to_string()),
        MechSourceCode::String("3".to_string()),
    ]);

    let error = runtime
        .run_source_with_context(&mut context, &source)
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "source_bytes");
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
}

#[test]
fn max_memory_bytes_rejects_large_source_buffer() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(100);
    config.limits.max_memory_bytes = Some(3);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(&mut context, "1234")
        .unwrap_err();
    let budget = error.kind_as::<ResourceBudgetExceededError>().unwrap();
    assert_eq!(budget.resource, "bytes");
    assert_eq!(budget.used, 0);
    assert_eq!(budget.requested, 4);
    assert_eq!(budget.max, Some(3));
}

#[test]
fn tree_source_without_known_size_is_not_rejected_by_source_byte_limit() {
    let mut config = RuntimeConfig::default();
    config.limits.max_source_bytes = Some(1);
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .config(config)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let tree = mech_syntax::parser::parse("tree-value := 1").unwrap();
    let source = MechSourceCode::Tree(tree);

    runtime
        .run_source_with_context(&mut context, &source)
        .unwrap();
}
