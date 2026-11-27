//! Commit literal chunks in source order, stopping before the first mixed chunk.
//! The existing entry proves the complete node and counter views are disjoint.
use super::*;

pub(super) fn emit(f: &mut Function, group: Group, visits: usize, wide_keys: bool) {
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
    let chunks = visits / 4;
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

    let flags = MemFlags::new().with_notrap();
    let mut address = None;
    for chunk in 0..chunks {
        c.goto_bottom(reads[chunk]);
        let address = *address.get_or_insert_with(|| {
            let offset = c.ins().ishl_imm(group.ordinal, 3);
            c.ins().iadd(group.nodes, offset)
        });
        let words = (0..4)
            .map(|lane| {
                let visit = chunk * 4 + lane;
                if visit == 0 {
                    if wide_keys {
                        group.packed
                    } else {
                        c.ins().ireduce(types::I32, group.packed)
                    }
                } else {
                    let at = 4 - 8 * visit as i32;
                    if wide_keys {
                        c.ins().uload32(flags, address, at)
                    } else {
                        c.ins().load(types::I32, flags, address, at)
                    }
                }
            })
            .collect::<Vec<_>>();
        // The first source visit already selected length one. Every later
        // chunk checks all four tags before publishing any of its increments.
        let tags = words
            .iter()
            .skip(usize::from(chunk == 0))
            .map(|&word| c.ins().bxor_imm(word, 1))
            .collect::<Vec<_>>();
        let mut other = tags[0];
        for &tag in &tags[1..] {
            other = c.ins().bor(other, tag);
        }
        let other = c.ins().band_imm(other, i64::from(group.length_mask));
        let literals = c.ins().icmp_imm(IntCC::Equal, other, 0);
        if chunk == 0 {
            c.ins()
                .brif(literals, updates[chunk], &[], scalar, &scalar_args);
        } else {
            // Earlier chunks have committed. Reenter the exact original
            // iteration boundary at the first visit of this uncommitted chunk.
            let at = c.ins().iadd_imm(group.cursor, (chunk * 4) as i64);
            c.ins()
                .brif(literals, updates[chunk], &[], group.header, &[at.into()]);
        }

        c.goto_bottom(updates[chunk]);
        for packed in words {
            let key = c.ins().ushr_imm(packed, i64::from(group.key_shift));
            let key = c.ins().band_imm(key, i64::from(group.key_mask));
            let key = if wide_keys {
                key
            } else {
                c.ins().uextend(types::I64, key)
            };
            let offset = c.ins().ishl_imm(key, 2);
            let bin = c.ins().iadd(group.bins, offset);
            let count = c.ins().load(types::I32, flags, bin, 0);
            let updated = c.ins().iadd_imm(count, 1);
            c.ins().store(flags, updated, bin, 0);
        }
        let next = c.ins().iadd_imm(group.cursor, ((chunk + 1) * 4) as i64);
        if chunk + 1 == chunks {
            c.ins().jump(group.header, &[next.into()]);
        } else if chunk == 0 {
            // Admit the whole extension before any additional input read.
            // The four-visit prefix remains useful on shorter logical tails.
            let enough =
                c.ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, visits as i64);
            let readable = c.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                group.ordinal,
                (visits - 1) as i64,
            );
            let ready = c.ins().band(enough, readable);
            c.ins()
                .brif(ready, reads[chunk + 1], &[], group.header, &[next.into()]);
        } else {
            c.ins().jump(reads[chunk + 1], &[]);
        }
    }
}
