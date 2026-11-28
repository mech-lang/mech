//! Execute the same rolling emitter and store-completion protocol over private
//! test banks. Production source eligibility is exercised separately below.
use super::*;
use cranelift_codegen::ir::{AbiParam, Signature};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
fn point() -> (Prefix, Recipe) {
    let mut kinds = BTreeMap::from([
        (10, Kind::U64),
        (11, Kind::U64),
        (12, Kind::U64),
        (13, Kind::U64),
    ]);
    let mut steps = vec![
        (10, Step::Primitive(Op::U64Sub, vec![1, 3])),
        (11, Step::Primitive(Op::U64Add, vec![1, 3])),
        (12, Step::Primitive(Op::U64Sub, vec![2, 3])),
        (13, Step::Primitive(Op::U64Add, vec![2, 3])),
    ];
    let mut offsets = BTreeMap::new();
    let mut id = 20;
    for (r, dr) in [(10, -1), (1, 0), (11, 1)] {
        for (c, dc) in [(12, -1), (2, 0), (13, 1)] {
            steps.push((
                id,
                Step::Read {
                    array: 0,
                    row: r,
                    column: c,
                },
            ));
            kinds.insert(id, Kind::F64);
            offsets.insert(id, (dr, dc));
            id += 1;
        }
    }
    let mut accumulator = 20;
    for next in 21..=28 {
        let output = 40 + next - 21;
        steps.push((output, Step::Primitive(Op::F64Add, vec![accumulator, next])));
        kinds.insert(output, Kind::F64);
        accumulator = output;
    }
    steps.push((48, Step::Primitive(Op::F64Div, vec![accumulator, 4])));
    kinds.insert(48, Kind::F64);
    let prefix = Prefix {
        first: 0,
        next: 100,
        sites: (0..32).collect(),
        read_sites: (0..9).collect(),
        exports: vec![48],
        inputs: BTreeMap::from([
            (1, Kind::U64),
            (2, Kind::U64),
            (3, Kind::U64),
            (4, Kind::F64),
        ]),
        kinds,
        steps,
    };
    let recipe = Recipe {
        first: 0,
        write: 100,
        latch: 200,
        continuation: 199,
        array: 0,
        row: 1,
        column: 2,
        result: 48,
        old: Some(24),
        keys: offsets.values().copied().collect(),
        offsets,
        max_column: 1,
        max_checked: 1,
        new_reads: 3,
    };
    (prefix, recipe)
}
type Kernel = unsafe extern "C" fn(*const u64, *mut u64) -> u64;
fn compile() -> (JITModule, Kernel) {
    let (prefix, recipe) = point();
    let mut module =
        JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
    let mut context = module.make_context();
    context.func.signature = Signature {
        params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
        returns: vec![AbiParam::new(types::I64)],
        call_conv: module.isa().default_call_conv(),
    };
    let function = module
        .declare_function("rolling_cells", Linkage::Export, &context.func.signature)
        .unwrap();
    let mut bc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut bc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        let out = b.block_params(entry)[1];
        let mut load = |at| b.ins().load(types::I64, MemFlags::new(), frame, at);
        let pointer = load(0);
        let rows = load(8);
        let columns = load(16);
        let first_row = load(24);
        let end_row = load(32);
        let first_column = load(40);
        let end_column = load(48);
        let steps = load(56);
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let gain = b.ins().iconst(types::I64, 9f64.to_bits() as i64);
        let cache = Cache::new(&mut b, recipe);
        let time = b.declare_var(types::I64);
        let row = b.declare_var(types::I64);
        let column = b.declare_var(types::I64);
        let fresh = b.declare_var(types::I64);
        let reuse = b.declare_var(types::I64);
        let count = b.declare_var(types::I64);
        for at in [time, row, column, fresh, reuse, count] {
            b.def_var(at, zero);
        }
        let time_head = b.create_block();
        let time_body = b.create_block();
        let row_head = b.create_block();
        let row_body = b.create_block();
        let col_head = b.create_block();
        let body = b.create_block();
        let row_done = b.create_block();
        let time_done = b.create_block();
        let done = b.create_block();
        let slow = b.create_block();
        b.ins().jump(time_head, &[]);
        b.switch_to_block(time_head);
        let t = b.use_var(time);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, t, steps);
        b.ins().brif(more, time_body, &[], done, &[]);
        b.switch_to_block(time_body);
        cache.invalidate(&mut b);
        b.def_var(row, first_row);
        b.ins().jump(row_head, &[]);
        b.switch_to_block(row_head);
        let r = b.use_var(row);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, r, end_row);
        b.ins().brif(more, row_body, &[], time_done, &[]);
        b.switch_to_block(row_body);
        cache.invalidate(&mut b);
        b.def_var(column, first_column);
        b.ins().jump(col_head, &[]);
        b.switch_to_block(col_head);
        let c = b.use_var(column);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, c, end_column);
        b.ins().brif(more, body, &[], row_done, &[]);
        b.switch_to_block(body);
        let inputs = BTreeMap::from([(1, r), (2, c), (3, one), (4, gain)]);
        let views = BTreeMap::from([(0, [pointer, rows, columns])]);
        let result = super::super::evaluate(
            &mut b,
            OptimizationOptions::default(),
            &prefix,
            &inputs,
            &views,
            slow,
            Some(&cache),
        );
        let marker = result.reused.unwrap();
        let marker = b.ins().uextend(types::I64, marker);
        let n = b.use_var(reuse);
        let n = b.ins().iadd(n, marker);
        b.def_var(reuse, n);
        let other = b.ins().isub(one, marker);
        let n = b.use_var(fresh);
        let n = b.ins().iadd(n, other);
        b.def_var(fresh, n);
        let value = result.words.iter().find(|(id, _)| *id == 48).unwrap().1;
        let r0 = b.ins().iadd_imm(r, -1);
        let c0 = b.ins().iadd_imm(c, -1);
        let base = b.ins().imul(r0, columns);
        let index = b.ins().iadd(base, c0);
        let bytes = b.ins().ishl_imm(index, 3);
        let address = b.ins().iadd(pointer, bytes);
        b.ins().store(MemFlags::new(), value, address, 0);
        cache.finish_write(&mut b, 100);
        let n = b.use_var(count);
        let n = b.ins().iadd_imm(n, 1);
        b.def_var(count, n);
        let next = b.ins().iadd_imm(c, 1);
        b.def_var(column, next);
        b.ins().jump(col_head, &[]);
        b.switch_to_block(row_done);
        let next = b.ins().iadd_imm(r, 1);
        b.def_var(row, next);
        b.ins().jump(row_head, &[]);
        b.switch_to_block(time_done);
        let next = b.ins().iadd_imm(t, 1);
        b.def_var(time, next);
        b.ins().jump(time_head, &[]);
        for (block, status) in [(done, 0), (slow, 1)] {
            b.switch_to_block(block);
            for (i, at) in [fresh, reuse, count].iter().enumerate() {
                let value = b.use_var(*at);
                b.ins().store(MemFlags::new(), value, out, (i * 8) as i32);
            }
            let status = b.ins().iconst(types::I64, status);
            b.ins().return_(&[status]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    module.define_function(function, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: actual emitted function has this exact two-pointer signature.
    let kernel = unsafe {
        core::mem::transmute::<*const u8, Kernel>(module.get_finalized_function(function))
    };
    (module, kernel)
}
fn reference(data: &mut [u64], shape: [usize; 2], range: [usize; 4], steps: usize) -> (u64, u64) {
    let [_, columns] = shape;
    let [first_row, end_row, first_column, end_column] = range;
    let mut count = 0;
    for _ in 0..steps {
        for r in first_row..end_row {
            for c in first_column..end_column {
                if r <= 1 || c <= 1 || r >= shape[0] || c >= columns {
                    return (1, count);
                }
                let mut values = Vec::new();
                for i in [r - 1, r, r + 1] {
                    for j in [c - 1, c, c + 1] {
                        values.push(f64::from_bits(data[(i - 1) * columns + j - 1]));
                    }
                }
                let mut value = values[0];
                for term in &values[1..] {
                    value += term;
                }
                value /= 9.0;
                if !value.is_finite() {
                    return (1, count);
                }
                data[(r - 1) * columns + c - 1] = value.to_bits();
                count += 1;
            }
        }
    }
    (0, count)
}
#[test]
fn cell_prefix_rolling_native_reuses_six_values_and_resets_rows_and_turns() {
    let (_module, kernel) = compile();
    for shape in [[3usize, 3usize], [5, 8], [8, 5]] {
        for steps in [0usize, 1, 3] {
            let cells = (0..shape[0] * shape[1])
                .map(|n| ((n as f64 - 4.25) / 7.0).to_bits())
                .collect::<Vec<_>>();
            let mut expected = cells.clone();
            let range = [2, shape[0], 2, shape[1]];
            let (status, count) = reference(&mut expected, shape, range, steps);
            let mut bank = vec![0xface_dead_beef_cafe];
            bank.extend(cells);
            bank.push(0xface_dead_beef_cafe);
            let frame = [
                bank.as_mut_ptr().wrapping_add(1) as u64,
                shape[0] as u64,
                shape[1] as u64,
                2,
                shape[0] as u64,
                2,
                shape[1] as u64,
                steps as u64,
            ];
            let mut counters = [0u64; 3];
            // SAFETY: complete initialized shape and three-word counter buffer.
            assert_eq!(
                unsafe { kernel(frame.as_ptr(), counters.as_mut_ptr()) },
                status
            );
            assert_eq!(counters[2], count);
            assert_eq!(&bank[1..bank.len() - 1], expected);
            assert_eq!(bank[0], bank[bank.len() - 1]);
            let first = (shape[0] - 2) * steps;
            assert_eq!(counters[0], first as u64);
            assert_eq!(counters[1], count - first as u64);
            assert_eq!(
                counters[0] * 9 + counters[1] * 3,
                first as u64 * 9 + (count - first as u64) * 3,
                "three newly exposed source loads on a rolling point"
            );
        }
    }
}
#[test]
fn cell_prefix_rolling_native_late_upper_and_nonfinite_refuse_without_current_store() {
    let (_module, kernel) = compile();
    let shape = [5usize, 8usize];
    for (special, index, last) in [
        (1.25f64.to_bits(), 0, 9),
        (f64::INFINITY.to_bits(), 15, 8),
        (0x7ff0_1234_5678_9abc, 23, 8),
        (0xfff8_1234_5678_abcd, 31, 8),
    ] {
        let mut cells = (0..40)
            .map(|n| (n as f64 / 11.0).to_bits())
            .collect::<Vec<_>>();
        cells[index] = special;
        let mut expected = cells.clone();
        let (status, count) = reference(&mut expected, shape, [2, 5, 2, last], 1);
        assert_eq!(status, 1);
        let frame = [cells.as_mut_ptr() as u64, 5, 8, 2, 5, 2, last as u64, 1];
        let mut counters = [0u64; 3];
        // SAFETY: complete initialized bank; refusal must precede failing store.
        assert_eq!(
            unsafe { kernel(frame.as_ptr(), counters.as_mut_ptr()) },
            status
        );
        assert_eq!(counters[2], count);
        assert_eq!(cells, expected);
    }
    for (rows, columns, row, column) in [
        (0, 8, 2, 2),
        (5, 0, 2, 2),
        (5, 8, 0, 2),
        (5, 8, 2, 0),
        (5, 8, u64::MAX - 1, 2),
    ] {
        let frame = [0, rows, columns, row, row + 1, column, column + 1, 1];
        let mut counters = [0u64; 3];
        // SAFETY: complete admission must reject the inactive null source.
        assert_eq!(unsafe { kernel(frame.as_ptr(), counters.as_mut_ptr()) }, 1);
        assert_eq!(counters, [0, 0, 0]);
    }
}
