#[cfg(feature = "compiler")]
use super::super::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use super::super::{CapabilityId, MResult, MechFunctionImpl, MechRuntime, RuntimeConfig, Value};
use crate::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject, PlannedPureHostFunction,
    RuntimeCallContext, RuntimeValueSnapshot,
};
use mech_core::{ExecutionHostFunctionRequest, InitialSolvePolicy, Ref};
use mech_engine::ExternalHostCallFunction;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "functions")]
struct RuntimeStepProbe {
    calls: Arc<AtomicUsize>,
    output: Ref<usize>,
}

#[cfg(feature = "functions")]
impl MechFunctionImpl for RuntimeStepProbe {
    fn solve_result(&self) -> MResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.output.borrow_mut() += 1;
        Ok(())
    }

    fn out(&self) -> Value {
        Value::Index(self.output.clone())
    }

    fn to_string(&self) -> String {
        "RuntimeStepProbe".into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(all(feature = "functions", feature = "compiler"))]
impl MechFunctionCompiler for RuntimeStepProbe {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(feature = "functions")]
#[test]
fn step_with_context_recomputes_runtime_host_function_with_provided_context() {
    let host_calls = Arc::new(AtomicUsize::new(0));
    let host_calls_for_host = host_calls.clone();
    let mut runtime = MechRuntime::builder()
        .config(RuntimeConfig::default())
        .host_function(PlannedPureHostFunction::new(
            "demo/echo",
            |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| Ok(args[0].clone()),
            move |context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
                assert_eq!(context.subject(), "program:step-host-test");
                host_calls_for_host.fetch_add(1, Ordering::SeqCst);
                let argument = args[0].to_value();
                let value = match argument {
                    Value::F64(value) => Value::F64(Ref::new(*value.borrow())),
                    Value::MutableReference(value) => match &*value.borrow() {
                        Value::F64(value) => Value::F64(Ref::new(*value.borrow())),
                        other => panic!("expected F64 mutable reference, got {:?}", other),
                    },
                    other => panic!("expected F64 argument, got {:?}", other),
                };
                RuntimeValueSnapshot::try_capture(&value)
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(1),
            &BasicSubject::new("program:step-host-test"),
            &BasicResource::new("host:demo/echo"),
            [BasicOperation::new("call")],
        )))
        .unwrap();

    let mut context = runtime
        .runtime_context()
        .unwrap()
        .with_subject("program:step-host-test");
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let host_output = Ref::new(Value::F64(Ref::new(1.0)));
    let plan = runtime.program().interpreter().plan();
    plan.add_function(Box::new(RuntimeStepProbe {
        calls: first_calls.clone(),
        output: Ref::new(0),
    }));
    plan.add_function(Box::new(RuntimeStepProbe {
        calls: second_calls.clone(),
        output: Ref::new(0),
    }));
    plan.add_function(Box::new(ExternalHostCallFunction {
        request: ExecutionHostFunctionRequest {
            name: "demo/echo".to_string(),
        },
        arguments: vec![Value::F64(Ref::new(2.0))],
        output: host_output.clone(),
        initial_solve_policy: InitialSolvePolicy::Solve,
    }));

    runtime.step_with_context(&mut context, 3).unwrap();

    assert_eq!(first_calls.load(Ordering::SeqCst), 0);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host_calls.load(Ordering::SeqCst), 1);
    match host_output.borrow().clone() {
        Value::F64(value) => assert_eq!(*value.borrow(), 2.0),
        other => panic!("expected F64(2.0), got {:?}", other),
    }
}
