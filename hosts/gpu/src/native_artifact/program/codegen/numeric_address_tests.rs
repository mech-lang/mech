//! Admission-proven vector geometry must actually remove numeric division.
use super::linear_address;
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Opcode, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use mech_core::{DimensionExpr as Dim, DimensionParameterId, IntegerWidth, SchemaBody};

#[test]
fn unit_dimension_addresses_emit_no_division_and_keep_general_geometry() {
    for width in [1, 2, 4, 8] {
        for (dimensions, vector) in [
            (
                [
                    Dim::Constant(1),
                    Dim::Parameter(DimensionParameterId::new(0)),
                ],
                true,
            ),
            (
                [
                    Dim::Parameter(DimensionParameterId::new(0)),
                    Dim::Constant(1),
                ],
                true,
            ),
            ([Dim::Constant(1), Dim::Constant(0)], true),
            ([Dim::Constant(0), Dim::Constant(1)], true),
            ([Dim::Constant(2), Dim::Constant(3)], false),
            (
                [
                    Dim::Parameter(DimensionParameterId::new(0)),
                    Dim::Parameter(DimensionParameterId::new(1)),
                ],
                false,
            ),
        ] {
            let schema = SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(match width {
                    1 => IntegerWidth::W8,
                    2 => IntegerWidth::W16,
                    4 => IntegerWidth::W32,
                    8 => IntegerWidth::W64,
                    _ => unreachable!(),
                })),
                dimensions: Box::new(dimensions),
            };
            let mut function = Function::new();
            function.signature.params = vec![AbiParam::new(types::I64); 4];
            function.signature.returns = vec![AbiParam::new(types::I64)];
            let mut context = FunctionBuilderContext::new();
            {
                let mut builder = FunctionBuilder::new(&mut function, &mut context);
                let entry = builder.create_block();
                builder.append_block_params_for_function_params(entry);
                builder.switch_to_block(entry);
                let args = builder.block_params(entry).to_vec();
                let (address, _) = linear_address(
                    &mut builder,
                    args[0],
                    args[1],
                    args[2],
                    width,
                    &schema,
                    |_| args[3],
                )
                .unwrap();
                builder.ins().return_(&[address]);
                builder.seal_all_blocks();
                builder.finalize();
            }
            let opcodes = function
                .layout
                .blocks()
                .flat_map(|block| function.layout.block_insts(block))
                .map(|inst| function.dfg.insts[inst].opcode())
                .collect::<Vec<_>>();
            assert_eq!(opcodes.contains(&Opcode::Udiv), !vector, "{schema:?}");
            assert_eq!(opcodes.contains(&Opcode::Urem), !vector, "{schema:?}");
        }
    }
}
