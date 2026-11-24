//! Verify the checked loop before grouping constant byte stores indexed by source bytes.
use super::{Boundary, Effect, Path, Walk, Word};
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{
        Block, BlockCall, Function, Inst, InstBuilder, MemFlags, Opcode, Value, condcodes::IntCC,
        types,
    },
};

pub(crate) struct Scatter {
    entry: Inst,
    original: BlockCall,
    start: Value,
    limit: Value,
    offset: Value,
    src: Value,
    src_len: Value,
    dst: Value,
    dst_len: Value,
    byte: u8,
}

fn next(path: &mut Path<'_>, mut at: Block) -> Option<(Word, Block)> {
    for _ in 0..32 {
        match path.step_mode(at, Walk::ScatterBody)? {
            Boundary::Branch(cond, edges) => return Some((cond, path.enter(edges[0])?)),
            Boundary::Jump(_, edge) => at = path.enter(edge)?,
            Boundary::Return => return None,
        }
    }
    None
}

pub(crate) fn prove(
    f: &Function,
    packet_words: usize,
    src_word: usize,
    dst_word: usize,
) -> Option<Scatter> {
    use IntCC::{UnsignedGreaterThanOrEqual as Ge, UnsignedLessThan as Lt};
    use Word::{Constant, Input};
    let entry = f.layout.entry_block()?;
    let [heap, packet, result] = f.dfg.block_params(entry) else {
        return None;
    };
    if ![*heap, *packet, *result]
        .iter()
        .all(|v| f.dfg.value_type(*v) == types::I64)
    {
        return None;
    }
    // Division is the only potentially trapping scalar accepted by the shared
    // preheader interpreter. Even an unused division must keep its source trap.
    if f.layout.blocks().any(|b| {
        f.layout
            .block_insts(b)
            .any(|i| f.dfg.insts[i].opcode() == Opcode::Udiv)
    }) {
        return None;
    }
    let mut path = Path {
        f,
        words: [
            (*heap, Word::Heap),
            (*packet, Word::InputPacket),
            (*result, Word::OutputPacket),
        ]
        .into(),
        effects: Vec::new(),
        input_bytes: i32::try_from(packet_words.checked_mul(8)?).ok()?,
    };
    let Boundary::Jump(entry, original) = path.step_mode(entry, Walk::ScatterEntry)? else {
        return None;
    };
    let index = Word::Cursor(0);
    let mut args = original.args(&f.dfg.value_lists);
    let start = args.next()?.as_value()?;
    let Input(_) = path.word(start)? else {
        return None;
    };
    if !path.effects.is_empty() {
        return None;
    }
    // Only these values dominate the guard that replaces the preheader jump.
    // A later block parameter may forward the same input with a smaller Value ID.
    let preheader = path;
    let mut path = preheader.clone();
    let header = path.bind_loop(original, &preheader.word(start)?, index.clone())?;

    let (active, block) = next(&mut path, header)?;
    let Word::Compare(Lt, left, bound) = &active else {
        return None;
    };
    let Input(limit) = **bound else { return None };
    if **left != index {
        return None;
    }
    let (overflow, block) = next(&mut path, block)?;
    let Word::Compare(Ge, _, base) = &overflow else {
        return None;
    };
    let Input(offset) = **base else { return None };
    let add = |a, b| Word::binary(Opcode::Iadd, a, b);
    let position = add(Input(offset), index.clone());
    if overflow != Word::comparison(Ge, position.clone(), Input(offset)) {
        return None;
    }
    let src = src_word.checked_mul(8)?;
    let dst = dst_word.checked_mul(8)?;
    let (readable, block) = next(&mut path, block)?;
    if readable != Word::comparison(Lt, position.clone(), Input(src + 8)) {
        return None;
    }

    let source = add(Input(src), position);
    let byte = Word::Byte(Box::new(source.clone()));
    let (one_based, block) = next(&mut path, block)?;
    if one_based != Word::comparison(Ge, add(byte.clone(), Constant(1)), byte.clone()) {
        return None;
    }
    let (writable, block) = next(&mut path, block)?;
    if writable != Word::comparison(Lt, byte.clone(), Input(dst + 8)) {
        return None;
    }
    let (again, block) = next(&mut path, block)?;
    if again != active {
        return None;
    }
    let Boundary::Jump(_, edge) = path.step_mode(block, Walk::ScatterBody)? else {
        return None;
    };
    let next = add(index.clone(), Constant(1));
    if path.bind_loop(edge, &next, index)? != header {
        return None;
    }
    let [
        Effect::ReadByte(address),
        Effect::StoreByte(target, Constant(stored)),
    ] = path.effects.as_slice()
    else {
        return None;
    };
    if *address != source || *target != add(Input(dst), byte) {
        return None;
    }
    Some(Scatter {
        entry,
        original,
        start,
        limit: preheader.packet_value(limit)?,
        offset: preheader.packet_value(offset)?,
        src: preheader.packet_value(src)?,
        src_len: preheader.packet_value(src + 8)?,
        dst: preheader.packet_value(dst)?,
        dst_len: preheader.packet_value(dst + 8)?,
        byte: u8::try_from(*stored).ok()?,
    })
}

pub(crate) fn lower(f: &mut Function, s: Scatter) {
    use IntCC::{UnsignedGreaterThanOrEqual as Ge, UnsignedLessThanOrEqual as Le};
    let top = s.original.block(&f.dfg.value_lists);
    let original_args = s.original.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let group = f.dfg.make_block();
    f.layout.append_block(group);
    let cursor = f.dfg.append_block_param(group, types::I64);
    let mut b = FuncCursor::new(f).at_inst(s.entry);
    let ordered = b.ins().icmp(Le, s.start, s.limit);
    let count = b.ins().isub(s.limit, s.start);
    let enough = b.ins().icmp_imm(Ge, count, 8);
    let within = b.ins().icmp(Le, s.offset, s.src_len);
    let available = b.ins().isub(s.src_len, s.offset);
    let covered = b.ins().icmp(Le, s.limit, available);
    let full = b.ins().icmp_imm(Ge, s.dst_len, 256);
    let admitted = [enough, within, covered, full]
        .into_iter()
        .fold(ordered, |v, x| b.ins().band(v, x));
    let source = b.ins().iadd(s.src, s.offset);
    let stop = b.ins().iadd_imm(s.limit, -8);
    let byte = b.ins().iconst(types::I8, i64::from(s.byte));
    b.func
        .dfg
        .replace(s.entry)
        .brif(admitted, group, &[s.start.into()], top, &original_args);
    b.goto_bottom(group);
    let pointer = b.ins().iadd(source, cursor);
    let mem = MemFlags::new();
    // Each input load follows the previous scatter, including for overlapping views.
    for i in 0..8 {
        let value = b.ins().uload8(types::I64, mem, pointer, i);
        let dest = b.ins().iadd(s.dst, value);
        b.ins().store(mem, byte, dest, 0);
    }
    let next = b.ins().iadd_imm(cursor, 8);
    let more = b.ins().icmp(Le, next, stop);
    b.ins()
        .brif(more, group, &[next.into()], top, &[next.into()]);
}

#[cfg(test)]
#[path = "byte_scatter_tests.rs"]
mod tests;
