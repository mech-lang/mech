#[cfg(feature = "compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    FunctionArgs, FunctionDefinition, Functions, FunctionsSnapshot, MechFunction, MechFunctionImpl,
    NativeFunctionCompiler, Plan, TransactionStateUnsupportedError,
};
use super::support::{PureStaticTestCompiler, TestFunction};
#[cfg(feature = "compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{
    Dictionary, FunctionDefine, MResult, MechError, Ref, ValRef, Value, ValueStateJournal,
    internal_pattern_value_identifier,
};
use std::sync::Arc;

fn original_test_factory(_arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    Ok(Box::new(TestFunction::new("original factory")))
}

fn replacement_test_factory(_arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    Ok(Box::new(TestFunction::new("replacement factory")))
}

fn empty_function_definition(id: u64, name: &str) -> FunctionDefinition {
    FunctionDefinition::new(
        id,
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

struct UnsupportedStateFunction;
impl MechFunctionImpl for UnsupportedStateFunction {
    fn solve(&self) {}
    fn out(&self) -> Value {
        Value::Empty
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: self.to_string(),
                reason: "test-only opaque state".to_string(),
            },
            None,
        ))
    }
    fn to_string(&self) -> String {
        "unsupported".to_string()
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for UnsupportedStateFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct MisleadingRuntimeHostNameFunction {
    output: ValRef,
}
impl MechFunctionImpl for MisleadingRuntimeHostNameFunction {
    fn solve(&self) {}
    fn out(&self) -> Value {
        Value::MutableReference(self.output.clone())
    }
    fn to_string(&self) -> String {
        "RuntimeHostNativeFunction::misleading-name".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for MisleadingRuntimeHostNameFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct UnschedulableOutputFunction {
    state: ValRef,
}
impl MechFunctionImpl for UnschedulableOutputFunction {
    fn solve(&self) {}
    fn out(&self) -> Value {
        Value::MutableReference(self.state.clone())
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        Vec::new()
    }
    fn to_string(&self) -> String {
        "unschedulable-output".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for UnschedulableOutputFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn functions_snapshot_restores_containers_and_nested_user_structure() {
    let target = Ref::new(Functions::new());
    let original_dictionary = target.borrow().dictionary.clone();
    let original_compiler: Arc<dyn NativeFunctionCompiler> = Arc::new(PureStaticTestCompiler);
    let function_id = 1;
    let compiler_id = 2;
    let user_id = 3;
    let symbol_id = 4;
    let mut definition = empty_function_definition(user_id, "user");
    *definition.out.borrow_mut() = Value::Index(Ref::new(10));
    let symbol =
        definition
            .symbols
            .borrow_mut()
            .insert(symbol_id, Value::Index(Ref::new(20)), true);
    let original_symbol_dictionary = definition.symbols.borrow().dictionary.clone();
    original_symbol_dictionary
        .borrow_mut()
        .insert(symbol_id, "symbol".to_string());
    definition
        .plan
        .add_function(Box::new(TestFunction::new("nested")));
    let original_symbols = definition.symbols.clone();
    let original_out = definition.out.clone();
    let original_plan = definition.plan.clone();
    let original_plan_checkpoint = original_plan.checkpoint();
    {
        let mut functions = target.borrow_mut();
        functions
            .functions
            .insert(function_id, original_test_factory);
        functions
            .function_compilers
            .insert(compiler_id, original_compiler.clone());
        functions.user_functions.insert(user_id, definition);
        functions
            .dictionary
            .borrow_mut()
            .insert(function_id, "original".to_string());
    }

    let snapshot = FunctionsSnapshot::capture(&target).unwrap();
    let mut journal = ValueStateJournal::new();
    for value in snapshot.transaction_state_values().unwrap() {
        journal.capture_value(&value).unwrap();
    }

    *original_out.borrow_mut() = Value::Index(Ref::new(99));
    *symbol.borrow_mut() = Value::Index(Ref::new(98));
    {
        let mut symbols = original_symbols.borrow_mut();
        symbols.symbols.clear();
        symbols.mutable_variables.clear();
        symbols.dictionary = Ref::new(Dictionary::new());
    }
    {
        let mut nested = original_plan.borrow_mut();
        nested.nodes[0].plan_index = 88;
        nested.push(Box::new(TestFunction::new("nested tail")));
    }
    let replacement_dictionary = Ref::new(Dictionary::new());
    {
        let mut functions = target.borrow_mut();
        functions.functions.clear();
        functions
            .functions
            .insert(function_id, replacement_test_factory);
        functions.function_compilers.clear();
        functions.user_functions.clear();
        functions.dictionary = replacement_dictionary;
    }

    snapshot.preflight_restore().unwrap();
    journal.preflight_restore_before().unwrap();
    snapshot.apply_restore();
    journal.apply_restore_before();

    assert_eq!(
        target.borrow().dictionary.addr(),
        original_dictionary.addr()
    );
    assert_eq!(
        target.borrow().dictionary.borrow().get(&function_id),
        Some(&"original".to_string()),
    );
    assert_eq!(
        *target.borrow().functions.get(&function_id).unwrap() as usize,
        original_test_factory as usize,
    );
    let restored_compiler = target
        .borrow()
        .function_compilers
        .get(&compiler_id)
        .unwrap()
        .clone();
    assert!(Arc::ptr_eq(&restored_compiler, &original_compiler));
    let functions = target.borrow();
    let restored = functions.user_functions.get(&user_id).unwrap();
    assert_eq!(restored.symbols.addr(), original_symbols.addr());
    assert_eq!(restored.out.addr(), original_out.addr());
    assert_eq!(restored.plan.0.addr(), original_plan.0.addr());
    assert_eq!(
        restored.symbols.borrow().dictionary.addr(),
        original_symbol_dictionary.addr(),
    );
    assert_eq!(restored.plan.checkpoint(), original_plan_checkpoint);
    assert_eq!(
        restored
            .symbols
            .borrow()
            .symbols
            .get(&symbol_id)
            .unwrap()
            .addr(),
        symbol.addr(),
    );
}

#[test]
fn functions_snapshot_preflight_failure_is_atomic() {
    let target = Ref::new(Functions::new());
    target
        .borrow_mut()
        .functions
        .insert(1, original_test_factory);
    let snapshot = FunctionsSnapshot::capture(&target).unwrap();
    target.borrow_mut().functions.clear();
    let dictionary = snapshot.dictionary_target.borrow();

    let error = snapshot.preflight_restore().unwrap_err();

    assert_eq!(error.kind_name(), "FunctionsSnapshotBorrowConflict");
    assert!(target.borrow().functions.is_empty());
    drop(dictionary);
}

#[test]
fn transaction_state_unsupported_error_is_structured() {
    let function = UnsupportedStateFunction;
    let error = function.transaction_state_values().unwrap_err();
    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
}

#[test]
fn host_like_display_name_does_not_change_checkpoint_behavior() {
    let plan = Plan::new();
    let output = Ref::new(Value::Index(Ref::new(42)));
    plan.add_function(Box::new(MisleadingRuntimeHostNameFunction {
        output: output.clone(),
    }));

    let values = plan.transaction_state_values().unwrap();

    assert!(values.iter().any(|value| matches!(
      value,
      Value::MutableReference(cell) if cell.same_handle(&output)
    ),));
}

#[test]
fn plan_transaction_state_retains_outputs_excluded_from_scheduling() {
    let state = Ref::new(Value::Index(Ref::new(1)));
    let plan = Plan::new();
    plan.add_function(Box::new(UnschedulableOutputFunction {
        state: state.clone(),
    }));

    let values = plan.transaction_state_values().unwrap();

    assert!(values.iter().any(
        |value| matches!(value, Value::MutableReference(cell) if cell.addr() == state.addr())
    ));
}
