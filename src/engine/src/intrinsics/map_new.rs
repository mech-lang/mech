//! A general empty map constructor; key/value kinds come from its destination.
use crate::*;
use std::sync::LazyLock;

static CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("map/new", 0, false).unwrap());

fn validate(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    if !inputs.is_empty()
        || !matches!(output.closed_schema_body()?, SchemaBody::Map { .. })
        || !output.requires_canonical_output_builder()?
    {
        return Err(super::invalid(
            "map/new requires a declared map result and no inputs",
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct NewMap {
    output: ValueCell,
}
impl MechFunctionImpl for NewMap {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        let footprint = self.output.prospective_aggregate_memory_footprint([])?;
        Ok(Some(vec![footprint].into_boxed_slice()))
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let footprint = self.output.prospective_aggregate_memory_footprint([])?;
        frame.with_admitted_canonical_output(&self.output, footprint, |_, construction| {
            let next = construction.try_rebuild_data_draft(
                &self.output,
                mech_core::snapshot::ValueDataDraft::Map(Box::new([])),
            )?;
            Ok(((), next))
        })?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some("map/new")
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&CONTRACT)
    }
    fn to_string(&self) -> String {
        "map/new".into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for NewMap {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        context.emit_varop(hash_str("map/new"), output, Vec::new());
        Ok(output)
    }
}
struct Factory;
impl MechFunctionFactory for Factory {
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue);
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        invocation.expect_nullary()?;
        validate(invocation.output_cell(), invocation.input_cells())?;
        Ok(Box::new(NewMap {
            output: invocation.output_cell().clone(),
        }))
    }
    fn implementation_memory_class() -> ImplementationMemoryClass {
        ImplementationMemoryClass::CanonicalFinalize
    }
    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&CONTRACT)
    }
}
mech_core::declare_native_runtime_factory! {
    cfg: feature = "map_ops", registration: register_new, installer: install_map_new,
    name: "map/new", factory_type: Factory,
    contract: RuntimeFunctionContract::canonical_custom("map/new",
        RuntimeOutputAliasPolicy::DisallowInputAlias, validate),
    operations: [OperationId::from_name("map/new")],
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_map_new",
    extra_cargo_features: ["map_ops"],
}

#[cfg(feature = "semantic-compiler")]
struct Source;
#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for Source {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if !invocation.is_empty() {
            return Err(super::invalid("map/new takes no inputs"));
        }
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name("map/new")),
            ExecutionTarget::DirectRuntime,
            vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
            &[],
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(super) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let operation = builder.insert_canonical_specializer_with_contracts(
        "map/new",
        mech_core::maintained_source_type_declaration("map/new")?,
        vec![CONTRACT.clone()],
        std::sync::Arc::new(Source),
    )?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: "map/new".into(),
        module: Some("map".into()),
        item: Some("new".into()),
        exposure: FunctionExposure::ModuleOnly,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_constructor_builds_an_empty_map_through_managed_runtime() {
        let output = ValueCell::from_schema_data(
            SchemaBody::Map {
                key: Box::new(SchemaBody::String),
                value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            mech_core::snapshot::ValueDataDraft::Map(Box::new([])),
        )
        .unwrap();
        let invocation = FunctionInvocation::nullary(output.clone());
        let implementation = Factory::new_invocation(invocation.clone()).unwrap();
        let specialized = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name("map/new", CONTRACT.clone()).unwrap(),
            RuntimeFunctionId::from_name("map/new"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap();
        specialized.instance().solve_result().unwrap();
        assert!(
            matches!(output.snapshot().unwrap().canonical_data_draft().unwrap(),
            mech_core::snapshot::ValueDataDraft::Map(entries) if entries.is_empty())
        );
    }

    #[test]
    fn map_constructor_rejects_non_map_outputs_and_arguments() {
        let scalar = ValueCell::from_exact(1u64).unwrap();
        assert!(validate(&scalar, &[]).is_err());
        assert!(validate(&scalar, &[scalar.clone()]).is_err());
    }
}
