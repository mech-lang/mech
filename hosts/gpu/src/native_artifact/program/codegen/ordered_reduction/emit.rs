//! Compact ordered reduction; long current intervals also use four-lane costs.
use super::*;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::compiler::primitive;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, ConstantData, Endianness, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;

impl Reduction {
    pub fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        proof: &Proof,
        shadow: usize,
        ordinary: Block,
        facts: &super::super::traversal_facts::Proof,
        initialize: usize,
        carry_item: bool,
        countdown: bool,
    ) -> Block {
        let c = &self.candidate;
        let start = words.read(b, c.induction);
        let stop = words.read(b, c.count);
        let tag = words.read(b, c.tag);
        let active = b.ins().icmp_imm(IntCC::Equal, tag, c.active as i64);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, start, stop);
        let remaining = b.ins().isub(stop, start);
        let fits = if facts.check(
            initialize,
            super::super::traversal_facts::Check::Le(
                super::super::traversal_facts::Operand::Word(c.count),
                super::super::traversal_facts::Operand::Constant(u32::MAX as u64),
            ),
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, stop, u32::MAX as i64)
        };
        let mut valid = b.ins().band(active, more);
        valid = b.ins().band(valid, fits);
        for read in &self.reads {
            if facts.flat_arrays.contains(&read.array) {
                continue;
            }
            let view = shadow + proof.view(read.array);
            let rows = words.read(b, view + 2);
            let columns = words.read(b, view + 3);
            let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
            let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
            let flat = b.ins().bor(row, column);
            valid = b.ins().band(valid, flat);
        }
        let setup = b.create_block();
        b.ins().brif(valid, setup, &[], ordinary, &[]);
        b.switch_to_block(setup);
        let best = words.read(b, self.best);
        let best = b.ins().ireduce(types::I32, best);
        let inputs = super::cost::Inputs::new(b, words, proof, shadow, &self.cost);
        let old_item = carry_item.then(|| words.read(b, self.item));
        let carried = if carry_item { types::I64 } else { types::I32 };
        let scalar_body = b.create_block();
        for ty in [types::I64, types::I32, carried] {
            b.append_block_param(scalar_body, ty);
        }
        let singleton =
            carry_item && std::env::var_os("MECH_SINGLETON_ORDERED_REDUCTION").is_some();
        let reverse = if singleton
            && cfg!(target_arch = "aarch64")
            && std::env::var_os("MECH_REVERSE_REDUCTION_CURSOR").is_some()
        {
            self.reads.iter().copied().find(|read| read.reverse)
        } else {
            None
        };
        let biased = reverse.and_then(|_| {
            if !countdown
                || std::env::var_os("MECH_NARROW_ORDERED_ITEM").is_none()
                || super::cost::Inputs::uses_index(&self.cost)
            {
                return None;
            }
            let window = self
                .reads
                .iter()
                .copied()
                .find(|read| !read.reverse && read.delta <= i32::MAX as u64)?;
            Some((window, split_shifted_item(&self.selected, window.delta)?))
        });
        let forward = biased.map(|(window, _)| window);
        if reverse.is_some() {
            b.append_block_param(scalar_body, types::I64);
        }
        if forward.is_some() {
            b.append_block_param(scalar_body, types::I64);
        }
        let scalar_finish = b.create_block();
        b.append_block_param(scalar_finish, types::I32);
        b.append_block_param(scalar_finish, carried);
        let vector = b.create_block();
        let missing = b.ins().iconst(types::I32, -1);
        let initial = old_item.unwrap_or(missing);
        let long = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 16);
        if let Some(old_item) = old_item.filter(|_| singleton) {
            let short = b.create_block();
            b.ins().brif(long, vector, &[], short, &[]);
            b.switch_to_block(short);
            // Do the first admitted visit directly. Only the continuing
            // route constructs the next cursor and enters the scalar loop.
            let reverse_cursor =
                reverse.map(|window| (window, inputs.pointer(b, proof, window, start)));
            let cost = inputs.scalar(b, proof, &self.cost, start, reverse_cursor.as_slice());
            let less = b.ins().icmp(IntCC::UnsignedLessThan, cost, best);
            let selected = emit_item(
                b,
                words,
                proof,
                shadow,
                &self.selected,
                start,
                ordinary,
                true,
            );
            let minimum = b.ins().select(less, cost, best);
            let item = b.ins().select(less, selected, old_item);
            let one = b.ins().icmp_imm(IntCC::Equal, remaining, 1);
            let advance = b.create_block();
            b.ins().brif(
                one,
                scalar_finish,
                &[minimum.into(), item.into()],
                advance,
                &[],
            );
            b.switch_to_block(advance);
            let next = b.ins().iadd_imm(start, 1);
            let pointer = reverse_cursor.map_or(next, |(_, pointer)| pointer);
            let mut args = vec![next.into(), minimum.into(), item.into()];
            if reverse.is_some() {
                args.push(pointer.into());
            }
            if let Some(window) = forward {
                let index = b.ins().iadd_imm(next, window.delta as i64);
                let left = b.ins().isub(stop, next);
                args[0] = index.into();
                args.push(left.into());
            }
            b.ins().jump(scalar_body, &args);
        } else {
            b.ins().brif(
                long,
                vector,
                &[],
                scalar_body,
                &[start.into(), best.into(), initial.into()],
            );
        }
        b.switch_to_block(vector);
        let prefix = b.ins().band_imm(remaining, -4);
        let end = b.ins().iadd(start, prefix);
        let costs = b.ins().splat(types::I32X4, best);
        let indices = super::cost::constant(b, [u32::MAX; 4]);
        let lanes = super::cost::constant(b, [0, 1, 2, 3]);
        let header = b.create_block();
        let body = b.create_block();
        let finish = b.create_block();
        for ty in [types::I64, types::I32X4, types::I32X4] {
            b.append_block_param(header, ty);
        }
        b.ins()
            .jump(header, &[start.into(), costs.into(), indices.into()]);
        b.switch_to_block(header);
        let params = b.block_params(header).to_vec();
        let k = params[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, k, end);
        b.ins().brif(more, body, &[], finish, &[]);
        b.switch_to_block(body);
        let k32 = b.ins().ireduce(types::I32, k);
        let index = b.ins().splat(types::I32X4, k32);
        let index = b.ins().iadd(index, lanes);
        let cost = inputs.emit(b, proof, &self.cost, k, index);
        let better = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
        let costs = b.ins().umin(cost, params[1]);
        let indices = b.ins().bitselect(better, index, params[2]);
        let next = b.ins().iadd_imm(k, 4);
        b.ins()
            .jump(header, &[next.into(), costs.into(), indices.into()]);
        b.switch_to_block(finish);
        // The incoming best already occupies every cost lane. A lane records
        // an index only after a strict improvement, so ties with that incoming
        // best retain the missing sentinel. Among improved lanes, reduce the
        // matching indices separately: the earliest index need not be in the
        // lowest lane after multiple vector groups.
        let minima = minimum_splat(b, params[1]);
        let equal = b.ins().icmp(IntCC::Equal, params[1], minima);
        let missing_indices = b.ins().splat(types::I32X4, missing);
        let indices = b.ins().bitselect(equal, params[2], missing_indices);
        let winners = minimum_splat(b, indices);
        let minimum = b.ins().extractlane(minima, 0);
        let winner = b.ins().extractlane(winners, 0);
        let carried_value = if let Some(old_item) = old_item {
            let changed = b.ins().icmp_imm(IntCC::NotEqual, winner, -1);
            let index = b.ins().uextend(types::I64, winner);
            let selected = emit_item(
                b,
                words,
                proof,
                shadow,
                &self.selected,
                index,
                ordinary,
                true,
            );
            b.ins().select(changed, selected, old_item)
        } else {
            winner
        };
        let more = b.ins().icmp(IntCC::UnsignedLessThan, end, stop);
        let finish_args = [minimum.into(), carried_value.into()];
        if let Some(window) = reverse {
            let remainder = b.create_block();
            b.ins()
                .brif(more, remainder, &[], scalar_finish, &finish_args);
            b.switch_to_block(remainder);
            // The vector prefix is nonempty. Seed the cursor at the last
            // vector candidate's low word, without loading it again.
            let previous = b.ins().iadd_imm(end, -1);
            let pointer = inputs.pointer(b, proof, window, previous);
            let mut args = vec![
                end.into(),
                minimum.into(),
                carried_value.into(),
                pointer.into(),
            ];
            if let Some(window) = forward {
                let index = b.ins().iadd_imm(end, window.delta as i64);
                let left = b.ins().isub(stop, end);
                args[0] = index.into();
                args.push(left.into());
            }
            b.ins().jump(scalar_body, &args);
        } else {
            b.ins().brif(
                more,
                scalar_body,
                &[end.into(), minimum.into(), carried_value.into()],
                scalar_finish,
                &finish_args,
            );
        }
        // Short entries have completed their first visit. Vector entries
        // reach this body only with a remainder.
        b.switch_to_block(scalar_body);
        let params = b.block_params(scalar_body).to_vec();
        let reverse_cursor = reverse.map(|window| {
            let stride = i64::from(proof.arrays[window.array].width);
            (window, b.ins().iadd_imm(params[3], -stride))
        });
        let mut pointers = reverse_cursor.into_iter().collect::<Vec<_>>();
        if let Some(window) = forward {
            // This index is already biased by the original forward access's
            // delta. Both entries have a nonempty range and the same admitted
            // immutable windows as the original scalar continuation.
            let base = words.read(b, shadow + proof.view(window.array));
            let offset = b
                .ins()
                .imul_imm(params[0], i64::from(proof.arrays[window.array].width));
            let pointer = b.ins().iadd(base, offset);
            pointers.push((window, pointer));
        }
        let cost = inputs.scalar(b, proof, &self.cost, params[0], &pointers);
        let less = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
        let minimum = if forward.is_some() {
            b.ins().select(less, cost, params[1])
        } else {
            b.ins().umin(cost, params[1])
        };
        let selected = if let Some((_, other)) = biased {
            let other = emit_item_low32(b, words, proof, shadow, other, params[0], ordinary);
            let index = b.ins().ireduce(types::I32, params[0]);
            let low = b.ins().bor(index, other);
            b.ins().uextend(types::I64, low)
        } else if carry_item {
            emit_item(
                b,
                words,
                proof,
                shadow,
                &self.selected,
                params[0],
                ordinary,
                true,
            )
        } else {
            b.ins().ireduce(types::I32, params[0])
        };
        let choice = b.ins().select(less, selected, params[2]);
        let next = b.ins().iadd_imm(params[0], 1);
        let mut next_args = vec![next.into(), minimum.into(), choice.into()];
        if let Some((_, pointer)) = reverse_cursor {
            next_args.push(pointer.into());
        }
        let more = if forward.is_some() {
            let left = b.ins().iadd_imm(params[4], -1);
            next_args.push(left.into());
            b.ins().icmp_imm(IntCC::NotEqual, left, 0)
        } else {
            b.ins().icmp(IntCC::UnsignedLessThan, next, stop)
        };
        b.ins().brif(
            more,
            scalar_body,
            &next_args,
            scalar_finish,
            &[minimum.into(), choice.into()],
        );
        b.switch_to_block(scalar_finish);
        let params = b.block_params(scalar_finish).to_vec();
        let minimum = params[0];
        let item = if carry_item {
            params[1]
        } else {
            let winner = params[1];
            let selected = b.create_block();
            let done = b.create_block();
            b.append_block_param(done, types::I64);
            let fallback = b.create_block();
            b.append_block_param(fallback, types::I64);
            let old_item = words.read(b, self.item);
            let changed = b.ins().icmp_imm(IntCC::NotEqual, winner, -1);
            b.ins()
                .brif(changed, selected, &[], done, &[old_item.into()]);
            b.switch_to_block(selected);
            let winner = b.ins().uextend(types::I64, winner);
            let item = emit_item(
                b,
                words,
                proof,
                shadow,
                &self.selected,
                winner,
                fallback,
                false,
            );
            b.ins().jump(done, &[item.into()]);
            b.switch_to_block(fallback);
            // Pure reduction state is still private. Retain the checked body.
            b.ins().jump(ordinary, &[]);
            b.switch_to_block(done);
            b.block_params(done)[0]
        };
        let minimum = b.ins().uextend(types::I64, minimum);
        let terminal = b.ins().iconst(types::I64, c.terminal as i64);
        words.write(b, c.tag, terminal);
        words.write(b, c.induction, stop);
        words.write(b, self.best, minimum);
        words.write(b, self.item, item);
        // Exact original prices/visits were already settled. The recorded
        // exhausted payload identity and terminal dependency proofs exclude
        // skipped updates/temporaries. The original terminal traversal still
        // constructs the returned value.
        let completed = b.current_block().expect("reduction completion");
        b.ins().jump(ordinary, &[]);
        completed
    }
}

/// Broadcast the unsigned minimum using the existing portable vector vocabulary.
/// Two pairwise shuffles keep both costs and stable indices in vector registers.
fn minimum_splat(b: &mut FunctionBuilder<'_>, mut values: Value) -> Value {
    for mask in [
        // Both inputs are the same vector. Name the wrapped half through the
        // second input so the backend recognizes one contiguous extraction,
        // rather than a general table lookup with a loaded shuffle mask.
        [
            8u8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
        ],
        [4u8, 5, 6, 7, 0, 1, 2, 3, 12, 13, 14, 15, 8, 9, 10, 11],
    ] {
        let flags = MemFlags::new().with_endianness(Endianness::Little);
        let bytes = b.ins().bitcast(types::I8X16, flags, values);
        let mask = b
            .func
            .dfg
            .immediates
            .push(ConstantData::from(mask.to_vec()));
        let shuffled = b.ins().shuffle(bytes, bytes, mask);
        let shuffled = b.ins().bitcast(types::I32X4, flags, shuffled);
        values = b.ins().umin(values, shuffled);
    }
    values
}

fn emit_item(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    proof: &Proof,
    shadow: usize,
    item: &Item,
    index: Value,
    error: Block,
    total: bool,
) -> Value {
    match item {
        Item::Constant(value) => b.ins().iconst(types::I64, *value as i64),
        Item::Word(word) => words.read(b, *word),
        Item::Index => index,
        Item::Length(array) => words.read(b, shadow + proof.view(*array) + 1),
        Item::Scalar(Op::U64ToU32, args)
            if total && std::env::var_os("MECH_NARROW_ORDERED_ITEM").is_some() =>
        {
            let low = emit_item_low32(b, words, proof, shadow, &args[0], index, error);
            b.ins().uextend(types::I64, low)
        }
        Item::Scalar(op, args) => {
            let args = args
                .iter()
                .map(|arg| emit_item(b, words, proof, shadow, arg, index, error, total))
                .collect::<Vec<_>>();
            // The complete-child proof certifies all reached source checks,
            // and this value is observable only on the same strict winner
            // edge. For other candidates evaluate a pure total extension:
            // it can neither fail nor read memory. Their result is discarded.
            match (total, *op) {
                (true, Op::U64Add) => b.ins().iadd(args[0], args[1]),
                (true, Op::U64Sub) => b.ins().isub(args[0], args[1]),
                (true, Op::U64ToU32) => {
                    let low = b.ins().ireduce(types::I32, args[0]);
                    b.ins().uextend(types::I64, low)
                }
                (
                    true,
                    Op::U32ToU64
                    | Op::U64WrappingAdd
                    | Op::U64WrappingSub
                    | Op::U64WrappingMul
                    | Op::U64BitAnd
                    | Op::U64BitOr
                    | Op::U64BitXor
                    | Op::U64BitNot
                    | Op::U64ShlWrap
                    | Op::U64ShrWrap,
                )
                | (false, _) => primitive(b, *op, &args, error),
                (true, _) => unreachable!("normalized item operation needs a total extension"),
            }
        }
    }
}

/// Project a total item expression into its observed low word. Each
/// specialized operator commutes with truncation to U32. Unrecognized
/// operators retain their full-width inputs before the projection.
fn emit_item_low32(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    proof: &Proof,
    shadow: usize,
    item: &Item,
    index: Value,
    error: Block,
) -> Value {
    if let Item::Constant(value) = item {
        return b.ins().iconst(types::I32, i64::from(*value as u32));
    }
    let low = |b: &mut FunctionBuilder<'_>, arg| {
        emit_item_low32(b, words, proof, shadow, arg, index, error)
    };
    if let Item::Scalar(op, args) = item {
        match (*op, args.as_slice()) {
            (Op::U32ToU64 | Op::U64ToU32, [arg]) => return low(b, arg),
            (Op::U64BitAnd, [Item::Constant(mask), arg] | [arg, Item::Constant(mask)])
                if *mask as u32 == u32::MAX =>
            {
                return low(b, arg);
            }
            (
                Op::U64Add | Op::U64WrappingAdd | Op::U64BitAnd | Op::U64BitOr | Op::U64BitXor,
                [left, right],
            ) => {
                let left = low(b, left);
                let right = low(b, right);
                return match op {
                    Op::U64Add | Op::U64WrappingAdd => b.ins().iadd(left, right),
                    Op::U64BitAnd => b.ins().band(left, right),
                    Op::U64BitOr => b.ins().bor(left, right),
                    Op::U64BitXor => b.ins().bxor(left, right),
                    _ => unreachable!(),
                };
            }
            (Op::U64ShlWrap, [arg, Item::Constant(count)]) if count & 63 < 32 => {
                let arg = low(b, arg);
                return b.ins().ishl_imm(arg, (count & 63) as i64);
            }
            _ => {}
        }
    }
    let full = emit_item(b, words, proof, shadow, item, index, error, true);
    b.ins().ireduce(types::I32, full)
}

fn biased_index(item: &Item, delta: u64) -> bool {
    matches!(item, Item::Scalar(Op::U64Add | Op::U64WrappingAdd, args)
        if matches!(args.as_slice(),
            [Item::Index, Item::Constant(value)] | [Item::Constant(value), Item::Index]
                if *value == delta))
}

fn split_shifted_item(mut item: &Item, delta: u64) -> Option<&Item> {
    // The carried value must have exactly the current U32 representation.
    // Source may express this as a conversion, a low-word mask, or both.
    let mut narrow = false;
    loop {
        match item {
            Item::Scalar(Op::U64ToU32, args) => {
                let [inner] = args.as_slice() else {
                    return None;
                };
                item = inner;
                narrow = true;
            }
            Item::Scalar(Op::U32ToU64, args) => {
                let [inner] = args.as_slice() else {
                    return None;
                };
                item = inner;
            }
            Item::Scalar(Op::U64BitAnd, args) => {
                let ([Item::Constant(mask), inner] | [inner, Item::Constant(mask)]) =
                    args.as_slice()
                else {
                    return None;
                };
                if *mask != u64::from(u32::MAX) {
                    return None;
                }
                item = inner;
                narrow = true;
            }
            _ => break,
        }
    }
    let Item::Scalar(Op::U64BitOr, args) = item else {
        return None;
    };
    let [left, right] = args.as_slice() else {
        return None;
    };
    if !narrow {
        return None;
    }
    if biased_index(left, delta) && !uses_index(right) {
        Some(right)
    } else if biased_index(right, delta) && !uses_index(left) {
        Some(left)
    } else {
        None
    }
}

fn uses_index(item: &Item) -> bool {
    match item {
        Item::Index => true,
        Item::Scalar(_, children) => children.iter().any(uses_index),
        _ => false,
    }
}
