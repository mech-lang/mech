use crate::intrinsics::*;

/// Runtime alias emitted for canonical tuple element access.
#[derive(Debug)]
pub struct TupleAccessElement {
    output: FunctionValueOutput,
}

impl MechFunctionImpl for TupleAccessElement {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.cell().clone()]
    }
}

impl MechFunctionFactory for TupleAccessElement {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    }

    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        Ok(Box::new(Self {
            output: invocation.expect_nullary()?.value(),
        }))
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "access", feature = "tuple"),
    registration: register_tuple_access_element,
    installer: install_tuple_access_element,
    name: "TupleAccessElement",
    factory_type: TupleAccessElement,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("TupleAccessElement"),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_tuple_access_element",
    extra_cargo_features: ["access"],
}

pub(super) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_tuple_access_element(builder)
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TupleAccessElement {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.cell().clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = self.output.compile_register(context)?;
        context.emit_nullop(hash_str("TupleAccessElement"), output);
        Ok(output)
    }
}
