//! Compact ordered reduction; long current intervals also use four-lane costs.
use super::cost::ScalarRead;
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
        let encoded_winner = biased.and_then(|(window, offset)| {
            if std::env::var_os("MECH_ORDERED_ENCODE_WINNER_WITH_OFFSET").is_none() {
                return None;
            }
            let low_bits = shifted_item_low_zeros(offset)?;
            let max = (1u64 << low_bits).checked_sub(window.delta)?;
            (max != 0).then_some((offset, max.min(u32::MAX as u64)))
        });
        let limit = encoded_winner.map_or(u32::MAX as u64, |(_, limit)| limit);
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
                super::super::traversal_facts::Operand::Constant(limit),
            ),
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, stop, limit as i64)
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
        let mut inputs = super::cost::Inputs::new(b, words, proof, shadow, &self.cost);
        let key = encoded_winner.map(|(offset, _)| {
            let low = emit_item_low32(b, words, proof, shadow, offset, start, ordinary);
            b.ins().uextend(types::I64, low)
        });
        let old_item = carry_item.then(|| {
            let item = words.read(b, self.item);
            // Restore any unchanged incoming item by applying the same XOR
            // again. Every admitted biased index lies below the key's first
            // possible set bit, so XOR packs it exactly like the source OR.
            key.map_or(item, |key| b.ins().bxor(item, key))
        });
        let carried = if carry_item { types::I64 } else { types::I32 };
        let scalar_body = b.create_block();
        for ty in [types::I64, types::I32, carried] {
            b.append_block_param(scalar_body, ty);
        }
        let forward = biased.map(|(window, _)| window);
        let deferred = biased.filter(|_| {
            encoded_winner.is_some()
                || std::env::var_os("MECH_ORDERED_PACK_WINNER_AT_EXIT").is_some()
        });
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
            let wide = reverse.zip(biased).filter(|(back, (front, _))| {
                cfg!(target_endian = "little")
                    && back.delta == front.delta
                    && proof.arrays[back.array].width == 8
                    && proof.arrays[front.array].width == 4
                    && std::env::var_os("MECH_FIRST_PACKED_REVERSE_READ").is_some()
            });
            let mut reads = Vec::new();
            let cursor = if let Some((back, (front, other))) = wide {
                let index = if front.delta <= 1
                    && std::env::var_os("MECH_ORDERED_SHORT_NARROW_ORDINAL").is_some()
                {
                    // The selected entry proved start < stop <= U32::MAX.
                    let narrow = b.ins().ireduce(types::I32, start);
                    let narrow = b.ins().iadd_imm(narrow, front.delta as i64);
                    b.ins().uextend(types::I64, narrow)
                } else {
                    b.ins().iadd_imm(start, front.delta as i64)
                };
                let base = words.read(b, shadow + proof.view(front.array));
                let offset = b.ins().ishl_imm(index, 2);
                reads.push((front, ScalarRead::LowWord(b.ins().iadd(base, offset))));
                let from_end = std::env::var_os("MECH_ORDERED_FIRST_REVERSE_FROM_END").is_some();
                let pointer = if from_end {
                    // Both addresses use the same admitted U64 cell. Basing
                    // the reverse access on start leaves the forward ordinal
                    // available for the winning item and the next visit.
                    inputs.pointer(b, proof, back, start)
                } else {
                    let view = shadow + proof.view(back.array);
                    let base = words.read(b, view);
                    let length = words.read(b, view + 1);
                    let ordinal = b.ins().isub(length, index);
                    let offset = b.ins().ishl_imm(ordinal, 3);
                    b.ins().iadd(base, offset)
                };
                // The complete-child witness admits this initialized element,
                // including both halves of the U64. Only its low cost is used.
                reads.push((back, ScalarRead::PackedWord(pointer)));
                Some((pointer, index, other, from_end))
            } else {
                if let Some(window) = reverse {
                    let pointer = inputs.pointer(b, proof, window, start);
                    reads.push((window, ScalarRead::LowWord(pointer)));
                }
                None
            };
            let cost = inputs.scalar(b, proof, &self.cost, start, &reads);
            let less = b.ins().icmp(IntCC::UnsignedLessThan, cost, best);
            let selected = if let Some((window, _)) = deferred {
                if let Some((_, index, _, _)) = cursor {
                    index
                } else {
                    b.ins().iadd_imm(start, window.delta as i64)
                }
            } else if let Some((_, index, other, _)) = cursor {
                let other = emit_item_low32(b, words, proof, shadow, other, start, ordinary);
                let index = b.ins().ireduce(types::I32, index);
                let item = b.ins().bor(index, other);
                b.ins().uextend(types::I64, item)
            } else {
                emit_item(
                    b,
                    words,
                    proof,
                    shadow,
                    &self.selected,
                    start,
                    ordinary,
                    true,
                )
            };
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
            let mut args = vec![next.into(), minimum.into(), item.into()];
            if let Some(window) = reverse {
                let pointer = if let Some((pointer, _, _, from_end)) = cursor {
                    if !from_end && std::env::var_os("MECH_ORDERED_LAZY_REVERSE_CURSOR").is_some() {
                        // The same address expressed from the adjusted reverse
                        // window base. Only the continuing edge needs this cursor.
                        inputs.pointer(b, proof, window, start)
                    } else {
                        pointer
                    }
                } else {
                    let (_, ScalarRead::LowWord(pointer)) =
                        reads.iter().find(|(read, _)| *read == window).unwrap()
                    else {
                        unreachable!("ordinary reverse cursor is a low-word address")
                    };
                    *pointer
                };
                args.push(pointer.into());
            }
            if let Some(window) = forward {
                let previous = cursor
                    .filter(|_| std::env::var_os("MECH_ORDERED_REUSE_FIRST_ORDINAL").is_some());
                let index = if let Some((_, index, _, _)) = previous {
                    b.ins().iadd_imm(index, 1)
                } else {
                    b.ins().iadd_imm(next, window.delta as i64)
                };
                let left = if std::env::var_os("MECH_ARM64_SUBTRACT_IMMEDIATE_BRANCH").is_some() {
                    b.ins().iadd_imm(remaining, -1)
                } else {
                    b.ins().isub(stop, next)
                };
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
        if std::env::var_os("MECH_ORDERED_VECTOR_HOIST_COST_WORDS").is_some() {
            inputs.prepare_vector_words(b);
        }
        let prefix = b.ins().band_imm(remaining, -4);
        let end = b.ins().iadd(start, prefix);
        let costs = b.ins().splat(types::I32X4, best);
        let indices = super::cost::constant(b, [u32::MAX; 4]);
        let lanes = super::cost::constant(b, [0, 1, 2, 3]);
        let induction_step = std::env::var_os("MECH_ORDERED_VECTOR_CARRY_LANES")
            .is_some()
            .then(|| super::cost::constant(b, [4; 4]));
        let test_at_latch = std::env::var_os("MECH_ORDERED_VECTOR_LATCH_TEST").is_some();
        let pointer_windows = if std::env::var_os("MECH_ORDERED_VECTOR_CARRY_POINTERS").is_some() {
            self.reads.as_slice()
        } else {
            &[]
        };
        let header = b.create_block();
        let body = (!test_at_latch).then(|| b.create_block());
        let finish = b.create_block();
        if test_at_latch {
            b.append_block_param(finish, types::I32X4);
            b.append_block_param(finish, types::I32X4);
            for _ in pointer_windows {
                b.append_block_param(finish, types::I64);
            }
        }
        for ty in [types::I64, types::I32X4, types::I32X4] {
            b.append_block_param(header, ty);
        }
        let mut entry = vec![start.into(), costs.into(), indices.into()];
        if induction_step.is_some() {
            b.append_block_param(header, types::I32X4);
            let first = b.ins().ireduce(types::I32, start);
            let first = b.ins().splat(types::I32X4, first);
            entry.push(b.ins().iadd(first, lanes).into());
        }
        let pointer_base = 3 + usize::from(induction_step.is_some());
        for &window in pointer_windows {
            b.append_block_param(header, types::I64);
            entry.push(inputs.pointer(b, proof, window, start).into());
        }
        b.ins().jump(header, &entry);
        b.switch_to_block(header);
        let params = b.block_params(header).to_vec();
        let k = params[0];
        if let Some(body) = body {
            let more = b.ins().icmp(IntCC::UnsignedLessThan, k, end);
            b.ins().brif(more, body, &[], finish, &[]);
            b.switch_to_block(body);
        }
        let index = if induction_step.is_some() {
            params[3]
        } else {
            let k32 = b.ins().ireduce(types::I32, k);
            let index = b.ins().splat(types::I32X4, k32);
            b.ins().iadd(index, lanes)
        };
        let pointers = pointer_windows
            .iter()
            .enumerate()
            .map(|(i, &window)| (window, params[pointer_base + i]))
            .collect::<Vec<_>>();
        let cost = if pointers.is_empty() {
            inputs.emit(b, proof, &self.cost, k, index)
        } else {
            inputs.emit_with_pointers(b, proof, &self.cost, k, index, &pointers)
        };
        let better = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
        let costs = b.ins().umin(cost, params[1]);
        let indices = b.ins().bitselect(better, index, params[2]);
        let next = b.ins().iadd_imm(k, 4);
        let mut next_group = vec![next.into(), costs.into(), indices.into()];
        if let Some(step) = induction_step {
            // The current lanes equal the low 32 bits of k + [0, 1, 2, 3].
            // Advancing by four preserves that identity and leaves the
            // admitted window and the scalar remainder bounds unchanged.
            next_group.push(b.ins().iadd(index, step).into());
        }
        let next_pointers = pointers
            .iter()
            .map(|&(window, pointer)| {
                // k advances by four. The immutable admitted window stays
                // the same; only its integer address geometry changes. The
                // terminal address is carried without dereferencing it.
                let stride = 4 * i64::from(proof.arrays[window.array].width);
                b.ins()
                    .iadd_imm(pointer, if window.reverse { -stride } else { stride })
            })
            .collect::<Vec<_>>();
        for &pointer in &next_pointers {
            next_group.push(pointer.into());
        }
        if test_at_latch {
            // This path admits at least 16 elements. Its rounded prefix
            // contains a full first group, and next never exceeds end.
            let more = b.ins().icmp(IntCC::UnsignedLessThan, next, end);
            let mut finished = vec![costs.into(), indices.into()];
            for &pointer in &next_pointers {
                finished.push(pointer.into());
            }
            b.ins().brif(more, header, &next_group, finish, &finished);
        } else {
            b.ins().jump(header, &next_group);
        }
        b.switch_to_block(finish);
        let (final_costs, final_indices) = if test_at_latch {
            let values = b.block_params(finish);
            (values[0], values[1])
        } else {
            (params[1], params[2])
        };
        let end_pointers = pointer_windows
            .iter()
            .enumerate()
            .map(|(i, &window)| {
                let pointer = if test_at_latch {
                    b.block_params(finish)[2 + i]
                } else {
                    params[pointer_base + i]
                };
                (window, pointer)
            })
            .collect::<Vec<_>>();
        // The incoming best already occupies every cost lane. A lane records
        // an index only after a strict improvement, so ties with that incoming
        // best retain the missing sentinel. Among improved lanes, reduce the
        // matching indices separately: the earliest index need not be in the
        // lowest lane after multiple vector groups.
        let minima = minimum_splat(b, final_costs);
        let equal = b.ins().icmp(IntCC::Equal, final_costs, minima);
        let missing_indices = b.ins().splat(types::I32X4, missing);
        let indices = b.ins().bitselect(equal, final_indices, missing_indices);
        let winners = minimum_splat(b, indices);
        let minimum = b.ins().extractlane(minima, 0);
        let winner = b.ins().extractlane(winners, 0);
        let carried_value = if let Some(old_item) = old_item {
            let changed = b.ins().icmp_imm(IntCC::NotEqual, winner, -1);
            let index = b.ins().uextend(types::I64, winner);
            let selected = if let Some((window, _)) = deferred {
                b.ins().iadd_imm(index, window.delta as i64)
            } else {
                emit_item(
                    b,
                    words,
                    proof,
                    shadow,
                    &self.selected,
                    index,
                    ordinary,
                    true,
                )
            };
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
            let pointer =
                if let Some((_, pointer)) = end_pointers.iter().find(|(read, _)| *read == window) {
                    b.ins()
                        .iadd_imm(*pointer, i64::from(proof.arrays[window.array].width))
                } else {
                    let previous = b.ins().iadd_imm(end, -1);
                    inputs.pointer(b, proof, window, previous)
                };
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
        let mut pointers = reverse_cursor
            .into_iter()
            .map(|(w, p)| (w, ScalarRead::LowWord(p)))
            .collect::<Vec<_>>();
        if let Some(window) = forward {
            // This index is already biased by the original forward access's
            // delta. Both entries have a nonempty range and the same admitted
            // immutable windows as the original scalar continuation.
            let base = words.read(b, shadow + proof.view(window.array));
            let offset = b
                .ins()
                .imul_imm(params[0], i64::from(proof.arrays[window.array].width));
            let pointer = b.ins().iadd(base, offset);
            pointers.push((window, ScalarRead::LowWord(pointer)));
        }
        let cost = inputs.scalar(b, proof, &self.cost, params[0], &pointers);
        let less = b.ins().icmp(IntCC::UnsignedLessThan, cost, params[1]);
        let minimum = if forward.is_some() {
            b.ins().select(less, cost, params[1])
        } else {
            b.ins().umin(cost, params[1])
        };
        let selected = if deferred.is_some() {
            params[0]
        } else if let Some((_, other)) = biased {
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
        let item = if let Some(key) = key {
            b.ins().bxor(params[1], key)
        } else if let Some((_, offset)) = deferred {
            // All updates strictly reduce the incoming best. It stays
            // unchanged exactly when the carried word is the original
            // item. Otherwise that word is the winning biased ordinal.
            let improved = b.ins().icmp(IntCC::UnsignedLessThan, minimum, best);
            let other = emit_item_low32(b, words, proof, shadow, offset, params[1], ordinary);
            let index = b.ins().ireduce(types::I32, params[1]);
            let low = b.ins().bor(index, other);
            let packed = b.ins().uextend(types::I64, low);
            b.ins().select(improved, packed, params[1])
        } else if carry_item {
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
    if cfg!(target_arch = "aarch64")
        && std::env::var_os("MECH_ORDERED_HORIZONTAL_MINIMUM").is_some()
    {
        let minimum = minimum_word(b, values);
        return b.ins().splat(types::I32X4, minimum);
    }
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

/// The unsigned minimum of all four words, expressed in portable scalar IR.
/// AArch64 recognizes the complete same-vector tree as a horizontal minimum.
pub(super) fn minimum_word(b: &mut FunctionBuilder<'_>, values: Value) -> Value {
    let a = b.ins().extractlane(values, 0);
    let b1 = b.ins().extractlane(values, 1);
    let c = b.ins().extractlane(values, 2);
    let d = b.ins().extractlane(values, 3);
    let low = b.ins().umin(a, b1);
    let high = b.ins().umin(c, d);
    b.ins().umin(low, high)
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

/// Guaranteed clear low bits in the projected U32 item component.
fn shifted_item_low_zeros(item: &Item) -> Option<u32> {
    let Item::Scalar(op, args) = item else {
        return None;
    };
    match (*op, args.as_slice()) {
        (Op::U32ToU64 | Op::U64ToU32, [inner]) => shifted_item_low_zeros(inner),
        (Op::U64ShlWrap, [_, Item::Constant(shift)]) if shift & 63 != 0 => {
            Some((shift & 63).min(32) as u32)
        }
        _ => None,
    }
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
