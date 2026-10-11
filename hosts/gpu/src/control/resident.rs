//! Loaded, safe interpretation of the canonical closed-control graph.
//! Arithmetic, callback bodies and traversal remain instructions from source.
//! Only general IO, range scheduling, planned storage and GMP are host services.
use mech_engine::native_sequence::control::{Expr, Kind, Op, Program};
use rayon::prelude::*;
use std::collections::BTreeMap;

type Registers = Vec<usize>;
fn width(kind: &Kind) -> usize {
    match kind {
        Kind::Tuple(items) => items.iter().map(width).sum(),
        Kind::Record(items) => items.iter().map(|(_, k)| width(k)).sum(),
        Kind::Bytes(n) => *n,
        Kind::ReadBytes | Kind::Floats => 2,
        Kind::Type(_) | Kind::Text => 0,
        _ => 1,
    }
}
#[derive(Clone, Copy)]
enum Scalar {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Max,
    FAdd,
    FSub,
    FMul,
    FDiv,
    FEq,
    FNe,
    FLt,
    FLe,
    FGt,
    FGe,
    ToFloat,
    Shift,
    Factorial,
}
enum Instruction {
    Scalar(Scalar, usize, usize, usize),
    Copy(Vec<(usize, usize)>),
    Select(usize, Registers, Registers, Registers),
    Argument(usize, usize),
    Bounded(usize, usize, usize, usize),
    Fail(String),
    Positive(usize),
    Get(Registers, usize, usize),
    TableGet(usize, usize, usize),
    Set(Registers, usize, usize, Registers),
    Prefix(Registers, usize, Option<usize>, Registers),
    Jump(usize),
    JumpZero(usize, usize),
    Call(usize, Registers, Registers),
    Heap(u64, Registers, Registers, usize, Kind),
    BigintFrame(u64, Registers),
    Arena(Option<usize>, usize, Kind, usize, Registers, Registers),
    Print(String, Registers),
    Write(String),
    Scan(usize, Registers, usize, usize, String, usize, Registers),
    Parallel(
        usize,
        usize,
        usize,
        usize,
        Registers,
        Vec<String>,
        Registers,
    ),
}
struct Tape {
    initial: Vec<u64>,
    inputs: Vec<Registers>,
    code: Vec<Instruction>,
    output: Registers,
}

/// Compilation builds an instruction tape, never executable machine code.
pub struct ResidentProgram {
    tapes: Vec<Tape>,
    tables: Vec<Vec<u64>>,
    descriptor: serde_json::Value,
}
struct Builder<'a> {
    program: &'a Program,
    functions: &'a BTreeMap<String, usize>,
    tables: &'a mut Vec<Vec<u64>>,
    initial: Vec<u64>,
    code: Vec<Instruction>,
    active: Vec<String>,
    locals: BTreeMap<usize, Registers>,
    bigint_values: bool,
}
impl Builder<'_> {
    fn bigint_frame(&mut self, action: u64, kind: &Kind, values: &[usize]) {
        if self.bigint_values {
            let roots = mech_engine::native_sequence::control::bigint_value_lanes(kind, false)
                .iter().map(|&lane| values[lane]).collect();
            self.code.push(Instruction::BigintFrame(action, roots));
        }
    }
    fn reserve(&mut self, n: usize) -> Registers {
        let start = self.initial.len();
        self.initial.resize(start + n, 0);
        (start..start + n).collect()
    }
    fn constant(&mut self, n: u64) -> usize {
        let r = self.reserve(1)[0];
        self.initial[r] = n;
        r
    }
    fn copy(&mut self, source: &[usize], destination: &[usize]) {
        let copies = source
            .iter()
            .zip(destination)
            .filter(|(a, b)| a != b)
            .map(|(&a, &b)| (a, b))
            .collect::<Vec<_>>();
        if copies
            .iter()
            .any(|(a, _)| copies.iter().any(|(_, d)| a == d))
        {
            let temporary = self.reserve(copies.len());
            self.code.push(Instruction::Copy(
                copies
                    .iter()
                    .zip(&temporary)
                    .map(|(&(a, _), &t)| (a, t))
                    .collect(),
            ));
            self.code.push(Instruction::Copy(
                copies
                    .iter()
                    .zip(&temporary)
                    .map(|(&(_, d), &t)| (t, d))
                    .collect(),
            ));
        } else if !copies.is_empty() {
            self.code.push(Instruction::Copy(copies));
        }
    }
    fn call(&mut self, name: &str, args: Registers) -> Result<Registers, String> {
        let function = &self.program.functions[name];
        if self.active.iter().any(|s| s == name) {
            let result = self.reserve(width(&function.output.kind));
            self.code.push(Instruction::Call(
                self.functions[name],
                args,
                result.clone(),
            ));
            return Ok(result);
        }
        // General inlining removes interpreter call traffic for nonrecursive
        // functions. Recursive calls retain explicit checked resident frames.
        self.active.push(name.into());
        let caller_bigint_values = self.bigint_values;
        // A callee without transitive value creation returns only borrowed
        // handles; omitting its empty frame leaves their caller ownership intact.
        self.bigint_values &= self.program.computation.functions[name].big_integer_values;
        let mut offset = 0;
        let params = function
            .inputs
            .iter()
            .map(|kind| {
                let n = width(kind);
                let r = args[offset..offset + n].to_vec();
                offset += n;
                r
            })
            .collect::<Vec<_>>();
        let result = (|| {
            self.bigint_frame(40, &Kind::Integer, &[]);
            let values = self.expr(&function.output, &params, &mut BTreeMap::new())?;
            self.bigint_frame(41, &function.output.kind, &values);
            Ok(values)
        })();
        self.bigint_values = caller_bigint_values;
        self.active.pop();
        result
    }
    fn expr(
        &mut self,
        e: &Expr,
        params: &[Registers],
        memo: &mut BTreeMap<Expr, Registers>,
    ) -> Result<Registers, String> {
        if let Some(v) = memo.get(e) {
            return Ok(v.clone());
        }
        let value = match &e.op {
            Op::Integer(n) | Op::Float(n) => vec![self.constant(*n)],
            Op::Boolean(b) => vec![self.constant(u64::from(*b))],
            Op::Text(_) | Op::Type => vec![],
            Op::ReadBytes(bytes) => {
                let id = self.tables.len();
                self.tables
                    .push(bytes.iter().map(|&b| u64::from(b)).collect());
                vec![self.constant(id as u64), self.constant(bytes.len() as u64)]
            }
            Op::Floats(values) => {
                let id = self.tables.len();
                self.tables.push(values.clone());
                vec![self.constant(id as u64), self.constant(values.len() as u64)]
            }
            Op::Parameter(i) => params[*i].clone(),
            Op::Local(i) => self.locals.get(i).ok_or("unbound resident local")?.clone(),
            Op::Tuple(items) => {
                let mut result = vec![];
                for item in items {
                    result.extend(self.expr(item, params, memo)?);
                }
                result
            }
            Op::Field(value, index) => {
                let values = self.expr(value, params, memo)?;
                let kinds = match &value.kind {
                    Kind::Tuple(items) => items.clone(),
                    Kind::Record(items) => items.iter().map(|(_, k)| k.clone()).collect(),
                    _ => return Err("resident projection requires tuple or record".into()),
                };
                let start: usize = kinds[..*index].iter().map(width).sum();
                values[start..start + width(&e.kind)].to_vec()
            }
            Op::Sequence { effects, output } => {
                for effect in effects {
                    self.expr(effect, params, memo)?;
                }
                self.expr(output, params, memo)?
            }
            Op::Call(name, arguments)
            | Op::Invoke {
                name, arguments, ..
            } => {
                let mut args = vec![];
                for a in arguments {
                    args.extend(self.expr(a, params, memo)?);
                }
                self.call(name, args)?
            }
            Op::Loop {
                initial,
                predicate,
                step,
                ..
            } => {
                let seed = self.expr(initial, params, memo)?;
                self.bigint_frame(40, &Kind::Integer, &[]);
                let carried = self.reserve(seed.len());
                self.copy(&seed, &carried);
                let head = self.code.len();
                let condition = self.call(predicate, carried.clone())?[0];
                let exit = self.code.len();
                self.code.push(Instruction::JumpZero(condition, 0));
                let next = self.call(step, carried.clone())?;
                self.bigint_frame(42, &e.kind, &next);
                self.copy(&next, &carried);
                self.code.push(Instruction::Jump(head));
                let end = self.code.len();
                self.code[exit] = Instruction::JumpZero(condition, end);
                self.bigint_frame(41, &e.kind, &carried);
                carried
            }
            Op::Branch {
                condition,
                yes,
                no,
                state,
                ..
            } => {
                let c = self.expr(condition, params, memo)?[0];
                let args = self.expr(state, params, memo)?;
                let output = self.reserve(width(&e.kind));
                let branch = self.code.len();
                self.code.push(Instruction::JumpZero(c, 0));
                let value = self.call(yes, args.clone())?;
                self.copy(&value, &output);
                let done = self.code.len();
                self.code.push(Instruction::Jump(0));
                let otherwise = self.code.len();
                self.code[branch] = Instruction::JumpZero(c, otherwise);
                let value = self.call(no, args)?;
                self.copy(&value, &output);
                let end = self.code.len();
                self.code[done] = Instruction::Jump(end);
                output
            }
            Op::Match {
                scrutinee, arms, ..
            } => {
                self.expr(scrutinee, params, memo)?;
                let output = self.reserve(width(&e.kind));
                let mut jumps = vec![];
                let mut exhaustive = false;
                for (condition, body) in arms {
                    let mut local_memo = memo.clone();
                    let c = self.expr(condition, params, &mut local_memo)?[0];
                    let branch = self.code.len();
                    self.code.push(Instruction::JumpZero(c, 0));
                    let value = self.expr(body, params, &mut local_memo)?;
                    self.copy(&value, &output);
                    jumps.push(self.code.len());
                    self.code.push(Instruction::Jump(0));
                    let next = self.code.len();
                    self.code[branch] = Instruction::JumpZero(c, next);
                    if matches!(condition.op, Op::Boolean(true)) {
                        exhaustive = true;
                        break;
                    }
                }
                if !exhaustive {
                    self.code
                        .push(Instruction::Fail("no matching pattern arm".into()));
                }
                let end = self.code.len();
                for i in jumps {
                    self.code[i] = Instruction::Jump(end);
                }
                output
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
                let first = self.expr(start, params, memo)?[0];
                let step = self.expr(step, params, memo)?[0];
                let end = self.expr(end, params, memo)?[0];
                self.code.push(Instruction::Positive(step));
                let i = self.reserve(1)[0];
                self.copy(&[first], &[i]);
                let sum = self.reserve(1)[0];
                let zero = self.constant(0);
                self.copy(&[zero], &[sum]);
                let head = self.code.len();
                let cond = self.reserve(1)[0];
                self.code.push(Instruction::Scalar(
                    if *inclusive { Scalar::Le } else { Scalar::Lt },
                    i,
                    end,
                    cond,
                ));
                let exit = self.code.len();
                self.code.push(Instruction::JumpZero(cond, 0));
                self.locals.insert(*index, vec![i]);
                self.bigint_frame(40, &Kind::Integer, &[]);
                let value = self.expr(body, params, &mut memo.clone())?[0];
                self.bigint_frame(41, &Kind::Integer, &[]);
                self.locals.remove(index);
                self.code.push(Instruction::Scalar(
                    if e.kind == Kind::Float {
                        Scalar::FAdd
                    } else {
                        Scalar::Add
                    },
                    sum,
                    value,
                    sum,
                ));
                let remaining = self.reserve(1)[0];
                self.code
                    .push(Instruction::Scalar(Scalar::Sub, end, i, remaining));
                let another = self.reserve(1)[0];
                self.code.push(Instruction::Scalar(
                    if *inclusive { Scalar::Le } else { Scalar::Lt },
                    step,
                    remaining,
                    another,
                ));
                let finish = self.code.len();
                self.code.push(Instruction::JumpZero(another, 0));
                self.code.push(Instruction::Scalar(Scalar::Add, i, step, i));
                self.code.push(Instruction::Jump(head));
                let done = self.code.len();
                self.code[exit] = Instruction::JumpZero(cond, done);
                self.code[finish] = Instruction::JumpZero(another, done);
                vec![sum]
            }
            Op::Heap {
                action,
                arguments,
                arity,
                layout,
                ..
            } => {
                let mut args = vec![];
                for a in arguments {
                    args.extend(self.expr(a, params, memo)?);
                }
                let output = self.reserve(width(&e.kind));
                self.code.push(Instruction::Heap(
                    *action,
                    args,
                    output.clone(),
                    *arity,
                    layout.clone().unwrap_or(Kind::Integer),
                ));
                output
            }
            Op::Arena {
                capacity,
                arity,
                layout,
                function,
                arguments,
                ..
            } => {
                let capacity = capacity
                    .as_ref()
                    .map(|e| self.expr(e, params, memo).map(|r| r[0]))
                    .transpose()?;
                let args = self.expr(arguments, params, memo)?;
                let output = self.reserve(width(&e.kind));
                self.code.push(Instruction::Arena(
                    capacity,
                    *arity,
                    layout.clone(),
                    self.functions[function],
                    args,
                    output.clone(),
                ));
                output
            }
            Op::Print { format, values, .. } => {
                let mut args = vec![];
                for a in values {
                    args.extend(self.expr(a, params, memo)?);
                }
                self.code.push(Instruction::Print(format.clone(), args));
                vec![self.constant(0)]
            }
            Op::Write { text, .. } => {
                self.code.push(Instruction::Write(text.clone()));
                vec![self.constant(0)]
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
                let count = self.expr(count, params, memo)?[0];
                let state = self.expr(state, params, memo)?;
                let line = self.expr(width, params, memo)?[0];
                let buffer = self.expr(buffer, params, memo)?[0];
                let result = self.reserve(width_of_expr(e));
                self.code.push(Instruction::Scan(
                    count,
                    state,
                    self.functions[function],
                    line,
                    separator.clone(),
                    buffer,
                    result.clone(),
                ));
                result
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
                let start = self.expr(start, params, memo)?[0];
                let end = self.expr(end, params, memo)?[0];
                let grain = self.expr(grain, params, memo)?[0];
                let args = self.expr(arguments, params, memo)?;
                let result = self.reserve(width(&e.kind));
                self.code.push(Instruction::Parallel(
                    start,
                    end,
                    grain,
                    self.functions[function],
                    args,
                    reductions.clone(),
                    result.clone(),
                ));
                result
            }
            Op::Primitive(name, arguments) => {
                let mut args = vec![];
                for a in arguments {
                    args.push(self.expr(a, params, memo)?);
                }
                let output = self.reserve(width(&e.kind));
                let a = args.first().and_then(|v| v.first()).copied().unwrap_or(0);
                let b = args.get(1).and_then(|v| v.first()).copied().unwrap_or(a);
                let floating = arguments.first().is_some_and(|a| a.kind == Kind::Float);
                let scalar = match (name.as_str(), floating) {
                    ("+", false) => Some(Scalar::Add),
                    ("-", false) => Some(Scalar::Sub),
                    ("*", false) => Some(Scalar::Mul),
                    ("/", false) => Some(Scalar::Div),
                    ("==", false) => Some(Scalar::Eq),
                    ("!=", false) => Some(Scalar::Ne),
                    ("<", false) => Some(Scalar::Lt),
                    ("<=", false) => Some(Scalar::Le),
                    (">", false) => Some(Scalar::Gt),
                    (">=", false) => Some(Scalar::Ge),
                    ("+", true) => Some(Scalar::FAdd),
                    ("-", true) => Some(Scalar::FSub),
                    ("*", true) => Some(Scalar::FMul),
                    ("/", true) => Some(Scalar::FDiv),
                    ("==", true) => Some(Scalar::FEq),
                    ("!=", true) => Some(Scalar::FNe),
                    ("<", true) => Some(Scalar::FLt),
                    ("<=", true) => Some(Scalar::FLe),
                    (">", true) => Some(Scalar::FGt),
                    (">=", true) => Some(Scalar::FGe),
                    ("number/mod", _) => Some(Scalar::Mod),
                    ("number/max", _) => Some(Scalar::Max),
                    ("&&", _) => Some(Scalar::And),
                    ("||", _) => Some(Scalar::Or),
                    ("number/to-f64", _) => Some(Scalar::ToFloat),
                    ("number/shift-left", _) => Some(Scalar::Shift),
                    ("number/factorial", _) => Some(Scalar::Factorial),
                    _ => None,
                };
                if let Some(op) = scalar {
                    self.code.push(Instruction::Scalar(op, a, b, output[0]));
                } else {
                    match name.as_str() {
                        "number/bounded" => {
                            self.code
                                .push(Instruction::Bounded(a, args[1][0], args[2][0], output[0]));
                        }
                        "io/argument-u64" => {
                            let Op::Integer(index) = arguments[0].op else {
                                return Err("resident argument ordinal".into());
                            };
                            self.code
                                .push(Instruction::Argument(index as usize, output[0]));
                        }
                        "sequence/length" => self.copy(&[args[0][1]], &output),
                        "sequence/iota-bytes" | "sequence/zero-bytes" => {
                            for (i, &r) in output.iter().enumerate() {
                                self.initial[r] = if name == "sequence/iota-bytes" {
                                    i as u64
                                } else {
                                    0
                                };
                            }
                        }
                        "sequence/get" => {
                            if matches!(arguments[0].kind, Kind::ReadBytes | Kind::Floats) {
                                self.code.push(Instruction::TableGet(a, b, output[0]));
                            } else {
                                self.code
                                    .push(Instruction::Get(args[0].clone(), b, output[0]));
                            }
                        }
                        "sequence/set" => self.code.push(Instruction::Set(
                            args[0].clone(),
                            b,
                            args[2][0],
                            output.clone(),
                        )),
                        "sequence/reverse-prefix" | "sequence/rotate-prefix" => {
                            self.code.push(Instruction::Prefix(
                                args[0].clone(),
                                b,
                                if name == "sequence/rotate-prefix" {
                                    Some(args[2][0])
                                } else {
                                    None
                                },
                                output.clone(),
                            ))
                        }
                        "control/select" => self.code.push(Instruction::Select(
                            a,
                            args[1].clone(),
                            args[2].clone(),
                            output.clone(),
                        )),
                        _ => return Err(format!("resident primitive lowering missing: {name}")),
                    }
                }
                output
            }
        };
        memo.insert(e.clone(), value.clone());
        Ok(value)
    }
}
fn width_of_expr(e: &Expr) -> usize {
    width(&e.kind)
}

impl ResidentProgram {
    pub fn compile(source: &str) -> Result<Self, String> {
        let program = mech_engine::native_sequence::control::compile(
            source,
            mech_stdlib::native_control_catalog(),
        )?;
        Self::lower(&program)
    }
    pub fn lower(program: &Program) -> Result<Self, String> {
        let functions = program
            .functions
            .keys()
            .enumerate()
            .map(|(i, n)| (n.clone(), i + 1))
            .collect::<BTreeMap<_, _>>();
        let mut tables = vec![];
        let mut tapes = vec![];
        for (name, inputs, output, effects) in
            std::iter::once(("", vec![], &program.output, program.effects.as_slice())).chain(
                program
                    .functions
                    .iter()
                    .map(|(n, f)| (n.as_str(), f.inputs.clone(), &f.output, &[][..])),
            )
        {
            let mut b = Builder {
                program,
                functions: &functions,
                tables: &mut tables,
                initial: vec![],
                code: vec![],
                active: if name.is_empty() {
                    vec![]
                } else {
                    vec![name.into()]
                },
                locals: BTreeMap::new(),
                bigint_values: if name.is_empty() { program.computation.entry.big_integer_values }
                    else { program.computation.functions[name].big_integer_values },
            };
            let params = inputs
                .iter()
                .map(|kind| b.reserve(width(kind)))
                .collect::<Vec<_>>();
            let mut memo = BTreeMap::new();
            b.bigint_frame(40, &Kind::Integer, &[]);
            for e in effects {
                b.expr(e, &params, &mut memo)?;
            }
            let result = b.expr(output, &params, &mut memo)?;
            b.bigint_frame(41, &output.kind, &result);
            tapes.push(Tape {
                initial: b.initial,
                inputs: params,
                code: b.code,
                output: result,
            });
        }
        let descriptor = serde_json::json!({"arguments":program.argument_count,"outputs":width(&program.output.kind),"format":program.format,"big_integers":program.computation.entry.coeffects.big_integers,"stream_output":!program.effects.is_empty()});
        Ok(Self {
            tapes,
            tables,
            descriptor,
        })
    }
    pub fn run(&self, args: &[u64]) -> Result<(), String> {
        super::runtime::execute(&self.descriptor, args, |args, output| {
            let mut vm = Machine::new(self, args);
            vm.root(0, &[], output, None)
        })
    }
}
struct Arena<'a> {
    cells: &'a mut [u64],
    cursor: usize,
    arity: usize,
    layout: Kind,
}
struct Machine<'a> {
    program: &'a ResidentProgram,
    args: &'a [u64],
    cells: Vec<u64>,
    top: usize,
    depth: usize,
}
impl<'a> Machine<'a> {
    fn new(program: &'a ResidentProgram, args: &'a [u64]) -> Self {
        Self {
            program,
            args,
            cells: vec![],
            top: 0,
            depth: 0,
        }
    }
    fn root(
        &mut self,
        callee: usize,
        args: &[u64],
        output: &mut [u64],
        arena: Option<&mut Arena<'_>>,
    ) -> Result<(), String> {
        let input = args.iter().enumerate().map(|(i, _)| i).collect::<Vec<_>>();
        let out = (args.len()..args.len() + output.len()).collect::<Vec<_>>();
        self.cells.resize(args.len() + output.len(), 0);
        self.cells[..args.len()].copy_from_slice(args);
        self.top = self.cells.len();
        self.call(callee, 0, &input, &out, arena)?;
        for (dst, &r) in output.iter_mut().zip(&out) {
            *dst = self.cells[r];
        }
        Ok(())
    }
    fn call(
        &mut self,
        callee: usize,
        parent: usize,
        args: &[usize],
        output: &[usize],
        mut arena: Option<&mut Arena<'_>>,
    ) -> Result<(), String> {
        if self.depth >= 128 {
            return Err("resident call depth exceeds 128".into());
        }
        let tape = &self.program.tapes[callee];
        let base = self.top;
        self.top += tape.initial.len();
        self.depth += 1;
        self.cells.resize(self.cells.len().max(self.top), 0);
        self.cells[base..self.top].copy_from_slice(&tape.initial);
        for (&src, &dst) in args.iter().zip(tape.inputs.iter().flatten()) {
            self.cells[base + dst] = self.cells[parent + src];
        }
        let mut pc = 0;
        while pc < tape.code.len() {
            let instruction = &tape.code[pc];
            pc += 1;
            match instruction {
                Instruction::Scalar(op, a, b, d) => {
                    let x = self.cells[base + a];
                    let y = self.cells[base + b];
                    let xf = f64::from_bits(x);
                    let yf = f64::from_bits(y);
                    self.cells[base + d] = match op {
                        Scalar::Add => x.wrapping_add(y),
                        Scalar::Sub => x.wrapping_sub(y),
                        Scalar::Mul => x.wrapping_mul(y),
                        Scalar::Div => x.checked_div(y).ok_or("division by zero")?,
                        Scalar::Mod => x.checked_rem(y).ok_or("division by zero")?,
                        Scalar::Eq => u64::from(x == y),
                        Scalar::Ne => u64::from(x != y),
                        Scalar::Lt => u64::from(x < y),
                        Scalar::Le => u64::from(x <= y),
                        Scalar::Gt => u64::from(x > y),
                        Scalar::Ge => u64::from(x >= y),
                        Scalar::And => x & y,
                        Scalar::Or => x | y,
                        Scalar::Max => x.max(y),
                        Scalar::FAdd => (xf + yf).to_bits(),
                        Scalar::FSub => (xf - yf).to_bits(),
                        Scalar::FMul => (xf * yf).to_bits(),
                        Scalar::FDiv => (xf / yf).to_bits(),
                        Scalar::FEq => u64::from(xf == yf),
                        Scalar::FNe => u64::from(xf != yf),
                        Scalar::FLt => u64::from(xf < yf),
                        Scalar::FLe => u64::from(xf <= yf),
                        Scalar::FGt => u64::from(xf > yf),
                        Scalar::FGe => u64::from(xf >= yf),
                        Scalar::ToFloat => (x as f64).to_bits(),
                        Scalar::Shift => x
                            .checked_shl(u32::try_from(y).map_err(|_| "invalid shift")?)
                            .ok_or("invalid shift")?,
                        Scalar::Factorial => {
                            if x > 20 {
                                return Err("factorial domain exceeds 20".into());
                            }
                            (1..=x).product()
                        }
                    };
                }
                Instruction::Copy(pairs) => {
                    for &(src, dst) in pairs {
                        self.cells[base + dst] = self.cells[base + src];
                    }
                }
                Instruction::Select(c, a, b, dst) => {
                    let src = if self.cells[base + c] != 0 { a } else { b };
                    for (&a, &d) in src.iter().zip(dst) {
                        self.cells[base + d] = self.cells[base + a];
                    }
                }
                Instruction::Argument(index, dst) => {
                    self.cells[base + dst] = *self.args.get(index - 1).ok_or("missing argument")?;
                }
                Instruction::Bounded(a, low, high, dst) => {
                    let value = self.cells[base + a];
                    if value < self.cells[base + low] || value > self.cells[base + high] {
                        return Err("value outside bounded domain".into());
                    }
                    self.cells[base + dst] = value;
                }
                Instruction::Fail(error) => return Err(error.clone()),
                Instruction::Positive(r) => {
                    if self.cells[base + r] == 0 {
                        return Err("range step must be positive".into());
                    }
                }
                Instruction::Get(data, index, dst) => {
                    let i = usize::try_from(self.cells[base + index])
                        .ok()
                        .and_then(|i| i.checked_sub(1))
                        .ok_or("invalid byte index")?;
                    let r = *data.get(i).ok_or("byte index out of bounds")?;
                    self.cells[base + dst] = self.cells[base + r];
                }
                Instruction::TableGet(table, index, dst) => {
                    let i = usize::try_from(self.cells[base + index])
                        .ok()
                        .and_then(|i| i.checked_sub(1))
                        .ok_or("invalid table index")?;
                    let table = self
                        .program
                        .tables
                        .get(self.cells[base + table] as usize)
                        .ok_or("invalid table handle")?;
                    self.cells[base + dst] = *table.get(i).ok_or("table index out of bounds")?;
                }
                Instruction::Set(src, index, value, dst) => {
                    let i = usize::try_from(self.cells[base + index])
                        .ok()
                        .and_then(|i| i.checked_sub(1))
                        .ok_or("invalid byte index")?;
                    if i >= src.len() || self.cells[base + value] > 255 {
                        return Err("byte assignment out of bounds".into());
                    }
                    for (&a, &d) in src.iter().zip(dst) {
                        self.cells[base + d] = self.cells[base + a];
                    }
                    self.cells[base + dst[i]] = self.cells[base + value];
                }
                Instruction::Prefix(src, count, rotation, dst) => {
                    let n =
                        usize::try_from(self.cells[base + count]).map_err(|_| "invalid prefix")?;
                    if n > src.len() {
                        return Err("prefix out of bounds".into());
                    }
                    let shift = rotation.map(|r| {
                        if n == 0 {
                            0
                        } else {
                            self.cells[base + r] as usize % n
                        }
                    });
                    for (i, &d) in dst.iter().enumerate() {
                        let j = if i >= n {
                            i
                        } else if let Some(shift) = shift {
                            (i + shift) % n
                        } else {
                            n - i - 1
                        };
                        self.cells[base + d] = self.cells[base + src[j]];
                    }
                }
                Instruction::Jump(next) => pc = *next,
                Instruction::JumpZero(c, next) => {
                    if self.cells[base + c] == 0 {
                        pc = *next;
                    }
                }
                Instruction::Call(c, args, out) => {
                    self.call(*c, base, args, out, arena.as_deref_mut())?
                }
                Instruction::Heap(action, args, out, arity, layout) => {
                    if (10..=33).contains(action) {
                        let a = args.first().map_or(0, |r| self.cells[base + r]);
                        let b = args.get(1).map_or(0, |r| self.cells[base + r]);
                        let c = args.get(2).map_or(0, |r| self.cells[base + r]);
                        self.cells[base + out[0]] = super::bigint::apply(*action, a, b, c)?;
                    } else if *action == 0 {
                        self.cells[base + out[0]] = 0;
                    } else {
                        let arena = arena.as_deref_mut().ok_or("missing reference region")?;
                        if arena.arity != *arity || arena.layout != *layout {
                            return Err("reference layout mismatch".into());
                        }
                        if *action == 1 {
                            let start = arena
                                .cursor
                                .checked_mul(*arity)
                                .ok_or("reference allocation overflow")?;
                            if start + arity > arena.cells.len() {
                                return Err("reference arena exhausted".into());
                            }
                            for (i, &r) in args.iter().enumerate() {
                                arena.cells[start + i] = self.cells[base + r];
                            }
                            arena.cursor += 1;
                            self.cells[base + out[0]] = arena.cursor as u64;
                        } else {
                            let h = self.cells[base + args[0]];
                            let index = usize::try_from(h)
                                .ok()
                                .and_then(|i| i.checked_sub(1))
                                .filter(|&i| i < arena.cursor)
                                .ok_or("invalid reference")?;
                            for (i, &r) in out.iter().enumerate() {
                                self.cells[base + r] = arena.cells[index * arity + i];
                            }
                        }
                    }
                }
                Instruction::BigintFrame(action, roots) => {
                    if *action == 40 { super::bigint::apply(40, 0, 0, 0)?; }
                    else {
                        let roots = roots.iter().map(|r| self.cells[base + r]).collect::<Vec<_>>();
                        super::bigint::frame_roots(&roots, *action == 41)?;
                    }
                }
                Instruction::Arena(capacity, arity, layout, callee, args, out) => {
                    if let Some(capacity) = capacity {
                        let n = self.cells[base + capacity];
                        super::runtime::with_arena_storage(n, *arity as u64, |cells| {
                            let mut child = Arena {
                                cells,
                                cursor: 0,
                                arity: *arity,
                                layout: layout.clone(),
                            };
                            self.call(*callee, base, args, out, Some(&mut child))
                        })?;
                    } else {
                        let region = arena.as_deref_mut().ok_or("missing parent arena")?;
                        let mark = region.cursor;
                        let result = self.call(*callee, base, args, out, Some(region));
                        region.cursor = mark;
                        result?;
                    }
                }
                Instruction::Print(format, args) => {
                    let mut text = String::new();
                    let mut rest = format.as_str();
                    for &r in args {
                        let at = rest.find('{').ok_or("format mismatch")?;
                        text.push_str(&rest[..at]);
                        let value = self.cells[base + r];
                        if rest[at..].starts_with("{:i64}") {
                            text.push_str(&(value as i64).to_string());
                            rest = &rest[at + 6..];
                        } else if rest[at..].starts_with("{}") {
                            text.push_str(&value.to_string());
                            rest = &rest[at + 2..];
                        } else {
                            return Err("unsupported format".into());
                        }
                    }
                    text.push_str(rest);
                    if super::runtime::write_bytes(text.as_bytes()) != 0 {
                        return Err("output failed".into());
                    }
                }
                Instruction::Write(text) => {
                    if super::runtime::write_bytes(text.as_bytes()) != 0 {
                        return Err("output failed".into());
                    }
                }
                Instruction::Scan(count, state, callee, line, separator, buffer, out) => {
                    let count = self.cells[base + count];
                    let line = usize::try_from(self.cells[base + line])
                        .map_err(|_| "invalid line width")?;
                    let buffer = usize::try_from(self.cells[base + buffer])
                        .map_err(|_| "invalid scan buffer")?;
                    if !(1..=1048576).contains(&buffer) {
                        return Err("invalid scan buffer".into());
                    }
                    let state_start = self.top;
                    let n = state.len();
                    self.cells
                        .resize(self.cells.len().max(state_start + 2 * n + 1), 0);
                    self.top = state_start + 2 * n + 1;
                    for (i, &r) in state.iter().enumerate() {
                        self.cells[state_start + i] = self.cells[base + r];
                    }
                    let src = (0..n).collect::<Vec<_>>();
                    let dst = (n..2 * n + 1).collect::<Vec<_>>();
                    let mut bytes = Vec::with_capacity(buffer + separator.len());
                    let mut column = 0;
                    for _ in 0..count {
                        self.call(*callee, state_start, &src, &dst, arena.as_deref_mut())?;
                        let byte = self.cells[state_start + 2 * n];
                        if byte > 255 {
                            return Err("scan byte exceeds 255".into());
                        }
                        bytes.push(byte as u8);
                        for i in 0..n {
                            self.cells[state_start + i] = self.cells[state_start + n + i];
                        }
                        column += 1;
                        if line != 0 && column == line {
                            bytes.extend_from_slice(separator.as_bytes());
                            column = 0;
                        }
                        if bytes.len() >= buffer {
                            if super::runtime::write_bytes(&bytes) != 0 {
                                return Err("output failed".into());
                            }
                            bytes.clear();
                        }
                    }
                    if line != 0 && column != 0 {
                        bytes.extend_from_slice(separator.as_bytes());
                    }
                    if super::runtime::write_bytes(&bytes) != 0 {
                        return Err("output failed".into());
                    }
                    for (i, &r) in out.iter().enumerate() {
                        self.cells[base + r] = self.cells[state_start + i];
                    }
                    self.top = state_start;
                }
                Instruction::Parallel(start, end, grain, callee, args, reductions, out) => {
                    let start = self.cells[base + start];
                    let end = self.cells[base + end];
                    let grain = self.cells[base + grain];
                    if grain == 0 || start > end {
                        return Err("invalid parallel range".into());
                    }
                    let extra = args
                        .iter()
                        .map(|r| self.cells[base + r])
                        .collect::<Vec<_>>();
                    let count = (end - start).div_ceil(grain);
                    let program = self.program;
                    let arguments = self.args;
                    let results = (0..count)
                        .into_par_iter()
                        .map(|i| {
                            let a = start + i * grain;
                            let b = a.saturating_add(grain).min(end);
                            let mut input = vec![a, b];
                            input.extend(&extra);
                            let mut result = vec![0; out.len()];
                            Machine::new(program, arguments).root(
                                *callee,
                                &input,
                                &mut result,
                                None,
                            )?;
                            Ok::<_, String>(result)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    for (i, &r) in out.iter().enumerate() {
                        self.cells[base + r] = 0;
                        for result in &results {
                            self.cells[base + r] = if reductions[i] == "max" {
                                self.cells[base + r].max(result[i])
                            } else {
                                self.cells[base + r].wrapping_add(result[i])
                            };
                        }
                    }
                }
            }
        }
        for (&src, &dst) in tape.output.iter().zip(output) {
            self.cells[parent + dst] = self.cells[base + src];
        }
        self.top = base;
        self.depth -= 1;
        Ok(())
    }
}
