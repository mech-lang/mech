//! Word operations qualify without linking the broad math/source distribution.
#![cfg(feature = "integer_bits")]
use mech_core::{
    ExecutionTarget, FunctionCatalogBuilder, FunctionInvocation, IntegerWidth, OperationId,
    ResolvedOperationDescriptor, RuntimeFunctionId, SchemaBody, SpecializedFunction, ValueCell,
    ValueDataDraft,
};

fn count(value: u64) -> ValueCell {
    ValueCell::from_schema_data(
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        ValueDataDraft::U64(value),
    )
    .unwrap()
}

fn bind(
    operation: &str,
    factory: &str,
    output: &ValueCell,
    inputs: &[ValueCell],
) -> SpecializedFunction {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    let operation_id = OperationId::from_name(operation);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name(factory))
        .unwrap();
    let invocation = match inputs {
        [input] => FunctionInvocation::unary(output.clone(), input.clone()),
        [first, second] => {
            FunctionInvocation::binary(output.clone(), first.clone(), second.clone())
        }
        _ => unreachable!(),
    };
    let parts = entry
        .bind_resolved_invocation(operation_id, ExecutionTarget::DirectRuntime, invocation)
        .unwrap();
    SpecializedFunction::syntax_directed(
        parts,
        ResolvedOperationDescriptor::from_name(
            operation,
            entry.operation_contract(operation_id).unwrap().clone(),
        )
        .unwrap(),
        entry.id,
        ExecutionTarget::DirectRuntime,
        entry.implementation_memory_class(),
    )
    .unwrap()
}

#[test]
fn maintained_managed_word_factories_refresh_exact_u64_inputs() {
    let cases: [(&str, bool, fn(u64, u64) -> u64); 11] = [
        ("and", false, |a, b| a & b),
        ("or", false, |a, b| a | b),
        ("xor", false, |a, b| a ^ b),
        ("not", true, |a, _| !a),
        ("shift-left-wrap", false, |a, b| {
            a.wrapping_shl((b % 64) as u32)
        }),
        ("shift-right-wrap", false, |a, b| {
            a.wrapping_shr((b % 64) as u32)
        }),
        ("leading-zeros", true, |a, _| u64::from(a.leading_zeros())),
        ("trailing-zeros", true, |a, _| u64::from(a.trailing_zeros())),
        ("wrapping-add", false, u64::wrapping_add),
        ("wrapping-sub", false, u64::wrapping_sub),
        ("wrapping-mul", false, u64::wrapping_mul),
    ];
    for (item, unary, expected) in cases {
        let name = format!("bits/{item}");
        // The skinny profile has canonical storage. When u64 is selected,
        // also exercise every fixed-width and mixed call-frame combination.
        #[cfg(feature = "u64")]
        let backings = 0..8;
        #[cfg(not(feature = "u64"))]
        let backings = 0..1;
        for backing in backings {
            let word = |mask: u8| {
                #[cfg(feature = "u64")]
                if backing & mask != 0 {
                    return ValueCell::from_exact(0u64).unwrap();
                }
                let _ = mask;
                ValueCell::from_snapshot(count(0).snapshot().unwrap()).unwrap()
            };
            let lhs = word(1);
            let rhs = word(2);
            let output = word(4);
            let inputs = if unary {
                vec![lhs.clone()]
            } else {
                vec![lhs.clone(), rhs.clone()]
            };
            let function = bind(&name, &name, &output, &inputs);
            for left in [0, 1, u64::MAX, 1u64 << 63] {
                for right in [0, 1, 63, 64, 65, u64::MAX] {
                    lhs.replace(&count(left).snapshot().unwrap()).unwrap();
                    rhs.replace(&count(right).snapshot().unwrap()).unwrap();
                    function.instance().solve_result().unwrap_or_else(|error| {
                        panic!("{name} {left} {right}, backing {backing}: {error:?}")
                    });
                    assert_eq!(
                        output.snapshot().unwrap().canonical_data_draft().unwrap(),
                        ValueDataDraft::U64(expected(left, right)),
                        "{name} {left} {right}, backing {backing}"
                    );
                }
            }
        }
    }
}
