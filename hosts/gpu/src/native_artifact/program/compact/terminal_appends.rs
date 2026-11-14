//! A selected terminal byte append can share one borrowed-prefix acquisition.
//! Rebuild the write list from the typed root, with the ordinary entry as a
//! fallback for any current length, capacity or ownership outside the proof.
use super::{
    constants::Initial, flow::Flow, layout::Layout, owner_flow::OwnerFlow, recipes::Recipe,
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, offset, primitive},
    program::prepare::word_collections::bounds::Range,
};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, Signature, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::{
    NativeAggregateWord as Lane, NativeCollectionPrimitive as Buffer, NativeControlLoop as Loop,
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeMatchValue as Port,
    NativeProgramArtifactPlan as Plan, NativeScalarAggregateAction as Aggregate, NativeScalarKind,
    NativeScalarOp as Op,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Clone, Debug)]
enum Word {
    Constant(u64),
    Length(usize),
    Operation(Op, Rc<[Word]>, Range),
}

impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Constant(a), Self::Constant(b)) => a == b,
            (Self::Length(a), Self::Length(b)) => a == b,
            (Self::Operation(a, left, _), Self::Operation(b, right, _)) => {
                a == b && Rc::ptr_eq(left, right)
            }
            _ => false,
        }
    }
}

#[derive(Clone)]
enum LaneValue {
    Scalar(Word),
    View(View, usize),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum View {
    Borrowed(usize),
    Appended(usize),
}

#[derive(Clone, Copy)]
struct Lengths {
    port: usize,
    range: Range,
}

struct Route {
    lengths: Lengths,
    prefix: usize,
    reserve: Word,
    packed: Vec<(Word, u8)>,
    span: Option<(usize, Word)>,
    origin: usize,
}

pub(super) struct TerminalAppends {
    routes: Vec<Route>,
    inputs: usize,
}

enum TraceFailure {
    Unsupported,
    Split(usize, u64),
}

type TraceResult<T> = Result<T, TraceFailure>;

struct Trace<'a> {
    layout: &'a Layout,
    initial: &'a BTreeMap<usize, Initial>,
    words: BTreeMap<usize, LaneValue>,
    lengths: Option<Lengths>,
    byte_count: u8,
    packed: Vec<(Word, u8)>,
    span: Option<(usize, Word)>,
    prefix: Option<usize>,
    reserve: Option<Word>,
    origin: Option<usize>,
    last: Option<usize>,
}

impl Trace<'_> {
    fn lanes(&self, start: usize, count: usize) -> TraceResult<Vec<LaneValue>> {
        (start..start + count)
            .map(|slot| {
                self.words
                    .get(&slot)
                    .cloned()
                    .or_else(|| match self.initial.get(&slot) {
                        Some(Initial::Bits(bits)) => Some(LaneValue::Scalar(Word::Constant(*bits))),
                        _ => None,
                    })
                    .ok_or(TraceFailure::Unsupported)
            })
            .collect()
    }

    fn read(&self, value: Port) -> TraceResult<Vec<LaneValue>> {
        self.lanes(
            self.layout.slot(value.get()),
            self.layout.ty(value.get()).words,
        )
    }

    fn write(&mut self, value: Port, lanes: Vec<LaneValue>) {
        let start = self.layout.slot(value.get());
        self.words.extend(
            lanes
                .into_iter()
                .enumerate()
                .map(|(lane, value)| (start + lane, value)),
        );
    }

    fn scalar(&self, value: Port) -> TraceResult<Word> {
        match self.read(value)?.as_slice() {
            [LaneValue::Scalar(word)] => Ok(word.clone()),
            _ => Err(TraceFailure::Unsupported),
        }
    }

    fn view(&self, value: Port) -> TraceResult<View> {
        match self.read(value)?.as_slice() {
            lanes @ [LaneValue::View(view, 0), ..] if lanes.len() == 6 && lanes.iter().enumerate()
                .all(|(lane, word)| matches!(word, LaneValue::View(found, at) if found == view && *at == lane)) => Ok(*view),
            _ => Err(TraceFailure::Unsupported),
        }
    }

    fn write_view(&mut self, value: Port, view: View) {
        self.write(
            value,
            (0..6).map(|lane| LaneValue::View(view, lane)).collect(),
        );
    }

    fn range(&self, word: &Word) -> TraceResult<Range> {
        match word {
            Word::Constant(bits) => Ok(Range(*bits, *bits)),
            Word::Length(port) => Ok(self
                .lengths
                .filter(|bound| bound.port == *port)
                .map_or(Range(0, u64::MAX), |bound| bound.range)),
            Word::Operation(_, _, range) => Ok(*range),
        }
    }

    fn result_range(&self, op: Op, args: &[Word]) -> TraceResult<Range> {
        use Op::*;
        let a = self.range(&args[0])?;
        let z = args.get(1).map(|word| self.range(word)).transpose()?;
        let invalid = TraceFailure::Unsupported;
        let checked = |n: Option<u64>| n.ok_or(TraceFailure::Unsupported);
        Ok(match (op, z) {
            (U64BitNot, None) => Range(!a.1, !a.0),
            (U64ToU8, None) if a.1 <= u8::MAX.into() => a,
            (U64Add, Some(z)) => Range(
                checked(a.0.checked_add(z.0))?,
                checked(a.1.checked_add(z.1))?,
            ),
            (U64Sub, Some(z)) if a.0 >= z.1 => Range(a.0 - z.1, a.1 - z.0),
            (U64Mul, Some(z)) => Range(
                checked(a.0.checked_mul(z.0))?,
                checked(a.1.checked_mul(z.1))?,
            ),
            (U64Div, Some(z)) if z.0 != 0 => Range(a.0 / z.1, a.1 / z.0),
            (U64BitAnd, Some(z)) => a.bit_and(z),
            (U64BitOr, Some(z)) if a.0 == a.1 && z.0 == z.1 => {
                let bits = a.0 | z.0;
                Range(bits, bits)
            }
            (U64ShlWrap, Some(z)) if z.0 == z.1 && a.1 <= u64::MAX >> (z.0 & 63) => {
                Range(a.0 << (z.0 & 63), a.1 << (z.0 & 63))
            }
            (U64Eq | U64Lt | U64Le | U64Gt | U64Ge, Some(z)) => {
                let same = args[0] == args[1];
                let (yes, no) = match op {
                    U64Eq => (same || a.0 == a.1 && a == z, a.1 < z.0 || z.1 < a.0),
                    U64Lt => (a.1 < z.0, same || a.0 >= z.1),
                    U64Le => (same || a.1 <= z.0, a.0 > z.1),
                    U64Gt => (a.0 > z.1, same || a.1 <= z.0),
                    U64Ge => (same || a.0 >= z.1, a.1 < z.0),
                    _ => unreachable!(),
                };
                if yes || no {
                    let bit = u64::from(yes);
                    Range(bit, bit)
                } else {
                    Range(0, 1)
                }
            }
            _ => return Err(invalid),
        })
    }

    fn operation(&self, op: Op, args: Vec<Word>) -> TraceResult<Word> {
        use Op::*;
        let range = self.result_range(op, &args)?;
        if range.0 == range.1 {
            return Ok(Word::Constant(range.0));
        }
        match (op, args.as_slice()) {
            (U64Add | U64BitOr, [value, Word::Constant(0)] | [Word::Constant(0), value])
            | (U64Sub, [value, Word::Constant(0)]) => Ok(value.clone()),
            (U64Eq | U64Lt | U64Le | U64Gt | U64Ge, [Word::Length(port), Word::Constant(n)]) => {
                // Split at a comparison boundary of an actual current length.
                // Equality with an interior value can split again when revisited.
                let at = if matches!(op, U64Lt | U64Ge)
                    || op == U64Eq && range_contains_lower(self.lengths, *port, *n)
                {
                    *n
                } else {
                    n.checked_add(1).ok_or(TraceFailure::Unsupported)?
                };
                Err(TraceFailure::Split(*port, at))
            }
            _ => Ok(Word::Operation(op, args.into(), range)),
        }
    }

    fn aggregate(&mut self, action: &Aggregate) -> TraceResult<()> {
        let (output, lanes) = match action {
            Aggregate::Copy { source, output } => (*output, self.read(*source)?),
            Aggregate::Pack { output, words } => {
                let lanes = words
                    .iter()
                    .map(|word| match *word {
                        Lane::Scalar(value) => Ok(LaneValue::Scalar(self.scalar(value)?)),
                        Lane::Field(value, at) => {
                            Ok(self.lanes(self.layout.slot(value.get()) + at, 1)?.remove(0))
                        }
                        Lane::Constant(bits) => Ok(LaneValue::Scalar(Word::Constant(bits))),
                    })
                    .collect::<TraceResult<_>>()?;
                (*output, lanes)
            }
            Aggregate::Project {
                source,
                offset,
                guards,
                output,
            } if guards.is_empty() => {
                let start = self.layout.slot(source.get()) + offset;
                (
                    *output,
                    self.lanes(start, self.layout.ty(output.get()).words)?,
                )
            }
            _ => return Err(TraceFailure::Unsupported),
        };
        self.write(output, lanes);
        Ok(())
    }

    fn buffer(
        &mut self,
        site: usize,
        operation: Buffer,
        operands: &[Port],
        output: Port,
        owners: &OwnerFlow,
        can_finish: bool,
    ) -> TraceResult<()> {
        let [source, ..] = operands else {
            return Err(TraceFailure::Unsupported);
        };
        let view = self.view(*source)?;
        if operation == Buffer::SequenceReserve {
            let View::Borrowed(port) = view else {
                return Err(TraceFailure::Unsupported);
            };
            if self.reserve.is_some() || self.origin.is_some() {
                return Err(TraceFailure::Unsupported);
            }
            self.prefix = Some(port);
            self.reserve = Some(self.scalar(operands[1])?);
            self.write_view(output, view);
            return Ok(());
        }
        match (self.origin, view) {
            (None, View::Borrowed(port)) if self.prefix.is_none_or(|found| found == port) => {
                self.prefix = Some(port);
                self.origin = Some(site);
            }
            (Some(_), View::Appended(_)) if can_finish && self.span.is_none() => {
                // A second held alias would force the original append to copy.
                let source = self.layout.slot(source.get()) + 5;
                if self.words.iter().any(|(&slot, word)| {
                    slot != source
                        && matches!(word, LaneValue::View(View::Appended(_), 5))
                        && owners.live_at_entry(site, slot)
                }) {
                    return Err(TraceFailure::Unsupported);
                }
            }
            _ => return Err(TraceFailure::Unsupported),
        }
        self.last = Some(site);
        match operation {
            Buffer::SequenceAppend => self.packed.push((self.scalar(operands[1])?, 1)),
            Buffer::SequenceAppendLeBytes => {
                let Word::Constant(count @ 1..=8) = self.scalar(operands[2])? else {
                    return Err(TraceFailure::Unsupported);
                };
                self.packed.push((self.scalar(operands[1])?, count as u8));
            }
            Buffer::SequenceAppendSublist if self.byte_count != 0 => {
                let View::Borrowed(port) = self.view(operands[1])? else {
                    return Err(TraceFailure::Unsupported);
                };
                let start = self.scalar(operands[2])?;
                let count = self.scalar(operands[3])?;
                let length = Word::Length(port);
                if start != Word::Constant(0)
                    || self.range(&count)?.1 >= 16
                    || count != length && self.range(&count)?.1 > self.range(&length)?.0
                {
                    return Err(TraceFailure::Unsupported);
                }
                self.span = Some((port, count));
                self.write_view(output, View::Appended(site));
                return Ok(());
            }
            _ => return Err(TraceFailure::Unsupported),
        }
        self.byte_count += self.packed.last().unwrap().1;
        if self.byte_count > 8 {
            return Err(TraceFailure::Unsupported);
        }
        self.write_view(output, View::Appended(site));
        Ok(())
    }
}

fn range_contains_lower(lengths: Option<Lengths>, port: usize, at: u64) -> bool {
    lengths
        .filter(|bound| bound.port == port)
        .is_some_and(|bound| bound.range.0 < at)
}

impl TerminalAppends {
    pub(super) fn derive(
        plan: &Plan,
        layout: &Layout,
        flow: &Flow,
        recipes: &[Recipe],
        owners: &OwnerFlow,
        initial: &BTreeMap<usize, Initial>,
        finishes: &[Option<(usize, usize, bool)>],
    ) -> Option<Self> {
        let [output] = flow.outputs.as_slice() else {
            return None;
        };
        let byte_array = |id| {
            layout.ty(id).element().is_ok_and(|e| {
                matches!(
                    e.kind,
                    super::layout::Kind::Scalar {
                        bytes: 1,
                        signed: false,
                        float: false
                    }
                )
            })
        };
        if flow.inputs.is_empty()
            || !byte_array(*output)
            || !flow.inputs.iter().copied().all(byte_array)
        {
            return None;
        }
        let mut routes = Vec::new();
        let mut work = vec![None::<Lengths>];
        // Optional selection only. An open path or an unsupported action stays
        // in the checked entry. Explore only a few source comparison intervals.
        for _ in 0..8 {
            let Some(lengths) = work.pop() else { break };
            let mut state = Trace {
                layout,
                initial,
                lengths,
                words: BTreeMap::new(),
                byte_count: 0,
                packed: Vec::new(),
                span: None,
                prefix: None,
                reserve: None,
                origin: None,
                last: None,
            };
            for (port, &input) in flow.inputs.iter().enumerate() {
                let base = layout.slot(input);
                for lane in 0..6 {
                    state
                        .words
                        .insert(base + lane, LaneValue::View(View::Borrowed(port * 6), lane));
                }
            }
            let path = (|| {
                let mut visited = BTreeSet::new();
                let mut site = flow.entry;
                for _ in 0..128 {
                    if !visited.insert(site) {
                        return Err(TraceFailure::Unsupported);
                    }
                    let block = &plan.blocks()[site];
                    match &recipes[site] {
                        Recipe::Aggregate(action) => state.aggregate(action)?,
                        Recipe::Length { source, output } => {
                            let View::Borrowed(port) = state.view(*source)? else {
                                return Err(TraceFailure::Unsupported);
                            };
                            state.write(*output, vec![LaneValue::Scalar(Word::Length(port))]);
                        }
                        Recipe::Buffer {
                            operation,
                            operands,
                            output,
                        } => state.buffer(
                            site,
                            *operation,
                            operands,
                            *output,
                            owners,
                            finishes[site].is_some_and(|x| x.2),
                        )?,
                        Recipe::None => match block.action() {
                            Action::Scalar(Scalar::None) => {}
                            Action::Scalar(Scalar::Primitive {
                                op,
                                operands,
                                output,
                            }) => {
                                let args = operands
                                    .iter()
                                    .map(|v| state.scalar(*v))
                                    .collect::<TraceResult<_>>()?;
                                let result = state.operation(*op, args)?;
                                state.write(*output, vec![LaneValue::Scalar(result)]);
                            }
                            Action::Initialize { iteration } => {
                                let Loop::Tail(tail) = &plan.iterations()[*iteration as usize]
                                else {
                                    return Err(TraceFailure::Unsupported);
                                };
                                let seed = state.read(tail.seed())?;
                                state.write(tail.carrier(), seed);
                            }
                            _ => return Err(TraceFailure::Unsupported),
                        },
                        _ => return Err(TraceFailure::Unsupported),
                    }
                    site = match block.terminator() {
                        Term::Scalar(Branch::Jump(next)) | Term::TailHeader { body: next, .. } => {
                            next
                        }
                        Term::Scalar(Branch::Branch { condition, yes, no }) => {
                            match state.range(&state.scalar(condition)?)? {
                                Range(0, 0) => no,
                                Range(1, 1) => yes,
                                _ => return Err(TraceFailure::Unsupported),
                            }
                        }
                        Term::Scalar(Branch::LiteralBranch {
                            left,
                            right,
                            yes,
                            no,
                        }) => {
                            if plan.values()[left.get() as usize].scalar_kind()
                                == Some(NativeScalarKind::F64)
                                || matches!(
                                    layout.ty(left.get()).kind,
                                    super::layout::Kind::Scalar { float: true, .. }
                                )
                            {
                                return Err(TraceFailure::Unsupported);
                            }
                            let left = state.scalar(left)?;
                            let right = state.scalar(right)?;
                            let equal = state.result_range(Op::U64Eq, &[left, right])?;
                            match equal {
                                Range(1, 1) => yes,
                                Range(0, 0) => no,
                                _ => return Err(TraceFailure::Unsupported),
                            }
                        }
                        Term::Scalar(Branch::Complete) => {
                            let value = plan.outputs()[0].value();
                            let View::Appended(at) = state.view(value)? else {
                                return Err(TraceFailure::Unsupported);
                            };
                            return if Some(at) == state.last && state.byte_count != 0 {
                                Ok(())
                            } else {
                                Err(TraceFailure::Unsupported)
                            };
                        }
                        _ => return Err(TraceFailure::Unsupported),
                    } as usize;
                }
                Err(TraceFailure::Unsupported)
            })();
            match path {
                Ok(()) => {
                    let Some(lengths) = lengths else { continue };
                    routes.push(Route {
                        lengths,
                        prefix: state.prefix?,
                        reserve: state.reserve.unwrap_or(Word::Constant(0)),
                        packed: state.packed,
                        span: state.span,
                        origin: state.origin?,
                    });
                }
                Err(TraceFailure::Split(port, at)) => {
                    let bound = lengths.unwrap_or(Lengths {
                        port,
                        range: Range(0, u64::MAX),
                    });
                    if port == bound.port && bound.range.0 < at && at <= bound.range.1 {
                        work.push(Some(Lengths {
                            port,
                            range: Range(at, bound.range.1),
                        }));
                        work.push(Some(Lengths {
                            port,
                            range: Range(bound.range.0, at - 1),
                        }));
                    }
                }
                Err(TraceFailure::Unsupported) => {}
            }
        }
        (!routes.is_empty()).then_some(Self {
            routes,
            inputs: flow.inputs.len(),
        })
    }
}

struct Packet {
    base: Value,
    lanes: BTreeMap<usize, Value>,
    computed: BTreeMap<*const [Word], Value>,
}

impl Packet {
    fn load(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        lane: usize,
    ) -> Result<Value, NativeCompileError> {
        if let Some(&value) = self.lanes.get(&lane) {
            return Ok(value);
        }
        let value = b
            .ins()
            .load(types::I64, MemFlags::new(), self.base, offset(lane)?);
        self.lanes.insert(lane, value);
        Ok(value)
    }

    fn scalar(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        word: &Word,
        fallback: cranelift_codegen::ir::Block,
    ) -> Result<Value, NativeCompileError> {
        Ok(match word {
            Word::Constant(bits) => b.ins().iconst(types::I64, *bits as i64),
            Word::Length(port) => self.load(b, port + 1)?,
            Word::Operation(op, args, _) => {
                let key = Rc::as_ptr(args);
                if let Some(&value) = self.computed.get(&key) {
                    return Ok(value);
                }
                let args = args
                    .iter()
                    .map(|arg| self.scalar(b, arg, fallback))
                    .collect::<Result<Vec<_>, _>>()?;
                // The selected trace proves this operation. The normal
                // emitter still sends a refused check to the checked entry.
                let value = primitive(b, *op, &args, fallback);
                self.computed.insert(key, value);
                value
            }
        })
    }
}

impl TerminalAppends {
    pub(super) fn define<M: Module>(
        &self,
        module: &mut M,
        symbol: &str,
        signature: &Signature,
        original: FuncId,
    ) -> Result<serde_json::Value, NativeCompileError> {
        let mut helper_sig = module.make_signature();
        helper_sig.params = vec![AbiParam::new(types::I64); 7];
        helper_sig.returns = vec![AbiParam::new(types::I64)];
        let helper = module
            .declare_function(
                "mech_compact_finish_byte_appends",
                Linkage::Import,
                &helper_sig,
            )
            .map_err(backend)?;
        let entry_id = module
            .declare_function(symbol, Linkage::Export, signature)
            .map_err(backend)?;
        let mut context = module.make_context();
        context.func.signature = signature.clone();
        let mut builder = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let [heap, input, output] = b.block_params(entry) else {
                unreachable!()
            };
            let (heap, input, output) = (*heap, *input, *output);
            let zero = b.ins().iconst(types::I64, 0);
            let fallback = b.create_block();
            let scalar_failure = b.create_block();
            b.append_block_param(scalar_failure, types::I64);
            let mut candidates = vec![entry];
            for _ in 1..self.routes.len() {
                candidates.push(b.create_block());
            }
            candidates.push(fallback);
            for (route, blocks) in self.routes.iter().zip(candidates.windows(2)) {
                b.switch_to_block(blocks[0]);
                let mut packet = Packet {
                    base: input,
                    lanes: BTreeMap::new(),
                    computed: BTreeMap::new(),
                };
                let length = packet.load(&mut b, route.lengths.port + 1)?;
                let Range(low, high) = route.lengths.range;
                let selected = match Range(low, high) {
                    Range(low, high) if low == high => {
                        b.ins().icmp_imm(IntCC::Equal, length, low as i64)
                    }
                    Range(0, high) => {
                        b.ins()
                            .icmp_imm(IntCC::UnsignedLessThanOrEqual, length, high as i64)
                    }
                    Range(low, u64::MAX) => {
                        b.ins()
                            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, low as i64)
                    }
                    Range(low, high) => {
                        let shifted = b.ins().iadd_imm(length, low.wrapping_neg() as i64);
                        b.ins().icmp_imm(
                            IntCC::UnsignedLessThanOrEqual,
                            shifted,
                            (high - low) as i64,
                        )
                    }
                };
                let inspect = b.create_block();
                b.ins().brif(selected, inspect, &[], blocks[1], &[]);
                b.switch_to_block(inspect);
                let mut tests = vec![
                    b.ins().icmp_imm(IntCC::NotEqual, heap, 0),
                    b.ins().icmp_imm(IntCC::NotEqual, output, 0),
                ];
                let mut owners = zero;
                for port in 0..self.inputs {
                    let owner = packet.load(&mut b, port * 6 + 5)?;
                    owners = b.ins().bor(owners, owner);
                }
                tests.push(b.ins().icmp_imm(IntCC::Equal, owners, 0));
                for port in std::iter::once(route.prefix)
                    .chain(route.span.iter().map(|(port, _)| *port))
                    .collect::<BTreeSet<_>>()
                {
                    let rows = packet.load(&mut b, port + 3)?;
                    let len = packet.load(&mut b, port + 1)?;
                    let cap = packet.load(&mut b, port + 2)?;
                    tests.push(b.ins().icmp_imm(IntCC::Equal, rows, 1));
                    tests.push(b.ins().icmp(IntCC::UnsignedLessThanOrEqual, len, cap));
                }
                let mut word = zero;
                let mut written = 0u8;
                for (bits, bytes) in &route.packed {
                    let bits = packet.scalar(&mut b, bits, scalar_failure)?;
                    let bits = if *bytes == 8 {
                        bits
                    } else {
                        b.ins().band_imm(bits, (1i64 << (*bytes * 8)) - 1)
                    };
                    let part = b.ins().ishl_imm(bits, i64::from(written) * 8);
                    word = b.ins().bor(word, part);
                    written += bytes;
                }
                let (span, count) = if let Some((port, count)) = &route.span {
                    (
                        packet.load(&mut b, *port)?,
                        packet.scalar(&mut b, count, scalar_failure)?,
                    )
                } else {
                    (zero, zero)
                };
                let capacity = packet.load(&mut b, route.prefix + 2)?;
                let prefix_len = packet.load(&mut b, route.prefix + 1)?;
                let space = b.ins().isub(capacity, prefix_len);
                let needed = b.ins().iadd_imm(count, i64::from(written));
                let same_minimum =
                    match (&route.reserve, route.span.as_ref().map(|(_, count)| count)) {
                        (Word::Constant(cap), None) => *cap == u64::from(written),
                        (Word::Operation(Op::U64Add, args, _), Some(count)) => {
                            matches!(args.as_ref(), [n, Word::Constant(k)] | [Word::Constant(k), n]
                            if n == count && *k == u64::from(written))
                        }
                        _ => false,
                    };
                if !same_minimum {
                    let reserve = packet.scalar(&mut b, &route.reserve, scalar_failure)?;
                    tests.push(b.ins().icmp(IntCC::UnsignedLessThanOrEqual, reserve, space));
                }
                tests.push(b.ins().icmp(IntCC::UnsignedLessThanOrEqual, needed, space));
                // Small groups leave the conjunction in condition flags on
                // targets with conditional comparisons.
                for group in tests.chunks(5) {
                    let allowed = group
                        .iter()
                        .copied()
                        .reduce(|a, z| b.ins().band(a, z))
                        .unwrap();
                    let next = b.create_block();
                    b.ins().brif(allowed, next, &[], fallback, &[]);
                    b.switch_to_block(next);
                }
                let prefix = b.ins().iadd_imm(input, i64::from(offset(route.prefix)?));
                let count_word = b.ins().iconst(types::I64, i64::from(written));
                let helper = module.declare_func_in_func(helper, b.func);
                let call = b.ins().call(
                    helper,
                    &[heap, output, prefix, word, count_word, span, count],
                );
                let status = b.inst_results(call)[0];
                let success = b.ins().icmp_imm(IntCC::Equal, status, 0);
                let origin = b.ins().iconst(types::I64, route.origin as i64);
                let origin = b.ins().select(success, zero, origin);
                b.ins().return_(&[status, origin]);
            }
            b.switch_to_block(scalar_failure);
            b.set_cold_block(scalar_failure);
            b.ins().jump(fallback, &[]);
            b.switch_to_block(fallback);
            let original = module.declare_func_in_func(original, b.func);
            let call = b.ins().call(original, &[heap, input, output]);
            let results = b.inst_results(call).to_vec();
            b.ins().return_(&results);
            b.seal_all_blocks();
            b.finalize();
        }
        module
            .define_function(entry_id, &mut context)
            .map_err(backend)?;
        Ok(serde_json::json!({
            "code_bytes": context.compiled_code().map(|c| c.code_info().total_size),
            "routes": self.routes.iter().map(|route| serde_json::json!({
                "length_port": route.lengths.port / 6,
                "lower": route.lengths.range.0,
                "upper": route.lengths.range.1,
                "allocation_site": route.origin,
                "packed_bytes": route.packed.iter().map(|(_, width)| usize::from(*width)).sum::<usize>(),
                "source_span": route.span.as_ref().map(|(port, _)| port / 6),
            })).collect::<Vec<_>>(),
        }))
    }
}
