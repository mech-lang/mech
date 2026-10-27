//! Functional insertion/replacement of one ordinary map entry.
//! Traversal, accumulation and arithmetic remain source-level operations.
use crate::*;
use mech_core::snapshot::{MapEntryDraft, ValueDataDraft};
use std::sync::LazyLock;

static CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| mech_core::maintained_operation_contract("map/insert", 3, false).unwrap());

fn validate(output: &ValueCell, inputs: &[ValueCell]) -> MResult<()> {
    let [map, key_cell, value_cell] = inputs else {
        return Err(super::invalid("map/insert requires map, key and value"));
    };
    let SchemaBody::Map { key, value, .. } = map.closed_schema_body()? else {
        return Err(super::invalid("map/insert requires a map"));
    };
    let SchemaBody::Map {
        key: out_key,
        value: out_value,
        cardinality,
    } = output.closed_schema_body()?
    else {
        return Err(super::invalid("map/insert requires a map output"));
    };
    if key != out_key
        || value != out_value
        || key_cell.closed_schema_body()? != *key
        || value_cell.closed_schema_body()? != *value
        || cardinality != (CardinalitySpec::Dynamic { upper_bound: None })
        || !output.requires_canonical_output_builder()?
    {
        return Err(super::invalid(
            "map/insert key/value kinds must match its declared map kinds",
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct Insert {
    output: ValueCell,
    inputs: [ValueCell; 3],
}
impl Insert {
    fn footprint(&self) -> MResult<CurrentMemoryFootprint> {
        // Input snapshots, temporary comparison keys/drafts and the final map
        // coexist. Fund all three inputs before searching or copying entries.
        let mut footprint = self
            .output
            .prospective_aggregate_memory_footprint(self.inputs.iter().map(|input| (input, 6)))?;
        footprint.logical_elements = self.inputs[0]
            .current_map_entry_count()?
            .checked_add(1)
            .ok_or_else(|| super::invalid("map cardinality overflow"))?;
        Ok(footprint)
    }
}
impl MechFunctionImpl for Insert {
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(Some(vec![self.footprint()?].into_boxed_slice()))
    }
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        frame.with_admitted_canonical_output(
            &self.output,
            self.footprint()?,
            |frame, construction| {
                let map = frame.snapshot_input_cell_with_construction(
                    &self.inputs[0],
                    0,
                    construction,
                )?;
                let selected = frame.snapshot_input_cell_with_construction(
                    &self.inputs[1],
                    1,
                    construction,
                )?;
                let replacement = frame.snapshot_input_cell_with_construction(
                    &self.inputs[2],
                    2,
                    construction,
                )?;
                let SchemaBody::Map { key, .. } = self.inputs[0].closed_schema_body()? else {
                    unreachable!()
                };
                let ValueDataDraft::Map(entries) = map
                    .canonical_data_draft()
                    .map_err(|_| super::invalid("map entry snapshot reconstruction failed"))?
                else {
                    unreachable!()
                };
                let selected_cell = ValueCell::from_schema_data(
                    (*key).clone(),
                    selected
                        .canonical_data_draft()
                        .map_err(|_| super::invalid("map entry snapshot reconstruction failed"))?,
                )?;
                let mut selected_ordinal = None;
                for (index, entry) in entries.iter().enumerate() {
                    let candidate =
                        ValueCell::from_schema_data((*key).clone(), entry.items[0].clone())?;
                    if candidate.key_eq(&selected_cell)? {
                        selected_ordinal = Some(index);
                        break;
                    }
                }
                let count = entries
                    .len()
                    .checked_add(usize::from(selected_ordinal.is_none()))
                    .ok_or_else(|| super::invalid("map cardinality overflow"))?;
                let mut entries = entries.into_vec().into_iter();
                let mut replacement = Some(
                    replacement
                        .canonical_data_draft()
                        .map_err(|_| super::invalid("map entry snapshot reconstruction failed"))?,
                );
                let mut selected = Some(
                    selected
                        .canonical_data_draft()
                        .map_err(|_| super::invalid("map entry snapshot reconstruction failed"))?,
                );
                let result = construction.try_boxed_slice_with(count, |construction, index| {
                    if let Some(mut entry) = entries.next() {
                        if selected_ordinal == Some(index) {
                            entry.items[1] = replacement.take().expect("one replacement");
                        }
                        Ok(entry)
                    } else {
                        Ok(MapEntryDraft {
                            items: construction.try_boxed_slice_with(2, |_, field| {
                                Ok(if field == 0 {
                                    selected.take().expect("one new key")
                                } else {
                                    replacement.take().expect("one new value")
                                })
                            })?,
                        })
                    }
                })?;
                Ok((
                    (),
                    construction
                        .try_rebuild_data_draft(&self.output, ValueDataDraft::Map(result))?,
                ))
            },
        )?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn semantic_operation_name(&self) -> Option<&str> {
        Some("map/insert")
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&CONTRACT)
    }
    fn to_string(&self) -> String {
        "map/insert".into()
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for Insert {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        std::iter::once(self.output.clone())
            .chain(self.inputs.iter().cloned())
            .collect()
    }
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(&self.output, context)?;
        let inputs = self
            .inputs
            .iter()
            .map(|input| compile_value_cell_register(input, context))
            .collect::<MResult<Vec<_>>>()?;
        context.emit_varop(hash_str("map/insert"), output, inputs);
        Ok(output)
    }
}
struct Factory;
impl MechFunctionFactory for Factory {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
        FunctionValueRepresentation::AnyValue,
    );
    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        validate(invocation.output_cell(), invocation.input_cells())?;
        let inputs: [ValueCell; 3] = invocation
            .input_cells()
            .to_vec()
            .try_into()
            .map_err(|_| super::invalid("map/insert requires three inputs"))?;
        Ok(Box::new(Insert {
            output: invocation.output_cell().clone(),
            inputs,
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
    cfg: feature = "map_ops", registration: register_insert, installer: install_map_insert,
    name: "map/insert", factory_type: Factory,
    contract: RuntimeFunctionContract::canonical_custom("map/insert",
        RuntimeOutputAliasPolicy::DisallowInputAlias, validate),
    operations: [OperationId::from_name("map/insert")],
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_map_insert",
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
        let [map, key, value] = invocation.inputs() else {
            return Err(super::invalid("map/insert requires three inputs"));
        };
        context.bind_resolved_runtime(
            RuntimeBindingSelector::Operation(OperationId::from_name("map/insert")),
            ExecutionTarget::DirectRuntime,
            vec![Vec::<u64>::new().into_boxed_slice()].into_boxed_slice(),
            &[map, key, value],
        )
    }
}
#[cfg(feature = "semantic-compiler")]
pub(super) fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let operation = builder.insert_canonical_specializer_with_contracts(
        "map/insert",
        mech_core::maintained_source_type_declaration("map/insert")?,
        vec![CONTRACT.clone()],
        std::sync::Arc::new(Source),
    )?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: "map/insert".into(),
        module: Some("map".into()),
        item: Some("insert".into()),
        exposure: FunctionExposure::ModuleOnly,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_map_insert_managed_runtime_preserves_its_input() {
        let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W64);
        let map_schema = |cardinality| SchemaBody::Map {
            key: Box::new(scalar.clone()),
            value: Box::new(scalar.clone()),
            cardinality,
        };
        let map = ValueCell::from_schema_data(
            map_schema(CardinalitySpec::Exact(DimensionExpr::Constant(1))),
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![ValueDataDraft::U64(1), ValueDataDraft::U64(4)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )
        .unwrap();
        let previous = map.snapshot().unwrap();
        let output = ValueCell::from_schema_data(
            map_schema(CardinalitySpec::Dynamic { upper_bound: None }),
            ValueDataDraft::Map(Box::new([])),
        )
        .unwrap();
        let key = ValueCell::from_exact(2u64).unwrap();
        let value = ValueCell::from_exact(8u64).unwrap();
        let invocation =
            FunctionInvocation::ternary(output.clone(), map.clone(), key.clone(), value.clone());
        let implementation = Factory::new_invocation(invocation.clone()).unwrap();
        let specialized = SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name("map/insert", CONTRACT.clone()).unwrap(),
            RuntimeFunctionId::from_name("map/insert"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap();
        specialized.instance().solve_result().unwrap();
        let ValueDataDraft::Map(entries) =
            output.snapshot().unwrap().canonical_data_draft().unwrap()
        else {
            unreachable!()
        };
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0].items[1], ValueDataDraft::U64(4)));
        assert!(matches!(entries[1].items[1], ValueDataDraft::U64(8)));
        assert!(map.snapshot().unwrap().shares_frozen_storage(&previous));
        key.replace(&ValueCell::from_exact(1u64).unwrap().snapshot().unwrap())
            .unwrap();
        specialized.instance().solve_result().unwrap();
        let ValueDataDraft::Map(entries) =
            output.snapshot().unwrap().canonical_data_draft().unwrap()
        else {
            unreachable!()
        };
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].items[1], ValueDataDraft::U64(8)));
        assert!(map.snapshot().unwrap().shares_frozen_storage(&previous));
    }
}
