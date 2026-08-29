use crate::intrinsics::*;

// Convert --------------------------------------------------------------------

#[cfg(feature = "enum")]
#[derive(Debug)]
struct ConvertSEnum {
    out: FunctionValueOutput,
}
#[cfg(feature = "enum")]
impl MechFunctionFactory for ConvertSEnum {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::Enum);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let out = invocation.expect_nullary()?.value();
        Ok(Box::new(Self { out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}
#[cfg(feature = "enum")]
impl MechFunctionImpl for ConvertSEnum {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(self.out.state_port())
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![self.out.state_port()]))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(all(feature = "semantic-compiler", feature = "enum"))]
impl MechFunctionCompiler for ConvertSEnum {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        self.out.compile_register(ctx)
    }
}
#[derive(Debug)]
struct ConvertSEmpty {
    out: FunctionValueOutput,
}

impl MechFunctionFactory for ConvertSEmpty {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::MutableValueCell);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let out = invocation.expect_nullary()?.value();
        Ok(Box::new(Self { out }))
    }

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        Self::new_invocation(args.into())
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "convert",
    registration: register_convert_empty,
    installer: install_convert_empty,
    name: "ConvertSEmpty<empty>",
    factory_type: ConvertSEmpty,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_convert_empty",
    extra_cargo_features: ["convert"],
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "convert", feature = "enum"),
    registration: register_convert_enum,
    installer: install_convert_enum,
    name: "ConvertSEnum<enum>",
    factory_type: ConvertSEnum,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_convert_enum",
    extra_cargo_features: ["convert"],
}

impl MechFunctionImpl for ConvertSEmpty {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(self.out.state_port())
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![self.out.state_port()]))
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ConvertSEmpty {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        self.out.compile_register(ctx)
    }
}
pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "enum")]
    register_convert_enum(builder)?;
    register_convert_empty(builder)?;
    Ok(())
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(feature = "convert")]
    pub use super::install_convert_empty;
    #[cfg(all(feature = "convert", feature = "enum"))]
    pub use super::install_convert_enum;
}
