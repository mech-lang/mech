use super::super::super::program::{
    reset_runtime_program_checkpoint_count, runtime_program_checkpoint_count,
};
use super::add_test_function;
use crate::capability::{BasicCapability, BasicOperation, BasicResource, BasicSubject};
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, MechRuntime, ObjectId, ObjectRecord, PlannedRuntimeManagedHostFunction,
    RuntimeValueSnapshot,
};
use mech_core::{LegacyValue, Ref};
use std::sync::Arc;

fn snapshot(value: LegacyValue) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn implicit_reactive_turns_use_no_full_program_checkpoints() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    add_test_function(&mut runtime, None);
    reset_runtime_program_checkpoint_count();

    for _ in 0..100 {
        runtime.step(0).unwrap();
    }

    assert_eq!(runtime_program_checkpoint_count(), 0);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
}

#[test]
fn explicit_reactive_turns_reuse_one_outer_program_checkpoint() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    reset_runtime_program_checkpoint_count();

    runtime.step_with_context(&mut context, 0).unwrap();
    assert_eq!(runtime_program_checkpoint_count(), 1);
    for _ in 0..99 {
        runtime.step_with_context(&mut context, 0).unwrap();
    }

    assert_eq!(runtime_program_checkpoint_count(), 1);
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id),);
    runtime
        .abort_runtime_transaction(&mut context, "checkpoint test")
        .unwrap();
}

#[test]
fn explicit_program_owner_excludes_other_reactive_transactions() {
    let mut runtime = test_runtime_builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context_a = runtime.runtime_context().unwrap();
    let transaction_a = runtime.begin_transaction(&mut context_a).unwrap();
    runtime.step_with_context(&mut context_a, 0).unwrap();

    let mut context_b = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context_b).unwrap();
    let error = runtime.step_with_context(&mut context_b, 0).unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramBusy");
    assert_eq!(*output.borrow(), 1);

    runtime
        .put_object_with_context(
            &mut context_b,
            ObjectRecord::text(ObjectId(699), "note", "independent store work"),
        )
        .unwrap();
    runtime
        .run_string_with_context(&mut context_a, "reactive-owner-source := 1")
        .unwrap();
    runtime.step_with_context(&mut context_a, 0).unwrap();
    assert_eq!(*output.borrow(), 2);
    assert_eq!(runtime.program_transaction_owner, Some(transaction_a),);

    runtime
        .abort_runtime_transaction(&mut context_a, "restore owner A")
        .unwrap();
    assert_eq!(*output.borrow(), 0);
    assert!(
        runtime
            .program
            .root_symbol_value("reactive-owner-source")
            .is_err()
    );
    assert!(runtime.get_object(ObjectId(699)).unwrap().is_none());
    runtime
        .abort_runtime_transaction(&mut context_b, "discard B store work")
        .unwrap();
}

#[test]
fn reactive_host_callback_uses_scoped_transaction_services() {
    let mut runtime = test_runtime_builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "demo/reactive-reenter",
            |_context, _args| Ok(snapshot(LegacyValue::F64(Ref::new(1.0)))),
            move |services, _context, _args| {
                let object = ObjectRecord::text(ObjectId(922), "note", "reactive staging");
                if services.get_object(object.id)?.is_some() {
                    services.update_object(object)?;
                } else {
                    services.put_object(object)?;
                }
                Ok(snapshot(LegacyValue::F64(Ref::new(1.0))))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(920),
            &BasicSubject::new(&subject),
            &BasicResource::new("host:demo/reactive-reenter"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(&mut context, "reactive-result := demo/reactive-reenter()")
        .unwrap();

    runtime.step_with_context(&mut context, 0).unwrap();

    assert!(runtime.get_object(ObjectId(922)).unwrap().is_some());
}
