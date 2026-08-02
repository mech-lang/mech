use super::super::arm_coordinated_service_reentry;
use super::ReactiveTransactionalProbe;
use crate::capability::{BasicCapability, BasicOperation, BasicResource, BasicSubject};
use crate::{
    CapabilityId, MechRuntime, PlannedPureHostFunction, PlannedStagedHostFunction,
    PreparedRuntimeEffect, RuntimeHealth, RuntimePreparedHostCall, RuntimeValueSnapshot,
};
#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use mech_core::{
    ExecutionHostFunctionRequest, MResult, MechExecutionServices, MechFunctionImpl,
    ReactiveSolveStatus, Ref, Value,
};
use mech_engine::ExecutionServicesBorrowConflict;
use std::sync::{Arc, Mutex};

fn snapshot(value: Value) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

struct ReentrantRuntimeServiceFunction {
    output: Ref<usize>,
    staged_host: &'static str,
    reentrant_host: &'static str,
}

impl ReentrantRuntimeServiceFunction {
    fn execute(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        *self.output.borrow_mut() += 1;
        services.invoke_host_function(
            &ExecutionHostFunctionRequest {
                name: self.staged_host.to_string(),
            },
            &[],
        )?;
        services.invoke_host_function(
            &ExecutionHostFunctionRequest {
                name: self.reentrant_host.to_string(),
            },
            &[],
        )?;
        Ok(())
    }
}

impl MechFunctionImpl for ReentrantRuntimeServiceFunction {
    fn solve(&self) {}

    fn solve_result_with(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        self.execute(services)
    }

    fn solve_reactive_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.execute(services)?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn out(&self) -> Value {
        Value::Index(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::Index(self.output.clone())])
    }

    fn to_string(&self) -> String {
        "ReentrantRuntimeServiceFunction".to_string()
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ReentrantRuntimeServiceFunction {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn reentrant_runtime_service_borrow_returns_structured_error_and_recovers() {
    const STAGED_HOST: &str = "test/reentrant-staged";
    const REENTRANT_HOST: &str = "test/reentrant-service";

    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            STAGED_HOST,
            |_context, _arguments| Ok(snapshot(Value::F64(Ref::new(1.0)))),
            move |_context, _arguments| {
                Ok(RuntimePreparedHostCall {
                    value: snapshot(Value::F64(Ref::new(1.0))),
                    effect: PreparedRuntimeEffect::Transactional(Box::new(
                        ReactiveTransactionalProbe {
                            log: effect_log.clone(),
                            fail_prepare: false,
                            fail_commit: false,
                            fail_abort: false,
                        },
                    )),
                })
            },
        ))
        .unwrap()
        .host_function(PlannedPureHostFunction::new(
            REENTRANT_HOST,
            |_context, _arguments| Ok(snapshot(Value::F64(Ref::new(1.0)))),
            |_context, _arguments| Ok(snapshot(Value::F64(Ref::new(1.0)))),
        ))
        .unwrap()
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    for (id, name) in [
        (CapabilityId(940), STAGED_HOST),
        (CapabilityId(941), REENTRANT_HOST),
    ] {
        runtime
            .grant_capability(Arc::new(BasicCapability::new(
                id,
                &BasicSubject::new(&subject),
                &BasicResource::new(format!("host:{name}")),
                [BasicOperation::new("call")],
            )))
            .unwrap();
    }
    let output = Ref::new(0usize);
    runtime
        .program
        .interpreter()
        .plan()
        .add_function(Box::new(ReentrantRuntimeServiceFunction {
            output: output.clone(),
            staged_host: STAGED_HOST,
            reentrant_host: REENTRANT_HOST,
        }));
    let mut context = runtime.runtime_context().unwrap();
    arm_coordinated_service_reentry(REENTRANT_HOST);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.step_with_context(&mut context, 0)
    }));

    let error = result
        .expect("reentrant runtime service access must not panic")
        .unwrap_err();
    assert_eq!(error.kind_name(), "ExecutionServicesBorrowConflict");
    assert_eq!(
        error
            .kind_as::<ExecutionServicesBorrowConflict>()
            .unwrap()
            .operation,
        "runtime_invoke_host_function",
    );
    assert_eq!(*output.borrow(), 0);
    assert_eq!(*log.lock().unwrap(), vec!["abort"]);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));

    runtime.step_with_context(&mut context, 0).unwrap();

    assert_eq!(*output.borrow(), 1);
    assert_eq!(*log.lock().unwrap(), vec!["abort", "prepare", "commit"],);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(context.transaction, None);
}
