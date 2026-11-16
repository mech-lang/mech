//! A cold refinement of an existing conservative append-capacity guard.
//! The certificate follows only a bounded pure prefix of the exact frontier.
//! Its leaves retain the independently derived worst-case suffix count. Reads
//! are restricted to immutable admitted views and checked before loading.
//! Any unknown value, overflow or invalid read retains the original fallback.
//! Preview expressions never write source homes, run callbacks, or debit work.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeProgramFacts,
    NativeScalarOp as Op, NativeTailLoop,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const MAX_FRONTIER: usize = 4096;
const MAX_PREFIX: usize = 64;
const MAX_CHOICES: usize = 12;
type Expr = Arc<Expression>;
#[derive(Clone)]
enum Expression {
    Word(usize),
    Constant(u64),
    Read(usize, Expr),
    Primitive(Op, Vec<Expr>),
    Equal(Expr, Expr),
}
enum Decision {
    Count(usize),
    Choose(Expr, Box<Decision>, Box<Decision>),
}
pub(super) struct Certificate {
    decision: Decision,
}

fn suffix(
    code: CodePlan<'_>,
    proof: &Proof,
    headers: &BTreeSet<usize>,
    exit: usize,
    array: usize,
    site: usize,
    active: &mut BTreeSet<usize>,
    limits: &mut BTreeMap<usize, usize>,
) -> Option<usize> {
    if site == exit || headers.contains(&site) {
        return Some(0);
    }
    if let Some(n) = limits.get(&site) {
        return Some(*n);
    }
    if !proof.sites.contains(&site) || !active.insert(site) || active.len() > MAX_FRONTIER {
        return None;
    }
    let mut count = 0;
    for target in NativeProgramFacts::control_successors(code.blocks()[site].terminator())
        .into_iter()
        .flatten()
    {
        count = count.max(suffix(
            code,
            proof,
            headers,
            exit,
            array,
            target as usize,
            active,
            limits,
        )?);
    }
    count = count.checked_add(proof.action(site).map_or(0, |op| op.maximum_append(array)))?;
    active.remove(&site);
    limits.insert(site, count);
    Some(count)
}
fn allowed(op: Op) -> bool {
    matches!(
        op,
        Op::U64Add
            | Op::U64Sub
            | Op::U64Mul
            | Op::U64Eq
            | Op::U64Ne
            | Op::U64Lt
            | Op::U64Le
            | Op::U64Gt
            | Op::U64Ge
            | Op::BoolNot
            | Op::BoolAnd
            | Op::BoolOr
            | Op::BoolXor
    )
}
pub(super) fn certificate(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    tail: &NativeTailLoop,
    tree: Option<&super::super::prepare::numeric_tree::Proof>,
    header: usize,
    array: usize,
    maximum: usize,
) -> Option<Certificate> {
    if proof.sites.len() > MAX_FRONTIER || code.constant_count() > MAX_FRONTIER {
        return None;
    }
    let (headers, exit, body, carriers) = if let Some(tree) = tree {
        let node = tree
            .nodes
            .iter()
            .position(|n| n.tail.header() as usize == header)?;
        let mut carriers = Vec::new();
        let mut ancestor = Some(node);
        while let Some(n) = ancestor {
            carriers.push(tree.nodes[n].tail.carrier().get());
            ancestor = tree.nodes[n].parent;
        }
        (
            tree.nodes
                .iter()
                .map(|n| n.tail.header() as usize)
                .collect::<BTreeSet<_>>(),
            tree.nodes[0].tail.exit() as usize,
            tree.nodes[node].tail.body() as usize,
            carriers,
        )
    } else {
        // Standalone leaves have a whole-visit sum rather than a maximum-path
        // bound. Phases without an explicit tree retain their existing route.
        if header != tail.header() as usize || proof.phase.is_some() {
            return None;
        }
        (
            BTreeSet::from([header]),
            tail.exit() as usize,
            tail.body() as usize,
            vec![tail.carrier().get()],
        )
    };
    let mut limits = BTreeMap::new();
    if suffix(
        code,
        proof,
        &headers,
        exit,
        array,
        body,
        &mut BTreeSet::new(),
        &mut limits,
    )? > maximum
    {
        return None;
    }
    let mut env = BTreeMap::new();
    for i in 0..code.constant_count() {
        let (value, bits) = code.constant(i).ok()?;
        if let Some(bits) = bits {
            env.insert(value.get() as usize, Arc::new(Expression::Constant(bits)));
        }
    }
    // Only this activation's current carrier and its active ancestors are
    // available. Internal/next frame lanes are never assumed initialized.
    for carrier in carriers {
        let frame = proof.frame(carrier)?;
        for word in frame.scalar_slots() {
            env.insert(shadow + word, Arc::new(Expression::Word(shadow + word)));
        }
    }
    for binding in &proof.scalar_captures {
        env.insert(
            shadow + binding.word,
            Arc::new(Expression::Word(shadow + binding.word)),
        );
    }
    struct Build<'a> {
        code: CodePlan<'a>,
        proof: &'a Proof,
        shadow: usize,
        headers: &'a BTreeSet<usize>,
        exit: usize,
        limits: &'a BTreeMap<usize, usize>,
        choices: usize,
    }
    impl Build<'_> {
        fn walk(
            &mut self,
            mut site: usize,
            mut env: BTreeMap<usize, Expr>,
            mut left: usize,
        ) -> Option<Decision> {
            loop {
                if site == self.exit || self.headers.contains(&site) {
                    return Some(Decision::Count(0));
                }
                let count = *self.limits.get(&site)?;
                if left == 0 {
                    return Some(Decision::Count(count));
                }
                left -= 1;
                match self.proof.action(site) {
                    Some(Operation::Frame) => {
                        let moves = super::numeric_words::frame_word_moves(
                            self.shadow,
                            self.code,
                            site,
                            &self.proof.frames,
                        )
                        .ok()?;
                        let next = moves
                            .assignments
                            .iter()
                            .map(|(to, from)| (*to, env.get(from).cloned()))
                            .collect::<Vec<_>>();
                        for (to, expr) in next {
                            if let Some(expr) = expr {
                                env.insert(to, expr);
                            } else {
                                env.remove(&to);
                            }
                        }
                    }
                    Some(Operation::ArrayCopy) => {}
                    Some(Operation::Read {
                        array,
                        index,
                        output,
                    }) => {
                        let a = &self.proof.arrays[*array];
                        let expr = (!a.writable && !a.appends)
                            .then(|| env.get(&(*index as usize)).cloned())
                            .flatten()
                            .map(|e| Arc::new(Expression::Read(*array, e)));
                        if let Some(expr) = expr {
                            env.insert(*output as usize, expr);
                        } else {
                            env.remove(&(*output as usize));
                        }
                    }
                    // A length read of an appendable view cannot change in
                    // this prefix: every write/append ends the preview below.
                    Some(Operation::Length { array, output }) => {
                        env.insert(
                            *output as usize,
                            Arc::new(Expression::Word(self.shadow + self.proof.view(*array) + 1)),
                        );
                    }
                    Some(_) => return Some(Decision::Count(count)),
                    None => match self.code.blocks()[site].action() {
                        Action::Scalar(Scalar::None) => {}
                        Action::Scalar(Scalar::Copy { source, output }) => {
                            if let Some(e) = env.get(&(source.get() as usize)).cloned() {
                                env.insert(output.get() as usize, e);
                            } else {
                                env.remove(&(output.get() as usize));
                            }
                        }
                        Action::Scalar(Scalar::Primitive {
                            op,
                            operands,
                            output,
                        }) => {
                            let args = operands
                                .iter()
                                .map(|v| env.get(&(v.get() as usize)).cloned())
                                .collect::<Option<Vec<_>>>();
                            if allowed(*op)
                                && let Some(args) = args
                            {
                                env.insert(
                                    output.get() as usize,
                                    Arc::new(Expression::Primitive(*op, args)),
                                );
                            } else {
                                env.remove(&(output.get() as usize));
                            }
                        }
                        _ => return Some(Decision::Count(count)),
                    },
                }
                match self.code.blocks()[site].terminator() {
                    Term::Scalar(Branch::Jump(next)) => site = next as usize,
                    Term::Scalar(branch) => {
                        let (cond, yes, no) = match branch {
                            Branch::Branch { condition, yes, no } => {
                                let Some(cond) = env.get(&(condition.get() as usize)) else {
                                    return Some(Decision::Count(count));
                                };
                                (cond.clone(), yes, no)
                            }
                            Branch::LiteralBranch {
                                left,
                                right,
                                yes,
                                no,
                            } => {
                                let (Some(a), Some(c)) = (
                                    env.get(&(left.get() as usize)),
                                    env.get(&(right.get() as usize)),
                                ) else {
                                    return Some(Decision::Count(count));
                                };
                                (Arc::new(Expression::Equal(a.clone(), c.clone())), yes, no)
                            }
                            _ => return Some(Decision::Count(count)),
                        };
                        if self.choices == MAX_CHOICES {
                            return Some(Decision::Count(count));
                        }
                        self.choices += 1;
                        let a = self.walk(yes as usize, env.clone(), left)?;
                        let b = self.walk(no as usize, env, left)?;
                        return Some(Decision::Choose(cond, Box::new(a), Box::new(b)));
                    }
                    _ => return Some(Decision::Count(count)),
                }
            }
        }
    }
    let mut build = Build {
        code,
        proof,
        shadow,
        headers: &headers,
        exit,
        limits: &limits,
        choices: 0,
    };
    let decision = build.walk(body, env, MAX_PREFIX)?;
    fn minimum(d: &Decision) -> usize {
        match d {
            Decision::Count(n) => *n,
            Decision::Choose(_, a, b) => minimum(a).min(minimum(b)),
        }
    }
    // Bound expanded emission as well as source visits: a small source DAG
    // can otherwise duplicate one symbolic expression exponentially.
    fn cost(e: &Expr, cache: &mut BTreeMap<usize, usize>) -> usize {
        let key = Arc::as_ptr(e) as usize;
        if let Some(n) = cache.get(&key) {
            return *n;
        }
        let n = match e.as_ref() {
            Expression::Read(_, e) => 1 + cost(e, cache),
            Expression::Primitive(_, args) => {
                1 + args.iter().map(|e| cost(e, cache)).sum::<usize>()
            }
            Expression::Equal(a, b) => 1 + cost(a, cache) + cost(b, cache),
            _ => 1,
        }
        .min(MAX_FRONTIER + 1);
        cache.insert(key, n);
        n
    }
    fn total(d: &Decision, cache: &mut BTreeMap<usize, usize>) -> usize {
        match d {
            Decision::Count(_) => 1,
            Decision::Choose(e, a, b) => {
                (cost(e, cache) + total(a, cache) + total(b, cache)).min(MAX_FRONTIER + 1)
            }
        }
    }
    (minimum(&decision) < maximum
        && build.choices > 0
        && total(&decision, &mut BTreeMap::new()) <= MAX_FRONTIER)
        .then_some(Certificate { decision })
}

struct Emit<'a, 'b> {
    b: &'a mut FunctionBuilder<'b>,
    words: &'a Words,
    code: CodePlan<'a>,
    proof: &'a Proof,
    shadow: usize,
    failed: Block,
}
impl Emit<'_, '_> {
    fn checked(&mut self, valid: Value) {
        let good = self.b.create_block();
        self.b.ins().brif(valid, good, &[], self.failed, &[]);
        self.b.switch_to_block(good);
    }
    fn expression(&mut self, e: &Expression) -> Result<Value, NativeCompileError> {
        Ok(match e {
            Expression::Word(word) => self.words.read(self.b, *word),
            Expression::Constant(n) => self.b.ins().iconst(types::I64, *n as i64),
            Expression::Equal(a, c) => {
                let a = self.expression(a)?;
                let c = self.expression(c)?;
                let v = self.b.ins().icmp(IntCC::Equal, a, c);
                self.b.ins().uextend(types::I64, v)
            }
            Expression::Read(array, index) => {
                let index = self.expression(index)?;
                let view = self.shadow + self.proof.view(*array);
                let length = self.words.read(self.b, view + 1);
                let ordinal = self.b.ins().iadd_imm(index, -1);
                let valid = self.b.ins().icmp(IntCC::UnsignedLessThan, ordinal, length);
                self.checked(valid);
                let a = &self.proof.arrays[*array];
                let Action::LibraryCall { template, .. } = self.code.blocks()[a.witness].action()
                else {
                    return Err(crate::native_artifact::compiler::backend(
                        "capacity preview lost immutable witness",
                    ));
                };
                let rows = self.words.read(self.b, view + 2);
                let cols = self.words.read(self.b, view + 3);
                let (address, ty) = super::numeric_words::linear_address(
                    self.b,
                    ordinal,
                    rows,
                    cols,
                    a.width,
                    template.inputs()[0].body(),
                    |b| self.words.read(b, view),
                )?;
                let value = self.b.ins().load(ty, MemFlags::new(), address, 0);
                if a.width < 8 {
                    self.b.ins().uextend(types::I64, value)
                } else {
                    value
                }
            }
            Expression::Primitive(op, args) => {
                let a = self.expression(&args[0])?;
                let c = if args.len() > 1 {
                    Some(self.expression(&args[1])?)
                } else {
                    None
                };
                match op {
                    Op::U64Add | Op::U64Sub | Op::U64Mul => {
                        let c = c.unwrap();
                        let (v, overflow) = match op {
                            Op::U64Add => self.b.ins().uadd_overflow(a, c),
                            Op::U64Sub => self.b.ins().usub_overflow(a, c),
                            _ => self.b.ins().umul_overflow(a, c),
                        };
                        let valid = self.b.ins().icmp_imm(IntCC::Equal, overflow, 0);
                        self.checked(valid);
                        v
                    }
                    Op::BoolNot => self.b.ins().bxor_imm(a, 1),
                    Op::BoolAnd => self.b.ins().band(a, c.unwrap()),
                    Op::BoolOr => self.b.ins().bor(a, c.unwrap()),
                    Op::BoolXor => self.b.ins().bxor(a, c.unwrap()),
                    _ => {
                        let cc = match op {
                            Op::U64Eq => IntCC::Equal,
                            Op::U64Ne => IntCC::NotEqual,
                            Op::U64Lt => IntCC::UnsignedLessThan,
                            Op::U64Le => IntCC::UnsignedLessThanOrEqual,
                            Op::U64Gt => IntCC::UnsignedGreaterThan,
                            Op::U64Ge => IntCC::UnsignedGreaterThanOrEqual,
                            _ => unreachable!(),
                        };
                        let v = self.b.ins().icmp(cc, a, c.unwrap());
                        self.b.ins().uextend(types::I64, v)
                    }
                }
            }
        })
    }
    fn decision(
        &mut self,
        d: &Decision,
        spare: Value,
        done: Block,
    ) -> Result<(), NativeCompileError> {
        match d {
            Decision::Count(count) => {
                let fits =
                    self.b
                        .ins()
                        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, spare, *count as i64);
                self.b.ins().jump(done, &[fits.into()]);
            }
            Decision::Choose(expr, a, c) => {
                let value = self.expression(expr)?;
                let yes = self.b.create_block();
                let no = self.b.create_block();
                self.b.ins().brif(value, yes, &[], no, &[]);
                self.b.switch_to_block(yes);
                self.decision(a, spare, done)?;
                self.b.switch_to_block(no);
                self.decision(c, spare, done)?;
            }
        }
        Ok(())
    }
}
pub(super) fn refine(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    certificate: &Certificate,
    fits: Value,
    valid: Value,
    spare: Value,
) -> Result<Value, NativeCompileError> {
    let cold = b.create_block();
    let preview = b.create_block();
    let failed = b.create_block();
    let done = b.create_block();
    b.set_cold_block(cold);
    b.set_cold_block(preview);
    b.set_cold_block(failed);
    b.append_block_param(done, types::I8);
    let yes = b.ins().iconst(types::I8, 1);
    b.ins().brif(fits, done, &[yes.into()], cold, &[]);
    b.switch_to_block(cold);
    b.ins().brif(valid, preview, &[], failed, &[]);
    b.switch_to_block(preview);
    Emit {
        b,
        words,
        code,
        proof,
        shadow,
        failed,
    }
    .decision(&certificate.decision, spare, done)?;
    b.switch_to_block(failed);
    let no = b.ins().iconst(types::I8, 0);
    b.ins().jump(done, &[no.into()]);
    b.switch_to_block(done);
    Ok(b.block_params(done)[0])
}
