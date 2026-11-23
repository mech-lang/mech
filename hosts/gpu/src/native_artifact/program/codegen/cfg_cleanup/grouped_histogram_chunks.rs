//! Commit literal chunks in source order, stopping before the first mixed chunk.
//! The existing entry proves the complete node and counter views are disjoint.
use super::*;
use cranelift_codegen::ir::{BlockArg, ConstantData};

enum Chunk {
    Scalar {
        words: [Value; 4],
        literals: Value,
    },
    Vector {
        words: Value,
        second: Option<Value>,
        mixed: Value,
    },
}

impl Chunk {
    fn scalar(
        c: &mut FuncCursor<'_>,
        group: &Group,
        address: Value,
        visit: usize,
        first_selected: bool,
        wide_keys: bool,
    ) -> Self {
        let flags = MemFlags::new().with_notrap();
        let words = core::array::from_fn(|lane| {
            if first_selected && lane == 0 {
                if wide_keys {
                    group.packed
                } else {
                    c.ins().ireduce(types::I32, group.packed)
                }
            } else {
                let at = 4 - 8 * (visit + lane) as i32;
                if wide_keys {
                    c.ins().uload32(flags, address, at)
                } else {
                    c.ins().load(types::I32, flags, address, at)
                }
            }
        });
        let tags = words[usize::from(first_selected)..]
            .iter()
            .map(|&word| c.ins().bxor_imm(word, 1))
            .collect::<Vec<_>>();
        let mut other = tags[0];
        for &tag in &tags[1..] {
            other = c.ins().bor(other, tag);
        }
        let other = c.ins().band_imm(other, i64::from(group.length_mask));
        let literals = c.ins().icmp_imm(IntCC::Equal, other, 0);
        Self::Scalar { words, literals }
    }

    fn vector(
        c: &mut FuncCursor<'_>,
        group: &Group,
        address: Value,
        visit: usize,
        paired: bool,
    ) -> Self {
        // The visit address is the highest of the four admitted U64 records.
        let start = -8 * (visit + 3) as i32;
        let words = vector_words(c, address, start);
        let second = paired.then(|| vector_words(c, address, start - 32));
        let mixed = vector_mixed(c, group.length_mask, words, second);
        Self::Vector {
            words,
            second,
            mixed,
        }
    }

    fn branch(&self, c: &mut FuncCursor<'_>, update: Block, fallback: Block, args: &[BlockArg]) {
        match *self {
            Self::Scalar { literals, .. } => c.ins().brif(literals, update, &[], fallback, args),
            Self::Vector { mixed, .. } => c.ins().brif(mixed, fallback, args, update, &[]),
        };
    }

    fn update(&self, c: &mut FuncCursor<'_>, group: &Group, options: &Options) {
        match *self {
            Self::Scalar { words, .. } => {
                for packed in words {
                    let key = c.ins().ushr_imm(packed, i64::from(group.key_shift));
                    let key = c.ins().band_imm(key, i64::from(group.key_mask));
                    let key = if options.wide_keys {
                        key
                    } else {
                        c.ins().uextend(types::I64, key)
                    };
                    increment(c, group.bins, key);
                }
            }
            Self::Vector { words, second, .. } => {
                increment_vector(c, group, words, options.byte_keys);
                if let Some(second) = second {
                    increment_vector(c, group, second, options.byte_keys);
                }
            }
        }
    }
}

pub(super) fn vector_words(c: &mut FuncCursor<'_>, address: Value, offset: i32) -> Value {
    // Both loads stay inside the four admitted U64 records. Little endian
    // places the packed fields in odd lanes in ascending address order.
    let flags = MemFlags::new().with_notrap();
    let lo = c.ins().load(types::I8X16, flags, address, offset);
    let hi = c.ins().load(types::I8X16, flags, address, offset + 16);
    let pick = c.func.dfg.immediates.push(ConstantData::from(
        &[4, 5, 6, 7, 12, 13, 14, 15, 20, 21, 22, 23, 28, 29, 30, 31][..],
    ));
    let words = c.ins().shuffle(lo, hi, pick);
    c.ins().bitcast(
        types::I32X4,
        MemFlags::new().with_endianness(Endianness::Little),
        words,
    )
}

fn vector_mixed(
    c: &mut FuncCursor<'_>,
    length_mask: u32,
    words: Value,
    second: Option<Value>,
) -> Value {
    let one = c.ins().iconst(types::I32, 1);
    let mask = c.ins().iconst(types::I32, i64::from(length_mask));
    let one = c.ins().splat(types::I32X4, one);
    let mask = c.ins().splat(types::I32X4, mask);
    let mut other = c.ins().bxor(words, one);
    if let Some(second) = second {
        let mismatch = c.ins().bxor(second, one);
        other = c.ins().bor(other, mismatch);
    }
    let other = c.ins().band(other, mask);
    c.ins().vany_true(other)
}

fn increment_vector(c: &mut FuncCursor<'_>, group: &Group, words: Value, byte_keys: bool) {
    let shift = i64::from(group.key_shift);
    let (keys, lane_stride) = if byte_keys && group.key_mask == u32::from(u8::MAX) {
        // The low byte of each shifted word holds exactly the admitted key.
        let keys = c.ins().ushr_imm(words, shift);
        let bytes = c.ins().bitcast(
            types::I8X16,
            MemFlags::new().with_endianness(Endianness::Little),
            keys,
        );
        (bytes, 4)
    } else {
        let mask = c.ins().iconst(types::I32, i64::from(group.key_mask));
        let mask = c.ins().splat(types::I32X4, mask);
        let keys = c.ins().ushr_imm(words, shift);
        (c.ins().band(keys, mask), 1)
    };
    for lane in (0..4).rev() {
        let key = c.ins().extractlane(keys, lane * lane_stride);
        let key = c.ins().uextend(types::I64, key);
        increment(c, group.bins, key);
    }
}

fn increment(c: &mut FuncCursor<'_>, bins: Value, key: Value) {
    let flags = MemFlags::new().with_notrap();
    let offset = c.ins().ishl_imm(key, 2);
    let bin = c.ins().iadd(bins, offset);
    let count = c.ins().load(types::I32, flags, bin, 0);
    let updated = c.ins().iadd_imm(count, 1);
    c.ins().store(flags, updated, bin, 0);
}

// The first chunk committed and the extension has at least one complete
// chunk. Its exclusive logical limit and last physical group are invariant.
fn stream(
    c: &mut FuncCursor<'_>,
    group: &Group,
    first: Value,
    remaining: Value,
    options: &Options,
) {
    let paired = options.vector_chunks && options.pair_vector_chunks;
    let visits = if paired { 8 } else { 4 };
    let read = block(c.func);
    let update = block(c.func);
    let mixed = block(c.func);
    let address = c.func.dfg.append_block_param(read, types::I64);
    // The original first read proved ordinal < node_len. The admitted
    // extent implies node_len <= U64::MAX / 8, so ordinal + 1 cannot wrap.
    let extent = c.ins().iadd_imm(group.ordinal, 1);
    let limit = c.ins().umin(remaining, extent);
    let bulk = if paired {
        // Keep the already committed four visits outside the paired loop.
        let after_prefix = c.ins().iadd_imm(limit, -4);
        let groups = c.ins().band_imm(after_prefix, -visits);
        c.ins().iadd_imm(groups, 4)
    } else {
        c.ins().band_imm(limit, -visits)
    };
    let resume = c.ins().iadd(group.cursor, bulk);
    let last = c.ins().iadd_imm(bulk, -visits);
    let last = c.ins().ishl_imm(last, 3);
    let last = c.ins().isub(first, last);
    let next = c.ins().iadd_imm(first, -32);
    c.ins().jump(read, &[next.into()]);

    c.goto_bottom(read);
    let loaded = if options.vector_chunks {
        Chunk::vector(c, group, address, 0, paired)
    } else {
        Chunk::scalar(c, group, address, 0, false, options.wide_keys)
    };
    loaded.branch(c, update, mixed, &[]);

    c.goto_bottom(update);
    loaded.update(c, group, options);
    let finished = c.ins().icmp(IntCC::Equal, address, last);
    let next = c.ins().iadd_imm(address, -8 * visits);
    c.ins().brif(
        finished,
        group.header,
        &[resume.into()],
        read,
        &[next.into()],
    );

    c.goto_bottom(mixed);
    let delta = c.ins().isub(first, address);
    let visits = c.ins().ushr_imm(delta, 3);
    let at = c.ins().iadd(group.cursor, visits);
    if let Chunk::Vector {
        words,
        second: Some(_),
        ..
    } = loaded
    {
        // Keep a full first chunk if only the second chunk had mixed tags.
        let prefix = block(c.func);
        let mixed = vector_mixed(c, group.length_mask, words, None);
        c.ins().brif(mixed, group.header, &[at.into()], prefix, &[]);
        c.goto_bottom(prefix);
        increment_vector(c, group, words, options.byte_keys);
        let next = c.ins().iadd_imm(at, 4);
        c.ins().jump(group.header, &[next.into()]);
    } else {
        c.ins().jump(group.header, &[at.into()]);
    }
}

pub(super) fn emit(f: &mut Function, group: Group, options: Options) {
    let Options {
        visits,
        wide_keys,
        vector_chunks,
        stream_chunks,
        pair_vector_chunks,
        ..
    } = options;
    let Data::Brif {
        arg, mut blocks, ..
    } = f.dfg.insts[group.literal]
    else {
        unreachable!("group was derived from an integer branch")
    };
    let fallback = blocks[group.selected];
    let scalar = fallback.block(&f.dfg.value_lists);
    let scalar_args = fallback.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let check = block(f);
    let chunks = if stream_chunks { 1 } else { visits / 4 };
    let reads = (0..chunks).map(|_| block(f)).collect::<Vec<_>>();
    let updates = (0..chunks).map(|_| block(f)).collect::<Vec<_>>();
    blocks[group.selected] = BlockCall::new(check, [], &mut f.dfg.value_lists);
    f.dfg.insts[group.literal] = Data::Brif {
        opcode: Op::Brif,
        arg,
        blocks,
    };

    let mut c = FuncCursor::new(f);
    c.goto_bottom(check);
    let before_end = c
        .ins()
        .icmp(IntCC::UnsignedLessThan, group.cursor, group.end);
    let remaining = c.ins().isub(group.end, group.cursor);
    let enough = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 4);
    let readable = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, group.ordinal, 3);
    let enough = c.ins().band(before_end, enough);
    let ready = c.ins().band(enough, readable);
    c.ins().brif(ready, reads[0], &[], scalar, &scalar_args);

    let mut address = None;
    for chunk in 0..chunks {
        c.goto_bottom(reads[chunk]);
        let address = *address.get_or_insert_with(|| {
            let offset = c.ins().ishl_imm(group.ordinal, 3);
            c.ins().iadd(group.nodes, offset)
        });
        // The first source visit already selected length one. Every later
        // chunk checks all four tags before publishing any of its increments.
        let loaded = if vector_chunks && chunk != 0 {
            Chunk::vector(&mut c, &group, address, chunk * 4, false)
        } else {
            Chunk::scalar(&mut c, &group, address, chunk * 4, chunk == 0, wide_keys)
        };
        if chunk == 0 {
            loaded.branch(&mut c, updates[chunk], scalar, &scalar_args);
        } else {
            // Earlier chunks have committed. Reenter the exact original
            // iteration boundary at the first visit of this uncommitted chunk.
            let at = c.ins().iadd_imm(group.cursor, (chunk * 4) as i64);
            loaded.branch(&mut c, updates[chunk], group.header, &[at.into()]);
        }

        c.goto_bottom(updates[chunk]);
        loaded.update(&mut c, &group, &options);
        let next = c.ins().iadd_imm(group.cursor, ((chunk + 1) * 4) as i64);
        if chunk + 1 == chunks && !stream_chunks {
            c.ins().jump(group.header, &[next.into()]);
        } else if chunk == 0 {
            // Admit the whole extension before any additional input read.
            // The four-visit prefix remains useful on shorter logical tails.
            let extension = if stream_chunks && vector_chunks && pair_vector_chunks {
                visits.max(12)
            } else {
                visits
            };
            let enough = c.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                remaining,
                extension as i64,
            );
            let readable = c.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                group.ordinal,
                (extension - 1) as i64,
            );
            let ready = c.ins().band(enough, readable);
            let extend = if stream_chunks {
                block(c.func)
            } else {
                reads[chunk + 1]
            };
            c.ins()
                .brif(ready, extend, &[], group.header, &[next.into()]);
            if stream_chunks {
                c.goto_bottom(extend);
                stream(&mut c, &group, address, remaining, &options);
            }
        } else {
            c.ins().jump(reads[chunk + 1], &[]);
        }
    }
}
