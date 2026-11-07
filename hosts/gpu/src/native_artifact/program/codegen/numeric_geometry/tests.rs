use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::{DimensionExpr as Dim, DimensionParameterId, IntegerWidth};
fn schema(dimensions: Vec<Dim>) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
        dimensions: dimensions.into_boxed_slice(),
    }
}
fn native(
    schema: &SchemaBody,
) -> (
    cranelift_jit::JITModule,
    unsafe extern "C" fn(u64, u64, u64, *mut u64),
) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature = module.make_signature();
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    let mut f = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut f);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let (rows, columns) = axes(&mut b, args[0], schema).unwrap_or((args[1], args[2]));
        b.ins().store(MemFlags::trusted(), rows, args[3], 0);
        b.ins().store(MemFlags::trusted(), columns, args[3], 8);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    let id = module
        .declare_function("shape", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: this retained native function has exactly three words, a writable
    // two-word pointer and no return value. Tests provide live aligned storage.
    let function = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    (module, function)
}
#[test]
fn admitted_unit_axes_keep_empty_and_large_native_geometry_exact() {
    let p = Dim::Parameter(DimensionParameterId::new(0));
    for row in [true, false] {
        let shape = schema(if row {
            vec![Dim::Constant(1), p.clone()]
        } else {
            vec![p.clone(), Dim::Constant(1)]
        });
        let (_owner, run) = native(&shape);
        for length in [0, 1, 2, 17, (1u64 << 53) + 7, i64::MAX as u64] {
            let expected = if row { [1, length] } else { [length, 1] };
            let mut out = [0; 2];
            // SAFETY: the owned function writes exactly the two provided cells.
            unsafe { run(length, expected[0], expected[1], out.as_mut_ptr()) };
            assert_eq!(out, expected);
        }
    }
}
#[test]
fn general_and_zero_axes_keep_their_actual_geometry() {
    let p = Dim::Parameter(DimensionParameterId::new(0));
    let q = Dim::Parameter(DimensionParameterId::new(1));
    for (shape, values) in [
        (
            schema(vec![p.clone(), q]),
            vec![(6, 2, 3), (0, 0, u64::MAX), (0, u64::MAX, 0), (7, 1, 7)],
        ),
        (
            schema(vec![Dim::Constant(0), p.clone()]),
            vec![(0, 0, u64::MAX), (0, 0, 1)],
        ),
        (
            schema(vec![Dim::Constant(2), p]),
            vec![(6, 2, 3), (0, 2, 0)],
        ),
    ] {
        let (_owner, run) = native(&shape);
        for (length, rows, columns) in values {
            let mut out = [0; 2];
            unsafe { run(length, rows, columns, out.as_mut_ptr()) };
            assert_eq!(out, [rows, columns]);
        }
    }
}
