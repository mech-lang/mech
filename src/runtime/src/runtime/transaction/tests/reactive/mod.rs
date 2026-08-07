use crate::{MechRuntime, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeTransactionalEffect};
#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use mech_core::{
    GenericError, MResult, MechError, MechFunctionImpl, ReactiveSolveStatus, Ref, Value,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod coordination;
mod finalization;
mod rollback;
mod service_borrow;

struct ReactiveTransactionTestFunction {
    output: Ref<usize>,
    calls: Rc<RefCell<usize>>,
    fail_on_call: Option<usize>,
}

struct PanickingReactiveFunction {
    output: Ref<usize>,
    message: &'static str,
}

#[derive(Debug)]
struct ReactiveTransactionalProbe {
    log: Arc<Mutex<Vec<&'static str>>>,
    fail_prepare: bool,
    fail_commit: bool,
    fail_abort: bool,
}

impl RuntimeTransactionalEffect for ReactiveTransactionalProbe {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "reactive-transaction-probe".to_string(),
            },
            "reactive-transaction-probe",
        )
    }

    fn prepare(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push("prepare");
        if self.fail_prepare {
            return Err(MechError::new(
                GenericError {
                    msg: "deliberate reactive prepare failure".to_string(),
                },
                None,
            ));
        }
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push("commit");
        if self.fail_commit {
            return Err(MechError::new(
                GenericError {
                    msg: "deliberate reactive commit failure".to_string(),
                },
                None,
            ));
        }
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push("abort");
        if self.fail_abort {
            return Err(MechError::new(
                GenericError {
                    msg: "deliberate reactive abort failure".to_string(),
                },
                None,
            ));
        }
        Ok(())
    }
}

impl MechFunctionImpl for ReactiveTransactionTestFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_reactive().map(|_| ())
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        let call = {
            let mut calls = self.calls.borrow_mut();
            *calls += 1;
            *calls
        };
        *self.output.borrow_mut() += 1;
        if self.fail_on_call == Some(call) {
            return Err(MechError::new(
                GenericError {
                    msg: "deliberate reactive transaction failure".to_string(),
                },
                None,
            ));
        }
        Ok(ReactiveSolveStatus::Changed)
    }

    fn out(&self) -> Value {
        Value::Index(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::Index(self.output.clone())])
    }

    fn to_string(&self) -> String {
        "ReactiveTransactionTestFunction".to_string()
    }
}

impl MechFunctionImpl for PanickingReactiveFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_reactive().map(|_| ())
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.output.borrow_mut() += 1;
        panic!("{}", self.message);
    }

    fn out(&self) -> Value {
        Value::Index(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::Index(self.output.clone())])
    }

    fn to_string(&self) -> String {
        "PanickingReactiveFunction".to_string()
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ReactiveTransactionTestFunction {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for PanickingReactiveFunction {
    fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn add_test_function(
    runtime: &mut MechRuntime,
    fail_on_call: Option<usize>,
) -> (Ref<usize>, Rc<RefCell<usize>>) {
    let output = Ref::new(0usize);
    let calls = Rc::new(RefCell::new(0usize));
    runtime
        .program
        .interpreter()
        .plan()
        .add_function(Box::new(ReactiveTransactionTestFunction {
            output: output.clone(),
            calls: calls.clone(),
            fail_on_call,
        }));
    (output, calls)
}

fn add_panicking_test_function(runtime: &mut MechRuntime, message: &'static str) -> Ref<usize> {
    let output = Ref::new(0usize);
    runtime
        .program
        .interpreter()
        .plan()
        .add_function(Box::new(PanickingReactiveFunction {
            output: output.clone(),
            message,
        }));
    output
}
