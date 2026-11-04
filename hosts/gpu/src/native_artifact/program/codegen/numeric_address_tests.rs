//! Admission-proven vector geometry must actually remove numeric division.
use super::linear_address;
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Opcode, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use mech_core::{DimensionExpr as Dim, DimensionParameterId, IntegerWidth, SchemaBody};

fn address_function(schema: &SchemaBody, width: u8) -> Function {
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
            schema,
            |_| args[3],
        )
        .unwrap();
        builder.ins().return_(&[address]);
        builder.seal_all_blocks();
        builder.finalize();
    }
    function
}

fn general_schema(width: u8) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(SchemaBody::UnsignedInteger(match width {
            1 => IntegerWidth::W8,
            2 => IntegerWidth::W16,
            4 => IntegerWidth::W32,
            8 => IntegerWidth::W64,
            _ => unreachable!(),
        })),
        dimensions: Box::new([
            Dim::Parameter(DimensionParameterId::new(0)),
            Dim::Parameter(DimensionParameterId::new(1)),
        ]),
    }
}

type Address = unsafe extern "C" fn(u64, u64, u64, u64) -> u64;

fn native_address(width: u8) -> (cranelift_jit::JITModule, Address, String) {
    use cranelift_module::{Linkage, Module, default_libcall_names};
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func = address_function(&general_schema(width), width);
    context.func.signature.call_conv = module.make_signature().call_conv;
    context.set_disasm(true);
    let id = module
        .declare_function("numeric_address", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let assembly = context.compiled_code().unwrap().vcode.clone().unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the owned generated function has exactly four word arguments
    // and one word result. It computes an address without dereferencing it.
    let address: Address = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    (module, address, assembly)
}

#[test]
fn general_numeric_addresses_emit_one_machine_division() {
    for width in [1, 2, 4, 8] {
        let (_module, _, assembly) = native_address(width);
        if cfg!(target_arch = "aarch64") {
            let divisions = assembly
                .lines()
                .filter(|line| line.contains("udiv "))
                .count();
            assert_eq!(divisions, 1, "width={width}\n{assembly}");
        }
        let function = address_function(&general_schema(width), width);
        let divisions = function
            .layout
            .blocks()
            .flat_map(|block| function.layout.block_insts(block))
            .filter(|&inst| {
                matches!(
                    function.dfg.insts[inst].opcode(),
                    Opcode::Udiv | Opcode::Urem
                )
            })
            .count();
        assert_eq!(divisions, 1, "width={width}\n{}", function.display());
    }
}

#[test]
fn native_numeric_addresses_follow_independent_column_order() {
    for width in [1u8, 2, 4, 8] {
        let (_module, address, _) = native_address(width);
        for (rows, columns) in [(1u64, 1u64), (1, 6), (6, 1), (2, 3), (3, 2), (7, 13)] {
            let mut ordinal = 0;
            for column in 0..columns {
                for row in 0..rows {
                    let expected = 4096 + (row * columns + column) * u64::from(width);
                    // SAFETY: this retained JIT function only computes words;
                    // all ordinals are valid and all row counts are nonzero.
                    assert_eq!(unsafe { address(ordinal, rows, columns, 4096) }, expected);
                    ordinal += 1;
                }
            }
        }
        // Exercise exact integer division above floating-point integer precision.
        // These are arithmetic-only addresses, never allocated or dereferenced.
        let rows = u64::MAX / u64::from(width);
        let ordinal = rows - 1;
        // SAFETY: the same live function computes words with a nonzero row count.
        let observed = unsafe { address(ordinal, rows, 1, 0) };
        assert_eq!(observed, ordinal * u64::from(width));
    }
}

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
            let function = address_function(&schema, width);
            let opcodes = function
                .layout
                .blocks()
                .flat_map(|block| function.layout.block_insts(block))
                .map(|inst| function.dfg.insts[inst].opcode())
                .collect::<Vec<_>>();
            assert_eq!(opcodes.contains(&Opcode::Udiv), !vector, "{schema:?}");
            assert!(!opcodes.contains(&Opcode::Urem), "{schema:?}");
        }
    }
}
