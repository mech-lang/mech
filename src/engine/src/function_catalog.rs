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
