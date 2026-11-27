//! Production rectangle emission: exact native bits, source error order and
//! malformed-frame guards, independently of the maintained runtime loan tests.
use super::geometry_tests::compile_source;
use super::*;
use cranelift_codegen::ir::{AbiParam, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};

struct Kernel {
    call: unsafe extern "C" fn(*mut Frame) -> u64,
    region: Region,
    _module: cranelift_jit::JITModule,
}
impl Kernel {
    fn new(kind: &str, range: bool, failure: bool) -> Self {
        Self::rows(kind, range, failure, 2)
    }
    fn rows(kind: &str, range: bool, failure: bool, inner_rows: usize) -> Self {
        let source = if range { "1u64..=3u64" } else { "items<[u64]>" };
        let index = if failure {
            "((j == 2u64)? | true => i + 1u64 | false => i)"
        } else {
            "i"
        };
        let jit = compile_source(format!(
            "data := values<[{kind}]:2,3>\n[[data[{index},j] | i <- 1u64..={inner_rows}u64]' | j <- {source}]\n"
        ));
        let metadata = jit.descriptor().metadata();
        let region = metadata
            .numeric_maps
            .regions
            .iter()
            .find(|r| r.rectangle.is_some())
            .unwrap()
            .clone();
        assert_eq!(region.rectangle.as_ref().unwrap().rows, inner_rows as u64);
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut cx = module.make_context();
        cx.func.signature.params = vec![AbiParam::new(types::I64)];
        cx.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function("native_rectangle", Linkage::Local, &cx.func.signature)
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut cx.func, &mut fc);
            let root = b.create_block();
            b.append_block_params_for_function_params(root);
            b.switch_to_block(root);
            let frame = b.block_params(root)[0];
            let mut captures = BTreeMap::new();
            for &value in &region.captures {
                let constant = metadata
                    .plan
                    .constants()
                    .iter()
                    .find(|c| c.value().get() == value)
                    .expect("only literal captures in the source fixture");
                let word = match constant.constant().data() {
                    mech_core::ValueData::U64(word) => *word,
                    mech_core::ValueData::Bool(word) => u64::from(*word),
                    other => panic!("unexpected source literal {other:?}"),
                };
                captures.insert(value, b.ins().iconst(types::I64, word as i64));
            }
            let complete = b.create_block();
            let miss = b.create_block();
            b.append_block_param(miss, types::I64);
            let malformed = b.create_block();
            emit(
                &mut b,
                frame,
                &region,
                &metadata.plan,
                &captures,
                complete,
                miss,
                malformed,
            )
            .unwrap();
            for (block, status) in [(complete, 0), (miss, 1), (malformed, 2)] {
                b.switch_to_block(block);
                let status = b.ins().iconst(types::I64, status);
                b.ins().return_(&[status]);
            }
            b.seal_all_blocks();
            b.finalize();
        }
        // The production body only has two-axis reads and literal arithmetic.
        // Any division here would come from reconstructing rectangle cursors.
        for block in cx.func.layout.blocks() {
            for inst in cx.func.layout.block_insts(block) {
                assert!(
                    !matches!(
                        cx.func.dfg.insts[inst].opcode(),
                        Opcode::Udiv | Opcode::Urem
                    ),
                    "{}",
                    cx.func.display()
                );
            }
        }
        cx.verify(module.isa()).unwrap();
        module.define_function(id, &mut cx).unwrap();
        module.finalize_definitions().unwrap();
        let call = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        Self {
            call,
            region,
            _module: module,
        }
    }
    fn frame(&self, data: &[u8], items: &[u64], output: &mut [u8]) -> Frame {
        let width = self.region.output_width as usize;
        let count = self.region.rectangle.as_ref().unwrap().rows as usize * items.len();
        assert!(output.len() >= count * width);
        let mut frame = Frame {
            input: if self.region.range || items.is_empty() {
                0
            } else {
                items.as_ptr() as u64
            },
            input_bytes: if self.region.range {
                0
            } else {
                (items.len() * 8) as u64
            },
            output: if count == 0 {
                0
            } else {
                output.as_mut_ptr() as u64
            },
            capacity_bytes: (count * width) as u64,
            count: count as u64,
            source_count: items.len() as u64,
            output_count: count as u64,
            input_width: self.region.input_width as u64,
            output_width: width as u64,
            epoch: 17,
            issued: 17,
            range_start: 1,
            ..Default::default()
        };
        for matrix in &mut frame.matrices[..self.region.matrices.len()] {
            *matrix = MatrixView {
                input: data.as_ptr() as u64,
                count: 6,
                rows: 2,
                columns: 3,
            };
        }
        frame
    }
    fn run(&self, frame: &mut Frame) -> u64 {
        unsafe { (self.call)(frame) }
    }
}
fn bits(kind: &str, width: usize) -> Vec<u8> {
    let words = if kind == "bool" {
        [0u64, 1, 1, 0, 1, 0]
    } else {
        [
            0u64,
            1 << 63,
            0x7ff0_0000_0000_0031,
            0x7ff8_0000_0000_0042,
            0xfff8_0000_0000_0064,
            u64::MAX,
        ]
    };
    words
        .into_iter()
        .flat_map(|word| word.to_ne_bytes()[..width].to_vec())
        .collect()
}
#[test]
fn numeric_rectangular_cursors_keep_all_widths_exact_bits_axes_sources_and_empty_columns() {
    for (kind, width) in [
        ("bool", 1),
        ("u8", 1),
        ("u16", 2),
        ("u32", 4),
        ("u64", 8),
        ("i64", 8),
        ("f64", 8),
    ] {
        for rows in [1, 2] {
            for range in [false, true] {
                let kernel = Kernel::rows(kind, range, false, rows);
                let source = bits(kind, width);
                let before = source.clone();
                for items in [vec![], vec![1], vec![1, 2, 3], vec![3, 1, 2]] {
                    if range && items != [1, 2, 3] {
                        continue;
                    }
                    let mut output = vec![0xa5u8; rows * items.len() * width + 2];
                    let mut frame = kernel.frame(&source, &items, &mut output[1..]);
                    assert_eq!(kernel.run(&mut frame), 0);
                    assert_eq!(frame.next, rows as u64 * items.len() as u64);
                    let mut expected = vec![0xa5u8; output.len()];
                    for row in 0..rows {
                        for (column, &item) in items.iter().enumerate() {
                            let from = (row * 3 + item as usize - 1) * width;
                            let to = 1 + (row * items.len() + column) * width;
                            expected[to..to + width].copy_from_slice(&source[from..from + width]);
                        }
                    }
                    assert_eq!(output, expected, "{kind} range{range} {items:?}");
                    assert_eq!(source, before);
                }
            }
        }
    }
}
#[test]
fn numeric_rectangular_cursors_preserve_column_first_error_and_unfinished_frame() {
    let kernel = Kernel::new("f64", true, true);
    let source = bits("f64", 8);
    let mut output = vec![0xa5u8; 50];
    let mut frame = kernel.frame(&source, &[1, 2, 3], &mut output[1..]);
    assert_eq!(kernel.run(&mut frame), 1);
    assert_eq!(frame.next, 0);
    let mut expected = vec![0xa5u8; 50];
    // Column1 completes before column2 starts. In column2 row1 reads row2;
    // row2 then requests invalid row3. No later cell may have been visited.
    for (from, to) in [(0, 0), (3, 3), (4, 1)] {
        expected[1 + to * 8..1 + (to + 1) * 8].copy_from_slice(&source[from * 8..(from + 1) * 8]);
    }
    assert_eq!(output, expected);
}
#[test]
fn numeric_rectangular_cursors_reject_changed_geometry_epochs_and_overflow_before_stores() {
    for range in [false, true] {
        let kernel = Kernel::new("u64", range, false);
        let source = bits("u64", 8);
        for fault in 0..9 {
            let mut output = vec![0xa5u8; 50];
            let mut frame = kernel.frame(&source, &[1, 2, 3], &mut output[1..]);
            match fault {
                0 => frame.issued += 1,
                1 => frame.next = 1,
                2 => frame.count += 1,
                3 => frame.capacity_bytes -= 1,
                4 => frame.source_count = u64::MAX,
                5 => frame.matrices[0].count += 1,
                6 => frame.matrices[0].rows = 3,
                7 => frame.fold = 1,
                8 => {
                    if range {
                        frame.range_start = u64::MAX
                    } else {
                        frame.input = 0
                    }
                }
                _ => unreachable!(),
            }
            assert_eq!(kernel.run(&mut frame), 2, "range{range} fault{fault}");
            assert!(output.iter().all(|&b| b == 0xa5));
        }
    }
}
