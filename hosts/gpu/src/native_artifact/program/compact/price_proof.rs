//! Witness the complete checked fixed-span packed price recurrence.
use super::{Boundary, Effect, Path, Word};
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, Opcode as Op, Value, condcodes::IntCC, types,
};

pub(crate) const START: u64 = 3;
pub(crate) const ENDS: [u64; 3] = [259, 515, 547];
pub(crate) const MAX_BASE: u64 = u32::MAX as u64 - 127 * 16;
const INDEX: Word = Word::Cursor(0);
const WEIGHT: Word = Word::Input(16);
const OUTPUT_LEN: Word = Word::Input(40);
const ENTRY_LEN: Word = Word::Input(72);
const WEIGHT_SUM: u64 = 8;

pub(crate) struct Blend {
    pub entry: Inst,
    pub original: BlockCall,
    pub prices: Value,
    pub price_len: Value,
    pub entries: Value,
    pub entry_len: Value,
    pub at: Value,
    pub weight: Value,
    pub literal: Value,
    pub length: Value,
}

fn arith(op: Op, a: Word, b: Word) -> Word {
    Word::binary(op, a, b)
}

fn indexed(view: usize, scale: u64) -> Word {
    let offset = arith(Op::Ishl, INDEX, Word::Constant(scale));
    arith(Op::Iadd, Word::Input(view), offset)
}

fn upper(word: &Word) -> Option<u64> {
    match word {
        Word::Constant(n) => Some(*n),
        Word::Input(0 | 8) => Some(MAX_BASE),
        Word::Input(16) => Some(WEIGHT_SUM),
        Word::Halfword(_) => Some(u64::from(u16::MAX)),
        Word::Read(_) => Some(u64::from(u32::MAX)),
        Word::Binary(Op::Iadd, a, b) => upper(a)?.checked_add(upper(b)?),
        Word::Binary(Op::Imul, a, b) => upper(a)?.checked_mul(upper(b)?),
        Word::Binary(Op::Ushr, a, b) => {
            let Word::Constant(n) = **b else { return None };
            upper(a)?.checked_shr((n % 64) as u32)
        }
        Word::Binary(Op::Isub, a, b) if **a == Word::Constant(WEIGHT_SUM) && **b == WEIGHT => {
            Some(WEIGHT_SUM)
        }
        _ => None,
    }
}

fn decision(word: Word, segment: usize) -> Option<bool> {
    let Word::Compare(kind, a, b) = word else {
        return match word {
            Word::Constant(n @ (0 | 1)) => Some(n != 0),
            _ => None,
        };
    };
    use IntCC::{Equal, NotEqual, UnsignedGreaterThanOrEqual, UnsignedLessThan};
    match (kind, *a, *b) {
        (UnsignedGreaterThanOrEqual, Word::Constant(a), Word::Constant(b)) => Some(a >= b),
        (UnsignedLessThan, Word::Constant(a), Word::Constant(b)) => Some(a < b),
        (Equal, Word::Constant(a), Word::Constant(b)) => Some(a == b),
        (NotEqual, Word::Constant(a), Word::Constant(b)) => Some(a != b),
        (UnsignedLessThan, INDEX, Word::Constant(k)) => ENDS
            .iter()
            .position(|&n| n == k)
            .map(|boundary| segment <= boundary),
        (UnsignedLessThan, INDEX, limit) if limit == OUTPUT_LEN || limit == ENTRY_LEN => {
            (segment < ENDS.len()).then_some(true)
        }
        (UnsignedGreaterThanOrEqual, Word::Constant(WEIGHT_SUM), WEIGHT) => Some(true),
        (UnsignedGreaterThanOrEqual, Word::Binary(Op::Iadd, a, b), prior) if *a == prior => {
            upper(&a)?.checked_add(upper(&b)?).map(|_| true)
        }
        (Equal, Word::Binary(Op::Umulhi, a, b), Word::Constant(0)) => {
            upper(&a)?.checked_mul(upper(&b)?).map(|_| true)
        }
        _ => None,
    }
}

fn next(path: &Path<'_>, edge: BlockCall, expected: &Word) -> Option<Block> {
    let lists = &path.f.dfg.value_lists;
    let target = edge.block(lists);
    let [parameter] = path.f.dfg.block_params(target) else {
        return None;
    };
    let mut args = edge.args(lists);
    if path.f.dfg.value_type(*parameter) != types::I64
        || path.word(args.next()?.as_value()?)? != *expected
        || args.next().is_some()
    {
        return None;
    }
    Some(target)
}

fn trace(
    path: &mut Path<'_>,
    top: Block,
    finish: bool,
    select: impl Fn(Word) -> Option<bool>,
    effects: &[Effect],
) -> Option<()> {
    let mut block = top;
    for _ in 0..96 {
        let edge = match path.step(block)? {
            Boundary::Jump(_, edge) => edge,
            Boundary::Branch(condition, edges) => edges[usize::from(!select(condition)?)],
            Boundary::Return => return (finish && path.effects.is_empty()).then_some(()),
        };
        if edge.block(&path.f.dfg.value_lists) == top {
            let advanced = arith(Op::Iadd, INDEX, Word::Constant(1));
            next(path, edge, &advanced)?;
            return (!finish && path.effects == effects).then_some(());
        }
        block = path.enter(edge)?;
    }
    None
}

pub(crate) fn prove(f: &Function) -> Option<Blend> {
    let entry = f.layout.entry_block()?;
    let [heap, packet, output] = f.dfg.block_params(entry) else {
        return None;
    };
    if ![*heap, *packet, *output]
        .iter()
        .all(|v| f.dfg.value_type(*v) == types::I64)
        || f.signature.returns.len() != 2
        || f.signature
            .returns
            .iter()
            .any(|r| r.value_type != types::I64)
    {
        return None;
    }
    let mut path = Path {
        f,
        words: [
            (*heap, Word::Heap),
            (*packet, Word::InputPacket),
            (*output, Word::OutputPacket),
        ]
        .into(),
        effects: Vec::new(),
        input_bytes: 112,
    };
    let Boundary::Jump(entry, original) = path.step(entry)? else {
        return None;
    };
    let top = next(&path, original, &Word::Input(24))?;
    let [index] = f.dfg.block_params(top) else {
        return None;
    };
    path.words.insert(*index, INDEX);
    if !path.effects.is_empty() {
        return None;
    }
    let code_at = indexed(64, 1);
    let price_at = indexed(32, 2);
    let halfword = Word::Halfword(Box::new(code_at.clone()));
    let extra = arith(
        Op::Imul,
        arith(Op::Ushr, halfword, Word::Constant(9)),
        Word::Constant(16),
    );
    let current = Word::Read(Box::new(price_at.clone()));
    let other_weight = arith(Op::Isub, Word::Constant(WEIGHT_SUM), WEIGHT);
    let prior = arith(Op::Imul, current, other_weight);
    for (segment, base) in [Word::Input(8), Word::Input(0), Word::Constant(78)]
        .into_iter()
        .enumerate()
    {
        let default = arith(Op::Imul, arith(Op::Iadd, base, extra.clone()), WEIGHT);
        let sum = arith(Op::Iadd, default, prior.clone());
        let price = Word::LowWord(Box::new(arith(Op::Udiv, sum, Word::Constant(WEIGHT_SUM))));
        let effects = [
            Effect::ReadHalfword(code_at.clone()),
            Effect::Read(price_at.clone()),
            Effect::Store(price_at.clone(), price),
        ];
        trace(
            &mut path.clone(),
            top,
            false,
            |word| decision(word, segment),
            &effects,
        )?;
    }
    trace(
        &mut path.clone(),
        top,
        true,
        |word| decision(word, ENDS.len()),
        &[],
    )?;
    Some(Blend {
        entry,
        original,
        prices: path.packet_value(32)?,
        price_len: path.packet_value(40)?,
        entries: path.packet_value(64)?,
        entry_len: path.packet_value(72)?,
        at: path.packet_value(24)?,
        weight: path.packet_value(16)?,
        literal: path.packet_value(0)?,
        length: path.packet_value(8)?,
    })
}

pub(crate) struct Codes {
    pub entry: Inst,
    pub original: BlockCall,
    pub prices: Value,
    pub price_len: Value,
    pub entries: Value,
    pub entry_len: Value,
    pub codes: Value,
    pub code_len: Value,
    pub at: Value,
}

pub(crate) fn code_prices(f: &Function) -> Option<Codes> {
    let entry = f.layout.entry_block()?;
    let [heap, packet, output] = f.dfg.block_params(entry) else {
        return None;
    };
    if ![*heap, *packet, *output]
        .iter()
        .all(|v| f.dfg.value_type(*v) == types::I64)
        || f.signature.returns.len() != 2
        || f.signature
            .returns
            .iter()
            .any(|r| r.value_type != types::I64)
    {
        return None;
    }
    let mut path = Path {
        f,
        words: [
            (*heap, Word::Heap),
            (*packet, Word::InputPacket),
            (*output, Word::OutputPacket),
        ]
        .into(),
        effects: Vec::new(),
        input_bytes: 120,
    };
    let Boundary::Jump(entry, original) = path.step(entry)? else {
        return None;
    };
    let top = next(&path, original, &Word::Input(0))?;
    let [index] = f.dfg.block_params(top) else {
        return None;
    };
    path.words.insert(*index, INDEX);
    if !path.effects.is_empty() {
        return None;
    }
    use IntCC::{NotEqual, UnsignedLessThan};
    let entry_at = indexed(72, 1);
    let price_at = indexed(8, 2);
    let halfword = Word::Halfword(Box::new(entry_at.clone()));
    let sym = arith(Op::Band, halfword.clone(), Word::Constant(511));
    let sym_offset = arith(Op::Ishl, sym.clone(), Word::Constant(2));
    let sym_at = arith(Op::Iadd, Word::Input(40), sym_offset);
    let extra = arith(Op::Ushr, halfword, Word::Constant(9));
    let sym_valid = Word::comparison(UnsignedLessThan, sym.clone(), Word::Input(48));
    let small_symbol = Word::comparison(UnsignedLessThan, sym, Word::Constant(288));
    let entry_fits = Word::comparison(UnsignedLessThan, INDEX, Word::Input(80));
    let price_fits = Word::comparison(UnsignedLessThan, INDEX, Word::Input(16));
    for (lookup, small, nonzero) in [
        (false, true, false),
        (false, false, false),
        (true, true, false),
        (true, false, false),
        (true, true, true),
        (true, false, true),
    ] {
        let width = if lookup {
            arith(
                Op::Ushr,
                Word::Read(Box::new(sym_at.clone())),
                Word::Constant(16),
            )
        } else {
            Word::Constant(0)
        };
        let has_width = Word::comparison(NotEqual, width.clone(), Word::Constant(0));
        let selected = if nonzero {
            width
        } else {
            Word::Constant(if small { 13 } else { 10 })
        };
        let price = arith(
            Op::Imul,
            arith(Op::Iadd, selected, extra.clone()),
            Word::Constant(16),
        );
        let mut effects = vec![Effect::ReadHalfword(entry_at.clone())];
        if lookup {
            effects.push(Effect::Read(sym_at.clone()));
        }
        effects.push(Effect::Store(
            price_at.clone(),
            Word::LowWord(Box::new(price)),
        ));
        trace(
            &mut path.clone(),
            top,
            false,
            |word| {
                if word == entry_fits || word == price_fits {
                    return Some(true);
                }
                if word == sym_valid {
                    return Some(lookup);
                }
                if word == small_symbol {
                    return Some(small);
                }
                if word == has_width {
                    return Some(nonzero);
                }
                decision(word, 0)
            },
            &effects,
        )?;
    }
    trace(
        &mut path.clone(),
        top,
        true,
        |word| decision(word, ENDS.len()),
        &[],
    )?;
    Some(Codes {
        entry,
        original,
        prices: path.packet_value(8)?,
        price_len: path.packet_value(16)?,
        entries: path.packet_value(72)?,
        entry_len: path.packet_value(80)?,
        codes: path.packet_value(40)?,
        code_len: path.packet_value(48)?,
        at: path.packet_value(0)?,
    })
}
