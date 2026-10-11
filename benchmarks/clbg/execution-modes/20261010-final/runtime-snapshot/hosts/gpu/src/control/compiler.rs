use cranelift_codegen::{
    ir::{
        AbiParam, Block, ConstantData, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Type,
        Value,
        condcodes::{FloatCC, IntCC},
        immediates::Ieee64,
        types,
    },
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_engine::native_sequence::control::{Expr, Kind, Op, Program};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
type Values = Vec<Value>;
/// First integer whose correctly rounded division is >= (or >) a threshold.
/// Conversion is exact through 2^53 and division by a finite positive constant
/// is monotone. Search the actual IEEE operation rather than multiplying the
/// threshold: multiplication would move some rounded comparison boundaries.
fn rounded_division_cutoff(
    maximum: u64,
    divisor: f64,
    threshold: f64,
    strict: bool,
) -> Option<u64> {
    if maximum > (1 << 53) || !divisor.is_finite() || divisor <= 0.0 || threshold.is_nan() {
        return None;
    }
    let (mut first, mut last) = (0, maximum + 1);
    while first < last {
        let middle = first + (last - first) / 2;
        let quotient = middle as f64 / divisor;
        let matches = if strict {
            quotient > threshold
        } else {
            quotient >= threshold
        };
        if matches {
            last = middle;
        } else {
            first = middle + 1;
        }
    }
    Some(first)
}
/// An exact reciprocal for every unsigned dividend in 0..=maximum.
/// For m = ceil(2^s / d), let e = m*d - 2^s. If e*maximum < 2^s,
/// the approximation cannot cross the next quotient boundary, even for
/// remainder d-1. The product must also fit the emitted 64-bit multiply.
fn bounded_reciprocal(divisor: u64, maximum: u64) -> Option<(u64, u32)> {
    if divisor == 0 || maximum > u32::MAX as u64 || divisor > u32::MAX as u64 {
        return None;
    }
    for shift in 32..64 {
        let scale = 1u128 << shift;
        let multiplier = scale.div_ceil(divisor as u128);
        if multiplier > u32::MAX as u128 {
            return None;
        }
        let error = multiplier * divisor as u128 - scale;
        if error * (maximum as u128) < scale && multiplier * (maximum as u128) <= u64::MAX as u128 {
            return Some((multiplier as u64, shift));
        }
    }
    None
}
#[derive(Clone)]
struct FloatCount {
    comparison: FloatCC,
    left: Value,
    right: Value,
    scalar: Value,
}
#[derive(Clone)]
struct IntegerCount {
    comparison: IntCC,
    left: Value,
    right: Value,
    scalar: Value,
}
#[derive(Clone, Default)]
struct CountSum {
    terms: Vec<FloatCount>,
    integers: Vec<IntegerCount>,
    other: Values,
}
fn types_for(k: &Kind) -> Vec<Type> {
    match k {
        Kind::Integer | Kind::Reference | Kind::BigInteger => vec![types::I64],
        Kind::Float => vec![types::F64],
        Kind::Boolean => vec![types::I8],
        Kind::Bytes(n) => vec![types::I8X16; n.div_ceil(16)],
        Kind::ReadBytes | Kind::Floats => vec![types::I64; 2],
        Kind::Tuple(k) => k.iter().flat_map(types_for).collect(),
        Kind::Record(fields) => fields.iter().flat_map(|(_, k)| types_for(k)).collect(),
        Kind::Type(_) => vec![],
        Kind::Text => vec![],
    }
}
// Conservative value domains: a proof over source operations, never input names.
fn abstract_bounds(e: &Expr, params: &[Option<u64>], program: &Program) -> Vec<Option<u64>> {
    let count = types_for(&e.kind).len();
    if program.native_functions {
        return vec![None; count];
    }
    match &e.op {
        Op::Integer(n) => vec![Some(*n)],
        Op::Float(_) => vec![None],
        Op::ReadBytes(bytes) => vec![None, Some(bytes.len() as u64)],
        Op::Floats(bits) => vec![None, Some(bits.len() as u64)],
        Op::Boolean(b) => vec![Some(u64::from(*b))],
        Op::Text(_) | Op::Type => vec![],
        Op::Parameter(i) => params[*i..*i + count].to_vec(),
        Op::Tuple(a) => a
            .iter()
            .flat_map(|e| abstract_bounds(e, params, program))
            .collect(),
        Op::Field(e, i) => {
            let k = field_kinds(&e.kind);
            let first: usize = k[..*i].iter().map(|k| types_for(k).len()).sum();
            let values = abstract_bounds(e, params, program);
            values[first..first + types_for(&k[*i]).len()].to_vec()
        }
        Op::Call(name, a) => {
            let values = a
                .iter()
                .flat_map(|e| abstract_bounds(e, params, program))
                .collect::<Vec<_>>();
            abstract_bounds(&program.functions[name].output, &values, program)
        }
        Op::Loop { initial, step, .. } => loop_bounds(
            abstract_bounds(initial, params, program),
            step,
            &e.kind,
            program,
        ),
        Op::Branch { yes, no, state, .. } => {
            let state = abstract_bounds(state, params, program);
            let a = abstract_bounds(&program.functions[yes].output, &state, program);
            let b = abstract_bounds(&program.functions[no].output, &state, program);
            a.iter()
                .zip(b)
                .map(|(a, b)| a.zip(b).map(|(a, b)| a.max(b)))
                .collect()
        }
        Op::Invoke { .. }
        | Op::Local(_)
        | Op::Reduce { .. }
        | Op::Match { .. }
        | Op::Heap { .. }
        | Op::Arena { .. }
        | Op::Print { .. }
        | Op::Sequence { .. }
        | Op::Parallel { .. } => vec![None; count],
        Op::Write { .. } => vec![Some(0)],
        Op::Scan {
            state, function, ..
        } => scan_bounds(abstract_bounds(state, params, program), function, program),
        Op::Primitive(name, a) => {
            let bounds = a
                .iter()
                .map(|e| abstract_bounds(e, params, program))
                .collect::<Vec<_>>();
            let x = bounds.first().and_then(|v| v.first()).copied().flatten();
            let y = bounds.get(1).and_then(|v| v.first()).copied().flatten();
            match name.as_str() {
                "sequence/iota-bytes" => {
                    let Kind::Bytes(n) = e.kind else {
                        unreachable!()
                    };
                    vec![Some((n - 1) as u64); count]
                }
                "sequence/zero-bytes" => vec![Some(0); count],
                "sequence/reverse-prefix" | "sequence/rotate-prefix" => bounds[0].clone(),
                "sequence/set" => {
                    let max = bounds[0]
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(|b| b.into_iter().max().unwrap());
                    let value = bounds[2][0].unwrap_or(255).min(255);
                    vec![Some(max.unwrap_or(255).max(value)); count]
                }
                "sequence/length" => vec![bounds[0][1]],
                "sequence/get" if a[0].kind == Kind::Floats => vec![None],
                "sequence/get" if a[0].kind == Kind::ReadBytes => vec![Some(255)],
                "sequence/get" => vec![Some(
                    bounds[0]
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(|b| b.into_iter().max().unwrap())
                        .unwrap_or(255),
                )],
                "+" => vec![x.zip(y).and_then(|(a, b)| a.checked_add(b))],
                "*" => vec![x.zip(y).and_then(|(a, b)| a.checked_mul(b))],
                "number/mod" => vec![y.and_then(|v| v.checked_sub(1))],
                "number/max" => vec![x.zip(y).map(|(a, b)| a.max(b))],
                "number/bounded" => vec![bounds[2][0]],
                "control/select" => bounds[1]
                    .iter()
                    .zip(&bounds[2])
                    .map(|(a, b)| a.zip(*b).map(|(a, b)| a.max(b)))
                    .collect(),
                "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||" => vec![Some(1)],
                _ => vec![None; count],
            }
        }
    }
}
// Join initial and successor states until the domain is an inductive invariant.
// Growth that does not converge loses its bounds rather than guessing a trip count.
fn scan_bounds(mut bounds: Vec<Option<u64>>, step: &str, program: &Program) -> Vec<Option<u64>> {
    for _ in 0..8 {
        let next = abstract_bounds(&program.functions[step].output, &bounds, program);
        let joined = bounds
            .iter()
            .zip(next)
            .map(|(a, b)| a.zip(b).map(|(a, b)| a.max(b)))
            .collect::<Vec<_>>();
        if joined == bounds {
            return bounds;
        }
        bounds = joined;
    }
    vec![None; bounds.len()]
}
fn scan_domains(e: &Expr, program: &Program, domains: &mut BTreeMap<String, Vec<Option<u64>>>) {
    match &e.op {
        Op::Scan {
            state,
            function,
            count,
            width,
            buffer,
            ..
        } => {
            let bounds = scan_bounds(abstract_bounds(state, &[], program), function, program);
            domains
                .entry(function.clone())
                .and_modify(|old| {
                    for (a, b) in old.iter_mut().zip(&bounds) {
                        *a = a.zip(*b).map(|(a, b)| a.max(b));
                    }
                })
                .or_insert(bounds);
            for child in [&**state, &**count, &**width, &**buffer] {
                scan_domains(child, program, domains);
            }
        }
        Op::Tuple(items) | Op::Call(_, items) | Op::Primitive(_, items) => {
            for child in items {
                scan_domains(child, program, domains);
            }
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            scan_domains(scrutinee, program, domains);
            for (condition, body) in arms {
                scan_domains(condition, program, domains);
                scan_domains(body, program, domains);
            }
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            for e in [start, step, end, body] {
                scan_domains(e, program, domains);
            }
        }
        Op::Field(child, _) => scan_domains(child, program, domains),
        _ => {}
    }
}
fn nested_loop(
    e: &Expr,
    program: &Program,
    visited: &mut std::collections::BTreeSet<String>,
) -> bool {
    match &e.op {
        Op::Loop { .. } | Op::Reduce { .. } => true,
        Op::Call(name, items) => {
            items.iter().any(|e| nested_loop(e, program, visited))
                || (visited.insert(name.clone())
                    && nested_loop(&program.functions[name].output, program, visited))
        }
        Op::Tuple(items) | Op::Primitive(_, items) => {
            items.iter().any(|e| nested_loop(e, program, visited))
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            nested_loop(scrutinee, program, visited)
                || arms.iter().any(|(condition, body)| {
                    nested_loop(condition, program, visited) || nested_loop(body, program, visited)
                })
        }
        Op::Field(e, _) => nested_loop(e, program, visited),
        Op::Branch {
            condition,
            state,
            yes,
            no,
            ..
        } => {
            nested_loop(condition, program, visited)
                || nested_loop(state, program, visited)
                || [yes, no].iter().any(|name| {
                    visited.insert((*name).clone())
                        && nested_loop(&program.functions[*name].output, program, visited)
                })
        }
        _ => false,
    }
}
/// Positive-index proof uses an operation's domains, including the no-wrap
/// proof for addition. A bound on a wrapped result alone is insufficient.
fn positive(e: &Expr, params: &[Option<u64>], program: &Program) -> bool {
    match &e.op {
        Op::Integer(n) => *n > 0,
        Op::Primitive(name, _) if name == "number/factorial" => true,
        Op::Primitive(name, args) if name == "number/bounded" => {
            matches!(args[1].op, Op::Integer(n) if n > 0)
        }
        Op::Primitive(name, args) if name == "+" => {
            let left = abstract_bounds(&args[0], params, program)[0];
            let right = abstract_bounds(&args[1], params, program)[0];
            left.zip(right)
                .and_then(|(a, b)| a.checked_add(b))
                .is_some()
                && (positive(&args[0], params, program) || positive(&args[1], params, program))
        }
        _ => false,
    }
}
fn loop_bounds(
    mut bounds: Vec<Option<u64>>,
    step: &str,
    kind: &Kind,
    program: &Program,
) -> Vec<Option<u64>> {
    let types = types_for(kind);
    // Widen every increasing byte domain directly to its checked type limit and
    // every increasing integer domain to unknown. One further transfer establishes
    // the invariant; no speculative loop-trip-count assumption is made.
    for _ in 0..3 {
        let next = abstract_bounds(&program.functions[step].output, &bounds, program);
        let mut changed = false;
        for i in 0..bounds.len() {
            if let Some(old) = bounds[i] {
                if next[i].is_none_or(|new| new > old) {
                    bounds[i] = if types[i] == types::I8X16 {
                        Some(255)
                    } else {
                        None
                    };
                    changed = true;
                }
            }
        }
        if !changed {
            return bounds;
        }
    }
    vec![None; bounds.len()]
}
/// Source fields returned unchanged by a loop step remain the initial SSA value.
/// Keeping that value also retains literal-table and exact-conversion facts.
fn identity_fields(e: &Expr) -> Vec<Option<usize>> {
    match &e.op {
        Op::Parameter(i) => (0..types_for(&e.kind).len()).map(|j| Some(i + j)).collect(),
        Op::Tuple(items) => items.iter().flat_map(identity_fields).collect(),
        Op::Field(value, i) => {
            let kinds = field_kinds(&value.kind);
            let offset: usize = kinds[..*i].iter().map(|k| types_for(k).len()).sum();
            identity_fields(value)[offset..offset + types_for(&kinds[*i]).len()].to_vec()
        }
        _ => vec![None; types_for(&e.kind).len()],
    }
}
/// Prove a byte scan is a cyclic traversal of a literal table. The runtime can
/// replay a source-generated cycle without evaluating the step for every byte.
/// No function names, alphabets or benchmark-specific arithmetic are involved.
fn scan_period(function: &mech_engine::native_sequence::control::Function) -> Option<u64> {
    if function.inputs != [Kind::Integer] {
        return None;
    }
    let Op::Tuple(items) = &function.output.op else {
        return None;
    };
    if items.len() != 2 || items.iter().any(|e| e.kind != Kind::Integer) {
        return None;
    }
    let Op::Primitive(name, next) = &items[0].op else {
        return None;
    };
    if name != "number/mod" || next.len() != 2 {
        return None;
    }
    let Op::Primitive(name, byte) = &items[1].op else {
        return None;
    };
    if name != "sequence/get" || byte.len() != 2 || next[0] != byte[1] {
        return None;
    }
    let Op::ReadBytes(table) = &byte[0].op else {
        return None;
    };
    let period = table.len() as u64;
    if period == 0 || period > 1048576 {
        return None;
    }
    let Op::Primitive(name, index) = &next[0].op else {
        return None;
    };
    if name != "+"
        || index.len() != 2
        || index[0].op != Op::Parameter(0)
        || index[1].op != Op::Integer(1)
    {
        return None;
    }
    let modulus_matches = match &next[1].op {
        Op::Integer(n) => *n == period,
        Op::Primitive(name, args) => {
            name == "sequence/length" && args.len() == 1 && args[0] == byte[0]
        }
        _ => false,
    };
    modulus_matches.then_some(period)
}
/// Replace a scan's next-state expression with a buffered sample. An old-state
/// parameter outside that expression rejects splitting: byte mapping must be a
/// closed, pure function of the next state. Local loop/call parameters stay local.
fn scan_sample_expression(e: &Expr, next: &Expr) -> Option<Expr> {
    if e == next {
        return Some(Expr {
            kind: Kind::Integer,
            op: Op::Parameter(0),
        });
    }
    let children = |items: &[Expr]| {
        items
            .iter()
            .map(|e| scan_sample_expression(e, next))
            .collect::<Option<Vec<_>>>()
    };
    let op = match &e.op {
        Op::Parameter(_) => return None,
        Op::Tuple(items) => Op::Tuple(children(items)?),
        Op::Primitive(name, items) => Op::Primitive(name.clone(), children(items)?),
        Op::Call(name, items) => Op::Call(name.clone(), children(items)?),
        Op::Field(value, i) => Op::Field(Box::new(scan_sample_expression(value, next)?), *i),
        Op::Loop {
            id,
            initial,
            predicate,
            step,
        } => Op::Loop {
            id: *id,
            initial: Box::new(scan_sample_expression(initial, next)?),
            predicate: predicate.clone(),
            step: step.clone(),
        },
        Op::Branch {
            id,
            condition,
            yes,
            no,
            state,
        } => Op::Branch {
            id: *id,
            condition: Box::new(scan_sample_expression(condition, next)?),
            yes: yes.clone(),
            no: no.clone(),
            state: Box::new(scan_sample_expression(state, next)?),
        },
        Op::Integer(_)
        | Op::Float(_)
        | Op::Boolean(_)
        | Op::Text(_)
        | Op::ReadBytes(_)
        | Op::Floats(_) => e.op.clone(),
        _ => return None,
    };
    Some(Expr {
        kind: e.kind.clone(),
        op,
    })
}
/// A conservative profitability hint: leave trivial mappings in the original
/// scan. Structural/domain proofs above determine correctness independently.
fn scan_mapping_work(
    e: &Expr,
    program: &Program,
    active: &mut std::collections::BTreeSet<String>,
) -> usize {
    let mut call = |name: &String| {
        if !active.insert(name.clone()) {
            return 0;
        }
        let work = scan_mapping_work(&program.functions[name].output, program, active);
        active.remove(name);
        work
    };
    match &e.op {
        Op::Call(name, args) => {
            let work = call(name);
            1 + work
                + args
                    .iter()
                    .map(|e| scan_mapping_work(e, program, active))
                    .sum::<usize>()
        }
        Op::Loop {
            initial,
            predicate,
            step,
            ..
        } => {
            let work = call(predicate) + call(step);
            4 + work + scan_mapping_work(initial, program, active)
        }
        Op::Branch {
            condition,
            yes,
            no,
            state,
            ..
        } => {
            let work = call(yes) + call(no);
            1 + work
                + scan_mapping_work(condition, program, active)
                + scan_mapping_work(state, program, active)
        }
        Op::Primitive(_, args) | Op::Tuple(args) => {
            1 + args
                .iter()
                .map(|e| scan_mapping_work(e, program, active))
                .sum::<usize>()
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            1 + scan_mapping_work(scrutinee, program, active)
                + arms
                    .iter()
                    .map(|(condition, body)| {
                        scan_mapping_work(condition, program, active)
                            + scan_mapping_work(body, program, active)
                    })
                    .sum::<usize>()
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            4 + [start, step, end, body]
                .into_iter()
                .map(|e| scan_mapping_work(e, program, active))
                .sum::<usize>()
        }
        Op::Field(e, _) => 1 + scan_mapping_work(e, program, active),
        _ => 0,
    }
}
struct Emit<'a, 'b, 'c, M: Module> {
    b: &'a mut FunctionBuilder<'b>,
    module: &'a mut M,
    program: &'c Program,
    simd: bool,
    narrow_uint: bool,
    narrow_arithmetic: bool,
    stack_index: bool,
    callbacks: &'c BTreeMap<String, FuncId>,
    functions: &'c BTreeMap<String, FuncId>,
    trusted_functions: &'c BTreeMap<String, FuncId>,
    function_layouts: &'c BTreeMap<String, Kind>,
    layouts: &'c BTreeMap<Kind, u64>,
    context_facts: ContextFacts,
    error_block: Option<Block>,
    depth: Value,
    arena: Option<FuncId>,
    arena_callbacks: &'c BTreeMap<(String, Kind), FuncId>,
    arena_context: Value,
    active_inline: BTreeMap<String, usize>,
    recursive_inline: usize,
    print: Option<FuncId>,
    bigint: Option<FuncId>,
    reduce: FuncId,
    scan: Option<FuncId>,
    periodic_scan: Option<FuncId>,
    pipeline_scan: Option<FuncId>,
    write: Option<FuncId>,
    scan_callbacks: &'c BTreeMap<String, FuncId>,
    pipeline_callbacks: &'c BTreeMap<String, (FuncId, FuncId)>,
    arguments: Value,
    tables: &'a mut BTreeMap<String, DataId>,
    facts: BTreeMap<Value, u64>,
    constants: BTreeMap<Value, u64>,
    float_constants: BTreeMap<Value, u64>,
    float_tables: BTreeMap<Value, Vec<u64>>,
    float_conditions: BTreeMap<Value, (FloatCC, Value, Value)>,
    count_sums: BTreeMap<Value, CountSum>,
    uint_floats: BTreeMap<Value, Value>,
    uint_divisions: BTreeMap<Value, (Value, f64)>,
    exact_float_compare: bool,
    indexed_float_tables: BTreeMap<Value, (Value, Vec<u64>)>,
    integer_conditions: BTreeMap<Value, (IntCC, Value, Value)>,
}
impl<M: Module> Emit<'_, '_, '_, M> {
    fn exact_division_compare(&mut self, cc: FloatCC, left: Value, right: Value) -> Option<Value> {
        let (integer, divisor) = *self.uint_divisions.get(&left)?;
        let maximum = *self.facts.get(&integer)?;
        if maximum > (1 << 53) {
            return None;
        }
        let strict = matches!(cc, FloatCC::LessThanOrEqual | FloatCC::GreaterThan);
        let icc = match cc {
            FloatCC::LessThan | FloatCC::LessThanOrEqual => IntCC::UnsignedLessThan,
            FloatCC::GreaterThan | FloatCC::GreaterThanOrEqual => IntCC::UnsignedGreaterThanOrEqual,
            _ => return None,
        };
        let boundary = if let Some(&bits) = self.float_constants.get(&right) {
            let cutoff = rounded_division_cutoff(maximum, divisor, f64::from_bits(bits), strict)?;
            if cutoff == 0 || cutoff > maximum {
                let greater = cutoff == 0;
                let result = if icc == IntCC::UnsignedLessThan {
                    !greater
                } else {
                    greater
                };
                return Some(self.b.ins().iconst(types::I8, i64::from(result)));
            }
            self.int(cutoff)
        } else {
            let (index, bits) = self.indexed_float_tables.get(&right)?.clone();
            let cutoffs = bits
                .iter()
                .map(|&bits| {
                    rounded_division_cutoff(maximum, divisor, f64::from_bits(bits), strict)
                })
                .collect::<Option<Vec<_>>>()?;
            let ptr = self
                .literal_table(cutoffs.iter().flat_map(|n| n.to_le_bytes()).collect())
                .ok()?;
            // The original readonly load already checked this one-based index.
            let offset = self.b.ins().iadd_imm(index, -1);
            let offset = self.b.ins().ishl_imm(offset, 3);
            let address = self.b.ins().iadd(ptr, offset);
            let boundary = self.b.ins().load(types::I64, MemFlags::new(), address, 0);
            self.facts.insert(boundary, maximum + 1);
            boundary
        };
        let result = self.b.ins().icmp(icc, integer, boundary);
        if [integer, boundary]
            .iter()
            .all(|v| self.facts.get(v).is_some_and(|&n| n <= u32::MAX as u64))
        {
            self.integer_conditions
                .insert(result, (icc, integer, boundary));
        }
        Some(result)
    }
    fn integer_arithmetic(&mut self, name: &str, a: Value, b: Value) -> Value {
        let bound = self
            .facts
            .get(&a)
            .zip(self.facts.get(&b))
            .and_then(|(&a, &b)| {
                if name == "+" {
                    a.checked_add(b)
                } else {
                    a.checked_mul(b)
                }
            });
        if self.narrow_arithmetic && bound.is_some_and(|max| max <= u32::MAX as u64) {
            let a = self.b.ins().ireduce(types::I32, a);
            let b = self.b.ins().ireduce(types::I32, b);
            let result = if name == "+" {
                self.b.ins().iadd(a, b)
            } else {
                self.b.ins().imul(a, b)
            };
            self.b.ins().uextend(types::I64, result)
        } else if name == "+" {
            self.b.ins().iadd(a, b)
        } else {
            self.b.ins().imul(a, b)
        }
    }
    fn divide(&mut self, name: &str, a: Value, b: Value) -> Value {
        if let Some((&maximum, &divisor)) = self.facts.get(&a).zip(self.constants.get(&b)) {
            if divisor != 0 && (maximum as u128) < 2 * divisor as u128 {
                // A bounded cyclic index needs at most one subtraction. This
                // also preserves wrapping source arithmetic before division.
                if maximum < divisor {
                    return if name == "/" { self.int(0) } else { a };
                }
                let wraps = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, a, b);
                return if name == "/" {
                    self.b.ins().uextend(types::I64, wraps)
                } else {
                    let reduced = self.b.ins().isub(a, b);
                    self.b.ins().select(wraps, reduced, a)
                };
            }
        }
        if self.narrow_uint {
            if let Some((&maximum, &divisor)) = self.facts.get(&a).zip(self.constants.get(&b)) {
                if let Some((multiplier, shift)) = bounded_reciprocal(divisor, maximum) {
                    // Keep the proven 32-bit dividend in its existing 64-bit
                    // register. Cranelift's I32 umulhi expands into extensions
                    // and two shifts; a full product needs just one shift.
                    let product = self.b.ins().imul_imm(a, multiplier as i64);
                    let quotient = self.b.ins().ushr_imm(product, shift as i64);
                    return if name == "/" {
                        quotient
                    } else {
                        let multiple = self.b.ins().imul(quotient, b);
                        self.b.ins().isub(a, multiple)
                    };
                }
            }
        }
        let narrow = self.narrow_arithmetic
            && [a, b]
                .iter()
                .all(|v| self.facts.get(v).is_some_and(|&max| max <= u32::MAX as u64));
        let (a, b) = if narrow {
            (
                self.b.ins().ireduce(types::I32, a),
                self.b.ins().ireduce(types::I32, b),
            )
        } else {
            (a, b)
        };
        let result = if name == "/" {
            self.b.ins().udiv(a, b)
        } else {
            self.b.ins().urem(a, b)
        };
        if narrow {
            self.b.ins().uextend(types::I64, result)
        } else {
            result
        }
    }
    /// Pack independent Boolean counts, preserving each IEEE comparison and
    /// ordinary wrapping integer addition. No floating arithmetic is reassociated.
    fn add_counts(&mut self, left: Value, right: Value) -> Value {
        let mut sum = CountSum::default();
        for value in [left, right] {
            if let Some(part) = self.count_sums.get(&value) {
                sum.terms.extend(part.terms.clone());
                sum.integers.extend(part.integers.clone());
                sum.other.extend(&part.other);
            } else {
                sum.other.push(value);
            }
        }
        let count = sum.terms.len() + sum.integers.len();
        let value = if self.simd && count <= 64 && (sum.terms.len() >= 4 || sum.integers.len() >= 2)
        {
            let one = self.int(1);
            let ones = self.b.ins().splat(types::I64X2, one);
            let zero = self.int(0);
            let mut packed_sum = self.b.ins().splat(types::I64X2, zero);
            let mut scalar_sum = self.int(0);
            for pair in sum.terms.chunks(2) {
                if pair.len() == 2 && pair[0].comparison == pair[1].comparison {
                    let left = self.pair_floats(pair[0].left, pair[1].left);
                    let right = self.pair_floats(pair[0].right, pair[1].right);
                    let masks = self.b.ins().fcmp(pair[0].comparison, left, right);
                    let counts = self.b.ins().band(masks, ones);
                    packed_sum = self.b.ins().iadd(packed_sum, counts);
                } else {
                    for term in pair {
                        scalar_sum = self.b.ins().iadd(scalar_sum, term.scalar);
                    }
                }
            }
            let lo = self.b.ins().extractlane(packed_sum, 0);
            let hi = self.b.ins().extractlane(packed_sum, 1);
            let packed = self.b.ins().iadd(lo, hi);
            let mut result = self.b.ins().iadd(packed, scalar_sum);
            let zero32 = self.b.ins().iconst(types::I32, 0);
            let mut masks_sum = self.b.ins().splat(types::I32X4, zero32);
            for chunk in sum.integers.chunks(4) {
                if chunk
                    .iter()
                    .all(|term| term.comparison == chunk[0].comparison)
                {
                    let left =
                        self.quad_integers(&chunk.iter().map(|t| t.left).collect::<Vec<_>>());
                    let right =
                        self.quad_integers(&chunk.iter().map(|t| t.right).collect::<Vec<_>>());
                    let mut masks = self.b.ins().icmp(chunk[0].comparison, left, right);
                    if chunk.len() < 4 {
                        let bytes = (0..4)
                            .flat_map(|i| {
                                (if i < chunk.len() { u32::MAX } else { 0 }).to_le_bytes()
                            })
                            .collect::<Vec<_>>();
                        let data = self.b.func.dfg.constants.insert(ConstantData::from(bytes));
                        let mask = self.b.ins().vconst(types::I32X4, data);
                        masks = self.b.ins().band(masks, mask);
                    }
                    masks_sum = self.b.ins().iadd(masks_sum, masks);
                } else {
                    for term in chunk {
                        result = self.b.ins().iadd(result, term.scalar);
                    }
                }
            }
            let pairs = self.b.ins().iadd_pairwise(masks_sum, masks_sum);
            let total = self.b.ins().iadd_pairwise(pairs, pairs);
            let negative_count = self.b.ins().extractlane(total, 0);
            let integer_count = self.b.ins().ineg(negative_count);
            let integer_count = self.b.ins().uextend(types::I64, integer_count);
            result = self.b.ins().iadd(result, integer_count);
            for other in &sum.other {
                result = self.b.ins().iadd(result, *other);
            }
            result
        } else {
            self.integer_arithmetic("+", left, right)
        };
        if count != 0 && count <= 64 {
            self.count_sums.insert(value, sum);
        }
        value
    }
    fn quad_integers(&mut self, values: &[Value]) -> Value {
        let mut numbers = Vec::new();
        for value in values {
            if let Some(&n) = self.constants.get(value) {
                numbers.push(n as u32);
            } else {
                break;
            }
        }
        if numbers.len() == values.len() {
            numbers.resize(4, 0);
            let data = self.b.func.dfg.constants.insert(ConstantData::from(
                numbers
                    .iter()
                    .flat_map(|n| n.to_le_bytes())
                    .collect::<Vec<_>>(),
            ));
            return self.b.ins().vconst(types::I32X4, data);
        }
        let first = self.b.ins().ireduce(types::I32, values[0]);
        let mut packed = self.b.ins().splat(types::I32X4, first);
        for (lane, &value) in values.iter().enumerate().skip(1) {
            if value != values[0] {
                let value = self.b.ins().ireduce(types::I32, value);
                packed = self.b.ins().insertlane(packed, value, lane as u8);
            }
        }
        packed
    }
    fn pair_floats(&mut self, first: Value, second: Value) -> Value {
        if let Some((&a, &b)) = self
            .float_constants
            .get(&first)
            .zip(self.float_constants.get(&second))
        {
            let bytes = a
                .to_le_bytes()
                .into_iter()
                .chain(b.to_le_bytes())
                .collect::<Vec<_>>();
            let constant = self.b.func.dfg.constants.insert(ConstantData::from(bytes));
            return self.b.ins().vconst(types::F64X2, constant);
        }
        let vector = self.b.ins().splat(types::F64X2, first);
        if first == second {
            vector
        } else {
            self.b.ins().insertlane(vector, second, 1)
        }
    }
    fn literal_table(&mut self, data: Vec<u8>) -> Result<Value, String> {
        let key = format!(
            "mech_control_literal_{}",
            data.iter().map(|v| format!("{v:02x}")).collect::<String>()
        );
        self.table(key, data)
    }
    fn encoded_scalar(&mut self, value: Value) -> Value {
        match self.b.func.dfg.value_type(value) {
            types::F64 => self.b.ins().bitcast(types::I64, MemFlags::new(), value),
            types::I8 => self.b.ins().uextend(types::I64, value),
            _ => value,
        }
    }
    fn decoded_scalar(&mut self, value: Value, kind: Type) -> Value {
        match kind {
            types::F64 => self.b.ins().bitcast(types::F64, MemFlags::new(), value),
            types::I8 => self.b.ins().ireduce(types::I8, value),
            _ => value,
        }
    }
    fn int(&mut self, n: u64) -> Value {
        let v = self.b.ins().iconst(types::I64, n as i64);
        self.facts.insert(v, n);
        self.constants.insert(v, n);
        v
    }
    fn error_target(&mut self) -> Block {
        if let Some(block) = self.error_block {
            return block;
        }
        let block = self.b.create_block();
        self.b.append_block_param(block, types::I64);
        self.b.set_cold_block(block);
        self.error_block = Some(block);
        block
    }
    fn guard(&mut self, valid: Value, status: i64) {
        if self.program.native_functions {
            let good = self.b.create_block();
            let bad = self.error_target();
            let error = self.b.ins().iconst(types::I64, status);
            self.b.ins().brif(valid, good, &[], bad, &[error.into()]);
            self.b.switch_to_block(good);
            return;
        }

        let good = self.b.create_block();
        let bad = self.b.create_block();
        self.b.ins().brif(valid, good, &[], bad, &[]);
        self.b.switch_to_block(bad);
        let error = self.b.ins().iconst(types::I64, status);
        self.failure(error);
        self.b.switch_to_block(good);
    }
    fn vector(&mut self, data: [u8; 16]) -> Value {
        let constant = self
            .b
            .func
            .dfg
            .constants
            .insert(ConstantData::from(data.to_vec()));
        self.b.ins().vconst(types::I8X16, constant)
    }
    fn slot(&mut self, n: usize) -> Value {
        let s = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            n as u32,
            4,
        ));
        self.b.ins().stack_addr(types::I64, s, 0)
    }
    fn failure(&mut self, status: Value) {
        let kinds: Vec<_> = self
            .b
            .func
            .signature
            .returns
            .iter()
            .skip(1)
            .map(|p| p.value_type)
            .collect();
        let mut values = vec![status];
        for kind in kinds {
            values.push(if kind == types::F64 {
                self.b.ins().f64const(0.0)
            } else {
                self.b.ins().iconst(kind, 0)
            });
        }
        self.b.ins().return_(&values);
    }
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Values, String> {
        let f = self
            .program
            .functions
            .get(name)
            .ok_or("missing native function")?;
        let active = self.active_inline.get(name).copied().unwrap_or(0);
        if self.functions.contains_key(name)
            && (active == 0 || active <= self.recursive_inline)
            && std::env::var("MECH_CONTROL_INLINE").as_deref() != Ok("0")
        {
            let original_depth = self.depth;
            if active > 0 {
                self.depth = self.b.ins().iadd_imm(original_depth, 1);
                let valid = self
                    .b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThan, self.depth, 128);
                self.guard(valid, 8);
            }
            self.active_inline.insert(name.to_owned(), active + 1);
            let result = self.expression(&f.output, params, &mut BTreeMap::new());
            if active == 0 {
                self.active_inline.remove(name);
            } else {
                self.active_inline.insert(name.to_owned(), active);
            }
            self.depth = original_depth;
            return result;
        }
        if let Some(&normal) = self.functions.get(name) {
            let id = if self.context_facts.valid
                && self.context_facts.layout.as_ref() == self.function_layouts.get(name)
            {
                self.trusted_functions.get(name).copied().unwrap_or(normal)
            } else {
                normal
            };
            let kinds = types_for(&f.output.kind);
            let register = register_result(&f.output.kind);
            let output = if register {
                None
            } else {
                Some(self.slot((kinds.len() * 16).max(16)))
            };
            let depth = self.b.ins().iadd_imm(self.depth, 1);
            let mut args = vec![depth, self.arena_context];
            args.extend_from_slice(params);
            if let Some(output) = output {
                args.push(output);
            }
            let callee = self.module.declare_func_in_func(id, self.b.func);
            let call = self.b.ins().call(callee, &args);
            let results = self.b.inst_results(call).to_vec();
            self.status(results[0]);
            if register {
                Ok(vec![results[1]])
            } else {
                Ok(kinds
                    .into_iter()
                    .enumerate()
                    .map(|(i, kind)| {
                        self.b
                            .ins()
                            .load(kind, MemFlags::new(), output.unwrap(), (i * 16) as i32)
                    })
                    .collect())
            }
        } else {
            self.expression(&f.output, params, &mut BTreeMap::new())
        }
    }
    fn status(&mut self, status: Value) {
        if self.program.native_functions {
            let good = self.b.create_block();
            let bad = self.error_target();
            let valid = self.b.ins().icmp_imm(IntCC::Equal, status, 0);
            self.b.ins().brif(valid, good, &[], bad, &[status.into()]);
            self.b.switch_to_block(good);
            return;
        }

        let good = self.b.create_block();
        let bad = self.b.create_block();
        let valid = self.b.ins().icmp_imm(IntCC::Equal, status, 0);
        self.b.ins().brif(valid, good, &[], bad, &[]);
        self.b.switch_to_block(bad);
        self.failure(status);
        self.b.switch_to_block(good);
    }
    fn table(&mut self, name: String, data: Vec<u8>) -> Result<Value, String> {
        let id = if let Some(id) = self.tables.get(&name) {
            *id
        } else {
            let id = self
                .module
                .declare_data(&name, Linkage::Local, false, false)
                .map_err(|e| e.to_string())?;
            let mut desc = DataDescription::new();
            desc.define(data.into_boxed_slice());
            desc.set_align(16);
            self.module
                .define_data(id, &desc)
                .map_err(|e| e.to_string())?;
            self.tables.insert(name, id);
            id
        };
        let gv = self.module.declare_data_in_func(id, self.b.func);
        Ok(self.b.ins().global_value(types::I64, gv))
    }
    fn check_context(&mut self) {
        if !self.context_facts.valid {
            let valid = self
                .b
                .ins()
                .icmp_imm(IntCC::NotEqual, self.arena_context, 0);
            self.guard(valid, 9);
            self.context_facts.valid = true;
        }
    }
    fn check_layout(&mut self, layout: &Kind) {
        self.check_context();
        if self.context_facts.layout.as_ref() != Some(layout) {
            let actual = self.b.ins().load(
                types::I64,
                MemFlags::new(),
                self.arena_context,
                super::RefArenaContext::LAYOUT,
            );
            let valid = self
                .b
                .ins()
                .icmp_imm(IntCC::Equal, actual, self.layouts[layout] as i64);
            self.guard(valid, 9);
            self.context_facts.layout = Some(layout.clone());
        }
    }
    fn arena_base(&mut self) -> Value {
        if let Some(value) = self.context_facts.base {
            return value;
        }
        let value = self.b.ins().load(
            types::I64,
            MemFlags::new(),
            self.arena_context,
            super::RefArenaContext::BASE,
        );
        self.context_facts.base = Some(value);
        value
    }
    fn arena_capacity(&mut self) -> Value {
        if let Some(value) = self.context_facts.capacity {
            return value;
        }
        let value = self.b.ins().load(
            types::I64,
            MemFlags::new(),
            self.arena_context,
            super::RefArenaContext::CAPACITY,
        );
        self.context_facts.capacity = Some(value);
        value
    }
    fn expression(
        &mut self,
        e: &Expr,
        params: &[Value],
        memo: &mut BTreeMap<Expr, Values>,
    ) -> Result<Values, String> {
        if let Some(value) = memo.get(e) {
            return Ok(value.clone());
        }
        let out = match &e.op {
            Op::Integer(n) => vec![self.int(*n)],
            Op::Float(bits) => {
                let value = self.b.ins().f64const(Ieee64::with_bits(*bits));
                self.float_constants.insert(value, *bits);
                vec![value]
            }
            Op::ReadBytes(data) => vec![
                self.literal_table(data.clone())?,
                self.int(data.len() as u64),
            ],
            Op::Floats(bits) => {
                let ptr =
                    self.literal_table(bits.iter().flat_map(|v| v.to_le_bytes()).collect())?;
                self.float_tables.insert(ptr, bits.clone());
                vec![ptr, self.int(bits.len() as u64)]
            }
            Op::Boolean(b) => vec![self.b.ins().iconst(types::I8, i64::from(*b))],
            Op::Text(_) | Op::Type => vec![],
            Op::Parameter(i) => params[*i..*i + types_for(&e.kind).len()].to_vec(),
            Op::Local(_) => return Err("unbound native comprehension local".into()),
            Op::Tuple(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.extend(self.expression(item, params, memo)?);
                }
                out
            }
            Op::Field(value, i) => {
                let kinds = field_kinds(&value.kind);
                let offset: usize = kinds[..*i].iter().map(|k| types_for(k).len()).sum();
                let values = self.expression(value, params, memo)?;
                values[offset..offset + types_for(&kinds[*i]).len()].to_vec()
            }
            Op::Sequence { effects, output } => {
                for effect in effects {
                    self.expression(effect, params, memo)?;
                }
                self.expression(output, params, memo)?
            }
            Op::Heap {
                action,
                arguments,
                arity,
                layout,
                ..
            } => {
                if (10..=18).contains(action) {
                    let mut values = vec![self.int(*action)];
                    for arg in arguments {
                        values.extend(self.expression(arg, params, memo)?);
                    }
                    while values.len() < 4 {
                        values.push(self.int(0));
                    }
                    let output = self.slot(8);
                    values.push(output);
                    let callee = self.module.declare_func_in_func(
                        self.bigint.ok_or("missing big-integer ABI")?,
                        self.b.func,
                    );
                    let call = self.b.ins().call(callee, &values);
                    let status = self.b.inst_results(call)[0];
                    self.status(status);
                    let result = self.b.ins().load(types::I64, MemFlags::new(), output, 0);
                    vec![self.decoded_scalar(result, types_for(&e.kind)[0])]
                } else if *action == 0 {
                    vec![self.int(0)]
                } else {
                    let mut values = Vec::new();
                    for arg in arguments {
                        values.extend(self.expression(arg, params, memo)?);
                    }
                    self.check_layout(layout.as_ref().ok_or("record layout")?);
                    let base = self.arena_base();
                    let cursor = self.b.ins().load(
                        types::I64,
                        MemFlags::new(),
                        self.arena_context,
                        super::RefArenaContext::CURSOR,
                    );
                    if *action == 1 {
                        let capacity = self.arena_capacity();
                        let valid = self.b.ins().icmp(IntCC::UnsignedLessThan, cursor, capacity);
                        self.guard(valid, 9);
                        // Every Ref originates from null, an allocation, or a
                        // checked read in this live region. Scalar-only scope
                        // boundaries prevent cross-region and retired references.
                        let offset = self.b.ins().imul_imm(cursor, (*arity * 8) as i64);
                        let address = self.b.ins().iadd(base, offset);
                        for (i, value) in values.iter().enumerate() {
                            let value = self.encoded_scalar(*value);
                            self.b
                                .ins()
                                .store(MemFlags::new(), value, address, (i * 8) as i32);
                        }
                        let handle = self.b.ins().iadd_imm(cursor, 1);
                        self.b.ins().store(
                            MemFlags::new(),
                            handle,
                            self.arena_context,
                            super::RefArenaContext::CURSOR,
                        );
                        vec![handle]
                    } else if *action == 2 {
                        let handle = values[0];
                        let index = self.b.ins().iadd_imm(handle, -1);
                        let valid = self.b.ins().icmp(IntCC::UnsignedLessThan, index, cursor);
                        self.guard(valid, 9);
                        let offset = self.b.ins().imul_imm(index, (*arity * 8) as i64);
                        let address = self.b.ins().iadd(base, offset);
                        types_for(&e.kind)
                            .into_iter()
                            .enumerate()
                            .map(|(i, kind)| {
                                let bits = self.b.ins().load(
                                    types::I64,
                                    MemFlags::new(),
                                    address,
                                    (i * 8) as i32,
                                );
                                self.decoded_scalar(bits, kind)
                            })
                            .collect()
                    } else {
                        return Err("unsupported reference record action".into());
                    }
                }
            }
            Op::Arena {
                capacity,
                arity,
                layout,
                function,
                arguments,
                ..
            } => {
                let capacity_value = if let Some(capacity) = capacity {
                    Some(self.expression(capacity, params, memo)?[0])
                } else {
                    None
                };
                let args = self.expression(arguments, params, memo)?;
                if let Some(capacity) = capacity_value {
                    let input = self.slot((args.len() * 8).max(16));
                    for (i, value) in args.iter().enumerate() {
                        let value = self.encoded_scalar(*value);
                        self.b
                            .ins()
                            .store(MemFlags::new(), value, input, (i * 8) as i32);
                    }
                    let output = self.slot((types_for(&e.kind).len() * 8).max(16));
                    let layout_id = self.int(self.layouts[layout]);
                    let arity = self.int(*arity as u64);
                    let count = self.int(args.len() as u64);
                    let callback = self.module.declare_func_in_func(
                        self.arena_callbacks[&(function.clone(), layout.clone())],
                        self.b.func,
                    );
                    let address = self.b.ins().func_addr(types::I64, callback);
                    let callee = self
                        .module
                        .declare_func_in_func(self.arena.ok_or("missing arena ABI")?, self.b.func);
                    let call = self.b.ins().call(
                        callee,
                        &[capacity, arity, layout_id, address, input, count, output],
                    );
                    let status = self.b.inst_results(call)[0];
                    self.status(status);
                    types_for(&e.kind)
                        .iter()
                        .enumerate()
                        .map(|(i, kind)| {
                            self.b
                                .ins()
                                .load(*kind, MemFlags::new(), output, (i * 8) as i32)
                        })
                        .collect()
                } else {
                    self.check_context();
                    let mark = self.b.ins().load(
                        types::I64,
                        MemFlags::new(),
                        self.arena_context,
                        super::RefArenaContext::CURSOR,
                    );
                    let values = self.call(function, &args)?;
                    self.b.ins().store(
                        MemFlags::new(),
                        mark,
                        self.arena_context,
                        super::RefArenaContext::CURSOR,
                    );
                    values
                }
            }
            Op::Print { format, values, .. } => {
                let ptr = self.literal_table(format.as_bytes().to_vec())?;
                let length = self.int(format.len() as u64);
                let fields = self.int(values.len() as u64);
                let input = self.slot((values.len() * 8).max(16));
                for (i, value) in values.iter().enumerate() {
                    let value = self.expression(value, params, memo)?[0];
                    self.b
                        .ins()
                        .store(MemFlags::new(), value, input, (i * 8) as i32);
                }
                let callee = self
                    .module
                    .declare_func_in_func(self.print.ok_or("missing print ABI")?, self.b.func);
                let call = self.b.ins().call(callee, &[ptr, length, input, fields]);
                let status = self.b.inst_results(call)[0];
                self.status(status);
                vec![self.int(0)]
            }
            Op::Invoke {
                name,
                arguments: args,
                ..
            }
            | Op::Call(name, args) => {
                let mut values = Vec::new();
                for arg in args {
                    values.extend(self.expression(arg, params, memo)?);
                }
                self.call(name, &values)?
            }
            Op::Loop {
                initial,
                predicate,
                step,
                ..
            } => {
                let initial_values = self.expression(initial, params, memo)?;
                let loop_context = self.context_facts.clone();
                let head = self.b.create_block();
                let body = self.b.create_block();
                let exit = self.b.create_block();
                for t in types_for(&e.kind) {
                    self.b.append_block_param(head, t);
                    self.b.append_block_param(exit, t);
                }
                self.b.ins().jump(
                    head,
                    &initial_values
                        .iter()
                        .copied()
                        .map(Into::into)
                        .collect::<Vec<_>>(),
                );
                self.b.switch_to_block(head);
                let invariant = identity_fields(&self.program.functions[step].output);
                let mut state = self.b.block_params(head).to_vec();
                for (i, value) in state.iter_mut().enumerate() {
                    if invariant[i] == Some(i) {
                        *value = initial_values[i];
                    }
                }
                let bounds = loop_bounds(
                    initial_values
                        .iter()
                        .map(|v| self.facts.get(v).copied())
                        .collect(),
                    step,
                    &e.kind,
                    self.program,
                );
                for (v, bound) in state.iter().zip(bounds) {
                    if let Some(bound) = bound {
                        self.facts.insert(*v, bound);
                    }
                }
                let condition = self.call(predicate, &state)?[0];
                self.b.ins().brif(
                    condition,
                    body,
                    &[],
                    exit,
                    &state.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                self.b.switch_to_block(body);
                let next = self.call(step, &state)?;
                self.b.ins().jump(
                    head,
                    &next.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                self.context_facts = loop_context;
                self.b.switch_to_block(exit);
                self.b
                    .block_params(exit)
                    .iter()
                    .enumerate()
                    .map(|(i, &value)| {
                        if invariant[i] == Some(i) {
                            initial_values[i]
                        } else {
                            value
                        }
                    })
                    .collect()
            }
            Op::Branch {
                condition,
                yes,
                no,
                state,
                ..
            } => {
                let condition = self.expression(condition, params, memo)?[0];
                let state = self.expression(state, params, memo)?;
                let branch_facts = self.facts.clone();
                let branch_context = self.context_facts.clone();
                let y = self.b.create_block();
                let n = self.b.create_block();
                let exit = self.b.create_block();
                for t in types_for(&e.kind) {
                    self.b.append_block_param(exit, t);
                }
                self.b.ins().brif(condition, y, &[], n, &[]);
                self.b.switch_to_block(y);
                let out = self.call(yes, &state)?;
                self.b.ins().jump(
                    exit,
                    &out.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                self.facts = branch_facts.clone();
                self.context_facts = branch_context.clone();
                self.b.switch_to_block(n);
                let out = self.call(no, &state)?;
                self.b.ins().jump(
                    exit,
                    &out.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                self.facts = branch_facts;
                self.context_facts = branch_context;
                self.b.switch_to_block(exit);
                self.b.block_params(exit).to_vec()
            }
            Op::Reduce {
                index,
                start,
                step,
                end,
                inclusive,
                body,
                ..
            } => {
                let start = self.expression(start, params, memo)?[0];
                let step = self.expression(step, params, memo)?[0];
                let end = self.expression(end, params, memo)?[0];
                let positive = self.b.ins().icmp_imm(IntCC::NotEqual, step, 0);
                self.guard(positive, 2);
                let before_memo = memo.clone();
                let before_facts = self.facts.clone();
                let before_context = self.context_facts.clone();
                let sum_type = types_for(&e.kind)[0];
                let zero = if sum_type == types::F64 {
                    self.b.ins().f64const(0.0)
                } else {
                    self.int(0)
                };
                let head = self.b.create_block();
                self.b.append_block_param(head, types::I64);
                self.b.append_block_param(head, sum_type);
                let advance = self.b.create_block();
                let exit = self.b.create_block();
                self.b.append_block_param(exit, sum_type);
                let comparison = if *inclusive {
                    IntCC::UnsignedLessThanOrEqual
                } else {
                    IntCC::UnsignedLessThan
                };
                let has_items = self.b.ins().icmp(comparison, start, end);
                self.b.ins().brif(
                    has_items,
                    head,
                    &[start.into(), zero.into()],
                    exit,
                    &[zero.into()],
                );
                self.b.switch_to_block(head);
                let current = self.b.block_params(head)[0];
                let previous = self.b.block_params(head)[1];
                let mut body_memo = before_memo.clone();
                body_memo.insert(
                    Expr {
                        kind: Kind::Integer,
                        op: Op::Local(*index),
                    },
                    vec![current],
                );
                let item = self.expression(body, params, &mut body_memo)?[0];
                let sum = if sum_type == types::F64 {
                    self.b.ins().fadd(previous, item)
                } else {
                    self.integer_arithmetic("+", previous, item)
                };
                // Compare remaining distance before incrementing: inclusive
                // ranges ending at u64::MAX terminate without wrapping.
                let remaining = self.b.ins().isub(end, current);
                let another = self.b.ins().icmp(comparison, step, remaining);
                self.b
                    .ins()
                    .brif(another, advance, &[], exit, &[sum.into()]);
                self.b.switch_to_block(advance);
                let next = self.b.ins().iadd(current, step);
                self.b.ins().jump(head, &[next.into(), sum.into()]);
                self.b.switch_to_block(exit);
                *memo = before_memo;
                self.facts = before_facts;
                self.context_facts = before_context;
                self.b.block_params(exit).to_vec()
            }
            Op::Match {
                scrutinee, arms, ..
            } => {
                // Evaluate once, even when the first arm is a wildcard.
                self.expression(scrutinee, params, memo)?;
                let before_memo = memo.clone();
                let before_facts = self.facts.clone();
                let before_context = self.context_facts.clone();
                let exit = self.b.create_block();
                let kinds = types_for(&e.kind);
                for &kind in &kinds {
                    self.b.append_block_param(exit, kind);
                }
                let mut exhaustive = false;
                for (condition, body) in arms {
                    // An irrefutable arm is the remaining path, not another
                    // runtime boolean test or an unmatched-call failure.
                    if matches!(condition.op, Op::Boolean(true)) {
                        let out = self.expression(body, params, memo)?;
                        self.b.ins().jump(
                            exit,
                            &out.iter().copied().map(Into::into).collect::<Vec<_>>(),
                        );
                        exhaustive = true;
                        break;
                    }
                    let condition = self.expression(condition, params, memo)?[0];
                    let matched = self.b.create_block();
                    let next = self.b.create_block();
                    self.b.ins().brif(condition, matched, &[], next, &[]);
                    self.b.switch_to_block(matched);
                    let out = self.expression(body, params, memo)?;
                    self.b.ins().jump(
                        exit,
                        &out.iter().copied().map(Into::into).collect::<Vec<_>>(),
                    );
                    self.b.switch_to_block(next);
                    *memo = before_memo.clone();
                    self.facts = before_facts.clone();
                    self.context_facts = before_context.clone();
                }
                if !exhaustive {
                    // Partial pattern functions retain checked-call failure.
                    let no_match = self.b.ins().iconst(types::I8, 0);
                    self.guard(no_match, 10);
                    let empty = kinds
                        .into_iter()
                        .map(|kind| {
                            if kind == types::F64 {
                                self.b.ins().f64const(0.0)
                            } else {
                                self.b.ins().iconst(kind, 0)
                            }
                        })
                        .collect::<Vec<_>>();
                    self.b
                        .ins()
                        .jump(exit, &empty.into_iter().map(Into::into).collect::<Vec<_>>());
                }
                self.b.switch_to_block(exit);
                *memo = before_memo;
                self.facts = before_facts;
                self.context_facts = before_context;
                self.b.block_params(exit).to_vec()
            }
            Op::Parallel {
                start,
                end,
                grain,
                function,
                arguments,
                reductions,
                ..
            } => {
                let first = self.expression(start, params, memo)?[0];
                let last = self.expression(end, params, memo)?[0];
                let grain = self.expression(grain, params, memo)?[0];
                let args = self.expression(arguments, params, memo)?;
                let argptr = self.slot(args.len() * 8);
                for (i, v) in args.iter().enumerate() {
                    self.b
                        .ins()
                        .store(MemFlags::new(), *v, argptr, (i * 8) as i32);
                }
                let arglen = self.int(args.len() as u64);
                let function = self
                    .module
                    .declare_func_in_func(self.callbacks[function], self.b.func);
                let function = self.b.ins().func_addr(types::I64, function);
                let codes = reductions
                    .iter()
                    .map(|r| if r == "sum" { 0u8 } else { 1u8 })
                    .collect::<Vec<_>>();
                let table = self.table(
                    format!("mech_control_reducers_{}", reductions.join("_")),
                    codes,
                )?;
                let len = self.int(reductions.len() as u64);
                let out = self.slot(reductions.len() * 8);
                let reduce = self.module.declare_func_in_func(self.reduce, self.b.func);
                let call = self.b.ins().call(
                    reduce,
                    &[
                        first, last, grain, argptr, arglen, function, table, len, out,
                    ],
                );
                let status = self.b.inst_results(call)[0];
                let valid = self.b.ins().icmp_imm(IntCC::Equal, status, 0);
                self.guard(valid, 5);
                (0..reductions.len())
                    .map(|i| {
                        self.b
                            .ins()
                            .load(types::I64, MemFlags::new(), out, (i * 8) as i32)
                    })
                    .collect()
            }
            Op::Write { text, .. } => {
                let data = self.literal_table(text.as_bytes().to_vec())?;
                let len = self.int(text.len() as u64);
                let f = self
                    .module
                    .declare_func_in_func(self.write.ok_or("missing writer import")?, self.b.func);
                let call = self.b.ins().call(f, &[data, len]);
                let status = self.b.inst_results(call)[0];
                let valid = self.b.ins().icmp_imm(IntCC::Equal, status, 0);
                self.guard(valid, 6);
                vec![self.int(0)]
            }
            Op::Scan {
                count,
                state,
                function,
                width,
                separator,
                buffer,
                ..
            } => {
                let count = self.expression(count, params, memo)?[0];
                let values = self.expression(state, params, memo)?;
                let stateptr = self.slot(values.len() * 8);
                for (i, value) in values.iter().enumerate() {
                    let encoded = self.encoded_scalar(*value);
                    self.b
                        .ins()
                        .store(MemFlags::new(), encoded, stateptr, (i * 8) as i32);
                }
                let fields = self.int(values.len() as u64);
                let f = self
                    .module
                    .declare_func_in_func(self.scan_callbacks[function], self.b.func);
                let callback = self.b.ins().func_addr(types::I64, f);
                let width = self.expression(width, params, memo)?[0];
                let sep = self.literal_table(separator.as_bytes().to_vec())?;
                let seplen = self.int(separator.len() as u64);
                let buffer = self.expression(buffer, params, memo)?[0];
                let out = self.slot(values.len() * 8);
                let period = (std::env::var("MECH_CONTROL_SCAN_CACHE").as_deref() != Ok("0"))
                    .then(|| scan_period(&self.program.functions[function]))
                    .flatten();
                let mut arguments = vec![
                    count, stateptr, fields, callback, width, sep, seplen, buffer, out,
                ];
                let import = if let Some(period) = period {
                    arguments.push(self.int(period));
                    self.periodic_scan.ok_or("missing periodic scan import")?
                } else if let Some(&(producer, mapper)) = self.pipeline_callbacks.get(function) {
                    for id in [producer, mapper] {
                        let f = self.module.declare_func_in_func(id, self.b.func);
                        arguments.push(self.b.ins().func_addr(types::I64, f));
                    }
                    self.pipeline_scan.ok_or("missing pipeline scan import")?
                } else {
                    self.scan.ok_or("missing scan import")?
                };
                let scan = self.module.declare_func_in_func(import, self.b.func);
                let call = self.b.ins().call(scan, &arguments);
                let status = self.b.inst_results(call)[0];
                let valid = self.b.ins().icmp_imm(IntCC::Equal, status, 0);
                self.guard(valid, 6);
                types_for(&state.kind)
                    .iter()
                    .enumerate()
                    .map(|(i, kind)| {
                        let v = self
                            .b
                            .ins()
                            .load(types::I64, MemFlags::new(), out, (i * 8) as i32);
                        self.decoded_scalar(v, *kind)
                    })
                    .collect()
            }
            Op::Primitive(name, args) => {
                let mut v = Vec::new();
                for arg in args {
                    v.push(self.expression(arg, params, memo)?);
                }
                let x = v.first().and_then(|v| v.first()).copied();
                if args.first().is_some_and(|e| e.kind == Kind::Float) {
                    let a = x.unwrap();
                    let b = v[1][0];
                    let comparison = match name.as_str() {
                        "<" => Some(FloatCC::LessThan),
                        "<=" => Some(FloatCC::LessThanOrEqual),
                        ">" => Some(FloatCC::GreaterThan),
                        ">=" => Some(FloatCC::GreaterThanOrEqual),
                        _ => None,
                    };
                    if let Some(value) =
                        comparison.and_then(|cc| self.exact_division_compare(cc, a, b))
                    {
                        memo.insert(e.clone(), vec![value]);
                        return Ok(vec![value]);
                    }
                    let value = match name.as_str() {
                        "+" => self.b.ins().fadd(a, b),
                        "-" => self.b.ins().fsub(a, b),
                        "*" => self.b.ins().fmul(a, b),
                        "/" => self.b.ins().fdiv(a, b),
                        "==" => self.b.ins().fcmp(FloatCC::Equal, a, b),
                        "!=" => self.b.ins().fcmp(FloatCC::NotEqual, a, b),
                        "<" => self.b.ins().fcmp(FloatCC::LessThan, a, b),
                        "<=" => self.b.ins().fcmp(FloatCC::LessThanOrEqual, a, b),
                        ">" => self.b.ins().fcmp(FloatCC::GreaterThan, a, b),
                        ">=" => self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, a, b),
                        _ => return Err("floating control operation".into()),
                    };
                    if name == "/" && self.exact_float_compare {
                        if let Some((&integer, &bits)) =
                            self.uint_floats.get(&a).zip(self.float_constants.get(&b))
                        {
                            let divisor = f64::from_bits(bits);
                            if divisor.is_finite() && divisor > 0.0 {
                                self.uint_divisions.insert(value, (integer, divisor));
                            }
                        }
                    }
                    let comparison = match name.as_str() {
                        "==" => Some(FloatCC::Equal),
                        "!=" => Some(FloatCC::NotEqual),
                        "<" => Some(FloatCC::LessThan),
                        "<=" => Some(FloatCC::LessThanOrEqual),
                        ">" => Some(FloatCC::GreaterThan),
                        ">=" => Some(FloatCC::GreaterThanOrEqual),
                        _ => None,
                    };
                    if let Some(cc) = comparison {
                        self.float_conditions.insert(value, (cc, a, b));
                    }
                    memo.insert(e.clone(), vec![value]);
                    return Ok(vec![value]);
                }
                match name.as_str() {
                    "number/shift-left" => {
                        let valid = self.b.ins().icmp_imm(IntCC::UnsignedLessThan, v[1][0], 64);
                        self.guard(valid, 2);
                        vec![self.b.ins().ishl(x.unwrap(), v[1][0])]
                    }
                    "number/to-f64" => {
                        let integer = x.unwrap();
                        let mut value = integer;
                        if self.narrow_uint
                            && self
                                .facts
                                .get(&value)
                                .is_some_and(|&max| max <= u32::MAX as u64)
                        {
                            value = self.b.ins().ireduce(types::I32, value);
                        }
                        let value = self.b.ins().fcvt_from_uint(types::F64, value);
                        if self.exact_float_compare
                            && self
                                .facts
                                .get(&integer)
                                .is_some_and(|&maximum| maximum <= (1 << 53))
                        {
                            self.uint_floats.insert(value, integer);
                        }
                        vec![value]
                    }
                    "sequence/length" => vec![v[0][1]],
                    "sequence/get" if matches!(args[0].kind, Kind::ReadBytes | Kind::Floats) => {
                        let index = v[1][0];
                        let lower_proved = positive(
                            &args[1],
                            &params
                                .iter()
                                .map(|v| self.facts.get(v).copied())
                                .collect::<Vec<_>>(),
                            self.program,
                        );
                        // A length's upper bound is insufficient here: only an
                        // exact length can establish that every index fits.
                        let upper_proved = self
                            .facts
                            .get(&index)
                            .zip(self.constants.get(&v[0][1]))
                            .is_some_and(|(&maximum, &length)| maximum <= length);
                        let mut valid = None;
                        if !lower_proved {
                            valid = Some(self.b.ins().icmp_imm(
                                IntCC::UnsignedGreaterThanOrEqual,
                                index,
                                1,
                            ));
                        }
                        if !upper_proved {
                            let upper =
                                self.b
                                    .ins()
                                    .icmp(IntCC::UnsignedLessThanOrEqual, index, v[0][1]);
                            valid = Some(if let Some(lower) = valid {
                                self.b.ins().band(lower, upper)
                            } else {
                                upper
                            });
                        }
                        if let Some(valid) = valid {
                            self.guard(valid, 4);
                        }
                        let mut offset = self.b.ins().iadd_imm(index, -1);
                        if args[0].kind == Kind::Floats {
                            offset = self.b.ins().ishl_imm(offset, 3);
                        }
                        let address = self.b.ins().iadd(v[0][0], offset);
                        if args[0].kind == Kind::Floats {
                            if let Some(bits) = self
                                .constants
                                .get(&index)
                                .and_then(|i| i.checked_sub(1))
                                .and_then(|i| usize::try_from(i).ok())
                                .and_then(|i| {
                                    self.float_tables
                                        .get(&v[0][0])
                                        .and_then(|table| table.get(i))
                                })
                                .copied()
                            {
                                let value = self.b.ins().f64const(Ieee64::with_bits(bits));
                                self.float_constants.insert(value, bits);
                                vec![value]
                            } else {
                                let value =
                                    self.b.ins().load(types::F64, MemFlags::new(), address, 0);
                                if let Some(bits) = self.float_tables.get(&v[0][0]) {
                                    self.indexed_float_tables
                                        .insert(value, (index, bits.clone()));
                                }
                                vec![value]
                            }
                        } else {
                            let byte = self.b.ins().load(types::I8, MemFlags::new(), address, 0);
                            vec![self.b.ins().uextend(types::I64, byte)]
                        }
                    }
                    "io/argument-u64" => {
                        let Op::Integer(index) = args[0].op else {
                            return Err("argument index".into());
                        };
                        vec![self.b.ins().load(
                            types::I64,
                            MemFlags::new(),
                            self.arguments,
                            ((index - 1) * 8) as i32,
                        )]
                    }
                    "sequence/iota-bytes" | "sequence/zero-bytes" => {
                        let Kind::Bytes(n) = e.kind else {
                            return Err("byte constructor".into());
                        };
                        (0..n.div_ceil(16))
                            .map(|block| {
                                self.vector(std::array::from_fn(|i| {
                                    if name == "sequence/iota-bytes" && block * 16 + i < n {
                                        (block * 16 + i) as u8
                                    } else {
                                        0
                                    }
                                }))
                            })
                            .collect()
                    }
                    "+" => vec![self.add_counts(x.unwrap(), v[1][0])],
                    "-" => vec![self.b.ins().isub(x.unwrap(), v[1][0])],
                    "*" => vec![self.integer_arithmetic("*", x.unwrap(), v[1][0])],
                    "/" | "number/mod" => {
                        let a = x.unwrap();
                        let d = v[1][0];
                        if let Some(&n) = self.constants.get(&d) {
                            if n == 0 {
                                return Err("constant division by zero".into());
                            }
                            if n.is_power_of_two() {
                                vec![if name == "/" {
                                    self.b.ins().ushr_imm(a, n.trailing_zeros() as i64)
                                } else {
                                    self.b.ins().band_imm(a, (n - 1) as i64)
                                }]
                            } else {
                                vec![self.divide(name, a, d)]
                            }
                        } else {
                            let valid = self.b.ins().icmp_imm(IntCC::NotEqual, d, 0);
                            self.guard(valid, 2);
                            vec![self.divide(name, a, d)]
                        }
                    }
                    "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                        let cc = match name.as_str() {
                            "==" => IntCC::Equal,
                            "!=" => IntCC::NotEqual,
                            "<" => IntCC::UnsignedLessThan,
                            "<=" => IntCC::UnsignedLessThanOrEqual,
                            ">" => IntCC::UnsignedGreaterThan,
                            _ => IntCC::UnsignedGreaterThanOrEqual,
                        };
                        vec![self.b.ins().icmp(cc, x.unwrap(), v[1][0])]
                    }
                    "&&" => vec![self.b.ins().band(x.unwrap(), v[1][0])],
                    "||" => vec![self.b.ins().bor(x.unwrap(), v[1][0])],
                    "number/max" => {
                        let a = x.unwrap();
                        let b = v[1][0];
                        let greater = self.b.ins().icmp(IntCC::UnsignedGreaterThan, a, b);
                        vec![self.b.ins().select(greater, a, b)]
                    }
                    "control/select" => {
                        let values: Values = v[1]
                            .iter()
                            .zip(&v[2])
                            .map(|(a, b)| self.b.ins().select(x.unwrap(), *a, *b))
                            .collect();
                        if v[1].len() == 1
                            && self.constants.get(&v[1][0]) == Some(&1)
                            && self.constants.get(&v[2][0]) == Some(&0)
                        {
                            if let Some(&(comparison, left, right)) =
                                self.float_conditions.get(&x.unwrap())
                            {
                                self.count_sums.insert(
                                    values[0],
                                    CountSum {
                                        terms: vec![FloatCount {
                                            comparison,
                                            left,
                                            right,
                                            scalar: values[0],
                                        }],
                                        integers: vec![],
                                        other: vec![],
                                    },
                                );
                            } else if let Some(&(comparison, left, right)) =
                                self.integer_conditions.get(&x.unwrap())
                            {
                                self.count_sums.insert(
                                    values[0],
                                    CountSum {
                                        terms: vec![],
                                        integers: vec![IntegerCount {
                                            comparison,
                                            left,
                                            right,
                                            scalar: values[0],
                                        }],
                                        other: vec![],
                                    },
                                );
                            }
                        }
                        values
                    }
                    "number/bounded" => {
                        let a = x.unwrap();
                        let low = self
                            .b
                            .ins()
                            .icmp(IntCC::UnsignedGreaterThanOrEqual, a, v[1][0]);
                        let high = self
                            .b
                            .ins()
                            .icmp(IntCC::UnsignedLessThanOrEqual, a, v[2][0]);
                        let valid = self.b.ins().band(low, high);
                        self.guard(valid, 1);
                        vec![a]
                    }
                    "number/factorial" => {
                        let n = x.unwrap();
                        let valid = self.b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, n, 20);
                        self.guard(valid, 3);
                        let head = self.b.create_block();
                        let body = self.b.create_block();
                        let done = self.b.create_block();
                        self.b.append_block_param(head, types::I64);
                        self.b.append_block_param(head, types::I64);
                        self.b.append_block_param(done, types::I64);
                        let one = self.int(1);
                        self.b.ins().jump(head, &[one.into(), one.into()]);
                        self.b.switch_to_block(head);
                        let i = self.b.block_params(head)[0];
                        let product = self.b.block_params(head)[1];
                        let more = self.b.ins().icmp(IntCC::UnsignedLessThanOrEqual, i, n);
                        self.b.ins().brif(more, body, &[], done, &[product.into()]);
                        self.b.switch_to_block(body);
                        let product = self.b.ins().imul(product, i);
                        let next = self.b.ins().iadd_imm(i, 1);
                        self.b.ins().jump(head, &[next.into(), product.into()]);
                        self.b.switch_to_block(done);
                        vec![self.b.block_params(done)[0]]
                    }
                    "sequence/get"
                    | "sequence/set"
                    | "sequence/reverse-prefix"
                    | "sequence/rotate-prefix" => self.bytes(name, args, &v, params)?,
                    _ => return Err(format!("unlowered native primitive {name}")),
                }
            }
        };
        let domains = abstract_bounds(
            e,
            &params
                .iter()
                .map(|v| self.facts.get(v).copied())
                .collect::<Vec<_>>(),
            self.program,
        );
        for (v, bound) in out.iter().zip(domains) {
            if let Some(bound) = bound {
                self.facts.insert(*v, bound);
            }
        }
        memo.insert(e.clone(), out.clone());
        Ok(out)
    }
    fn bytes(
        &mut self,
        name: &str,
        args: &[Expr],
        v: &[Values],
        params: &[Value],
    ) -> Result<Values, String> {
        let Kind::Bytes(n) = args[0].kind else {
            return Err("byte operand".into());
        };
        let index = v[1][0];
        let upper = self
            .b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, index, n as i64);
        let valid = if name == "sequence/get" || name == "sequence/set" {
            let lower = self
                .b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, index, 1);
            self.b.ins().band(lower, upper)
        } else {
            upper
        };
        let proved = if name == "sequence/get" || name == "sequence/set" {
            self.facts.get(&index).is_some_and(|&max| max <= n as u64)
                && positive(
                    &args[1],
                    &params
                        .iter()
                        .map(|v| self.facts.get(v).copied())
                        .collect::<Vec<_>>(),
                    self.program,
                )
        } else {
            self.facts.get(&index).is_some_and(|&max| max <= n as u64)
        };
        if !proved {
            self.guard(valid, 4);
        }
        if name == "sequence/set" {
            let valid = self
                .b
                .ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, v[2][0], 255);
            if !self.facts.get(&v[2][0]).is_some_and(|&max| max <= 255) {
                self.guard(valid, 4);
            }
        }
        if name == "sequence/get" {
            if let Op::Integer(i) = args[1].op {
                if i == 0 || i > n as u64 {
                    // The failing bounds guard above dominates this unreachable
                    // continuation. Avoid indexing the compiler's vector list;
                    // a lazy source branch can still leave this read untaken.
                    return Ok(vec![self.int(0)]);
                }
                let lane = self
                    .b
                    .ins()
                    .extractlane(v[0][(i as usize - 1) / 16], ((i - 1) % 16) as u8);
                return Ok(vec![self.b.ins().uextend(types::I64, lane)]);
            }
            if !self.simd || self.stack_index {
                let temp = self.slot(n.div_ceil(16) * 16);
                for (i, vector) in v[0].iter().enumerate() {
                    self.b
                        .ins()
                        .store(MemFlags::new(), *vector, temp, (i * 16) as i32);
                }
                let offset = self.b.ins().iadd_imm(index, -1);
                let address = self.b.ins().iadd(temp, offset);
                let byte = self.b.ins().load(types::I8, MemFlags::new(), address, 0);
                return Ok(vec![self.b.ins().uextend(types::I64, byte)]);
            }
            let offset = self.b.ins().iadd_imm(index, -1);
            let byte = self.b.ins().ireduce(types::I8, offset);
            let mask = self.b.ins().splat(types::I8X16, byte);
            let mut selected = self.b.ins().swizzle(v[0][0], mask);
            for (chunk, vector) in v[0].iter().enumerate().skip(1) {
                let byte = self.b.ins().iadd_imm(byte, -((chunk * 16) as i64));
                let mask = self.b.ins().splat(types::I8X16, byte);
                let candidate = self.b.ins().swizzle(*vector, mask);
                selected = self.b.ins().bor(selected, candidate);
            }
            let byte = self.b.ins().extractlane(selected, 0);
            return Ok(vec![self.b.ins().uextend(types::I64, byte)]);
        }
        if name == "sequence/set" && self.simd {
            let offset = self.b.ins().iadd_imm(index, -1);
            let byte = self.b.ins().ireduce(types::I8, offset);
            let indices = self.b.ins().splat(types::I8X16, byte);
            let data = self.b.ins().ireduce(types::I8, v[2][0]);
            let data = self.b.ins().splat(types::I8X16, data);
            return Ok(v[0]
                .iter()
                .enumerate()
                .map(|(chunk, vector)| {
                    let lanes = self.vector(std::array::from_fn(|i| (chunk * 16 + i) as u8));
                    let mask = self.b.ins().icmp(IntCC::Equal, indices, lanes);
                    self.b.ins().bitselect(mask, data, *vector)
                })
                .collect());
        }
        if self.simd && n <= 16 && name != "sequence/set" {
            // Exhaustively precompute each legal byte permutation; only shape and
            // operation semantics determine this immutable shuffle table.
            let rotate = name == "sequence/rotate-prefix";
            let one = rotate && matches!(args[2].op, Op::Integer(1));
            let rows = if one || !rotate { 1 } else { 16 };
            let shift = if rows == 1 {
                self.int(0)
            } else {
                let length_nonzero = self.b.ins().icmp_imm(IntCC::NotEqual, index, 0);
                let one_value = self.int(1);
                let safe = self.b.ins().select(length_nonzero, index, one_value);
                self.b.ins().urem(v[2][0], safe)
            };
            let mut data = Vec::new();
            for length in 0..=n {
                for shift in 0..rows {
                    for i in 0..16 {
                        data.push(if i >= length {
                            i as u8
                        } else if rotate {
                            ((i + if one { 1 } else { shift }) % length) as u8
                        } else {
                            (length - 1 - i) as u8
                        });
                    }
                }
            }
            let key = format!(
                "mech_control_{}_{}_{}",
                if rotate { "rotate" } else { "reverse" },
                n,
                rows
            );
            let table = self.table(key, data)?;
            let offset = if rows == 1 {
                self.b.ins().ishl_imm(index, 4)
            } else {
                let row = self.b.ins().ishl_imm(index, 4);
                let row = self.b.ins().iadd(row, shift);
                self.b.ins().ishl_imm(row, 4)
            };
            let address = self.b.ins().iadd(table, offset);
            let mask = self.b.ins().load(types::I8X16, MemFlags::new(), address, 0);
            return Ok(vec![self.b.ins().swizzle(v[0][0], mask)]);
        }
        let source = self.slot(n.div_ceil(16) * 16);
        let out = self.slot(n.div_ceil(16) * 16);
        for (i, vector) in v[0].iter().enumerate() {
            self.b
                .ins()
                .store(MemFlags::new(), *vector, source, (i * 16) as i32);
            self.b
                .ins()
                .store(MemFlags::new(), *vector, out, (i * 16) as i32);
        }
        if name == "sequence/set" {
            let offset = self.b.ins().iadd_imm(index, -1);
            let address = self.b.ins().iadd(out, offset);
            let byte = self.b.ins().ireduce(types::I8, v[2][0]);
            self.b.ins().store(MemFlags::new(), byte, address, 0);
        } else {
            let head = self.b.create_block();
            let body = self.b.create_block();
            let exit = self.b.create_block();
            self.b.append_block_param(head, types::I64);
            let zero = self.int(0);
            self.b.ins().jump(head, &[zero.into()]);
            self.b.switch_to_block(head);
            let i = self.b.block_params(head)[0];
            let more = self.b.ins().icmp(IntCC::UnsignedLessThan, i, index);
            self.b.ins().brif(more, body, &[], exit, &[]);
            self.b.switch_to_block(body);
            let from = if name == "sequence/reverse-prefix" {
                let last = self.b.ins().iadd_imm(index, -1);
                self.b.ins().isub(last, i)
            } else {
                let shift = self.b.ins().urem(v[2][0], index);
                let shifted = self.b.ins().iadd(i, shift);
                self.b.ins().urem(shifted, index)
            };
            let from = self.b.ins().iadd(source, from);
            let byte = self.b.ins().load(types::I8, MemFlags::new(), from, 0);
            let dest = self.b.ins().iadd(out, i);
            self.b.ins().store(MemFlags::new(), byte, dest, 0);
            let next = self.b.ins().iadd_imm(i, 1);
            self.b.ins().jump(head, &[next.into()]);
            self.b.switch_to_block(exit);
        }
        Ok((0..n.div_ceil(16))
            .map(|i| {
                self.b
                    .ins()
                    .load(types::I8X16, MemFlags::new(), out, (i * 16) as i32)
            })
            .collect())
    }
}
fn flatten_parameters(e: &mut Expr, inputs: &[Kind]) {
    match &mut e.op {
        Op::Parameter(i) => {
            *i = inputs[..*i].iter().map(|k| types_for(k).len()).sum();
        }
        Op::Tuple(a)
        | Op::Call(_, a)
        | Op::Primitive(_, a)
        | Op::Invoke { arguments: a, .. }
        | Op::Heap { arguments: a, .. }
        | Op::Print { values: a, .. } => {
            for e in a {
                flatten_parameters(e, inputs)
            }
        }
        Op::Sequence { effects, output } => {
            for e in effects {
                flatten_parameters(e, inputs);
            }
            flatten_parameters(output, inputs);
        }
        Op::Arena {
            capacity,
            arguments,
            ..
        } => {
            if let Some(e) = capacity {
                flatten_parameters(e, inputs);
            }
            flatten_parameters(arguments, inputs);
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            flatten_parameters(scrutinee, inputs);
            for (condition, body) in arms {
                flatten_parameters(condition, inputs);
                flatten_parameters(body, inputs);
            }
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            for e in [start, step, end, body] {
                flatten_parameters(e, inputs);
            }
        }
        Op::Field(e, _) => flatten_parameters(e, inputs),
        Op::Loop { initial, .. } => flatten_parameters(initial, inputs),
        Op::Branch {
            condition, state, ..
        } => {
            flatten_parameters(condition, inputs);
            flatten_parameters(state, inputs)
        }
        Op::Parallel {
            start,
            end,
            grain,
            arguments,
            ..
        } => {
            for e in [start, end, grain, arguments] {
                flatten_parameters(e, inputs)
            }
        }
        _ => {}
    }
}
fn parallel_names(e: &Expr, out: &mut Vec<String>) {
    match &e.op {
        Op::Parallel {
            function,
            start,
            end,
            grain,
            arguments,
            ..
        } => {
            if !out.contains(function) {
                out.push(function.clone());
            }
            for e in [start, end, grain, arguments] {
                parallel_names(e, out)
            }
        }
        Op::Tuple(a)
        | Op::Call(_, a)
        | Op::Primitive(_, a)
        | Op::Invoke { arguments: a, .. }
        | Op::Heap { arguments: a, .. }
        | Op::Print { values: a, .. } => {
            for e in a {
                parallel_names(e, out)
            }
        }
        Op::Sequence { effects, output } => {
            for e in effects {
                parallel_names(e, out);
            }
            parallel_names(output, out);
        }
        Op::Arena {
            capacity,
            arguments,
            ..
        } => {
            if let Some(e) = capacity {
                parallel_names(e, out);
            }
            parallel_names(arguments, out);
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            parallel_names(scrutinee, out);
            for (condition, body) in arms {
                parallel_names(condition, out);
                parallel_names(body, out);
            }
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            for e in [start, step, end, body] {
                parallel_names(e, out);
            }
        }
        Op::Field(e, _) => parallel_names(e, out),
        _ => {}
    }
}
fn scan_names(e: &Expr, out: &mut Vec<String>) {
    match &e.op {
        Op::Scan {
            function,
            count,
            state,
            width,
            buffer,
            ..
        } => {
            if !out.contains(function) {
                out.push(function.clone());
            }
            for e in [count, state, width, buffer] {
                scan_names(e, out);
            }
        }
        Op::Tuple(a)
        | Op::Call(_, a)
        | Op::Primitive(_, a)
        | Op::Invoke { arguments: a, .. }
        | Op::Heap { arguments: a, .. }
        | Op::Print { values: a, .. } => {
            for e in a {
                scan_names(e, out);
            }
        }
        Op::Sequence { effects, output } => {
            for e in effects {
                scan_names(e, out);
            }
            scan_names(output, out);
        }
        Op::Arena {
            capacity,
            arguments,
            ..
        } => {
            if let Some(e) = capacity {
                scan_names(e, out);
            }
            scan_names(arguments, out);
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            scan_names(scrutinee, out);
            for (condition, body) in arms {
                scan_names(condition, out);
                scan_names(body, out);
            }
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            for e in [start, step, end, body] {
                scan_names(e, out);
            }
        }
        Op::Field(e, _) => scan_names(e, out),
        Op::Loop { initial, .. } => scan_names(initial, out),
        Op::Branch {
            condition, state, ..
        } => {
            scan_names(condition, out);
            scan_names(state, out);
        }
        _ => {}
    }
}
fn register_result(kind: &Kind) -> bool {
    let kinds = types_for(kind);
    kinds.len() == 1 && matches!(kinds[0], types::I64 | types::F64 | types::I8)
}
fn arena_names(e: &Expr, out: &mut BTreeSet<(String, Kind)>) {
    match &e.op {
        Op::Arena {
            capacity,
            function,
            layout,
            arguments,
            ..
        } => {
            if let Some(capacity) = capacity {
                out.insert((function.clone(), layout.clone()));
                arena_names(capacity, out);
            }
            arena_names(arguments, out);
        }
        Op::Sequence { effects, output } => {
            for e in effects {
                arena_names(e, out);
            }
            arena_names(output, out);
        }
        Op::Tuple(a)
        | Op::Call(_, a)
        | Op::Primitive(_, a)
        | Op::Invoke { arguments: a, .. }
        | Op::Heap { arguments: a, .. }
        | Op::Print { values: a, .. } => {
            for e in a {
                arena_names(e, out);
            }
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            arena_names(scrutinee, out);
            for (condition, body) in arms {
                arena_names(condition, out);
                arena_names(body, out);
            }
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            for e in [start, step, end, body] {
                arena_names(e, out);
            }
        }
        Op::Field(e, _) | Op::Loop { initial: e, .. } => arena_names(e, out),
        Op::Branch {
            condition, state, ..
        } => {
            arena_names(condition, out);
            arena_names(state, out);
        }
        Op::Parallel {
            start,
            end,
            grain,
            arguments,
            ..
        } => {
            for e in [start, end, grain, arguments] {
                arena_names(e, out);
            }
        }
        _ => {}
    }
}
#[derive(Clone, Default)]
struct ContextFacts {
    valid: bool,
    layout: Option<Kind>,
    base: Option<Value>,
    capacity: Option<Value>,
}
fn field_kinds(kind: &Kind) -> Vec<Kind> {
    match kind {
        Kind::Tuple(kinds) => kinds.clone(),
        Kind::Record(fields) => fields.iter().map(|(_, k)| k.clone()).collect(),
        _ => unreachable!("aggregate field kind"),
    }
}
fn memory_layouts(
    e: &Expr,
    program: &Program,
    active: &mut BTreeSet<String>,
    out: &mut BTreeSet<Kind>,
    regions: bool,
) {
    let visit = |name: &str, active: &mut BTreeSet<String>, out: &mut BTreeSet<Kind>| {
        if active.insert(name.to_owned()) {
            memory_layouts(
                &program.functions[name].output,
                program,
                active,
                out,
                regions,
            );
            active.remove(name);
        }
    };
    match &e.op {
        Op::Heap {
            layout, arguments, ..
        } => {
            if let Some(layout) = layout {
                out.insert(layout.clone());
            }
            for e in arguments {
                memory_layouts(e, program, active, out, regions);
            }
        }
        Op::Arena {
            capacity,
            layout,
            function,
            arguments,
            ..
        } => {
            if let Some(capacity) = capacity {
                if regions {
                    out.insert(layout.clone());
                }
                memory_layouts(capacity, program, active, out, regions);
            } else {
                visit(function, active, out);
            }
            memory_layouts(arguments, program, active, out, regions);
        }
        Op::Invoke {
            name, arguments, ..
        }
        | Op::Call(name, arguments) => {
            visit(name, active, out);
            for e in arguments {
                memory_layouts(e, program, active, out, regions);
            }
        }
        Op::Sequence { effects, output } => {
            for e in effects {
                memory_layouts(e, program, active, out, regions);
            }
            memory_layouts(output, program, active, out, regions);
        }
        Op::Tuple(a) | Op::Primitive(_, a) | Op::Print { values: a, .. } => {
            for e in a {
                memory_layouts(e, program, active, out, regions);
            }
        }
        Op::Match {
            scrutinee, arms, ..
        } => {
            memory_layouts(scrutinee, program, active, out, regions);
            for (condition, body) in arms {
                memory_layouts(condition, program, active, out, regions);
                memory_layouts(body, program, active, out, regions);
            }
        }
        Op::Reduce {
            start,
            step,
            end,
            body,
            ..
        } => {
            for e in [start, step, end, body] {
                memory_layouts(e, program, active, out, regions);
            }
        }
        Op::Field(e, _) => memory_layouts(e, program, active, out, regions),
        Op::Loop {
            initial,
            predicate,
            step,
            ..
        } => {
            memory_layouts(initial, program, active, out, regions);
            visit(predicate, active, out);
            visit(step, active, out);
        }
        Op::Branch {
            condition,
            state,
            yes,
            no,
            ..
        } => {
            memory_layouts(condition, program, active, out, regions);
            memory_layouts(state, program, active, out, regions);
            visit(yes, active, out);
            visit(no, active, out);
        }
        Op::Parallel {
            start,
            end,
            grain,
            arguments,
            ..
        } => {
            for e in [start, end, grain, arguments] {
                memory_layouts(e, program, active, out, regions);
            }
        }
        _ => {}
    }
}
pub fn emit(source: &str, directory: &Path, simd: bool) -> Result<(), String> {
    let mut flags = settings::builder();
    flags.set("is_pic", "true").map_err(|e| e.to_string())?;
    flags.set("opt_level", "speed").map_err(|e| e.to_string())?;
    let isa = cranelift_native::builder()
        .map_err(|e| e.to_string())?
        .finish(settings::Flags::new(flags))
        .map_err(|e| e.to_string())?;
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "mech_control", default_libcall_names())
            .map_err(|e| e.to_string())?,
    );
    let (_, descriptor) = define(source, &mut module, simd)?;
    let object = module.finish().emit().map_err(|e| e.to_string())?;
    std::fs::write(directory.join("mech-control.o"), object).map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("control.json"),
        serde_json::to_string(&descriptor).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn define<M: Module>(
    source: &str,
    module: &mut M,
    simd: bool,
) -> Result<(FuncId, serde_json::Value), String> {
    let program = mech_engine::native_sequence::control::compile(
        source,
        mech_stdlib::native_control_catalog(),
    )?;
    define_program(program, module, simd)
}
fn define_program<M: Module>(
    mut program: Program,
    module: &mut M,
    simd: bool,
) -> Result<(FuncId, serde_json::Value), String> {
    let stack_index = match std::env::var("MECH_CONTROL_BYTE_INDEX")
        .unwrap_or_else(|_| "auto".into())
        .as_str()
    {
        // Scalar byte results with dynamic indices need a shuffle and lane
        // transfer on ARM. A private vector store plus indexed byte load has a
        // shorter dependent instruction chain. Constant indices still extract
        // a lane directly; prefix operations retain their SIMD lowering.
        "auto" => cfg!(target_arch = "aarch64"),
        "vector" => false,
        "stack" => true,
        _ => return Err("MECH_CONTROL_BYTE_INDEX must be auto, vector or stack".into()),
    };
    for function in program.functions.values_mut() {
        flatten_parameters(&mut function.output, &function.inputs);
    }
    if module.target_config().pointer_type() != types::I64 {
        return Err("native control requires a 64-bit host".into());
    }
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 9];
    signature.returns.push(AbiParam::new(types::I64));
    let reduce = module
        .declare_function("mech_control_reduce", Linkage::Import, &signature)
        .map_err(|e| e.to_string())?;
    let (scan, periodic_scan, pipeline_scan, write) = if !program.effects.is_empty() {
        let scan = module
            .declare_function("mech_control_scan", Linkage::Import, &signature)
            .map_err(|e| e.to_string())?;
        let mut periodic_signature = signature.clone();
        periodic_signature.params.push(AbiParam::new(types::I64));
        let periodic_scan = module
            .declare_function(
                "mech_control_scan_periodic",
                Linkage::Import,
                &periodic_signature,
            )
            .map_err(|e| e.to_string())?;
        let mut pipeline_signature = periodic_signature.clone();
        pipeline_signature.params.push(AbiParam::new(types::I64));
        let pipeline_scan = module
            .declare_function(
                "mech_control_scan_pipeline",
                Linkage::Import,
                &pipeline_signature,
            )
            .map_err(|e| e.to_string())?;
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 2];
        sig.returns.push(AbiParam::new(types::I64));
        let write = module
            .declare_function("mech_control_write", Linkage::Import, &sig)
            .map_err(|e| e.to_string())?;
        (
            Some(scan),
            Some(periodic_scan),
            Some(pipeline_scan),
            Some(write),
        )
    } else {
        (None, None, None, None)
    };
    let (arena, print) = if program.native_functions {
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 7];
        sig.returns.push(AbiParam::new(types::I64));
        let arena = module
            .declare_function("mech_control_ref_arena", Linkage::Import, &sig)
            .map_err(|e| e.to_string())?;
        sig.params.truncate(4);
        let print = module
            .declare_function("mech_control_print", Linkage::Import, &sig)
            .map_err(|e| e.to_string())?;
        (Some(arena), Some(print))
    } else {
        (None, None)
    };
    let mut names = Vec::new();
    let bigint = if program.computation.entry.coeffects.big_integers {
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 5];
        sig.returns.push(AbiParam::new(types::I64));
        Some(
            module
                .declare_function("mech_control_bigint", Linkage::Import, &sig)
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    parallel_names(&program.output, &mut names);
    if program.native_functions {
        for e in &program.effects {
            parallel_names(e, &mut names);
        }
        for f in program.functions.values() {
            parallel_names(&f.output, &mut names);
        }
    }
    let mut callbacks = BTreeMap::new();
    for (i, name) in names.iter().enumerate() {
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 4];
        sig.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function(&format!("mech_control_range_{i}"), Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;
        callbacks.insert(name.clone(), id);
    }
    let mut units = names
        .iter()
        .map(|name| (Some(name.clone()), callbacks[name], 0))
        .collect::<Vec<_>>();
    let mut scans = Vec::new();
    for e in &program.effects {
        scan_names(e, &mut scans);
    }
    scan_names(&program.output, &mut scans);
    let mut state_domains = BTreeMap::new();
    for e in program
        .effects
        .iter()
        .chain(std::iter::once(&program.output))
    {
        scan_domains(e, &program, &mut state_domains);
    }
    let mut scan_callbacks = BTreeMap::new();
    for (i, name) in scans.iter().enumerate() {
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 4];
        sig.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function(
                &format!("mech_control_scan_block_{i}"),
                Linkage::Local,
                &sig,
            )
            .map_err(|e| e.to_string())?;
        scan_callbacks.insert(name.clone(), id);
        units.push((Some(name.clone()), id, 1));
    }
    let mut pipeline_callbacks = BTreeMap::new();
    let pipeline_policy = std::env::var("MECH_CONTROL_SCAN_PIPELINE").unwrap_or("auto".into());
    if !["0", "1", "auto"].contains(&pipeline_policy.as_str()) {
        return Err("MECH_CONTROL_SCAN_PIPELINE must be auto, 0 or 1".into());
    }
    if pipeline_policy != "0" {
        for (i, name) in scans.iter().enumerate() {
            let f = &program.functions[name];
            if f.inputs != [Kind::Integer] || scan_period(f).is_some() {
                continue;
            }
            let Op::Tuple(items) = &f.output.op else {
                continue;
            };
            if items.len() != 2 || items.iter().any(|e| e.kind != Kind::Integer) {
                continue;
            }
            let Some(maximum) = abstract_bounds(&items[0], &state_domains[name], &program)[0]
            else {
                continue;
            };
            if maximum > u32::MAX as u64 {
                continue;
            }
            let Some(mapping) = scan_sample_expression(&items[1], &items[0]) else {
                continue;
            };
            if pipeline_policy == "auto"
                && scan_mapping_work(&mapping, &program, &mut std::collections::BTreeSet::new()) < 8
            {
                continue;
            }
            let producer_name = format!("__mech_scan_producer_{i}");
            let mapper_name = format!("__mech_scan_mapper_{i}");
            if program.functions.contains_key(&producer_name)
                || program.functions.contains_key(&mapper_name)
            {
                return Err("source name conflicts with internal scan callback".into());
            }
            let producer = mech_engine::native_sequence::control::Function {
                inputs: f.inputs.clone(),
                output: Expr {
                    kind: Kind::Tuple(vec![Kind::Integer, Kind::Integer]),
                    op: Op::Tuple(vec![items[0].clone(), items[0].clone()]),
                },
            };
            program.functions.insert(producer_name.clone(), producer);
            program.functions.insert(
                mapper_name.clone(),
                mech_engine::native_sequence::control::Function {
                    inputs: vec![Kind::Integer],
                    output: mapping,
                },
            );
            state_domains.insert(producer_name.clone(), state_domains[name].clone());
            state_domains.insert(mapper_name.clone(), vec![Some(maximum)]);
            let mut sig = module.make_signature();
            sig.params = vec![AbiParam::new(types::I64); 4];
            sig.returns.push(AbiParam::new(types::I64));
            let producer_id = module
                .declare_function(&producer_name, Linkage::Local, &sig)
                .map_err(|e| e.to_string())?;
            let mapper_id = module
                .declare_function(&mapper_name, Linkage::Local, &sig)
                .map_err(|e| e.to_string())?;
            units.push((Some(producer_name), producer_id, 2));
            units.push((Some(mapper_name), mapper_id, 3));
            pipeline_callbacks.insert(name.clone(), (producer_id, mapper_id));
        }
    }
    // Parameter flattening and scan synthesis changed the IR; refresh facts
    // before selecting context-dependent native entries.
    program.refresh_computation()?;
    let mut sig = module.make_signature();
    sig.params = vec![AbiParam::new(types::I64); 2];
    sig.returns.push(AbiParam::new(types::I64));
    let main = module
        .declare_function("mech_control_entry", Linkage::Export, &sig)
        .map_err(|e| e.to_string())?;
    let mut all_layouts = BTreeSet::new();
    for e in program
        .effects
        .iter()
        .chain(std::iter::once(&program.output))
        .chain(program.functions.values().map(|f| &f.output))
    {
        memory_layouts(e, &program, &mut BTreeSet::new(), &mut all_layouts, true);
    }
    let layouts = all_layouts
        .into_iter()
        .enumerate()
        .map(|(i, layout)| (layout, i as u64 + 1))
        .collect::<BTreeMap<_, _>>();
    let mut function_layouts = BTreeMap::new();
    for (name, summary) in &program.computation.functions {
        let required = summary
            .coeffects
            .arenas
            .iter()
            .filter_map(|requirement| requirement.layout.clone())
            .collect::<BTreeSet<_>>();
        if summary.arena_conflicts.is_empty() && required.len() == 1 {
            function_layouts.insert(name.clone(), required.into_iter().next().unwrap());
        }
    }
    let mut trusted_functions = BTreeMap::new();
    let mut functions = BTreeMap::new();
    if program.native_functions {
        for (i, (name, f)) in program.functions.iter().enumerate() {
            let mut sig = module.make_signature();
            sig.params.push(AbiParam::new(types::I64)); // checked recursion depth
            sig.params.push(AbiParam::new(types::I64)); // exclusive live arena context
            sig.params
                .extend(f.inputs.iter().flat_map(types_for).map(AbiParam::new));
            if !register_result(&f.output.kind) {
                sig.params.push(AbiParam::new(types::I64));
            }
            sig.returns.push(AbiParam::new(types::I64));
            if register_result(&f.output.kind) {
                sig.returns
                    .push(AbiParam::new(types_for(&f.output.kind)[0]));
            }
            let id = module
                .declare_function(&format!("mech_control_function_{i}"), Linkage::Local, &sig)
                .map_err(|e| e.to_string())?;
            functions.insert(name.clone(), id);
            units.push((Some(name.clone()), id, 4));
            if function_layouts.contains_key(name) {
                let trusted = module
                    .declare_function(&format!("mech_control_trusted_{i}"), Linkage::Local, &sig)
                    .map_err(|e| e.to_string())?;
                trusted_functions.insert(name.clone(), trusted);
                units.push((Some(name.clone()), trusted, 6));
            }
        }
    }
    let mut arena_names_set = BTreeSet::new();
    arena_names(&program.output, &mut arena_names_set);
    for e in &program.effects {
        arena_names(e, &mut arena_names_set);
    }
    for f in program.functions.values() {
        arena_names(&f.output, &mut arena_names_set);
    }
    let mut arena_callbacks = BTreeMap::new();
    let mut arena_unit_layouts = BTreeMap::new();
    for (i, (name, layout)) in arena_names_set.into_iter().enumerate() {
        let mut sig = module.make_signature();
        sig.params = vec![AbiParam::new(types::I64); 3];
        sig.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function(&format!("mech_control_arena_{i}"), Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;
        arena_callbacks.insert((name.clone(), layout.clone()), id);
        arena_unit_layouts.insert(id, layout);
        units.push((Some(name), id, 5));
    }
    units.push((None, main, 0));
    let recursive_inline = std::env::var("MECH_CONTROL_RECURSIVE_INLINE")
        .ok()
        .map(|s| s.parse::<usize>())
        .transpose()
        .map_err(|_| "recursive inline budget must be 0, 1 or 2")?
        .unwrap_or(2);
    if recursive_inline > 2 {
        return Err("recursive inline budget must be 0, 1 or 2".into());
    }
    let mut tables = BTreeMap::new();
    for (name, id, scan_stage) in units {
        let is_scan = (1..=3).contains(&scan_stage);
        let is_function = scan_stage == 4 || scan_stage == 6;
        let is_arena = scan_stage == 5;
        let register =
            is_function && register_result(&program.functions[name.as_ref().unwrap()].output.kind);
        let mut ctx = module.make_context();
        ctx.func.signature = module
            .declarations()
            .get_function_decl(id)
            .signature
            .clone();
        let mut fb = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let p = b.block_params(entry).to_vec();
        let arguments = if is_function {
            b.ins().iconst(types::I64, 0)
        } else if is_scan || is_arena {
            p[1]
        } else if name.is_some() {
            p[2]
        } else {
            p[0]
        };
        let output = if register {
            b.ins().iconst(types::I64, 0)
        } else if is_function {
            *p.last().unwrap()
        } else if is_arena {
            p[2]
        } else if name.is_some() {
            p[3]
        } else {
            p[1]
        };
        let depth = if is_function {
            p[0]
        } else {
            b.ins().iconst(types::I64, 0)
        };
        // AArch64's I32 arithmetic followed by uextend currently retains
        // redundant extension moves. Preserve the source's I64 registers by
        // default, using range proofs for exact low-product division and
        // integer-to-float conversion. Full narrowing remains measurable.
        let width_override = std::env::var("MECH_CONTROL_NARROW_UINT");
        let narrow_uint = is_scan
            && (width_override.as_deref() == Ok("1")
                || (cfg!(target_arch = "aarch64") && width_override.is_err()));
        let narrow_arithmetic = narrow_uint
            && (std::env::var("MECH_CONTROL_NARROW_ARITHMETIC").as_deref() == Ok("1")
                || (width_override.as_deref() == Ok("1")
                    && std::env::var("MECH_CONTROL_NARROW_ARITHMETIC").as_deref() != Ok("0")));
        let mut emit = Emit {
            b: &mut b,
            module,
            program: &program,
            simd,
            narrow_uint,
            narrow_arithmetic,
            stack_index,
            callbacks: &callbacks,
            functions: &functions,
            trusted_functions: &trusted_functions,
            function_layouts: &function_layouts,
            layouts: &layouts,
            context_facts: ContextFacts {
                valid: is_arena || scan_stage == 6,
                layout: if is_arena {
                    Some(arena_unit_layouts[&id].clone())
                } else if scan_stage == 6 {
                    Some(function_layouts[name.as_ref().unwrap()].clone())
                } else {
                    None
                },
                ..ContextFacts::default()
            },
            error_block: None,
            depth,
            arena,
            arena_callbacks: &arena_callbacks,
            arena_context: if is_function {
                p[1]
            } else if is_arena {
                p[0]
            } else {
                depth
            },
            active_inline: if is_function {
                BTreeMap::from([(name.clone().unwrap(), 1)])
            } else {
                BTreeMap::new()
            },
            recursive_inline,
            print,
            bigint,
            reduce,
            scan,
            periodic_scan,
            pipeline_scan,
            write,
            scan_callbacks: &scan_callbacks,
            pipeline_callbacks: &pipeline_callbacks,
            arguments,
            tables: &mut tables,
            facts: BTreeMap::new(),
            constants: BTreeMap::new(),
            float_constants: BTreeMap::new(),
            float_tables: BTreeMap::new(),
            float_conditions: BTreeMap::new(),
            count_sums: BTreeMap::new(),
            uint_floats: BTreeMap::new(),
            uint_divisions: BTreeMap::new(),
            exact_float_compare: std::env::var("MECH_CONTROL_EXACT_FLOAT_COMPARE").as_deref()
                != Ok("0"),
            indexed_float_tables: BTreeMap::new(),
            integer_conditions: BTreeMap::new(),
        };
        let values = if is_function {
            let valid = emit.b.ins().icmp_imm(IntCC::UnsignedLessThan, depth, 128);
            emit.guard(valid, 8);
            emit.expression(
                &program.functions[name.as_ref().unwrap()].output,
                &p[2..p.len() - usize::from(!register)],
                &mut BTreeMap::new(),
            )?
        } else if is_arena {
            let name = name.as_ref().unwrap();
            let kinds = program.functions[name]
                .inputs
                .iter()
                .flat_map(types_for)
                .collect::<Vec<_>>();
            let args = kinds
                .iter()
                .enumerate()
                .map(|(i, kind)| {
                    let bits =
                        emit.b
                            .ins()
                            .load(types::I64, MemFlags::new(), arguments, (i * 8) as i32);
                    emit.decoded_scalar(bits, *kind)
                })
                .collect::<Vec<_>>();
            emit.call(name, &args)?
        } else if scan_stage == 3 {
            // Mapping consumes independent, range-proved u32 samples. Only the
            // producer carries source state, and output order stays with the IO thread.
            let mapper = name.as_ref().unwrap();
            let head = emit.b.create_block();
            let body = emit.b.create_block();
            let done = emit.b.create_block();
            emit.b.append_block_param(head, types::I64);
            let zero = emit.int(0);
            emit.b.ins().jump(head, &[zero.into()]);
            emit.b.switch_to_block(head);
            let index = emit.b.block_params(head)[0];
            let valid = emit.b.ins().icmp(IntCC::UnsignedLessThan, index, p[0]);
            emit.b.ins().brif(valid, body, &[], done, &[]);
            emit.b.switch_to_block(body);
            let offset = emit.b.ins().ishl_imm(index, 2);
            let address = emit.b.ins().iadd(p[1], offset);
            let sample = emit.b.ins().load(types::I32, MemFlags::new(), address, 0);
            let sample = emit.b.ins().uextend(types::I64, sample);
            let maximum = state_domains[mapper][0].unwrap();
            let valid =
                emit.b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, sample, maximum as i64);
            emit.guard(valid, 4);
            emit.facts.insert(sample, maximum);
            let byte = emit.call(mapper, &[sample])?[0];
            if !emit.facts.get(&byte).is_some_and(|&max| max <= 255) {
                let valid = emit
                    .b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, byte, 255);
                emit.guard(valid, 4);
            }
            let address = emit.b.ins().iadd(p[2], index);
            let byte = emit.b.ins().ireduce(types::I8, byte);
            emit.b.ins().store(MemFlags::new(), byte, address, 0);
            let index = emit.b.ins().iadd_imm(index, 1);
            emit.b.ins().jump(head, &[index.into()]);
            emit.b.switch_to_block(done);
            Vec::new()
        } else if is_scan {
            let f = &program.functions[name.as_ref().unwrap()];
            let state_types = f.inputs.iter().flat_map(types_for).collect::<Vec<_>>();
            let mut state = Vec::new();
            for (i, kind) in state_types.iter().enumerate() {
                let bits =
                    emit.b
                        .ins()
                        .load(types::I64, MemFlags::new(), arguments, (i * 8) as i32);
                state.push(emit.decoded_scalar(bits, *kind));
            }
            let domains = &state_domains[name.as_ref().unwrap()];
            for (value, bound) in state.iter().zip(domains) {
                if let Some(max) = bound {
                    let valid =
                        emit.b
                            .ins()
                            .icmp_imm(IntCC::UnsignedLessThanOrEqual, *value, *max as i64);
                    emit.guard(valid, 4);
                    emit.facts.insert(*value, *max);
                }
            }
            let default_unroll = if state_types.len() <= 4
                && !nested_loop(&f.output, &program, &mut std::collections::BTreeSet::new())
            {
                4
            } else {
                1
            };
            let unroll = std::env::var("MECH_CONTROL_SCAN_UNROLL")
                .ok()
                .map(|v| v.parse::<usize>())
                .transpose()
                .map_err(|_| "scan unroll must be 1, 2, 4 or 8")?
                .unwrap_or(default_unroll);
            if ![1, 2, 4, 8].contains(&unroll) {
                return Err("scan unroll must be 1, 2, 4 or 8".into());
            }
            let zero = emit.int(0);
            let mut carried = vec![zero];
            carried.extend(state);
            // Unroll a pure stateful transducer to expose independent output
            // calculations around its recurrence. The scalar tail preserves
            // every step and handles all lengths, including zero.
            for factor in if unroll == 1 {
                vec![1]
            } else {
                vec![unroll, 1]
            } {
                let head = emit.b.create_block();
                let body = emit.b.create_block();
                let done = emit.b.create_block();
                for block in [head, done] {
                    emit.b.append_block_param(block, types::I64);
                    for kind in &state_types {
                        emit.b.append_block_param(block, *kind);
                    }
                }
                emit.b.ins().jump(
                    head,
                    &carried.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                emit.b.switch_to_block(head);
                let values = emit.b.block_params(head).to_vec();
                for (value, bound) in values[1..].iter().zip(domains) {
                    if let Some(max) = bound {
                        emit.facts.insert(*value, *max);
                    }
                }
                let remaining = emit.b.ins().isub(p[0], values[0]);
                let valid = emit.b.ins().icmp_imm(
                    IntCC::UnsignedGreaterThanOrEqual,
                    remaining,
                    factor as i64,
                );
                emit.b.ins().brif(
                    valid,
                    body,
                    &[],
                    done,
                    &values.iter().copied().map(Into::into).collect::<Vec<_>>(),
                );
                emit.b.switch_to_block(body);
                let mut state = values[1..].to_vec();
                for offset in 0..factor {
                    let next = emit.call(name.as_ref().unwrap(), &state)?;
                    let byte = *next.last().ok_or("scan byte output")?;
                    let maximum = if scan_stage == 2 {
                        u32::MAX as u64
                    } else {
                        255
                    };
                    if !emit.facts.get(&byte).is_some_and(|&max| max <= maximum) {
                        let valid = emit.b.ins().icmp_imm(
                            IntCC::UnsignedLessThanOrEqual,
                            byte,
                            maximum as i64,
                        );
                        emit.guard(valid, 4);
                    }
                    let index = emit.b.ins().iadd_imm(values[0], offset as i64);
                    let (offset, kind) = if scan_stage == 2 {
                        (emit.b.ins().ishl_imm(index, 2), types::I32)
                    } else {
                        (index, types::I8)
                    };
                    let address = emit.b.ins().iadd(p[2], offset);
                    let byte = emit.b.ins().ireduce(kind, byte);
                    emit.b.ins().store(MemFlags::new(), byte, address, 0);
                    state = next[..state_types.len()].to_vec();
                }
                let index = emit.b.ins().iadd_imm(values[0], factor as i64);
                let mut arguments = vec![index];
                arguments.extend(state);
                emit.b.ins().jump(
                    head,
                    &arguments
                        .iter()
                        .copied()
                        .map(Into::into)
                        .collect::<Vec<_>>(),
                );
                emit.b.switch_to_block(done);
                carried = emit.b.block_params(done).to_vec();
            }
            let state = carried[1..].to_vec();
            for (i, value) in state.iter().enumerate() {
                let encoded = emit.encoded_scalar(*value);
                emit.b
                    .ins()
                    .store(MemFlags::new(), encoded, p[3], (i * 8) as i32);
            }
            Vec::new()
        } else if let Some(name) = &name {
            let f = &program.functions[name];
            let mut args = vec![p[0], p[1]];
            for i in 2..f.inputs.len() {
                args.push(emit.b.ins().load(
                    types::I64,
                    MemFlags::new(),
                    arguments,
                    ((i - 2) * 8) as i32,
                ));
            }
            emit.call(name, &args)?
        } else {
            let mut memo = BTreeMap::new();
            for e in &program.effects {
                emit.expression(e, &[], &mut memo)?;
            }
            emit.expression(&program.output, &[], &mut memo)?
        };
        if !register {
            for (i, v) in values.iter().enumerate() {
                emit.b.ins().store(
                    MemFlags::new(),
                    *v,
                    output,
                    (i * if is_function { 16 } else { 8 }) as i32,
                );
            }
        }
        let zero = emit.int(0);
        if register {
            emit.b.ins().return_(&[zero, values[0]]);
        } else {
            emit.b.ins().return_(&[zero]);
        }
        if let Some(error) = emit.error_block {
            emit.b.switch_to_block(error);
            let status = emit.b.block_params(error)[0];
            emit.failure(status);
        }
        drop(emit);
        b.seal_all_blocks();
        b.finalize();
        module
            .define_function(id, &mut ctx)
            .map_err(|e| format!("native control codegen: {e}"))?;
    }
    let mut descriptor = serde_json::json!({"arguments":program.argument_count,"format":program.format,"outputs":types_for(&program.output.kind).len(),"simd":simd,"dynamic_byte_index_lowering":if !simd || stack_index {"stack"} else {"vector"}});
    if program.computation.entry.coeffects.big_integers {
        descriptor["big_integers"] = true.into();
    }
    if !program.effects.is_empty() {
        descriptor["stream_output"] = true.into();
        descriptor["scan_cache"] =
            (std::env::var("MECH_CONTROL_SCAN_CACHE").as_deref() != Ok("0")).into();
        descriptor["scan_pipeline_policy"] = pipeline_policy.into();
        descriptor["pipelined_scan_functions"] = pipeline_callbacks
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into();
    }
    Ok((main, descriptor))
}

/// The JIT owns its code and uses the same definition pass as object emission.
pub struct JitProgram {
    _module: cranelift_jit::JITModule,
    entry: unsafe extern "C" fn(*const u64, *mut u64) -> u64,
    descriptor: serde_json::Value,
}
impl JitProgram {
    pub fn compile(source: &str, simd: bool) -> Result<Self, String> {
        Self::from_program(
            mech_engine::native_sequence::control::compile(
                source,
                mech_stdlib::native_control_catalog(),
            )?,
            simd,
        )
    }
    pub fn from_program(program: Program, simd: bool) -> Result<Self, String> {
        let mut builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("is_pic", "true")],
            default_libcall_names(),
        )
        .map_err(|e| e.to_string())?;
        super::runtime::register_symbols(&mut builder);
        let mut module = cranelift_jit::JITModule::new(builder);
        let (function, descriptor) = define_program(program, &mut module, simd)?;
        module.finalize_definitions().map_err(|e| e.to_string())?;
        // SAFETY: define emits exactly this ABI; the owning module outlives calls.
        let entry = unsafe {
            std::mem::transmute::<*const u8, unsafe extern "C" fn(*const u64, *mut u64) -> u64>(
                module.get_finalized_function(function),
            )
        };
        Ok(Self {
            _module: module,
            entry,
            descriptor,
        })
    }
    pub fn run(&self, args: &[u64]) -> Result<(), String> {
        super::runtime::execute(&self.descriptor, args, |args, output| {
            // SAFETY: execute validates descriptor lengths; storage stays live.
            let status = unsafe { (self.entry)(args.as_ptr(), output.as_mut_ptr()) };
            if status == 0 {
                Ok(())
            } else {
                Err(format!("native control error {status}"))
            }
        })
    }
}
