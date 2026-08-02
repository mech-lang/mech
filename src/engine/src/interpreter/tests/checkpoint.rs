#[cfg(all(test, feature = "functions", feature = "symbol_table", feature = "f64"))]
mod checkpoint_tests {
    use super::super::super::{
        Dictionary, ExtensionFunctionId, FunctionBinding, FunctionCatalogBuilder, FunctionDefine,
        FunctionDefinition, FunctionExport, FunctionExposure, FunctionExtensionEntry,
        FunctionSpecializer, Interpreter, MResult, MechFunction, MechSourceCode,
        ModuleManifestCatalog, OperationId, ProgramState, ReactiveCellId, Ref,
        RuntimeContextBinding, ValRef, Value, ValueStateBorrowConflict, hash_str,
        internal_pattern_value_identifier,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    #[cfg(feature = "invariant_define")]
    use super::super::super::{ComparisonOp, FormulaOperator, IntegrityConstraint};
    #[cfg(feature = "state_machines")]
    use super::super::super::{FsmImplementation, FsmSpecification, Pattern};

    fn f64_value(value: &Ref<f64>) -> Value {
        Value::F64(value.clone())
    }

    struct CheckpointSpecializer(u8);

    impl FunctionSpecializer for CheckpointSpecializer {
        fn specialize(&self, _: &[Value]) -> MResult<Box<dyn MechFunction>> {
            unreachable!("checkpoint store tests do not specialize marker {}", self.0)
        }
    }

    fn empty_user_function(name: &str) -> FunctionDefinition {
        FunctionDefinition::new(
            hash_str(name),
            name.to_string(),
            FunctionDefine {
                name: internal_pattern_value_identifier(name),
                input: Vec::new(),
                output: Vec::new(),
                statements: Vec::new(),
                match_arms: Vec::new(),
            },
        )
    }

    fn index_payload(value: &Ref<Value>) -> usize {
        let value = value.borrow();
        let Value::Index(index) = &*value else {
            panic!("expected retained index value, got {value:?}")
        };
        *index.borrow()
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

    #[cfg(feature = "math_add")]
    #[test]
    fn catalog_identity_and_function_environment_survive_children_clear_and_restore() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_specializer("math/add", Arc::new(CheckpointSpecializer(1)))
            .unwrap();
        let operation = OperationId::from_name("math/add");
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: "math/add".to_string(),
                module: Some("math".to_string()),
                item: Some("plus".to_string()),
                exposure: FunctionExposure::ModuleOnly,
            })
            .unwrap();
        let catalog = Arc::new(builder.build().unwrap());
        let mut interpreter = Interpreter::with_function_catalog(41, 100, Arc::clone(&catalog));

        assert!(Arc::ptr_eq(interpreter.function_catalog(), &catalog));
        assert!(
            interpreter
                .state
                .borrow()
                .function_environment
                .operation_is_enabled(operation)
        );

        let extension_name = "host/child-marker";
        let extension = ExtensionFunctionId::from_name(extension_name);
        let user_name = "user/child-marker";
        {
            let mut state = interpreter.state.borrow_mut();
            state
                .function_extensions
                .insert_or_replace(FunctionExtensionEntry::new(
                    extension_name,
                    Arc::new(CheckpointSpecializer(0)),
                ))
                .unwrap();
            state
                .function_environment
                .bind_extension(extension_name, "child-marker", extension)
                .unwrap();
            state
                .user_functions
                .insert_or_replace(empty_user_function(user_name))
                .unwrap();
        }

        let cloned = interpreter.clone();
        assert!(Arc::ptr_eq(cloned.function_catalog(), &catalog));

        let child = interpreter.new_child_interpreter(42, 100);
        assert!(Arc::ptr_eq(child.function_catalog(), &catalog));
        assert!(
            child
                .state
                .borrow()
                .function_environment
                .operation_is_enabled(operation)
        );
        assert_eq!(
            child
                .state
                .borrow()
                .function_environment
                .resolve_name("child-marker"),
            Some(FunctionBinding::Extension(extension)),
        );
        assert!(
            child
                .state
                .borrow()
                .function_extensions
                .entry(extension)
                .is_some()
        );
        assert!(
            child
                .state
                .borrow()
                .user_functions
                .resolve_name(user_name)
                .is_some()
        );
        let environment_before = interpreter.state.borrow().function_environment.clone();
        assert_eq!(environment_before.resolve_name("plus"), None);
        let checkpoint = interpreter.checkpoint().unwrap();
        let module_only_export = catalog.module_export("math", "plus").unwrap().clone();
        interpreter
            .state
            .borrow_mut()
            .function_environment
            .bind_catalog_export(&module_only_export, "plus")
            .unwrap();
        assert_eq!(
            interpreter
                .state
                .borrow()
                .function_environment
                .resolve_name("plus"),
            Some(FunctionBinding::CatalogOperation(operation)),
        );
        assert_ne!(
            interpreter.state.borrow().function_environment,
            environment_before,
        );

        interpreter.restore(checkpoint).unwrap();
        assert!(Arc::ptr_eq(interpreter.function_catalog(), &catalog));
        assert_eq!(
            interpreter.state.borrow().function_environment,
            environment_before,
        );
        assert_eq!(
            interpreter
                .state
                .borrow()
                .function_environment
                .resolve_name("plus"),
            None,
        );

        interpreter.clear();
        assert!(Arc::ptr_eq(interpreter.function_catalog(), &catalog));
        assert!(
            interpreter
                .state
                .borrow()
                .function_environment
                .operation_is_enabled(operation)
        );
    }

    #[test]
    fn checkpoint_restores_extension_entries_exports_bindings_and_arc_identity() {
        let mut interpreter = Interpreter::new(43, 100);
        let canonical_name = "host/read";
        let extension = ExtensionFunctionId::from_name(canonical_name);
        let original: Arc<dyn FunctionSpecializer> = Arc::new(CheckpointSpecializer(1));
        {
            let mut state = interpreter.state.borrow_mut();
            state
                .function_extensions
                .insert_or_replace(FunctionExtensionEntry::new(
                    canonical_name,
                    Arc::clone(&original),
                ))
                .unwrap();
            state
                .function_extensions
                .insert_module_export_or_replace("dynamic-host", "read", extension)
                .unwrap();
            state
                .function_environment
                .bind_extension(canonical_name, "read", extension)
                .unwrap();
        }

        let catalog = Arc::clone(interpreter.function_catalog());
        let checkpoint = interpreter.checkpoint().unwrap();
        let replacement: Arc<dyn FunctionSpecializer> = Arc::new(CheckpointSpecializer(2));
        let added_name = "dynamic-host/write";
        let added = ExtensionFunctionId::from_name(added_name);
        {
            let mut state = interpreter.state.borrow_mut();
            state
                .function_extensions
                .insert_or_replace(FunctionExtensionEntry::new(
                    canonical_name,
                    Arc::clone(&replacement),
                ))
                .unwrap();
            state
                .function_extensions
                .insert_or_replace(FunctionExtensionEntry::new(
                    added_name,
                    Arc::new(CheckpointSpecializer(3)),
                ))
                .unwrap();
            state
                .function_extensions
                .insert_module_export_or_replace("dynamic-host", "write", added)
                .unwrap();
            state
                .function_environment
                .bind_extension(added_name, "write", added)
                .unwrap();
        }

        interpreter.restore(checkpoint).unwrap();

        assert!(Arc::ptr_eq(interpreter.function_catalog(), &catalog));
        let state = interpreter.state.borrow();
        assert!(Arc::ptr_eq(
            &state
                .function_extensions
                .entry(extension)
                .unwrap()
                .specializer,
            &original,
        ));
        assert!(!Arc::ptr_eq(
            &state
                .function_extensions
                .entry(extension)
                .unwrap()
                .specializer,
            &replacement,
        ));
        assert_eq!(
            state
                .function_extensions
                .module_export("dynamic-host", "read"),
            Some(extension),
        );
        assert_eq!(
            state
                .function_extensions
                .module_export("dynamic-host", "write"),
            None,
        );
        assert!(state.function_extensions.entry(added).is_none());
        assert_eq!(
            state.function_environment.resolve_name("read"),
            Some(FunctionBinding::Extension(extension)),
        );
        assert_eq!(state.function_environment.resolve_name("write"), None);
    }

    #[test]
    fn checkpoint_restores_user_function_definitions_and_retained_state() {
        let mut interpreter = Interpreter::new(44, 100);
        let function_name = "user/checkpoint";
        let added_name = "user/added-after-checkpoint";
        let symbol_id = hash_str("retained");
        let mut definition = empty_user_function(function_name);
        *definition.out.borrow_mut() = Value::Index(Ref::new(10));
        let symbol =
            definition
                .symbols
                .borrow_mut()
                .insert(symbol_id, Value::Index(Ref::new(20)), true);
        let original_symbol_dictionary = definition.symbols.borrow().dictionary.clone();
        original_symbol_dictionary
            .borrow_mut()
            .insert(symbol_id, "retained".to_string());
        definition
            .plan
            .push_activation_registration_scope(vec![ReactiveCellId::new(1)]);
        let original_symbols = definition.symbols.clone();
        let original_out = definition.out.clone();
        let original_plan = definition.plan.clone();
        let original_plan_checkpoint = original_plan.checkpoint();
        interpreter
            .state
            .borrow_mut()
            .user_functions
            .insert_or_replace(definition)
            .unwrap();

        let checkpoint = interpreter.checkpoint().unwrap();

        *original_out.borrow_mut() = Value::Index(Ref::new(99));
        *symbol.borrow_mut() = Value::Index(Ref::new(98));
        {
            let mut symbols = original_symbols.borrow_mut();
            symbols.symbols.clear();
            symbols.mutable_variables.clear();
            symbols.dictionary = Ref::new(Dictionary::new());
        }
        original_plan.push_activation_registration_scope(vec![ReactiveCellId::new(2)]);
        {
            let mut state = interpreter.state.borrow_mut();
            state
                .user_functions
                .insert_or_replace(empty_user_function(function_name))
                .unwrap();
            state
                .user_functions
                .insert_or_replace(empty_user_function(added_name))
                .unwrap();
        }

        interpreter.restore(checkpoint).unwrap();

        let state = interpreter.state.borrow();
        assert!(state.user_functions.resolve_name(added_name).is_none());
        let restored = state.user_functions.resolve_name(function_name).unwrap();
        assert_eq!(restored.symbols.addr(), original_symbols.addr());
        assert_eq!(restored.out.addr(), original_out.addr());
        assert_eq!(restored.plan.0.addr(), original_plan.0.addr());
        assert_eq!(
            restored.symbols.borrow().dictionary.addr(),
            original_symbol_dictionary.addr(),
        );
        assert_eq!(restored.plan.checkpoint(), original_plan_checkpoint);
        assert_eq!(index_payload(&restored.out), 10);
        let restored_symbol = restored
            .symbols
            .borrow()
            .symbols
            .get(&symbol_id)
            .unwrap()
            .clone();
        assert_eq!(restored_symbol.addr(), symbol.addr());
        assert_eq!(index_payload(&restored_symbol), 20);
        assert_eq!(
            restored
                .symbols
                .borrow()
                .dictionary
                .borrow()
                .get(&symbol_id),
            Some(&"retained".to_string()),
        );
    }

    #[test]
    fn user_function_restore_preflight_failure_is_atomic() {
        let mut interpreter = Interpreter::new(45, 100);
        let function_name = "user/atomic";
        let definition = empty_user_function(function_name);
        let original_symbols = definition.symbols.clone();
        interpreter
            .state
            .borrow_mut()
            .user_functions
            .insert_or_replace(definition)
            .unwrap();
        let checkpoint = interpreter.checkpoint().unwrap();

        let replacement = empty_user_function(function_name);
        let replacement_symbols = replacement.symbols.clone();
        interpreter
            .state
            .borrow_mut()
            .user_functions
            .insert_or_replace(replacement)
            .unwrap();
        let held_symbols = original_symbols.borrow();

        let error = interpreter.restore(checkpoint).unwrap_err();

        assert_eq!(error.kind_name(), "UserFunctionsCheckpointBorrowConflict");
        assert_eq!(
            interpreter
                .state
                .borrow()
                .user_functions
                .resolve_name(function_name)
                .unwrap()
                .symbols
                .addr(),
            replacement_symbols.addr(),
        );
        drop(held_symbols);
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
