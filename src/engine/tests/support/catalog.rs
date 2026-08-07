use mech_core::MResult;
use mech_core::{FunctionCatalog, FunctionCatalogBuilder};
use std::sync::Arc;

/// Installs the concrete factories owned by the engine's intrinsic fragment.
///
/// Standard distribution composition lives outside the engine; this narrow
/// installer exists so composition crates can include engine-owned operations.
pub fn install_intrinsic_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_runtime(builder)
}

/// Installs the source specializers owned by the engine's intrinsic fragment.
#[cfg(feature = "source")]
pub fn install_intrinsic_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_source(builder)
}

/// Returns a new empty catalog for a bare engine instance.
pub fn empty_function_catalog() -> Arc<FunctionCatalog> {
    Arc::new(FunctionCatalog::empty())
}

/// Builds the explicitly injected catalog used by source-behavior unit tests.
///
/// This deliberately contains no standard-machine implementations. The small
/// arithmetic and comparison specializers below are test doubles: they prove
/// engine behavior against caller-supplied operations without restoring a
/// distribution dependency or fallback.
pub(crate) fn function_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    install_intrinsic_runtime(&mut builder)
        .expect("engine intrinsic runtime catalog must be valid");
    #[cfg(feature = "source")]
    install_intrinsic_source(&mut builder).expect("engine intrinsic source catalog must be valid");
    test_operations::install(&mut builder).expect("engine test catalog must be valid");
    Arc::new(
        builder
            .build()
            .expect("engine intrinsic catalog must be valid"),
    )
}

#[cfg(test)]
mod test_operations {
    use mech_core::*;
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug)]
    enum BinaryArithmetic {
        Add,
        Subtract,
        Multiply,
        Divide,
    }

    #[derive(Debug)]
    struct BinaryArithmeticFunction {
        operation: BinaryArithmetic,
        lhs: Ref<f64>,
        rhs: Ref<f64>,
        out: Ref<f64>,
    }

    impl MechFunctionImpl for BinaryArithmeticFunction {
        fn solve(&self) {
            let lhs = *self.lhs.borrow();
            let rhs = *self.rhs.borrow();
            *self.out.borrow_mut() = match self.operation {
                BinaryArithmetic::Add => lhs + rhs,
                BinaryArithmetic::Subtract => lhs - rhs,
                BinaryArithmetic::Multiply => lhs * rhs,
                BinaryArithmetic::Divide => lhs / rhs,
            };
        }

        fn out(&self) -> Value {
            Value::F64(self.out.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for BinaryArithmeticFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct BinaryArithmeticSpecializer(BinaryArithmetic);

    impl FunctionSpecializer for BinaryArithmeticSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [lhs, rhs] = arguments else {
                return Err(test_operation_error(
                    "binary arithmetic expects two arguments",
                ));
            };
            Ok(Box::new(BinaryArithmeticFunction {
                operation: self.0,
                lhs: lhs.as_f64()?,
                rhs: rhs.as_f64()?,
                out: Ref::new(0.0),
            }))
        }
    }

    #[derive(Debug)]
    struct NegateFunction {
        input: Ref<f64>,
        out: Ref<f64>,
    }

    impl MechFunctionImpl for NegateFunction {
        fn solve(&self) {
            *self.out.borrow_mut() = -*self.input.borrow();
        }

        fn out(&self) -> Value {
            Value::F64(self.out.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            "TestNegate".to_string()
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for NegateFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct NegateSpecializer;

    impl FunctionSpecializer for NegateSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [input] = arguments else {
                return Err(test_operation_error("negation expects one argument"));
            };
            Ok(Box::new(NegateFunction {
                input: input.as_f64()?,
                out: Ref::new(0.0),
            }))
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Comparison {
        Equal,
        NotEqual,
        Less,
        Greater,
        LessEqual,
        GreaterEqual,
    }

    #[derive(Debug)]
    struct ComparisonFunction {
        operation: Comparison,
        lhs: Value,
        rhs: Value,
        out: Ref<bool>,
    }

    impl MechFunctionImpl for ComparisonFunction {
        fn solve(&self) {
            *self.out.borrow_mut() = match self.operation {
                Comparison::Equal => values_equal(&self.lhs, &self.rhs),
                Comparison::NotEqual => !values_equal(&self.lhs, &self.rhs),
                Comparison::Less => numeric_pair(&self.lhs, &self.rhs, |a, b| a < b),
                Comparison::Greater => numeric_pair(&self.lhs, &self.rhs, |a, b| a > b),
                Comparison::LessEqual => numeric_pair(&self.lhs, &self.rhs, |a, b| a <= b),
                Comparison::GreaterEqual => numeric_pair(&self.lhs, &self.rhs, |a, b| a >= b),
            };
        }

        fn out(&self) -> Value {
            Value::Bool(self.out.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for ComparisonFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct ComparisonSpecializer(Comparison);

    impl FunctionSpecializer for ComparisonSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [lhs, rhs] = arguments else {
                return Err(test_operation_error("comparison expects two arguments"));
            };
            Ok(Box::new(ComparisonFunction {
                operation: self.0,
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out: Ref::new(false),
            }))
        }

        fn guard_safety(&self) -> GuardFunctionSafety {
            GuardFunctionSafety::PureStatic
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum BooleanOperation {
        And,
        Or,
        Xor,
    }

    #[derive(Debug)]
    struct BooleanFunction {
        operation: BooleanOperation,
        lhs: Ref<bool>,
        rhs: Ref<bool>,
        out: Ref<bool>,
    }

    impl MechFunctionImpl for BooleanFunction {
        fn solve(&self) {
            let lhs = *self.lhs.borrow();
            let rhs = *self.rhs.borrow();
            *self.out.borrow_mut() = match self.operation {
                BooleanOperation::And => lhs && rhs,
                BooleanOperation::Or => lhs || rhs,
                BooleanOperation::Xor => lhs ^ rhs,
            };
        }

        fn out(&self) -> Value {
            Value::Bool(self.out.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for BooleanFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct BooleanSpecializer(BooleanOperation);

    impl FunctionSpecializer for BooleanSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [lhs, rhs] = arguments else {
                return Err(test_operation_error(
                    "boolean operation expects two arguments",
                ));
            };
            Ok(Box::new(BooleanFunction {
                operation: self.0,
                lhs: lhs.as_bool()?,
                rhs: rhs.as_bool()?,
                out: Ref::new(false),
            }))
        }

        fn guard_safety(&self) -> GuardFunctionSafety {
            GuardFunctionSafety::PureStatic
        }
    }

    #[derive(Debug)]
    struct NotFunction {
        input: Ref<bool>,
        out: Ref<bool>,
    }

    impl MechFunctionImpl for NotFunction {
        fn solve(&self) {
            *self.out.borrow_mut() = !*self.input.borrow();
        }

        fn out(&self) -> Value {
            Value::Bool(self.out.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            "TestNot".to_string()
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for NotFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct NotSpecializer;

    impl FunctionSpecializer for NotSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [input] = arguments else {
                return Err(test_operation_error("boolean not expects one argument"));
            };
            Ok(Box::new(NotFunction {
                input: input.as_bool()?,
                out: Ref::new(false),
            }))
        }

        fn guard_safety(&self) -> GuardFunctionSafety {
            GuardFunctionSafety::PureStatic
        }
    }

    #[derive(Debug)]
    struct AddAssignFunction {
        sink: Ref<f64>,
        source: Ref<f64>,
    }

    impl MechFunctionImpl for AddAssignFunction {
        fn solve(&self) {
            let source = *self.source.borrow();
            *self.sink.borrow_mut() += source;
        }

        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
            let next = *self.sink.borrow() + *self.source.borrow();
            Ok(Box::new(ReactiveRegisterWrite::new(
                self.sink.clone(),
                next,
                self.reactive_output_cell_ids(),
            )))
        }

        fn out(&self) -> Value {
            Value::F64(self.sink.clone())
        }

        fn reactive_node_kind(&self) -> ReactiveNodeKind {
            ReactiveNodeKind::Register
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            "TestAddAssign".to_string()
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for AddAssignFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct AddAssignSpecializer;

    impl FunctionSpecializer for AddAssignSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [sink, source] = arguments else {
                return Err(test_operation_error("add assignment expects two arguments"));
            };
            Ok(Box::new(AddAssignFunction {
                sink: sink.as_f64()?,
                source: source.as_f64()?,
            }))
        }
    }

    fn values_equal(lhs: &Value, rhs: &Value) -> bool {
        match (lhs, rhs) {
            (Value::MutableReference(lhs), rhs) => values_equal(&lhs.borrow(), rhs),
            (lhs, Value::MutableReference(rhs)) => values_equal(lhs, &rhs.borrow()),
            (Value::Typed(lhs, _), rhs) => values_equal(lhs, rhs),
            (lhs, Value::Typed(rhs, _)) => values_equal(lhs, rhs),
            _ => lhs == rhs,
        }
    }

    fn numeric_pair(lhs: &Value, rhs: &Value, compare: impl FnOnce(f64, f64) -> bool) -> bool {
        match (lhs.as_f64(), rhs.as_f64()) {
            (Ok(lhs), Ok(rhs)) => compare(*lhs.borrow(), *rhs.borrow()),
            _ => false,
        }
    }

    fn test_operation_error(message: &str) -> MechError {
        MechError::new(
            GenericError {
                msg: message.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    }

    pub(super) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
        for (name, operation) in [
            ("math/add", BinaryArithmetic::Add),
            ("math/sub", BinaryArithmetic::Subtract),
            ("math/mul", BinaryArithmetic::Multiply),
            ("math/div", BinaryArithmetic::Divide),
        ] {
            builder.insert_intrinsic_specializer(
                name,
                Arc::new(BinaryArithmeticSpecializer(operation)),
            )?;
        }
        builder.insert_intrinsic_specializer("math/neg", Arc::new(NegateSpecializer))?;

        for (name, operation) in [
            ("compare/eq", Comparison::Equal),
            ("compare/neq", Comparison::NotEqual),
            ("compare/lt", Comparison::Less),
            ("compare/gt", Comparison::Greater),
            ("compare/lte", Comparison::LessEqual),
            ("compare/gte", Comparison::GreaterEqual),
        ] {
            builder
                .insert_intrinsic_specializer(name, Arc::new(ComparisonSpecializer(operation)))?;
        }

        for (name, operation) in [
            ("logic/and", BooleanOperation::And),
            ("logic/or", BooleanOperation::Or),
            ("logic/xor", BooleanOperation::Xor),
        ] {
            builder.insert_intrinsic_specializer(name, Arc::new(BooleanSpecializer(operation)))?;
        }
        builder.insert_intrinsic_specializer("logic/not", Arc::new(NotSpecializer))?;
        builder.insert_intrinsic_specializer("math/add-assign", Arc::new(AddAssignSpecializer))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Interpreter;
    #[cfg(feature = "program")]
    use crate::{ExtensionFunctionId, MechProgram, MechProgramConfig};
    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64",
        feature = "compiler"
    ))]
    use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    use mech_core::{
        FunctionExport, FunctionExposure, FunctionSpecializer, MechFunction, MechFunctionImpl, Ref,
        Value,
    };

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    struct TestAddFunction {
        lhs: Ref<f64>,
        rhs: Ref<f64>,
        out: Ref<f64>,
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    impl MechFunctionImpl for TestAddFunction {
        fn solve(&self) {
            *self.out.borrow_mut() = *self.lhs.borrow() + *self.rhs.borrow();
        }

        fn out(&self) -> Value {
            Value::F64(self.out.clone())
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(Vec::new())
        }

        fn to_string(&self) -> String {
            String::from("TestAddFunction")
        }
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64",
        feature = "compiler"
    ))]
    impl MechFunctionCompiler for TestAddFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    struct TestAddSpecializer;

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    impl FunctionSpecializer for TestAddSpecializer {
        fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
            let [Value::F64(lhs), Value::F64(rhs)] = arguments else {
                panic!("test math/add expects two f64 arguments");
            };
            let out = Ref::new(*lhs.borrow() + *rhs.borrow());
            Ok(Box::new(TestAddFunction {
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                out,
            }))
        }
    }

    #[test]
    fn empty_catalog_has_no_function_surface() {
        let catalog = empty_function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[test]
    fn empty_catalog_is_not_cached() {
        let first = empty_function_catalog();
        let second = empty_function_catalog();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn interpreter_new_has_an_empty_catalog() {
        let interpreter = Interpreter::new(41, 100);
        let catalog = interpreter.function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[cfg(feature = "program")]
    #[test]
    fn program_new_has_an_empty_catalog() {
        let program = MechProgram::new(MechProgramConfig::default());
        let catalog = program.function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[cfg(feature = "program")]
    #[test]
    fn bare_programs_have_independent_catalogs_and_environments() {
        let first = MechProgram::new(MechProgramConfig::default());
        let second = MechProgram::new(MechProgramConfig::default());
        let extension = ExtensionFunctionId::from_name("host/first-only");

        assert!(!Arc::ptr_eq(
            first.function_catalog(),
            second.function_catalog()
        ));

        first
            .interpreter()
            .state
            .borrow_mut()
            .function_environment
            .bind_extension("host/first-only", "first-only", extension)
            .unwrap();

        assert_eq!(
            first
                .interpreter()
                .state
                .borrow()
                .function_environment
                .resolve_name("first-only"),
            Some(crate::FunctionBinding::Extension(extension)),
        );
        assert_eq!(
            second
                .interpreter()
                .state
                .borrow()
                .function_environment
                .resolve_name("first-only"),
            None,
        );
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    #[test]
    fn bare_program_cannot_execute_math_add() {
        let mut program = MechProgram::new(MechProgramConfig::default());

        let error = program.run_string("1.0 + 2.0").unwrap_err();

        assert_eq!(error.kind_name(), "FunctionOperationUnavailable");
        assert!(error.kind_message().contains("math/add"));
    }

    #[cfg(all(feature = "program", feature = "source"))]
    #[test]
    fn bare_program_does_not_know_standard_modules() {
        let mut program = MechProgram::new(MechProgramConfig::default());

        assert!(!program.function_catalog().has_module("math"));
        let error = program.run_string("+> math").unwrap_err();

        assert_eq!(error.kind_name(), "MissingFunction");
    }

    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64"
    ))]
    #[test]
    fn supplied_custom_catalog_executes_math_add() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_specializer("math/add", Arc::new(TestAddSpecializer))
            .unwrap();
        builder
            .insert_export(FunctionExport {
                operation,
                canonical_name: String::from("math/add"),
                module: None,
                item: None,
                exposure: FunctionExposure::Prelude,
            })
            .unwrap();
        let catalog = Arc::new(builder.build().unwrap());
        let mut program = MechProgram::with_function_catalog(MechProgramConfig::default(), catalog);

        let output = program.run_string("1.0 + 2.0").unwrap();

        let Value::F64(output) = output else {
            panic!("custom math/add must return f64");
        };
        assert_eq!(*output.borrow(), 3.0);
    }
}
