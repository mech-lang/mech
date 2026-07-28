#[cfg(all(
    test,
    feature = "program",
    feature = "compiler",
    feature = "functions",
    feature = "variables",
    feature = "variable_define",
    feature = "variable_assign",
    feature = "assign",
    feature = "f64",
    feature = "math"
))]
mod reactive_turn_interpreter_state_tests {
    use super::super::super::{
        Interpreter, ReactiveCellId, ReactiveNodeId, ReactiveNodeKind, hash_str,
    };
    const SOURCE: &str = "input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0";
    fn interpreter() -> Interpreter {
        let mut i = Interpreter::new_with_full_stdlib(1);
        let t = mech_syntax::parser::parse(SOURCE).unwrap();
        i.interpret(&t).unwrap();
        i
    }
    fn value(i: &Interpreter, n: &str) -> f64 {
        let value = i
            .symbols()
            .borrow()
            .get(hash_str(n))
            .expect("symbol")
            .borrow()
            .clone();
        *value.as_f64().expect("f64").borrow()
    }
    fn cell(i: &Interpreter, n: &str) -> ReactiveCellId {
        let v = i
            .symbols()
            .borrow()
            .get(hash_str(n))
            .expect("symbol")
            .borrow()
            .reactive_root_cell_ids();
        assert_eq!(v.len(), 1, "root cell");
        v[0]
    }
    fn register(i: &Interpreter, c: ReactiveCellId) -> ReactiveNodeId {
        let p = i.plan();
        let v = p
            .borrow()
            .nodes
            .iter()
            .filter(|n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&c))
            .map(|n| n.id)
            .collect::<Vec<_>>();
        assert_eq!(v.len(), 1, "register");
        v[0]
    }
    fn first(i: &mut Interpreter) -> (ReactiveNodeId, ReactiveNodeId) {
        assert_eq!(
            (
                value(i, "input"),
                value(i, "a"),
                value(i, "middle"),
                value(i, "b"),
                value(i, "output")
            ),
            (1., 1., 2., 2., 3.)
        );
        let (input, a, b) = (
            cell(i, "input"),
            register(i, cell(i, "a")),
            register(i, cell(i, "b")),
        );
        let input_value = i
            .symbols()
            .borrow()
            .get(hash_str("input"))
            .unwrap()
            .borrow()
            .clone();
        *input_value.as_f64().unwrap().borrow_mut() = 10.;
        let o = i.advance_reactive_turn(&[input]).unwrap();
        assert_eq!(o.register_commit.committed_nodes, vec![a]);
        assert_eq!(o.after_commit.pending_register_nodes, vec![b]);
        (a, b)
    }
    #[test]
    fn reactive_turn_interpreter_state_persists_between_calls() {
        let mut i = interpreter();
        let (a, b) = first(&mut i);
        assert_eq!(
            (
                value(&i, "a"),
                value(&i, "middle"),
                value(&i, "b"),
                value(&i, "output")
            ),
            (10., 11., 2., 3.)
        );
        assert!(i.has_pending_reactive_registers());
        let o = i.advance_reactive_turn(&[]).unwrap();
        assert_eq!(o.register_commit.committed_nodes, vec![b]);
        assert!(!o.register_commit.committed_nodes.contains(&a));
        assert_eq!(
            (
                value(&i, "a"),
                value(&i, "middle"),
                value(&i, "b"),
                value(&i, "output")
            ),
            (10., 11., 11., 12.)
        );
        assert!(o.after_commit.pending_register_nodes.is_empty());
        assert!(!i.has_pending_reactive_registers());
    }
    #[test]
    fn reactive_turn_interpreter_state_clear_plan_resets_pending() {
        let mut i = interpreter();
        first(&mut i);
        assert!(i.has_pending_reactive_registers());
        assert!(i.plan_len() > 0);
        i.clear_plan();
        assert_eq!(i.plan_len(), 0);
        assert!(!i.has_pending_reactive_registers());
    }
    #[test]
    fn reactive_turn_interpreter_state_clone_preserves_pending() {
        let mut i = interpreter();
        first(&mut i);
        let c = i.clone();
        assert!(i.has_pending_reactive_registers());
        assert!(c.has_pending_reactive_registers());
        assert_eq!(i.plan_len(), c.plan_len());
    }
}

#[cfg(all(test, feature = "functions"))]
mod compact_reactive_turn_checkpoint_tests {
    #[cfg(feature = "compiler")]
    use super::super::super::{CompileCtx, MechFunctionCompiler, Register};
    use super::super::super::{
        GenericError, Interpreter, MResult, MechError, MechFunction, MechFunctionImpl,
        NoMechExecutionServices, Plan, ReactiveJournalAutomaticRollbackFailed, ReactiveNodeId,
        ReactiveNodeKind, ReactiveSolveStatus, Ref, Value, with_reactive_journal_participant,
    };
    use std::{cell::RefCell, rc::Rc};

    struct CompactTestFunction {
        name: &'static str,
        output: Ref<usize>,
        captures: Rc<RefCell<usize>>,
        solves: Rc<RefCell<usize>>,
        fail_on_solve: Option<usize>,
        leak_borrow_on_failure: bool,
    }

    impl CompactTestFunction {
        fn new(
            name: &'static str,
            output: Ref<usize>,
            captures: Rc<RefCell<usize>>,
            solves: Rc<RefCell<usize>>,
        ) -> Self {
            Self {
                name,
                output,
                captures,
                solves,
                fail_on_solve: None,
                leak_borrow_on_failure: false,
            }
        }

        fn execute(&self) -> MResult<()> {
            let solve = {
                let mut solves = self.solves.borrow_mut();
                *solves += 1;
                *solves
            };
            *self.output.borrow_mut() += 1;
            if self.fail_on_solve == Some(solve) {
                if self.leak_borrow_on_failure {
                    let borrow = self.output.borrow_mut();
                    std::mem::forget(borrow);
                }
                return Err(MechError::new(
                    GenericError {
                        msg: format!("deliberate {} execution failure", self.name),
                    },
                    None,
                ));
            }
            Ok(())
        }
    }

    impl MechFunctionImpl for CompactTestFunction {
        fn solve(&self) {}

        fn solve_result(&self) -> MResult<()> {
            self.execute()
        }

        fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
            self.execute()?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn out(&self) -> Value {
            Value::Index(self.output.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            *self.captures.borrow_mut() += 1;
            Ok(vec![Value::Index(self.output.clone())])
        }

        fn to_string(&self) -> String {
            self.name.into()
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for CompactTestFunction {
        fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
            Ok(0)
        }
    }

    fn function(
        name: &'static str,
        output: Ref<usize>,
    ) -> (CompactTestFunction, Rc<RefCell<usize>>, Rc<RefCell<usize>>) {
        let captures = Rc::new(RefCell::new(0));
        let solves = Rc::new(RefCell::new(0));
        (
            CompactTestFunction::new(name, output, captures.clone(), solves.clone()),
            captures,
            solves,
        )
    }

    fn add_reactive(
        interpreter: &Interpreter,
        function: CompactTestFunction,
        input: &Ref<usize>,
    ) -> ReactiveNodeId {
        interpreter
            .plan()
            .0
            .borrow_mut()
            .register(Box::new(function), &[Value::Index(input.clone())])
            .unwrap()
    }

    #[test]
    fn reactive_turn_checkpoint_contains_only_scheduler_metadata_and_plan_handles() {
        let interpreter = Interpreter::new(7, 100);
        let plan = interpreter.plan();
        let (function, _, _) = function("node", Ref::new(0));
        plan.add_function(Box::new(function));

        let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();

        assert_eq!(checkpoint.interpreter_id, 7);
        assert_eq!(checkpoint.plan_node_len, 1);
        assert_eq!(checkpoint.activation_registration_depth, 0);
        assert_eq!(checkpoint.plan.0.addr(), plan.0.addr());
        assert_eq!(checkpoint.plan.1.addr(), plan.1.addr());
    }

    #[test]
    fn reactive_turn_checkpoint_does_not_capture_function_values() {
        let interpreter = Interpreter::new(8, 100);
        let (function, captures, _) = function("node", Ref::new(0));
        interpreter.plan().add_function(Box::new(function));

        interpreter.reactive_turn_checkpoint().unwrap();

        assert_eq!(*captures.borrow(), 0);
    }

    #[test]
    fn reactive_turn_checkpoint_restores_pending_registers_exactly() {
        let mut interpreter = Interpreter::new(9, 100);
        interpreter.reactive_turn_state.pending_register_nodes = vec![3, 5, 3];
        let checkpoint = match interpreter.reactive_turn_checkpoint() {
            Ok(_) => panic!("invalid pending registers unexpectedly checkpointed"),
            Err(error) => error,
        };
        assert_eq!(checkpoint.kind_name(), "InterpreterReactiveTurnInvariant");

        let (function, _, _) = function("register-shaped", Ref::new(0));
        let node = interpreter.plan().add_function(Box::new(function));
        interpreter.plan().0.borrow_mut().nodes[node].kind = ReactiveNodeKind::Register;
        interpreter.reactive_turn_state.pending_register_nodes = vec![node, node];
        let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        interpreter
            .reactive_turn_state
            .pending_register_nodes
            .clear();
        interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap();
        interpreter.apply_restore_reactive_turn(&checkpoint);
        assert_eq!(
            interpreter.reactive_turn_state.pending_register_nodes,
            vec![node, node]
        );
    }

    #[test]
    fn reactive_turn_failure_truncates_appended_trace_events() {
        let mut interpreter = Interpreter::new(10, 100);
        #[cfg(feature = "trace")]
        {
            interpreter.trace = true;
            interpreter.trace_to_stdout = false;
        }
        let output = Ref::new(0);
        let (mut function, _, _) = function("trace-fail", output);
        function.fail_on_solve = Some(1);
        interpreter.plan().add_function(Box::new(function));
        #[cfg(feature = "trace")]
        let original_len = interpreter.trace_events.borrow().len();

        interpreter.step(0, 1).unwrap_err();

        #[cfg(feature = "trace")]
        assert_eq!(interpreter.trace_events.borrow().len(), original_len);
    }

    #[test]
    fn reactive_turn_owner_mismatch_fails_before_mutation() {
        let first = Interpreter::new(11, 100);
        let second = Interpreter::new(11, 100);
        let checkpoint = first.reactive_turn_checkpoint().unwrap();

        let error = second
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnOwnerMismatch");
    }

    #[test]
    fn reactive_turn_interpreter_id_mismatch_fails_before_mutation() {
        let mut interpreter = Interpreter::new(12, 100);
        let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        interpreter.id = 13;

        let error = interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert!(error.kind_message().contains("interpreter ID"));
    }

    #[test]
    fn reactive_turn_plan_handle_mismatch_fails_before_mutation() {
        let mut interpreter = Interpreter::new(14, 100);
        let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        interpreter.state.borrow_mut().plan = Plan::new();

        let error = interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert!(error.kind_message().contains("plan handles"));
    }

    #[test]
    fn reactive_turn_plan_length_mismatch_fails_before_mutation() {
        let interpreter = Interpreter::new(15, 100);
        let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        let (function, _, _) = function("tail", Ref::new(0));
        interpreter.plan().add_function(Box::new(function));

        let error = interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert!(error.kind_message().contains("plan length"));
    }

    #[test]
    fn reactive_turn_activation_depth_mismatch_fails_before_mutation() {
        let interpreter = Interpreter::new(16, 100);
        let checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        interpreter
            .plan()
            .push_activation_registration_scope(Vec::new());

        let error = interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert!(
            error
                .kind_message()
                .contains("activation registration depth")
        );
    }

    #[test]
    fn reactive_turn_saved_missing_pending_node_is_rejected() {
        let interpreter = Interpreter::new(17, 100);
        let mut checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        checkpoint.reactive_turn_state.pending_register_nodes = vec![99];

        let error = interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert!(error.kind_message().contains("does not exist"));
    }

    #[test]
    fn reactive_turn_saved_non_register_pending_node_is_rejected() {
        let interpreter = Interpreter::new(18, 100);
        let (function, _, _) = function("comb", Ref::new(0));
        let node = interpreter.plan().add_function(Box::new(function));
        let mut checkpoint = interpreter.reactive_turn_checkpoint().unwrap();
        checkpoint.reactive_turn_state.pending_register_nodes = vec![node];

        let error = interpreter
            .preflight_restore_reactive_turn(&checkpoint)
            .unwrap_err();

        assert_eq!(error.kind_name(), "InterpreterReactiveTurnInvariant");
        assert!(error.kind_message().contains("not a register"));
    }

    #[test]
    fn participating_interpreter_turn_leaves_rollback_to_coordinator() {
        let mut interpreter = Interpreter::new(19, 100);
        let input = Ref::new(1usize);
        let output = Ref::new(4usize);
        let (mut function, _, _) = function("failure", output.clone());
        function.fail_on_solve = Some(1);
        add_reactive(&interpreter, function, &input);
        with_reactive_journal_participant(|mut participant| {
            let mut services = NoMechExecutionServices;
            interpreter
                .advance_reactive_turn_participating(
                    &Value::Index(input).reactive_root_cell_ids(),
                    &mut participant,
                    &mut services,
                )
                .unwrap_err();
            assert_eq!(*output.borrow(), 5);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*output.borrow(), 4);
    }

    #[test]
    fn ordinary_interpreter_reactive_turn_rolls_back_on_failure() {
        let mut interpreter = Interpreter::new(20, 100);
        let input = Ref::new(1usize);
        let output = Ref::new(4usize);
        let (mut function, _, _) = function("failure", output.clone());
        function.fail_on_solve = Some(1);
        add_reactive(&interpreter, function, &input);

        interpreter
            .advance_reactive_turn(&Value::Index(input).reactive_root_cell_ids())
            .unwrap_err();

        assert_eq!(*output.borrow(), 4);
    }

    #[test]
    fn ordinary_interpreter_step_rolls_back_earlier_function_mutation() {
        let mut interpreter = Interpreter::new(21, 100);
        let first_output = Ref::new(1usize);
        let second_output = Ref::new(2usize);
        let (first, _, _) = function("first", first_output.clone());
        let (mut second, _, _) = function("second", second_output.clone());
        second.fail_on_solve = Some(1);
        interpreter.plan().add_function(Box::new(first));
        interpreter.plan().add_function(Box::new(second));

        interpreter.step(0, 1).unwrap_err();

        assert_eq!((*first_output.borrow(), *second_output.borrow()), (1, 2));
    }

    #[test]
    fn repeated_whole_plan_step_restores_before_first_repetition() {
        let mut interpreter = Interpreter::new(22, 100);
        let output = Ref::new(10usize);
        let (mut function, _, _) = function("repeat", output.clone());
        function.fail_on_solve = Some(3);
        interpreter.plan().add_function(Box::new(function));

        interpreter.step(0, 4).unwrap_err();

        assert_eq!(*output.borrow(), 10);
    }

    #[test]
    fn indexed_step_captures_only_selected_function() {
        let mut interpreter = Interpreter::new(23, 100);
        let (first, first_captures, _) = function("first", Ref::new(1));
        let (second, second_captures, _) = function("second", Ref::new(2));
        interpreter.plan().add_function(Box::new(first));
        interpreter.plan().add_function(Box::new(second));
        with_reactive_journal_participant(|mut participant| {
            let mut services = NoMechExecutionServices;
            interpreter.step_reactive_turn_participating(2, 1, &mut participant, &mut services)?;
            assert_eq!(
                (*first_captures.borrow(), *second_captures.borrow()),
                (0, 1),
            );
            participant.commit();
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn reactive_turn_rollback_failure_retains_original_error() {
        let mut interpreter = Interpreter::new(24, 100);
        let output = Ref::new(0usize);
        let (mut function, _, _) = function("borrow-leak", output);
        function.fail_on_solve = Some(1);
        function.leak_borrow_on_failure = true;
        interpreter.plan().add_function(Box::new(function));

        let error = interpreter.step(0, 1).unwrap_err();

        assert_eq!(error.kind_name(), "ReactiveJournalAutomaticRollbackFailed",);
        let rollback = error
            .kind_as::<ReactiveJournalAutomaticRollbackFailed>()
            .unwrap();
        let original_error = rollback.original_error.as_ref().unwrap();
        assert!(original_error.contains("InterpreterReactiveTurnRollbackFailed"),);
        assert!(original_error.contains("deliberate borrow-leak execution failure"),);
        assert!(original_error.contains("ValueStateBorrowConflict"));
        assert!(rollback.rollback_error.contains("ValueStateBorrowConflict"));
    }

    #[test]
    fn reactive_turn_paths_preserve_plan_identity_without_full_checkpoints() {
        let mut interpreter = Interpreter::new(25, 100);
        let plan = interpreter.plan();
        let plan_address = plan.0.addr();
        let (function, _, _) = function("success", Ref::new(0));
        let node = plan.add_function(Box::new(function));

        interpreter.step(0, 2).unwrap();

        assert_eq!(interpreter.plan().0.addr(), plan_address);
        assert_eq!(interpreter.plan().borrow().nodes[node].id, node);
    }
}
