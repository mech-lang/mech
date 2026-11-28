//! Typed independent cells over proved affine streams. Complete current map
//! admission is unchanged. A failed speculative window restarts the checked
//! DAG over immutable sources; only its private disjoint scratch was touched.
use super::*;
use crate::native_artifact::compiler::f64_arithmetic;
use crate::native_artifact::program::codegen::numeric_map::selection::Selection;
use crate::native_artifact::program::prepare::numeric_maps::dots::{
    CellPath, CellProgram, CellValue,
};
use mech_engine::native_artifact::NativeScalarOp;

fn evaluate(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    path: &CellPath,
    prepared: &[streams::Prepared],
    x: Value,
    y: Value,
    vector: bool,
) -> Result<Value, NativeCompileError> {
    let kind = if vector {
        types::F64.by(2).unwrap()
    } else {
        types::F64
    };
    let mut values = Vec::new();
    for node in &path.values {
        let value = match *node {
            CellValue::Constant(bits) => {
                let value = b
                    .ins()
                    .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(bits));
                if vector {
                    b.ins().splat(kind, value)
                } else {
                    value
                }
            }
            CellValue::Capture(id) => {
                let word = *context
                    .captures
                    .get(&id)
                    .ok_or_else(|| backend("affine cell lost current F64 capture"))?;
                let value = b.ins().bitcast(types::F64, MemFlags::new(), word);
                if vector {
                    b.ins().splat(kind, value)
                } else {
                    value
                }
            }
            CellValue::Read(at) => prepared[at].cell(b, x, y, vector),
            CellValue::Unary(NativeScalarOp::F64Neg, at) => b.ins().fneg(values[at]),
            CellValue::Unary(NativeScalarOp::F64Sqrt, at) => b.ins().sqrt(values[at]),
            CellValue::Binary(op, [a, c]) => f64_arithmetic(b, op, values[a], values[c]),
            _ => return Err(backend("affine cell lost source-selected F64 operation")),
        };
        // Preserve all intermediate rounding and defer nonfinite payload cases
        // to the original scalar DAG. No store precedes these checks.
        super::super::numeric_product_epilogues::finite(b, value, vector, context.slow);
        values.push(value);
    }
    Ok(values[path.result])
}
fn window(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    path: &CellPath,
    matrices: &[(Value, Value, Value, Value)],
    first: [Value; 2],
    counts: [Value; 2],
    source_start: [Value; 2],
    source_columns: Value,
    rectangular: bool,
    output: Value,
) -> Result<(), NativeCompileError> {
    let work = b.create_block();
    let done = b.create_block();
    let nx = b.ins().icmp_imm(IntCC::NotEqual, counts[0], 0);
    let ny = b.ins().icmp_imm(IntCC::NotEqual, counts[1], 0);
    let active = b.ins().band(nx, ny);
    b.ins().brif(active, work, &[], done, &[]);
    b.switch_to_block(work);
    let mut prepared = Vec::new();
    for operand in &path.reads {
        let (p, n, r, c) = matrices[operand.matrix];
        prepared.push(streams::prepare(
            b,
            context,
            operand,
            [p, n, r, c],
            first[0],
            counts[0],
            rectangular.then_some((first[1], counts[1])),
        )?);
    }
    let mut packed = b.ins().iconst(types::I8, 1);
    for stream in &prepared {
        let contiguous = stream.packed_inner(b);
        packed = b.ins().band(packed, contiguous);
    }
    let column = b.ins().isub(first[0], source_start[0]);
    let row = b.ins().isub(first[1], source_start[1]);
    let row_base = b.ins().imul(row, source_columns);
    let index = b.ins().iadd(row_base, column);
    let bytes = b.ins().imul_imm(index, 8);
    let base = b.ins().iadd(output, bytes);
    let x = b.declare_var(types::I64);
    let y = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(y, zero);
    let rows = b.create_block();
    let row_body = b.create_block();
    let head = b.create_block();
    let body = b.create_block();
    let pair = b.create_block();
    let single = b.create_block();
    let row_done = b.create_block();
    b.ins().jump(rows, &[]);
    b.switch_to_block(rows);
    let at_y = b.use_var(y);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at_y, counts[1]);
    b.ins().brif(more, row_body, &[], done, &[]);
    b.switch_to_block(row_body);
    b.def_var(x, zero);
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let at_x = b.use_var(x);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at_x, counts[0]);
    b.ins().brif(more, body, &[], row_done, &[]);
    b.switch_to_block(body);
    let remaining = b.ins().isub(counts[0], at_x);
    let two = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    let two = b.ins().band(two, packed);
    b.ins().brif(two, pair, &[], single, &[]);
    for (block, vector, step) in [(pair, true, 2), (single, false, 1)] {
        b.switch_to_block(block);
        let result = evaluate(b, context, path, &prepared, at_x, at_y, vector)?;
        let row = b.ins().imul(at_y, source_columns);
        let at = b.ins().iadd(row, at_x);
        let bytes = b.ins().imul_imm(at, 8);
        let address = b.ins().iadd(base, bytes);
        b.ins().store(MemFlags::new(), result, address, 0);
        let next = b.ins().iadd_imm(at_x, step);
        b.def_var(x, next);
        b.ins().jump(head, &[]);
    }
    b.switch_to_block(row_done);
    let next = b.ins().iadd_imm(at_y, 1);
    b.def_var(y, next);
    b.ins().jump(rows, &[]);
    b.switch_to_block(done);
    Ok(())
}
/// Returns in the original DAG's entry block. No runtime recipe is serialized.
pub(in crate::native_artifact::program::codegen) fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    region: &Region,
    plan: &Plan,
    captures: &BTreeMap<u32, Value>,
    matrices: &[(Value, Value, Value, Value)],
    start: Value,
    count: Value,
    source_count: Value,
    inner_start: Value,
    inner_count: Value,
    output: Value,
    selection: Option<&Selection>,
    whole_initialized: Option<Value>,
    complete: Block,
) -> Result<(), NativeCompileError> {
    let Some(recipe) = CellProgram::derive(plan, region) else {
        return Ok(());
    };
    // Selection already proves one exact source edge. Keep only compatible
    // complete paths; no runtime coordinate or arbitrary predicate is trusted.
    let paths = recipe
        .paths
        .iter()
        .filter(|path| {
            !selection.is_some_and(|selection| {
                path.edges.iter().any(|&(site, to)| {
                    selection
                        .selected_edge(site)
                        .is_some_and(|active| active != to)
                })
            })
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(());
    }
    let slow = b.create_block();
    let arithmetic = b.create_block();
    b.append_block_param(arithmetic, types::I64);
    let context = Context {
        plan,
        captures,
        slow,
        arithmetic,
    };
    // A speculative later decline can replay only when writes cannot change a
    // source. Keep the full-span proof even for currently empty selected arms.
    let output_bytes = b.ins().imul_imm(count, 8);
    let output_end = b.ins().iadd(output, output_bytes);
    for (at, &(pointer, cells, _, _)) in matrices.iter().enumerate() {
        let bytes = b
            .ins()
            .imul_imm(cells, i64::from(region.matrices[at].width));
        let end = b.ins().iadd(pointer, bytes);
        let before = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, output_end, pointer);
        let after = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, output);
        let disjoint = b.ins().bor(before, after);
        let empty = b.ins().icmp_imm(IntCC::Equal, cells, 0);
        let good = b.ins().bor(empty, disjoint);
        require(b, good, slow);
    }
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let rectangular = region.rectangle.is_some();
    let source_start = [start, if rectangular { inner_start } else { zero }];
    let boxes = selection.map_or_else(
        || {
            vec![(
                [zero, zero],
                [source_count, if rectangular { inner_count } else { one }],
            )]
        },
        |s| s.windows(b),
    );
    for path in paths {
        for &(offset, counts) in &boxes {
            let first = [
                b.ins().iadd(start, offset[0]),
                if rectangular {
                    b.ins().iadd(inner_start, offset[1])
                } else {
                    zero
                },
            ];
            let mut windows = vec![(first, counts)];
            for predicate in &path.predicates {
                let bound = context.word(b, predicate.bound)?;
                let axis = predicate.axis as usize;
                let mut clipped = Vec::new();
                for (first, counts) in windows {
                    for (offset, n) in super::windows::comparison(
                        b,
                        predicate.op,
                        bound,
                        first[axis],
                        counts[axis],
                        false,
                    ) {
                        let mut new_first = first;
                        let mut new_counts = counts;
                        new_first[axis] = b.ins().iadd(first[axis], offset);
                        new_counts[axis] = n;
                        clipped.push((new_first, new_counts));
                    }
                }
                windows = clipped;
            }
            for (first, counts) in windows {
                window(
                    b,
                    &context,
                    path,
                    matrices,
                    first,
                    counts,
                    source_start,
                    source_count,
                    rectangular,
                    output,
                )?;
            }
        }
    }
    // Same complete-prefix certificate as the checked selected map. Holes in
    // selected-only scratch never become initialized merely through fast code.
    let initialized = whole_initialized
        .map(|whole| b.ins().select(whole, count, zero))
        .unwrap_or(count);
    b.ins().store(
        MemFlags::new(),
        initialized,
        frame,
        offset_of!(Frame, next) as i32,
    );
    b.ins().jump(complete, &[]);
    b.switch_to_block(arithmetic);
    b.ins().jump(slow, &[]);
    b.switch_to_block(slow);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, Function};
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_module::{Linkage, Module, default_libcall_names};
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    type Kernel = unsafe extern "C" fn(
        *mut Frame,
        *const f64,
        *const f64,
        *const f64,
        *mut u64,
        u64,
        u64,
        u64,
        u64,
        u64,
    ) -> u64;
    fn function() -> Function {
        let source = "a := left<[f64]>\nx := rows<[f64]>\ny := columns<[f64]>\n[[((j > cut<u64>)? | true => a[i,j] - x[i] * y[j] | false => a[i,j]) | i <- 1u64..=last_row<u64>]' | j <- 1u64..=last_column<u64>]\n";
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x4146434b), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let catalog = mech_stdlib::source_catalog();
        let artifact = mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let plan = mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog)
            .unwrap();
        let layout =
            crate::native_artifact::program::prepare::numeric_maps::Layout::derive(&plan).unwrap();
        let region = layout
            .regions
            .iter()
            .find(|r| r.rectangle.is_some() && CellProgram::derive(&plan, r).is_some())
            .unwrap();
        let recipe = CellProgram::derive(&plan, region).unwrap();
        assert_eq!(recipe.paths.len(), 2);
        let mut f = Function::new();
        f.signature.params = vec![AbiParam::new(types::I64); 10];
        f.signature.returns = vec![AbiParam::new(types::I64)];
        let mut context = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut f, &mut context);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let one = b.ins().iconst(types::I64, 1);
            let count = b.ins().imul(args[5], args[6]);
            let captures = region
                .captures
                .iter()
                .map(|&id| (id, args[7]))
                .collect::<BTreeMap<_, _>>();
            let mut matrices = Vec::new();
            for at in 0..region.matrices.len() {
                let operand = recipe
                    .paths
                    .iter()
                    .flat_map(|p| &p.reads)
                    .find(|o| o.matrix == at)
                    .unwrap();
                matrices.push(if operand.indices.len() == 2 {
                    (args[1], count, args[5], args[6])
                } else if operand.indices[0].dependencies[1] {
                    (args[2], args[8], args[8], one)
                } else {
                    (args[3], args[9], one, args[9])
                });
            }
            let complete = b.create_block();
            // The production caller validates complete frames/spans. This
            // wrapper supplies exact live banks and exposes fast/declined
            // status without executing the original checked DAG afterward.
            emit(
                &mut b, args[0], region, &plan, &captures, &matrices, one, count, args[6], one,
                args[5], args[4], None, None, complete,
            )
            .unwrap();
            let declined = b.ins().iconst(types::I64, 1);
            b.ins().return_(&[declined]);
            b.switch_to_block(complete);
            let fast = b.ins().iconst(types::I64, 0);
            b.ins().return_(&[fast]);
            b.seal_all_blocks();
            b.finalize();
        }
        f
    }
    #[test]
    fn affine_map_cells_direct_dynamic_kernel_simd_tails_null_inactive_bounds_alias_and_canaries() {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func = function();
        context.func.signature.call_conv = module.make_signature().call_conv;
        context.set_disasm(true);
        let id = module
            .declare_function("affine_cells", Linkage::Local, &context.func.signature)
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        let asm = context.compiled_code().unwrap().vcode.as_ref().unwrap();
        assert!(!asm.contains("fmadd") && !asm.contains("fmla"), "{asm}");
        assert!(
            asm.contains("f64x2") || asm.contains("v0.2d") || asm.contains(".2d"),
            "independent two-lane F64 cells: {asm}"
        );
        module.finalize_definitions().unwrap();
        // SAFETY: exact ten-word signature and retained executable module.
        let kernel =
            unsafe { core::mem::transmute::<*const u8, Kernel>(module.get_finalized_function(id)) };
        let sentinel = 0xbad0_bad0_bad0_bad0;
        for rows in [0, 1, 2, 3, 5] {
            for columns in [0, 1, 2, 3, 5, 8] {
                for cut in [0, 1, 3, 8, u64::MAX] {
                    let a = (0..rows * columns)
                        .map(|i| i as f64 * 0.25 - 1.0)
                        .collect::<Vec<_>>();
                    let x = (0..rows).map(|i| i as f64 - 0.5).collect::<Vec<_>>();
                    let y = (0..columns).map(|i| i as f64 + 0.75).collect::<Vec<_>>();
                    let mut out = vec![sentinel; rows * columns + 2];
                    let mut frame = Frame::default();
                    // SAFETY: all used inputs/output are exact live disjoint banks.
                    let status = unsafe {
                        kernel(
                            &mut frame,
                            a.as_ptr(),
                            x.as_ptr(),
                            y.as_ptr(),
                            out.as_mut_ptr().add(1),
                            rows as u64,
                            columns as u64,
                            cut,
                            rows as u64,
                            columns as u64,
                        )
                    };
                    assert_eq!(status, 0, "fast kernel for {rows}/{columns}/{cut}");
                    assert_eq!(frame.next, (rows * columns) as u64);
                    assert_eq!(out[0], sentinel);
                    assert_eq!(*out.last().unwrap(), sentinel);
                    for r in 0..rows {
                        for c in 0..columns {
                            let expected = if c as u64 + 1 > cut {
                                let term = std::hint::black_box(x[r] * y[c]);
                                std::hint::black_box(a[r * columns + c] - term)
                            } else {
                                a[r * columns + c]
                            };
                            assert_eq!(
                                out[1 + r * columns + c],
                                expected.to_bits(),
                                "{rows}/{columns}/{cut} ({r},{c})"
                            );
                        }
                    }
                }
            }
        }
        let a = vec![1.0f64; 15];
        let mut out = vec![sentinel; 15];
        let mut frame = Frame::default();
        // SAFETY: j>MAX is inactive. Empty null X/Y views cannot be accessed.
        assert_eq!(
            unsafe {
                kernel(
                    &mut frame,
                    a.as_ptr(),
                    core::ptr::null(),
                    core::ptr::null(),
                    out.as_mut_ptr(),
                    3,
                    5,
                    u64::MAX,
                    0,
                    0,
                )
            },
            0
        );
        assert!(out.iter().all(|&v| v == 1.0f64.to_bits()));
        frame.next = 0;
        out.fill(sentinel);
        // SAFETY: active arm has an empty X/Y bank; preflight must decline.
        assert_eq!(
            unsafe {
                kernel(
                    &mut frame,
                    a.as_ptr(),
                    core::ptr::null(),
                    core::ptr::null(),
                    out.as_mut_ptr(),
                    3,
                    5,
                    0,
                    0,
                    0,
                )
            },
            1
        );
        assert_eq!(frame.next, 0);
        assert!(out.iter().all(|&v| v == sentinel));
        let x = vec![2.0f64; 3];
        let y = vec![3.0f64; 5];
        let mut a = a;
        let before = a.clone();
        // SAFETY: the alias case is rejected before any speculative store.
        assert_eq!(
            unsafe {
                kernel(
                    &mut frame,
                    a.as_ptr(),
                    x.as_ptr(),
                    y.as_ptr(),
                    a.as_mut_ptr().cast(),
                    3,
                    5,
                    0,
                    3,
                    5,
                )
            },
            1
        );
        assert_eq!(a, before);
        assert_eq!(frame.next, 0);
        a[14] = f64::INFINITY;
        out.fill(sentinel);
        // SAFETY: complete banks; late nonfinite result leaves only private
        // scratch partial and grants no complete-prefix certificate.
        assert_eq!(
            unsafe {
                kernel(
                    &mut frame,
                    a.as_ptr(),
                    x.as_ptr(),
                    y.as_ptr(),
                    out.as_mut_ptr(),
                    3,
                    5,
                    0,
                    3,
                    5,
                )
            },
            1
        );
        assert_eq!(frame.next, 0);
        assert!(out.iter().any(|&v| v != sentinel));
        // SAFETY: the completely empty map never dereferences null pointers.
        assert_eq!(
            unsafe {
                kernel(
                    &mut frame,
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null_mut(),
                    0,
                    0,
                    u64::MAX,
                    0,
                    0,
                )
            },
            0
        );
    }
}
