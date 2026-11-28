//! Execute the production old/new publication emitters with runtime bounds.
use super::*;
use cranelift_codegen::ir::{AbiParam, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
#[derive(Clone, Copy, Debug)]
enum Spec {
    Linear(Comparison),
    Rows(Comparison),
    Columns(Comparison),
    Rectangle(Comparison, Comparison),
}
fn selection(
    b: &mut FunctionBuilder<'_>,
    spec: Spec,
    rows: u64,
    columns: u64,
    a: Value,
    c: Value,
) -> Selection {
    let (coordinates, count, rectangular) = match spec {
        Spec::Linear(op) => {
            let range = AxisSelection::new(b, op, a, rows * columns);
            let count = range.selected;
            (
                Coordinates::Single {
                    axis: Axis::Column,
                    range,
                },
                count,
                false,
            )
        }
        Spec::Rows(op) => {
            let range = AxisSelection::new(b, op, a, rows);
            let count = b.ins().imul_imm(range.selected, columns as i64);
            (
                Coordinates::Single {
                    axis: Axis::Row,
                    range,
                },
                count,
                true,
            )
        }
        Spec::Columns(op) => {
            let range = AxisSelection::new(b, op, a, columns);
            let count = b.ins().imul_imm(range.selected, rows as i64);
            (
                Coordinates::Single {
                    axis: Axis::Column,
                    range,
                },
                count,
                true,
            )
        }
        Spec::Rectangle(left, right) => {
            let row = AxisSelection::new(b, left, a, rows);
            let column = AxisSelection::new(b, right, c, columns);
            let count = b.ins().imul(row.selected, column.selected);
            (Coordinates::Rectangle { row, column }, count, true)
        }
    };
    Selection {
        coordinates,
        rectangular,
        rows,
        columns,
        count,
        edge: None,
    }
}
fn function(spec: Spec, rows: u64, columns: u64, width: u8, spans: bool) -> Function {
    let mut f = Function::new();
    f.signature.params = vec![AbiParam::new(types::I64); 4];
    let mut context = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut f, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let selected = selection(&mut b, spec, rows, columns, args[2], args[3]);
        if spans {
            selected
                .publish_spans(&mut b, args[0], args[1], width)
                .unwrap();
        } else {
            crate::native_artifact::program::codegen::numeric_phase_maps::publish_selected(
                &mut b, &selected, args[0], args[1], width,
            )
            .unwrap();
        }
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    f
}
type Copy = unsafe extern "C" fn(*const u8, *mut u8, u64, u64);
fn compile(f: Function) -> (cranelift_jit::JITModule, Copy, String) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func = f;
    context.func.signature.call_conv = module.make_signature().call_conv;
    context.set_disasm(true);
    let id = module
        .declare_function(
            "selected_publication",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let assembly = context.compiled_code().unwrap().vcode.clone().unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: retained function uses exactly these two pointers and two words.
    let copy =
        unsafe { core::mem::transmute::<*const u8, Copy>(module.get_finalized_function(id)) };
    (module, copy, assembly)
}
fn relation(op: Comparison, value: u64, bound: u64) -> bool {
    match op {
        Comparison::Eq => value == bound,
        Comparison::Ne => value != bound,
        Comparison::Lt => value < bound,
        Comparison::Le => value <= bound,
        Comparison::Gt => value > bound,
        Comparison::Ge => value >= bound,
    }
}
fn selected(spec: Spec, row: u64, column: u64, columns: u64, a: u64, c: u64) -> bool {
    match spec {
        Spec::Linear(op) => relation(op, row * columns + column + 1, a),
        Spec::Rows(op) => relation(op, row + 1, a),
        Spec::Columns(op) => relation(op, column + 1, a),
        Spec::Rectangle(left, right) => {
            relation(left, row + 1, a) && relation(right, column + 1, c)
        }
    }
}
#[test]
fn selected_map_spans_runtime_bounds_cover_exact_old_set_poisoned_holes_unaligned_banks_and_tails()
{
    let ops = [
        Comparison::Eq,
        Comparison::Ne,
        Comparison::Lt,
        Comparison::Le,
        Comparison::Gt,
        Comparison::Ge,
    ];
    let specs = ops
        .into_iter()
        .flat_map(|op| [Spec::Linear(op), Spec::Rows(op), Spec::Columns(op)])
        .chain([
            Spec::Rectangle(Comparison::Eq, Comparison::Ge),
            Spec::Rectangle(Comparison::Ne, Comparison::Ne),
            Spec::Rectangle(Comparison::Lt, Comparison::Gt),
            Spec::Rectangle(Comparison::Le, Comparison::Eq),
            Spec::Rectangle(Comparison::Ge, Comparison::Le),
            Spec::Rectangle(Comparison::Gt, Comparison::Lt),
        ])
        .collect::<Vec<_>>();
    for (rows, columns) in [(0, 0), (0, 7), (7, 0), (1, 17), (19, 1), (3, 4), (7, 9)] {
        for width in [1, 2, 4, 8] {
            for &spec in &specs {
                let (_new, spans, _) = compile(function(spec, rows, columns, width, true));
                let (_old, ordinary, _) = compile(function(spec, rows, columns, width, false));
                let len = (rows * columns * u64::from(width)) as usize;
                for a in [0, 1, 2, rows, columns, rows.max(columns) + 1, u64::MAX] {
                    for c in [0, 1, 3, columns, columns + 1, u64::MAX] {
                        for pad in [1, 9] {
                            let mut source = vec![0xCC; pad + len + 17];
                            let mut expected = vec![0xA5; pad + len + 17];
                            for row in 0..rows {
                                for column in 0..columns {
                                    if selected(spec, row, column, columns, a, c) {
                                        let index = (row * columns + column) as usize;
                                        let bits = [
                                            0u64,
                                            1,
                                            0x8000_0000_0000_0000,
                                            0x7ff8_0000_0000_0037,
                                            0x7ff0_0000_0000_0041,
                                            0xfff0_0000_0000_0000,
                                        ][index % 6]
                                            .to_ne_bytes();
                                        let at = pad + index * usize::from(width);
                                        source[at..at + usize::from(width)]
                                            .copy_from_slice(&bits[..usize::from(width)]);
                                        expected[at..at + usize::from(width)]
                                            .copy_from_slice(&source[at..at + usize::from(width)]);
                                    }
                                }
                            }
                            let before = source.clone();
                            let mut actual = vec![0xA5; source.len()];
                            let mut old = actual.clone();
                            // SAFETY: complete banks and each selected cell lie in these live byte slices; padding deliberately misaligns the logical bank.
                            unsafe {
                                spans(source.as_ptr().add(pad), actual.as_mut_ptr().add(pad), a, c);
                                ordinary(source.as_ptr().add(pad), old.as_mut_ptr().add(pad), a, c);
                            }
                            assert_eq!(
                                actual, expected,
                                "{spec:?} {rows}x{columns} w{width} bounds{a},{c} pad{pad}"
                            );
                            assert_eq!(actual, old);
                            assert_eq!(source, before);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn selected_map_spans_actual_ssa_has_vector_copies_without_per_cell_coordinate_division() {
    let spec = Spec::Rectangle(Comparison::Ne, Comparison::Ge);
    let old = function(spec, 7, 19, 8, false);
    let new = function(spec, 7, 19, 8, true);
    let divisions = |f: &Function| {
        f.layout
            .blocks()
            .flat_map(|b| f.layout.block_insts(b))
            .filter(|&i| matches!(f.dfg.insts[i].opcode(), Opcode::Udiv | Opcode::Urem))
            .count()
    };
    assert!(divisions(&old) > 0);
    assert_eq!(divisions(&new), 0);
    assert!(
        new.layout
            .blocks()
            .flat_map(|b| new.layout.block_insts(b))
            .any(|i| new.dfg.insts[i].opcode() == Opcode::Load
                && new.dfg.value_type(new.dfg.first_result(i)) == types::I8X16)
    );
    let (_module, _, assembly) = compile(new);
    if cfg!(target_arch = "aarch64") {
        let machine = assembly.split(";; REGALLOC_INPUT_BEGIN").next().unwrap();
        assert!(machine.contains("q") || machine.contains("v"), "{machine}");
        assert!(!machine.contains("udiv "), "{machine}");
        assert!(
            !machine.contains("fmadd ") && !machine.contains("fadd "),
            "{machine}"
        );
    }
}
