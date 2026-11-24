//! Verify the complete fixed-view insertion loop before selecting native sort.
use cranelift_codegen::ir::{
    Block, BlockCall, Endianness, Function, Inst, InstructionData as Data, Opcode as Op, Value,
    condcodes::IntCC, types,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Word {
    Heap,
    InputPacket,
    OutputPacket,
    Input(usize),
    GapBytes,
    SourceIndex,
    DestinationIndex,
    Cursor(usize),
    Constant(u64),
    Binary(Op, Box<Word>, Box<Word>),
    Compare(IntCC, Box<Word>, Box<Word>),
    LowWord(Box<Word>),
    Read(Box<Word>),
    Halfword(Box<Word>),
}

impl Word {
    fn binary(op: Op, a: Word, b: Word) -> Self {
        if let (Self::Constant(a), Self::Constant(b)) = (&a, &b) {
            let value = match op {
                Op::Band => Some(a & b),
                Op::Bor => Some(a | b),
                Op::Bxor => Some(a ^ b),
                Op::Imul => Some(a.wrapping_mul(*b)),
                Op::Iadd => Some(a.wrapping_add(*b)),
                Op::Isub => Some(a.wrapping_sub(*b)),
                Op::Ishl => Some(a.wrapping_shl(*b as u32)),
                Op::Ushr => Some(a.wrapping_shr(*b as u32)),
                _ => None,
            };
            if let Some(value) = value {
                return Self::Constant(value);
            }
        }
        if op == Op::Iadd
            && let Self::Constant(delta) = b
        {
            if delta == 0 {
                return a;
            }
            if let Self::Binary(Op::Iadd, base, previous) = &a
                && let Self::Constant(previous) = **previous
            {
                return Self::binary(
                    Op::Iadd,
                    base.as_ref().clone(),
                    Self::Constant(previous.wrapping_add(delta)),
                );
            }
        }
        if op == Op::Imul
            && let Self::Constant(scale @ (2 | 4)) = b
        {
            return Self::binary(Op::Ishl, a, Self::Constant(scale.trailing_zeros().into()));
        }
        Self::Binary(op, Box::new(a), Box::new(b))
    }

    fn comparison(kind: IntCC, a: Word, b: Word) -> Self {
        Self::Compare(kind, Box::new(a), Box::new(b))
    }

    fn boolean(&self) -> bool {
        match self {
            Self::Compare(..) => true,
            Self::Constant(n) => *n <= 1,
            Self::Binary(Op::Band | Op::Bor | Op::Bxor, a, b) => a.boolean() && b.boolean(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Effect {
    Read(Word),
    ReadHalfword(Word),
    Store(Word, Word),
    Output,
}

#[derive(Clone)]
struct Path<'a> {
    f: &'a Function,
    words: BTreeMap<Value, Word>,
    effects: Vec<Effect>,
    input_bytes: i32,
}

enum Boundary {
    Jump(Inst, BlockCall),
    Branch(Word, [BlockCall; 2]),
    Return,
}

impl Path<'_> {
    fn word(&self, value: Value) -> Option<Word> {
        self.words.get(&self.f.dfg.resolve_aliases(value)).cloned()
    }

    fn packet_value(&self, offset: usize) -> Option<Value> {
        self.words
            .iter()
            .find_map(|(&value, word)| (*word == Word::Input(offset)).then_some(value))
    }

    fn bind_loop(&mut self, edge: BlockCall, carried: &Word, local: Word) -> Option<Block> {
        let lists = &self.f.dfg.value_lists;
        let block = edge.block(lists);
        let [word] = self.f.dfg.block_params(block) else {
            return None;
        };
        let mut args = edge.args(lists);
        if self.word(args.next()?.as_value()?)? != *carried
            || args.next().is_some()
            || self.f.dfg.value_type(*word) != types::I64
        {
            return None;
        }
        self.words.insert(*word, local);
        Some(block)
    }

    fn enter(&mut self, edge: BlockCall) -> Option<Block> {
        let lists = &self.f.dfg.value_lists;
        let block = edge.block(lists);
        let params = self.f.dfg.block_params(block);
        let mut args = edge.args(lists);
        let mut values = Vec::with_capacity(params.len());
        for &param in params {
            let word = self.word(args.next()?.as_value()?)?;
            let kind = self.f.dfg.value_type(param);
            if kind != types::I64 && !(kind == types::I8 && word.boolean()) {
                return None;
            }
            values.push((param, word));
        }
        if args.next().is_some() {
            return None;
        }
        self.words.extend(values);
        Some(block)
    }

    fn empty_edge(&self, edge: BlockCall) -> Option<Block> {
        let lists = &self.f.dfg.value_lists;
        let target = edge.block(lists);
        (edge.args(lists).next().is_none() && self.f.dfg.block_params(target).is_empty())
            .then_some(target)
    }

    fn step(&mut self, mut block: Block) -> Option<Boundary> {
        // A bounded scan follows straight-line forwarding blocks. Loops and
        // branches are visited explicitly by the recurrence below.
        for _ in 0..128 {
            for inst in self.f.layout.block_insts(block) {
                let result = self.f.dfg.inst_results(inst);
                let out = match self.f.dfg.insts[inst] {
                    Data::Jump { destination, .. } => {
                        if let Some(next) = self.empty_edge(destination) {
                            block = next;
                            break;
                        }
                        return Some(Boundary::Jump(inst, destination));
                    }
                    Data::Brif { arg, blocks, .. } => {
                        return Some(Boundary::Branch(self.word(arg)?, blocks));
                    }
                    Data::MultiAry {
                        opcode: Op::Return,
                        args,
                    } => {
                        let [status, site] = args.as_slice(&self.f.dfg.value_lists) else {
                            return None;
                        };
                        return (self.word(*status)? == Word::Constant(0)
                            && self.word(*site)? == Word::Constant(0))
                        .then_some(Boundary::Return);
                    }
                    Data::UnaryImm {
                        opcode: Op::Iconst,
                        imm,
                    } if self.single_type(result, types::I64)
                        || self.single_type(result, types::I8) && (0..=1).contains(&imm.bits()) =>
                    {
                        Word::Constant(imm.bits() as u64)
                    }
                    Data::Binary {
                        opcode:
                            op @ (Op::Iadd
                            | Op::Isub
                            | Op::Ishl
                            | Op::Ushr
                            | Op::Band
                            | Op::Bor
                            | Op::Bxor
                            | Op::Imul
                            | Op::Umulhi
                            | Op::Udiv),
                        args,
                    } if self.single_type(result, types::I64) => {
                        Word::binary(op, self.word(args[0])?, self.word(args[1])?)
                    }
                    Data::BinaryImm64 {
                        opcode:
                            op @ (Op::IaddImm
                            | Op::IshlImm
                            | Op::UshrImm
                            | Op::BandImm
                            | Op::ImulImm
                            | Op::BxorImm),
                        arg,
                        imm,
                    } => {
                        let word = self.word(arg)?;
                        let value = imm.bits() as u64;
                        if op == Op::BandImm && value == 1 && word.boolean() {
                            word
                        } else {
                            if !self.single_type(result, types::I64) {
                                return None;
                            }
                            let op = match op {
                                Op::IaddImm => Op::Iadd,
                                Op::IshlImm => Op::Ishl,
                                Op::UshrImm => Op::Ushr,
                                Op::BandImm => Op::Band,
                                Op::ImulImm => Op::Imul,
                                Op::BxorImm => Op::Bxor,
                                _ => unreachable!(),
                            };
                            Word::binary(op, word, Word::Constant(value))
                        }
                    }
                    Data::Unary {
                        opcode: Op::Uextend,
                        arg,
                    } if self.single_type(result, types::I64) => {
                        let word = self.word(arg)?;
                        if !word.boolean() {
                            return None;
                        }
                        word
                    }
                    Data::Unary {
                        opcode: Op::Ireduce,
                        arg,
                    } if self.single_type(result, types::I32) => {
                        let mut word = self.word(arg)?;
                        if let Word::Binary(Op::Band, inner, mask) = &word
                            && **mask == Word::Constant(u64::from(u32::MAX))
                        {
                            word = inner.as_ref().clone();
                        }
                        Word::LowWord(Box::new(word))
                    }
                    Data::IntCompare { cond, args, .. }
                        if args
                            .iter()
                            .all(|arg| self.f.dfg.value_type(*arg) == types::I64) =>
                    {
                        Word::comparison(cond, self.word(args[0])?, self.word(args[1])?)
                    }
                    Data::IntCompareImm { cond, arg, imm, .. }
                        if self.f.dfg.value_type(arg) == types::I64 =>
                    {
                        Word::comparison(cond, self.word(arg)?, Word::Constant(imm.bits() as u64))
                    }
                    Data::Load {
                        opcode,
                        flags,
                        arg,
                        offset,
                    } if self.single_type(result, types::I64)
                        && flags.endianness(Endianness::Little) == Endianness::Little =>
                    {
                        let offset = i32::from(offset);
                        let address = self.word(arg)?;
                        if opcode == Op::Load
                            && address == Word::InputPacket
                            && (0..self.input_bytes).contains(&offset)
                            && offset % 8 == 0
                        {
                            Word::Input(offset as usize)
                        } else if opcode == Op::Uload32 && offset == 0 {
                            self.effects.push(Effect::Read(address.clone()));
                            Word::Read(Box::new(address))
                        } else if opcode == Op::Uload16 && offset == 0 {
                            self.effects.push(Effect::ReadHalfword(address.clone()));
                            Word::Halfword(Box::new(address))
                        } else {
                            return None;
                        }
                    }
                    Data::Store {
                        opcode: Op::Store,
                        flags,
                        args,
                        offset,
                    } if flags.endianness(Endianness::Little) == Endianness::Little => {
                        let offset = i32::from(offset);
                        let address = self.word(args[1])?;
                        let width = self.f.dfg.value_type(args[0]);
                        if width == types::I64
                            && address == Word::OutputPacket
                            && (0..88).contains(&offset)
                            && offset % 8 == 0
                        {
                            self.effects.push(Effect::Output);
                        } else if width == types::I32 && offset == 0 {
                            self.effects
                                .push(Effect::Store(address, self.word(args[0])?));
                        } else {
                            return None;
                        }
                        continue;
                    }
                    _ => return None,
                };
                let [value] = result else { return None };
                self.words.insert(*value, out);
            }
        }
        None
    }

    fn single_type(&self, results: &[Value], expected: cranelift_codegen::ir::Type) -> bool {
        matches!(results, [value] if self.f.dfg.value_type(*value) == expected)
    }

    fn condition(&mut self, start: Block, value: &Word) -> Option<[BlockCall; 2]> {
        let Boundary::Branch(actual, routes) = self.step(start)? else {
            return None;
        };
        (actual == *value).then_some(routes)
    }

    fn jump_expected(&mut self, edge: BlockCall, target: Block, word: &Word) -> Option<()> {
        let Boundary::Jump(_, next) = self.step(self.empty_edge(edge)?)? else {
            return None;
        };
        let block = self.bind_loop(next, word, word.clone())?;
        (block == target).then_some(())
    }
}

pub(super) struct Sort {
    pub entry: Inst,
    pub original: BlockCall,
    pub success: Block,
    pub data: Value,
    pub length: Value,
    pub gaps: Value,
}

fn address(index: Word) -> Word {
    let offset = Word::binary(Op::Ishl, index, Word::Constant(2));
    Word::binary(Op::Iadd, Word::Input(16), offset)
}

fn next_gap(path: &Path<'_>, edge: BlockCall, header: Block, exists: &Word) -> Option<Block> {
    let mut path = path.clone();
    let [repeat, finish] = path.condition(path.empty_edge(edge)?, exists)?;
    let shift = Word::binary(Op::Ushr, Word::GapBytes, Word::Constant(8));
    path.jump_expected(repeat, header, &shift)?;
    if !path.effects.is_empty() {
        return None;
    }
    let finish = path.empty_edge(finish)?;
    if !matches!(path.step(finish)?, Boundary::Return)
        || path.effects.iter().any(|effect| *effect != Effect::Output)
    {
        return None;
    }
    Some(finish)
}

pub(super) fn prove(f: &Function) -> Option<Sort> {
    let entry = f.layout.entry_block()?;
    let [heap, packet, output] = f.dfg.block_params(entry) else {
        return None;
    };
    if ![heap, packet, output]
        .into_iter()
        .all(|value| f.dfg.value_type(*value) == types::I64)
        || f.signature.returns.len() != 2
        || f.signature
            .returns
            .iter()
            .any(|item| item.value_type != types::I64)
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
        input_bytes: 64,
    };
    let Boundary::Jump(entry, original) = path.step(entry)? else {
        return None;
    };
    if !path.effects.is_empty() {
        return None;
    }
    let data = path.packet_value(16)?;
    let length = path.packet_value(0)?;
    let gaps = path.packet_value(8)?;
    let top = path.bind_loop(original, &Word::Input(8), Word::GapBytes)?;

    let gap = Word::binary(Op::Band, Word::GapBytes, Word::Constant(255));
    let exists = Word::comparison(IntCC::NotEqual, gap.clone(), Word::Constant(0));
    let fits = Word::comparison(IntCC::UnsignedLessThan, gap.clone(), Word::Input(0));
    let should_sort = Word::binary(Op::Band, exists.clone(), fits);
    let [start, skip] = path.condition(top, &should_sort)?;
    let success = next_gap(&path, skip, top, &exists)?;
    let from = path.bind_loop(start, &gap, Word::SourceIndex)?;
    let more = Word::comparison(IntCC::UnsignedLessThan, Word::SourceIndex, Word::Input(0));
    let [visit, finish] = path.condition(from, &more)?;
    if next_gap(&path, finish, top, &exists)? != success {
        return None;
    }

    let Boundary::Jump(_, loop_entry) = path.step(path.empty_edge(visit)?)? else {
        return None;
    };
    let key_at = address(Word::SourceIndex);
    if path.effects != [Effect::Read(key_at.clone())] {
        return None;
    }
    path.effects.clear();
    let key = Word::Read(Box::new(key_at));
    let inner = path.bind_loop(loop_entry, &Word::SourceIndex, Word::DestinationIndex)?;
    if top == from || top == inner || from == inner {
        return None;
    }
    let exists = Word::comparison(
        IntCC::UnsignedGreaterThanOrEqual,
        Word::DestinationIndex,
        gap.clone(),
    );
    let [compare, place] = path.condition(inner, &exists)?;
    let previous = Word::binary(Op::Isub, Word::DestinationIndex, gap);
    let prior_at = address(previous.clone());
    let prior = Word::Read(Box::new(prior_at.clone()));
    let greater = Word::comparison(IntCC::UnsignedGreaterThan, prior.clone(), key.clone());
    let mut competing = path.clone();
    let [shift, stop] = competing.condition(competing.empty_edge(compare)?, &greater)?;
    if competing.effects != [Effect::Read(prior_at)] {
        return None;
    }
    competing.effects.clear();
    let mut move_key = competing.clone();
    move_key.jump_expected(shift, inner, &previous)?;
    let at = address(Word::DestinationIndex);
    if move_key.effects != [Effect::Store(at.clone(), Word::LowWord(Box::new(prior)))] {
        return None;
    }
    let next = Word::binary(Op::Iadd, Word::SourceIndex, Word::Constant(1));
    for (mut path, edge) in [(path, place), (competing, stop)] {
        path.jump_expected(edge, from, &next)?;
        if path.effects
            != [Effect::Store(
                at.clone(),
                Word::LowWord(Box::new(key.clone())),
            )]
        {
            return None;
        }
    }
    Some(Sort {
        entry,
        original,
        success,
        data,
        length,
        gaps,
    })
}

#[path = "price_proof.rs"]
pub(super) mod price;
#[path = "price_emit.rs"]
pub(super) mod price_emit;
#[path = "tree_proof.rs"]
pub(super) mod tree;
#[path = "tree_emit.rs"]
pub(super) mod tree_emit;

#[cfg(test)]
#[path = "gap_proof_tests.rs"]
mod tests;
