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
    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        any(feature = "math_add_assign", feature = "range_inclusive")
    ))]
    use mech_core::matrix::Matrix;
    use mech_core::*;
    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    use nalgebra::DVector;
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
        fn solve_result(&self) -> MResult<()> {
            let lhs = *self.lhs.borrow();
            let rhs = *self.rhs.borrow();
            *self.out.borrow_mut() = match self.operation {
                BinaryArithmetic::Add => lhs + rhs,
                BinaryArithmetic::Subtract => lhs - rhs,
                BinaryArithmetic::Multiply => lhs * rhs,
                BinaryArithmetic::Divide => lhs / rhs,
            };
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<mech_core::FunctionStatePort<'_>> {
            Some(mech_core::FunctionStatePort::from_ref(&self.out))
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for BinaryArithmeticFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct BinaryArithmeticSpecializer(BinaryArithmetic);

    impl FunctionSpecializer for BinaryArithmeticSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        fn solve_result(&self) -> MResult<()> {
            *self.out.borrow_mut() = -*self.input.borrow();
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.out))
        }

        fn to_string(&self) -> String {
            "TestNegate".to_string()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for NegateFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct NegateSpecializer;

    impl FunctionSpecializer for NegateSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        lhs: LegacyValue,
        rhs: LegacyValue,
        out: Ref<bool>,
    }

    impl MechFunctionImpl for ComparisonFunction {
        fn solve_result(&self) -> MResult<()> {
            *self.out.borrow_mut() = match self.operation {
                Comparison::Equal => values_equal(&self.lhs, &self.rhs),
                Comparison::NotEqual => !values_equal(&self.lhs, &self.rhs),
                Comparison::Less => numeric_pair(&self.lhs, &self.rhs, |a, b| a < b),
                Comparison::Greater => numeric_pair(&self.lhs, &self.rhs, |a, b| a > b),
                Comparison::LessEqual => numeric_pair(&self.lhs, &self.rhs, |a, b| a <= b),
                Comparison::GreaterEqual => numeric_pair(&self.lhs, &self.rhs, |a, b| a >= b),
            };
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.out))
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for ComparisonFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct ComparisonSpecializer(Comparison);

    impl FunctionSpecializer for ComparisonSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        fn solve_result(&self) -> MResult<()> {
            let lhs = *self.lhs.borrow();
            let rhs = *self.rhs.borrow();
            *self.out.borrow_mut() = match self.operation {
                BooleanOperation::And => lhs && rhs,
                BooleanOperation::Or => lhs || rhs,
                BooleanOperation::Xor => lhs ^ rhs,
            };
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.out))
        }

        fn to_string(&self) -> String {
            format!("Test{:?}", self.operation)
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for BooleanFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct BooleanSpecializer(BooleanOperation);

    impl FunctionSpecializer for BooleanSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        fn solve_result(&self) -> MResult<()> {
            *self.out.borrow_mut() = !*self.input.borrow();
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.out))
        }

        fn to_string(&self) -> String {
            "TestNot".to_string()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for NotFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct NotSpecializer;

    impl FunctionSpecializer for NotSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
        fn solve_result(&self) -> MResult<()> {
            let source = *self.source.borrow();
            *self.sink.borrow_mut() += source;
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.sink))
        }

        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
            let next = *self.sink.borrow() + *self.source.borrow();
            Ok(Box::new(ReactiveRegisterWrite::new(
                self.sink.clone(),
                next,
                self.reactive_output_cell_ids(),
            )))
        }

        fn reactive_node_kind(&self) -> ReactiveNodeKind {
            ReactiveNodeKind::Register
        }

        fn to_string(&self) -> String {
            "TestAddAssign".to_string()
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for AddAssignFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct AddAssignSpecializer;

    impl FunctionSpecializer for AddAssignSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            let [sink, source] = arguments else {
                return Err(test_operation_error("add assignment expects two arguments"));
            };
            Ok(Box::new(AddAssignFunction {
                sink: sink.as_f64()?,
                source: source.as_f64()?,
            }))
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    #[derive(Debug)]
    struct InclusiveRangeFunction {
        start: Ref<f64>,
        terminal: Ref<f64>,
        out: Ref<DVector<f64>>,
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    impl MechFunctionImpl for InclusiveRangeFunction {
        fn solve_result(&self) -> MResult<()> {
            let start = *self.start.borrow() as usize;
            let terminal = *self.terminal.borrow() as usize;
            *self.out.borrow_mut() =
                DVector::from_vec((start..=terminal).map(|value| value as f64).collect());
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
            Some(FunctionStatePort::from_ref(&self.out))
        }

        fn to_string(&self) -> String {
            "TestInclusiveRange".to_string()
        }
    }

    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        feature = "range_inclusive",
        feature = "semantic-compiler"
    ))]
    impl MechFunctionCompiler for InclusiveRangeFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    struct InclusiveRangeSpecializer;

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    impl FunctionSpecializer for InclusiveRangeSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            let [start, terminal] = arguments else {
                return Err(test_operation_error(
                    "inclusive range expects two arguments",
                ));
            };
            let start = start.as_f64()?;
            let terminal = terminal.as_f64()?;
            let out = Ref::new(DVector::from_vec(Vec::new()));
            let function = Self::function(start, terminal, out);
            function.solve_result()?;
            Ok(Box::new(function))
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
    impl InclusiveRangeSpecializer {
        fn function(
            start: Ref<f64>,
            terminal: Ref<f64>,
            out: Ref<DVector<f64>>,
        ) -> InclusiveRangeFunction {
            InclusiveRangeFunction {
                start,
                terminal,
                out,
            }
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "math_add_assign"))]
    fn test_f64_matrix(value: &LegacyValue) -> MResult<Matrix<f64>> {
        match value {
            LegacyValue::MatrixF64(matrix) => Ok(matrix.clone()),
            LegacyValue::MutableReference(value) => test_f64_matrix(&value.borrow()),
            _ => Err(test_operation_error(
                "matrix range/all add assignment expects an f64 matrix sink",
            )),
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "math_add_assign"))]
    #[derive(Debug)]
    struct AddAssignRangeAllFunction {
        sink: Matrix<f64>,
        source: LegacyValue,
        rows: Vec<usize>,
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "math_add_assign"))]
    impl MechFunctionImpl for AddAssignRangeAllFunction {
        fn solve_result(&self) -> MResult<()> {
            let shape = self.sink.shape();
            let mut sink_values = self.sink.as_vec();
            let source_matrix = test_f64_matrix(&self.source).ok();
            let source_scalar = self.source.as_f64().ok();
            for column in 0..shape[1] {
                for row in &self.rows {
                    let offset = column * shape[0] + row;
                    let addend = if let Some(source) = &source_scalar {
                        *source.borrow()
                    } else if let Some(source) = &source_matrix {
                        source.as_vec()[offset]
                    } else {
                        return Err(test_operation_error(
                            "matrix range/all add assignment expects an f64 source",
                        ));
                    };
                    sink_values[offset] += addend;
                }
            }
            self.sink.set(sink_values);
            Ok(())
        }

        fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
            vec![value_cell_from_legacy_function_value(
                LegacyValue::MatrixF64(self.sink.clone()),
            )]
        }

        fn to_string(&self) -> String {
            "TestAddAssignRangeAll".to_string()
        }
    }

    #[cfg(all(
        feature = "f64",
        feature = "matrix",
        feature = "math_add_assign",
        feature = "semantic-compiler"
    ))]
    impl MechFunctionCompiler for AddAssignRangeAllFunction {
        fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix", feature = "math_add_assign"))]
    struct AddAssignRangeAllSpecializer;

    #[cfg(all(feature = "f64", feature = "matrix", feature = "math_add_assign"))]
    impl FunctionSpecializer for AddAssignRangeAllSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            let [sink, source, rows, all] = arguments else {
                return Err(test_operation_error(
                    "matrix range/all add assignment expects four arguments",
                ));
            };
            if !matches!(all, LegacyValue::IndexAll) {
                return Err(test_operation_error(
                    "matrix range/all add assignment expects an all-column selector",
                ));
            }
            let rows = match rows {
                LegacyValue::MatrixIndex(rows) => {
                    rows.as_vec().into_iter().map(|row| row - 1).collect()
                }
                #[cfg(feature = "bool")]
                LegacyValue::MatrixBool(rows) => rows
                    .as_vec()
                    .into_iter()
                    .enumerate()
                    .filter_map(|(row, selected)| selected.then_some(row))
                    .collect(),
                _ => {
                    return Err(test_operation_error(
                        "matrix range/all add assignment expects row indices",
                    ));
                }
            };
            Ok(Box::new(AddAssignRangeAllFunction {
                sink: test_f64_matrix(sink)?,
                source: source.clone(),
                rows,
            }))
        }
    }

    fn values_equal(lhs: &LegacyValue, rhs: &LegacyValue) -> bool {
        match (lhs, rhs) {
            (LegacyValue::MutableReference(lhs), rhs) => values_equal(&lhs.borrow(), rhs),
            (lhs, LegacyValue::MutableReference(rhs)) => values_equal(lhs, &rhs.borrow()),
            (LegacyValue::Typed(lhs, _), rhs) => values_equal(lhs, rhs),
            (lhs, LegacyValue::Typed(rhs, _)) => values_equal(lhs, rhs),
            _ => lhs == rhs,
        }
    }

    fn numeric_pair(
        lhs: &LegacyValue,
        rhs: &LegacyValue,
        compare: impl FnOnce(f64, f64) -> bool,
    ) -> bool {
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
        #[cfg(all(feature = "f64", feature = "matrix", feature = "math_add_assign"))]
        builder.insert_intrinsic_specializer(
            "math/add-assign/range-all",
            Arc::new(AddAssignRangeAllSpecializer),
        )?;
        #[cfg(all(feature = "f64", feature = "matrix", feature = "range_inclusive"))]
        builder
            .insert_intrinsic_specializer("range/inclusive", Arc::new(InclusiveRangeSpecializer))?;
        Ok(())
    }
}

#[cfg(all(test, feature = "semantic-compiler"))]
mod tests {
    use super::*;
    use crate::Interpreter;
    #[cfg(feature = "program")]
    use crate::{CompilerPlanningConfig, CompilerPlanningProgram, ExtensionFunctionId};
    #[cfg(all(
        feature = "program",
        feature = "source",
        feature = "formulas",
        feature = "math_add",
        feature = "f64",
        feature = "semantic-compiler"
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
        FunctionExport, FunctionExposure, FunctionSpecializer, LegacyValue, MechFunction,
        MechFunctionImpl, Ref,
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
        fn solve_result(&self) -> MResult<()> {
            *self.out.borrow_mut() = *self.lhs.borrow() + *self.rhs.borrow();
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<mech_core::FunctionStatePort<'_>> {
            Some(mech_core::FunctionStatePort::from_ref(&self.out))
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
        feature = "semantic-compiler"
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
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            let [LegacyValue::F64(lhs), LegacyValue::F64(rhs)] = arguments else {
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
        let program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let catalog = program.function_catalog();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.intrinsic_specializer_count(), 0);
        assert_eq!(catalog.all_exports().len(), 0);
    }

    #[cfg(feature = "program")]
    #[test]
    fn bare_programs_have_independent_catalogs_and_environments() {
        let first = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let second = CompilerPlanningProgram::new(CompilerPlanningConfig::default());
        let extension = ExtensionFunctionId::from_name("host/first-only");

        assert!(!Arc::ptr_eq(
            first.function_catalog(),
            second.function_catalog()
        ));

        first
            .interpreter
            .state
            .borrow_mut()
            .function_environment
            .bind_extension("host/first-only", "first-only", extension)
            .unwrap();

        assert_eq!(
            first
                .interpreter
                .state
                .borrow()
                .function_environment
                .resolve_name("first-only"),
            Some(crate::FunctionBinding::Extension(extension)),
        );
        assert_eq!(
            second
                .interpreter
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
        let mut program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());

        let error = program.plan_source_for_test("1.0 + 2.0").unwrap_err();

        assert_eq!(error.kind_name(), "FunctionOperationUnavailable");
        assert!(error.kind_message().contains("math/add"));
    }

    #[cfg(all(feature = "program", feature = "source"))]
    #[test]
    fn bare_program_does_not_know_standard_modules() {
        let mut program = CompilerPlanningProgram::new(CompilerPlanningConfig::default());

        assert!(!program.function_catalog().has_module("math"));
        let error = program.plan_source_for_test("+> math").unwrap_err();

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
        let mut program = CompilerPlanningProgram::with_function_catalog(
            CompilerPlanningConfig::default(),
            catalog,
        );

        let output = program.plan_source_for_test("1.0 + 2.0").unwrap().unwrap();
        let output = mech_core::legacy_value_from_cell_compat(&output).unwrap();

        let LegacyValue::F64(output) = output else {
            panic!("custom math/add must return f64");
        };
        assert_eq!(*output.borrow(), 3.0);
    }
}
