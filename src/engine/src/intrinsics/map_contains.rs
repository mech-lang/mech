//! Direct membership in an ordinary map, without constructing a key collection.
use crate::*;
use std::sync::LazyLock;
static CONTRACT: LazyLock<OperationContractDeclaration> = LazyLock::new(|| {
    mech_core::maintained_operation_contract("map/contains-key", 2, true).unwrap()
});
fn validate(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [map, candidate] = inputs else {
        return Err(super::invalid("map/contains-key requires map and key"));
    };
    let SchemaBody::Map { key, .. } = map.closed_schema_body()? else {
        return Err(super::invalid("map/contains-key requires a map"));
    };
    if output.closed_schema_body()? != SchemaBody::Bool || candidate.closed_schema_body()? != *key {
        return Err(super::invalid(
            "map/contains-key requires the map key kind and a Bool result",
        ));
    }
    Ok(())
}
#[derive(Debug)]
struct Contains {
    inputs: [ValueCell; 2],
    output: ManagedPort<bool>,
}
impl MechFunctionImpl for Contains {
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(self.output.cell()))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(self.output.cell())]))
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        let map = frame.snapshot_input_cell(&self.inputs[0], 0)?;
        let candidate = frame.snapshot_input_cell(&self.inputs[1], 1)?;
        let descriptor = frame.input_cell_descriptor(&self.inputs[0], 0)?;
        let SchemaBody::Map { key, .. } = descriptor.schema().body() else {
            unreachable!()
        };
        let ValueData::Map(map) = map.data() else {
            unreachable!()
        };
        let (mut lower, mut upper) = (0, map.entries().len());
        let mut found = false;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let order = mech_core::snapshot::compare_key_data(
                key,
                candidate.data(),
                map.entries()[middle].key().data(),
            )
            .map_err(|_| super::invalid("map/contains-key comparison failed"))?;
            match order {
                core::cmp::Ordering::Less => upper = middle,
                core::cmp::Ordering::Greater => lower = middle + 1,
                core::cmp::Ordering::Equal => {
                    found = true;
                    break;
                }
            }
        }
        frame.with_port_init_writer(&self.output, |output| output.write_next(found))?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some("map/contains-key")
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&CONTRACT)
    }
    fn to_string(&self) -> String {
        "map/contains-key".into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for Contains {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![
            self.inputs[0].clone(),
            self.inputs[1].clone(),
            self.output.cell().clone(),
        ]
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(self.output.cell(), context)?;
        let map = compile_value_cell_register(&self.inputs[0], context)?;
        let candidate = compile_value_cell_register(&self.inputs[1], context)?;
        context.emit_binop(hash_str("map/contains-key"), output, map, candidate);
        Ok(output)
    }
}
struct Factory;
impl MechFunctionFactory for Factory {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        validate(invocation.output_cell(), invocation.input_cells())?;
        let inputs = [
            invocation.input_cells()[0].clone(),
            invocation.input_cells()[1].clone(),
        ];
        let (output, _, _) = invocation.expect_binary()?;
        Ok(Box::new(Contains {
            inputs,
            output: output.try_managed()?,
        }))
    }
    fn implementation_memory_class() -> ImplementationMemoryClass {
        ImplementationMemoryClass::NoAdditionalScratch
    }
    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&CONTRACT)
    }
}
mech_core::declare_native_runtime_factory! {
    cfg: feature = "map_ops", registration: register_contains, installer: install_map_contains_key,
    name: "map/contains-key", factory_type: Factory,
    contract: RuntimeFunctionContract::canonical_custom("map/contains-key", RuntimeOutputAliasPolicy::DisallowInputAlias, validate),
    operations: [OperationId::from_name("map/contains-key")],
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_map_contains_key", extra_cargo_features: ["map_ops"],
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
        let [map, candidate] = invocation.inputs() else {
            return Err(super::invalid("map/contains-key requires two inputs"));
        };
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name("map/contains-key")),
            ExecutionTarget::DirectRuntime,
            vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
            &[map, candidate],
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(super) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let operation = builder.insert_canonical_specializer_with_contracts(
        "map/contains-key",
        mech_core::maintained_source_type_declaration("map/contains-key")?,
        vec![CONTRACT.clone()],
        std::sync::Arc::new(Source),
    )?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: "map/contains-key".into(),
        module: Some("map".into()),
        item: Some("contains-key".into()),
        exposure: FunctionExposure::ModuleOnly,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn map_membership_managed_runtime_reads_without_changing_the_map() {
        let map = ValueCell::from_schema_data(
            SchemaBody::Map {
                key: Box::new(SchemaBody::String),
                value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Map(
                vec![mech_core::snapshot::MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String("é".to_owned()),
                        ValueDataDraft::U64(1),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
        .unwrap();
        let previous = map.snapshot().unwrap();
        let key = ValueCell::from_exact("é".to_owned()).unwrap();
        let output = ValueCell::from_exact(false).unwrap();
        let invocation = FunctionInvocation::binary(output.clone(), map.clone(), key.clone());
        let implementation = Factory::new_invocation(invocation.clone()).unwrap();
        let specialized = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name("map/contains-key", CONTRACT.clone()).unwrap(),
            RuntimeFunctionId::from_name("map/contains-key"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::NoAdditionalScratch,
        )
        .unwrap();
        specialized.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::Bool(true)
        ));
        key.replace(
            &ValueCell::from_exact("missing".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        specialized.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::Bool(false)
        ));
        assert!(map.snapshot().unwrap().shares_frozen_storage(&previous));
    }
}
