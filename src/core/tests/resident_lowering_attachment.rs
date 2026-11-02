#![cfg(feature = "functions")]

use mech_core::{
    BoundCall, BoundResidentKernel, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentOperationKey, ResidentSnapshotOutput, ResidentValueMut,
    ResolvedOperationDescriptor, ResolvedValueDescriptor, SchemaBody, SchemaDraft, SchemaId,
    SchemaTable, SchemaTableBuilder,
};
use std::sync::Arc;

#[derive(Debug, Eq, PartialEq)]
struct Marker(u64);
fn dynamic(
    _: &BoundResidentKernel,
    _: &dyn ResidentKernelInputs,
    _: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    Err(ResidentKernelError::InvalidInput)
}
macro_rules! executor {
    ($name:ident($($input:ident),*), $output:ty) => {
        fn $name(_: &BoundResidentKernel, $($input: &[f64],)* _: $output)
            -> Result<bool, ResidentKernelError> {
            $(let _ = $input;)*
            Err(ResidentKernelError::InvalidInput)
        }
    };
}
executor!(f1(a), ResidentValueMut<'_>);
executor!(f2(a, b), ResidentValueMut<'_>);
executor!(f3(a, b, c), ResidentValueMut<'_>);
executor!(f4(a, b, c, d), ResidentValueMut<'_>);
executor!(o1(a), &mut [f64]);
executor!(o2(a, b), &mut [f64]);
executor!(o3(a, b, c), &mut [f64]);
executor!(o4(a, b, c, d), &mut [f64]);

fn metadata() -> (SchemaTable, SchemaId, BoundCall, ResidentSnapshotOutput) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Bool,
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(Box::new([])).unwrap();
    let descriptor = ResolvedValueDescriptor::from_schema(schema.clone(), shape.clone()).unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let key = build.table.entry(id).unwrap().key();
    let call = BoundCall::artifact_operation(
        ResolvedOperationDescriptor::from_name(
            "logic/not",
            mech_core::maintained_operation_contract("logic/not", 1, false).unwrap(),
        )
        .unwrap(),
        Box::new([descriptor.clone()]),
        Box::new([descriptor]),
        ResidentOperationKey::new(Box::new(["logic".into()]), "not".into()).unwrap(),
    )
    .unwrap();
    let output = ResidentSnapshotOutput {
        schema: id,
        schema_key: key,
        shape,
        exact_cardinality: None,
        maximum_cardinality: None,
    };
    (build.table, id, call, output)
}

#[test]
fn all_resident_constructors_begin_without_lowering_attachment() {
    let kernels = [
        BoundResidentKernel::new(dynamic, Box::new([])),
        BoundResidentKernel::new_f64_1(f1, Box::new([])),
        BoundResidentKernel::new_f64_2(f2, Box::new([])),
        BoundResidentKernel::new_f64_3(f3, Box::new([])),
        BoundResidentKernel::new_f64_4(f4, Box::new([])),
        BoundResidentKernel::new_f64_output_1(o1, Box::new([])),
        BoundResidentKernel::new_f64_output_2(o2, Box::new([])),
        BoundResidentKernel::new_f64_output_3(o3, Box::new([])),
        BoundResidentKernel::new_f64_output_4(o4, Box::new([])),
    ];
    for kernel in kernels {
        assert!(kernel.lowering_witness::<Marker>().is_none());
    }
}

#[test]
fn concrete_attachment_borrow_and_unchanged_clone_preserve_identity() {
    let kernel = BoundResidentKernel::new(dynamic, Box::new([7]))
        .with_lowering_witness(Arc::new(Marker(41)));
    let cloned = kernel.clone();
    assert_eq!(kernel.lowering_witness::<Marker>(), Some(&Marker(41)));
    assert!(std::ptr::eq(
        kernel.lowering_witness::<Marker>().unwrap(),
        cloned.lowering_witness::<Marker>().unwrap()
    ));
    assert!(kernel.lowering_witness::<u64>().is_none());
    assert_eq!(cloned.parameters(), [7]);
    let replaced = cloned.with_lowering_witness(Arc::new(17u64));
    assert!(replaced.lowering_witness::<Marker>().is_none());
    assert_eq!(replaced.lowering_witness::<u64>(), Some(&17));
    assert_eq!(kernel.lowering_witness::<Marker>(), Some(&Marker(41)));
}

#[test]
fn every_semantic_and_cost_builder_invalidates_even_unchanged_attachment() {
    let (schemas, _, call, output) = metadata();
    let base = BoundResidentKernel::new(dynamic, Box::new([]))
        .with_snapshot_output(output.clone())
        .with_snapshot_schemas(schemas.clone())
        .with_bound_call(call.clone())
        .with_retained_state(Arc::new(1u64))
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered)
        .with_lowering_witness(Arc::new(Marker(9)));
    let changed = [
        base.clone()
            .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered),
        base.clone()
            .with_work_accounting(ResidentKernelWorkAccounting::Unclassified),
        base.clone().with_snapshot_output(output),
        base.clone().with_snapshot_schemas(schemas),
        base.clone().with_bound_call(call),
        base.clone().with_retained_state(Arc::new(1u64)),
    ];
    for kernel in changed {
        assert!(kernel.lowering_witness::<Marker>().is_none());
    }
    assert_eq!(base.lowering_witness::<Marker>(), Some(&Marker(9)));
}
