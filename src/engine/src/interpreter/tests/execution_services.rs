#[cfg(test)]
mod execution_services_borrow_tests {
    use super::super::super::{Interpreter, InterpreterExecution, NoMechExecutionServices};

    #[test]
    fn direct_execution_uses_the_root_presentation_namespace() {
        let interpreter = Interpreter::new(7001, 100);
        let nested = Interpreter::new(7002, 100);
        let mut services = NoMechExecutionServices;
        let execution = InterpreterExecution::new(&interpreter, &mut services);

        assert_eq!(execution.presentation_namespace(), 0);
        execution
            .with_interpreter(&nested, |nested_execution| {
                assert_eq!(nested_execution.presentation_namespace(), nested.id);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn nested_execution_service_access_returns_a_structured_error() {
        let interpreter = Interpreter::new(7001, 100);
        let mut services = NoMechExecutionServices;
        let execution = InterpreterExecution::new(&interpreter, &mut services);

        let error = execution
            .with_services(|_| execution.with_services(|_| Ok(())))
            .unwrap_err();

        assert_eq!(error.kind_name(), "ExecutionServicesBorrowConflict");
        assert!(error.kind_message().contains("with_services"));
    }

    #[test]
    fn reentrant_interpreter_execution_returns_an_error_without_panicking() {
        let interpreter = Interpreter::new(7002, 100);
        let nested = Interpreter::new(7003, 100);
        let mut services = NoMechExecutionServices;
        let execution = InterpreterExecution::new(&interpreter, &mut services);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execution.with_services(|_| execution.with_interpreter(&nested, |_| Ok(())))
        }));

        let error = result
            .expect("reentrant service access must not panic")
            .unwrap_err();
        assert_eq!(error.kind_name(), "ExecutionServicesBorrowConflict");
        assert!(error.kind_message().contains("with_interpreter"));
    }
}
