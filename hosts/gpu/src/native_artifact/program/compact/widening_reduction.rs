//! Direct native reduction for a complete read-only U32 × U8 dot product.
//! Admission checks both visits of the canonical CFG. The ordinary checked
//! traversal remains the fallback.
use super::{
    arrays::Arrays,
    constants::Initial,
    flow::Flow,
    layout::{ARRAY_WORDS, Kind, Layout},
    owner_flow::OwnerFlow,
    recipes::Recipe,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{
    PreparedNativeProgram, readonly_numeric::ResultRecipe,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, Endianness, FuncRef, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_artifact::{
    NativeAggregateWord as Lane, NativeIterationAction as Action,
    NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue as Value,
    NativeScalarAggregateAction as Aggregate, NativeScalarOp as Op,
};
use std::collections::BTreeMap;

const MAX_PRODUCT: u64 = u32::MAX as u64 * u8::MAX as u64;
const VECTOR_ITEMS: i64 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Word {
    Current(usize),
    Constant(u64),
    Length,
    NextIndex,
    Read32,
    Read8,
    Wide32,
    Wide8,
    Product,
    UpdatedCost,
}

#[derive(Clone, Copy)]
pub(super) struct Reduction {
    pub header: usize,
    pub exit: usize,
    arrays: [usize; 2],
    cursor: usize,
    cost: usize,
    result: usize,
    end: u64,
}

struct Visit<'a> {
    layout: &'a Layout,
    initial: &'a BTreeMap<usize, Initial>,
    locals: BTreeMap<u32, Vec<Word>>,
    arrays: [usize; 2],
    cursor: usize,
    cost: usize,
    active: bool,
    end: Option<u64>,
}

impl Visit<'_> {
    fn value(&self, value: Value) -> Option<Vec<Word>> {
        if let Some(words) = self.locals.get(&value.get()) {
            return Some(words.clone());
        }
        let Initial::Bits(bits) = *self.initial.get(&self.layout.slot(value.get()))? else {
            return None;
        };
        Some(vec![Word::Constant(bits)])
    }

    fn word(&self, value: Value) -> Option<Word> {
        let [word] = self.value(value)?[..] else {
            return None;
        };
        Some(word)
    }

    fn array(&self, value: Value) -> Option<usize> {
        let current = self.value(value)?;
        self.arrays.iter().position(|&base| {
            current
                .iter()
                .copied()
                .eq((base..base + ARRAY_WORDS).map(Word::Current))
        })
    }

    fn primitive(&mut self, op: Op, args: &[Value]) -> Option<Word> {
        use Word::*;
        let args = args
            .iter()
            .map(|&value| self.word(value))
            .collect::<Option<Vec<_>>>()?;
        let cursor = Current(self.cursor);
        let cost = Current(self.cost);
        Some(match (op, args.as_slice()) {
            (Op::U64Lt, [a, Constant(end)]) if *a == cursor => {
                if self.end.is_some_and(|old| old != *end) {
                    return None;
                }
                self.end = Some(*end);
                Constant(u64::from(self.active))
            }
            (Op::U64Lt, [a, Length]) if self.active && *a == cursor => Constant(1),
            (Op::U64Add | Op::U64WrappingAdd, [a, Constant(1)]) if self.active && *a == cursor => {
                NextIndex
            }
            (Op::U32ToU64, [Read32]) => Wide32,
            (Op::U8ToU64, [Read8]) => Wide8,
            (Op::U32ToU64, [Constant(n)]) if *n <= u64::from(u32::MAX) => Constant(*n),
            (Op::U8ToU64, [Constant(n)]) if *n <= u64::from(u8::MAX) => Constant(*n),
            (Op::U64Mul, [Wide32, Wide8] | [Wide8, Wide32]) => Product,
            (Op::U64Mul, [Constant(0), Constant(0)]) => Constant(0),
            (Op::U64Add, [a, Product] | [Product, a]) if *a == cost => UpdatedCost,
            (Op::U64Add, [a, Constant(0)] | [Constant(0), a]) if *a == cost => cost,
            _ => return None,
        })
    }

    fn action(&mut self, recipe: &Recipe, action: &Action) -> Option<()> {
        let (output, words) = match recipe {
            Recipe::Aggregate(Aggregate::Copy { source, output }) => {
                (*output, self.value(*source)?)
            }
            Recipe::Aggregate(Aggregate::Project {
                source,
                offset,
                guards,
                output,
            }) if guards.is_empty() => {
                let width = self.layout.ty(output.get()).words;
                (
                    *output,
                    self.value(*source)?.get(*offset..*offset + width)?.to_vec(),
                )
            }
            Recipe::Aggregate(Aggregate::Pack { output, words }) => (
                *output,
                words
                    .iter()
                    .map(|lane| match *lane {
                        Lane::Scalar(v) => self.word(v),
                        Lane::Field(v, field) => self.value(v)?.get(field).copied(),
                        Lane::Constant(n) => Some(Word::Constant(n)),
                    })
                    .collect::<Option<Vec<_>>>()?,
            ),
            Recipe::Read {
                source,
                index,
                output,
            } if self.active && self.word(*index)? == Word::NextIndex => (
                *output,
                vec![[Word::Read32, Word::Read8][self.array(*source)?]],
            ),
            Recipe::Length { source, output } => {
                self.array(*source)?;
                (*output, vec![Word::Length])
            }
            Recipe::None => match action {
                Action::Scalar(Scalar::None) => return Some(()),
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => (*output, vec![self.primitive(*op, operands)?]),
                _ => return None,
            },
            _ => return None,
        };
        self.locals.insert(output.get(), words);
        Some(())
    }

    fn successor(&self, term: Term) -> Option<usize> {
        let site = match term {
            Term::Scalar(Branch::Jump(site)) | Term::TailHeader { body: site, .. } => site,
            Term::Scalar(Branch::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let (Word::Constant(left), Word::Constant(right)) =
                    (self.word(left)?, self.word(right)?)
                else {
                    return None;
                };
                if left == right { yes } else { no }
            }
            Term::Scalar(Branch::Branch { condition, yes, no }) => match self.word(condition)? {
                Word::Constant(1) => yes,
                Word::Constant(0) => no,
                _ => return None,
            },
            _ => return None,
        };
        Some(site as usize)
    }
}

pub(super) fn derive(
    prepared: &PreparedNativeProgram,
    layout: &Layout,
    flow: &Flow,
    actions: &[Recipe],
    initial: &BTreeMap<usize, Initial>,
) -> BTreeMap<usize, Reduction> {
    if !cfg!(all(target_arch = "aarch64", target_endian = "little"))
        || std::env::var_os("MECH_WIDENING_REDUCTION").is_none()
    {
        return BTreeMap::new();
    }
    let plan = prepared.plan();
    prepared
        .metadata()
        .closed_tails
        .regions
        .iter()
        .filter_map(|region| {
            let proof = region.numeric.as_ref()?;
            if proof.writes()
                || proof.arrays.len() != 2
                || proof.phase.is_some()
                || !proof.exits.is_empty()
                || !matches!(proof.result, ResultRecipe::Scalar)
            {
                return None;
            }
            let tail = &region.tail;
            let carrier = tail.carrier().get();
            let returned = tail.returned().get();
            let fields = layout.ty(carrier).fields().ok()?;
            let base = layout.slot(carrier);
            let array_slot = |width| {
                let array = proof.arrays.iter().find(|a| a.width == width)?;
                let (slot, ty) = layout.array_field(array, carrier).ok()?;
                (ty.words == ARRAY_WORDS).then_some(slot)
            };
            let arrays = [array_slot(4)?, array_slot(1)?];
            let unsigned_word = Kind::Scalar {
                bytes: 8,
                signed: false,
                float: false,
            };
            if fields.len() != 4
                || layout.ty(returned).kind != unsigned_word
                || proof.boundary_values.exit.iter().any(|value| {
                    *value != returned
                        && proof
                            .sites
                            .iter()
                            .any(|&site| flow.defs[site].contains(value))
                })
            {
                return None;
            }
            let scalar = fields
                .iter()
                .filter(|field| field.ty.kind == unsigned_word)
                .map(|field| base + field.offset)
                .collect::<Vec<_>>();
            let &[first, second] = scalar.as_slice() else {
                return None;
            };
            let header = tail.header() as usize;
            let exit = tail.exit() as usize;
            let latch = tail.latch() as usize;
            let finish = tail.finish() as usize;
            if !flow.reachable[header]
                || !flow.reachable[exit]
                || !matches!(plan.blocks()[finish].action(), Action::Scalar(Scalar::None))
                || plan.blocks()[finish].terminator() != Term::Scalar(Branch::Jump(tail.exit()))
            {
                return None;
            }
            let current = (base..base + layout.ty(carrier).words)
                .map(Word::Current)
                .collect::<Vec<_>>();
            for (cursor, cost) in [(first, second), (second, first)] {
                let check = |active, end| -> Option<u64> {
                    let mut visit = Visit {
                        layout,
                        initial,
                        locals: BTreeMap::from([(carrier, current.clone())]),
                        arrays,
                        cursor,
                        cost,
                        active,
                        end,
                    };
                    let mut site = header;
                    for _ in 0..proof.sites.len() {
                        if site == latch && active {
                            let mut expected = current.clone();
                            expected[cursor - base] = Word::NextIndex;
                            expected[cost - base] = Word::UpdatedCost;
                            return (visit.value(tail.next())? == expected).then_some(visit.end?);
                        }
                        if site == finish && !active {
                            return (visit.word(tail.returned())? == Word::Current(cost))
                                .then_some(visit.end?);
                        }
                        if proof.sites.binary_search(&site).is_err() {
                            return None;
                        }
                        let block = &plan.blocks()[site];
                        visit.action(&actions[site], block.action())?;
                        site = visit.successor(block.terminator())?;
                    }
                    None
                };
                if let Some(end) = check(true, None)
                    && end >= VECTOR_ITEMS as u64
                    && end.checked_mul(MAX_PRODUCT).is_some()
                    && check(false, Some(end)) == Some(end)
                {
                    return Some((
                        header,
                        Reduction {
                            header,
                            exit,
                            arrays,
                            cursor,
                            cost,
                            result: layout.slot(returned),
                            end,
                        },
                    ));
                }
            }
            None
        })
        .collect()
}

impl Reduction {
    pub(super) fn emit<M: Module>(
        self,
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        function: FuncId,
        words: &Words,
        arrays: &Arrays,
        owners: &OwnerFlow,
        observer: Option<FuncRef>,
        exit: Block,
        error: Block,
    ) {
        let cursor = words.read(b, self.cursor);
        let cost = words.read(b, self.cost);
        let fallback = b.create_block();
        let fast = b.create_block();
        let end = b.ins().iconst(types::I64, self.end as i64);
        let limit = self.end - VECTOR_ITEMS as u64;
        let count = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, cursor, limit as i64);
        // All terms are nonnegative. This bounds every source prefix as well
        // as each independently accumulated vector lane.
        let budget = b
            .ins()
            .iconst(types::I64, (u64::MAX - self.end * MAX_PRODUCT) as i64);
        let safe = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, cost, budget);
        let mut admitted = b.ins().band(count, safe);
        for base in self.arrays {
            let length = words.read(b, base + 1);
            let rows = words.read(b, base + 3);
            let columns = words.read(b, base + 4);
            let length = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, length, end);
            let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
            let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
            let flat = b.ins().bor(row, column);
            let valid = b.ins().band(length, flat);
            admitted = b.ins().band(admitted, valid);
        }
        b.ins().brif(admitted, fast, &[], fallback, &[]);
        b.switch_to_block(fast);
        if let Some(observer) = observer {
            let header = b.ins().iconst(types::I64, self.header as i64);
            let route = b.ins().iconst(types::I64, 5);
            b.ins().call(observer, &[arrays.heap, header, route]);
        }
        let [left, right] = self.arrays.map(|base| words.read(b, base));
        let function = module.declare_func_in_func(function, b.func);
        let call = b.ins().call(function, &[left, right, cursor, end]);
        let sum = b.inst_results(call)[0];
        let result = b.ins().iadd(cost, sum);
        words.write(b, self.result, result);
        arrays.retire(
            b,
            words,
            &owners.retire_bypass(self.header, self.exit, &[]),
            error,
        );
        b.ins().jump(exit, &[]);
        b.switch_to_block(fallback);
        if let Some(observer) = observer {
            let header = b.ins().iconst(types::I64, self.header as i64);
            let route = b.ins().iconst(types::I64, 0);
            b.ins().call(observer, &[arrays.heap, header, route]);
        }
    }
}

pub(super) fn declare<M: Module>(module: &mut M) -> Result<FuncId, NativeCompileError> {
    module
        .declare_function(
            "mech_compact_unsigned_dot_product",
            Linkage::Local,
            &signature(module),
        )
        .map_err(backend)
}

fn signature<M: Module>(module: &M) -> cranelift_codegen::ir::Signature {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 4];
    signature.returns = vec![AbiParam::new(types::I64)];
    signature
}

pub(super) fn define<M: Module>(
    module: &mut M,
    function: FuncId,
) -> Result<(), NativeCompileError> {
    let mut context = module.make_context();
    context.func.signature = signature(module);
    let mut storage = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut storage);
    let entry = b.create_block();
    let vector = b.create_block();
    let reduce = b.create_block();
    let tail = b.create_block();
    let single = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    for block in [vector, reduce, tail] {
        b.append_block_param(block, types::I64);
        b.append_block_param(
            block,
            if block == tail {
                types::I64
            } else {
                types::I64X2
            },
        );
    }
    b.switch_to_block(entry);
    let args = b.block_params(entry);
    let [left, right, start, end] = [args[0], args[1], args[2], args[3]];
    let zero = b.ins().iconst(types::I64, 0);
    let sum = b.ins().splat(types::I64X2, zero);
    let remaining = b.ins().isub(end, start);
    let bulk = b.ins().band_imm(remaining, -VECTOR_ITEMS);
    let limit = b.ins().iadd(start, bulk);
    b.ins().jump(vector, &[start.into(), sum.into()]);
    b.switch_to_block(vector);
    let at = b.block_params(vector)[0];
    let sum = b.block_params(vector)[1];
    let left_offset = b.ins().ishl_imm(at, 2);
    let lhs = b.ins().iadd(left, left_offset);
    let rhs = b.ins().iadd(right, at);
    let left_vector = b.ins().load(types::I32X4, MemFlags::new(), lhs, 0);
    let right_bytes = b.ins().load(types::I32, MemFlags::new(), rhs, 0);
    let right_vector = b.ins().scalar_to_vector(types::I32X4, right_bytes);
    let little = MemFlags::new().with_endianness(Endianness::Little);
    let right_vector = b.ins().bitcast(types::I8X16, little, right_vector);
    let right_vector = b.ins().uwiden_low(right_vector);
    let right_vector = b.ins().uwiden_low(right_vector);
    let left_low = b.ins().uwiden_low(left_vector);
    let right_low = b.ins().uwiden_low(right_vector);
    let low = b.ins().imul(left_low, right_low);
    let left_high = b.ins().uwiden_high(left_vector);
    let right_high = b.ins().uwiden_high(right_vector);
    let high = b.ins().imul(left_high, right_high);
    let sum = b.ins().iadd(sum, low);
    let sum = b.ins().iadd(sum, high);
    let next = b.ins().iadd_imm(at, VECTOR_ITEMS);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, next, limit);
    b.ins().brif(
        more,
        vector,
        &[next.into(), sum.into()],
        reduce,
        &[next.into(), sum.into()],
    );
    b.switch_to_block(reduce);
    let at = b.block_params(reduce)[0];
    let sum = b.block_params(reduce)[1];
    let low = b.ins().extractlane(sum, 0);
    let high = b.ins().extractlane(sum, 1);
    let sum = b.ins().iadd(low, high);
    b.ins().jump(tail, &[at.into(), sum.into()]);
    b.switch_to_block(tail);
    let at = b.block_params(tail)[0];
    let result = b.block_params(tail)[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, end);
    b.ins().brif(more, single, &[], done, &[]);
    b.switch_to_block(single);
    let left_offset = b.ins().ishl_imm(at, 2);
    let lhs = b.ins().iadd(left, left_offset);
    let rhs = b.ins().iadd(right, at);
    let a = b.ins().uload32(MemFlags::new(), lhs, 0);
    let z = b.ins().uload8(types::I64, MemFlags::new(), rhs, 0);
    let product = b.ins().imul(a, z);
    let sum = b.ins().iadd(result, product);
    let next = b.ins().iadd_imm(at, 1);
    b.ins().jump(tail, &[next.into(), sum.into()]);
    b.switch_to_block(done);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    context.verify(module.isa()).map_err(backend)?;
    module
        .define_function(function, &mut context)
        .map_err(backend)
}
