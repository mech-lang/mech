#[cfg(all(test, feature = "functions", feature = "symbol_table", feature = "f64"))]
mod checkpoint_tests {
    use super::super::super::{
        Interpreter, MechSourceCode, ModuleManifestCatalog, ProgramState, ReactiveCellId, Ref,
        RuntimeContextBinding, ValRef, Value, ValueStateBorrowConflict, hash_str,
    };
    use std::collections::HashMap;

    #[cfg(feature = "invariant_define")]
    use super::super::super::{ComparisonOp, FormulaOperator, IntegrityConstraint};
    #[cfg(feature = "state_machines")]
    use super::super::super::{
        FsmImplementation, FsmSpecification, Pattern, internal_pattern_value_identifier,
    };

    fn f64_value(value: &Ref<f64>) -> Value {
        Value::F64(value.clone())
    }

    fn install_scalar(interpreter: &Interpreter, name: &str, value: f64) -> (ValRef, Ref<f64>) {
        let backing = Ref::new(value);
        let id = hash_str(name);
        let symbols = interpreter.symbols();
        let cell = symbols.borrow_mut().insert(id, f64_value(&backing), true);
        symbols
            .borrow()
            .dictionary
            .borrow_mut()
            .insert(id, name.to_string());
        (cell, backing)
    }

    #[test]
    fn interpreter_checkpoint_restores_private_state_and_recursive_child_identity() {
        let mut root = Interpreter::new(1, 100);
        root.ip = 7;
        root.profile = true;
        let (symbol_cell, symbol_backing) = install_scalar(&root, "kept", 1.0);
        let symbol_cell_address = symbol_cell.addr();
        let symbol_backing_address = symbol_backing.addr();
        let symbol_backing_identity = ReactiveCellId::new(symbol_backing.id());
        root.registers = vec![f64_value(&symbol_backing)];
        root.constants = vec![f64_value(&Ref::new(2.0))];
        root.out = f64_value(&symbol_backing);
        root.code.push(MechSourceCode::String("before".to_string()));
        root.out_values
            .borrow_mut()
            .insert(hash_str("out"), f64_value(&symbol_backing));
        *root.inline_eval_counter.borrow_mut() = 4;
        *root.persistent_user_function_plan_depth.borrow_mut() = 2;
        *root.deferred_expression_solve_depth.borrow_mut() = 3;
        let context_binding_id = hash_str("checkpoint-context");
        root.context_bindings.borrow_mut().insert(
            context_binding_id,
            RuntimeContextBinding {
                name: "checkpoint-context".to_string(),
                base_uri: "test://checkpoint".to_string(),
            },
        );
        let original_manifests = root.module_manifests.borrow().clone();
        #[cfg(feature = "trace")]
        {
            root.trace = true;
            root.trace_to_stdout = false;
        }
        #[cfg(feature = "state_machines")]
        {
            let name = internal_pattern_value_identifier("checkpoint-fsm");
            root.user_state_machines.borrow_mut().insert(
                hash_str("checkpoint-fsm"),
                FsmImplementation {
                    name: name.clone(),
                    input: Vec::new(),
                    start: Pattern::Wildcard,
                    arms: Vec::new(),
                },
            );
            root.user_state_machine_specs.borrow_mut().insert(
                hash_str("checkpoint-fsm"),
                FsmSpecification {
                    name,
                    input: Vec::new(),
                    output: None,
                    states: Vec::new(),
                },
            );
        }
        #[cfg(feature = "invariant_define")]
        let invariant_result = Ref::new(Value::Bool(Ref::new(true)));
        #[cfg(feature = "invariant_define")]
        let invariant_rhs = Ref::new(Value::F64(Ref::new(2.0)));
        #[cfg(feature = "invariant_define")]
        {
            let invariant_id = hash_str("checkpoint-invariant");
            let mut state = root.state.borrow_mut();
            state.integrity_constraints.insert(
                invariant_id,
                IntegrityConstraint {
                    id: invariant_id,
                    name: "checkpoint invariant".to_string(),
                    expression: "kept <= 2.0".to_string(),
                    result: invariant_result.clone(),
                    lhs: Some(symbol_cell.clone()),
                    operator: Some(FormulaOperator::Comparison(ComparisonOp::LessThanEqual)),
                    rhs: Some(invariant_rhs.clone()),
                    tokens: Vec::new(),
                },
            );
        }

        let state_address = root.state.addr();
        let symbols_address = root.symbols().addr();
        let symbol_dictionary_address = root.symbols().borrow().dictionary.addr();
        let out_values_address = root.out_values.addr();
        let inline_counter_address = root.inline_eval_counter.addr();
        let context_bindings_address = root.context_bindings.addr();
        let module_manifests_address = root.module_manifests.addr();
        #[cfg(feature = "trace")]
        let trace_events_address = root.trace_events.addr();
        #[cfg(feature = "state_machines")]
        let user_state_machines_address = root.user_state_machines.addr();
        #[cfg(feature = "state_machines")]
        let user_state_machine_specs_address = root.user_state_machine_specs.addr();
        let sub_interpreters_address = root.sub_interpreters.addr();

        let child_id = 2;
        let grandchild_id = 3;
        let mut child = Interpreter::new(child_id, 200);
        child.ip = 8;
        let (_child_cell, child_backing) = install_scalar(&child, "child", 20.0);
        let mut grandchild = Interpreter::new(grandchild_id, 300);
        grandchild.ip = 9;
        let (_grandchild_cell, grandchild_backing) =
            install_scalar(&grandchild, "grandchild", 30.0);
        let grandchild_ref = Ref::new(Box::new(grandchild));
        let grandchild_handle_address = grandchild_ref.addr();
        child
            .sub_interpreters
            .borrow_mut()
            .insert(grandchild_id, grandchild_ref);
        let child_ref = Ref::new(Box::new(child));
        let child_handle_address = child_ref.addr();
        root.sub_interpreters
            .borrow_mut()
            .insert(child_id, child_ref);

        let checkpoint = root.checkpoint().unwrap();

        root.id = 99;
        root.ip = 99;
        root.profile = false;
        root.max_steps = 999;
        root.registers.clear();
        root.constants.clear();
        root.code.push(MechSourceCode::String("after".to_string()));
        root.out = Value::Empty;
        root.state = Ref::new(ProgramState::new());
        root.out_values = Ref::new(HashMap::new());
        root.inline_eval_counter = Ref::new(99);
        root.persistent_user_function_plan_depth = Ref::new(99);
        root.deferred_expression_solve_depth = Ref::new(99);
        root.context_bindings = Ref::new(HashMap::new());
        root.module_manifests = Ref::new(ModuleManifestCatalog::new());
        #[cfg(feature = "trace")]
        {
            root.trace = false;
            root.trace_to_stdout = true;
            root.trace_events = Ref::new(Vec::new());
        }
        #[cfg(feature = "state_machines")]
        {
            root.user_state_machines = Ref::new(HashMap::new());
            root.user_state_machine_specs = Ref::new(HashMap::new());
        }
        *symbol_backing.borrow_mut() = 11.0;
        *child_backing.borrow_mut() = 21.0;
        *grandchild_backing.borrow_mut() = 31.0;
        #[cfg(feature = "invariant_define")]
        {
            if let Value::Bool(value) = &*invariant_result.borrow() {
                *value.borrow_mut() = false;
            }
            if let Value::F64(value) = &*invariant_rhs.borrow() {
                *value.borrow_mut() = 99.0;
            }
        }

        let removed_child = root
            .sub_interpreters
            .borrow_mut()
            .remove(&child_id)
            .unwrap();
        {
            let mut child = removed_child.borrow_mut();
            child.id = 22;
            child.ip = 88;
            let removed_grandchild = child
                .sub_interpreters
                .borrow_mut()
                .remove(&grandchild_id)
                .unwrap();
            drop(removed_grandchild);
        }
        drop(removed_child);
        root.sub_interpreters
            .borrow_mut()
            .insert(999, Ref::new(Box::new(Interpreter::new(999, 10))));

        let held = symbol_backing.borrow();
        let error = root.restore(checkpoint.clone()).unwrap_err();
        assert_eq!(
            error.kind_as::<ValueStateBorrowConflict>().unwrap().phase,
            "restore-before"
        );
        assert_eq!(root.id, 99);
        assert_eq!(root.ip, 99);
        assert_ne!(root.state.addr(), state_address);
        assert!(root.sub_interpreters.borrow().contains_key(&999));
        assert!(!root.sub_interpreters.borrow().contains_key(&child_id));
        assert_eq!(*held, 11.0);
        drop(held);

        root.restore(checkpoint).unwrap();

        assert_eq!(root.id, 1);
        assert_eq!(root.ip, 7);
        assert!(root.profile);
        assert_eq!(root.max_steps, 100);
        assert_eq!(root.state.addr(), state_address);
        assert_eq!(root.symbols().addr(), symbols_address);
        assert_eq!(
            root.symbols().borrow().dictionary.addr(),
            symbol_dictionary_address
        );
        assert_eq!(root.out_values.addr(), out_values_address);
        assert_eq!(root.inline_eval_counter.addr(), inline_counter_address);
        assert_eq!(root.context_bindings.addr(), context_bindings_address);
        assert_eq!(root.module_manifests.addr(), module_manifests_address);
        assert_eq!(*root.module_manifests.borrow(), original_manifests);
        assert_eq!(
            root.context_bindings
                .borrow()
                .get(&context_binding_id)
                .unwrap()
                .base_uri,
            "test://checkpoint",
        );
        #[cfg(feature = "trace")]
        {
            assert!(root.trace);
            assert!(!root.trace_to_stdout);
            assert_eq!(root.trace_events.addr(), trace_events_address);
        }
        #[cfg(feature = "state_machines")]
        {
            assert_eq!(root.user_state_machines.addr(), user_state_machines_address);
            assert_eq!(
                root.user_state_machine_specs.addr(),
                user_state_machine_specs_address,
            );
            assert!(
                root.user_state_machines
                    .borrow()
                    .contains_key(&hash_str("checkpoint-fsm"))
            );
            assert!(
                root.user_state_machine_specs
                    .borrow()
                    .contains_key(&hash_str("checkpoint-fsm"))
            );
        }
        #[cfg(feature = "invariant_define")]
        {
            let state = root.state.borrow();
            let invariant_id = hash_str("checkpoint-invariant");
            let constraint = state.integrity_constraints.get(&invariant_id).unwrap();
            assert_eq!(constraint.id, invariant_id);
            assert_eq!(constraint.name, "checkpoint invariant");
            assert_eq!(constraint.expression, "kept <= 2.0");
            assert_eq!(
                constraint.operator,
                Some(FormulaOperator::Comparison(ComparisonOp::LessThanEqual,)),
            );
            assert_eq!(constraint.result.addr(), invariant_result.addr());
            assert_eq!(
                constraint.rhs.as_ref().unwrap().addr(),
                invariant_rhs.addr()
            );
            if let Value::Bool(value) = &*constraint.result.borrow() {
                assert!(*value.borrow());
            } else {
                panic!("restored constraint result must remain bool");
            }
            if let Value::F64(value) = &*constraint.rhs.as_ref().unwrap().borrow() {
                assert_eq!(*value.borrow(), 2.0);
            } else {
                panic!("restored constraint rhs must remain f64");
            }
        }
        assert_eq!(root.sub_interpreters.addr(), sub_interpreters_address);
        assert_eq!(root.registers.len(), 1);
        assert_eq!(root.constants.len(), 1);
        assert_eq!(
            root.code,
            vec![MechSourceCode::String("before".to_string())]
        );
        assert_eq!(*root.inline_eval_counter.borrow(), 4);
        assert_eq!(*root.persistent_user_function_plan_depth.borrow(), 2);
        assert_eq!(*root.deferred_expression_solve_depth.borrow(), 3);
        assert_eq!(symbol_cell.addr(), symbol_cell_address);
        assert_eq!(symbol_backing.addr(), symbol_backing_address);
        assert_eq!(
            ReactiveCellId::new(symbol_backing.id()),
            symbol_backing_identity
        );
        assert_eq!(*symbol_backing.borrow(), 1.0);
        assert!(root.sub_interpreters.borrow().get(&999).is_none());

        let restored_child = root
            .sub_interpreters
            .borrow()
            .get(&child_id)
            .cloned()
            .unwrap();
        assert_eq!(restored_child.addr(), child_handle_address);
        let restored_grandchild = {
            let child = restored_child.borrow();
            assert_eq!(child.id, child_id);
            assert_eq!(child.ip, 8);
            assert_eq!(*child_backing.borrow(), 20.0);
            child
                .sub_interpreters
                .borrow()
                .get(&grandchild_id)
                .cloned()
                .unwrap()
        };
        assert_eq!(restored_grandchild.addr(), grandchild_handle_address);
        let grandchild = restored_grandchild.borrow();
        assert_eq!(grandchild.id, grandchild_id);
        assert_eq!(grandchild.ip, 9);
        assert_eq!(*grandchild_backing.borrow(), 30.0);
    }

}
