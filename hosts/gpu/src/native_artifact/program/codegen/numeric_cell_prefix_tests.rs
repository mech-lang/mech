//! Execute the actual prefix emitter, with no source interpreter or test kernel.
use super::*;
use cranelift_codegen::ir::{AbiParam, Signature};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

fn recipe(reciprocal: bool) -> Prefix {
    let inputs = BTreeMap::from([
        (1, Kind::U64),
        (2, Kind::U64),
        (3, Kind::U64),
        (4, Kind::F64),
        (5, Kind::U64),
    ]);
    let kinds = BTreeMap::from([
        (10, Kind::U64),
        (11, Kind::U64),
        (12, Kind::U64),
        (20, Kind::F64),
        (21, Kind::F64),
        (30, Kind::F64),
        (31, Kind::F64),
    ]);
    Prefix {
        first: 0,
        next: 8,
        sites: (0..8).collect(),
        read_sites: vec![3, 4],
        exports: vec![31],
        inputs,
        kinds,
        steps: vec![
            (10, Step::Primitive(Op::U64Mul, vec![1, 5])),
            (11, Step::Primitive(Op::U64Add, vec![10, 3])),
            (12, Step::Primitive(Op::U64Add, vec![2, 3])),
            (
                20,
                Step::Read {
                    array: 0,
                    row: 10,
                    column: 2,
                },
            ),
            (
                21,
                Step::Read {
                    array: 0,
                    row: 11,
                    column: 12,
                },
            ),
            (
                30,
                Step::Primitive(
                    if reciprocal { Op::F64Mul } else { Op::F64Add },
                    vec![20, 21],
                ),
            ),
            (
                31,
                Step::Primitive(
                    Op::F64Div,
                    if reciprocal { vec![4, 30] } else { vec![30, 4] },
                ),
            ),
        ],
    }
}
type Kernel = unsafe extern "C" fn(*const u64, *mut u64) -> u64;
fn compile(prefix: Prefix) -> (JITModule, Kernel) {
    let mut module =
        JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
    let mut ctx = module.make_context();
    ctx.func.signature = Signature {
        params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
        returns: vec![AbiParam::new(types::I64)],
        call_conv: module.isa().default_call_conv(),
    };
    let id = module
        .declare_function("closed_cell_prefix", Linkage::Export, &ctx.func.signature)
        .unwrap();
    let mut bc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut bc);
        let entry = b.create_block();
        let slow = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        let output = b.block_params(entry)[1];
        let mut load = |offset| b.ins().load(types::I64, MemFlags::new(), frame, offset);
        let pointer = load(0);
        let rows = load(8);
        let columns = load(16);
        let inputs = BTreeMap::from([
            (1, load(24)),
            (2, load(32)),
            (3, load(40)),
            (4, load(48)),
            (5, load(56)),
        ]);
        let length = prefix
            .steps
            .iter()
            .any(|(_, step)| matches!(step, Step::LinearRead { .. }))
            .then(|| b.ins().imul(rows, columns));
        let views = BTreeMap::from([(
            0,
            View {
                pointer,
                rows,
                columns,
                length,
            },
        )]);
        let values = evaluate(
            &mut b,
            OptimizationOptions::default(),
            &prefix,
            &inputs,
            &views,
            slow,
        );
        assert_eq!(values.len(), 1);
        b.ins().store(MemFlags::new(), values[0].1, output, 0);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[zero]);
        b.switch_to_block(slow);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().return_(&[one]);
        b.seal_all_blocks();
        b.finalize();
    }
    let expected_branches = if prefix
        .steps
        .iter()
        .any(|(_, s)| matches!(s, Step::LinearRead { .. }))
    {
        3
    } else {
        2
    };
    assert_eq!(ctx.func.layout.blocks().flat_map(|block| ctx.func.layout.block_insts(block)).filter(|inst| ctx.func.dfg.insts[*inst].opcode()==cranelift_codegen::ir::Opcode::Brif).count(),expected_branches,"complete read admission/result guards plus general linear address conversion");
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: owned emitted function has the exact two-pointer signature above.
    let function =
        unsafe { core::mem::transmute::<*const u8, Kernel>(module.get_finalized_function(id)) };
    (module, function)
}
#[test]
fn cell_prefix_direct_complete_admission_dynamic_geometry_bits_and_untouched_failure() {
    let (_module, kernel) = compile(recipe(false));
    let data = (0..42).map(|n| n as f64 - 8.25).collect::<Vec<_>>();
    for rows in 0..=6u64 {
        for columns in 0..=7u64 {
            for row in [0, 1, 2, rows, u64::MAX] {
                for column in [0, 1, 2, columns, u64::MAX] {
                    for offset in [0, 1, 2, u64::MAX] {
                        let valid = row >= 1
                            && row <= rows
                            && column >= 1
                            && column <= columns
                            && row.checked_add(offset).is_some_and(|r| r <= rows)
                            && column.checked_add(offset).is_some_and(|c| c <= columns);
                        let frame = [
                            data.as_ptr() as u64,
                            rows,
                            columns,
                            row,
                            column,
                            offset,
                            9.0f64.to_bits(),
                            1,
                        ];
                        let mut out = [0xface_f00d_dead_beef; 3];
                        // SAFETY: complete rows*columns view is within data; invalid bounds must not read.
                        let status = unsafe { kernel(frame.as_ptr(), out.as_mut_ptr().add(1)) };
                        assert_eq!(status, u64::from(!valid));
                        assert_eq!(out[0], 0xface_f00d_dead_beef);
                        assert_eq!(out[2], out[0]);
                        if valid {
                            let a = data[((row - 1) * columns + column - 1) as usize];
                            let c =
                                data[((row + offset - 1) * columns + column + offset - 1) as usize];
                            assert_eq!(out[1], ((a + c) / 9.0).to_bits());
                        } else {
                            assert_eq!(out[1], out[0]);
                        }
                    }
                }
            }
        }
    }
    // First source address is in bounds but the later address/checked operation
    // fails. A null bank proves no speculative read occurs before full admission.
    for (row, column, offset, scale) in [
        (1, 1, 8, 1),
        (u64::MAX, 1, 1, 1),
        (u64::MAX, 1, 0, 2),
        (1, u64::MAX, 1, 1),
    ] {
        let frame = [0, 2, 3, row, column, offset, 9.0f64.to_bits(), scale];
        let mut out = 123;
        // SAFETY: guard must reject before dereferencing the inactive null bank.
        assert_eq!(unsafe { kernel(frame.as_ptr(), &mut out) }, 1);
        assert_eq!(out, 123);
    }
}
#[test]
fn cell_prefix_direct_nonfinite_results_decline_but_intermediate_infinity_can_complete() {
    let (_normal, kernel) = compile(recipe(false));
    let (_inverse, inverse) = compile(recipe(true));
    for bits in [
        0u64,
        (-0.0f64).to_bits(),
        1,
        1.25f64.to_bits(),
        f64::MAX.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_1234_5678_9abc,
        0x7ff0_1234_5678_9abc,
        0xfff8_4321_1234_abcd,
    ] {
        let data = [bits, bits];
        let frame = [data.as_ptr() as u64, 1, 2, 1, 1, 0, 1.0f64.to_bits(), 1];
        let mut out = 0xabcd;
        let expected = f64::from_bits(bits) + f64::from_bits(bits);
        // SAFETY: both reads refer to the initialized first cell of this view.
        let status = unsafe { kernel(frame.as_ptr(), &mut out) };
        assert_eq!(status, u64::from(!expected.is_finite()));
        if expected.is_finite() {
            assert_eq!(out, expected.to_bits())
        } else {
            assert_eq!(out, 0xabcd)
        }
        let expected = 1.0 / (f64::from_bits(bits) * f64::from_bits(bits));
        out = 0xabcd;
        // SAFETY: same owned kernel ABI and complete initialized view.
        let status = unsafe { inverse(frame.as_ptr(), &mut out) };
        assert_eq!(status, u64::from(!expected.is_finite()));
        if expected.is_finite() {
            assert_eq!(out, expected.to_bits())
        } else {
            assert_eq!(out, 0xabcd)
        }
    }
}

#[test]
fn cell_prefix_direct_single_linear_read_checks_full_index_before_load_and_preserves_bits() {
    let mut prefix = recipe(false);
    prefix.read_sites = vec![3];
    prefix.kinds.remove(&21);
    prefix.steps[3] = (
        20,
        Step::LinearRead {
            array: 0,
            index: 11,
            schema: mech_core::SchemaBody::Matrix {
                element: Box::new(mech_core::SchemaBody::FloatingPoint(
                    mech_core::FloatWidth::W64,
                )),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(2),
                    mech_core::DimensionExpr::Constant(3),
                ]
                .into(),
            },
        },
    );
    prefix.steps.remove(4);
    prefix.steps[4] = (30, Step::Primitive(Op::F64Mul, vec![20, 4]));
    prefix.steps[5] = (31, Step::Primitive(Op::F64Add, vec![30, 20]));
    let (_module, kernel) = compile(prefix);
    for bits in [
        0,
        (-0.0f64).to_bits(),
        1,
        1.25f64.to_bits(),
        f64::MAX.to_bits(),
        f64::INFINITY.to_bits(),
        0x7ff8_1234_5678_9abc,
        0x7ff0_1234_5678_9abc,
    ] {
        let data = if bits == 0 {
            [0.0f64, 1.25, -4.0, 8.5, 0.125, -0.0].map(f64::to_bits)
        } else {
            [bits; 6]
        };
        for (rows, columns) in [(0, 3), (2, 3), (3, 2), (1, 6)] {
            for base in [0, 1, 2, 5, u64::MAX] {
                for offset in [0, 1, 6, u64::MAX] {
                    for scale in [0, 1, u64::MAX] {
                        let index = base.checked_mul(scale).and_then(|v| v.checked_add(offset));
                        let admitted = index.is_some_and(|v| v >= 1 && v <= rows * columns)
                            && 1u64.checked_add(offset).is_some();
                        let value = if admitted {
                            let ordinal = index.unwrap() - 1;
                            let physical = (ordinal % rows) * columns + ordinal / rows;
                            f64::from_bits(data[physical as usize])
                        } else {
                            0.0
                        };
                        let expected = value * 9.0 + value;
                        let frame = [
                            if admitted { data.as_ptr() as u64 } else { 0 },
                            rows,
                            columns,
                            base,
                            1,
                            offset,
                            9.0f64.to_bits(),
                            scale,
                        ];
                        let mut out = [0xface_f00d_dead_beef; 3];
                        // SAFETY: complete successful views contain six cells. Invalid
                        // checked indices use a null bank and must refuse before loading.
                        let status = unsafe { kernel(frame.as_ptr(), out.as_mut_ptr().add(1)) };
                        let success = admitted && expected.is_finite();
                        assert_eq!(status, u64::from(!success));
                        assert_eq!(out[0], 0xface_f00d_dead_beef);
                        assert_eq!(out[2], out[0]);
                        assert_eq!(out[1], if success { expected.to_bits() } else { out[0] });
                    }
                }
            }
        }
    }
}
