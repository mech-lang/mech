//! Native higher-order array loops. Coefficients are typed Mech scalar graphs;
//! neither the backend nor runtime contains a particular matrix formula.
use cranelift_codegen::ir::{
    AbiParam, Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_sequence::{Callable, Constant, Kind, Program};
use std::collections::BTreeMap;

/// Checked unsigned arithmetic agrees with Mech's ordinary math operations.
/// Matrix range proofs may omit the checks only for the proven invocation.
pub(super) fn integer_arithmetic(
    b: &mut FunctionBuilder,
    op: u64,
    left: Value,
    right: Value,
    error: Option<Block>,
) -> Value {
    let (value, invalid) = match (op, error.is_some()) {
        (20, true) => b.ins().umul_overflow(left, right),
        (22, true) => b.ins().uadd_overflow(left, right),
        (23, true) => b.ins().usub_overflow(left, right),
        (21, true) => {
            let invalid = b.ins().icmp_imm(IntCC::Equal, right, 0);
            let next = b.create_block();
            let code = b.ins().iconst(types::I32, 2);
            b.ins()
                .brif(invalid, error.unwrap(), &[code.into()], next, &[]);
            b.switch_to_block(next);
            return b.ins().udiv(left, right);
        }
        (20, false) => return b.ins().imul(left, right),
        (21, false) => return b.ins().udiv(left, right),
        (22, false) => return b.ins().iadd(left, right),
        (23, false) => return b.ins().isub(left, right),
        _ => unreachable!("admitted unsigned arithmetic"),
    };
    let next = b.create_block();
    let code = b.ins().iconst(types::I32, 1);
    b.ins()
        .brif(invalid, error.unwrap(), &[code.into()], next, &[]);
    b.switch_to_block(next);
    value
}

fn arithmetic(
    b: &mut FunctionBuilder,
    op: u64,
    args: &[Value],
    ty: cranelift_codegen::ir::Type,
    error: Option<Block>,
) -> Value {
    if op == 19 {
        let value = b.ins().fcvt_from_uint(types::F64, args[0]);
        return if ty == types::F64X2 {
            b.ins().splat(ty, value)
        } else {
            value
        };
    }
    if b.func.dfg.value_type(args[0]) == types::I64 {
        return integer_arithmetic(b, op, args[0], args[1], error);
    }
    match op {
        20 => b.ins().fmul(args[0], args[1]),
        21 => b.ins().fdiv(args[0], args[1]),
        22 => b.ins().fadd(args[0], args[1]),
        23 => b.ins().fsub(args[0], args[1]),
        38 => b.ins().sqrt(args[0]),
        _ => unreachable!("admitted scalar arithmetic"),
    }
}
fn scalar_graph(
    b: &mut FunctionBuilder,
    function: &Callable,
    inputs: &[Value],
    previous: Option<&[Option<Value>]>,
    hoist: bool,
    varying: usize,
    error: Option<Block>,
) -> (Vec<Option<Value>>, Option<Value>) {
    let graph = &function.program;
    let ty = if inputs
        .iter()
        .any(|&v| b.func.dfg.value_type(v) == types::F64X2)
    {
        types::F64X2
    } else {
        types::F64
    };
    let mut values = Vec::new();
    let mut masks = Vec::new();
    for (id, c) in graph.constants.iter().enumerate() {
        let (value, mask) = match c {
            Constant::Integer(value) => (Some(b.ins().iconst(types::I64, *value as i64)), 0),
            Constant::Float(bits) => {
                let value = previous.and_then(|p| p[id]).unwrap_or_else(|| {
                    let value = b.ins().f64const(f64::from_bits(*bits));
                    if ty == types::F64 {
                        value
                    } else {
                        b.ins().splat(ty, value)
                    }
                });
                (Some(value), 0)
            }
            Constant::Parameter(index, _) => (
                if hoist && *index == varying {
                    None
                } else {
                    Some(inputs[*index])
                },
                1 << index,
            ),
            _ => (None, 0), // unrelated, precollected literals are not evaluated
        };
        values.push(value);
        masks.push(mask);
    }
    // CSE and loop invariants use graph dependencies, never source spellings.
    let mut common = BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let id = graph.constants.len() + index;
        let mask = node.args.iter().fold(0, |m, &i| m | masks[i]);
        let value = if let Some(value) = previous.and_then(|p| p[id]) {
            Some(value)
        } else if hoist && mask & (1 << varying) != 0 {
            None
        } else if let Some(value) = common.get(&(node.op, node.args.clone())) {
            Some(*value)
        } else {
            let args: Vec<_> = node
                .args
                .iter()
                .map(|&i| values[i].expect("typed scalar dependency"))
                .collect();
            let value = arithmetic(b, node.op, &args, ty, error);
            common.insert((node.op, node.args.clone()), value);
            Some(value)
        };
        values.push(value);
        masks.push(mask);
    }
    let output = values[function.output];
    (values, output)
}

/// Pack scalar lanes before the floating-point portion of an integer-indexed
/// coefficient graph. Unsigned operations are still evaluated lane-by-lane in
/// source graph order, so checked fallback errors retain their original order.
/// Float operations keep their DAG and rounding, but run once on two lanes.
fn scalar_graph_integer_simd(
    b: &mut FunctionBuilder,
    function: &Callable,
    inputs: &[[Value; 2]; 2],
    previous: &[Vec<Option<Value>>; 2],
    error: Option<Block>,
) -> Value {
    let graph = &function.program;
    let mut integers = Vec::with_capacity(2);
    for lane in 0..2 {
        let mut values: Vec<_> = graph
            .constants
            .iter()
            .map(|constant| match constant {
                Constant::Integer(value) => Some(b.ins().iconst(types::I64, *value as i64)),
                Constant::Parameter(index, Kind::Integer) => Some(inputs[lane][*index]),
                _ => None,
            })
            .collect();
        let mut common = BTreeMap::new();
        for (index, node) in graph.nodes.iter().enumerate() {
            let id = graph.constants.len() + index;
            let value = if graph.kinds[id] != Kind::Integer {
                None
            } else if let Some(value) = previous[lane][id] {
                Some(value)
            } else if let Some(value) = common.get(&(node.op, node.args.clone())) {
                Some(*value)
            } else {
                let left = values[node.args[0]].expect("typed integer prefix");
                let right = values[node.args[1]].expect("typed integer prefix");
                let value = integer_arithmetic(b, node.op, left, right, error);
                common.insert((node.op, node.args.clone()), value);
                Some(value)
            };
            values.push(value);
        }
        integers.push(values);
    }
    fn pack(b: &mut FunctionBuilder, left: Value, right: Value) -> Value {
        let vector = b.ins().splat(types::F64X2, left);
        if left == right {
            vector
        } else {
            b.ins().insertlane(vector, right, 1)
        }
    }
    let mut floats: Vec<_> = graph
        .constants
        .iter()
        .enumerate()
        .map(|(id, constant)| match constant {
            Constant::Float(bits) => {
                let value =
                    previous[0][id].unwrap_or_else(|| b.ins().f64const(f64::from_bits(*bits)));
                Some(pack(b, value, previous[1][id].unwrap_or(value)))
            }
            _ => None,
        })
        .collect();
    let mut common = BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let id = graph.constants.len() + index;
        let value = if graph.kinds[id] != Kind::Float {
            None
        } else if let (Some(left), Some(right)) = (previous[0][id], previous[1][id]) {
            Some(pack(b, left, right))
        } else if let Some(value) = common.get(&(node.op, node.args.clone())) {
            Some(*value)
        } else {
            let value = if node.op == 19 {
                let left = b.ins().fcvt_from_uint(
                    types::F64,
                    integers[0][node.args[0]].expect("integer cast prefix"),
                );
                let right = b.ins().fcvt_from_uint(
                    types::F64,
                    integers[1][node.args[0]].expect("integer cast prefix"),
                );
                pack(b, left, right)
            } else {
                let args: Vec<_> = node
                    .args
                    .iter()
                    .map(|&arg| floats[arg].expect("typed vector float dependency"))
                    .collect();
                arithmetic(b, node.op, &args, types::F64X2, None)
            };
            common.insert((node.op, node.args.clone()), value);
            Some(value)
        };
        floats.push(value);
    }
    floats[function.output].expect("matrix coefficient has f64 output")
}

pub(super) fn emit<M: Module>(
    module: &mut M,
    graph: &Program,
    node_index: usize,
    name: &str,
    simd: bool,
    callee: Option<FuncId>,
) -> Result<FuncId, String> {
    let rows = std::env::var("MECH_NUMERIC_ROWS")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .map_err(|_| "invalid numeric rows")?;
    if ![1, 2, 4].contains(&rows) {
        return Err("numeric rows must be 1,2,4".into());
    }
    emit_with_rows(module, graph, node_index, name, simd, callee, rows, false)
}

/// Emit conservative unsigned intervals over the coefficient DAG, once per
/// kernel invocation. This is native eligibility code, not runtime evaluation
/// of the coefficient graph. Failed proofs select checked arithmetic.
fn integer_ranges(b: &mut FunctionBuilder, function: &Callable, maxima: [Value; 2]) -> Value {
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let mut valid = b.ins().iconst(types::I8, 1);
    let mut ranges: Vec<Option<(Value, Value)>> = function
        .program
        .constants
        .iter()
        .map(|c| match c {
            Constant::Integer(n) => {
                let n = b.ins().iconst(types::I64, *n as i64);
                Some((n, n))
            }
            Constant::Parameter(i, Kind::Integer) => Some((zero, maxima[*i])),
            _ => None,
        })
        .collect();
    for (index, node) in function.program.nodes.iter().enumerate() {
        if function.program.kinds[function.program.constants.len() + index] != Kind::Integer {
            ranges.push(None);
            continue;
        }
        let (lo, hi) = ranges[node.args[0]].expect("integer interval");
        let (rlo, rhi) = ranges[node.args[1]].expect("integer interval");
        let (low, high, ok) = match node.op {
            20 | 22 => {
                let (high, overflow) = if node.op == 20 {
                    b.ins().umul_overflow(hi, rhi)
                } else {
                    b.ins().uadd_overflow(hi, rhi)
                };
                let low = if node.op == 20 {
                    b.ins().imul(lo, rlo)
                } else {
                    b.ins().iadd(lo, rlo)
                };
                let ok = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
                (low, high, ok)
            }
            23 => {
                let ok = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, lo, rhi);
                (b.ins().isub(lo, rhi), b.ins().isub(hi, rlo), ok)
            }
            21 => {
                let ok = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, rlo, 0);
                let rlo = b.ins().select(ok, rlo, one);
                let nonzero = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, rhi, 0);
                let rhi = b.ins().select(nonzero, rhi, one);
                (b.ins().udiv(lo, rhi), b.ins().udiv(hi, rlo), ok)
            }
            _ => unreachable!("unsigned interval operation"),
        };
        valid = b.ins().band(valid, ok);
        ranges.push(Some((low, high)));
    }
    valid
}

/// Select maximal varying polynomial multiplication cones (degree <= 2).
/// Divisions, casts and floating arithmetic terminate polynomial eligibility.
fn induction_nodes(function: &Callable, varying: usize) -> Vec<usize> {
    let graph = &function.program;
    let base = graph.constants.len();
    let mut degrees: Vec<Option<usize>> = graph
        .constants
        .iter()
        .map(|c| match c {
            Constant::Integer(_) => Some(0),
            Constant::Parameter(i, Kind::Integer) => Some(usize::from(*i == varying)),
            _ => None,
        })
        .collect();
    for node in &graph.nodes {
        let degree = if matches!(node.op, 20 | 22 | 23) {
            degrees[node.args[0]]
                .zip(degrees[node.args[1]])
                .and_then(|(a, c)| {
                    let degree = if node.op == 20 { a + c } else { a.max(c) };
                    (degree <= 2).then_some(degree)
                })
        } else {
            None
        };
        degrees.push(degree);
    }
    fn visit(
        id: usize,
        base: usize,
        graph: &Program,
        degrees: &[Option<usize>],
        selected: &mut std::collections::BTreeSet<usize>,
    ) {
        if id < base {
            return;
        }
        let node = &graph.nodes[id - base];
        if node.op == 20 && degrees[id].is_some_and(|d| d > 0) {
            selected.insert(id);
        } else {
            for &input in &node.args {
                visit(input, base, graph, degrees, selected);
            }
        }
    }
    let mut selected = std::collections::BTreeSet::new();
    visit(function.output, base, graph, &degrees, &mut selected);
    selected.into_iter().collect()
}

/// Expand a selected cone in the ring of u64 values. Its coefficients depend
/// only on the fixed row. Auxiliary differences may wrap; the range guard
/// proves that every observable source operation remains checked and valid.
fn polynomial_coefficients(
    b: &mut FunctionBuilder,
    graph: &Program,
    id: usize,
    varying: usize,
    row: Value,
    cache: &mut BTreeMap<usize, [Value; 3]>,
) -> [Value; 3] {
    if let Some(value) = cache.get(&id) {
        return *value;
    }
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let result = if id < graph.constants.len() {
        match graph.constants[id] {
            Constant::Integer(n) => [b.ins().iconst(types::I64, n as i64), zero, zero],
            Constant::Parameter(i, Kind::Integer) if i == varying => [zero, one, zero],
            Constant::Parameter(_, Kind::Integer) => [row, zero, zero],
            _ => unreachable!("polynomial constant"),
        }
    } else {
        let node = &graph.nodes[id - graph.constants.len()];
        let a = polynomial_coefficients(b, graph, node.args[0], varying, row, cache);
        let c = polynomial_coefficients(b, graph, node.args[1], varying, row, cache);
        let mut out = [zero; 3];
        for power in 0..3 {
            out[power] = match node.op {
                22 => b.ins().iadd(a[power], c[power]),
                23 => b.ins().isub(a[power], c[power]),
                20 => {
                    let mut value = zero;
                    for i in 0..=power {
                        let product = b.ins().imul(a[i], c[power - i]);
                        value = b.ins().iadd(value, product);
                    }
                    value
                }
                _ => unreachable!("polynomial arithmetic"),
            };
        }
        out
    };
    cache.insert(id, result);
    result
}

fn emit_with_rows<M: Module>(
    module: &mut M,
    graph: &Program,
    node_index: usize,
    name: &str,
    simd: bool,
    callee: Option<FuncId>,
    rows: usize,
    checked: bool,
) -> Result<FuncId, String> {
    let node = &graph.nodes[node_index];
    let ptr = module.target_config().pointer_type();
    let mut ctx = module.make_context();
    let mut fbctx = FunctionBuilderContext::new();
    match node.op {
        73 => {
            let Some(Constant::Function(function)) = graph.constants.get(node.args[2]) else {
                return Err("missing matrix domain coefficient".into());
            };
            ctx.func.signature.params = vec![AbiParam::new(types::I64); 2];
            ctx.func.signature.returns.push(AbiParam::new(types::I32));
            let id = module
                .declare_function(name, Linkage::Local, &ctx.func.signature)
                .map_err(|e| e.to_string())?;
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            let success = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            if function.program.kinds[function.parameters[0]] == Kind::Integer {
                let dims = b.block_params(entry).to_vec();
                let nonempty = b.create_block();
                let a = b.ins().icmp_imm(IntCC::NotEqual, dims[0], 0);
                let c = b.ins().icmp_imm(IntCC::NotEqual, dims[1], 0);
                let both = b.ins().band(a, c);
                b.ins().brif(both, nonempty, &[], success, &[]);
                b.switch_to_block(nonempty);
                let maxima = [b.ins().iadd_imm(dims[0], -1), b.ins().iadd_imm(dims[1], -1)];
                let valid = integer_ranges(&mut b, function, maxima);
                let row = b.create_block();
                let row_body = b.create_block();
                let column = b.create_block();
                let column_body = b.create_block();
                let row_next = b.create_block();
                let error = b.create_block();
                b.append_block_param(error, types::I32);
                b.append_block_param(row, types::I64);
                b.append_block_param(column, types::I64);
                let zero = b.ins().iconst(types::I64, 0);
                b.ins().brif(valid, success, &[], row, &[zero.into()]);
                b.switch_to_block(row);
                let r = b.block_params(row)[0];
                let more = b.ins().icmp(IntCC::UnsignedLessThan, r, dims[0]);
                b.ins().brif(more, row_body, &[], success, &[]);
                b.switch_to_block(row_body);
                b.ins().jump(column, &[zero.into()]);
                b.switch_to_block(column);
                let c = b.block_params(column)[0];
                let more = b.ins().icmp(IntCC::UnsignedLessThan, c, dims[1]);
                b.ins().brif(more, column_body, &[], row_next, &[]);
                b.switch_to_block(column_body);
                scalar_graph(&mut b, function, &[r, c], None, false, 1, Some(error));
                let next = b.ins().iadd_imm(c, 1);
                b.ins().jump(column, &[next.into()]);
                b.switch_to_block(row_next);
                let next = b.ins().iadd_imm(r, 1);
                b.ins().jump(row, &[next.into()]);
                b.switch_to_block(error);
                let status = b.block_params(error)[0];
                b.ins().return_(&[status]);
            } else {
                b.ins().jump(success, &[]);
            }
            b.switch_to_block(success);
            let status = b.ins().iconst(types::I32, 0);
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            Ok(id)
        }
        35 | 36 => {
            let Some(Constant::Function(function)) = graph.constants.get(node.args[2]) else {
                return Err("missing matrix function".into());
            };
            let divide = match graph.constants.get(node.args[3]) {
                Some(Constant::Text(mode)) if mode == "multiply" => false,
                Some(Constant::Text(mode)) if mode == "divide" => true,
                _ => return Err("matrix weight mode must be constant multiply or divide".into()),
            };
            let integer = function.program.kinds[function.parameters[0]] == Kind::Integer;
            let varying = if node.op == 36 { 0 } else { 1 };
            let induction = if integer
                && !checked
                && std::env::var("MECH_NUMERIC_INDUCTION").as_deref() != Ok("0")
            {
                induction_nodes(function, varying)
            } else {
                Vec::new()
            };
            let slow = if integer && !checked {
                Some(emit_with_rows(
                    module,
                    graph,
                    node_index,
                    &format!("{name}_checked"),
                    simd,
                    None,
                    rows,
                    true,
                )?)
            } else {
                None
            };
            let fallback = if rows > 1 {
                Some(emit_with_rows(
                    module,
                    graph,
                    node_index,
                    &format!("{name}_tail"),
                    simd,
                    None,
                    1,
                    checked,
                )?)
            } else {
                None
            };
            // (input, columns, first_row, end_row, output_for_first_row)
            ctx.func.signature.params = vec![AbiParam::new(ptr); 5];
            ctx.func.signature.returns.push(AbiParam::new(types::I32));
            let id = module
                .declare_function(name, Linkage::Local, &ctx.func.signature)
                .map_err(|e| e.to_string())?;
            let fallback = fallback.map(|id| module.declare_func_in_func(id, &mut ctx.func));
            let slow = slow.map(|id| module.declare_func_in_func(id, &mut ctx.func));
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            let row = b.create_block();
            let loop_ = b.create_block();
            let tail = b.create_block();
            let tail_body = b.create_block();
            let finish = b.create_block();
            let exit = b.create_block();
            let error = b.create_block();
            b.append_block_param(error, types::I32);
            b.append_block_params_for_function_params(entry);
            b.append_block_param(row, ptr);
            let lanes = if simd { 2 } else { 1 };
            // Independent SIMD accumulators expose ILP without changing workload.
            let unroll: usize = if simd {
                std::env::var("MECH_NUMERIC_UNROLL")
                    .unwrap_or_else(|_| "2".into())
                    .parse()
                    .map_err(|_| "invalid numeric unroll")?
            } else {
                1
            };
            if ![1, 2, 4, 8].contains(&unroll) {
                return Err("numeric unroll must be 1,2,4,8".into());
            }
            let ty = if simd { types::F64X2 } else { types::F64 };
            b.append_block_param(loop_, ptr);
            for _ in 0..unroll * rows {
                b.append_block_param(loop_, ty);
            }
            for _ in 0..rows * induction.len() * 2 {
                b.append_block_param(loop_, types::I64);
            }
            b.append_block_param(tail, ptr);
            b.append_block_param(tail_body, ptr);
            for _ in 0..rows {
                b.append_block_param(tail, types::F64);
                b.append_block_param(tail_body, types::F64);
                b.append_block_param(finish, types::F64);
            }
            b.switch_to_block(entry);
            let p = b.block_params(entry).to_vec();
            // Hoisted checked arithmetic must not run for an empty reduction.
            // Write every zero result without evaluating the coefficient graph.
            if integer {
                let empty = b.create_block();
                let empty_body = b.create_block();
                let nonempty = b.create_block();
                b.append_block_param(empty, ptr);
                let no_columns = b.ins().icmp_imm(IntCC::Equal, p[1], 0);
                b.ins()
                    .brif(no_columns, empty, &[p[2].into()], nonempty, &[]);
                b.switch_to_block(empty);
                let r = b.block_params(empty)[0];
                let more = b.ins().icmp(IntCC::UnsignedLessThan, r, p[3]);
                b.ins().brif(more, empty_body, &[], exit, &[]);
                b.switch_to_block(empty_body);
                let offset = b.ins().isub(r, p[2]);
                let offset = b.ins().ishl_imm(offset, 3);
                let address = b.ins().iadd(p[4], offset);
                let zero = b.ins().f64const(0.0);
                b.ins().store(MemFlags::new(), zero, address, 0);
                let next = b.ins().iadd_imm(r, 1);
                b.ins().jump(empty, &[next.into()]);
                b.switch_to_block(nonempty);
            }
            if let Some(slow) = slow {
                let zero = b.ins().iconst(ptr, 0);
                let mut maxima = Vec::new();
                for length in [p[3], p[1]] {
                    let nonempty = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, length, 0);
                    let max = b.ins().iadd_imm(length, -1);
                    let max = b.ins().select(nonempty, max, zero);
                    maxima.push(if ptr == types::I64 {
                        max
                    } else {
                        b.ins().uextend(types::I64, max)
                    });
                }
                if node.op == 36 {
                    maxima.swap(0, 1);
                }
                let proof = integer_ranges(&mut b, function, [maxima[0], maxima[1]]);
                let fast = b.create_block();
                let slow_block = b.create_block();
                b.ins().brif(proof, fast, &[], slow_block, &[]);
                b.switch_to_block(slow_block);
                let call = b.ins().call(slow, &p);
                let status = b.inst_results(call)[0];
                b.ins().return_(&[status]);
                b.switch_to_block(fast);
            }
            let more = b.ins().icmp(IntCC::UnsignedLessThan, p[2], p[3]);
            b.ins().brif(more, row, &[p[2].into()], exit, &[]);
            b.switch_to_block(row);
            let r = b.block_params(row)[0];
            if let Some(fallback) = fallback {
                let end = b.ins().iadd_imm(r, rows as i64);
                let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, p[3]);
                let full = b.create_block();
                let partial = b.create_block();
                b.ins().brif(fits, full, &[], partial, &[]);
                b.switch_to_block(partial);
                let offset = b.ins().isub(r, p[2]);
                let offset = b.ins().ishl_imm(offset, 3);
                let out = b.ins().iadd(p[4], offset);
                let call = b.ins().call(fallback, &[p[0], p[1], r, p[3], out]);
                let status = b.inst_results(call)[0];
                b.ins().return_(&[status]);
                b.switch_to_block(full);
            }
            let zero = b.ins().f64const(0.0);
            let z = b.ins().iconst(ptr, 0);
            let mut row_scalars = Vec::new();
            let mut row_inputs = Vec::new();
            let mut row_invariants = Vec::new();
            for offset in 0..rows {
                let ri = b.ins().iadd_imm(r, offset as i64);
                let rf = if integer {
                    if ptr == types::I64 {
                        ri
                    } else {
                        b.ins().uextend(types::I64, ri)
                    }
                } else {
                    b.ins().fcvt_from_uint(types::F64, ri)
                };
                let input = if simd && !integer {
                    b.ins().splat(ty, rf)
                } else {
                    rf
                };
                let placeholder = if integer {
                    b.ins().iconst(types::I64, 0)
                } else {
                    zero
                };
                let inputs = if varying == 0 {
                    [placeholder, input]
                } else {
                    [input, placeholder]
                };
                let (invariants, _) = scalar_graph(
                    &mut b,
                    function,
                    &inputs,
                    None,
                    true,
                    varying,
                    if checked { Some(error) } else { None },
                );
                row_scalars.push(rf);
                row_inputs.push(input);
                row_invariants.push(invariants);
            }
            let vector_zero = if simd { b.ins().splat(ty, zero) } else { zero };
            let mut init = vec![z.into()];
            init.extend(
                (0..unroll * rows).map(|_| cranelift_codegen::ir::BlockArg::from(vector_zero)),
            );
            let mut seconds = Vec::new();
            for &row in &row_scalars {
                let mut cache = BTreeMap::new();
                for &id in &induction {
                    let [constant, linear, quadratic] = polynomial_coefficients(
                        &mut b,
                        &function.program,
                        id,
                        varying,
                        row,
                        &mut cache,
                    );
                    let first = b.ins().iadd(linear, quadratic);
                    let second = b.ins().ishl_imm(quadratic, 1);
                    init.push(constant.into());
                    init.push(first.into());
                    seconds.push(second);
                }
            }
            b.ins().jump(loop_, &init);
            b.switch_to_block(loop_);
            let args = b.block_params(loop_).to_vec();
            let j = args[0];
            let end = b.ins().iadd_imm(j, (lanes * unroll) as i64);
            let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, p[1]);
            let body = b.create_block();
            let reduce = b.create_block();
            b.ins().brif(fits, body, &[], reduce, &[]);
            b.switch_to_block(body);
            let mut next = vec![end.into()];
            for k in 0..unroll {
                let jk = b.ins().iadd_imm(j, (k * lanes) as i64);
                let jf = b.ins().fcvt_from_uint(types::F64, jk);
                let column = if simd {
                    let one = b.ins().f64const(1.0);
                    let second = b.ins().fadd(jf, one);
                    let v = b.ins().splat(ty, jf);
                    b.ins().insertlane(v, second, 1)
                } else {
                    jf
                };
                let off = b.ins().ishl_imm(jk, 3);
                let addr = b.ins().iadd(p[0], off);
                let input = b.ins().load(ty, MemFlags::new(), addr, 0);
                for row_index in 0..rows {
                    let row_input = row_inputs[row_index];
                    let coefficient = if integer {
                        let mut lane_inputs = Vec::new();
                        let mut lane_previous = Vec::new();
                        for lane in 0..lanes {
                            let column = b.ins().iadd_imm(jk, lane as i64);
                            let column = if ptr == types::I64 {
                                column
                            } else {
                                b.ins().uextend(types::I64, column)
                            };
                            let inputs = if varying == 0 {
                                [column, row_input]
                            } else {
                                [row_input, column]
                            };
                            let mut previous = row_invariants[row_index].clone();
                            let offset = (k * lanes + lane) as i64;
                            for (index, &id) in induction.iter().enumerate() {
                                let state = row_index * induction.len() + index;
                                let start = 1 + unroll * rows + state * 2;
                                let linear = b.ins().imul_imm(args[start + 1], offset);
                                let quadratic =
                                    b.ins().imul_imm(seconds[state], offset * (offset - 1) / 2);
                                let delta = b.ins().iadd(linear, quadratic);
                                previous[id] = Some(b.ins().iadd(args[start], delta));
                            }
                            lane_inputs.push(inputs);
                            lane_previous.push(previous);
                        }
                        if simd {
                            scalar_graph_integer_simd(
                                &mut b,
                                function,
                                &lane_inputs.try_into().expect("two SIMD lanes"),
                                &lane_previous.try_into().expect("two SIMD lanes"),
                                if checked { Some(error) } else { None },
                            )
                        } else {
                            let (_, coefficient) = scalar_graph(
                                &mut b,
                                function,
                                &lane_inputs[0],
                                Some(&lane_previous[0]),
                                false,
                                varying,
                                if checked { Some(error) } else { None },
                            );
                            coefficient.unwrap()
                        }
                    } else {
                        let inputs = if varying == 0 {
                            [column, row_input]
                        } else {
                            [row_input, column]
                        };
                        let (_, coefficient) = scalar_graph(
                            &mut b,
                            function,
                            &inputs,
                            Some(&row_invariants[row_index]),
                            false,
                            varying,
                            None,
                        );
                        coefficient.unwrap()
                    };
                    let term = if divide {
                        b.ins().fdiv(input, coefficient)
                    } else {
                        b.ins().fmul(input, coefficient)
                    };
                    let index = 1 + k * rows + row_index;
                    let sum = b.ins().fadd(args[index], term);
                    next.push(sum.into());
                }
            }
            let step = (lanes * unroll) as i64;
            for (state, &second) in seconds.iter().enumerate() {
                let start = 1 + unroll * rows + state * 2;
                let linear = b.ins().imul_imm(args[start + 1], step);
                let quadratic = b.ins().imul_imm(second, step * (step - 1) / 2);
                let delta = b.ins().iadd(linear, quadratic);
                let value = b.ins().iadd(args[start], delta);
                let increment = b.ins().imul_imm(second, step);
                let first = b.ins().iadd(args[start + 1], increment);
                next.push(value.into());
                next.push(first.into());
            }
            b.ins().jump(loop_, &next);
            b.switch_to_block(reduce);
            let mut sums = Vec::new();
            for row_index in 0..rows {
                let mut sum = args[1 + row_index];
                for k in 1..unroll {
                    sum = b.ins().fadd(sum, args[1 + k * rows + row_index]);
                }
                if simd {
                    let a = b.ins().extractlane(sum, 0);
                    let c = b.ins().extractlane(sum, 1);
                    sum = b.ins().fadd(a, c);
                }
                sums.push(sum);
            }
            let mut tail_args = vec![j.into()];
            tail_args.extend(
                sums.iter()
                    .copied()
                    .map(cranelift_codegen::ir::BlockArg::from),
            );
            b.ins().jump(tail, &tail_args);
            b.switch_to_block(tail);
            let t = b.block_params(tail).to_vec();
            let more = b.ins().icmp(IntCC::UnsignedLessThan, t[0], p[1]);
            let body_args: Vec<_> = t
                .iter()
                .copied()
                .map(cranelift_codegen::ir::BlockArg::from)
                .collect();
            let end_args: Vec<_> = t[1..]
                .iter()
                .copied()
                .map(cranelift_codegen::ir::BlockArg::from)
                .collect();
            b.ins().brif(more, tail_body, &body_args, finish, &end_args);
            b.switch_to_block(tail_body);
            let t = b.block_params(tail_body).to_vec();
            let jf = if integer {
                if ptr == types::I64 {
                    t[0]
                } else {
                    b.ins().uextend(types::I64, t[0])
                }
            } else {
                b.ins().fcvt_from_uint(types::F64, t[0])
            };
            let off = b.ins().ishl_imm(t[0], 3);
            let addr = b.ins().iadd(p[0], off);
            let v = b.ins().load(types::F64, MemFlags::new(), addr, 0);
            let j = b.ins().iadd_imm(t[0], 1);
            let mut tail_args = vec![j.into()];
            for row_index in 0..rows {
                let rf = row_scalars[row_index];
                let inputs = if varying == 0 { [jf, rf] } else { [rf, jf] };
                let (_, coefficient) = scalar_graph(
                    &mut b,
                    function,
                    &inputs,
                    None,
                    false,
                    varying,
                    if checked { Some(error) } else { None },
                );
                let term = if divide {
                    b.ins().fdiv(v, coefficient.unwrap())
                } else {
                    b.ins().fmul(v, coefficient.unwrap())
                };
                let sum = b.ins().fadd(t[1 + row_index], term);
                tail_args.push(sum.into());
            }
            b.ins().jump(tail, &tail_args);
            b.switch_to_block(finish);
            let sums = b.block_params(finish).to_vec();
            let offset = b.ins().isub(r, p[2]);
            let offset = b.ins().ishl_imm(offset, 3);
            let addr = b.ins().iadd(p[4], offset);
            for (row_index, &sum) in sums.iter().enumerate() {
                b.ins()
                    .store(MemFlags::new(), sum, addr, (row_index * 8) as i32);
            }
            let next = b.ins().iadd_imm(r, rows as i64);
            let more = b.ins().icmp(IntCC::UnsignedLessThan, next, p[3]);
            b.ins().brif(more, row, &[next.into()], exit, &[]);
            b.switch_to_block(exit);
            let success = b.ins().iconst(types::I32, 0);
            b.ins().return_(&[success]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            Ok(id)
        }
        37 => {
            ctx.func.signature.params = vec![AbiParam::new(ptr); 3];
            ctx.func.signature.returns.push(AbiParam::new(types::F64));
            let id = module
                .declare_function(name, Linkage::Local, &ctx.func.signature)
                .map_err(|e| e.to_string())?;
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            let check = b.create_block();
            let body = b.create_block();
            let exit = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(check, ptr);
            b.append_block_param(check, types::F64);
            b.switch_to_block(entry);
            let p = b.block_params(entry).to_vec();
            let z = b.ins().iconst(ptr, 0);
            let sum = b.ins().f64const(0.0);
            b.ins().jump(check, &[z.into(), sum.into()]);
            b.switch_to_block(check);
            let a = b.block_params(check).to_vec();
            let more = b.ins().icmp(IntCC::UnsignedLessThan, a[0], p[2]);
            b.ins().brif(more, body, &[], exit, &[]);
            b.switch_to_block(body);
            let off = b.ins().ishl_imm(a[0], 3);
            let left = b.ins().iadd(p[0], off);
            let right = b.ins().iadd(p[1], off);
            let left = b.ins().load(types::F64, MemFlags::new(), left, 0);
            let right = b.ins().load(types::F64, MemFlags::new(), right, 0);
            let product = b.ins().fmul(left, right);
            let sum = b.ins().fadd(a[1], product);
            let next = b.ins().iadd_imm(a[0], 1);
            b.ins().jump(check, &[next.into(), sum.into()]);
            b.switch_to_block(exit);
            b.ins().return_(&[a[1]]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            Ok(id)
        }
        38 => {
            ctx.func.signature.params.push(AbiParam::new(types::F64));
            ctx.func.signature.returns.push(AbiParam::new(types::F64));
            let id = module
                .declare_function(name, Linkage::Local, &ctx.func.signature)
                .map_err(|e| e.to_string())?;
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let x = b.block_params(entry)[0];
            let y = b.ins().sqrt(x);
            b.ins().return_(&[y]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            Ok(id)
        }
        39 => {
            ctx.func.signature.params = vec![AbiParam::new(ptr); 2];
            let id = module
                .declare_function(name, Linkage::Local, &ctx.func.signature)
                .map_err(|e| e.to_string())?;
            let mut sig = module.make_signature();
            sig.params = vec![AbiParam::new(ptr); 2];
            sig.returns.push(AbiParam::new(ptr));
            let step = module
                .declare_function("mech_sequence_invoke", Linkage::Import, &sig)
                .map_err(|e| e.to_string())?;
            let step = module.declare_func_in_func(step, &mut ctx.func);
            let callee = module.declare_func_in_func(
                callee.ok_or("missing callable native entry")?,
                &mut ctx.func,
            );
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            let check = b.create_block();
            let body = b.create_block();
            let again = b.create_block();
            let exit = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(check, ptr);
            b.switch_to_block(entry);
            let p = b.block_params(entry).to_vec();
            let z = b.ins().iconst(ptr, 0);
            let fnptr = b.ins().func_addr(ptr, callee);
            b.ins().jump(check, &[z.into()]);
            b.switch_to_block(check);
            let i = b.block_params(check)[0];
            let more = b.ins().icmp(IntCC::UnsignedLessThan, i, p[1]);
            b.ins().brif(more, body, &[], exit, &[]);
            b.switch_to_block(body);
            let call = b.ins().call(step, &[p[0], fnptr]);
            let status = b.inst_results(call)[0];
            let good = b.ins().icmp_imm(IntCC::Equal, status, 0);
            b.ins().brif(good, again, &[], exit, &[]);
            b.switch_to_block(again);
            let next = b.ins().iadd_imm(i, 1);
            b.ins().jump(check, &[next.into()]);
            b.switch_to_block(exit);
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            Ok(id)
        }
        _ => Err("unsupported native numeric loop".into()),
    }
}
